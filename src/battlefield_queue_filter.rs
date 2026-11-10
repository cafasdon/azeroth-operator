//! Historical slot-status discovery; never current queue membership or actions.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::num::NonZeroU32;

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BattlefieldQueueFilter {
    /// Exact captured token: none, queued, confirm, active or locked.
    pub reported_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 32))]
    pub slot: Option<NonZeroU32>,
    /// Match at least this many slots after the optional exact-slot restriction; defaults to one.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 32))]
    pub minimum_matching_slots: Option<NonZeroU32>,
}

impl BattlefieldQueueFilter {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            matches!(
                self.reported_status.as_str(),
                "none" | "queued" | "confirm" | "active" | "locked"
            ),
            "Invalid battlefield queue status filter"
        );
        ensure!(
            self.slot.is_none_or(|slot| slot.get() <= 32)
                && self
                    .minimum_matching_slots
                    .is_none_or(|count| count.get() <= 32),
            "Battlefield queue filter bounds exceed recorded slot budget"
        );
        Ok(())
    }

    pub(crate) fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        if snapshot.client.family != Family::Forever {
            return None;
        }
        self.validate().ok()?;
        let section = snapshot.sections.get("battlefield_queue_slots")?;
        crate::schema::validate("battlefield_queue_slots", section).ok()?;
        let mut slots = Vec::new();
        for (slot, row) in section["entries"].as_object()? {
            let slot = slot.parse::<u32>().ok()?;
            if self.slot.is_none_or(|selected| selected.get() == slot)
                && row["reported_status"].as_str() == Some(self.reported_status.as_str())
            {
                slots.push(slot);
            }
        }
        if slots.len() < self.minimum_matching_slots.map_or(1, |count| count.get()) as usize {
            return None;
        }
        slots.sort_unstable();
        Some(json!({"filter":self,"coverage":section["coverage"],
            "reported_slot_count":section["reported_slot_count"],"matching_slot_count":slots.len(),
            "matching_slots":slots,"current_queue_membership_verified":false,
            "complete_queue_coverage_verified":false,"queue_identity_verified":false,
            "acceptance_eligibility_verified":false,"game_execution":false,"live":false}))
    }
}

/// Compare recorded positions, not persistent queue identities or join/leave events.
pub fn compare(before: Option<&Value>, after: Option<&Value>) -> Result<Value> {
    for section in [before, after].into_iter().flatten() {
        crate::schema::validate("battlefield_queue_slots", section)?;
    }
    let capacity = |section: Option<&Value>| {
        section.map(|section| section["reported_slot_count"].as_u64().unwrap())
    };
    let mut result = json!({"basis":"saved_battlefield_slot_observations_only",
        "before_reported_slot_count":capacity(before),"after_reported_slot_count":capacity(after),
        "slot_identity_verified":false,"same_queue_continuity_verified":false,
        "current_queue_membership_verified":false,"complete_queue_coverage_verified":false,
        "acceptance_eligibility_verified":false,"join_or_leave_inferred":false,
        "game_execution":false,"live":false});
    let (Some(before), Some(after)) = (before, after) else {
        for field in [
            "reported_slot_capacity_changed",
            "changed_slot_count",
            "unchanged_recorded_slot_count",
            "slot_changes",
        ] {
            result[field] = Value::Null;
        }
        return Ok(result);
    };
    let old_count = before["reported_slot_count"].as_u64().unwrap();
    let new_count = after["reported_slot_count"].as_u64().unwrap();
    let mut changed = Vec::new();
    let mut unchanged = 0;
    for slot in 1..=old_count.max(new_count) {
        let key = slot.to_string();
        let old = before["entries"]
            .get(&key)
            .map(|row| row["reported_status"].as_str().unwrap());
        let new = after["entries"]
            .get(&key)
            .map(|row| row["reported_status"].as_str().unwrap());
        if old == new {
            unchanged += 1;
            continue;
        }
        let kind = match (old, new) {
            (Some(_), None) => "observed_only_before",
            (None, Some(_)) => "observed_only_after",
            _ => "reported_status_changed",
        };
        changed.push(json!({"slot":slot,"kind":kind,"before_reported_status":old,"after_reported_status":new}));
    }
    result["reported_slot_capacity_changed"] = json!(old_count != new_count);
    result["changed_slot_count"] = json!(changed.len());
    result["unchanged_recorded_slot_count"] = json!(unchanged);
    result["slot_changes"] = json!(changed);
    Ok(result)
}
