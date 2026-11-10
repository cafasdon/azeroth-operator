//! Historical dialogue discovery, with no NPC attribution or action semantics.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
};

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GossipOptionFilter {
    pub name: Option<String>,
    pub option_id: Option<NonZeroU32>,
    pub spell_id: Option<NonZeroU32>,
    pub reported_status: Option<u32>,
    pub reported_flags: Option<u32>,
    /// Inclusive lower bound on the captured signed order index.
    pub minimum_reported_order_index: Option<i32>,
    /// Inclusive upper bound on the captured signed order index.
    pub maximum_reported_order_index: Option<i32>,
}

impl GossipOptionFilter {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.name.is_some()
                || self.option_id.is_some()
                || self.spell_id.is_some()
                || self.reported_status.is_some()
                || self.reported_flags.is_some()
                || self.minimum_reported_order_index.is_some()
                || self.maximum_reported_order_index.is_some(),
            "Nonempty gossip option filter required"
        );
        ensure!(
            self.name
                .as_ref()
                .is_none_or(|name| !name.trim().is_empty() && name.len() <= 512),
            "Invalid gossip option label"
        );
        ensure!(
            self.minimum_reported_order_index
                .zip(self.maximum_reported_order_index)
                .is_none_or(|(minimum, maximum)| minimum <= maximum),
            "Gossip option order index bounds must be ordered"
        );
        Ok(())
    }
    pub(crate) fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        if snapshot.client.family != Family::Forever {
            return None;
        }
        observe(snapshot.sections.get("gossip_options")?, self)
            .ok()
            .flatten()
    }
}

fn observe(section: &Value, filter: &GossipOptionFilter) -> Result<Option<Value>> {
    filter.validate()?;
    crate::schema::validate("gossip_options", section)?;
    let rows = section["entries"].as_object().unwrap();
    let name = filter.name.as_ref().map(|text| text.trim().to_lowercase());
    let mut matched = Vec::new();
    let mut unknown_count = 0;
    for (slot, row) in rows {
        if name
            .as_ref()
            .is_some_and(|name| row["name"].as_str().unwrap().trim().to_lowercase() != *name)
        {
            continue;
        }
        let mut unknown = false;
        let mut rejected = false;
        for (field, wanted) in [
            ("option_id", filter.option_id.map(|id| id.get())),
            ("spell_id", filter.spell_id.map(|id| id.get())),
            ("reported_status", filter.reported_status),
            ("reported_flags", filter.reported_flags),
        ] {
            if let Some(wanted) = wanted {
                match row.get(field).and_then(Value::as_u64) {
                    Some(value) if value != u64::from(wanted) => {
                        rejected = true;
                        break;
                    }
                    None => unknown = true,
                    _ => {}
                }
            }
        }
        if rejected {
            continue;
        }
        if filter.minimum_reported_order_index.is_some()
            || filter.maximum_reported_order_index.is_some()
        {
            if let Some(order) = row["reported_order_index"].as_i64() {
                if filter
                    .minimum_reported_order_index
                    .is_some_and(|minimum| order < i64::from(minimum))
                    || filter
                        .maximum_reported_order_index
                        .is_some_and(|maximum| order > i64::from(maximum))
                {
                    continue;
                }
            } else {
                unknown = true;
            }
        }
        if unknown {
            unknown_count += 1;
            continue;
        }
        matched.push(json!({"slot":slot.parse::<u32>()?,"observation":row}));
    }
    if matched.is_empty() {
        return Ok(None);
    }
    matched.sort_unstable_by_key(|row| row["slot"].as_u64().unwrap());
    Ok(Some(json!({"basis":"captured_visible_gossip_options_only",
        "captured_option_count":rows.len(),"matching_option_count":matched.len(),
        "unknown_option_count":unknown_count,"nonmatching_option_count":rows.len()-matched.len()-unknown_count,
        "matches_truncated":matched.len()>20,"matching_options":matched.into_iter().take(20).collect::<Vec<_>>(),
        "npc_identity_verified":false,"selection_eligibility_verified":false,
        "complete_dialogue_coverage_verified":false,"current_availability_verified":false,
        "game_execution":false,"live":false})))
}

