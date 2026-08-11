use crate::DEFAULT_SOURCE_URL;
use crate::parameters::*;
use crate::presentation::*;
use anyhow::anyhow;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};

#[derive(Debug, Clone)]
pub(crate) struct CorosServer {
    pub(crate) client: reqwest::Client,
    #[allow(dead_code)]
    pub(crate) tool_router: ToolRouter<Self>,
}

impl CorosServer {
    pub(crate) fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            tool_router: Self::tool_router(),
        }
    }
}

use serde_json::{Value, json};
use std::{collections::HashMap, env, fs};

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
    #[tool(
        description = "Create a structured running workout on COROS Training Hub. Each step needs exactly one of durationSeconds or distanceMeters."
    )]
    async fn create_run_workout(
        &self,
        Parameters(p): Parameters<CreateEnduranceWorkout>,
    ) -> String {
        result(async {
            let payload = endurance_workout_payload(&p, 1)?;
            self.create_program(&self.auth().await?, payload).await?;
            Ok(format!(
                "Running workout \"{}\" created successfully.",
                p.name
            ))
        })
        .await
    }
    #[tool(
        description = "Create a structured cycling workout on COROS Training Hub. Each step needs exactly one of durationSeconds or distanceMeters."
    )]
    async fn create_bike_workout(
        &self,
        Parameters(p): Parameters<CreateEnduranceWorkout>,
    ) -> String {
        result(async {
            let payload = endurance_workout_payload(&p, 2)?;
            self.create_program(&self.auth().await?, payload).await?;
            Ok(format!(
                "Cycling workout \"{}\" created successfully.",
                p.name
            ))
        })
        .await
    }
    #[tool(
        description = "Clone an existing workout and patch selected zero-based steps. dryRun defaults to true; the original workout is never changed."
    )]
    async fn update_workout(&self, Parameters(p): Parameters<UpdateWorkout>) -> String {
        result(async {
            let auth = self.auth().await?;
            let original = self.workout_detail(&auth, &p.workout_id).await?;
            let payload = clone_workout_payload(original, &p)?;
            if p.dry_run.unwrap_or(true) {
                Ok(dry("/training/program/add", payload))
            } else {
                self.create_program(&auth, payload).await?;
                Ok(
                    "Updated workout created as a new COROS workout; the original was preserved."
                        .into(),
                )
            }
        })
        .await
    }
    #[tool(
        description = "List completed activities recorded by a COROS watch. startDate/endDate are YYYYMMDD integers; sportTypes optionally filters by COROS sport type IDs."
    )]
    async fn list_activities(&self, Parameters(p): Parameters<ListActivities>) -> String {
        result(async {
            if let (Some(start), Some(end)) = (p.start_date, p.end_date) && start > end {
                return Err(anyhow!("startDate must be on or before endDate."));
            }
            let (count, activities) = self.activities(
                &self.auth().await?,
                p.page_number.unwrap_or(1).max(1),
                p.limit.unwrap_or(20).clamp(1, 50),
                p.start_date,
                p.end_date,
                p.sport_types.as_deref(),
            ).await?;
            if activities.is_empty() { return Ok("No activities found.".into()); }
            let formatted = activities.iter().map(|activity| {
                let sport_type = activity["sportType"].as_i64().unwrap_or_default();
                let sport = sport_type_name(sport_type).unwrap_or("Unknown sport");
                let mut metrics = format_duration(activity["totalTime"].as_i64().unwrap_or_default());
                if let Some(distance) = activity["distance"].as_f64().filter(|value| *value > 0.0) {
                    metrics.push_str(&format!(", {:.2} km", distance / 1000.0));
                }
                if let Some(calories) = activity["calorie"].as_f64().filter(|value| *value > 0.0) {
                    metrics.push_str(&format!(", {} kcal", (calories / 1000.0).round()));
                }
                if let Some(heart_rate) = activity["avgHr"].as_i64().filter(|value| *value > 0) {
                    metrics.push_str(&format!(", avgHR {heart_rate}"));
                }
                if let Some(load) = activity["trainingLoad"].as_i64().filter(|value| *value > 0) {
                    metrics.push_str(&format!(", TL {load}"));
                }
                format!(
                    "- **{}** ({}, {sport})\n  {metrics}\n  labelId: `{}`, sportType: {sport_type}",
                    field(activity, "name"),
                    format_activity_date(activity["date"].as_i64().unwrap_or_default()),
                    activity["labelId"].as_str().unwrap_or_default(),
                )
            }).collect::<Vec<_>>().join("\n");
            Ok(format!(
                "Found {} activit{} (total available: {count}):\n\n{formatted}",
                activities.len(), if activities.len() == 1 { "y" } else { "ies" }
            ))
        }).await
    }
    #[tool(
        description = "Get completed activity details. Strength activities include lifted sets; other sports return summary, laps, and available HR/power zones. Use labelId and sportType from list_activities; sportType defaults to 402 (Strength)."
    )]
    async fn get_activity_detail(&self, Parameters(p): Parameters<ActivityDetail>) -> String {
        result(async {
            let sport_type = p.sport_type.unwrap_or(402);
            let detail = self
                .activity_detail(&self.auth().await?, &p.label_id, sport_type)
                .await?;
            if sport_type != 402 {
                return Ok(format!(
                    "Activity {} ({}) detail:\n{}",
                    p.label_id,
                    sport_type_name(sport_type).unwrap_or("Unknown sport"),
                    text(activity_detail_overview(&detail))
                ));
            }
            let exercise_summary = summarize_strength_activity(&Self::catalog()?, &detail);
            if exercise_summary.is_empty() {
                return Ok(format!(
                    "Activity {} detail:\n{}",
                    p.label_id,
                    text(activity_detail_overview(&detail))
                ));
            }
            let summary = &detail["summary"];
            Ok(format!(
                "Activity {}:\n  Duration: {}, {} kcal, avgHR {}, TL {}\n  Total: {} sets, {} reps\n\nExercises:\n{exercise_summary}",
                p.label_id,
                format_duration(summary["totalTime"].as_i64().unwrap_or_default() / 100),
                (summary["calories"].as_f64().unwrap_or_default() / 1000.0).round(),
                summary["avgHr"].as_i64().unwrap_or_default(),
                summary["trainingLoad"].as_i64().unwrap_or_default(),
                summary["sets"].as_i64().unwrap_or_default(),
                summary["totalReps"].as_i64().unwrap_or_default(),
            ))
        })
        .await
    }
    #[tool(description = "List COROS sport type IDs and their current names from the service.")]
    async fn get_sport_types(&self) -> String {
        result(async { Ok(text(self.sport_types(&self.auth().await?).await?)) }).await
    }
    #[tool(
        description = "Get the COROS private profile, including available training-zone settings."
    )]
    async fn get_profile(&self) -> String {
        result(async { Ok(text(self.private_profile(&self.auth().await?).await?)) }).await
    }
    #[tool(
        description = "Get daily COROS training metrics for an inclusive YYYYMMDD date range, including HRV, resting HR, training load, and recent EvoLab analysis."
    )]
    async fn get_daily_metrics(&self, Parameters(p): Parameters<DailyMetrics>) -> String {
        result(async {
            if p.start_date > p.end_date {
                return Err(anyhow!("startDate must be on or before endDate."));
            }
            Ok(text(
                self.daily_metrics(&self.auth().await?, p.start_date, p.end_date)
                    .await?,
            ))
        })
        .await
    }
    #[tool(
        description = "Get a temporary COROS download URL for a completed activity. fileType must be csv, gpx, kml, tcx, or fit."
    )]
    async fn export_activity_file(&self, Parameters(p): Parameters<ExportActivityFile>) -> String {
        result(async {
            let url = self
                .activity_file_url(&self.auth().await?, &p.label_id, p.sport_type, &p.file_type)
                .await?;
            Ok(format!(
                "Temporary {} download URL:\n{url}",
                p.file_type.to_ascii_uppercase()
            ))
        })
        .await
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
            if p.dry_run.unwrap_or(true) { Ok(dry("/training/schedule/update", json!({"timezone":tz,"payload":payload}))) } else { self.post(&a,"/training/schedule/update", payload).await?; Ok(format!("Workout {id} scheduled for {} ({tz}).", p.date)) }
        }).await
    }
    #[tool(
        description = "Replace one scheduled calendar workout with another. dryRun defaults to true; live replacement requires confirm true."
    )]
    async fn replace_scheduled_workout(
        &self,
        Parameters(p): Parameters<ReplaceScheduledWorkout>,
    ) -> String {
        result(async {
            iso(&p.date)?;
            if p.replacement_workout_id.is_some() == p.replacement_workout_name.is_some() {
                return Err(anyhow!("Provide exactly one of replacementWorkoutId or replacementWorkoutName."));
            }
            let timezone = p.timezone.unwrap_or_else(|| "UTC".into());
            timezone.parse::<chrono_tz::Tz>().map_err(|_| anyhow!("Invalid IANA timezone \"{timezone}\"."))?;
            let auth = self.auth().await?;
            let calendar = self.get(&auth, "/training/schedule/query", &[("startDate", p.date.replace("-", "")), ("endDate", p.date.replace("-", "")), ("supportRestExercise", "1".into())]).await?["data"].clone();
            let entry = calendar["entities"].as_array().and_then(|entries| entries.iter().find(|entry| entry["idInPlan"].as_i64() == Some(p.scheduled_workout_id))).ok_or_else(|| anyhow!("No scheduled workout with that id exists on {}.", p.date))?;
            let replacement_id = self.resolve_workout(&auth, p.replacement_workout_id.as_deref(), p.replacement_workout_name.as_deref()).await?;
            let new_id_in_plan = calendar["maxIdInPlan"].as_i64().unwrap_or_default() + 1;
            let mut program = self.workout_detail(&auth, &replacement_id).await?;
            program["idInPlan"] = json!(new_id_in_plan);
            let mut removal = json!({"id":p.scheduled_workout_id,"status":3});
            for key in ["planId", "planProgramId", "labelId"] {
                if !entry[key].is_null() { removal[key] = entry[key].clone(); }
            }
            let add = json!({"entities":[{"happenDay":p.date.replace("-", ""),"idInPlan":new_id_in_plan,"sortNoInSchedule":entry["sortNoInSchedule"].as_i64().unwrap_or_default()}],"programs":[program],"versionObjects":[{"id":new_id_in_plan,"status":1}],"pbVersion":2});
            let remove = json!({"versionObjects":[removal],"pbVersion":2});
            if p.dry_run.unwrap_or(true) {
                Ok(dry("/training/schedule/update", json!({"timezone":timezone,"remove":remove,"add":add})))
            } else if !p.confirm.unwrap_or(false) {
                Err(anyhow!("Set confirm: true with dryRun: false to replace this calendar entry."))
            } else {
                self.post(&auth, "/training/schedule/update", remove).await?;
                if let Err(error) = self.post(&auth, "/training/schedule/update", add).await {
                    return Err(anyhow!("The original entry was removed but adding replacement {replacement_id} failed: {error}"));
                }
                Ok(format!("Scheduled workout {} replaced with {replacement_id} on {} ({timezone}).", p.scheduled_workout_id, p.date))
            }
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
    #[tool(
        description = "Get COROS dashboard data: recovery, HRV, race predictions, and personal records."
    )]
    async fn get_training_dashboard(&self) -> String {
        result(async { Ok(text(self.dashboard(&self.auth().await?).await?)) }).await
    }
    #[tool(
        description = "Summarize the trailing seven days of activities, daily metrics, calendar commitments, and dashboard recovery. endDate defaults to today (UTC)."
    )]
    async fn get_weekly_training_status(
        &self,
        Parameters(p): Parameters<WeeklyTrainingStatus>,
    ) -> String {
        result(async {
            let end = p.end_date.as_deref().map(iso).transpose()?.unwrap_or_else(|| chrono::Utc::now().date_naive());
            let start = end - chrono::Duration::days(6);
            let start_day = start.format("%Y%m%d").to_string().parse::<i64>()?;
            let end_day = end.format("%Y%m%d").to_string().parse::<i64>()?;
            let auth = self.auth().await?;
            let (_, activities) = self.activities(&auth, 1, 50, Some(start_day), Some(end_day), None).await?;
            let metrics = self.daily_metrics(&auth, start_day, end_day).await?;
            let calendar = self.get(&auth, "/training/schedule/query", &[("startDate", start_day.to_string()), ("endDate", end_day.to_string()), ("supportRestExercise", "1".into())]).await?["data"].clone();
            let dashboard = self.dashboard(&auth).await?;
            let duration = activities.iter().map(|a| a["totalTime"].as_i64().unwrap_or_default()).sum::<i64>();
            let distance = activities.iter().map(|a| a["distance"].as_f64().unwrap_or_default()).sum::<f64>();
            let load = activities.iter().map(|a| a["trainingLoad"].as_i64().unwrap_or_default()).sum::<i64>();
            Ok(text(json!({"range":{"startDate":start.to_string(),"endDate":end.to_string()},"completed":{"activities":activities.len(),"durationSeconds":duration,"distanceMeters":distance,"trainingLoad":load},"dailyMetrics":metrics,"scheduled":calendar["entities"],"dashboard":dashboard})))
        }).await
    }
    #[tool(
        description = "Compare two completed activities by duration, distance, training load, heart rate, and available lap data."
    )]
    async fn compare_activities(&self, Parameters(p): Parameters<CompareActivities>) -> String {
        result(async {
            let auth = self.auth().await?;
            let left = self
                .activity_detail(&auth, &p.left_label_id, p.left_sport_type)
                .await?;
            let right = self
                .activity_detail(&auth, &p.right_label_id, p.right_sport_type)
                .await?;
            Ok(text(compare_activity_details(
                &p.left_label_id,
                &left,
                &p.right_label_id,
                &right,
            )))
        })
        .await
    }
    #[tool(
        description = "Preview a calendar label or event (race, test, rest, travel). Live event writes are deliberately unavailable until COROS publishes or a request capture verifies their undocumented contract."
    )]
    async fn preview_calendar_event(
        &self,
        Parameters(p): Parameters<CalendarEventPreview>,
    ) -> String {
        result(async {
            iso(&p.date)?;
            Ok(dry(
                "unverified-calendar-event",
                json!({"date":p.date,"kind":p.kind,"title":p.title,"notes":p.notes}),
            ))
        })
        .await
    }
    #[tool(
        description = "Build a multi-activity session draft for triathlon, duathlon, brick, or run-plus-strength training. It does not pretend to create an unsupported unified COROS multisport workout."
    )]
    async fn build_multisport_session(
        &self,
        Parameters(p): Parameters<MultisportSession>,
    ) -> String {
        result(async { Ok(text(multisport_session_draft(&p)?)) }).await
    }
    #[tool(
        description = "Generate a dry-run, phased race-plan draft from start date to goal date. Review it, then create individual workouts/plans with the write tools."
    )]
    async fn generate_race_plan(&self, Parameters(p): Parameters<RacePlan>) -> String {
        result(async { Ok(dry("race-plan-draft", race_plan_draft(&p)?)) }).await
    }
    #[tool(
        description = "Clone a COROS training plan under a new name. dryRun defaults to true; the original plan is never modified."
    )]
    async fn clone_training_plan(&self, Parameters(p): Parameters<ClonePlan>) -> String {
        result(async {
            let auth = self.auth().await?;
            let mut plan = self
                .get(
                    &auth,
                    "/training/plan/detail",
                    &[
                        ("id", p.plan_id.clone()),
                        ("supportRestExercise", "1".into()),
                    ],
                )
                .await?["data"]
                .clone();
            if plan.is_null() {
                return Err(anyhow!(
                    "COROS returned no training plan detail for {}.",
                    p.plan_id
                ));
            }
            for key in ["id", "userId", "authorId"] {
                plan[key] = json!("0");
            }
            for key in ["createTimestamp", "version"] {
                plan[key] = json!(0);
            }
            plan["name"] = json!(p.name);
            if p.dry_run.unwrap_or(true) {
                Ok(dry("/training/plan/add", plan))
            } else {
                self.post(&auth, "/training/plan/add", plan).await?;
                Ok("Training plan cloned.".into())
            }
        })
        .await
    }
    #[tool(
        description = "Delete a training plan. dryRun defaults to true; a live deletion requires confirm true."
    )]
    async fn delete_training_plan(&self, Parameters(p): Parameters<DeletePlan>) -> String {
        result(async {
            let payload = json!([p.plan_id]);
            if p.dry_run.unwrap_or(true) {
                Ok(dry("/training/plan/delete", payload))
            } else if !p.confirm.unwrap_or(false) {
                Err(anyhow!(
                    "Set confirm: true with dryRun: false to delete this training plan."
                ))
            } else {
                self.post(&self.auth().await?, "/training/plan/delete", payload)
                    .await?;
                Ok("Training plan deleted.".into())
            }
        })
        .await
    }
    #[tool(
        description = "Create a run workout from human-friendly steps such as 4 × 5 min threshold with 2 min easy recovery. Intensity accepts easy/aerobic/tempo/threshold/vo2, rpe:N, hr:LOW-HIGH, or pace:LOW-HIGH seconds/km."
    )]
    async fn create_guided_run_workout(
        &self,
        Parameters(p): Parameters<CreateGuidedWorkout>,
    ) -> String {
        result(async {
            let payload = guided_workout_payload(&p, 1)?;
            self.create_program(&self.auth().await?, payload).await?;
            Ok(format!("Guided run workout \"{}\" created.", p.name))
        })
        .await
    }
    #[tool(
        description = "Create a bike workout from human-friendly steps such as 3 × 10 min tempo. Intensity accepts easy/aerobic/tempo/threshold/vo2, rpe:N, hr:LOW-HIGH, or pace:LOW-HIGH seconds/km."
    )]
    async fn create_guided_bike_workout(
        &self,
        Parameters(p): Parameters<CreateGuidedWorkout>,
    ) -> String {
        result(async {
            let payload = guided_workout_payload(&p, 2)?;
            self.create_program(&self.auth().await?, payload).await?;
            Ok(format!("Guided bike workout \"{}\" created.", p.name))
        })
        .await
    }
    #[tool(
        description = "Move a scheduled workout to another date. dryRun defaults to true; a live move adds the destination first and requires confirm true."
    )]
    async fn reschedule_workout(&self, Parameters(p): Parameters<RescheduleWorkout>) -> String {
        result(async {
            iso(&p.from_date)?; iso(&p.to_date)?;
            if p.from_date == p.to_date { return Err(anyhow!("fromDate and toDate must differ.")); }
            let auth = self.auth().await?;
            let source = self.get(&auth, "/training/schedule/query", &[("startDate", p.from_date.replace("-", "")), ("endDate", p.from_date.replace("-", "")), ("supportRestExercise", "1".into())]).await?["data"].clone();
            let entry = source["entities"].as_array().and_then(|items| items.iter().find(|item| item["idInPlan"].as_i64() == Some(p.scheduled_workout_id))).ok_or_else(|| anyhow!("No scheduled workout with that id exists on {}.", p.from_date))?;
            let program = source["programs"].as_array().and_then(|items| items.iter().find(|item| item["idInPlan"].as_i64() == Some(p.scheduled_workout_id))).cloned().ok_or_else(|| anyhow!("COROS calendar response did not include the scheduled workout program."))?;
            let destination = self.get(&auth, "/training/schedule/query", &[("startDate", p.to_date.replace("-", "")), ("endDate", p.to_date.replace("-", "")), ("supportRestExercise", "1".into())]).await?["data"].clone();
            let new_id = destination["maxIdInPlan"].as_i64().unwrap_or_default() + 1;
            let mut add_program = program; add_program["idInPlan"] = json!(new_id);
            let add = json!({"entities":[{"happenDay":p.to_date.replace("-", ""),"idInPlan":new_id,"sortNoInSchedule":destination["entities"].as_array().map_or(0, Vec::len)}],"programs":[add_program],"versionObjects":[{"id":new_id,"status":1}],"pbVersion":2});
            let mut removal = json!({"id":p.scheduled_workout_id,"status":3});
            for key in ["planId","planProgramId","labelId"] { if !entry[key].is_null() { removal[key] = entry[key].clone(); } }
            let remove = json!({"versionObjects":[removal],"pbVersion":2});
            if p.dry_run.unwrap_or(true) { Ok(dry("/training/schedule/update", json!({"addFirst":add,"thenRemove":remove}))) }
            else if !p.confirm.unwrap_or(false) { Err(anyhow!("Set confirm: true with dryRun: false to reschedule this workout.")) }
            else { self.post(&auth, "/training/schedule/update", add).await?; if let Err(error) = self.post(&auth, "/training/schedule/update", remove).await { return Err(anyhow!("Destination was added but source removal failed: {error}")); } Ok("Scheduled workout rescheduled.".into()) }
        }).await
    }
    #[tool(
        description = "Delete a workout from the COROS library. dryRun defaults to true; a live deletion requires confirm true."
    )]
    async fn delete_workout(&self, Parameters(p): Parameters<DeleteWorkout>) -> String {
        result(async {
            let payload = json!([p.workout_id]);
            if p.dry_run.unwrap_or(true) {
                Ok(dry("/training/program/delete", payload))
            } else if !p.confirm.unwrap_or(false) {
                Err(anyhow!(
                    "Set confirm: true with dryRun: false to delete this workout."
                ))
            } else {
                self.post(&self.auth().await?, "/training/program/delete", payload)
                    .await?;
                Ok("Workout deleted.".into())
            }
        })
        .await
    }
    #[tool(
        description = "Get a concise COROS performance dashboard with recovery, HRV, personal records, and race predictions."
    )]
    async fn get_performance_dashboard(&self) -> String {
        result(async {
            Ok(text(performance_dashboard(
                &self.dashboard(&self.auth().await?).await?,
            )))
        })
        .await
    }
    #[tool(
        description = "Compare scheduled calendar sessions with completed activities and suggest a conservative next-week adjustment. This is read-only."
    )]
    async fn get_plan_adherence(&self, Parameters(p): Parameters<Adherence>) -> String {
        result(async {
            let start = iso(&p.start_date)?;
            let end = iso(&p.end_date)?;
            if start > end {
                return Err(anyhow!("startDate must be on or before endDate."));
            }
            let auth = self.auth().await?;
            let start_day = start.format("%Y%m%d").to_string().parse::<i64>()?;
            let end_day = end.format("%Y%m%d").to_string().parse::<i64>()?;
            let (_, activities) = self
                .activities(&auth, 1, 50, Some(start_day), Some(end_day), None)
                .await?;
            let calendar = self
                .get(
                    &auth,
                    "/training/schedule/query",
                    &[
                        ("startDate", start_day.to_string()),
                        ("endDate", end_day.to_string()),
                        ("supportRestExercise", "1".into()),
                    ],
                )
                .await?["data"]
                .clone();
            Ok(text(plan_adherence_summary(
                start_day,
                end_day,
                &calendar,
                &activities,
            )))
        })
        .await
    }
    #[tool(
        description = "Record a local post-workout RPE journal entry (1-10) and optional notes. It never edits COROS activity records."
    )]
    async fn record_training_journal(&self, Parameters(p): Parameters<JournalEntry>) -> String {
        result(async {
            iso(&p.date)?;
            if !(1..=10).contains(&p.rpe) {
                return Err(anyhow!("rpe must be an integer from 1 to 10."));
            }
            let path = Self::auth_path()?.with_file_name("journal.json");
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut entries: Vec<Value> = fs::read_to_string(&path)
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default();
            entries.push(json!({"date":p.date,"rpe":p.rpe,"notes":p.notes,"labelId":p.label_id}));
            fs::write(&path, serde_json::to_vec_pretty(&entries)?)?;
            Ok("Training journal entry recorded locally.".into())
        })
        .await
    }
    #[tool(description = "List locally recorded post-workout RPE journal entries.")]
    async fn list_training_journal(&self) -> String {
        result(async {
            let path = Self::auth_path()?.with_file_name("journal.json");
            let entries: Value = fs::read_to_string(path)
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_else(|| json!([]));
            Ok(text(entries))
        })
        .await
    }
}

