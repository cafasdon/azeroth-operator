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

pub fn instance_lock_encounter_review(
    context: &Value,
    captured_at: DateTime<Utc>,
    reference: Option<DateTime<Utc>>,
) -> Option<Value> {
    let warning = &context["lock_warning"];
    let total = warning["encounters_total"].as_u64()?;
    let completed = warning["encounters_completed"].as_u64()?;
    let seconds = warning["remaining_seconds"].as_u64()?;
    let details = warning["encounters"].as_object().map(|rows| {
        let mut ordered: Vec<_> = rows
            .iter()
            .map(|(slot, row)| (slot.parse::<u32>().unwrap(), row))
            .collect();
        ordered.sort_unstable_by_key(|(slot, _)| *slot);
        let (completed, pending): (Vec<_>, Vec<_>) = ordered
            .into_iter()
            .partition(|(_, row)| row["killed"] == true);
        let describe = |rows: Vec<(u32, &Value)>| {
            rows.into_iter()
                .map(|(slot, row)| json!({"slot":slot,"name":row["name"]}))
                .collect::<Vec<_>>()
        };
        json!({"completed":describe(completed),"pending":describe(pending)})
    });
    let expiry = i64::try_from(seconds)
        .ok()
        .and_then(|seconds| projected_reset_at(seconds, captured_at));
    let mut report = json!({"encounters_total":total,"encounters_completed":completed,
        "encounters_remaining":total.checked_sub(completed),"encounter_details":details,
        "reported_warning_remaining_seconds":seconds,"projected_warning_expiry_at":expiry,
        "current_warning_verified":false,"saved_lockout_identity_verified":false,"live":false});
    if let Some(reference) = reference {
        report["warning_reference_at"] = json!(reference);
        report["warning_expiry_at_or_before_reference"] =
            json!(expiry.map(|expiry| expiry <= reference));
    }
    Some(report)
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

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstanceLockWarningFilter {
    pub minimum_completed_encounters: Option<u32>,
    pub maximum_completed_encounters: Option<u32>,
    pub minimum_remaining_encounters: Option<u32>,
    pub maximum_remaining_encounters: Option<u32>,
    pub maximum_warning_seconds: Option<u32>,
    pub has_encounter_details: Option<bool>,
    /// Exact case-insensitive encounter name from the saved warning.
    pub encounter_name: Option<String>,
    /// Require this defeated state on the same named encounter row.
    pub encounter_defeated: Option<bool>,
}

impl InstanceLockWarningFilter {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            self.encounter_name
                .as_ref()
                .is_none_or(|name| !name.trim().is_empty() && name.len() <= 256)
                && (self.encounter_defeated.is_none() || self.encounter_name.is_some()),
            "Encounter state requires a valid named encounter"
        );
        ensure!(
            self.minimum_completed_encounters
                .zip(self.maximum_completed_encounters)
                .is_none_or(|(min, max)| min <= max)
                && self
                    .minimum_remaining_encounters
                    .zip(self.maximum_remaining_encounters)
                    .is_none_or(|(min, max)| min <= max),
            "Encounter count bounds must be ordered"
        );
        Ok(())
    }

    pub(crate) fn match_observation(&self, context: &Value) -> Option<Value> {
        let warning = &context["lock_warning"];
        let completed = warning["encounters_completed"].as_u64()?;
        let remaining = warning["encounters_total"]
            .as_u64()?
            .checked_sub(completed)?;
        let seconds = warning["remaining_seconds"].as_u64()?;
        let has_details = warning["encounters"].is_object();
        let matches_aggregate = self
            .minimum_completed_encounters
            .is_none_or(|min| completed >= u64::from(min))
            && self
                .maximum_completed_encounters
                .is_none_or(|max| completed <= u64::from(max))
            && self
                .minimum_remaining_encounters
                .is_none_or(|min| remaining >= u64::from(min))
            && self
                .maximum_remaining_encounters
                .is_none_or(|max| remaining <= u64::from(max))
            && self
                .maximum_warning_seconds
                .is_none_or(|max| seconds <= u64::from(max))
            && self
                .has_encounter_details
                .is_none_or(|expected| expected == has_details);
        if !matches_aggregate {
            return None;
        }
        let mut report = json!({"instance_kind":context["kind"],"difficulty_id":context["difficulty_id"],
                "encounters_completed":completed,"encounters_remaining":remaining,
                "reported_warning_remaining_seconds":seconds,"has_encounter_details":has_details,
                "current_warning_verified":false,"saved_lockout_identity_verified":false});
        if let Some(name) = self.encounter_name.as_ref() {
            let name = name.trim().to_lowercase();
            let matches = warning["encounters"]
                .as_object()?
                .values()
                .filter(|row| {
                    row["name"]
                        .as_str()
                        .is_some_and(|label| label.trim().to_lowercase() == name)
                        && self
                            .encounter_defeated
                            .is_none_or(|defeated| row["killed"].as_bool() == Some(defeated))
                })
                .count();
            if matches == 0 {
                return None;
            }
            report["matching_encounter_count"] = json!(matches);
        }
        Some(report)
    }
}

