//! Joint saved bag targets. Conditional arithmetic, not craft execution or native feasibility.
use crate::{Snapshot, analysis, recipe_restock};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
};

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdditionalTarget {
    pub recipe_id: NonZeroU32,
    pub target_bag_quantity: NonZeroU32,
}

struct Recipe {
    id: u64,
    selection: Option<NonZeroU32>,
    item: u64,
    target: u64,
    owned: u64,
    minimum: u64,
    maximum: u64,
    reagents: BTreeMap<u64, u64>,
}

fn consumption(recipes: &[Recipe], crafts: &[u64], index: usize) -> Result<u64> {
    recipes
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != index)
        .try_fold(0u64, |total, (other, recipe)| {
            let units = recipe
                .reagents
                .get(&recipes[index].item)
                .copied()
                .unwrap_or(0);
            total
                .checked_add(
                    units
                        .checked_mul(crafts[other])
                        .ok_or_else(|| anyhow::anyhow!("Material quantity overflow"))?,
                )
                .ok_or_else(|| anyhow::anyhow!("Material quantity overflow"))
        })
}

pub fn plan(
    snapshot: &Snapshot,
    primary: Option<NonZeroU32>,
    target: NonZeroU32,
    additional: &[AdditionalTarget],
) -> Result<Value> {
    plan_with_stock_schedule(snapshot, primary, target, additional, None)
}