pub(crate) fn endurance_workout_payload(
    p: &CreateEnduranceWorkout,
    sport_type: i64,
) -> anyhow::Result<Value> {
    if p.steps.is_empty() {
        return Err(anyhow!("At least one step is required."));
    }
    let mut total_seconds = 0_i64;
    let mut total_distance = 0_i64;
    let exercises = p.steps.iter().enumerate().map(|(index, step)| {
        let kind = code(&[("warmup", 1), ("training", 2), ("rest", 3), ("cooldown", 4)], &step.kind)
            .ok_or_else(|| anyhow!("Step {} has unknown kind \"{}\". Use warmup, training, rest, or cooldown.", index + 1, step.kind))?;
        let has_duration = step.duration_seconds.is_some();
        let has_distance = step.distance_meters.is_some();
        if has_duration == has_distance { return Err(anyhow!("Step {} needs exactly one of durationSeconds or distanceMeters.", index + 1)); }
        let (target_type, target_value, target_display_unit) = if let Some(seconds) = step.duration_seconds {
            if seconds <= 0 { return Err(anyhow!("Step {} durationSeconds must be positive.", index + 1)); }
            total_seconds += seconds;
            (2, seconds, 0)
        } else {
            let meters = step.distance_meters.unwrap_or_default();
            if !meters.is_finite() || meters <= 0.0 { return Err(anyhow!("Step {} distanceMeters must be positive.", index + 1)); }
            let value = (meters * 100.0).round() as i64;
            total_distance += value;
            (5, value, 3)
        };
        Ok(json!({
            "access":0,"defaultOrder":index,"id":index + 1,"sortNo":index + 1,"name":step.name.clone().unwrap_or_else(|| step.kind.clone()),
            "nameText":step.name.clone().unwrap_or_else(|| step.kind.clone()),"overview":"","originId":"0","groupId":"","isGroup":false,"isIntensityPercent":false,
            "sportType":sport_type,"exerciseType":kind,"sets":1,"targetType":target_type,"targetValue":target_value,"targetDisplayUnit":target_display_unit,
            "intensityType":step.intensity_type.unwrap_or(0),"intensityValue":step.intensity_value.unwrap_or(0),"intensityValueExtend":step.intensity_value_extend.unwrap_or(0),"intensityDisplayUnit":step.intensity_display_unit.unwrap_or(0),
            "restType":3,"restValue":0,"hrType":3
        }))
    }).collect::<anyhow::Result<Vec<_>>>()?;
    Ok(json!({
        "access":1,"authorId":"0","createTimestamp":0,"distance":total_distance,"duration":total_seconds,"essence":0,"estimatedType":0,"estimatedValue":0,"exerciseNum":exercises.len(),"exercises":exercises,
        "headPic":"","id":"0","idInPlan":"0","name":p.name,"nickname":"","originEssence":0,"overview":p.overview.clone().unwrap_or_default(),"pbVersion":2,"planIdIndex":0,"poolLength":2500,"profile":"",
        "referExercise":{"intensityType":0,"hrType":3,"valueType":1},"sex":0,"shareUrl":"","simple":false,"sourceUrl":DEFAULT_SOURCE_URL,"sportType":sport_type,"star":0,"subType":65535,"targetType":0,"targetValue":0,"thirdPartyId":0,"totalSets":0,"trainingLoad":0,"type":0,"unit":0,"userId":"0","version":0,"videoCoverUrl":"","videoUrl":"","poolLengthId":1,"poolLengthUnit":2,"sourceId":"425868133463670784"
    }))
}

