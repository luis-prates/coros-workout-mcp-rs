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
}
