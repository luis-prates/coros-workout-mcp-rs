#![recursion_limit = "256"]
#![allow(clippy::possible_missing_else)]

mod tools;

use anyhow::Result;
use rmcp::{
    ServerHandler, ServiceExt,
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool_handler,
};

mod coros_api;
mod parameters;
mod presentation;

const DEFAULT_SOURCE_URL: &str = "https://d31oxp44ddzkyk.cloudfront.net/source/source_default/0/2fbd46e17bc54bc5873415c9fa767bdc.jpg";
use tools::CorosServer;

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
    use crate::{presentation::*, tools::CorosServer};
    use serde_json::json;

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

    #[test]
    fn strength_activity_summary_uses_actual_weights_and_skips_rest() {
        let catalog = vec![json!({"codeName": "T1041", "name": "Bench Press"})];
        let detail = json!({"lapList": [{"lapItemList": [
            {"exerciseIndex": 1, "exerciseNameKey": "T1041", "mode": 14, "weight": 60000},
            {"exerciseIndex": 1, "exerciseNameKey": "S3618", "mode": 15},
            {"exerciseIndex": 1, "exerciseNameKey": "T1041", "mode": 14, "weight": 60000},
            {"exerciseIndex": 1, "exerciseNameKey": "T1041", "mode": 16, "sets": 2, "reps": 16}
        ]}]});
        assert_eq!(
            summarize_strength_activity(&catalog, &detail),
            "  1. Bench Press — 2×8 (16 reps total) @ 60kg"
        );
    }

    #[test]
    fn activity_query_uses_documented_filters() {
        let params = crate::coros_api::activity_query_params(
            2,
            50,
            Some(20260801),
            Some(20260811),
            Some(&[100, 402]),
        );
        assert!(params.contains(&("startDay", "20260801".into())));
        assert!(params.contains(&("endDay", "20260811".into())));
        assert!(params.contains(&("modeList", "100,402".into())));
        assert!(
            !params
                .iter()
                .any(|(name, _)| *name == "startDate" || *name == "endDate")
        );
    }

    #[test]
    fn activity_exports_accept_documented_formats() {
        assert_eq!(crate::coros_api::activity_file_type_code("GPX").unwrap(), 1);
        assert_eq!(crate::coros_api::activity_file_type_code("fit").unwrap(), 4);
        assert!(crate::coros_api::activity_file_type_code("zip").is_err());
    }

    #[test]
    fn endurance_payload_has_valid_targets_and_sport_type() {
        let workout = crate::parameters::CreateEnduranceWorkout {
            name: "Intervals".into(),
            overview: None,
            steps: vec![
                crate::parameters::EnduranceStep {
                    kind: "warmup".into(),
                    name: None,
                    duration_seconds: Some(600),
                    distance_meters: None,
                    intensity_type: None,
                    intensity_value: None,
                    intensity_value_extend: None,
                    intensity_display_unit: None,
                },
                crate::parameters::EnduranceStep {
                    kind: "training".into(),
                    name: Some("400m".into()),
                    duration_seconds: None,
                    distance_meters: Some(400.0),
                    intensity_type: Some(1),
                    intensity_value: Some(300),
                    intensity_value_extend: None,
                    intensity_display_unit: Some(0),
                },
            ],
        };
        let payload = crate::tools::endurance_workout_payload(&workout, 1).unwrap();
        assert_eq!(payload["sportType"], 1);
        assert_eq!(payload["exercises"][0]["targetValue"], 600);
        assert_eq!(payload["exercises"][1]["targetValue"], 40_000);
    }

    #[test]
    fn workout_patch_clones_and_preserves_original_identity() {
        let original =
            json!({"id":"123","name":"Original","exercises":[{"name":"Easy","targetValue":300}]});
        let update = crate::parameters::UpdateWorkout {
            workout_id: "123".into(),
            name: Some("Edited".into()),
            dry_run: Some(true),
            step_updates: vec![crate::parameters::WorkoutStepUpdate {
                index: 0,
                name: Some("Hard".into()),
                target_type: None,
                target_value: Some(600),
                intensity_type: None,
                intensity_value: None,
                intensity_value_extend: None,
                intensity_display_unit: None,
            }],
        };
        let clone = crate::tools::clone_workout_payload(original, &update).unwrap();
        assert_eq!(clone["id"], "0");
        assert_eq!(clone["name"], "Edited");
        assert_eq!(clone["exercises"][0]["name"], "Hard");
        assert_eq!(clone["exercises"][0]["targetValue"], 600);
    }

    #[test]
    fn generic_activity_detail_keeps_laps_and_zones() {
        let overview = activity_detail_overview(&json!({
            "summary":{"avgHr":145}, "lapList":[{"lapDistance":1000}], "hrZoneList":[{"zone":3}], "powerZoneList":[{"zone":4}]
        }));
        assert_eq!(overview["summary"]["avgHr"], 145);
        assert_eq!(overview["laps"][0]["lapDistance"], 1000);
        assert_eq!(overview["heartRateZones"][0]["zone"], 3);
    }

    #[test]
    fn multisport_draft_requires_multiple_legs() {
        let session = crate::parameters::MultisportSession {
            name: "Brick".into(),
            notes: None,
            legs: vec![
                crate::parameters::MultisportLeg {
                    sport: "Bike".into(),
                    duration_seconds: Some(3600),
                    distance_meters: None,
                    notes: None,
                },
                crate::parameters::MultisportLeg {
                    sport: "Run".into(),
                    duration_seconds: None,
                    distance_meters: Some(5000.0),
                    notes: Some("off the bike".into()),
                },
            ],
        };
        let draft = crate::tools::multisport_session_draft(&session).unwrap();
        assert_eq!(draft["legs"].as_array().unwrap().len(), 2);
        assert!(draft["nextSteps"].as_str().unwrap().contains("COROS"));
    }

    #[test]
    fn race_plan_is_phased_and_has_goal_week() {
        let draft = crate::tools::race_plan_draft(&crate::parameters::RacePlan {
            event_name: "Autumn 10K".into(),
            start_date: Some("2026-08-17".into()),
            goal_date: "2026-10-12".into(),
            days_per_week: Some(4),
        })
        .unwrap();
        let weeks = draft["weeks"].as_array().unwrap();
        assert!(weeks.len() >= 8);
        assert_eq!(weeks.last().unwrap()["phase"], "Taper/Race");
    }

    #[test]
    fn guided_targets_expand_repeats_and_translate_intensity() {
        let payload = crate::tools::guided_workout_payload(
            &crate::parameters::CreateGuidedWorkout {
                name: "Threshold repeats".into(),
                overview: None,
                steps: vec![crate::parameters::GuidedStep {
                    kind: "training".into(),
                    duration_seconds: Some(300),
                    distance_meters: None,
                    intensity: Some("threshold".into()),
                    repeat: Some(4),
                }],
            },
            1,
        )
        .unwrap();
        assert_eq!(payload["exercises"].as_array().unwrap().len(), 4);
        assert_eq!(payload["exercises"][0]["intensityType"], 11);
        assert_eq!(payload["exercises"][0]["intensityValue"], 7);
    }

    #[test]
    fn adherence_identifies_unsatisfied_planned_days() {
        let result = crate::tools::plan_adherence_summary(
            20260810,
            20260816,
            &json!({"entities":[{"happenDay":20260810},{"happenDay":20260812}]}),
            &[json!({"date":20260810})],
        );
        assert_eq!(result["missedPlannedDays"], json!([20260812]));
        assert_eq!(result["adherence"], 0.5);
    }
}
