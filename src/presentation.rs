use anyhow::{Result, anyhow};
use chrono::{Datelike, NaiveDate};
use serde_json::{Value, json};
use std::collections::HashMap;

pub(crate) async fn result(
    f: impl std::future::Future<Output = Result<String>>,
) -> std::result::Result<String, String> {
    f.await.map_err(|error| format!("{error:#}"))
}

/// Read COROS numeric fields which may be encoded as JSON numbers or strings.
pub(crate) fn value_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
}
pub(crate) fn text(v: Value) -> String {
    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
}
pub(crate) fn field<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
pub(crate) fn code(map: &[(&str, i64)], value: &str) -> Option<i64> {
    map.iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(value))
        .map(|(_, c)| *c)
}
pub(crate) fn dry(endpoint: &str, payload: Value) -> String {
    text(json!({"dryRun":true,"endpoint":endpoint,"payload":payload}))
}
pub(crate) fn format_activity_date(date: i64) -> String {
    let date = date.to_string();
    if date.len() == 8 {
        format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8])
    } else {
        date
    }
}
pub(crate) fn format_duration(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}
pub(crate) fn sport_type_name(sport_type: i64) -> Option<&'static str> {
    match sport_type {
        100 => Some("Run"),
        101 => Some("Indoor Run"),
        102 => Some("Trail Run"),
        200 => Some("Cycling"),
        201 => Some("Indoor Cycling"),
        300 => Some("Pool Swim"),
        301 => Some("Open Water Swim"),
        400 => Some("Multi-Sport"),
        401 => Some("Triathlon"),
        402 => Some("Strength"),
        403 => Some("Cardio"),
        404 => Some("GPS Cardio"),
        500 => Some("Hike"),
        600 => Some("Ski"),
        700 => Some("Indoor Walk"),
        701 => Some("Indoor Rower"),
        _ => None,
    }
}
pub(crate) fn catalog_name(catalog: &[Value], code_name: &str) -> String {
    catalog
        .iter()
        .find(|exercise| field(exercise, "codeName") == code_name)
        .map(|exercise| field(exercise, "name").to_owned())
        .unwrap_or_else(|| code_name.to_owned())
}
pub(crate) fn summarize_strength_activity(catalog: &[Value], detail: &Value) -> String {
    let mut exercises: HashMap<i64, Vec<&Value>> = HashMap::new();
    for lap in detail["lapList"].as_array().into_iter().flatten() {
        for item in lap["lapItemList"].as_array().into_iter().flatten() {
            exercises
                .entry(item["exerciseIndex"].as_i64().unwrap_or_default())
                .or_default()
                .push(item);
        }
    }
    let mut indexes: Vec<_> = exercises.keys().copied().collect();
    indexes.sort_unstable();
    let mut lines = Vec::new();
    for index in indexes {
        let non_rest: Vec<_> = exercises[&index]
            .iter()
            .copied()
            .filter(|item| !field(item, "exerciseNameKey").starts_with('S'))
            .collect();
        let Some(rollup) = non_rest
            .iter()
            .copied()
            .find(|item| item["mode"].as_i64() == Some(16))
            .or_else(|| non_rest.last().copied())
        else {
            continue;
        };
        let sets: Vec<_> = non_rest
            .iter()
            .copied()
            .filter(|item| item["mode"].as_i64() == Some(14))
            .collect();
        let set_count = rollup["sets"]
            .as_i64()
            .unwrap_or_default()
            .max(sets.len() as i64);
        let reps = rollup["reps"].as_i64().unwrap_or_default();
        let exercise_detail = if set_count > 0 && reps > 0 {
            format!("{set_count}×{} ({reps} reps total)", reps / set_count)
        } else if reps > 0 {
            format!("{reps} reps")
        } else {
            format!(
                "{}s",
                rollup["totalLength"].as_i64().unwrap_or_default() / 1000
            )
        };
        let weights: Vec<_> = sets
            .iter()
            .filter_map(|item| item["weight"].as_f64())
            .map(|weight| weight / 1000.0)
            .filter(|weight| *weight > 0.0)
            .collect();
        let weight_detail = if weights.is_empty() {
            String::new()
        } else if weights.iter().all(|weight| *weight == weights[0]) {
            format!(" @ {}kg", weights[0])
        } else {
            format!(
                " @ {}",
                weights
                    .iter()
                    .map(|weight| format!("{weight}kg"))
                    .collect::<Vec<_>>()
                    .join("/")
            )
        };
        lines.push(format!(
            "  {index}. {} — {exercise_detail}{weight_detail}",
            catalog_name(catalog, field(rollup, "exerciseNameKey"))
        ));
    }
    lines.join("\n")
}
pub(crate) fn activity_detail_overview(detail: &Value) -> Value {
    let summary = &detail["summary"];
    json!({
        "summary": summary,
        "laps": detail["lapList"],
        "heartRateZones": detail["hrZoneList"],
        "powerZones": detail["powerZoneList"],
        "trainingEffect": detail["trainingEffect"],
    })
}
pub(crate) fn iso(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| anyhow!("Use ISO YYYY-MM-DD dates."))
}
pub(crate) fn monday(date: NaiveDate) -> NaiveDate {
    date - chrono::Duration::days(date.weekday().num_days_from_monday().into())
}
