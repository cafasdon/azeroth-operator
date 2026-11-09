//! Historical reset arithmetic, never a current lockout or eligibility check.
use anyhow::{Result, ensure};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn projected_reset_at(
    seconds: i64,
    captured_at: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    TimeDelta::try_seconds(seconds)
        .filter(|_| seconds >= 0)
        .and_then(|delta| captured_at.checked_add_signed(delta))
}

pub(crate) fn matches_projected_window(
    seconds: Option<i64>,
    captured_at: DateTime<Utc>,
    from: Option<DateTime<Utc>>,
    through: Option<DateTime<Utc>>,
) -> bool {
    (from.is_none() && through.is_none())
        || seconds
            .and_then(|seconds| projected_reset_at(seconds, captured_at))
            .is_some_and(|reset| {
                from.is_none_or(|bound| reset >= bound)
                    && through.is_none_or(|bound| reset <= bound)
            })
}

pub(crate) fn discovery_report(
    mut matches: Vec<(&String, &Value)>,
    saved: &Value,
    captured_at: DateTime<Utc>,
    include_projection: bool,
) -> Option<Value> {
    if matches.is_empty() {
        return None;
    }
    matches.sort_unstable_by_key(|(slot, _)| slot.parse::<u32>().unwrap());
    let entries: Vec<_> = matches
        .iter()
        .take(20)
        .map(|(slot, row)| {
            let mut entry = json!({"slot":slot.parse::<u32>().unwrap(),"data":row});
            if include_projection {
                entry["projected_reset_at"] = json!(
                    row["reset_seconds"]
                        .as_i64()
                        .and_then(|seconds| projected_reset_at(seconds, captured_at))
                );
            }
            entry
        })
        .collect();
    Some(json!({"update_age_seconds":saved["update_age_seconds"],
        "matched_count":matches.len(),"truncated":matches.len()>20,"entries":entries}))
}

pub fn project(section: &str, data: Option<&Value>, captured_at: DateTime<Utc>) -> Result<Value> {
    ensure!(
        matches!(section, "saved_instances" | "saved_world_bosses"),
        "Invalid reset section"
    );
    let mut result = json!({"basis":"captured_reset_seconds","captured_at":captured_at,
        "cache_update_age_seconds":null,"rows":null,"recorded_count":null,
        "projected_count":null,"unknown_count":null,"complete_coverage_verified":false,
        "current_lockout_verified":false,"current_availability_verified":false,
        "extension_outcome_inferred":false,"game_execution":false,"live":false});
    let Some(data) = data else { return Ok(result) };
    crate::schema::validate(section, data)?;
    let entries = data["entries"].as_object().unwrap();
    let mut rows = Vec::new();
    let mut projected = 0;
    for (slot, observation) in entries {
        let seconds = observation["reset_seconds"].as_i64().unwrap();
        let reset = projected_reset_at(seconds, captured_at);
        if reset.is_some() {
            projected += 1;
        }
        rows.push(json!({"slot":slot.parse::<u32>()?,"observation":observation,
            "projected_reset_at":reset,"projection_status":if reset.is_some(){"projected"}
                else if seconds < 0 {"negative_reported_duration"} else {"timestamp_out_of_range"}}));
    }
    rows.sort_unstable_by_key(|row| row["slot"].as_u64().unwrap());
    result["cache_update_age_seconds"] = data["update_age_seconds"].clone();
    result["recorded_count"] = json!(entries.len());
    result["projected_count"] = json!(projected);
    result["unknown_count"] = json!(entries.len() - projected);
    result["rows"] = json!(rows);
    Ok(result)
}

/// Classify historical projections relative to a caller-supplied timestamp.
pub fn classify_at(projection: &mut Value, reference: DateTime<Utc>) {
    let classification = projection["rows"].as_array().map(|rows| {
        let mut elapsed = Vec::new();
        let mut upcoming = Vec::new();
        let mut unknown = Vec::new();
        let mut by_time = BTreeMap::<DateTime<Utc>, Vec<u64>>::new();
        for row in rows {
            let bucket = match row["projected_reset_at"]
                .as_str()
                .and_then(|value| value.parse::<DateTime<Utc>>().ok())
            {
                Some(reset) if reset <= reference => &mut elapsed,
                Some(reset) => {
                    by_time
                        .entry(reset)
                        .or_default()
                        .push(row["slot"].as_u64().unwrap());
                    &mut upcoming
                }
                None => &mut unknown,
            };
            bucket.push(row["slot"].clone());
        }
        let schedule: Vec<_> = by_time
            .into_iter()
            .map(|(at, slots)| json!({"at":at,"slots":slots}))
            .collect();
        json!({"at_or_before_slots":elapsed,"after_slots":upcoming,"unknown_slots":unknown,
            "next_projected_reset":schedule.first(),"upcoming_projected_resets":schedule})
    });
    projection["reference_at"] = json!(reference);
    projection["relative_to_reference"] = json!(classification);
}