pub(crate) fn clone_workout_payload(
    mut workout: Value,
    update: &UpdateWorkout,
) -> anyhow::Result<Value> {
    let exercises = workout["exercises"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("COROS workout detail did not include editable steps."))?;
    for patch in &update.step_updates {
        let step = exercises
            .get_mut(patch.index)
            .ok_or_else(|| anyhow!("Step index {} is outside this workout.", patch.index))?;
        for (key, value) in [
            ("name", patch.name.clone().map(Value::String)),
            ("nameText", patch.name.clone().map(Value::String)),
            ("targetType", patch.target_type.map(|value| json!(value))),
            ("targetValue", patch.target_value.map(|value| json!(value))),
            (
                "intensityType",
                patch.intensity_type.map(|value| json!(value)),
            ),
            (
                "intensityValue",
                patch.intensity_value.map(|value| json!(value)),
            ),
            (
                "intensityValueExtend",
                patch.intensity_value_extend.map(|value| json!(value)),
            ),
            (
                "intensityDisplayUnit",
                patch.intensity_display_unit.map(|value| json!(value)),
            ),
        ] {
            if let Some(value) = value {
                step[key] = value;
            }
        }
    }
    if let Some(name) = &update.name {
        workout["name"] = json!(name);
    }
    for key in ["id", "idInPlan", "authorId", "userId"] {
        workout[key] = json!("0");
    }
    for key in ["createTimestamp", "version", "star", "planIdIndex"] {
        workout[key] = json!(0);
    }
    workout["sourceUrl"] = json!(DEFAULT_SOURCE_URL);
    Ok(workout)
}

