//! Constant-gain standing targets, not native reward, cap or eligibility predictions.
use crate::{Family, InventoryRequirement, InventoryRequirementsFilter, InventoryStorage, Snapshot, analysis};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeSet, num::{NonZeroU32, NonZeroU64}};

pub fn argument<'de,D:serde::Deserializer<'de>>(input:D)->Result<Option<i32>,D::Error> {
    Option::<Value>::deserialize(input)?.map(|value|value.as_i64()
        .and_then(|value|i32::try_from(value).ok())
        .ok_or_else(||serde::de::Error::custom("Invalid reputation standing target"))).transpose()
}

pub fn review(snapshot: &Snapshot, faction_id:u32, target:i32, points:NonZeroU32,
    costs:&[InventoryRequirement], copper:Option<(NonZeroU64,u64)>) -> Result<Value> {
    ensure!(snapshot.client.family==Family::Forever && faction_id>0 && costs.len()<=20
        && costs.iter().map(|cost|cost.item_id).collect::<BTreeSet<_>>().len()==costs.len()
        && copper.is_none_or(|(_,reserve)|reserve<=9_999_999_999_999), "Invalid reputation target inputs");
    let section = snapshot.sections.get("reputation").unwrap_or(&Value::Null);
    let row = &section["entries"][faction_id.to_string()];
    let progress = analysis::standard_reputation_progress(section);
    let progress = &progress[faction_id.to_string()];
    let mut result = json!({"status":"standard_progress_unavailable", "faction_id":faction_id,
        "target_standing":target,"assumed_points_per_turn_in":points.get(),
        "captured_standing":null,"captured_capped_reaction":progress["capped"],
        "standing_shortfall":null,"saved_target_already_met":null,"turn_ins_if_constant_gain":null,
        "projected_standing_before_caps_if_completed":null,"resource_plan":null,
        "basis":"explicit_standing_target_constant_gain_before_caps_and_restrictions",
        "captured_at":snapshot.captured_at,"cross_rank_reward_rate_verified":false,
        "caps_applied":false,"actual_costs_verified":false,"target_achievability_verified":false,
        "turn_in_eligibility_verified":false,"reward_repeatability_verified":false,
        "current_inventory_verified":false,"active_character_verified":false,
        "bank_or_mail_credit":false,"game_execution":false,"live":false});
    let Some(current) = row["standing"].as_i64().filter(|_|progress.is_object()) else {return Ok(result)};
    let shortfall = u64::try_from((i128::from(target)-i128::from(current)).max(0))?;
    let needed = shortfall.div_ceil(u64::from(points.get()));
    result["status"]=json!("conditional_constant_gain_estimate");
    result["captured_standing"]=json!(current);
    result["standing_shortfall"]=json!(shortfall);
    result["saved_target_already_met"]=json!(shortfall==0);
    result["turn_ins_if_constant_gain"]=json!(needed);
    result["projected_standing_before_caps_if_completed"]=json!(i128::from(current)+i128::from(needed)*i128::from(points.get()));
    if costs.is_empty() && copper.is_none() {return Ok(result)};
    let stock = InventoryRequirementsFilter {storage:InventoryStorage::Bags,container_id:None,items:costs.to_vec()}
        .review_observed_stock(snapshot,None);
    let mut capacities = Vec::new();
    let mut item_rows = Vec::new();
    for (cost,observed) in costs.iter().zip(stock["items"].as_array().into_iter().flatten()) {
        let carried = observed["observed_quantity"].as_u64();
        let required = needed.checked_mul(cost.quantity.get());
        let capacity = carried.map(|quantity|quantity/cost.quantity.get());
        capacities.push(capacity);
        item_rows.push(json!({"item_id":cost.item_id.get(),"assumed_quantity_per_turn_in":cost.quantity.get(),
            "observed_carried_quantity":carried,"required_quantity":required,
            "required_quantity_overflow":required.is_none(),"supported_turn_ins":capacity,
            "shortfall_vs_carried":required.zip(carried).map(|(need,have)|need.saturating_sub(have))}));
    }
    let wallet = copper.map(|(cost,reserve)| {
        let balance=snapshot.sections.get("wallet").and_then(|wallet|wallet["copper"].as_u64());
        let mut plan=analysis::reputation_turn_in_wallet_plan(Some(needed),cost,balance,reserve);
        capacities.push(plan["supported_turn_ins"].as_u64());
        let shortfall=plan.as_object_mut().unwrap().remove("turn_ins_shortfall_to_next_rank").unwrap();
        plan["turn_ins_shortfall_to_target"]=shortfall;
        plan
    });
    let unknown = capacities.iter().filter(|capacity|capacity.is_none()).count();
    let upper = capacities.iter().filter_map(|capacity|*capacity).min();
    let supported = if needed==0 || upper==Some(0) {Some(0)}
        else if unknown==0 {upper.map(|capacity|capacity.min(needed))} else {None};
    result["resource_plan"]=json!({"item_costs":item_rows,"wallet":wallet,
        "known_resource_turn_in_capacity_upper_bound":upper,"unknown_resource_count":unknown,
        "supported_turn_ins_toward_target":supported,
        "turn_ins_shortfall_to_target":supported.map(|capacity|needed.saturating_sub(capacity)),
        "remaining_standing_shortfall_after_supported_turn_ins":supported.map(|capacity|
            shortfall.saturating_sub(capacity*u64::from(points.get()))),
        "cost_model_scope":"supplied_item_and_copper_costs_only_captured_bags_and_wallet"});
    Ok(result)
}
