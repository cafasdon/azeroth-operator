//! Discovery of historical displayed loot, never owned inventory or balances.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

fn one() -> u64 {
    1
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LootObservationFilter {
    Item {
        item_id: NonZeroU32,
        #[serde(default = "one")]
        minimum_quantity: u64,
        maximum_quantity: Option<u64>,
    },
    ReportedCurrency {
        reported_currency_id: NonZeroU32,
        #[serde(default = "one")]
        minimum_quantity: u64,
        maximum_quantity: Option<u64>,
    },
    Money {},
}

impl LootObservationFilter {
    pub(crate) fn validate(&self) -> Result<()> {
        match self {
            Self::Item {
                minimum_quantity,
                maximum_quantity,
                ..
            }
            | Self::ReportedCurrency {
                minimum_quantity,
                maximum_quantity,
                ..
            } => {
                ensure!(
                    maximum_quantity.is_none_or(|maximum| maximum >= *minimum_quantity),
                    "Loot quantity range reversed"
                );
            }
            Self::Money {} => {}
        }
        Ok(())
    }

    pub(crate) fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        if snapshot.client.family != Family::Forever {
            return None;
        }
        let section = snapshot.sections.get("loot_window")?;
        let rows = section["entries"].as_object()?;
        let (status, id_key, id, quantity_key, minimum, maximum, basis) = match self {
            Self::Item {
                item_id,
                minimum_quantity,
                maximum_quantity,
            } => (
                "item",
                "item_id",
                Some(item_id.get()),
                "quantity",
                *minimum_quantity,
                *maximum_quantity,
                "captured_readable_loot_item_rows_only",
            ),
            Self::ReportedCurrency {
                reported_currency_id,
                minimum_quantity,
                maximum_quantity,
            } => (
                "reported_currency",
                "reported_currency_id",
                Some(reported_currency_id.get()),
                "reported_quantity",
                *minimum_quantity,
                *maximum_quantity,
                "raw_slot_info_before_container_conversion",
            ),
            Self::Money {} => (
                "money",
                "",
                None,
                "",
                0,
                None,
                "captured_money_slot_presence_only",
            ),
        };
        let mut total = 0_u64;
        let mut slots = Vec::new();
        for (slot, row) in rows {
            if row["status"] != status
                || id.is_some_and(|id| row[id_key].as_u64() != Some(u64::from(id)))
            {
                continue;
            }
            let slot = slot.parse::<u32>().ok()?;
            let quantity = if id.is_some() {
                Some(row[quantity_key].as_u64()?)
            } else {
                None
            };
            if let Some(quantity) = quantity {
                total = total.checked_add(quantity)?;
            }
            slots.push((slot, quantity));
        }
        // A minimum of zero is not a query for absence: require a reported row.
        if slots.is_empty() || total < minimum || maximum.is_some_and(|maximum| total > maximum) {
            return None;
        }
        slots.sort_unstable_by_key(|(slot, _)| *slot);
        let unknown = rows
            .values()
            .filter(|row| row["status"] == "unreadable_or_non_item")
            .count();
        let mut report = json!({"kind":status,"coverage":section["coverage"],"basis":basis,
            "observed_quantity":if id.is_some(){Some(total)}else{None},
            "matching_slots":slots.iter().map(|(slot,quantity)|json!({"slot":slot,"reported_quantity":quantity})).collect::<Vec<_>>(),
            "matching_slot_count":slots.len(),"unreadable_slot_count":unknown,
            "additional_matching_rows_unknown":unknown>0,
            "counts_toward_owned_inventory":false,"counts_toward_owned_balances":false,
            "ownership_verified":false,"lootability_verified":false,"currency_rewards_verified":false,
            "money_amounts_verified":false,"source_identity_verified":false,"game_execution":false,"live":false});
        if let Some(id) = id {
            report[id_key] = json!(id);
            report["minimum_quantity"] = json!(minimum);
            report["maximum_quantity"] = json!(maximum);
        }
        Some(report)
    }
}