fn compare_activity_details(left_id: &str, left: &Value, right_id: &str, right: &Value) -> Value {
    let metric = |detail: &Value, key: &str| {
        detail["summary"][key]
            .as_f64()
            .or_else(|| detail["summary"][key].as_i64().map(|value| value as f64))
            .unwrap_or_default()
    };
    let summary = |id: &str, detail: &Value| {
        json!({
            "labelId": id,
            "duration": metric(detail, "totalTime"),
            "distance": metric(detail, "distance"),
            "trainingLoad": metric(detail, "trainingLoad"),
            "avgHr": metric(detail, "avgHr"),
            "calories": metric(detail, "calories"),
            "lapCount": detail["lapList"].as_array().map_or(0, Vec::len),
        })
    };
    let left_summary = summary(left_id, left);
    let right_summary = summary(right_id, right);
    json!({
        "left": left_summary,
        "right": right_summary,
        "rightMinusLeft": {
            "duration": metric(right, "totalTime") - metric(left, "totalTime"),
            "distance": metric(right, "distance") - metric(left, "distance"),
            "trainingLoad": metric(right, "trainingLoad") - metric(left, "trainingLoad"),
            "avgHr": metric(right, "avgHr") - metric(left, "avgHr"),
        },
        "leftLaps": left["lapList"],
        "rightLaps": right["lapList"],
    })
}