/// Merge historical reset timestamps from both validated sections of one capture.
pub fn combined_schedule(snapshot: &crate::Snapshot, reference: DateTime<Utc>) -> Result<Value> {
    ensure!(
        snapshot.client.family == crate::Family::Forever,
        "Saved reset schedules require Forever"
    );
    let mut by_time = BTreeMap::<DateTime<Utc>, Vec<Value>>::new();
    let mut coverage = BTreeMap::new();
    for section in ["saved_instances", "saved_world_bosses"] {
        let projected = project(
            section,
            snapshot.sections.get(section),
            snapshot.captured_at,
        )?;
        coverage.insert(
            section,
            json!({"recorded_count":projected["recorded_count"],
            "unknown_count":projected["unknown_count"],
            "cache_update_age_seconds":projected["cache_update_age_seconds"]}),
        );
        for row in projected["rows"].as_array().into_iter().flatten() {
            let reset = row["projected_reset_at"]
                .as_str()
                .and_then(|value| value.parse::<DateTime<Utc>>().ok());
            if let Some(reset) = reset.filter(|reset| *reset > reference) {
                by_time
                    .entry(reset)
                    .or_default()
                    .push(json!({"section":section,"slot":row["slot"],
                    "name":row["observation"]["name"]}));
            }
        }
    }
    let schedule: Vec<_> = by_time
        .into_iter()
        .map(|(at, entries)| json!({"at":at,"entries":entries}))
        .collect();
    Ok(
        json!({"captured_at":snapshot.captured_at,"reference_at":reference,"coverage":coverage,
        "next_projected_reset":schedule.first(),"upcoming_projected_resets":schedule,
        "current_lockout_verified":false,"current_availability_verified":false,"live":false}),
    )
}

pub(crate) fn compare_world_bosses(before: &Value, after: &Value) -> Value {
    fn index(section: &Value) -> BTreeMap<String, Vec<&Value>> {
        let mut names: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
        for row in section["entries"].as_object().unwrap().values() {
            names
                .entry(row["name"].as_str().unwrap().to_lowercase())
                .or_default()
                .push(row);
        }
        names
    }
    let old = index(before);
    let new = index(after);
    let names: BTreeSet<_> = old.keys().chain(new.keys()).collect();
    let mut changes = Vec::new();
    let mut ambiguous = Vec::new();
    let mut only_before = 0;
    let mut only_after = 0;
    for name in names {
        match (
            old.get(name).map(Vec::as_slice),
            new.get(name).map(Vec::as_slice),
        ) {
            (Some([old]), Some([new])) => {
                if old["reset_seconds"] != new["reset_seconds"] {
                    changes.push(
                        json!({"name":new["name"],"before_reset_seconds":old["reset_seconds"],
                        "after_reset_seconds":new["reset_seconds"]}),
                    );
                }
            }
            (Some(rows), None) if rows.len() == 1 => only_before += 1,
            (None, Some(rows)) if rows.len() == 1 => only_after += 1,
            _ => ambiguous.push(name),
        }
    }
    json!({"basis":"captured_case_insensitive_names_only","changes":changes,
        "ambiguous_names":ambiguous,"observed_only_before_count":only_before,
        "observed_only_after_count":only_after,
        "before_cache_update_age_seconds":before["update_age_seconds"],
        "after_cache_update_age_seconds":after["update_age_seconds"],
        "lockout_identity_verified":false,"current_availability_verified":false,"live":false})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_bounds_missing_empty_and_cache_age() {
        let date = "2025-01-01T23:59:00Z".parse().unwrap();
        let data = json!({"coverage":"updated_cache","update_age_seconds":999,
            "entries":{"1":{"name":"A","reset_seconds":120},"2":{"name":"B","reset_seconds":-1},
            "3":{"name":"C","reset_seconds":i32::MAX},"4":{"name":"D","reset_seconds":0}}});
        let result = project("saved_world_bosses", Some(&data), date).unwrap();
        assert_eq!(
            result["rows"][0]["projected_reset_at"],
            "2025-01-02T00:01:00Z"
        );
        assert_eq!(
            result["rows"][1]["projection_status"],
            "negative_reported_duration"
        );
        assert_eq!(result["rows"][2]["projection_status"], "projected");
        assert_eq!(
            result["rows"][3]["projected_reset_at"],
            "2025-01-01T23:59:00Z"
        );
        assert_eq!(result["projected_count"], 3);
        assert_eq!(result["unknown_count"], 1);
        assert!(project("saved_instances", None, date).unwrap()["rows"].is_null());
        let empty = json!({"coverage":"updated_cache","update_age_seconds":0,"entries":{}});
        assert_eq!(
            project("saved_instances", Some(&empty), date).unwrap()["rows"],
            json!([])
        );
        assert_eq!(
            project("saved_world_bosses", Some(&data), DateTime::<Utc>::MAX_UTC).unwrap()["rows"]
                [0]["projection_status"],
            "timestamp_out_of_range"
        );
        assert!(project("quests", None, date).is_err());
        assert!(project("saved_instances", Some(&data), date).is_err());
    }
}