pub(crate) fn compare(before: &Value, after: &Value) -> Value {
    fn totals(section: &Value, kind: &str, id_key: &str, quantity_key: &str) -> BTreeMap<u64, u64> {
        let mut totals = BTreeMap::new();
        for row in section["entries"].as_object().unwrap().values() {
            if row["status"] == kind {
                *totals.entry(row[id_key].as_u64().unwrap()).or_default() +=
                    row[quantity_key].as_u64().unwrap();
            }
        }
        totals
    }
    let changes = |kind, id_key, quantity_key| {
        let old = totals(before, kind, id_key, quantity_key);
        let new = totals(after, kind, id_key, quantity_key);
        let ids: BTreeSet<_> = old.keys().chain(new.keys()).copied().collect();
        ids.into_iter()
            .filter(|id| old.get(id) != new.get(id))
            .map(|id| {
                json!({"id":id,"before_observed_quantity":old.get(&id),
                "after_observed_quantity":new.get(&id)})
            })
            .collect::<Vec<_>>()
    };
    let slot_counts = |section: &Value| {
        let rows = section["entries"].as_object().unwrap();
        json!({
            "money":rows.values().filter(|row| row["status"] == "money").count(),
            "unreadable_or_non_item":rows.values().filter(|row| row["status"] == "unreadable_or_non_item").count(),
        })
    };
    json!({
        "basis":"captured_displayed_slots_only",
        "item_changes":changes("item","item_id","quantity"),
        "reported_currency_changes":changes("reported_currency","reported_currency_id","reported_quantity"),
        "before_slot_counts":slot_counts(before),"after_slot_counts":slot_counts(after),
        "source_identity_verified":false,"lootability_verified":false,
        "counts_toward_owned_inventory":false,"counts_toward_owned_balances":false,"live":false
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(rows: Vec<Value>) -> Snapshot {
        serde_json::from_value(json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
            "client":{"family":"forever","variant":"live","build":"70862","interface":16001},
            "character":{"name":"Fixture","realm":"Test"},"sections":{"loot_window":{
                "coverage":"visible_loot_window_slots","slot_count":rows.len(),"entries":rows.into_iter().enumerate()
                    .map(|(index,row)|((index+1).to_string(),row)).collect::<serde_json::Map<_,_>>()}}})).unwrap()
    }
    fn filter(value: Value) -> LootObservationFilter {
        serde_json::from_value(value).unwrap()
    }
    #[test]
    fn kinds_quantities_and_unknowns_never_pool() {
        let snapshot = snapshot(vec![
            json!({"status":"item","item_id":10,"quantity":2}),
            json!({"status":"reported_currency","reported_currency_id":10,"reported_quantity":9}),
            json!({"status":"item","item_id":10,"quantity":3}),
            json!({"status":"money"}),
            json!({"status":"unreadable_or_non_item"}),
        ]);
        let matched = filter(json!({"kind":"item","item_id":10,"minimum_quantity":5}))
            .observation_match(&snapshot)
            .unwrap();
        assert_eq!(matched["observed_quantity"], 5);
        assert_eq!(matched["matching_slot_count"], 2);
        assert_eq!(matched["additional_matching_rows_unknown"], true);
        assert!(
            filter(json!({"kind":"item","item_id":10,"minimum_quantity":6}))
                .observation_match(&snapshot)
                .is_none()
        );
        let money = filter(json!({"kind":"money"}))
            .observation_match(&snapshot)
            .unwrap();
        assert_eq!(money["observed_quantity"], Value::Null);
        assert_eq!(money["matching_slots"][0]["reported_quantity"], Value::Null);
        let currency = filter(json!({"kind":"reported_currency","reported_currency_id":10}))
            .observation_match(&snapshot)
            .unwrap();
        assert_eq!(currency["observed_quantity"], 9);
        assert_eq!(currency["currency_rewards_verified"], false);
    }
    #[test]
    fn zero_requires_row_and_maximum_uses_observed_sum() {
        let zero =
            filter(json!({"kind":"item","item_id":10,"minimum_quantity":0,"maximum_quantity":0}));
        for rows in [vec![], vec![json!({"status":"unreadable_or_non_item"})]] {
            assert!(zero.observation_match(&snapshot(rows)).is_none());
        }
        assert!(
            zero.observation_match(&snapshot(vec![
                json!({"status":"item","item_id":10,"quantity":0})
            ]))
            .is_some()
        );
        let maximum = snapshot(vec![
            json!({"status":"item","item_id":10,"quantity":u32::MAX});
            64
        ]);
        assert_eq!(
            filter(json!({"kind":"item","item_id":10}))
                .observation_match(&maximum)
                .unwrap()["observed_quantity"],
            64 * u64::from(u32::MAX)
        );
        assert!(
            filter(json!({"kind":"item","item_id":10,"minimum_quantity":u64::MAX}))
                .observation_match(&maximum)
                .is_none()
        );
    }
    #[test]
    fn malformed_or_meaningless_filters_reject() {
        for bad in [
            json!({"kind":"money","minimum_quantity":0}),
            json!({"kind":"money","item_id":1}),
            json!({"kind":"item"}),
            json!({"kind":"reported_currency","currency_id":1}),
            json!({"kind":"item","item_id":0}),
            json!({"kind":"item","item_id":1,"minimum_quantity":true}),
            json!({"kind":"item","item_id":1,"maximum_quantity":1.0}),
        ] {
            assert!(
                serde_json::from_value::<LootObservationFilter>(bad.clone()).is_err(),
                "{bad}"
            );
        }
        assert!(
            filter(json!({"kind":"item","item_id":1,"minimum_quantity":2,"maximum_quantity":1}))
                .validate()
                .is_err()
        );
    }
}