pub(crate) fn multisport_session_draft(session: &MultisportSession) -> anyhow::Result<Value> {
    if session.legs.len() < 2 {
        return Err(anyhow!("A multisport session needs at least two legs."));
    }
    let legs = session.legs.iter().enumerate().map(|(index, leg)| {
        if leg.sport.trim().is_empty() { return Err(anyhow!("Leg {} needs a sport name.", index + 1)); }
        if leg.duration_seconds.is_some_and(|value| value <= 0) || leg.distance_meters.is_some_and(|value| !value.is_finite() || value <= 0.0) {
            return Err(anyhow!("Leg {} durationSeconds/distanceMeters must be positive.", index + 1));
        }
        if leg.duration_seconds.is_none() && leg.distance_meters.is_none() && !leg.sport.eq_ignore_ascii_case("transition") {
            return Err(anyhow!("Leg {} needs durationSeconds or distanceMeters.", index + 1));
        }
        Ok(json!({"order":index + 1,"sport":leg.sport,"durationSeconds":leg.duration_seconds,"distanceMeters":leg.distance_meters,"notes":leg.notes}))
    }).collect::<anyhow::Result<Vec<_>>>()?;
    Ok(
        json!({"name":session.name,"notes":session.notes,"legs":legs,"nextSteps":"Create each supported run/bike/strength leg with its dedicated COROS tool, then schedule them in sequence. COROS has no verified unified multisport structured-workout write contract."}),
    )
}

