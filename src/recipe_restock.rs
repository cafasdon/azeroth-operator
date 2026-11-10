//! Conditional base-item bag restock arithmetic; no craft or acquisition action.
use crate::{Snapshot, analysis};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::num::NonZeroU32;

#[cfg(test)]
mod tests {
    use super::*;
    fn saved() -> Snapshot {
        let snapshot: Snapshot = serde_json::from_value(json!({"schema_version":1,"source":"fixture",
            "captured_at":"2025-01-01T00:00:00Z","client":{"family":"forever","variant":"live","build":"70862","interface":16001},
            "character":{"name":"Fixture","realm":"Test"},"sections":{
            "selected_recipe":{"recipe_id":100,"name":"Fixture","learned":true,"requirements":"fixed_items","reagents":{"1":3},
                "output_item_id":2,"base_output_quantity":{"minimum":3,"maximum":5}},
            "bags":{"0":{"1":{"item_id":2,"count":4},"2":{"item_id":1,"count":9}}},
            "character_bank":{"6":{"1":{"item_id":2,"count":100}}}},"unavailable":{}})).unwrap();
        snapshot.validate().unwrap();
        snapshot
    }
    #[test]
    fn target_counts_bags_only_and_keeps_conditional_review() {
        let snapshot = saved();
        let before = serde_json::to_value(&snapshot).unwrap();
        let result = plan(&snapshot, None, NonZeroU32::new(10).unwrap()).unwrap();
        assert_eq!(result["observed_bag_quantity"], 4);
        assert_eq!(result["planned_crafts"], 2);
        assert_eq!(
            result["projected_bag_quantity_if_completed"],
            json!({"minimum":10,"maximum":14})
        );
        assert_eq!(result["recipe_review"]["materials"][0]["required"], 6);
        for flag in [
            "live",
            "game_execution",
            "craftability_verified",
            "item_variant_equivalence_verified",
        ] {
            assert_eq!(result[flag], false);
        }
        assert_eq!(before, serde_json::to_value(snapshot).unwrap());
    }
    #[test]
    fn enough_stock_does_not_invent_a_craft() {
        let result = plan(&saved(), None, NonZeroU32::new(4).unwrap()).unwrap();
        assert_eq!(result["planned_crafts"], 0);
        assert_eq!(result["recipe_review"], Value::Null);
        assert_eq!(result["saved_target_already_met"], true);
    }
    #[test]
    fn net_output_accounts_for_same_item_consumption() {
        let mut snapshot = saved();
        snapshot.sections.get_mut("selected_recipe").unwrap()["reagents"]["2"] = json!(1);
        snapshot.validate().unwrap();
        let result = plan(&snapshot, None, NonZeroU32::new(10).unwrap()).unwrap();
        assert_eq!(result["planned_crafts"], 3);
        assert_eq!(
            result["projected_bag_quantity_if_completed"],
            json!({"minimum":10,"maximum":16})
        );
        snapshot.sections.get_mut("selected_recipe").unwrap()["reagents"]["2"] = json!(3);
        assert!(plan(&snapshot, None, NonZeroU32::new(10).unwrap()).is_err());
    }
    #[test]
    fn unknown_inputs_and_over_budget_counts_refuse() {
        let snapshot = saved();
        assert!(plan(&snapshot, None, NonZeroU32::new(3005).unwrap()).is_err());
        for field in ["bags", "selected_recipe"] {
            let mut changed = snapshot.clone();
            changed.sections.remove(field);
            assert!(plan(&changed, None, NonZeroU32::new(10).unwrap()).is_err());
        }
        let mut changed = snapshot.clone();
        changed.client.family = crate::Family::Retail;
        assert!(plan(&changed, None, NonZeroU32::new(10).unwrap()).is_err());
    }
}