pub fn plan_with_stock_schedule(
    snapshot: &Snapshot,
    primary: Option<NonZeroU32>,
    target: NonZeroU32,
    additional: &[AdditionalTarget],
    page: Option<&crate::recipe_stock_schedule::Page>,
) -> Result<Value> {
    if let Some(page) = page {
        page.validate()?;
    }
    ensure!(
        (1..=7).contains(&additional.len()),
        "Expected one to seven additional targets"
    );
    let mut recipes = Vec::new();
    let mut ids = BTreeSet::new();
    let mut outputs = BTreeSet::new();
    for (selection, target) in std::iter::once((primary, target)).chain(
        additional
            .iter()
            .map(|entry| (Some(entry.recipe_id), entry.target_bag_quantity)),
    ) {
        // Reuse selected/bulk consistency and all single-target guards against this same immutable capture.
        let base = recipe_restock::plan(snapshot, selection, target)?;
        let unit = analysis::plan_recipe_by_id(snapshot, selection, Some(1), None, None)?;
        let id = base["recipe_id"].as_u64().unwrap();
        let item = base["output_item_id"].as_u64().unwrap();
        ensure!(
            ids.insert(id) && outputs.insert(item),
            "Batch needs distinct recipes and output items"
        );
        let reagents = unit["recipe"]["reagents"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(id, quantity)| Ok((id.parse::<u64>()?, quantity.as_u64().unwrap())))
            .collect::<Result<BTreeMap<_, _>>>()?;
        recipes.push(Recipe {
            id,
            selection,
            item,
            target: u64::from(target.get()),
            owned: base["observed_bag_quantity"].as_u64().unwrap(),
            minimum: base["output_units_per_craft"]["minimum"].as_u64().unwrap(),
            maximum: base["output_units_per_craft"]["maximum"].as_u64().unwrap(),
            reagents,
        });
    }
    let mut crafts = vec![0u64; recipes.len()];
    let mut converged = false;
    // Monotone integer constraints. Each changed pass adds at least one craft; at most 8*1000 are admitted.
    for _ in 0..=8000 {
        let mut changed = false;
        for (index, recipe) in recipes.iter().enumerate() {
            let required = recipe
                .target
                .checked_add(consumption(&recipes, &crafts, index)?)
                .ok_or_else(|| anyhow::anyhow!("Target quantity overflow"))?;
            let shortage = required.saturating_sub(recipe.owned);
            let next = if shortage == 0 {
                0
            } else {
                let own_input = recipe.reagents.get(&recipe.item).copied().unwrap_or(0);
                let net = recipe
                    .minimum
                    .checked_sub(own_input)
                    .filter(|n| *n > 0)
                    .ok_or_else(|| anyhow::anyhow!("No positive minimum net output"))?;
                shortage.div_ceil(net)
            };
            ensure!(
                next <= 1000,
                "Joint target exceeds the per-recipe craft bound"
            );
            ensure!(next >= crafts[index], "Nonmonotone target model");
            changed |= next != crafts[index];
            crafts[index] = next;
        }
        if !changed {
            converged = true;
            break;
        }
    }
    ensure!(
        converged,
        "Joint targets did not converge within the craft bound"
    );
    let mut remaining = (0..recipes.len())
        .filter(|i| crafts[*i] > 0)
        .collect::<BTreeSet<_>>();
    let mut order = Vec::new();
    while let Some(index) = remaining
        .iter()
        .copied()
        .filter(|index| {
            !remaining.iter().any(|other| {
                other != index && recipes[*index].reagents.contains_key(&recipes[*other].item)
            })
        })
        .min_by_key(|index| recipes[*index].id)
    {
        remaining.remove(&index);
        order.push(index);
    }
    let order_known = remaining.is_empty();
    let mut rows = Vec::new();
    let mut reviews = BTreeMap::new();
    for (index, recipe) in recipes.iter().enumerate() {
        let external = consumption(&recipes, &crafts, index)?;
        let own = recipe
            .reagents
            .get(&recipe.item)
            .copied()
            .unwrap_or(0)
            .checked_mul(crafts[index])
            .ok_or_else(|| anyhow::anyhow!("Material quantity overflow"))?;
        let used = external
            .checked_add(own)
            .ok_or_else(|| anyhow::anyhow!("Material quantity overflow"))?;
        let after = |yield_units: u64| -> Result<u64> {
            recipe
                .owned
                .checked_add(
                    yield_units
                        .checked_mul(crafts[index])
                        .ok_or_else(|| anyhow::anyhow!("Output quantity overflow"))?,
                )
                .and_then(|total| total.checked_sub(used))
                .ok_or_else(|| anyhow::anyhow!("Invalid final quantity"))
        };
        let minimum = after(recipe.minimum)?;
        let maximum = after(recipe.maximum)?;
        ensure!(
            minimum >= recipe.target,
            "Joint target not covered by minimum output"
        );
        let review = if crafts[index] == 0 {
            Value::Null
        } else {
            analysis::plan_recipe_by_id(
                snapshot,
                recipe.selection,
                Some(crafts[index] as u32),
                None,
                None,
            )?
        };
        if crafts[index] > 0 {
            reviews.insert(index, review.clone());
        }
        rows.push(json!({"recipe_id":recipe.id,"output_item_id":recipe.item,
            "target_bag_quantity":recipe.target,"observed_bag_quantity":recipe.owned,
            "other_requested_recipes_consume":external,"same_recipe_consumption":own,
            "planned_crafts":crafts[index],"modelled_final_bag_quantity":{"minimum":minimum,"maximum":maximum},
            "recipe_review":review}));
    }
    let combined = analysis::combine_recipe_materials(reviews.values())?;
    let (ordered, checkpoint) = if order_known {
        analysis::ordered_recipe_materials(order.iter().map(|index| &reviews[index]), &combined)?
    } else {
        (Value::Null, Value::Null)
    };
    let mut report = json!({"basis":"same_capture_joint_base_item_bag_targets","captured_at":snapshot.captured_at,
        "count_basis":"minimum_base_output_fixed_point_for_requested_recipes_only",
        "targets":rows,"combined_gross_materials":combined,"ordered_materials":ordered,
        "ordered_stock_checkpoint":checkpoint,
        "dependency_order_status":if !order_known {"cyclic_or_downstream"} else if order.is_empty(){"no_crafts_needed"}else{"acyclic_requested_outputs"},
        "suggested_recipe_order":order_known.then(||order.iter().map(|index|recipes[*index].id).collect::<Vec<_>>()),
        "unresolved_order_recipe_ids":remaining.iter().map(|index|recipes[*index].id).collect::<BTreeSet<_>>(),
        "projection_basis":"captured_bags_plus_requested_output_minus_requested_inputs_excludes_new_stock",
        "material_basis":"gross_batch_steps_with_minimum_producer_output_credit_when_order_known",
        "same_recipe_intermediate_output_credit":false,"acquisition_effects_included":false,
        "current_inventory_verified":false,"active_character_verified":false,"craftability_verified":false,
        "native_bag_placement_verified":false,"item_variant_equivalence_verified":false,
        "complete_ownership_verified":false,"game_execution":false,"live":false});
    if let Some(page) = page {
        let inputs = recipes
            .iter()
            .enumerate()
            .map(|(i, recipe)| crate::recipe_stock_schedule::Recipe {
                id: recipe.id,
                item: recipe.item,
                output: recipe.minimum,
                target: recipe.target,
                crafts: crafts[i],
                reagents: recipe.reagents.clone(),
            })
            .collect::<Vec<_>>();
        report["same_recipe_intermediate_output_credit_scope"] =
            json!("whole_recipe_material_ledger_only");
        report["stock_schedule"] = crate::recipe_stock_schedule::review(snapshot, &inputs, page)?;
    }
    Ok(report)
}