pub(crate) fn race_plan_draft(plan: &RacePlan) -> anyhow::Result<Value> {
    let goal = iso(&plan.goal_date)?;
    let start = plan
        .start_date
        .as_deref()
        .map(iso)
        .transpose()?
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    if goal <= start {
        return Err(anyhow!("goalDate must be after startDate."));
    }
    let days = plan.days_per_week.unwrap_or(4).clamp(2, 7) as usize;
    let weeks = ((goal - start).num_days() + 6) / 7;
    let schedule = (0..weeks).map(|week| {
        let fraction = (week + 1) as f64 / weeks.max(1) as f64;
        let phase = if fraction < 0.45 { "Base" } else if fraction < 0.75 { "Build" } else if fraction < 0.9 { "Peak" } else { "Taper/Race" };
        let sessions = (0..days).map(|day| match day {
            0 => "easy aerobic",
            1 => "quality intervals or tempo",
            2 if days >= 4 => "recovery or strength",
            x if x + 1 == days => "long endurance",
            _ => "easy aerobic",
        }).collect::<Vec<_>>();
        json!({"week":week + 1,"starts":(start + chrono::Duration::days(week * 7)).to_string(),"phase":phase,"sessions":sessions})
    }).collect::<Vec<_>>();
    Ok(
        json!({"eventName":plan.event_name,"startDate":start.to_string(),"goalDate":goal.to_string(),"daysPerWeek":days,"weeks":schedule,"reviewRequired":"This is a generic progression template, not medical or individualized coaching advice. Review volume, intensity, and recovery before creating workouts."}),
    )
}

