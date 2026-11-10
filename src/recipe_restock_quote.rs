//! Conservative saved merchant quotation. Never an acquisition or craft action.
use crate::{Snapshot, analysis};
use serde_json::{Value, json};

pub fn quote(plan: &Value, snapshot: &Snapshot) -> Value {
    let joint = plan.get("targets").is_some();
    let materials = if joint {
        plan["ordered_materials"].as_array()
    } else if plan["planned_crafts"] == 0 {
        Some(&EMPTY)
    } else {
        plan["recipe_review"]["materials"].as_array()
    };
    let basis = if joint { "ordered_whole_recipe_bag_shortfalls" }
        else { "gross_whole_recipe_bag_shortfalls" };
    let mut result = json!({"status":"material_order_unavailable", "basis":basis,
        "inventory_scope":"captured_bags_only", "captured_at":snapshot.captured_at,
        "same_capture_offers_and_wallet":true, "bank_credit":false,
        "per_craft_output_credit":false, "current_prices_verified":false,
        "native_purchase_eligibility_verified":false, "game_execution":false,"live":false,
        "budget":null});
    let Some(materials) = materials else { return result; };
    // The parent reports all-observed and bag shortages separately. Purchases must
    // not silently assume a bank withdrawal or any pending mail funds.
    let materials: Vec<_> = materials.iter().map(|row| {
        let mut row = row.clone();
        row["shortfall_vs_observed"] = row["shortfall_vs_bags"].clone();
        row
    }).collect();
    let budget = analysis::material_merchant_plan(&materials, snapshot, snapshot);
    result["status"] = json!(if budget.is_null() {"merchant_observation_unavailable"} else {"saved_quote"});
    result["budget"] = budget;
    result
}

static EMPTY: Vec<Value> = Vec::new();
