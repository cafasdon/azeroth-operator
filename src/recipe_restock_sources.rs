//! Saved provider references for the bag shortages of a validated restock plan.
use crate::{Snapshot, analysis};
use anyhow::Result;
use serde_json::{Value, json};

pub fn review(plan: &Value, snapshot: &Snapshot) -> Result<Value> {
    let joint = plan.get("targets").is_some();
    let empty = Vec::new();
    let materials = if joint {
        plan["ordered_materials"].as_array()
    } else if plan["planned_crafts"] == 0 {
        Some(&empty)
    } else {
        plan["recipe_review"]["materials"].as_array()
    };
    let basis = if joint { "ordered_whole_recipe_bag_shortfalls" }
        else { "gross_whole_recipe_bag_shortfalls" };
    let mut result = json!({"status":"material_order_unavailable", "basis":basis,
        "inventory_scope":"captured_bags_only", "same_capture_references":true,
        "bank_credit":false, "per_craft_output_credit":false,
        "acquisition_plan_complete":false,"current_availability_verified":false,
        "game_execution":false,"live":false,"source_review":null});
    let Some(materials) = materials else { return Ok(result) };
    let items: Vec<_> = materials.iter().map(|row| json!({
        "item_id":row["item_id"], "shortfall_vs_observed":row["shortfall_vs_bags"]
    })).collect();
    result["source_review"] = analysis::inventory_shortfall_captured_sources(
        &json!({"storage_observed":true,"items":items}), snapshot, snapshot, basis)?;
    result["status"] = json!("saved_reference_review");
    Ok(result)
}
