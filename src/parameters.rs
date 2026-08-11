use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub(crate) struct Authenticate {
    pub(crate) email: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) region: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct Search {
    pub(crate) query: Option<String>,
    pub(crate) muscle: Option<String>,
    #[serde(rename = "bodyPart")]
    pub(crate) body_part: Option<String>,
    pub(crate) equipment: Option<String>,
    pub(crate) limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Exercise {
    pub(crate) name: String,
    pub(crate) sets: Option<i64>,
    pub(crate) reps: Option<i64>,
    pub(crate) duration: Option<i64>,
    pub(crate) rest_seconds: Option<i64>,
    pub(crate) weight_kg: Option<f64>,
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct CreateWorkout {
    pub(crate) name: String,
    pub(crate) overview: Option<String>,
    pub(crate) exercises: Vec<Exercise>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateExercises {
    pub(crate) sport_type: Option<i64>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListWorkouts {
    pub(crate) name: Option<String>,
    pub(crate) sport_type: Option<i64>,
    pub(crate) limit: Option<i64>,
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct Status {
    pub(crate) status: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanId {
    pub(crate) plan_id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanWorkout {
    pub(crate) workout_id: Option<String>,
    pub(crate) workout_name: Option<String>,
    pub(crate) weekday: Option<String>,
    pub(crate) date: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
pub(crate) struct PlanWeek {
    pub(crate) workouts: Vec<PlanWorkout>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePlan {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) start_date: Option<String>,
    pub(crate) weeks: Vec<PlanWeek>,
    pub(crate) dry_run: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Calendar {
    pub(crate) start_date: String,
    pub(crate) end_date: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Schedule {
    pub(crate) workout_id: Option<String>,
    pub(crate) workout_name: Option<String>,
    pub(crate) date: String,
    pub(crate) timezone: Option<String>,
    pub(crate) allow_existing_entries: Option<bool>,
    pub(crate) dry_run: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Remove {
    pub(crate) date: String,
    pub(crate) scheduled_workout_id: i64,
    pub(crate) confirm: Option<bool>,
    pub(crate) dry_run: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Custom {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) body_part: String,
    pub(crate) primary_muscle: Option<String>,
    pub(crate) equipment: Option<String>,
    pub(crate) dry_run: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListActivities {
    pub(crate) start_date: Option<i64>,
    pub(crate) sport_types: Option<Vec<i64>>,
    pub(crate) end_date: Option<i64>,
    pub(crate) limit: Option<i64>,
    pub(crate) page_number: Option<i64>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportActivityFile {
    pub(crate) label_id: String,
    pub(crate) sport_type: i64,
    pub(crate) file_type: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DailyMetrics {
    pub(crate) start_date: i64,
    pub(crate) end_date: i64,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivityDetail {
    pub(crate) label_id: String,
    pub(crate) sport_type: Option<i64>,
}