pub(crate) fn guided_workout_payload(
    workout: &CreateGuidedWorkout,
    sport_type: i64,
) -> anyhow::Result<Value> {
    if workout.steps.is_empty() {
        return Err(anyhow!("At least one step is required."));
    }
    let mut steps = Vec::new();
    let mut intensities = Vec::new();
    for (index, step) in workout.steps.iter().enumerate() {
        let repeat = step.repeat.unwrap_or(1);
        if !(1..=99).contains(&repeat) {
            return Err(anyhow!(
                "Step {} repeat must be between 1 and 99.",
                index + 1
            ));
        }
        for _ in 0..repeat {
            steps.push(EnduranceStep {
                kind: step.kind.clone(),
                name: None,
                duration_seconds: step.duration_seconds,
                distance_meters: step.distance_meters,
                intensity_type: None,
                intensity_value: None,
                intensity_value_extend: None,
                intensity_display_unit: None,
            });
            intensities.push(step.intensity.clone());
        }
    }
    let mut payload = endurance_workout_payload(
        &CreateEnduranceWorkout {
            name: workout.name.clone(),
            overview: workout.overview.clone(),
            steps,
        },
        sport_type,
    )?;
    for (exercise, intensity) in payload["exercises"]
        .as_array_mut()
        .unwrap_or(&mut Vec::new())
        .iter_mut()
        .zip(intensities)
    {
        if let Some(intensity) = intensity {
            apply_guided_intensity(exercise, &intensity)?;
        }
    }
    Ok(payload)
}

