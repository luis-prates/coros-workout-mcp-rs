use crate::tools::CorosServer;
use anyhow::{Context, Result, anyhow};
use directories::BaseDirs;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{env, fs, path::PathBuf};

const EU_URL: &str = "https://teameuapi.coros.com";
const US_URL: &str = "https://teamapi.coros.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthData {
    pub(crate) access_token: String,
    pub(crate) user_id: Value,
    pub(crate) region: String,
    pub(crate) timestamp: i64,
}

impl CorosServer {
    pub(crate) fn catalog_path() -> PathBuf {
        env::var_os("COROS_CATALOG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/exercises.json")
            })
    }
    pub(crate) fn auth_path() -> Result<PathBuf> {
        Ok(BaseDirs::new()
            .ok_or_else(|| anyhow!("Could not find home config directory"))?
            .config_dir()
            .join("coros-workout-mcp/auth.json"))
    }
    pub(crate) fn load_auth() -> Result<Option<AuthData>> {
        let path = Self::auth_path()?;
        match fs::read_to_string(path) {
            Ok(text) => Ok(Some(serde_json::from_str(&text)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    pub(crate) fn store_auth(auth: &AuthData) -> Result<()> {
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
    pub(crate) fn base_url(region: &str) -> Result<&'static str> {
        match region {
            "eu" => Ok(EU_URL),
            "us" => Ok(US_URL),
            _ => Err(anyhow!("region must be 'eu' or 'us'")),
        }
    }
    pub(crate) fn catalog() -> Result<Vec<Value>> {
        Ok(serde_json::from_slice(&fs::read(Self::catalog_path())?)?)
    }
    pub(crate) async fn login(
        &self,
        email: String,
        password: String,
        region: String,
    ) -> Result<AuthData> {
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
    pub(crate) async fn auth(&self) -> Result<AuthData> {
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
    pub(crate) fn headers(auth: &AuthData) -> reqwest::header::HeaderMap {
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
    pub(crate) async fn post(&self, auth: &AuthData, path: &str, body: Value) -> Result<Value> {
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
    pub(crate) async fn get(
        &self,
        auth: &AuthData,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<Value> {
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
    pub(crate) async fn workouts(
        &self,
        auth: &AuthData,
        name: &str,
        sport_type: i64,
        limit: i64,
    ) -> Result<Vec<Value>> {
        Ok(self.post(auth,"/training/program/query",json!({"name":name,"supportRestExercise":1,"startNo":0,"limitSize":limit,"sportType":sport_type})).await?["data"].as_array().cloned().unwrap_or_default())
    }
    pub(crate) async fn workout_detail(&self, auth: &AuthData, id: &str) -> Result<Value> {
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
    pub(crate) async fn resolve_workout(
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
    pub(crate) async fn activities(
        &self,
        auth: &AuthData,
        page_number: i64,
        size: i64,
        start_date: Option<i64>,
        end_date: Option<i64>,
    ) -> Result<(i64, Vec<Value>)> {
        // COROS ignores date filters server-side, so also filter this page locally.
        let mut params = vec![
            ("pageNumber", page_number.to_string()),
            ("size", size.to_string()),
        ];
        if let Some(date) = start_date {
            params.push(("startDate", date.to_string()));
        }
        if let Some(date) = end_date {
            params.push(("endDate", date.to_string()));
        }
        let data = self.get(auth, "/activity/query", &params).await?["data"].clone();
        let count = data["count"].as_i64().unwrap_or_default();
        let activities = data["dataList"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|activity| {
                let date = activity["date"].as_i64().unwrap_or_default();
                start_date.is_none_or(|start| date >= start)
                    && end_date.is_none_or(|end| date <= end)
            })
            .collect();
        Ok((count, activities))
    }
    pub(crate) async fn activity_detail(
        &self,
        auth: &AuthData,
        label_id: &str,
        sport_type: i64,
    ) -> Result<Value> {
        use reqwest::header::{CONTENT_TYPE, HeaderValue};
        let sport_type = sport_type.to_string();
        let mut headers = Self::headers(auth);
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        let response = self
            .client
            .post(format!(
                "{}/activity/detail/query",
                Self::base_url(&auth.region)?
            ))
            .headers(headers)
            .query(&[
                ("screenW", "565"),
                ("screenH", "982"),
                ("labelId", label_id),
                ("sportType", &sport_type),
            ])
            .body("")
            .send()
            .await?;
        let data = response
            .json::<Value>()
            .await
            .context("COROS returned a non-JSON response")?;
        if data["result"] != "0000" {
            return Err(anyhow!(
                "COROS API error (/activity/detail/query): {}",
                data["message"]
                    .as_str()
                    .unwrap_or_else(|| data["result"].as_str().unwrap_or("unknown"))
            ));
        }
        Ok(data["data"].clone())
    }
}
trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}
