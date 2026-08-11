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
pub(crate) struct EnduranceStep {
    /// warmup, training, rest, or cooldown.
    pub(crate) kind: String,
    pub(crate) name: Option<String>,
    /// One of durationSeconds or distanceMeters is required.
    pub(crate) duration_seconds: Option<i64>,
    pub(crate) distance_meters: Option<f64>,
    /// COROS intensity mode; omit for an open-ended step.
    pub(crate) intensity_type: Option<i64>,
    pub(crate) intensity_value: Option<i64>,
    pub(crate) intensity_value_extend: Option<i64>,
    pub(crate) intensity_display_unit: Option<i64>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateEnduranceWorkout {
    pub(crate) name: String,
    pub(crate) overview: Option<String>,
    pub(crate) steps: Vec<EnduranceStep>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkoutStepUpdate {
    /// Zero-based exercise/step index in the workout detail.
    pub(crate) index: usize,
    pub(crate) name: Option<String>,
    pub(crate) target_type: Option<i64>,
    pub(crate) target_value: Option<i64>,
    pub(crate) intensity_type: Option<i64>,
    pub(crate) intensity_value: Option<i64>,
    pub(crate) intensity_value_extend: Option<i64>,
    pub(crate) intensity_display_unit: Option<i64>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateWorkout {
    pub(crate) workout_id: String,
    pub(crate) name: Option<String>,
    pub(crate) step_updates: Vec<WorkoutStepUpdate>,
    pub(crate) dry_run: Option<bool>,
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
pub(crate) struct ReplaceScheduledWorkout {
    pub(crate) date: String,
    pub(crate) scheduled_workout_id: i64,
    pub(crate) replacement_workout_id: Option<String>,
    pub(crate) replacement_workout_name: Option<String>,
    pub(crate) timezone: Option<String>,
    pub(crate) dry_run: Option<bool>,
    pub(crate) confirm: Option<bool>,
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

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeeklyTrainingStatus {
    /// Week ending on this ISO date; defaults to today in UTC.
    pub(crate) end_date: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompareActivities {
    pub(crate) left_label_id: String,
    pub(crate) left_sport_type: i64,
    pub(crate) right_label_id: String,
    pub(crate) right_sport_type: i64,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalendarEventPreview {
    pub(crate) date: String,
    /// race, test, rest, travel, or a custom label.
    pub(crate) kind: String,
    pub(crate) title: String,
    pub(crate) notes: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MultisportLeg {
    /// For example run, bike, transition, or strength.
    pub(crate) sport: String,
    pub(crate) duration_seconds: Option<i64>,
    pub(crate) distance_meters: Option<f64>,
    pub(crate) notes: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MultisportSession {
    pub(crate) name: String,
    pub(crate) legs: Vec<MultisportLeg>,
    pub(crate) notes: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RacePlan {
    pub(crate) event_name: String,
    pub(crate) goal_date: String,
    pub(crate) start_date: Option<String>,
    /// 2 through 7; defaults to 4.
    pub(crate) days_per_week: Option<i64>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClonePlan {
    pub(crate) plan_id: String,
    pub(crate) name: String,
    pub(crate) dry_run: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeletePlan {
    pub(crate) plan_id: String,
    pub(crate) confirm: Option<bool>,
    pub(crate) dry_run: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuidedStep {
    pub(crate) kind: String,
    pub(crate) duration_seconds: Option<i64>,
    pub(crate) distance_meters: Option<f64>,
    /// easy, aerobic, tempo, threshold, vo2, rpe:N, hr:LOW-HIGH, or pace:LOW-HIGH seconds/km.
    pub(crate) intensity: Option<String>,
    pub(crate) repeat: Option<i64>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateGuidedWorkout {
    pub(crate) name: String,
    pub(crate) overview: Option<String>,
    pub(crate) steps: Vec<GuidedStep>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RescheduleWorkout {
    pub(crate) from_date: String,
    pub(crate) to_date: String,
    pub(crate) scheduled_workout_id: i64,
    pub(crate) dry_run: Option<bool>,
    pub(crate) confirm: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteWorkout {
    pub(crate) workout_id: String,
    pub(crate) dry_run: Option<bool>,
    pub(crate) confirm: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Adherence {
    pub(crate) start_date: String,
    pub(crate) end_date: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JournalEntry {
    pub(crate) date: String,
    pub(crate) rpe: i64,
    pub(crate) notes: Option<String>,
    pub(crate) label_id: Option<String>,
}
