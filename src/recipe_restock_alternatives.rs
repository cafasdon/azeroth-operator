//! Compare explicit alternative recipes against one immutable bag target. No crafting or purchases.
use crate::{Snapshot, recipe_restock, recipe_restock_quote};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeSet, num::NonZeroU32};

pub fn validate(ids: &[NonZeroU32], primary: Option<NonZeroU32>) -> Result<()> {
    ensure!(
        (1..=7).contains(&ids.len()),
        "Expected one to seven alternative recipes"
    );
    let distinct = ids.iter().map(|id| id.get()).collect::<BTreeSet<_>>();
    ensure!(
        distinct.len() == ids.len() && primary.is_none_or(|id| !distinct.contains(&id.get())),
        "Alternative recipes must be distinct"
    );
    Ok(())
}

pub fn review(
    snapshot: &Snapshot,
    primary: &Value,
    target: NonZeroU32,
    ids: &[NonZeroU32],
) -> Result<Value> {
    let primary_id = primary["recipe_id"]
        .as_u64()
        .and_then(|id| u32::try_from(id).ok())
        .and_then(NonZeroU32::new)
        .ok_or_else(|| anyhow::anyhow!("Recipe identity unavailable"))?;
    validate(ids, Some(primary_id))?;
    let output = primary["output_item_id"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Output unavailable"))?;
    let mut plans = vec![(primary_id, Some(primary.clone()))];
    for id in ids {
        let plan = recipe_restock::plan(snapshot, Some(*id), target).ok();
        if let Some(plan) = &plan {
            ensure!(
                plan["output_item_id"].as_u64() == Some(output),
                "Alternative output differs"
            );
        }
        plans.push((*id, plan));
    }
    plans.sort_by_key(|(id, _)| id.get());
    let wallet = snapshot
        .sections
        .get("wallet")
        .and_then(|row| row["copper"].as_u64());
    let mut known = Vec::new();
    let mut rows = Vec::new();
    for (id, plan) in plans {
        let Some(plan) = plan else {
            rows.push(
                json!({"recipe_id":id.get(),"status":"restock_inputs_unavailable",
                "planned_crafts":null,"purchase_copper":null,"affordable_from_saved_wallet":null,
                "projected_bag_quantity_if_completed":null}),
            );
            continue;
        };
        let no_purchase = plan["planned_crafts"] == 0
            || plan["recipe_review"]["materials"]
                .as_array()
                .is_some_and(|materials| materials.iter().all(|row| row["shortfall_vs_bags"] == 0));
        let quote = if no_purchase {
            Value::Null
        } else {
            recipe_restock_quote::quote(&plan, snapshot)
        };
        let cost = if no_purchase {
            Some(0)
        } else {
            quote["budget"]["copper_only_bundle_mix_total"].as_u64()
        };
        if let Some(cost) = cost {
            known.push((cost, id.get()));
        }
        rows.push(json!({"recipe_id":id.get(),
            "status":if cost.is_some(){"saved_purchase_quote_known"}else{"saved_purchase_quote_unavailable"},
            "planned_crafts":plan["planned_crafts"],"purchase_copper":cost,
            "affordable_from_saved_wallet":cost.zip(wallet).map(|(cost,balance)|cost<=balance),
            "projected_bag_quantity_if_completed":plan["projected_bag_quantity_if_completed"]}));
    }
    known.sort_unstable();
    let lowest = known.first().map(|(cost, _)| *cost);
    let tied = known
        .iter()
        .filter(|(cost, _)| Some(*cost) == lowest)
        .map(|(_, id)| *id)
        .collect::<Vec<_>>();
    let complete = known.len() == rows.len();
    Ok(
        json!({"basis":"independent_same_capture_same_base_item_bag_target",
        "status":if complete{"all_requested_saved_quotes_known"}else{"requested_saved_quotes_incomplete"},
        "output_item_id":output,"target_bag_quantity":target.get(),"primary_recipe_id":primary_id.get(),
        "candidate_count":rows.len(),"known_quote_count":known.len(),"candidates":rows,
        "lowest_known_purchase_copper":lowest,"lowest_known_quote_recipe_ids":tied,
        "lowest_requested_purchase_copper":if complete{lowest}else{None},
        "lowest_requested_quote_recipe_ids":if complete{json!(tied)}else{Value::Null},
        "cost_basis":"additional_copper_only_bundles_for_gross_bag_shortfalls",
        "same_capture_offers_and_wallet":true,"bank_credit":false,
        "owned_material_opportunity_cost_included":false,"cross_recipe_stock_consumption":false,
        "complete_recipe_catalogue_verified":false,"native_output_equivalence_verified":false,
        "current_prices_verified":false,"craftability_verified":false,"game_execution":false,"live":false}),
    )
}