pub(crate) fn compare_instance_lock_warnings(before: &Value, after: &Value) -> Option<Value> {
    if ["kind", "name", "difficulty_id", "instance_map_id"]
        .into_iter()
        .any(|field| before[field] != after[field])
    {
        return None;
    }
    let old = &before["lock_warning"];
    let new = &after["lock_warning"];
    let total = old["encounters_total"].as_u64()?;
    if new["encounters_total"].as_u64() != Some(total) {
        return None;
    }
    let completed_before = old["encounters_completed"].as_i64()?;
    let completed_after = new["encounters_completed"].as_i64()?;
    let changes = old["encounters"].as_object().zip(new["encounters"].as_object()).and_then(|(old, new)| {
        if old.len() != new.len() || old.iter().any(|(slot, row)| new.get(slot).is_none_or(|after| row["name"] != after["name"])) {
            return None;
        }
        let mut changes: Vec<_> = old.iter().filter_map(|(slot, row)| {
            let after = &new[slot];
            (row["killed"] != after["killed"]).then(|| {
                let slot = slot.parse::<u32>().unwrap();
                (slot, json!({"slot":slot,"name":row["name"],"before_defeated":row["killed"],"after_defeated":after["killed"]}))
            })
        }).collect();
        changes.sort_unstable_by_key(|(slot, _)| *slot);
        Some(changes.into_iter().map(|(_, row)| row).collect::<Vec<_>>())
    });
    Some(
        json!({"encounters_total":total,"before_encounters_completed":completed_before,
        "after_encounters_completed":completed_after,"completed_count_delta":completed_after-completed_before,
        "encounter_flag_changes":changes,"saved_lockout_identity_verified":false,
        "encounter_completion_events_verified":false,"live":false}),
    )
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

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SavedResetKind {
    Dungeon,
    Raid,
    WorldBoss,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SavedResetInstanceFilter {
    pub difficulty_id: Option<u32>,
    pub locked: Option<bool>,
    pub extended: Option<bool>,
    pub max_players: Option<std::num::NonZeroU32>,
}

/// Merge historical reset timestamps from both validated sections of one capture.
pub fn combined_schedule(
    snapshot: &crate::Snapshot,
    reference: DateTime<Utc>,
    through: Option<DateTime<Utc>>,
    kind: Option<SavedResetKind>,
    instance_filter: Option<SavedResetInstanceFilter>,
    maximum_update_age_seconds: Option<u32>,
) -> Result<Value> {
    ensure!(
        snapshot.client.family == crate::Family::Forever,
        "Saved reset schedules require Forever"
    );
    let mut by_time = BTreeMap::<DateTime<Utc>, Vec<Value>>::new();
    let mut coverage = BTreeMap::new();
    let mut matching_reset_window_counts = BTreeMap::new();
    let mut cache_age_filter_matches = BTreeMap::new();
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
        let cache_age_match = maximum_update_age_seconds.map(|maximum| {
            projected["cache_update_age_seconds"]
                .as_u64()
                .map(|age| age <= u64::from(maximum))
        });
        if let Some(matched) = cache_age_match {
            cache_age_filter_matches.insert(section, matched);
        }
        let Some(rows) = projected["rows"].as_array() else {
            matching_reset_window_counts.insert(section, Value::Null);
            continue;
        };
        let (mut elapsed, mut within_window, mut after_window, mut unknown) = (0, 0, 0, 0);
        for row in rows {
            if cache_age_match.is_some_and(|matched| matched != Some(true)) {
                break;
            }
            let observation = &row["observation"];
            let is_instance = section == "saved_instances";
            if instance_filter.is_some_and(|filter| {
                !is_instance
                    || filter.difficulty_id.is_some_and(|id| {
                        observation["difficulty_id"].as_u64() != Some(u64::from(id))
                    })
                    || filter
                        .locked
                        .is_some_and(|locked| observation["locked"].as_bool() != Some(locked))
                    || filter
                        .extended
                        .is_some_and(|extended| observation["extended"].as_bool() != Some(extended))
                    || filter.max_players.is_some_and(|size| {
                        observation["max_players"].as_u64() != Some(u64::from(size.get()))
                    })
            }) {
                continue;
            }
            if !kind.is_none_or(|kind| match kind {
                SavedResetKind::Dungeon => is_instance && observation["is_raid"] == false,
                SavedResetKind::Raid => is_instance && observation["is_raid"] == true,
                SavedResetKind::WorldBoss => !is_instance,
            }) {
                continue;
            }
            let reset = row["projected_reset_at"]
                .as_str()
                .and_then(|value| value.parse::<DateTime<Utc>>().ok());
            match reset {
                None => unknown += 1,
                Some(reset) if reset <= reference => elapsed += 1,
                Some(reset) if through.is_some_and(|end| reset > end) => after_window += 1,
                Some(reset) => {
                    within_window += 1;
                    let mut entry =
                        json!({"section":section,"slot":row["slot"],"name":observation["name"]});
                    if is_instance {
                        entry["saved_instance_details"] = json!({
                            "difficulty_id":observation["difficulty_id"],
                            "locked":observation["locked"],"extended":observation["extended"],
                            "is_raid":observation["is_raid"],"max_players":observation["max_players"],
                        });
                    }
                    by_time.entry(reset).or_default().push(entry);
                }
            }
        }
        matching_reset_window_counts.insert(
            section,
            json!({"at_or_before_reference":elapsed,
            "within_window":within_window,"after_window":after_window,"unknown_reset":unknown}),
        );
    }
    let schedule: Vec<_> = by_time
        .into_iter()
        .map(|(at, entries)| json!({"at":at,"entries":entries}))
        .collect();
    let mut result = json!({"captured_at":snapshot.captured_at,"reference_at":reference,"coverage":coverage,
        "matching_reset_window_counts":matching_reset_window_counts,
        "next_projected_reset":schedule.first(),"upcoming_projected_resets":schedule,
        "current_lockout_verified":false,"current_availability_verified":false,"live":false});
    if let Some(end) = through {
        result["through_at"] = json!(end);
    }
    if let Some(kind) = kind {
        result["kind"] = json!(kind);
    }
    if let Some(filter) = instance_filter {
        result["instance_filter"] = json!(filter);
    }
    if let Some(maximum) = maximum_update_age_seconds {
        result["maximum_update_age_seconds"] = json!(maximum);
        result["cache_age_filter_matches"] = json!(cache_age_filter_matches);
    }
    Ok(result)
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
