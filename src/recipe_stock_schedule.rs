//! Bounded resource-order witnesses for fixed counts. Never a native crafting command.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};

fn budget() -> u32 {
    10_000
}
fn limit() -> u32 {
    20
}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Page {
    /// Maximum distinct remaining-count states including the root: 1..50000. Exhaustion stays unknown.
    #[schemars(range(min = 1, max = 50000))]
    #[serde(default = "budget")]
    pub state_budget: u32,
    /// Zero-based offset into the coalesced steps of a complete witness only.
    #[serde(default)]
    pub offset: u32,
    /// Returned witness steps per page: 1..50, default 20.
    #[schemars(range(min = 1, max = 50))]
    #[serde(default = "limit")]
    pub limit: u32,
}
impl Page {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=50_000).contains(&self.state_budget) && (1..=50).contains(&self.limit),
            "Invalid stock schedule page"
        );
        Ok(())
    }
}

pub(crate) struct Recipe {
    pub id: u64,
    pub item: u64,
    pub output: u64,
    pub target: u64,
    pub crafts: u64,
    pub reagents: BTreeMap<u64, u64>,
}
struct Indexed {
    id: u64,
    item: usize,
    output: u64,
    inputs: Vec<(usize, u64)>,
}
fn apply(stock: &mut [u64], recipe: &Indexed) -> Result<()> {
    for (item, units) in &recipe.inputs {
        stock[*item] = stock[*item]
            .checked_sub(*units)
            .ok_or_else(|| anyhow::anyhow!("Stock underflow"))?;
    }
    stock[recipe.item] = stock[recipe.item]
        .checked_add(recipe.output)
        .ok_or_else(|| anyhow::anyhow!("Stock overflow"))?;
    Ok(())
}
fn undo(stock: &mut [u64], recipe: &Indexed) -> Result<()> {
    stock[recipe.item] = stock[recipe.item]
        .checked_sub(recipe.output)
        .ok_or_else(|| anyhow::anyhow!("Stock underflow"))?;
    for (item, units) in &recipe.inputs {
        stock[*item] = stock[*item]
            .checked_add(*units)
            .ok_or_else(|| anyhow::anyhow!("Stock overflow"))?;
    }
    Ok(())
}

