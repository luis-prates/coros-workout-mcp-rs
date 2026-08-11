#![recursion_limit = "256"]
#![allow(clippy::possible_missing_else)]

use anyhow::{Context, Result, anyhow};
use chrono::{Datelike, NaiveDate};
use directories::BaseDirs;
use md5::{Digest, Md5};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashMap, env, fs, path::PathBuf};

const DEFAULT_SOURCE_URL: &str = "https://d31oxp44ddzkyk.cloudfront.net/source/source_default/0/2fbd46e17bc54bc5873415c9fa767bdc.jpg";
const EU_URL: &str = "https://teameuapi.coros.com";
const US_URL: &str = "https://teamapi.coros.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthData {
    access_token: String,
    user_id: Value,
    region: String,
    timestamp: i64,
}

#[derive(Debug, Clone)]
struct CorosServer {
    client: reqwest::Client,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl CorosServer {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            tool_router: Self::tool_router(),
        }
    }
    fn catalog_path() -> PathBuf {
        env::var_os("COROS_CATALOG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/exercises.json")
            })
    }
    fn auth_path() -> Result<PathBuf> {
        Ok(BaseDirs::new()
            .ok_or_else(|| anyhow!("Could not find home config directory"))?
            .config_dir()
            .join("coros-workout-mcp/auth.json"))
    }
    fn load_auth() -> Result<Option<AuthData>> {
        let path = Self::auth_path()?;
        match fs::read_to_string(path) {
            Ok(text) => Ok(Some(serde_json::from_str(&text)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    fn store_auth(auth: &AuthData) -> Result<()> {
        let path = Self::auth_path()?;
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir)?;
        fs::write(&path, serde_json::to_vec(auth)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
    fn base_url(region: &str) -> Result<&'static str> {
        match region {
            "eu" => Ok(EU_URL),
            "us" => Ok(US_URL),
            _ => Err(anyhow!("region must be 'eu' or 'us'")),
        }
    }
    fn catalog() -> Result<Vec<Value>> {
        Ok(serde_json::from_slice(&fs::read(Self::catalog_path())?)?)
    }
    async fn login(&self, email: String, password: String, region: String) -> Result<AuthData> {
        let mut hash = Md5::new();
        hash.update(password.as_bytes());
        let password = format!("{:x}", hash.finalize());
        let data = self
            .client
            .post(format!("{}/account/login", Self::base_url(&region)?))
            .json(&json!({"account":email,"accountType":2,"pwd":password}))
            .send()
            .await?
            .json::<Value>()
            .await?;
        if data["result"] != "0000" {
            return Err(anyhow!(
                "COROS login failed: {}",
                data["message"]
                    .as_str()
                    .unwrap_or_else(|| data["result"].as_str().unwrap_or("unknown"))
            ));
        }
        let auth = AuthData {
            access_token: data["data"]["accessToken"]
                .as_str()
                .ok_or_else(|| anyhow!("COROS response had no access token"))?
                .into(),
            user_id: data["data"]["userId"].clone(),
            region,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        Self::store_auth(&auth)?;
        Ok(auth)
    }
    async fn auth(&self) -> Result<AuthData> {
        if let Some(auth) = Self::load_auth()? {
            return Ok(auth);
        }
        let email = env::var("COROS_EMAIL").ok();
        let password = env::var("COROS_PASSWORD").ok();
        match (email, password) {
            (Some(e), Some(p)) => {
                self.login(
                    e,
                    p,
                    env::var("COROS_REGION").unwrap_or_else(|_| "eu".into()),
                )
                .await
            }
            _ => Err(anyhow!("Not authenticated. Use authenticate_coros first.")),
        }
    }
    fn headers(auth: &AuthData) -> reqwest::header::HeaderMap {
        use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
        let mut h = HeaderMap::new();
        h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        h.insert(
            "accesstoken",
            HeaderValue::from_str(&auth.access_token).unwrap(),
        );
        h.insert(
            "yfheader",
            HeaderValue::from_str(&json!({"userId":auth.user_id}).to_string()).unwrap(),
        );
        h
    }
    async fn post(&self, auth: &AuthData, path: &str, body: Value) -> Result<Value> {
        let response = self
            .client
            .post(format!("{}{}", Self::base_url(&auth.region)?, path))
            .headers(Self::headers(auth))
            .json(&body)
            .send()
            .await?;
        let data = response
            .json::<Value>()
            .await
            .context("COROS returned a non-JSON response")?;
        if data["result"] != "0000" {
            return Err(anyhow!(
                "COROS API error ({path}): {}",
                data["message"]
                    .as_str()
                    .unwrap_or_else(|| data["result"].as_str().unwrap_or("unknown"))
            ));
        }
        Ok(data)
    }
    async fn get(&self, auth: &AuthData, path: &str, params: &[(&str, String)]) -> Result<Value> {
        let response = self
            .client
            .get(format!("{}{}", Self::base_url(&auth.region)?, path))
            .headers(Self::headers(auth))
            .query(params)
            .send()
            .await?;
        let data = response
            .json::<Value>()
            .await
            .context("COROS returned a non-JSON response")?;
        if data["result"] != "0000" {
            return Err(anyhow!(
                "COROS API error ({path}): {}",
                data["message"]
                    .as_str()
                    .unwrap_or_else(|| data["result"].as_str().unwrap_or("unknown"))
            ));
        }
        Ok(data)
    }
    async fn workouts(
        &self,
        auth: &AuthData,
        name: &str,
        sport_type: i64,
        limit: i64,
    ) -> Result<Vec<Value>> {
        Ok(self.post(auth,"/training/program/query",json!({"name":name,"supportRestExercise":1,"startNo":0,"limitSize":limit,"sportType":sport_type})).await?["data"].as_array().cloned().unwrap_or_default())
    }
    async fn workout_detail(&self, auth: &AuthData, id: &str) -> Result<Value> {
        self.get(
            auth,
            "/training/program/detail",
            &[("id", id.into()), ("supportRestExercise", "1".into())],
        )
        .await?["data"]
            .clone()
            .pipe(|v| {
                if v.is_null() {
                    Err(anyhow!("COROS returned no workout detail for {id}"))
                } else {
                    Ok(v)
                }
            })
    }
    async fn resolve_workout(
        &self,
        auth: &AuthData,
        id: Option<&str>,
        name: Option<&str>,
    ) -> Result<String> {
        if let Some(id) = id {
            return Ok(id.into());
        }
        let name =
            name.ok_or_else(|| anyhow!("Provide exactly one of workoutId or workoutName."))?;
        let expected = name.trim().to_lowercase();
        let matches: Vec<_> = self
            .workouts(auth, name, 0, 50)
            .await?
            .into_iter()
            .filter(|w| w["name"].as_str().unwrap_or("").trim().to_lowercase() == expected)
            .collect();
        if matches.len() != 1 {
            return Err(anyhow!(if matches.is_empty() {
                format!("No workout named \"{name}\" was found.")
            } else {
                "Multiple workouts have that name. Provide workoutId instead.".into()
            }));
        }
        matches[0]["id"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| matches[0]["id"].as_i64().map(|v| v.to_string()))
            .ok_or_else(|| anyhow!("The workout did not include a stable ID."))
    }
}
trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}
async fn result(f: impl std::future::Future<Output = Result<String>>) -> String {
    match f.await {
        Ok(s) => s,
        Err(e) => format!("Error: {e:#}"),
    }
}
fn text(v: Value) -> String {
    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
}
fn field<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn code(map: &[(&str, i64)], value: &str) -> Option<i64> {
    map.iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(value))
        .map(|(_, c)| *c)
}
fn dry(endpoint: &str, payload: Value) -> String {
    text(json!({"dryRun":true,"endpoint":endpoint,"payload":payload}))
}
fn iso(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| anyhow!("Use ISO YYYY-MM-DD dates."))
}
fn monday(date: NaiveDate) -> NaiveDate {
    date - chrono::Duration::days(date.weekday().num_days_from_monday().into())
}

#[derive(Deserialize, JsonSchema)]
struct Authenticate {
    email: Option<String>,
    password: Option<String>,
    region: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
struct Search {
    query: Option<String>,
    muscle: Option<String>,
    #[serde(rename = "bodyPart")]
    body_part: Option<String>,
    equipment: Option<String>,
    limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Exercise {
    name: String,
    sets: Option<i64>,
    reps: Option<i64>,
    duration: Option<i64>,
    rest_seconds: Option<i64>,
    weight_kg: Option<f64>,
}
#[derive(Deserialize, JsonSchema)]
struct CreateWorkout {
    name: String,
    overview: Option<String>,
    exercises: Vec<Exercise>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct UpdateExercises {
    sport_type: Option<i64>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListWorkouts {
    name: Option<String>,
    sport_type: Option<i64>,
    limit: Option<i64>,
}
#[derive(Deserialize, JsonSchema)]
struct Status {
    status: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct PlanId {
    plan_id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct PlanWorkout {
    workout_id: Option<String>,
    workout_name: Option<String>,
    weekday: Option<String>,
    date: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
struct PlanWeek {
    workouts: Vec<PlanWorkout>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CreatePlan {
    name: String,
    description: Option<String>,
    start_date: Option<String>,
    weeks: Vec<PlanWeek>,
    dry_run: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Calendar {
    start_date: String,
    end_date: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Schedule {
    workout_id: Option<String>,
    workout_name: Option<String>,
    date: String,
    timezone: Option<String>,
    allow_existing_entries: Option<bool>,
    dry_run: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Remove {
    date: String,
    scheduled_workout_id: i64,
    confirm: Option<bool>,
    dry_run: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Custom {
    name: String,
    description: Option<String>,
    body_part: String,
    primary_muscle: Option<String>,
    equipment: Option<String>,
    dry_run: Option<bool>,
}

#[tool_router]
impl CorosServer {
    #[tool(
        description = "Log in to COROS Training Hub. Uses supplied credentials or COROS_EMAIL/COROS_PASSWORD environment variables."
    )]
    async fn authenticate_coros(&self, Parameters(p): Parameters<Authenticate>) -> String {
        result(async {
            let e = p
                .email
                .or_else(|| env::var("COROS_EMAIL").ok())
                .ok_or_else(|| anyhow!("No credentials provided."))?;
            let pass = p
                .password
                .or_else(|| env::var("COROS_PASSWORD").ok())
                .ok_or_else(|| anyhow!("No credentials provided."))?;
            let a = self
                .login(
                    e,
                    pass,
                    p.region
                        .or_else(|| env::var("COROS_REGION").ok())
                        .unwrap_or_else(|| "eu".into()),
                )
                .await?;
            Ok(format!(
                "Authenticated successfully. User ID: {}, Region: {}.",
                a.user_id, a.region
            ))
        })
        .await
    }
    #[tool(
        description = "Check whether COROS authentication is available from stored credentials or environment variables."
    )]
    async fn check_coros_auth(&self) -> String {
        result(async {
            let a = self.auth().await?;
            Ok(format!(
                "Authenticated. User ID: {}, Region: {}",
                a.user_id, a.region
            ))
        })
        .await
    }
    #[tool(
        description = "Search the bundled COROS strength exercise catalog by name, muscle, body part, or equipment."
    )]
    async fn search_exercises(&self, Parameters(p): Parameters<Search>) -> String {
        result(async { let mut rows=Self::catalog()?; let matches=|v:&Value, q:&Option<String>, key:&str|q.as_ref().is_none_or(|q|field(v,key).to_lowercase().contains(&q.to_lowercase())); rows.retain(|v|matches(v,&p.query,"name")&&matches(v,&p.muscle,"muscleText")&&matches(v,&p.body_part,"partText")&&matches(v,&p.equipment,"equipmentText")); let count=rows.len(); let rows:Vec<_>=rows.into_iter().take(p.limit.unwrap_or(20).min(50)).map(|e|json!({"name":e["name"],"muscles":e["muscleText"],"secondaryMuscles":e["secondaryMuscleText"],"bodyParts":e["partText"],"equipment":e["equipmentText"],"sets":e["sets"],"targetValue":e["targetValue"],"restSeconds":e["restValue"]})).collect(); Ok(if rows.is_empty(){"No exercises found matching your search criteria.".into()}else{format!("Found {count} exercises:\n{}",text(json!(rows)))}) }).await
    }
    #[tool(
        description = "Create a strength workout on COROS Training Hub from catalog exercise names."
    )]
    async fn create_workout(&self, Parameters(p): Parameters<CreateWorkout>) -> String {
        result(async { if p.exercises.is_empty(){return Err(anyhow!("At least one exercise is required."));} let auth=self.auth().await?; let catalog=Self::catalog()?; let mut payloads=Vec::new(); for (i,e) in p.exercises.iter().enumerate(){let source=catalog.iter().find(|v|field(v,"name")==e.name).ok_or_else(||anyhow!("Exercise not found in catalog: \"{}\"",e.name))?;let mut v=source.clone();let o=v.as_object_mut().unwrap(); o.insert("access".into(),json!(0));o.insert("defaultOrder".into(),json!(0));o.insert("id".into(),json!(i+1));o.insert("sortNo".into(),json!(i+1));o.insert("isDefaultAdd".into(),json!(0));o.insert("isGroup".into(),json!(false));o.insert("isIntensityPercent".into(),json!(false));o.insert("nameText".into(),source["name"].clone());o.insert("descText".into(),source["desc"].clone());o.insert("originId".into(),source["id"].clone());o.insert("groupId".into(),json!(""));o.insert("targetDisplayUnit".into(),json!(0));o.insert("hrType".into(),json!(0));o.insert("intensityValueExtend".into(),json!(0));o.insert("intensityMultiplier".into(),json!(0));o.insert("intensityPercent".into(),json!(0));o.insert("intensityPercentExtend".into(),json!(0));o.insert("intensityDisplayUnit".into(),json!("6")); if let Some(x)=e.sets{o.insert("sets".into(),json!(x));}if let Some(x)=e.reps{o.insert("targetType".into(),json!(3));o.insert("targetValue".into(),json!(x));}else if let Some(x)=e.duration{o.insert("targetType".into(),json!(2));o.insert("targetValue".into(),json!(x));}if let Some(x)=e.rest_seconds{o.insert("restValue".into(),json!(x));}if let Some(x)=e.weight_kg{o.insert("intensityType".into(),json!(1));o.insert("intensityValue".into(),json!(x*1000.0));}payloads.push(v);} let overview=p.overview.unwrap_or_default(); let base=json!({"access":1,"authorId":"0","createTimestamp":0,"distance":0,"duration":0,"essence":0,"estimatedType":0,"estimatedValue":0,"exerciseNum":0,"exercises":payloads,"headPic":"","id":"0","idInPlan":"0","name":p.name,"nickname":"","originEssence":0,"overview":overview,"pbVersion":2,"planIdIndex":0,"poolLength":2500,"profile":"","referExercise":{"intensityType":1,"hrType":0,"valueType":1},"sex":0,"shareUrl":"","simple":false,"sourceUrl":DEFAULT_SOURCE_URL,"sportType":4,"star":0,"subType":65535,"targetType":0,"targetValue":0,"thirdPartyId":0,"totalSets":0,"trainingLoad":0,"type":0,"unit":0,"userId":"0","version":0,"videoCoverUrl":"","videoUrl":"","fastIntensityTypeName":"weight","poolLengthId":1,"poolLengthUnit":2,"sourceId":"425868133463670784"}); let calc=self.post(&auth,"/training/program/calculate",base.clone()).await?; let mut add=base;let o=add.as_object_mut().unwrap();o.insert("duration".into(),calc["data"]["duration"].clone());o.insert("totalSets".into(),calc["data"]["totalSets"].clone());o.insert("trainingLoad".into(),calc["data"]["trainingLoad"].clone());o.insert("distance".into(),json!("0"));o.insert("sets".into(),calc["data"]["totalSets"].clone());o.insert("pitch".into(),json!(0));self.post(&auth,"/training/program/add",add).await?;Ok(format!("Workout \"{}\" created successfully.",p.name)) }).await
    }
    #[tool(
        description = "Fetch the latest COROS exercise catalog and rebuild the local catalog file. Requires authentication."
    )]
    async fn update_exercises(&self, Parameters(p): Parameters<UpdateExercises>) -> String {
        result(async {
            let auth = self.auth().await?;
            let raw = self
                .get(
                    &auth,
                    "/training/exercise/query",
                    &[
                        ("userId", auth.user_id.to_string()),
                        ("sportType", p.sport_type.unwrap_or(4).to_string()),
                    ],
                )
                .await?["data"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let script = self
                .client
                .get("https://static.coros.com/locale/coros-traininghub-v2/en-US.prod.js")
                .send()
                .await?
                .text()
                .await?;
            let i18n: Value = serde_json::from_str(
                script
                    .trim()
                    .trim_start_matches("window.en_US=")
                    .trim_end_matches(';'),
            )?;
            let old = Self::catalog().unwrap_or_default();
            let old_by_code: HashMap<_, _> =
                old.iter().map(|x| (field(x, "codeName"), x)).collect();
            let converted: Vec<Value> = raw
                .into_iter()
                .map(|r| {
                    let name = i18n[field(&r, "name")]
                        .as_str()
                        .or_else(|| old_by_code.get(field(&r, "name")).map(|v| field(v, "name")))
                        .unwrap_or(field(&r, "name"))
                        .trim()
                        .to_string();
                    let mut x = r.clone();
                    let o = x.as_object_mut().unwrap();
                    o.insert("codeName".into(), r["name"].clone());
                    o.insert("name".into(), json!(name));
                    o.insert(
                        "desc".into(),
                        i18n[format!("{}_desc", field(&r, "name"))].clone(),
                    );
                    for (key, text) in [
                        ("muscle", "muscleText"),
                        ("part", "partText"),
                        ("equipment", "equipmentText"),
                    ] {
                        let labels = match key {
                            "muscle" => vec![
                                "",
                                "Deltoids",
                                "Chest",
                                "Latissimus Dorsi",
                                "Triceps",
                                "Abs",
                                "Lower Back",
                                "Glutes",
                                "Quadriceps",
                                "Obliques",
                                "Trapezius",
                                "Forearms",
                                "Biceps",
                                "Calves",
                                "Posterior Thigh",
                                "Hip Flexors",
                            ],
                            "part" => vec![
                                "Whole Body",
                                "",
                                "Chest",
                                "Back",
                                "Shoulders",
                                "Legs/Hips",
                                "Arms",
                                "Core",
                            ],
                            _ => vec![
                                "",
                                "Bodyweight",
                                "Dumbbells",
                                "Barbells",
                                "Bands",
                                "Bosu Ball",
                                "Gym Equipment",
                                "Exercise Ball",
                                "Foam Roller",
                                "Medicine Ball",
                                "Bench",
                                "Kettlebell",
                            ],
                        };
                        let s = r[key]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .map(|n| {
                                labels
                                    .get(n.as_u64().unwrap_or(999) as usize)
                                    .copied()
                                    .unwrap_or("")
                            })
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(",");
                        o.insert(text.into(), json!(s));
                    }
                    x
                })
                .collect();
            fs::write(Self::catalog_path(), serde_json::to_vec_pretty(&converted)?)?;
            Ok(format!(
                "Exercise catalog updated successfully. Total exercises: {}",
                converted.len()
            ))
        })
        .await
    }
    #[tool(description = "List workouts from COROS Training Hub.")]
    async fn list_workouts(&self, Parameters(p): Parameters<ListWorkouts>) -> String {
        result(async { let rows=self.workouts(&self.auth().await?,p.name.as_deref().unwrap_or(""),p.sport_type.unwrap_or(0),p.limit.unwrap_or(10).clamp(1,50)).await?;Ok(if rows.is_empty(){"No workouts found.".into()}else{text(json!(rows.iter().map(|w|json!({"id":w["id"],"name":w["name"],"overview":w["overview"],"sportType":w["sportType"],"duration":w["duration"],"totalSets":w["totalSets"],"exerciseNum":w["exerciseNum"]})).collect::<Vec<_>>()))}) }).await
    }
    #[tool(description = "List COROS Training Hub plan-library records.")]
    async fn list_training_plans(&self, Parameters(p): Parameters<Status>) -> String {
        result(async {
            let a = self.auth().await?;
            let status = match p.status.as_deref() {
                Some("active") => json!([1]),
                Some("completed") => json!([2]),
                _ => json!([1, 2]),
            };
            Ok(text(
                self.post(&a, "/training/plan/query", json!({"statusList":status}))
                    .await?["data"]
                    .clone(),
            ))
        })
        .await
    }
    #[tool(description = "Get a training plan by its stable COROS plan ID.")]
    async fn get_training_plan(&self, Parameters(p): Parameters<PlanId>) -> String {
        result(async {
            let a = self.auth().await?;
            Ok(text(
                self.get(
                    &a,
                    "/training/plan/detail",
                    &[("id", p.plan_id), ("supportRestExercise", "1".into())],
                )
                .await?["data"]
                    .clone(),
            ))
        })
        .await
    }
    #[tool(description = "Create a training plan from existing workouts. dryRun defaults to true.")]
    async fn create_training_plan(&self, Parameters(p): Parameters<CreatePlan>) -> String {
        result(async{if p.weeks.is_empty(){return Err(anyhow!("At least one week is required."));}let a=self.auth().await?;let anchor=p.start_date.as_deref().map(iso).transpose()?.map(monday);let mut placements=Vec::new();for (week_i,week) in p.weeks.iter().enumerate(){for (order,w) in week.workouts.iter().enumerate(){if w.workout_id.is_some()==w.workout_name.is_some(){return Err(anyhow!("Each workout needs exactly one of workoutId/workoutName."));}let day=if let Some(d)=&w.date{let anchor=anchor.ok_or_else(||anyhow!("startDate is required with exact dates."))?;let day=(iso(d)?-anchor).num_days();if day<0||day/7!=week_i as i64{return Err(anyhow!("Date {d} is outside week {}.",week_i+1));}day}else{let d=match w.weekday.as_deref(){Some("monday")=>0,Some("tuesday")=>1,Some("wednesday")=>2,Some("thursday")=>3,Some("friday")=>4,Some("saturday")=>5,Some("sunday")=>6,_=>return Err(anyhow!("Each workout needs weekday or date."))};week_i as i64*7+d};let id=self.resolve_workout(&a,w.workout_id.as_deref(),w.workout_name.as_deref()).await?;placements.push((day,order as i64,self.workout_detail(&a,&id).await?));}}placements.sort_by_key(|p|(p.0,p.1));let max=placements.iter().map(|p|p.0).max().unwrap();let entities:Vec<_>=placements.iter().enumerate().map(|(i,(day,order,_))|json!({"happenDay":"","idInPlan":i+1,"sortNo":0,"dayNo":day,"sortNoInPlan":order,"sortNoInSchedule":order})).collect();let programs:Vec<_>=placements.into_iter().enumerate().map(|(i,(_,_,mut x))|{x["idInPlan"]=json!(i+1);x["happenDay"]=json!("");x}).collect();let payload=json!({"name":p.name,"overview":p.description.unwrap_or_default(),"entities":entities,"programs":programs,"weekStages":[],"maxIdInPlan":entities.len(),"totalDay":max+1,"unit":0,"sourceId":"425868133463670784","sourceUrl":DEFAULT_SOURCE_URL,"minWeeks":0,"maxWeeks":0,"region":if a.region=="eu"{3}else{1},"pbVersion":2,"versionObjects":(1..=entities.len()).map(|id|json!({"id":id,"status":1})).collect::<Vec<_>>()});if p.dry_run.unwrap_or(true){Ok(dry("/training/plan/add",payload))}else{self.post(&a,"/training/plan/add",payload).await?;Ok("Training plan created.".into())}}).await
    }
    #[tool(description = "List COROS calendar entries for an inclusive ISO date range.")]
    async fn list_training_calendar(&self, Parameters(p): Parameters<Calendar>) -> String {
        result(async {
            if iso(&p.start_date)? > iso(&p.end_date)? {
                return Err(anyhow!("startDate must be on or before endDate."));
            }
            let a = self.auth().await?;
            Ok(text(
                self.get(
                    &a,
                    "/training/schedule/query",
                    &[
                        ("startDate", p.start_date.replace("-", "")),
                        ("endDate", p.end_date.replace("-", "")),
                        ("supportRestExercise", "1".into()),
                    ],
                )
                .await?["data"]
                    .clone(),
            ))
        })
        .await
    }
    #[tool(
        description = "Schedule one existing workout. dryRun defaults to true and it never replaces entries."
    )]
    async fn schedule_workout(&self, Parameters(p): Parameters<Schedule>) -> String {
        result(async {
            iso(&p.date)?;
            if p.workout_id.is_some() == p.workout_name.is_some() { return Err(anyhow!("Provide exactly one of workoutId or workoutName.")); }
            let tz = p.timezone.unwrap_or_else(|| "UTC".into());
            tz.parse::<chrono_tz::Tz>().map_err(|_| anyhow!("Invalid IANA timezone \"{tz}\"."))?;
            let a = self.auth().await?;
            let cal = self.get(&a, "/training/schedule/query", &[("startDate", p.date.replace("-", "")), ("endDate", p.date.replace("-", "")), ("supportRestExercise", "1".into())]).await?["data"].clone();
            let entries = cal["entities"].as_array().cloned().unwrap_or_default();
            if !p.allow_existing_entries.unwrap_or(false) && !entries.is_empty() { return Err(anyhow!("Calendar date {} already has entries.", p.date)); }
            let id = self.resolve_workout(&a, p.workout_id.as_deref(), p.workout_name.as_deref()).await?;
            let n = cal["maxIdInPlan"].as_i64().unwrap_or(0) + 1;
            let mut program = self.workout_detail(&a, &id).await?; program["idInPlan"] = json!(n);
            let payload = json!({"entities":[{"happenDay":p.date.replace("-", ""),"idInPlan":n,"sortNoInSchedule":entries.len()}],"programs":[program],"versionObjects":[{"id":n,"status":1}],"pbVersion":2});
            if p.dry_run.unwrap_or(true) { Ok(dry("/training/schedule/update", json!({"timezone":tz,"payload":payload}))) } else { self.post(&a, "/training/schedule/update", payload).await?; Ok(format!("Workout {id} scheduled for {} ({tz}).", p.date)) }
        }).await
    }
    #[tool(
        description = "Remove a calendar entry. Requires confirm true and dryRun false for a live deletion."
    )]
    async fn remove_scheduled_workout(&self, Parameters(p): Parameters<Remove>) -> String {
        result(async {
            iso(&p.date)?;
            let a = self.auth().await?;
            let cal = self
                .get(
                    &a,
                    "/training/schedule/query",
                    &[
                        ("startDate", p.date.replace("-", "")),
                        ("endDate", p.date.replace("-", "")),
                        ("supportRestExercise", "1".into()),
                    ],
                )
                .await?["data"]
                .clone();
            let entry = cal["entities"]
                .as_array()
                .and_then(|x| {
                    x.iter()
                        .find(|x| x["idInPlan"].as_i64() == Some(p.scheduled_workout_id))
                })
                .ok_or_else(|| {
                    anyhow!("No scheduled workout with that id exists on {}.", p.date)
                })?;
            let mut version = json!({"id":p.scheduled_workout_id,"status":3});
            for k in ["planId", "planProgramId", "labelId"] {
                if !entry[k].is_null() {
                    version[k] = entry[k].clone();
                }
            }
            let payload = json!({"versionObjects":[version],"pbVersion":2});
            if p.dry_run.unwrap_or(true) {
                Ok(dry("/training/schedule/update", payload))
            } else if !p.confirm.unwrap_or(false) {
                Err(anyhow!(
                    "Set confirm: true with dryRun: false to remove this calendar entry."
                ))
            } else {
                self.post(&a, "/training/schedule/update", payload).await?;
                Ok(format!(
                    "Scheduled workout {} removed.",
                    p.scheduled_workout_id
                ))
            }
        })
        .await
    }
    #[tool(description = "List user-created Strength exercises.")]
    async fn list_custom_exercises(&self) -> String {
        result(async {
            let a = self.auth().await?;
            let raw = self
                .get(
                    &a,
                    "/training/exercise/query",
                    &[("userId", a.user_id.to_string()), ("sportType", "4".into())],
                )
                .await?["data"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            Ok(text(json!(
                raw.into_iter()
                    .filter(|e| e["access"] == 1)
                    .collect::<Vec<_>>()
            )))
        })
        .await
    }
    #[tool(description = "Create a custom Standard Strength exercise. dryRun defaults to true.")]
    async fn create_custom_exercise(&self, Parameters(p): Parameters<Custom>) -> String {
        result(async{let parts=[("Whole Body",0),("Chest",2),("Back",3),("Shoulders",4),("Legs/Hips",5),("Arms",6),("Core",7)];let muscles=[("Deltoids",1),("Chest",2),("Latissimus Dorsi",3),("Triceps",4),("Abs",5),("Lower Back",6),("Glutes",7),("Quadriceps",8),("Obliques",9),("Trapezius",10),("Forearms",11),("Biceps",12),("Calves",13),("Posterior Thigh",14),("Hip Flexors",15)];let equipment=[("Bodyweight",1),("Dumbbells",2),("Barbells",3),("Bands",4),("Bosu Ball",5),("Gym Equipment",6),("Exercise Ball",7),("Foam Roller",8),("Medicine Ball",9),("Bench",10),("Kettlebell",11)];let part=code(&parts,&p.body_part).ok_or_else(||anyhow!("Unknown bodyPart \"{}\".",p.body_part))?;let muscle=p.primary_muscle.as_deref().map(|s|code(&muscles,s).ok_or_else(||anyhow!("Unknown primaryMuscle \"{s}\"."))).transpose()?;if part!=0&&muscle.is_none(){return Err(anyhow!("primaryMuscle is required unless bodyPart is Whole Body."));}let eq=p.equipment.as_deref().map(|s|code(&equipment,s).ok_or_else(||anyhow!("Unknown equipment \"{s}\"."))).transpose()?;let payload=json!({"access":1,"sportType":4,"exerciseType":2,"name":p.name,"overview":p.description.unwrap_or_default(),"part":[part],"muscle":muscle.map(|x|vec![x]).unwrap_or_default(),"muscleRelevance":[],"equipment":eq.map(|x|vec![x]).unwrap_or_default(),"intensityCustom":0,"intensityMultiplier":0,"intensityType":1,"intensityValue":0,"intensityValueExtend":0,"restType":1,"restValue":30,"targetType":3,"targetValue":15});if p.dry_run.unwrap_or(true){Ok(dry("/training/exercise/add",payload))}else{self.post(&self.auth().await?,"/training/exercise/add",payload).await?;Ok("Custom Strength exercise created.".into())}}).await
    }
}
#[tool_handler(router = self.tool_router)]
impl ServerHandler for CorosServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("coros-workout-rs", "1.0.0"))
            .with_instructions(
                "Create and manage COROS strength workouts through the Training Hub API.",
            )
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    CorosServer::new()
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_is_searchable() {
        let catalog = CorosServer::catalog().expect("bundled catalog must parse");
        assert!(catalog.len() > 300);
        assert!(
            catalog
                .iter()
                .any(|exercise| field(exercise, "name") == "Push-ups")
        );
    }

    #[test]
    fn date_validation_and_monday_anchor_work() {
        let date = iso("2026-08-17").expect("valid ISO date");
        assert_eq!(monday(date), date);
        assert!(iso("17-08-2026").is_err());
    }

    #[test]
    fn coros_code_lookup_is_case_insensitive() {
        assert_eq!(code(&[("Chest", 2)], "cHeSt"), Some(2));
    }
}