pub(crate) fn compare_options(before: Option<&Value>, after: Option<&Value>) -> Value {
    let before = before.and_then(|section| section["entries"].as_object());
    let after = after.and_then(|section| section["entries"].as_object());
    let changes = before.zip(after).map(|(old, new)| {
        let mut rows = BTreeMap::new();
        for (index, entries) in [old, new].into_iter().enumerate() {
            for row in entries.values() {
                let (_, counts) = rows.entry(row.to_string()).or_insert((row, [0_usize; 2]));
                counts[index] += 1;
            }
        }
        rows.into_values()
            .filter_map(|(row, [before_count, after_count])| {
                (before_count != after_count).then(|| {
                    json!({
                        "observation":row,"before_count":before_count,"after_count":after_count,
                    })
                })
            })
            .collect::<Vec<_>>()
    });
    json!({
        "before_observed_rows":before.map(|rows| rows.len()),
        "after_observed_rows":after.map(|rows| rows.len()),
        "row_count_changes":changes,
        "same_npc_verified":false,"same_dialogue_verified":false,
        "complete_dialogue_coverage_verified":false,"live":false,
    })
}

pub(crate) fn compare_quest_lists(before: Option<&Value>, after: Option<&Value>) -> Value {
    let compare_list = |kind: &str| {
        let old = before?[kind].as_object()?;
        let new = after?[kind].as_object()?;
        let ids: BTreeSet<u32> = old
            .keys()
            .chain(new.keys())
            .map(|id| id.parse().unwrap())
            .collect();
        let mut only_before = Vec::new();
        let mut only_after = Vec::new();
        let mut changes = Vec::new();
        for id in ids {
            let key = id.to_string();
            match (old.get(&key), new.get(&key)) {
                (Some(first), Some(second)) => {
                    let fields: serde_json::Map<_, _> =
                        ["repeatable", "is_complete", "is_trivial", "is_ignored"]
                            .into_iter()
                            .filter_map(|field| {
                                let before = first[field].as_bool()?;
                                let after = second[field].as_bool()?;
                                (before != after).then(|| {
                                    (field.to_owned(), json!({"before":before,"after":after}))
                                })
                            })
                            .collect();
                    if !fields.is_empty() {
                        changes.push(json!({"quest_id":id,"fields":fields}));
                    }
                }
                (Some(_), None) => only_before.push(id),
                (None, Some(_)) => only_after.push(id),
                _ => {}
            }
        }
        Some(
            json!({"observed_only_before":only_before,"observed_only_after":only_after,
            "flag_changes":changes}),
        )
    };
    json!({"available":compare_list("available"),"active":compare_list("active"),
        "same_npc_verified":false,"acceptance_eligibility_verified":false,
        "turn_in_eligibility_verified":false,"live":false})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn filter(value: Value) -> GossipOptionFilter {
        serde_json::from_value(value).unwrap()
    }
    #[test]
    fn same_row_unknowns_false_dominance_and_zero_status() {
        let section = json!({"coverage":"visible_gossip_options","entries":{
            "1":{"name":"Écho","option_id":42,"reported_status":0},
            "2":{"name":"Écho","reported_status":0},
            "3":{"name":"Écho","reported_status":1},
            "4":{"name":"Other","option_id":42,"reported_status":0}}});
        let result = observe(
            &section,
            &filter(json!({"name":" éCHO ","option_id":42,"reported_status":0})),
        )
        .unwrap()
        .unwrap();
        assert_eq!(result["matching_option_count"], 1);
        assert_eq!(result["unknown_option_count"], 1);
        assert_eq!(result["nonmatching_option_count"], 2);
        assert_eq!(result["matching_options"][0]["slot"], 1);
        assert!(
            observe(
                &section,
                &filter(json!({"option_id":43,"reported_status":0}))
            )
            .unwrap()
            .is_none()
        );
    }
    #[test]
    fn validation_empty_unknown_only_and_numeric_order() {
        for query in [
            json!({}),
            json!({"name":" "}),
            json!({"name":"é".repeat(257)}),
        ] {
            assert!(filter(query).validate().is_err());
        }
        let section = json!({"coverage":"visible_gossip_options","entries":{"1":{"name":"A"}}});
        assert!(
            observe(&section, &filter(json!({"option_id":42})))
                .unwrap()
                .is_none()
        );
        let entries: serde_json::Map<String, Value> = (1..=128)
            .map(|slot| (slot.to_string(), json!({"name":"A","reported_flags":0})))
            .collect();
        let result = observe(
            &json!({"coverage":"visible_gossip_options","entries":entries}),
            &filter(json!({"reported_flags":0})),
        )
        .unwrap()
        .unwrap();
        assert_eq!(result["matching_option_count"], 128);
        assert_eq!(result["matching_options"].as_array().unwrap().len(), 20);
        assert_eq!(result["matching_options"][9]["slot"], 10);
        assert_eq!(result["matches_truncated"], true);
    }
}