fn apply_guided_intensity(exercise: &mut Value, target: &str) -> anyhow::Result<()> {
    let normalized = target.trim().to_ascii_lowercase();
    let rpe = match normalized.as_str() {
        "easy" => Some(3),
        "aerobic" => Some(4),
        "tempo" => Some(6),
        "threshold" => Some(7),
        "vo2" => Some(9),
        _ => None,
    };
    if let Some(rpe) = rpe {
        exercise["intensityType"] = json!(11);
        exercise["intensityValue"] = json!(rpe);
        return Ok(());
    }
    if let Some(value) = normalized.strip_prefix("rpe:") {
        let rpe: i64 = value
            .trim()
            .parse()
            .map_err(|_| anyhow!("Use rpe:N where N is 1-10."))?;
        if !(1..=10).contains(&rpe) {
            return Err(anyhow!("RPE must be 1-10."));
        }
        exercise["intensityType"] = json!(11);
        exercise["intensityValue"] = json!(rpe);
        return Ok(());
    }
    let parse_range = |value: &str| -> anyhow::Result<(i64, i64)> {
        let (low, high) = value
            .split_once('-')
            .ok_or_else(|| anyhow!("Use a LOW-HIGH range."))?;
        let low = low.trim().parse()?;
        let high = high.trim().parse()?;
        if low <= 0 || high < low {
            return Err(anyhow!("Intensity range must be positive and ascending."));
        }
        Ok((low, high))
    };
    if let Some(value) = normalized.strip_prefix("hr:") {
        let (low, high) = parse_range(value)?;
        exercise["intensityType"] = json!(2);
        exercise["hrType"] = json!(2);
        exercise["intensityValue"] = json!(low);
        exercise["intensityValueExtend"] = json!(high);
        return Ok(());
    }
    if let Some(value) = normalized.strip_prefix("pace:") {
        let (low, high) = parse_range(value)?;
        exercise["intensityType"] = json!(3);
        exercise["intensityValue"] = json!(low * 1000);
        exercise["intensityValueExtend"] = json!(high * 1000);
        exercise["intensityDisplayUnit"] = json!(1);
        exercise["intensityMultiplier"] = json!(1000);
        return Ok(());
    }
    Err(anyhow!(
        "Unknown intensity \"{target}\". Use easy, aerobic, tempo, threshold, vo2, rpe:N, hr:LOW-HIGH, or pace:LOW-HIGH."
    ))
}

fn performance_dashboard(raw: &Value) -> Value {
    let summary = &raw["summaryInfo"];
    json!({"recoveryPercent":summary["recoveryPct"],"recoveryState":summary["recoveryState"],"fullRecoveryHours":summary["fullRecoveryHours"],"restingHr":summary["rhr"],"sleepHrv":summary["sleepHrvData"],"racePredictor":summary["racePredictor"],"personalRecords":summary["recordDetailList"]})
}

pub(crate) fn plan_adherence_summary(
    start_day: i64,
    end_day: i64,
    calendar: &Value,
    activities: &[Value],
) -> Value {
    let planned: Vec<_> = calendar["entities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry["happenDay"].as_i64())
        .collect();
    let completed: Vec<_> = activities
        .iter()
        .filter_map(|activity| activity["date"].as_i64())
        .collect();
    let missed: Vec<_> = planned
        .iter()
        .filter(|day| !completed.contains(day))
        .copied()
        .collect();
    let adherence = if planned.is_empty() {
        None
    } else {
        Some((planned.len() - missed.len()) as f64 / planned.len() as f64)
    };
    let suggestion = match adherence {
        Some(value) if value < 0.5 => {
            "Reduce next week to essential sessions and restore consistency."
        }
        Some(value) if value < 0.8 => {
            "Keep volume steady; reschedule only the highest-priority missed session."
        }
        Some(_) => "Adherence is strong; progress conservatively if recovery is normal.",
        None => "No scheduled workouts in this range; create a plan before measuring adherence.",
    };
    json!({"range":{"startDay":start_day,"endDay":end_day},"plannedDays":planned,"completedDays":completed,"missedPlannedDays":missed,"adherence":adherence,"suggestedNextWeekAdjustment":suggestion})
}