pub fn plan(
    snapshot: &Snapshot,
    recipe_id: Option<NonZeroU32>,
    target: NonZeroU32,
) -> Result<Value> {
    // Reuse the existing exact recipe, conflicting-observation and family checks.
    let unit = analysis::plan_recipe_by_id(snapshot, recipe_id, Some(1), None, None)?;
    let recipe = &unit["recipe"];
    if recipe_id.is_none() {
        let id = recipe["recipe_id"]
            .as_u64()
            .and_then(|id| u32::try_from(id).ok())
            .and_then(NonZeroU32::new)
            .ok_or_else(|| anyhow::anyhow!("Recipe ID unavailable"))?;
        let status = &snapshot
            .sections
            .get("profession_recipe_materials")
            .and_then(|bulk| bulk["entries"].get(id.to_string()))
            .map(|row| &row["status"]);
        if status
            .is_some_and(|status| status == "fixed_items" || status == "unsupported_required_slots")
        {
            // Validate known contrary schematics without replacing richer selected output metadata.
            analysis::plan_recipe_by_id(snapshot, Some(id), Some(1), None, None)?;
        }
    }
    ensure!(
        recipe["requirements"] == "fixed_items",
        "Fixed recipe inputs required"
    );
    let output = &unit["base_output_if_completed"];
    let item = output["item_id"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Output item unavailable"))?;
    let minimum = output["minimum"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or_else(|| anyhow::anyhow!("Output yield unavailable"))?;
    let maximum = output["maximum"]
        .as_u64()
        .filter(|n| *n >= minimum)
        .ok_or_else(|| anyhow::anyhow!("Output yield unavailable"))?;
    let bags = snapshot
        .sections
        .get("bags")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Bag observation unavailable"))?;
    let mut owned = 0u64;
    for container in bags.values() {
        for row in container
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid bag data"))?
            .values()
        {
            if row["item_id"].as_u64() == Some(item) {
                owned = owned
                    .checked_add(
                        row["count"]
                            .as_u64()
                            .ok_or_else(|| anyhow::anyhow!("Item count unavailable"))?,
                    )
                    .ok_or_else(|| anyhow::anyhow!("Bag quantity overflow"))?;
            }
        }
    }
    let consumed = recipe["reagents"]
        .get(item.to_string())
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let shortfall = u64::from(target.get()).saturating_sub(owned);
    let crafts = if shortfall == 0 {
        0
    } else {
        let net = minimum
            .checked_sub(consumed)
            .filter(|n| *n > 0)
            .ok_or_else(|| anyhow::anyhow!("No positive minimum net output"))?;
        shortfall.div_ceil(net)
    };
    ensure!(crafts <= 1000, "Restock craft count exceeds review bound");
    let projected = |yield_per_craft: u64| -> Result<u64> {
        owned
            .checked_add(
                yield_per_craft
                    .checked_sub(consumed)
                    .ok_or_else(|| anyhow::anyhow!("Invalid net output"))?
                    .checked_mul(crafts)
                    .ok_or_else(|| anyhow::anyhow!("Output quantity overflow"))?,
            )
            .ok_or_else(|| anyhow::anyhow!("Output quantity overflow"))
    };
    let (after_min, after_max) = if crafts == 0 {
        (owned, owned)
    } else {
        (projected(minimum)?, projected(maximum)?)
    };
    let review = if crafts == 0 {
        Value::Null
    } else if crafts == 1 {
        unit.clone()
    } else {
        analysis::plan_recipe_by_id(snapshot, recipe_id, Some(crafts as u32), None, None)?
    };
    Ok(json!({"basis":"same_saved_capture_base_item_bag_stock",
        "captured_at":snapshot.captured_at,"recipe_id":recipe["recipe_id"],
        "output_item_id":item,"target_bag_quantity":target.get(),"observed_bag_quantity":owned,
        "shortfall":shortfall,"planned_crafts":crafts,"saved_target_already_met":shortfall==0,
        "output_units_per_craft":{"minimum":minimum,"maximum":maximum},
        "same_item_reagent_units_per_craft":consumed,
        "projected_bag_quantity_if_completed":{"minimum":after_min,"maximum":after_max},
        "recipe_review":review,"reagent_basis":"gross_batch_inputs_without_intermediate_output_credit",
        "item_variant_equivalence_verified":false,"current_inventory_verified":false,
        "active_character_verified":false,"craftability_verified":false,
        "native_bag_placement_verified":false,"complete_ownership_verified":false,
        "game_execution":false,"live":false}))
}
