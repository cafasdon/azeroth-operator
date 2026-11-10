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