pub(crate) fn review(snapshot: &Snapshot, recipes: &[Recipe], page: &Page) -> Result<Value> {
    page.validate()?;
    ensure!(
        snapshot.client.family == Family::Forever && (1..=8).contains(&recipes.len()),
        "Invalid stock schedule context"
    );
    let mut recipes = recipes.iter().collect::<Vec<_>>();
    recipes.sort_by_key(|recipe| recipe.id);
    ensure!(
        recipes.iter().map(|r| r.id).collect::<BTreeSet<_>>().len() == recipes.len()
            && recipes
                .iter()
                .all(|r| r.crafts <= 1000 && r.output > 0 && r.reagents.len() <= 64),
        "Invalid stock schedule recipes"
    );
    let items = recipes
        .iter()
        .flat_map(|recipe| std::iter::once(recipe.item).chain(recipe.reagents.keys().copied()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let index = items
        .iter()
        .enumerate()
        .map(|(index, item)| (*item, index))
        .collect::<BTreeMap<_, _>>();
    let mut initial = vec![0u64; items.len()];
    let bags = snapshot
        .sections
        .get("bags")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Bag observation unavailable"))?;
    for container in bags.values() {
        for row in container
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid bags"))?
            .values()
        {
            if let Some(slot) = row["item_id"].as_u64().and_then(|id| index.get(&id)) {
                initial[*slot] = initial[*slot]
                    .checked_add(
                        row["count"]
                            .as_u64()
                            .ok_or_else(|| anyhow::anyhow!("Stock unavailable"))?,
                    )
                    .ok_or_else(|| anyhow::anyhow!("Stock overflow"))?;
            }
        }
    }
    let model = recipes
        .iter()
        .map(|recipe| Indexed {
            id: recipe.id,
            item: index[&recipe.item],
            output: recipe.output,
            inputs: recipe
                .reagents
                .iter()
                .map(|(id, units)| (index[id], *units))
                .collect(),
        })
        .collect::<Vec<_>>();
    let mut stock = initial.clone();
    let mut left = recipes
        .iter()
        .map(|recipe| recipe.crafts as u16)
        .collect::<Vec<_>>();
    let mut visited = HashSet::from([left.clone()]);
    let mut frames = vec![0usize];
    let mut path = Vec::new();
    let mut found = false;
    let mut exhausted = false;
    let mut budget_hit = false;
    loop {
        if left.iter().all(|n| *n == 0) {
            found = true;
            break;
        }
        let Some(next) = frames.last_mut() else {
            exhausted = true;
            break;
        };
        if *next == model.len() {
            frames.pop();
            if let Some(previous) = path.pop() {
                undo(&mut stock, &model[previous])?;
                left[previous] += 1;
            }
            continue;
        }
        let candidate = *next;
        *next += 1;
        if left[candidate] == 0
            || model[candidate]
                .inputs
                .iter()
                .any(|(item, units)| stock[*item] < *units)
        {
            continue;
        }
        apply(&mut stock, &model[candidate])?;
        left[candidate] -= 1;
        if visited.contains(&left) {
            undo(&mut stock, &model[candidate])?;
            left[candidate] += 1;
            continue;
        }
        if visited.len() >= page.state_budget as usize {
            undo(&mut stock, &model[candidate])?;
            left[candidate] += 1;
            budget_hit = true;
            break;
        }
        visited.insert(left.clone());
        path.push(candidate);
        frames.push(0);
    }
    let mut runs = Vec::<(u64, u32)>::new();
    let mut targets = Value::Null;
    let mut witness = Value::Null;
    if found {
        let mut replay = initial.clone();
        let mut used = vec![0u64; recipes.len()];
        for &step in &path {
            ensure!(
                model[step]
                    .inputs
                    .iter()
                    .all(|(item, units)| replay[*item] >= *units),
                "Invalid stock witness"
            );
            apply(&mut replay, &model[step])?;
            used[step] += 1;
            if let Some((_, count)) = runs.last_mut().filter(|(id, _)| *id == model[step].id) {
                *count += 1;
            } else {
                runs.push((model[step].id, 1));
            }
        }
        ensure!(
            replay == stock
                && recipes
                    .iter()
                    .enumerate()
                    .all(|(i, recipe)| used[i] == recipe.crafts
                        && replay[index[&recipe.item]] >= recipe.target),
            "Stock witness does not meet fixed targets"
        );
        targets=json!(recipes.iter().map(|recipe|json!({"recipe_id":recipe.id,"output_item_id":recipe.item,
            "target_bag_quantity":recipe.target,"minimum_final_bag_quantity":replay[index[&recipe.item]]})).collect::<Vec<_>>());
        let binding = json!({"initial":items.iter().zip(&initial).collect::<Vec<_>>(),
            "recipes":recipes.iter().map(|r|json!({"id":r.id,"item":r.item,"output":r.output,"target":r.target,
                "crafts":r.crafts,"reagents":r.reagents})).collect::<Vec<_>>(),"steps":runs});
        witness = json!(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&binding)?)
        ));
    }
    let total = runs.len();
    let offset = page.offset as usize;
    let take = page.limit as usize;
    let steps=runs.iter().enumerate().skip(offset).take(take)
        .map(|(number,(recipe_id,crafts))|json!({"step":number+1,"recipe_id":recipe_id,"crafts":crafts})).collect::<Vec<_>>();
    let more = total.saturating_sub(offset) > take;
    Ok(
        json!({"basis":"captured_bags_and_minimum_base_output_for_fixed_planned_counts",
        "status":if found {"witness_found"}else if exhausted {"no_schedule_for_planned_counts"}else{"state_budget_exhausted"},
        "schedule_complete":found,"decision_complete":found||exhausted,"state_budget_exhausted":budget_hit,
        "states_visited":visited.len(),"state_budget":page.state_budget,"total_crafts":recipes.iter().map(|r|r.crafts).sum::<u64>(),
        "witness_sha256":witness,"total_steps":if found {Some(total)}else{None},"steps":steps,
        "offset":page.offset,"limit":page.limit,"has_more":more,"next_offset":more.then_some(offset+steps.len()),
        "final_targets":targets,"banks_mail_and_acquisitions_included":false,
        "per_craft_output_credit":true,"material_ledger_scope":"individual_crafts_separate_from_parent_gross_steps",
        "step_semantics":"repeat_individual_craft_with_output_available_after_each",
        "conditions_cooldowns_and_charges_checked":false,"current_inventory_verified":false,
        "native_craftability_verified":false,"item_variant_equivalence_verified":false,
        "optimal_order_verified":false,"game_execution":false,"live":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(stock: &[(u64, u64)]) -> Snapshot {
        let rows = stock
            .iter()
            .enumerate()
            .map(|(i, (id, count))| ((i + 1).to_string(), json!({"item_id":id,"count":count})))
            .collect::<serde_json::Map<_, _>>();
        let s:Snapshot=serde_json::from_value(json!({"schema_version":1,"source":"fixture",
            "captured_at":"2025-01-01T00:00:00Z","client":{"family":"forever","variant":"live","build":"70862","interface":16001},
            "character":{"name":"Fixture","realm":"Test"},"sections":{"bags":{"0":rows},
            "character_bank":{"6":{"1":{"item_id":30,"count":100}}}},"unavailable":{}})).unwrap();
        s.validate().unwrap();
        s
    }
    fn recipe(id: u64, item: u64, output: u64, inputs: &[(u64, u64)]) -> Recipe {
        Recipe {
            id,
            item,
            output,
            target: 1,
            crafts: 1,
            reagents: inputs.iter().copied().collect(),
        }
    }
    fn trap() -> Vec<Recipe> {
        vec![
            recipe(100, 10, 1, &[(30, 1)]),
            recipe(200, 20, 2, &[(30, 1)]),
            recipe(300, 30, 2, &[(20, 1)]),
        ]
    }
    fn page() -> Page {
        Page {
            state_budget: 10000,
            offset: 0,
            limit: 50,
        }
    }
    #[test]
    fn backtracking_finds_seeded_cycle_instead_of_stopping_at_first_dead_end() {
        let s = snapshot(&[(30, 1)]);
        let before = serde_json::to_value(&s).unwrap();
        let r = review(&s, &trap(), &page()).unwrap();
        assert_eq!(r["status"], "witness_found");
        assert_eq!(r["states_visited"], 5);
        assert_eq!(
            r["steps"],
            json!([{"step":1,"recipe_id":200,"crafts":1},
            {"step":2,"recipe_id":300,"crafts":1},{"step":3,"recipe_id":100,"crafts":1}])
        );
        assert_eq!(r["schedule_complete"], true);
        assert_eq!(r["native_craftability_verified"], false);
        assert_eq!(before, serde_json::to_value(&s).unwrap());
    }
    #[test]
    fn no_seed_and_small_budget_are_different_and_never_return_partial_advice() {
        let mut recipes = trap();
        recipes[2].output = 3;
        let impossible = review(&snapshot(&[]), &recipes, &page()).unwrap();
        assert_eq!(impossible["status"], "no_schedule_for_planned_counts");
        assert_eq!(impossible["decision_complete"], true);
        assert_eq!(impossible["states_visited"], 1);
        let mut small = page();
        small.state_budget = 2;
        let unknown = review(&snapshot(&[(30, 1)]), &trap(), &small).unwrap();
        assert_eq!(unknown["status"], "state_budget_exhausted");
        assert_eq!(unknown["decision_complete"], false);
        for report in [&impossible, &unknown] {
            assert_eq!(report["steps"], json!([]));
            assert!(report["witness_sha256"].is_null());
            assert!(report["final_targets"].is_null());
            assert_eq!(report["game_execution"], false);
        }
    }
    #[test]
    fn pages_keep_one_witness_and_support_recycling_within_a_recipe() {
        let s = snapshot(&[(30, 1)]);
        let full = review(&s, &trap(), &page()).unwrap();
        let mut p = page();
        p.offset = 1;
        p.limit = 1;
        let middle = review(&s, &trap(), &p).unwrap();
        assert_eq!(middle["witness_sha256"], full["witness_sha256"]);
        assert_eq!(
            middle["steps"],
            json!([{"step":2,"recipe_id":300,"crafts":1}])
        );
        assert_eq!(middle["has_more"], true);
        assert_eq!(middle["next_offset"], 2);
        p.offset = u32::MAX;
        assert_eq!(review(&s, &trap(), &p).unwrap()["steps"], json!([]));
        let mut self_recipe = recipe(100, 10, 2, &[(10, 1)]);
        self_recipe.crafts = 2;
        self_recipe.target = 3;
        let r = review(&snapshot(&[(10, 1)]), &[self_recipe], &page()).unwrap();
        assert_eq!(r["steps"], json!([{"step":1,"recipe_id":100,"crafts":2}]));
        assert_eq!(r["final_targets"][0]["minimum_final_bag_quantity"], 3);
    }
    #[test]
    fn maximum_depth_is_iterative_and_invalid_bounds_fail_closed() {
        let recipes = (1..=8)
            .map(|i| Recipe {
                id: i,
                item: i,
                output: 1,
                target: 1000,
                crafts: 1000,
                reagents: BTreeMap::new(),
            })
            .collect::<Vec<_>>();
        let s = snapshot(&[]);
        let r = review(&s, &recipes, &page()).unwrap();
        assert_eq!(r["schedule_complete"], true);
        assert_eq!(r["states_visited"], 8001);
        assert_eq!(r["total_crafts"], 8000);
        assert_eq!(r["total_steps"], 8);
        for (budget, limit) in [(0, 20), (50001, 20), (10, 0), (10, 51)] {
            assert!(
                review(
                    &s,
                    &recipes,
                    &Page {
                        state_budget: budget,
                        offset: 0,
                        limit
                    }
                )
                .is_err()
            );
        }
    }
}
