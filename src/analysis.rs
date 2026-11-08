//! Snapshot-only analysis. Missing data is not an empty inventory.
use crate::Snapshot;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct ObservedReagent {
    quantities: [u64; 3],
    stack_count: usize,
    locations: Vec<Value>,
}

pub fn review_recipe(snapshot: &Snapshot, crafts: u32) -> Result<Value> {
    review_recipe_with_inventory(snapshot, crafts, None)
}

pub fn recipe_tracking(snapshot: &Snapshot, recipe_id: u64) -> Value {
    let tracked = snapshot.sections.get("tracked_recipes");
    let key = recipe_id.to_string();
    let observation = |group: &str| tracked?.get(group)?.as_object().map(|rows| rows.get(&key));
    let craft = observation("crafts");
    let recraft = observation("recrafts");
    json!({
        "tracked_as_craft":craft.map(|row| row.is_some()),
        "tracked_as_recraft":recraft.map(|row| row.is_some()),
        "tracked_craft_learned":craft.flatten().and_then(|row| row["learned"].as_bool()),
        "tracked_recraft_learned":recraft.flatten().and_then(|row| row["learned"].as_bool())
    })
}

pub fn plan_recipe(
    snapshot: &Snapshot,
    crafts: Option<u32>,
    target_output_quantity: Option<std::num::NonZeroU32>,
    inventory: Option<&Snapshot>,
) -> Result<Value> {
    match (crafts, target_output_quantity) {
        (Some(crafts), None) => review_recipe_with_inventory(snapshot, crafts, inventory),
        (None, Some(target)) => {
            let minimum = snapshot
                .sections
                .get("selected_recipe")
                .and_then(|recipe| recipe["base_output_quantity"]["minimum"].as_u64())
                .and_then(|quantity| u32::try_from(quantity).ok())
                .filter(|quantity| *quantity > 0)
                .ok_or_else(|| anyhow::anyhow!("Recipe base output range unavailable"))?;
            let crafts = target.get().div_ceil(minimum);
            let mut review = review_recipe_with_inventory(snapshot, crafts, inventory)?;
            review["target_output_quantity"] = json!(target.get());
            Ok(review)
        }
        _ => anyhow::bail!("Supply one craft count or target output quantity"),
    }
}

pub fn review_recipe_with_inventory(
    snapshot: &Snapshot,
    crafts: u32,
    inventory: Option<&Snapshot>,
) -> Result<Value> {
    ensure!(
        snapshot.client.family == crate::Family::Forever,
        "Expected a Forever snapshot"
    );
    ensure!((1..=1000).contains(&crafts), "Craft count out of range");
    let recipe = snapshot
        .sections
        .get("selected_recipe")
        .ok_or_else(|| anyhow::anyhow!("Selected recipe was not captured"))?;
    if let Some(inventory) = inventory {
        ensure_comparable(snapshot, inventory)?;
    }
    let stock = inventory.unwrap_or(snapshot);
    let fixed = recipe["requirements"] == "fixed_items";
    let storage = ["bags", "character_bank", "account_bank"];
    let mut observed: BTreeMap<&str, ObservedReagent> = BTreeMap::new();
    let mut missing_storage = Vec::new();
    for (index, name) in storage.into_iter().enumerate() {
        let Some(containers) = stock.sections.get(name).and_then(Value::as_object) else {
            missing_storage.push(name);
            continue;
        };
        for (container, items) in containers {
            let Some(slots) = items.as_object() else {
                continue;
            };
            for (slot, item) in slots {
                if let (Some(id), Some(count)) = (item["item_id"].as_u64(), item["count"].as_u64())
                    && let Some((key, _)) = recipe["reagents"]
                        .as_object()
                        .and_then(|map| map.get_key_value(&id.to_string()))
                {
                    let reagent = observed.entry(key).or_default();
                    reagent.quantities[index] += count;
                    reagent.stack_count += 1;
                    if reagent.locations.len() < 20 {
                        reagent.locations.push(json!({
                            "storage":name,"container":container,"slot":slot,"count":count
                        }));
                    }
                }
            }
        }
    }
    let mut materials = Vec::new();
    let mut bag_limits = Vec::new();
    let mut observed_limits = Vec::new();
    if fixed {
        for (id, quantity) in recipe["reagents"]
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid reagent data"))?
        {
            let per_craft = quantity
                .as_u64()
                .filter(|quantity| *quantity > 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid reagent quantity"))?;
            let item_id = id.parse::<u32>()?;
            let required = per_craft * u64::from(crafts);
            let reagent = observed.remove(id.as_str()).unwrap_or_default();
            let quantities = reagent.quantities;
            let available: u64 = quantities.iter().sum();
            bag_limits.push((item_id, quantities[0] / per_craft));
            observed_limits.push((item_id, available / per_craft));
            let by_storage: BTreeMap<_, _> = storage
                .into_iter()
                .zip(quantities)
                .map(|(name, count)| (name, (!missing_storage.contains(&name)).then_some(count)))
                .collect();
            let by_storage = json!(by_storage);
            let (bag_shortfall, withdrawals) = bag_restock(&by_storage, required);
            materials.push(json!({"item_id":item_id, "required":required,
                "quantity_observed":(missing_storage.len()<storage.len()).then_some(available),
                "shortfall_vs_observed":(missing_storage.len()<storage.len()).then_some(required.saturating_sub(available)),
                "quantity_by_storage":by_storage,
                "shortfall_vs_bags":bag_shortfall,"suggested_bank_withdrawals":withdrawals,
                "observed_stack_count":(missing_storage.len()<storage.len()).then_some(reagent.stack_count),
                "observed_locations":reagent.locations}));
        }
    }
    let capacity = |limits: &[(u32, u64)], observed| {
        if observed && let Some(maximum) = limits.iter().map(|(_, count)| *count).min() {
            let mut limiting_item_ids: Vec<_> = limits
                .iter()
                .filter_map(|(id, count)| (*count == maximum).then_some(*id))
                .collect();
            limiting_item_ids.sort_unstable();
            return json!({"supported_crafts":maximum,"limiting_item_ids":limiting_item_ids});
        }
        Value::Null
    };
    let base_output_if_completed = recipe["base_output_quantity"]["minimum"]
        .as_u64()
        .zip(recipe["base_output_quantity"]["maximum"].as_u64())
        .map(|(minimum, maximum)| {
            json!({"item_id":recipe["output_item_id"],"minimum":minimum * u64::from(crafts),
                "maximum":maximum * u64::from(crafts)})
        });
    let recipe_id = recipe["recipe_id"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid recipe ID"))?;
    let charges = recipe["cooldown"]["charges"].as_u64();
    let estimated_cooldown_end = recipe["cooldown"]["remaining_seconds"]
        .as_i64()
        .filter(|seconds| *seconds > 0)
        .and_then(chrono::TimeDelta::try_seconds)
        .and_then(|remaining| snapshot.captured_at.checked_add_signed(remaining));
    let bags_capacity = capacity(&bag_limits, !missing_storage.contains(&"bags"));
    let observed_capacity = capacity(&observed_limits, missing_storage.len() < storage.len());
    let combined_capacity = |materials: &Value| {
        materials["supported_crafts"]
            .as_u64()
            .zip(charges)
            .map(|(material_limit, charges)| {
                let supported = material_limit.min(charges);
                json!({"supported_crafts":supported,
                "shortfall_vs_requested":u64::from(crafts).saturating_sub(supported)})
            })
    };
    let mut review = json!({"captured_at":snapshot.captured_at,"recipe":recipe,"requested_crafts":crafts,
        "recipe_tracking":recipe_tracking(snapshot, recipe_id),
        "profession_points_to_observed_cap":recipe["profession"]["max_rank"].as_u64().filter(|cap| *cap > 0)
            .zip(recipe["profession"]["rank"].as_u64()).and_then(|(cap, rank)| cap.checked_sub(rank)),
        "base_output_if_completed":base_output_if_completed,
        "estimated_cooldown_end_at":estimated_cooldown_end,
        "shortfall_vs_observed_recipe_charges":charges
            .map(|charges| u64::from(crafts).saturating_sub(charges)),
        "unmet_crafting_conditions":recipe["crafting_conditions"].as_object().map(|conditions|
            conditions.iter().filter(|(_, condition)| condition["met"].as_bool() == Some(false))
                .collect::<BTreeMap<_, _>>()),
        "materials":materials,"unobserved_storage":missing_storage,"live":false,
        "material_capacity":{
            "bags":bags_capacity,
            "all_observed_storage":observed_capacity
        },
        "material_and_charge_capacity":{
            "bags":combined_capacity(&bags_capacity),
            "all_observed_storage":combined_capacity(&observed_capacity)
        },
        "craftability_verified":false,"game_execution":false});
    if let Some(inventory) = inventory {
        review["inventory_captured_at"] = json!(inventory.captured_at);
    }
    Ok(review)
}

/// Combine demands from reviews already validated against the same inventory capture.
pub fn combine_recipe_materials<'a>(reviews: impl Iterator<Item = &'a Value>) -> Result<Value> {
    let mut totals = BTreeMap::<u64, (u64, Option<u64>, &Value)>::new();
    for review in reviews {
        if review["recipe"]["requirements"] != "fixed_items" {
            return Ok(Value::Null);
        }
        for material in review["materials"].as_array().into_iter().flatten() {
            let id = material["item_id"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing reagent ID"))?;
            let required = material["required"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing reagent quantity"))?;
            let observed = material["quantity_observed"].as_u64();
            let by_storage = &material["quantity_by_storage"];
            let total = totals.entry(id).or_insert((0, observed, by_storage));
            ensure!(
                total.1 == observed && total.2 == by_storage,
                "Inventory observations differ"
            );
            total.0 = total
                .0
                .checked_add(required)
                .ok_or_else(|| anyhow::anyhow!("Reagent quantity overflow"))?;
        }
    }
    Ok(json!(
        totals
            .into_iter()
            .map(|(item_id, (required, observed, by_storage))| json!({
                "item_id":item_id,"required":required,"quantity_observed":observed,
                "shortfall_vs_observed":observed.map(|quantity| required.saturating_sub(quantity)),
                "quantity_by_storage":by_storage,
                "shortfall_vs_bags":by_storage["bags"].as_u64().map(|count| required.saturating_sub(count))
            }))
            .collect::<Vec<_>>()
    ))
}

/// Model minimum yields in supplied recipe order, consuming reagents before producing output.
pub fn ordered_recipe_materials<'a>(
    reviews: impl Iterator<Item = &'a Value>,
    combined_materials: &Value,
) -> Result<Value> {
    let Some(materials) = combined_materials.as_array() else {
        return Ok(Value::Null);
    };
    let mut balance = BTreeMap::<u64, i64>::new();
    let mut starting_stock = BTreeMap::<u64, u64>::new();
    for review in reviews {
        for material in review["materials"].as_array().into_iter().flatten() {
            let item = material["item_id"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing reagent ID"))?;
            let required = material["required"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("Missing reagent quantity"))?;
            let available = balance.entry(item).or_default();
            *available = available
                .checked_sub(required)
                .ok_or_else(|| anyhow::anyhow!("Reagent balance overflow"))?;
            let minimum = starting_stock.entry(item).or_default();
            *minimum = (*minimum).max((*available).min(0).unsigned_abs());
        }
        let output = &review["base_output_if_completed"];
        if let Some((item, minimum)) = output["item_id"].as_u64().zip(output["minimum"].as_i64()) {
            let available = balance.entry(item).or_default();
            *available = available
                .checked_add(minimum)
                .ok_or_else(|| anyhow::anyhow!("Output balance overflow"))?;
        }
    }
    let mut starting_materials = Vec::new();
    for material in materials {
        let item = material["item_id"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing combined reagent ID"))?;
        let required = *starting_stock
            .get(&item)
            .ok_or_else(|| anyhow::anyhow!("Missing starting stock estimate"))?;
        let observed = material["quantity_observed"].as_u64();
        let by_storage = &material["quantity_by_storage"];
        let (bag_shortfall, withdrawals) = bag_restock(by_storage, required);
        starting_materials.push(json!({
            "item_id":item,"required_before_batch":required,"quantity_observed":observed,
            "shortfall_vs_observed":observed.map(|quantity| required.saturating_sub(quantity)),
            "shortfall_vs_bags":bag_shortfall,"suggested_bank_withdrawals":withdrawals
        }));
    }
    Ok(json!(starting_materials))
}

pub fn bag_restock(by_storage: &Value, required: u64) -> (Option<u64>, Option<Value>) {
    let bag_shortfall = by_storage["bags"]
        .as_u64()
        .map(|count| required.saturating_sub(count));
    let withdrawals = bag_shortfall.map(|shortfall| {
        let character_bank = by_storage["character_bank"]
            .as_u64()
            .map(|count| count.min(shortfall));
        let remaining = shortfall - character_bank.unwrap_or(0);
        let account_bank = by_storage["account_bank"]
            .as_u64()
            .map(|count| count.min(remaining));
        json!({"character_bank":character_bank,"account_bank":account_bank})
    });
    (bag_shortfall, withdrawals)
}

pub fn combine_recipe_outputs<'a>(reviews: impl Iterator<Item = &'a Value>) -> Value {
    let mut totals = BTreeMap::<u64, Option<(u64, u64)>>::new();
    let mut recipes_without_observed_output_item = 0;
    for review in reviews {
        let Some(item_id) = review["recipe"]["output_item_id"].as_u64() else {
            recipes_without_observed_output_item += 1;
            continue;
        };
        let output = &review["base_output_if_completed"];
        let range = output["minimum"].as_u64().zip(output["maximum"].as_u64());
        let total = totals.entry(item_id).or_insert(Some((0, 0)));
        *total = total
            .zip(range)
            .and_then(|((a_min, a_max), (b_min, b_max))| {
                Some((a_min.checked_add(b_min)?, a_max.checked_add(b_max)?))
            });
    }
    let items: Vec<_> = totals
        .into_iter()
        .map(|(item_id, range)| {
            json!({"item_id":item_id,"minimum":range.map(|(minimum, _)| minimum),
                "maximum":range.map(|(_, maximum)| maximum)})
        })
        .collect();
    json!({"items":items,"recipes_without_observed_output_item":recipes_without_observed_output_item})
}

pub fn find_item(snapshot: &Snapshot, item_id: u32, offset: usize, limit: usize) -> Result<Value> {
    ensure!(
        item_id > 0 && (1..=50).contains(&limit),
        "Invalid item query"
    );
    let mut locations = Vec::new();
    let mut matched_locations = 0usize;
    let mut quantities = BTreeMap::new();
    for storage in ["equipment", "bags", "character_bank", "account_bank"] {
        if storage.ends_with("_bank") && snapshot.client.family != crate::Family::Forever {
            continue;
        }
        let Some(section) = snapshot.sections.get(storage).and_then(Value::as_object) else {
            quantities.insert(storage, None);
            continue;
        };
        let mut quantity = 0;
        let mut record = |container: Option<&str>, slot: &str, item: &Value, count: u64| {
            if item["item_id"].as_u64() == Some(u64::from(item_id)) {
                quantity += count;
                if matched_locations >= offset && locations.len() < limit {
                    locations.push(json!({
                        "storage":storage,"container":container,"slot":slot,"count":count
                    }));
                }
                matched_locations += 1;
            }
        };
        for (key, item) in section {
            if storage == "equipment" {
                record(None, key, item, 1);
            } else if let Some(slots) = item.as_object() {
                for (slot, observed) in slots {
                    if let Some(count) = observed["count"].as_u64() {
                        record(Some(key), slot, observed, count);
                    }
                }
            }
        }
        quantities.insert(storage, Some(quantity));
    }
    Ok(json!({
        "captured_at":snapshot.captured_at,"item_id":item_id,"live":false,
        "locations":locations,"matched_locations":matched_locations,
        "quantity_by_storage":quantities,"has_more":matched_locations.saturating_sub(offset)>limit
    }))
}

fn compare_item(before: &Snapshot, after: &Snapshot, item_id: u32) -> Result<Value> {
    let before = find_item(before, item_id, 0, 1)?;
    let after = find_item(after, item_id, 0, 1)?;
    let quantities = after["quantity_by_storage"].as_object().unwrap();
    let mut net_delta = Some(0i64);
    let mut by_storage = BTreeMap::new();
    for (storage, current) in quantities {
        let previous = &before["quantity_by_storage"][storage];
        let delta = previous.as_i64().zip(current.as_i64()).map(|(a, b)| b - a);
        net_delta = net_delta.zip(delta).map(|(total, change)| total + change);
        by_storage.insert(
            storage,
            json!({"before":previous,"after":current,"delta":delta}),
        );
    }
    Ok(json!({"item_id":item_id,"by_storage":by_storage,"net_delta":net_delta}))
}

fn carried_bag_space(snapshot: &Snapshot) -> Option<Value> {
    let bags = snapshot.sections.get("bag_capacity")?.as_object()?;
    let total = bags
        .values()
        .filter_map(|bag| bag["total_slots"].as_i64())
        .sum::<i64>();
    let free = bags
        .values()
        .filter_map(|bag| bag["free_slots"].as_i64())
        .sum::<i64>();
    let general = bags
        .values()
        .filter(|bag| bag["family_mask"].as_u64() == Some(0))
        .filter_map(|bag| bag["free_slots"].as_i64())
        .sum::<i64>();
    Some(json!({"total_slots":total,"free_slots":free,"known_general_free_slots":general}))
}

fn carried_items(snapshot: &Snapshot) -> Option<Value> {
    let bags = snapshot.sections.get("bags")?.as_object()?;
    let mut groups: BTreeMap<u64, (u64, usize, Vec<Value>)> = BTreeMap::new();
    let mut total = 0u64;
    for (bag, slots) in bags {
        for (slot, item) in slots.as_object()? {
            let id = item.get("item_id")?.as_u64()?;
            let count = item.get("count")?.as_u64()?;
            total = total.checked_add(count)?;
            let group = groups.entry(id).or_default();
            group.0 = group.0.checked_add(count)?;
            group.1 += 1;
            if group.2.len() < 20 {
                group.2.push(json!({"bag":bag,"slot":slot,"count":count}));
            }
        }
    }
    let repeated_count = groups.values().filter(|group| group.1 > 1).count();
    let repeated: Vec<_> = groups
        .iter()
        .filter(|(_, group)| group.1 > 1)
        .take(20)
        .map(|(id, group)| {
            json!({"item_id":id,"quantity_observed":group.0,
            "occupied_slots_observed":group.1,"locations":group.2,
            "locations_truncated":group.1>group.2.len()})
        })
        .collect();
    Some(
        json!({"quantity_observed":total,"distinct_item_ids_observed":groups.len(),
        "repeated_item_id_count":repeated_count,"repeated_item_ids":repeated,
        "groups_truncated":repeated_count>repeated.len(),
        "grouping":"item_id_only_not_item_identity",
        "stack_compatibility":"unknown",
        "free_slots":if snapshot.sections.contains_key("bag_capacity") {"captured"} else {"not_captured"},
        "game_execution":false}),
    )
}

pub fn compare(before: &Snapshot, after: &Snapshot) -> Result<Value> {
    compare_with_item(before, after, None)
}

pub fn compare_with_item(
    before: &Snapshot,
    after: &Snapshot,
    item_id: Option<u32>,
) -> Result<Value> {
    compare_observations(
        before,
        after,
        ComparisonSelectors {
            item_id,
            ..ComparisonSelectors::default()
        },
    )
}

#[derive(Default)]
pub struct ComparisonSelectors {
    pub item_id: Option<u32>,
    pub currency_id: Option<u32>,
    pub faction_id: Option<u32>,
    pub quest_id: Option<u32>,
    pub quest_waypoints: bool,
    pub achievement_id: Option<u32>,
    pub profession_skill_line: Option<u32>,
    pub wallet: bool,
    pub experience: bool,
    pub spellbook: bool,
    pub action_bar_slots: bool,
}

pub fn compare_observations(
    before: &Snapshot,
    after: &Snapshot,
    selectors: ComparisonSelectors,
) -> Result<Value> {
    ensure_comparable(before, after)?;
    let names: BTreeSet<_> = before
        .sections
        .keys()
        .chain(after.sections.keys())
        .chain(before.unavailable.keys())
        .chain(after.unavailable.keys())
        .collect();
    let mut sections = serde_json::Map::new();
    for name in names {
        let result = match (before.sections.get(name), after.sections.get(name)) {
            (Some(a), Some(b)) => {
                let mut paths = Vec::new();
                let mut count = 0;
                diff(a, b, "", &mut paths, &mut count);
                json!({"status":if count==0 {"unchanged"} else {"changed"},"changed_paths":paths,"change_count":count,"truncated":count>paths.len()})
            }
            _ => {
                json!({"status":"not_comparable","before_available":before.sections.contains_key(name),"after_available":after.sections.contains_key(name)})
            }
        };
        sections.insert(name.clone(), result);
    }
    let mut comparison = json!({"client":after.client,"character":after.character,
        "before_captured_at":before.captured_at,"after_captured_at":after.captured_at,
        "live":false,"sections":sections});
    let skills_before = before
        .sections
        .get("skill_lines")
        .and_then(Value::as_object);
    let skills_after = after.sections.get("skill_lines").and_then(Value::as_object);
    if skills_before.is_some() || skills_after.is_some() {
        comparison["skill_line_progress"] = skills_before
            .zip(skills_after)
            .map(|(before, after)| compare_skill_lines(before, after))
            .unwrap_or(Value::Null);
    }
    let flight_before = before.sections.get("flight_map");
    let flight_after = after.sections.get("flight_map");
    if flight_before.is_some() || flight_after.is_some() {
        comparison["flight_map_changes"] = flight_before
            .zip(flight_after)
            .map(|(before, after)| compare_flight_maps(before, after))
            .unwrap_or(Value::Null);
    }
    let previous_space = carried_bag_space(before);
    let current_space = carried_bag_space(after);
    if previous_space.is_some() || current_space.is_some() {
        let delta = |field: &str| {
            Some(
                current_space.as_ref()?[field].as_i64()?
                    - previous_space.as_ref()?[field].as_i64()?,
            )
        };
        comparison["carried_bag_space_change"] = json!({
            "total_slots_delta":delta("total_slots"),
            "free_slots_delta":delta("free_slots"),
            "before":previous_space,
            "after":current_space,
        });
    }
    let tracked_before = before.sections.get("tracked_recipes");
    let tracked_after = after.sections.get("tracked_recipes");
    if tracked_before.is_some() || tracked_after.is_some() {
        comparison["tracked_recipe_changes"] = tracked_before
            .zip(tracked_after)
            .map(|(before, after)| {
                json!({
                    "crafts":tracked_recipe_membership(&before["crafts"], &after["crafts"]),
                    "recrafts":tracked_recipe_membership(&before["recrafts"], &after["recrafts"]),
                })
            })
            .unwrap_or(Value::Null);
        comparison["tracked_recipe_learned_changes"] = tracked_before
            .zip(tracked_after)
            .map(|(before, after)| json!({
                "crafts":tracked_recipe_learned_changes(&before["crafts"], &after["crafts"]),
                "recrafts":tracked_recipe_learned_changes(&before["recrafts"], &after["recrafts"])
            })).unwrap_or(Value::Null);
    }
    if let Some(item_id) = selectors.item_id {
        comparison["item_quantity_change"] = compare_item(before, after, item_id)?;
    }
    if let Some(currency_id) = selectors.currency_id {
        ensure!(
            before.client.family == crate::Family::Forever && currency_id > 0,
            "Currency balance comparisons require a Forever currency ID"
        );
        let key = currency_id.to_string();
        let balance = |snapshot: &Snapshot| {
            snapshot
                .sections
                .get("currency_balances")
                .and_then(|section| section["entries"].get(&key))
                .cloned()
        };
        let previous = balance(before);
        let current = balance(after);
        let change = previous.as_ref().zip(current.as_ref()).and_then(|(a, b)| {
            if a["account_wide"] != b["account_wide"] {
                return None;
            }
            let from = a["quantity"].as_u64()?;
            let to = b["quantity"].as_u64()?;
            let direction = match to.cmp(&from) {
                std::cmp::Ordering::Greater => "increased",
                std::cmp::Ordering::Less => "decreased",
                std::cmp::Ordering::Equal => "unchanged",
            };
            Some(json!({"direction":direction,"amount":to.abs_diff(from)}))
        });
        comparison["currency_balance_change"] =
            json!({"currency_id":currency_id,"before":previous,"after":current,"change":change});
    }
    if let Some(faction_id) = selectors.faction_id {
        ensure!(
            before.client.family == crate::Family::Forever && faction_id > 0,
            "Reputation comparisons require a Forever faction ID"
        );
        let key = faction_id.to_string();
        let faction = |snapshot: &Snapshot| {
            snapshot
                .sections
                .get("reputation")
                .and_then(|section| section["entries"].get(&key))
                .cloned()
        };
        let previous = faction(before);
        let current = faction(after);
        let standing_delta = previous.as_ref().zip(current.as_ref()).and_then(|(a, b)| {
            if a["kind"] != "standard"
                || b["kind"] != "standard"
                || a["account_wide"] != b["account_wide"]
            {
                return None;
            }
            Some(b["standing"].as_i64()? - a["standing"].as_i64()?)
        });
        let special_progress_change = previous
            .as_ref()
            .zip(current.as_ref())
            .and_then(|(before, after)| compare_special_reputation(before, after));
        comparison["reputation_change"] = json!({
            "faction_id":faction_id,"before":previous,"after":current,"standing_delta":standing_delta,
            "special_progress_change":special_progress_change
        });
    }
    if let Some(quest_id) = selectors.quest_id {
        ensure!(quest_id > 0, "Expected a positive quest ID");
        let key = quest_id.to_string();
        let quest = |snapshot: &Snapshot| {
            snapshot
                .sections
                .get("quests")
                .and_then(|section| section.get(&key))
                .cloned()
        };
        let previous = quest(before);
        let current = quest(after);
        comparison["quest_change"] = json!({
            "quest_id":quest_id,"before":previous,"after":current,
            "ready_for_turn_in_changed":observed_flag_changed(previous.as_ref(), current.as_ref(), "ready_for_turn_in"),
            "failed_changed":observed_flag_changed(previous.as_ref(), current.as_ref(), "failed")
        });
    }
    if selectors.quest_waypoints {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Quest waypoint comparison requires Forever"
        );
        let quest_rows = before
            .sections
            .get("quests")
            .and_then(Value::as_object)
            .zip(after.sections.get("quests").and_then(Value::as_object));
        comparison["quest_waypoint_changes"] = json!(quest_rows.map(|(old, new)| {
            let mut changes = BTreeMap::new();
            for (key, previous) in old {
                let Some(current) = new.get(key) else {
                    continue;
                };
                let Ok(id) = key.parse::<u32>() else { continue };
                let from = previous["next_waypoint_map_id"].as_u64();
                let to = current["next_waypoint_map_id"].as_u64();
                let status = match (from, to) {
                    (Some(a), Some(b)) if a != b => "map_changed",
                    (None, Some(_)) => "map_became_observed",
                    (Some(_), None) => "map_became_unknown",
                    _ => continue,
                };
                changes.insert(
                    id,
                    json!({"quest_id":id,"before_map_id":from,"after_map_id":to,"status":status}),
                );
            }
            changes.into_values().collect::<Vec<_>>()
        }));
    }
    if let Some(achievement_id) = selectors.achievement_id {
        ensure!(
            before.client.family == crate::Family::Forever && achievement_id > 0,
            "Tracked achievement comparisons require a Forever achievement ID"
        );
        let key = achievement_id.to_string();
        let achievement = |snapshot: &Snapshot| {
            snapshot
                .sections
                .get("tracked_achievements")
                .and_then(|section| section["entries"].get(&key))
                .cloned()
        };
        let previous = achievement(before);
        let current = achievement(after);
        let same_scope = previous
            .as_ref()
            .zip(current.as_ref())
            .is_some_and(|(a, b)| a["account_wide"] == b["account_wide"]);
        comparison["achievement_change"] = json!({
            "achievement_id":achievement_id,"before":previous,"after":current,
            "completed_changed":same_scope.then(|| observed_flag_changed(previous.as_ref(), current.as_ref(), "completed")).flatten(),
            "earned_by_character_changed":same_scope.then(|| observed_flag_changed(previous.as_ref(), current.as_ref(), "earned_by_character")).flatten(),
            "criterion_comparison":previous.as_ref().zip(current.as_ref()).filter(|_| same_scope)
                .map(|(a, b)| compare_criteria(&a["criteria"], &b["criteria"]))
        });
    }
    if let Some(skill_line) = selectors.profession_skill_line {
        ensure!(skill_line > 0, "Expected a positive profession skill line");
        let (before_status, previous) =
            profession_by_skill_line(before.sections.get("professions"), skill_line);
        let (after_status, current) =
            profession_by_skill_line(after.sections.get("professions"), skill_line);
        let rank_delta = previous
            .zip(current)
            .and_then(|(a, b)| Some(b["rank"].as_i64()? - a["rank"].as_i64()?));
        comparison["profession_change"] = json!({
            "skill_line":skill_line,"before_status":before_status,"after_status":after_status,
            "before":previous,"after":current,"rank_delta":rank_delta
        });
    }
    if selectors.action_bar_slots {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Action bar slot comparisons require Forever snapshots"
        );
        comparison["action_bar_slot_changes"] = compare_action_bar_slots(before, after);
    }
    if selectors.spellbook {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Spellbook comparisons require Forever snapshots"
        );
        comparison["spellbook_change"] = compare_spellbook(before, after);
    }
    if selectors.experience {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Experience comparisons require Forever snapshots"
        );
        let previous = before.sections.get("experience");
        let current = after.sections.get("experience");
        let before_level = before
            .sections
            .get("stats")
            .and_then(|stats| stats["level"].as_i64());
        let after_level = after
            .sections
            .get("stats")
            .and_then(|stats| stats["level"].as_i64());
        let levels = before_level.zip(after_level);
        let within_level = previous.zip(current).and_then(|(a, b)| {
            let (before_level, after_level) = levels?;
            let threshold = a["level_threshold_xp"].as_u64()?;
            if before_level != after_level || b["level_threshold_xp"].as_u64() != Some(threshold) {
                return None;
            }
            Some(b["current_xp"].as_i64()? - a["current_xp"].as_i64()?)
        });
        comparison["experience_change"] = json!({
            "before":previous,"after":current,"before_level":before_level,"after_level":after_level,
            "level_delta":levels.and_then(|(a,b)| b.checked_sub(a)),"within_level_xp_delta":within_level
        });
    }
    if selectors.wallet {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Wallet comparisons require Forever snapshots"
        );
        let copper = |snapshot: &Snapshot| {
            snapshot
                .sections
                .get("wallet")
                .and_then(|section| section["copper"].as_i64())
        };
        let previous = copper(before);
        let current = copper(after);
        comparison["wallet_change"] = json!({
            "before_copper":previous,"after_copper":current,
            "delta_copper":previous.zip(current).map(|(a,b)| b-a)
        });
    }
    Ok(comparison)
}

fn compare_action_bar_slots(before: &Snapshot, after: &Snapshot) -> Value {
    fn slots(snapshot: &Snapshot) -> Option<BTreeMap<u32, &Value>> {
        snapshot.sections.get("action_bar_observation")?["slots"]
            .as_object()?
            .iter()
            .map(|(slot, row)| Some((slot.parse().ok()?, row)))
            .collect()
    }
    let previous = slots(before);
    let current = slots(after);
    let changes = previous.as_ref().zip(current.as_ref()).map(|(a, b)| {
        let shared_changes = a
            .iter()
            .filter_map(|(slot, from)| {
                let to = b.get(slot)?;
                if ["empty", "kind", "id", "sub_type"]
                    .iter()
                    .all(|field| from[*field] == to[*field])
                {
                    return None;
                }
                let status = match (from["empty"].as_bool(), to["empty"].as_bool()) {
                    (Some(true), Some(false)) => "filled",
                    (Some(false), Some(true)) => "cleared",
                    _ => "observation_changed",
                };
                Some(json!({"slot":slot,"status":status,"before":from,"after":to}))
            })
            .collect::<Vec<_>>();
        json!({"changed_shared_slots":shared_changes,
            "before_only_slots":a.keys().filter(|slot| !b.contains_key(slot)).collect::<Vec<_>>(),
            "after_only_slots":b.keys().filter(|slot| !a.contains_key(slot)).collect::<Vec<_>>()})
    });
    json!({"before_available":previous.is_some(),"after_available":current.is_some(),"changes":changes,
        "button_hotkey_changes":compare_action_bar_hotkeys(before, after)})
}

fn compare_action_bar_hotkeys(before: &Snapshot, after: &Snapshot) -> Value {
    fn buttons(snapshot: &Snapshot) -> Option<BTreeMap<&str, &Value>> {
        snapshot.sections.get("action_bar_observation")?["button_hotkeys"]
            .as_object()?
            .values()
            .map(|row| Some((row["button"].as_str()?, row)))
            .collect()
    }
    let previous = buttons(before);
    let current = buttons(after);
    let changes = previous.as_ref().zip(current.as_ref()).map(|(a,b)| {
        let shared = a.iter().filter_map(|(button, from)| {
            let to = b.get(button)?;
            let keys_changed = from["key1"] != to["key1"] || from["key2"] != to["key2"];
            let slot_changed = from["slot"] != to["slot"];
            (keys_changed || slot_changed).then(|| json!({"button":button,"keys_changed":keys_changed,
                "slot_changed":slot_changed,"before":from,"after":to}))
        }).collect::<Vec<_>>();
        json!({"changed_shared_buttons":shared,
            "before_only_buttons":a.keys().filter(|button| !b.contains_key(*button)).collect::<Vec<_>>(),
            "after_only_buttons":b.keys().filter(|button| !a.contains_key(*button)).collect::<Vec<_>>()})
    });
    json!({"before_available":previous.is_some(),"after_available":current.is_some(),"changes":changes})
}

pub fn action_bar_spellbook(
    actions: &Value,
    spellbook: Option<&Value>,
    include_reverse: bool,
) -> Value {
    let book = spellbook.and_then(|book| book["entries"].as_object());
    let mut base_ids = BTreeSet::new();
    let mut reported_ids = BTreeSet::new();
    for row in book.into_iter().flat_map(|rows| rows.values()) {
        base_ids.extend(row["base_spell_id"].as_u64());
        reported_ids.extend(row["spell_id"].as_u64());
    }
    let action_slots = actions["slots"].as_object();
    let slots = action_slots.map(|slots| {
        slots
            .iter()
            .filter(|(_, row)| row["kind"] == "spell")
            .map(|(slot, row)| {
                let id = row["id"].as_u64();
                let observed = id.filter(|_| book.is_some());
                (
                    slot,
                    json!({"action_spell_id":id,
                "matches_base_spell_id":observed.map(|id| base_ids.contains(&id)),
                "matches_reported_spell_id":observed.map(|id| reported_ids.contains(&id))}),
                )
            })
            .collect::<BTreeMap<_, _>>()
    });
    let mut review = json!({"spellbook_available":book.is_some(),"spell_slots":slots});
    if !include_reverse {
        return review;
    }
    let unslotted = book.zip(action_slots).map(|(book, actions)| {
        let slotted = actions
            .values()
            .filter(|row| row["kind"] == "spell")
            .filter_map(|row| row["id"].as_u64())
            .collect::<BTreeSet<_>>();
        book.iter()
            .filter(|(_, row)| {
                row["passive"].as_bool() == Some(false)
                    && row["off_spec"].as_bool() == Some(false)
                    && !["base_spell_id", "spell_id"]
                        .into_iter()
                        .any(|field| row[field].as_u64().is_some_and(|id| slotted.contains(&id)))
            })
            .collect::<BTreeMap<_, _>>()
    });
    review["active_spellbook_slots_without_observed_action"] = json!(unslotted);
    review
}

fn compare_spellbook(before: &Snapshot, after: &Snapshot) -> Value {
    fn by_base(snapshot: &Snapshot) -> Option<BTreeMap<u64, BTreeSet<u64>>> {
        let rows = snapshot.sections.get("spellbook")?["entries"].as_object()?;
        let mut spells = BTreeMap::<u64, BTreeSet<u64>>::new();
        for row in rows.values() {
            spells
                .entry(row["base_spell_id"].as_u64()?)
                .or_default()
                .insert(row["spell_id"].as_u64()?);
        }
        Some(spells)
    }
    let previous = by_base(before);
    let current = by_base(after);
    let changes = previous.as_ref().zip(current.as_ref()).map(|(a,b)| {
        let added = b.keys().filter(|id| !a.contains_key(id)).collect::<Vec<_>>();
        let removed = a.keys().filter(|id| !b.contains_key(id)).collect::<Vec<_>>();
        let changed = a.iter().filter_map(|(id, old)| {
            let new = b.get(id)?;
            (old != new).then(|| json!({"base_spell_id":id,"before_reported_spell_ids":old,"after_reported_spell_ids":new}))
        }).collect::<Vec<_>>();
        json!({"added_base_spell_ids":added,"removed_base_spell_ids":removed,"reported_spell_id_changes":changed})
    });
    json!({"before_available":previous.is_some(),"after_available":current.is_some(),"changes":changes})
}

fn tracked_recipe_membership(before: &Value, after: &Value) -> Option<Value> {
    let ids = |group: &Value| -> Option<BTreeSet<u32>> {
        group
            .as_object()?
            .keys()
            .map(|id| id.parse().ok())
            .collect()
    };
    let before = ids(before)?;
    let after = ids(after)?;
    Some(json!({
        "added_recipe_ids":after.difference(&before).collect::<Vec<_>>(),
        "removed_recipe_ids":before.difference(&after).collect::<Vec<_>>()
    }))
}

fn tracked_recipe_learned_changes(before: &Value, after: &Value) -> Option<Value> {
    let after = after.as_object()?;
    let mut changes = before
        .as_object()?
        .iter()
        .filter_map(|(key, row)| {
            let previous = row["learned"].as_bool()?;
            let current = after.get(key)?["learned"].as_bool()?;
            let id = key.parse::<u32>().ok()?;
            (previous != current).then_some((id, previous, current))
        })
        .collect::<Vec<_>>();
    changes.sort_unstable_by_key(|(id, _, _)| *id);
    Some(json!(changes.into_iter().map(|(id, previous, current)|
        json!({"recipe_id":id,"before_learned":previous,"after_learned":current})
    ).collect::<Vec<_>>()))
}

fn remaining_renown_points(detail: &serde_json::Map<String, Value>) -> Option<u64> {
    if detail.get("capped")?.as_bool()? {
        return None;
    }
    let threshold = detail.get("level_threshold")?.as_u64()?;
    let earned = detail.get("earned_in_level")?.as_u64()?;
    threshold.checked_sub(earned).filter(|_| threshold > 0)
}

fn compare_special_reputation(before: &Value, after: &Value) -> Option<Value> {
    let kind = before["kind"].as_str()?;
    if after["kind"] != kind || before["account_wide"] != after["account_wide"] {
        return None;
    }
    let a = before.get(kind)?.as_object()?;
    let b = after.get(kind)?.as_object()?;
    let delta = |field| b.get(field)?.as_i64()?.checked_sub(a.get(field)?.as_i64()?);
    let flag_changed = |field| Some(a.get(field)?.as_bool()? != b.get(field)?.as_bool()?);
    match kind {
        "renown" => {
            let level_delta = delta("level")?;
            let earned_in_level_delta = if level_delta == 0
                && a.get("level_threshold") == b.get("level_threshold")
                && remaining_renown_points(a).is_some()
                && remaining_renown_points(b).is_some()
            {
                delta("earned_in_level")
            } else {
                None
            };
            Some(
                json!({"kind":kind,"level_delta":level_delta,"earned_in_level_delta":earned_in_level_delta,
                "capped_changed":flag_changed("capped")?}),
            )
        }
        "friendship" => Some(json!({"kind":kind,"standing_delta":delta("standing")?})),
        "paragon" => Some(
            json!({"kind":kind,"reward_pending_changed":flag_changed("reward_pending")?,
            "level_restriction_changed":flag_changed("level_too_low")?}),
        ),
        _ => None,
    }
}

fn criteria_by_id(criteria: &Value) -> BTreeMap<u64, Option<&Value>> {
    let mut indexed = BTreeMap::new();
    for entry in criteria
        .as_object()
        .into_iter()
        .flat_map(|rows| rows.values())
    {
        if let Some(id) = entry["criterion_id"].as_u64() {
            indexed
                .entry(id)
                .and_modify(|value| *value = None)
                .or_insert(Some(entry));
        }
    }
    indexed
}

fn compare_criteria(before: &Value, after: &Value) -> Value {
    let before = criteria_by_id(before);
    let after = criteria_by_id(after);
    let mut matched = 0;
    let mut changes = Vec::new();
    for (id, previous) in before {
        let Some((previous, current)) = previous.zip(after.get(&id).copied().flatten()) else {
            continue;
        };
        matched += 1;
        if previous != current {
            let quantity_delta = (previous["required"] == current["required"])
                .then(|| Some(current["quantity"].as_i64()? - previous["quantity"].as_i64()?))
                .flatten();
            changes.push(json!({
                "criterion_id":id,"before":previous,"after":current,"quantity_delta":quantity_delta
            }));
        }
    }
    json!({"matched_criterion_count":matched,"changes":changes})
}

pub fn profession_by_skill_line(
    section: Option<&Value>,
    skill_line: u32,
) -> (&'static str, Option<&Value>) {
    let Some(professions) = section.and_then(Value::as_object) else {
        return ("unavailable", None);
    };
    let mut matching = professions
        .values()
        .filter(|entry| entry["skill_line"].as_u64() == Some(u64::from(skill_line)));
    let Some(entry) = matching.next() else {
        return ("not_observed", None);
    };
    if matching.next().is_some() {
        ("ambiguous", None)
    } else {
        ("observed", Some(entry))
    }
}

fn observed_flag_changed(before: Option<&Value>, after: Option<&Value>, key: &str) -> Option<bool> {
    Some(before?.get(key)?.as_bool()? != after?.get(key)?.as_bool()?)
}

fn ensure_comparable(before: &Snapshot, after: &Snapshot) -> Result<()> {
    ensure!(before.client == after.client, "Client/build mismatch");
    ensure!(
        before.character.name == after.character.name
            && before.character.realm == after.character.realm,
        "Character mismatch"
    );
    ensure!(
        before.captured_at <= after.captured_at,
        "Snapshots are reversed"
    );
    Ok(())
}

fn compare_skill_lines(
    before: &serde_json::Map<String, Value>,
    after: &serde_json::Map<String, Value>,
) -> Value {
    let mut matched = 0;
    let mut changes = BTreeMap::new();
    for (key, old) in before {
        let Some(new) = after.get(key) else { continue };
        matched += 1;
        let (Some(id), Some(rank_before), Some(rank_after), Some(cap_before), Some(cap_after)) = (
            key.parse::<u32>().ok(),
            old["rank"].as_i64(),
            new["rank"].as_i64(),
            old["max_rank"].as_i64(),
            new["max_rank"].as_i64(),
        ) else {
            continue;
        };
        if rank_before != rank_after || cap_before != cap_after {
            changes.insert(id, json!({
                "skill_line_id":id,"name":new["name"],
                "rank_before":rank_before,"rank_after":rank_after,"rank_delta":rank_after-rank_before,
                "max_rank_before":cap_before,"max_rank_after":cap_after,"max_rank_delta":cap_after-cap_before,
            }));
        }
    }
    json!({
        "matched_skill_count":matched,
        "before_only_count":before.len()-matched,
        "after_only_count":after.len()-matched,
        "changes":changes.into_values().collect::<Vec<_>>()
    })
}

fn diff(a: &Value, b: &Value, path: &str, paths: &mut Vec<String>, count: &mut usize) {
    if a == b {
        return;
    }
    if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
        let keys: BTreeSet<_> = a.keys().chain(b.keys()).collect();
        for key in keys {
            let next = format!("{}/{}", path, key.replace('~', "~0").replace('/', "~1"));
            match (a.get(key), b.get(key)) {
                (Some(a), Some(b)) => diff(a, b, &next, paths, count),
                _ => record(next, paths, count),
            }
        }
    } else {
        record(path.to_owned(), paths, count);
    }
}
fn record(path: String, paths: &mut Vec<String>, count: &mut usize) {
    *count += 1;
    if paths.len() < 100 {
        paths.push(path);
    }
}

fn container_slots(s: &Snapshot, section: &str) -> Option<usize> {
    s.sections
        .get(section)
        .and_then(Value::as_object)
        .and_then(|b| {
            b.values()
                .map(|v| v.as_object().map(|v| v.len()))
                .sum::<Option<usize>>()
        })
}

pub fn standard_reputation_progress(section: &Value) -> Value {
    let Some(entries) = section["entries"].as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries
            .iter()
            .filter_map(|(id, entry)| {
                if entry["kind"] != "standard" {
                    return None;
                }
                // Forever treats reaction 8 as capped for standard reputation.
                let reaction = entry["reaction"].as_u64()?;
                let progress = match reaction {
                    8 => json!({"capped":true}),
                    1..=7 => {
                        let standing = entry["standing"].as_i64()?;
                        let threshold = entry["threshold"].as_i64()?;
                        let next = entry["next_threshold"].as_i64()?;
                        if next <= threshold || !(threshold..=next).contains(&standing) {
                            return None;
                        }
                        json!({"capped":false,"earned_in_rank":standing.checked_sub(threshold)?,
                            "rank_width":next.checked_sub(threshold)?,
                            "remaining_to_next_rank":next.checked_sub(standing)?})
                    }
                    _ => return None,
                };
                Some((id.clone(), progress))
            })
            .collect(),
    )
}

pub fn special_reputation_progress(section: &Value) -> Value {
    let Some(entries) = section["entries"].as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries
            .iter()
            .filter_map(|(id, entry)| {
                let kind = entry["kind"].as_str()?;
                let detail = entry.get(kind)?.as_object()?;
                let progress = match kind {
                    "renown" => {
                        let capped = detail.get("capped")?.as_bool()?;
                        let remaining = remaining_renown_points(detail);
                        json!({"kind":kind,"capped":capped,"remaining_to_next_rank":remaining})
                    }
                    "friendship" => {
                        let next = detail.get("next_threshold").and_then(Value::as_i64);
                        let remaining = next.and_then(|next| {
                            let threshold = detail.get("threshold")?.as_i64()?;
                            let standing = detail.get("standing")?.as_i64()?;
                            if next <= threshold || !(threshold..=next).contains(&standing) {
                                return None;
                            }
                            next.checked_sub(standing)
                        });
                        json!({"kind":kind,"capped":next.is_none(),"remaining_to_next_rank":remaining})
                    }
                    "paragon" => {
                        let pending = detail.get("reward_pending")?.as_bool()?;
                        let too_low = detail.get("level_too_low")?.as_bool()?;
                        let status = match (pending, too_low) {
                            (true, false) => "pending",
                            (true, true) => "pending_level_restricted",
                            (false, _) => "no_pending_reward",
                        };
                        json!({"kind":kind,"reward_status":status})
                    }
                    _ => return None,
                };
                Some((id.clone(), progress))
            })
            .collect(),
    )
}

pub fn currency_cap_headroom(section: &Value) -> Value {
    let Some(entries) = section["entries"].as_object() else {
        return Value::Null;
    };
    let remaining = |entry: &Value, limit: &str, used: &str| {
        entry[limit]
            .as_u64()
            .filter(|limit| *limit > 0)
            .zip(entry[used].as_u64())
            .map(|(limit, used)| limit.saturating_sub(used))
    };
    Value::Object(
        entries
            .iter()
            .filter_map(|(id, entry)| {
                let weekly = remaining(entry, "max_weekly_quantity", "earned_this_week");
                let basis = if entry["cap_uses_total_earned"].as_bool()? {
                    "total_earned"
                } else {
                    "quantity"
                };
                let maximum = remaining(entry, "max_quantity", basis);
                (weekly.is_some() || maximum.is_some()).then(|| {
                    (
                        id.clone(),
                        json!({"weekly_remaining":weekly,"maximum_remaining":maximum,
                            "maximum_basis":maximum.map(|_| basis)}),
                    )
                })
            })
            .collect(),
    )
}

pub fn currency_reward_projection(proposed: u64, entry: &Value, caps: &Value) -> Value {
    let maximum = caps["maximum_remaining"].as_u64();
    let weekly = caps["weekly_remaining"]
        .as_u64()
        .filter(|_| entry["can_earn_per_week"].as_bool() != Some(false));
    let limit = maximum.into_iter().chain(weekly).min();
    json!({
        "proposed_quantity":proposed,
        "quantity_within_recorded_headroom":limit.map(|cap| proposed.min(cap)),
        "quantity_above_recorded_headroom":limit.map(|cap| proposed.saturating_sub(cap))
    })
}

pub fn quest_list_item_rewards(quests: &serde_json::Map<String, Value>) -> Value {
    let mut totals = BTreeMap::<u64, u64>::new();
    let mut unknown_fixed = 0;
    let mut unknown_choices = 0;
    let mut quests_with_choices = Vec::new();
    for (id, quest) in quests {
        match quest["item_choices"].as_object() {
            Some(choices) if !choices.is_empty() => quests_with_choices.push(id),
            Some(_) => (),
            None => unknown_choices += 1,
        }
        let Some(rows) = quest["item_rewards"].as_object() else {
            unknown_fixed += 1;
            continue;
        };
        for row in rows.values() {
            if let (Some(id), Some(quantity)) = (row["item_id"].as_u64(), row["quantity"].as_u64())
            {
                *totals.entry(id).or_default() += quantity;
            }
        }
    }
    json!({"known_fixed_quantity_by_item_id":totals,
        "unknown_fixed_reward_list_count":unknown_fixed,
        "quests_with_observed_item_choices":quests_with_choices,
        "unknown_choice_list_count":unknown_choices})
}

pub fn quest_list_currency_projection(
    quests: &serde_json::Map<String, Value>,
    balances: &Value,
) -> Value {
    let mut totals = BTreeMap::<String, u128>::new();
    let mut unknown = 0;
    for quest in quests.values() {
        let Some(rows) = quest["currency_rewards"].as_object() else {
            unknown += 1;
            continue;
        };
        for row in rows.values() {
            if let (Some(id), Some(quantity)) =
                (row["currency_id"].as_u64(), row["quantity"].as_u64())
            {
                *totals.entry(id.to_string()).or_default() += u128::from(quantity);
            }
        }
    }
    let headroom = currency_cap_headroom(balances);
    let projections: BTreeMap<_, _> = totals
        .into_iter()
        .map(|(id, total)| {
            let projection = u64::try_from(total).ok().map(|quantity| {
                currency_reward_projection(quantity, &balances["entries"][&id], &headroom[&id])
            });
            (id, projection)
        })
        .collect();
    json!({"known_fixed_rewards":projections,"unknown_fixed_reward_list_count":unknown})
}

pub fn quest_currency_projection(quest: Option<&Value>, balances: &Value) -> Value {
    let Some(quest) = quest else {
        return Value::Null;
    };
    let headroom = currency_cap_headroom(balances);
    let project = |id: u64, quantity| {
        let key = id.to_string();
        currency_reward_projection(quantity, &balances["entries"][&key], &headroom[&key])
    };
    let receive = quest["currency_rewards"].as_object().map(|rows| {
        let mut totals = BTreeMap::<u64, u64>::new();
        for row in rows.values() {
            if let (Some(id), Some(quantity)) =
                (row["currency_id"].as_u64(), row["quantity"].as_u64())
            {
                *totals.entry(id).or_default() += quantity;
            }
        }
        totals
    });
    let fixed = receive.as_ref().map(|totals| {
        totals
            .iter()
            .map(|(&id, &quantity)| (id, project(id, quantity)))
            .collect::<BTreeMap<_, _>>()
    });
    let choices = quest["currency_choices"].as_object().map(|rows| {
        rows.iter().filter_map(|(index, row)| {
            let id = row["currency_id"].as_u64()?;
            let quantity = row["quantity"].as_u64()?;
            let combined = receive.as_ref().map(|totals| project(id, quantity + totals.get(&id).copied().unwrap_or(0)));
            Some((index, json!({"currency_id":id,"choice_quantity":quantity,"with_receive_rewards":combined})))
        }).collect::<BTreeMap<_, _>>()
    });
    json!({"receive_by_currency_id":fixed,"choices_by_position":choices})
}

pub fn quest_list_renown_projection(
    quests: &serde_json::Map<String, Value>,
    reputation: &Value,
) -> Value {
    let mut totals = BTreeMap::<String, u64>::new();
    let mut unknown = 0;
    for quest in quests.values() {
        let Some(rows) = quest["renown_rewards"].as_object() else {
            unknown += 1;
            continue;
        };
        for row in rows.values() {
            if let (Some(id), Some(points)) = (row["faction_id"].as_u64(), row["quantity"].as_u64())
            {
                *totals.entry(id.to_string()).or_default() += points;
            }
        }
    }
    let projections: BTreeMap<_, _> = totals
        .into_iter()
        .map(|(id, points)| {
            let mut projection = renown_reward_projection(points, &reputation["entries"][&id]);
            if unknown > 0 && projection["reaches_next_rank"] == false {
                projection["reaches_next_rank"] = Value::Null;
            }
            (id, projection)
        })
        .collect();
    json!({"known_rewards_by_faction":projections,"unknown_reward_list_count":unknown})
}

fn renown_reward_projection(quantity: u64, faction: &Value) -> Value {
    let detail = (faction["kind"] == "renown")
        .then(|| faction["renown"].as_object())
        .flatten();
    let capped = detail
        .and_then(|row| row.get("capped"))
        .and_then(Value::as_bool);
    let remaining = detail.and_then(remaining_renown_points);
    json!({"reward_points":quantity,"capped":capped,
        "points_to_next_rank":remaining,"reaches_next_rank":remaining.map(|points| quantity >= points),
        "points_short_of_next_rank":remaining.map(|points| points.saturating_sub(quantity))})
}

pub fn quest_renown_projection(quest: Option<&Value>, reputation: &Value) -> Value {
    let Some(rows) = quest.and_then(|q| q["renown_rewards"].as_object()) else {
        return Value::Null;
    };
    let mut totals = BTreeMap::<u64, u64>::new();
    for row in rows.values() {
        if let (Some(id), Some(quantity)) = (row["faction_id"].as_u64(), row["quantity"].as_u64()) {
            *totals.entry(id).or_default() += quantity;
        }
    }
    json!(
        totals
            .into_iter()
            .map(|(id, quantity)| {
                let key = id.to_string();
                let faction = &reputation["entries"][&key];
                let mut projection = renown_reward_projection(quantity, faction);
                projection["name"] = faction["name"].clone();
                (id, projection)
            })
            .collect::<BTreeMap<_, _>>()
    )
}

fn xp_to_next_level(xp: &Value) -> Option<u64> {
    xp["level_threshold_xp"]
        .as_u64()?
        .checked_sub(xp["current_xp"].as_u64()?)
}

pub fn quest_list_xp_projection(
    quests: &serde_json::Map<String, Value>,
    experience: Option<&Value>,
) -> Value {
    let mut unknown = 0;
    let mut known_xp = 0u128;
    for quest in quests.values() {
        match quest["reward_amounts"]["xp"].as_u64() {
            Some(xp) => known_xp += u128::from(xp),
            None => unknown += 1,
        }
    }
    let total = u64::try_from(known_xp).ok();
    let disabled = experience.and_then(|xp| xp["xp_disabled"].as_bool());
    let remaining = experience.and_then(xp_to_next_level);
    let estimate = remaining.zip(total).filter(|_| disabled != Some(true));
    json!({
        "known_reward_xp":total,"unknown_reward_count":unknown,
        "xp_to_next_level":remaining,"xp_disabled":disabled,
        "covers_recorded_xp_threshold":estimate.and_then(|(remaining,total)| {
            if total >= remaining { Some(true) } else if unknown == 0 { Some(false) } else { None }
        }),
        "xp_shortfall_after_known_rewards":estimate.map(|(remaining,total)| remaining.saturating_sub(total))
    })
}

pub fn quest_xp_projection(quest: Option<&Value>, experience: Option<&Value>) -> Value {
    let Some(quest) = quest else {
        return Value::Null;
    };
    let reward = quest["reward_amounts"]["xp"].as_u64();
    let remaining = experience.and_then(xp_to_next_level);
    let disabled = experience.and_then(|xp| xp["xp_disabled"].as_bool());
    let estimate = remaining.zip(reward).filter(|_| disabled != Some(true));
    json!({
        "reward_xp":reward,"experience_status":experience.and_then(|xp| xp["status"].as_str()),
        "xp_disabled":disabled,"xp_to_next_level":remaining,
        "covers_recorded_xp_threshold":estimate.map(|(remaining,reward)| reward >= remaining),
        "xp_shortfall_after_reward":estimate.map(|(remaining,reward)| remaining.saturating_sub(reward))
    })
}

fn flight_destinations(map: &Value) -> Option<(u64, BTreeMap<u64, &Value>)> {
    let mut origin = None;
    let mut nodes = BTreeMap::new();
    for row in map["entries"].as_object()?.values() {
        if row["map_transition"].as_bool()? {
            continue;
        }
        let id = row["node_id"].as_u64()?;
        if nodes.insert(id, row).is_some() {
            return None;
        }
        if row["state"].as_str()? == "current" && origin.replace(id).is_some() {
            return None;
        }
    }
    Some((origin?, nodes))
}

fn compare_flight_maps(before: &Value, after: &Value) -> Value {
    if before["map_id"] != after["map_id"] {
        return json!({"status":"different_maps"});
    }
    let Some(((from, previous), (to, current))) =
        flight_destinations(before).zip(flight_destinations(after))
    else {
        return json!({"status":"origin_or_destinations_ambiguous"});
    };
    if from != to {
        return json!({"status":"different_origins"});
    }
    let changes: Vec<_> = previous
        .iter()
        .filter_map(|(&id, old)| {
            let new = current.get(&id)?;
            let state_changed = old["state"] != new["state"];
            let fare_delta = old["fare_copper"]
                .as_i64()
                .zip(new["fare_copper"].as_i64())
                .map(|(old, new)| new - old)
                .filter(|delta| *delta != 0);
            (state_changed || fare_delta.is_some()).then(|| {
                json!({
                    "node_id":id,"before_state":old["state"],"after_state":new["state"],
                    "fare_copper_delta":fare_delta,
                })
            })
        })
        .collect();
    json!({"status":"compared","map_id":before["map_id"],"origin_node_id":from,"changes":changes})
}

fn available_trainer_prices(trainer: &Value, price_field: &str) -> Option<(usize, usize, u64)> {
    let entries = trainer["entries"].as_object()?;
    let mut available = 0;
    let mut unknown_prices = 0;
    let mut subtotal = 0u64;
    for lesson in entries
        .values()
        .filter(|lesson| lesson["status"] == "available")
    {
        available += 1;
        if let Some(price) = lesson[price_field].as_u64() {
            subtotal += price;
        } else {
            unknown_prices += 1;
        }
    }
    Some((available, unknown_prices, subtotal))
}

fn trainer_lesson_affordability(trainer: &Value, price_field: &str, balance: Option<u64>) -> Value {
    json!(trainer["entries"].as_object().into_iter().flatten()
        .filter(|(_, lesson)| lesson["status"] == "available")
        .map(|(id, lesson)| {
            let comparison = lesson[price_field].as_u64().zip(balance);
            (id, json!({
                "affordable": comparison.map(|(price, balance)| price <= balance),
                "shortfall": comparison.map(|(price, balance)| price.saturating_sub(balance))
            }))
        }).collect::<BTreeMap<_, _>>())
}

fn trainer_cheapest_affordable(
    trainer: &Value,
    price_field: &str,
    balance: Option<u64>,
) -> Option<Value> {
    let balance = balance?;
    let mut priced: Vec<_> = trainer["entries"]
        .as_object()?
        .iter()
        .filter(|(_, lesson)| lesson["status"] == "available")
        .filter_map(|(slot, lesson)| Some((lesson[price_field].as_u64()?, slot)))
        .collect();
    priced.sort_unstable();
    let mut subtotal = 0;
    let mut slots = Vec::new();
    for (price, slot) in priced {
        if price > balance - subtotal {
            break;
        }
        subtotal += price;
        slots.push(slot);
    }
    Some(json!({"lesson_slots":slots,"lesson_count":slots.len(),"total_price":subtotal}))
}

pub fn trainer_wallet_projection(trainer: &Value, wallet: Option<&Value>) -> Value {
    if trainer["kind"] == "pet" {
        return Value::Null;
    }
    let Some((available, unknown_prices, subtotal)) =
        available_trainer_prices(trainer, "price_copper")
    else {
        return Value::Null;
    };
    let total = (unknown_prices == 0).then_some(subtotal);
    let balance = wallet.and_then(|wallet| wallet["copper"].as_u64());
    json!({
        "available_lessons":available,
        "lessons_with_unknown_price":unknown_prices,
        "known_price_subtotal_copper":subtotal,
        "total_price_copper":total,
        "wallet_shortfall_copper":total.zip(balance).map(|(cost, balance)| cost.saturating_sub(balance)),
        "individual_lesson_affordability":trainer_lesson_affordability(trainer, "price_copper", balance),
        "cheapest_affordable_known_lessons":trainer_cheapest_affordable(trainer, "price_copper", balance),
    })
}

pub fn trainer_points_projection(trainer: &Value) -> Value {
    if trainer["kind"] != "pet" {
        return Value::Null;
    }
    let Some((available, unknown_prices, subtotal)) =
        available_trainer_prices(trainer, "price_training_points")
    else {
        return Value::Null;
    };
    let total = (unknown_prices == 0).then_some(subtotal);
    let balance = trainer["pet_training_points_available"].as_u64();
    json!({
        "available_lessons":available,
        "lessons_with_unknown_price":unknown_prices,
        "known_price_subtotal_points":subtotal,
        "total_price_points":total,
        "training_point_shortfall":total.zip(balance).map(|(cost, balance)| cost.saturating_sub(balance)),
        "individual_lesson_affordability":trainer_lesson_affordability(trainer, "price_training_points", balance),
        "cheapest_affordable_known_lessons":trainer_cheapest_affordable(trainer, "price_training_points", balance),
    })
}

pub fn merchant_budget(snapshot: &Snapshot, offer: &Value, bundles: u32) -> Result<Value> {
    ensure!((1..=1000).contains(&bundles), "Invalid bundle count");
    let bundles = u64::from(bundles);
    let price = offer["price_copper"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid merchant price"))?;
    let size = offer["bundle_size"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid merchant bundle"))?;
    let copper = price * bundles;
    let wallet = snapshot
        .sections
        .get("wallet")
        .and_then(|wallet| wallet["copper"].as_u64());
    let mut token_costs = BTreeMap::<u32, u64>::new();
    let mut named_currencies = Vec::new();
    let components = offer["cost_components"].as_object();
    for cost in components.into_iter().flat_map(|rows| rows.values()) {
        let quantity = cost["quantity"].as_u64().unwrap() * bundles;
        if let Some(id) = cost["item_id"].as_u64() {
            *token_costs.entry(u32::try_from(id)?).or_default() += quantity;
        } else {
            named_currencies
                .push(json!({"currency_name":cost["currency_name"],"required_quantity":quantity}));
        }
    }
    let mut tokens = Vec::new();
    for (id, required) in token_costs {
        let found = find_item(snapshot, id, 0, 1)?;
        let carried = found["quantity_by_storage"]["bags"].as_u64();
        tokens.push(json!({"item_id":id,"required_quantity":required,"shortfall_vs_bags":carried.map(|count| required.saturating_sub(count))}));
    }
    let costs_observed = offer["extended_cost"].as_bool() == Some(false) || components.is_some();
    Ok(json!({
        "requested_bundles":bundles,"requested_quantity":size * bundles,"required_copper":copper,
        "wallet_shortfall_copper":wallet.map(|balance| copper.saturating_sub(balance)),
        "item_token_costs":costs_observed.then_some(tokens),
        "named_currency_costs":costs_observed.then_some(named_currencies)
    }))
}

pub fn merchant_cart_budget(
    snapshot: &Snapshot,
    cart: impl Iterator<Item = (u32, u32)>,
) -> Result<Value> {
    let Some(offers) = snapshot
        .sections
        .get("merchant_stock")
        .and_then(Value::as_object)
    else {
        return Ok(Value::Null);
    };
    let mut lines = BTreeMap::new();
    let mut missing = Vec::new();
    let mut copper = 0u64;
    let mut token_costs = BTreeMap::<u32, u64>::new();
    let mut unknown_token_cost_slots = Vec::new();
    for (slot, bundles) in cart {
        let Some(offer) = offers.get(&slot.to_string()) else {
            missing.push(slot);
            continue;
        };
        let mut line = merchant_budget(snapshot, offer, bundles)?;
        copper = copper
            .checked_add(line["required_copper"].as_u64().unwrap())
            .ok_or_else(|| anyhow::anyhow!("Merchant cart cost is too large"))?;
        if let Some(tokens) = line["item_token_costs"].as_array() {
            for token in tokens {
                let id = u32::try_from(token["item_id"].as_u64().unwrap())?;
                *token_costs.entry(id).or_default() += token["required_quantity"].as_u64().unwrap();
            }
        } else {
            unknown_token_cost_slots.push(slot);
        }
        line["item_id"] = offer["item_id"].clone();
        line["currency_id"] = offer["currency_id"].clone();
        lines.insert(slot, line);
    }
    let wallet = snapshot
        .sections
        .get("wallet")
        .and_then(|row| row["copper"].as_u64());
    let mut tokens = Vec::new();
    for (id, required) in token_costs {
        let found = find_item(snapshot, id, 0, 1)?;
        let carried = found["quantity_by_storage"]["bags"].as_u64();
        tokens.push(json!({"item_id":id,"required_quantity":required,
            "shortfall_vs_bags":carried.map(|count| required.saturating_sub(count))}));
    }
    Ok(
        json!({"lines_by_slot":lines,"missing_slots":missing,"known_required_copper":copper,
        "total_required_copper":missing.is_empty().then_some(copper),
        "known_item_token_costs":tokens,
        "item_token_costs_complete":missing.is_empty() && unknown_token_cost_slots.is_empty(),
        "unknown_item_token_cost_slots":unknown_token_cost_slots,
        "known_copper_shortfall":wallet.map(|balance| copper.saturating_sub(balance))}),
    )
}

pub fn flight_wallet_projection(map: &Value, wallet: Option<&Value>) -> Value {
    let Some(entries) = map["entries"].as_object() else {
        return Value::Null;
    };
    let balance = wallet.and_then(|wallet| wallet["copper"].as_u64());
    Value::Object(
        entries.iter().map(|(slot, node)| {
            let fare_and_balance = node["fare_copper"].as_u64().zip(balance);
            (slot.clone(), json!({
                "affordable":fare_and_balance.map(|(fare, balance)| fare <= balance),
                "shortfall_copper":fare_and_balance.map(|(fare, balance)| fare.saturating_sub(balance)),
            }))
        }).collect()
    )
}

pub fn quest_list_wallet_projection(
    quests: &serde_json::Map<String, Value>,
    wallet: Option<&Value>,
) -> Value {
    let mut known_cost = 0u128;
    let mut known_reward = 0u128;
    let mut unknown_cost = 0;
    let mut unknown_reward = 0;
    for quest in quests.values() {
        match quest["required_copper"].as_u64() {
            Some(copper) => known_cost += u128::from(copper),
            None => unknown_cost += 1,
        }
        match quest["reward_amounts"]["copper"].as_u64() {
            Some(copper) => known_reward += u128::from(copper),
            None => unknown_reward += 1,
        }
    }
    let known_cost = u64::try_from(known_cost).ok();
    let balance = wallet.and_then(|row| row["copper"].as_u64());
    let estimate = known_cost.zip(balance);
    json!({
        "known_required_copper":known_cost,"unknown_cost_count":unknown_cost,
        "known_reward_copper":u64::try_from(known_reward).ok(),"unknown_reward_count":unknown_reward,
        "ready_turn_in_order":quest_wallet_turn_in_order(quests, balance),
        "wallet_observed":balance.is_some(),
        "known_cost_shortfall_without_rewards_copper":estimate.map(|(cost,balance)| cost.saturating_sub(balance)),
        "all_costs_fit_wallet_without_rewards":estimate.and_then(|(cost,balance)| {
            if cost > balance { Some(false) } else if unknown_cost == 0 { Some(true) } else { None }
        })
    })
}

fn quest_wallet_turn_in_order(
    quests: &serde_json::Map<String, Value>,
    balance: Option<u64>,
) -> Option<Value> {
    let mut steps = quests
        .iter()
        .map(|(id, quest)| {
            if quest_readiness(quest) != QuestReadiness::Ready {
                return None;
            }
            Some((
                id,
                quest["required_copper"].as_u64()?,
                quest["reward_amounts"]["copper"].as_u64()?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    if steps.is_empty() {
        return None;
    }
    // Do net gains first (cheapest first), then net losses (largest reward first).
    steps.sort_by_key(|(_, cost, reward)| {
        if reward >= cost {
            (false, *cost)
        } else {
            (true, u64::MAX - reward)
        }
    });
    let mut net = 0i128;
    let mut minimum = 0i128;
    let mut first_blocked = None;
    for (index, (id, cost, reward)) in steps.iter().enumerate() {
        let required = i128::from(*cost) - net;
        minimum = minimum.max(required);
        if first_blocked.is_none()
            && let Some(balance) = balance
            && required > i128::from(balance)
        {
            first_blocked = Some((index, *id, required - i128::from(balance)));
        }
        net += i128::from(*reward) - i128::from(*cost);
    }
    let minimum = u64::try_from(minimum).ok()?;
    let progress = balance.map(|_| {
        let (affordable, blocked_id, shortfall) = first_blocked
            .map_or((steps.len(), None, None), |(index, id, gap)| {
                (index, Some(id), u64::try_from(gap).ok())
            });
        json!({"affordable_prefix_count":affordable,"first_blocked_quest_id":blocked_id,
            "first_blocked_shortfall_copper":shortfall})
    });
    Some(json!({
        "quest_ids":steps.into_iter().map(|(id, _, _)| id).collect::<Vec<_>>(),
        "minimum_starting_copper":minimum,
        "shortfall_copper":balance.map(|balance| minimum.saturating_sub(balance)),
        "captured_wallet_progress":progress
    }))
}

pub fn quest_wallet_projection(quest: Option<&Value>, wallet: Option<&Value>) -> Value {
    let Some(quest) = quest else {
        return Value::Null;
    };
    let cost = quest["required_copper"].as_u64();
    let reward = quest["reward_amounts"]["copper"].as_u64();
    let balance = wallet.and_then(|wallet| wallet["copper"].as_u64());
    let upfront = balance.zip(cost);
    json!({
        "required_copper":cost,
        "reward_copper":reward,
        "net_copper":reward.zip(cost).map(|(reward, cost)| i128::from(reward) - i128::from(cost)),
        "upfront_shortfall_copper":upfront.map(|(balance, cost)| cost.saturating_sub(balance)),
        "projected_wallet_copper":upfront.and_then(|(balance, cost)| balance.checked_sub(cost)).zip(reward)
            .and_then(|(remaining, reward)| remaining.checked_add(reward))
    })
}

pub fn profession_training_progress(section: &Value) -> Value {
    let Some(entries) = section.as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries
            .iter()
            .map(|(slot, profession)| (slot.clone(), profession_rank_progress(profession)))
            .collect(),
    )
}

pub fn profession_rank_progress(profession: &Value) -> Value {
    let remaining = profession["max_rank"]
        .as_u64()
        .filter(|maximum| *maximum > 0)
        .zip(profession["rank"].as_u64())
        .and_then(|(maximum, rank)| maximum.checked_sub(rank));
    json!({"skill_line":profession["skill_line"],"remaining_to_observed_cap":remaining})
}

pub fn tracked_achievement_work(section: &Value) -> Value {
    let Some(entries) = section["entries"].as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries
            .iter()
            .filter_map(|(id, achievement)| {
                let criteria = achievement["criteria"].as_object()?;
                let unfinished = criteria
                    .iter()
                    .filter(|(_, criterion)| criterion["completed"].as_bool() == Some(false))
                    .map(|(index, criterion)| {
                        let remaining = criterion["required"]
                            .as_u64()
                            .zip(criterion["quantity"].as_u64())
                            .map(|(required, quantity)| required.saturating_sub(quantity));
                        (
                            index.clone(),
                            json!({"criterion_id":criterion["criterion_id"],"remaining_count":remaining}),
                        )
                    })
                    .collect();
                Some((id.clone(), Value::Object(unfinished)))
            })
            .collect(),
    )
}

pub fn unfinished_quest_objectives(quest: Option<&Value>) -> Value {
    let Some(objectives) = quest.and_then(|quest| quest["objectives"].as_object()) else {
        return Value::Null;
    };
    Value::Object(
        objectives
            .iter()
            .filter(|(_, objective)| objective["finished"].as_bool() == Some(false))
            .map(|(index, objective)| {
                let remaining = objective["required"]
                    .as_u64()
                    .zip(objective["fulfilled"].as_u64())
                    .map(|(required, fulfilled)| required.saturating_sub(fulfilled));
                (
                    index.clone(),
                    json!({"kind":objective["kind"],"remaining_count":remaining}),
                )
            })
            .collect(),
    )
}

#[derive(
    Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum QuestReadiness {
    Ready,
    NotReady,
    Failed,
    Unknown,
}

pub fn quest_readiness(quest: &Value) -> QuestReadiness {
    if quest["failed"].as_bool() == Some(true) {
        QuestReadiness::Failed
    } else {
        match quest["ready_for_turn_in"].as_bool() {
            Some(true) => QuestReadiness::Ready,
            Some(false) => QuestReadiness::NotReady,
            None => QuestReadiness::Unknown,
        }
    }
}

pub fn quest_checklist(quests: &serde_json::Map<String, Value>) -> Value {
    let (mut ready, mut not_ready, mut failed, mut unknown) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (key, quest) in quests {
        let Ok(id) = key.parse::<u32>() else {
            continue;
        };
        match quest_readiness(quest) {
            QuestReadiness::Ready => ready.push(id),
            QuestReadiness::NotReady => not_ready.push(id),
            QuestReadiness::Failed => failed.push(id),
            QuestReadiness::Unknown => unknown.push(id),
        }
    }
    for ids in [&mut ready, &mut not_ready, &mut failed, &mut unknown] {
        ids.sort_unstable();
    }
    json!({"ready_for_turn_in":ready,"turn_in_not_ready":not_ready,"failed":failed,"turn_in_unknown":unknown})
}

fn quest_waypoint_maps(quests: &serde_json::Map<String, Value>) -> Value {
    let mut maps = BTreeMap::<u64, Vec<u32>>::new();
    let mut unknown = Vec::new();
    for (key, quest) in quests {
        let Ok(id) = key.parse::<u32>() else { continue };
        if let Some(map) = quest["next_waypoint_map_id"].as_u64() {
            maps.entry(map).or_default().push(id);
        } else {
            unknown.push(id);
        }
    }
    for ids in maps.values_mut() {
        ids.sort_unstable();
    }
    unknown.sort_unstable();
    json!({"by_map_id":maps,"waypoint_map_unknown":unknown})
}

pub fn summary(s: &Snapshot) -> Value {
    let occupied = s
        .sections
        .get("equipment")
        .and_then(Value::as_object)
        .map(|v| v.len());
    let unspent = s
        .sections
        .get("talents")
        .and_then(|v| v.get("has_unspent"))
        .and_then(Value::as_bool);
    let quests = s.sections.get("quests").and_then(Value::as_object);
    let timed_quests = quests.filter(|_| s.client.family == crate::Family::Forever).map(|quests| {
        let mut rows = quests.iter().filter_map(|(id, quest)| {
            Some((quest["timer"]["remaining_seconds"].as_u64()?, id.parse::<u32>().ok()?))
        }).collect::<Vec<_>>();
        rows.sort_unstable();
        rows.into_iter().map(|(remaining_seconds, quest_id)| json!({"quest_id":quest_id,"remaining_seconds":remaining_seconds})).collect::<Vec<_>>()
    });
    let group_quests = quests.filter(|_| s.client.family == crate::Family::Forever).map(|quests| {
        let mut groups = Vec::new();
        let mut elite = Vec::new();
        for (key, quest) in quests {
            let Ok(id) = key.parse::<u32>() else { continue };
            if let Some(size) = quest["suggested_group_size"].as_u64().filter(|size| *size > 1) {
                groups.push((id, size));
            }
            if quest["elite"].as_bool() == Some(true) { elite.push(id); }
        }
        groups.sort_unstable();
        elite.sort_unstable();
        json!({"suggested_groups":groups.into_iter().map(|(quest_id,size)| json!({"quest_id":quest_id,"suggested_group_size":size})).collect::<Vec<_>>(),"elite_quest_ids":elite})
    });
    let is_forever = s.client.family == crate::Family::Forever;
    let wallet = s
        .sections
        .get("wallet")
        .and_then(|wallet| wallet["copper"].as_u64());
    let repair_status = is_forever.then(|| {
        let alerts = s.sections.get("durability_alerts").and_then(Value::as_object);
        let quote = s.sections.get("repair_quote").and_then(|quote| quote["copper"].as_u64());
        let shortfall = quote.zip(wallet).map(|(cost, balance)| cost.saturating_sub(balance));
        json!({
            "low_slots":alerts.map(|slots| slots.values().filter(|status| status.as_str()==Some("low")).count()),
            "broken_slots":alerts.map(|slots| slots.values().filter(|status| status.as_str()==Some("broken")).count()),
            "repair_all_quote_copper":quote,
            "wallet_shortfall_copper":shortfall
        })
    });
    let waypoint_maps = quests.filter(|_| is_forever).map(quest_waypoint_maps);
    let captured_map = s
        .sections
        .get("stats")
        .filter(|_| is_forever)
        .and_then(|stats| stats["map_id"].as_u64());
    let quests_on_captured_map =
        captured_map
            .zip(waypoint_maps.as_ref())
            .map(|(map, waypoints)| {
                waypoints["by_map_id"]
                    .get(map.to_string())
                    .cloned()
                    .unwrap_or_else(|| json!([]))
            });
    let quest_money = quests
        .filter(|_| s.client.family == crate::Family::Forever)
        .map(|quests| {
            let mut unknown = 0;
            let mut costs = BTreeMap::new();
            for (id, quest) in quests {
                match quest["required_copper"].as_u64() {
                    Some(0) => (),
                    Some(cost) => {
                        costs.insert(
                            id,
                            json!({"required_copper":cost,
                        "shortfall_copper":wallet.map(|balance| cost.saturating_sub(balance))}),
                        );
                    }
                    None => unknown += 1,
                }
            }
            json!({"quests_with_cost":costs,"cost_unknown_count":unknown})
        });
    let quest_turn_in = quests.map(|quests| {
        let (mut ready, mut known) = (0, 0);
        for quest in quests.values() {
            if let Some(value) = quest["ready_for_turn_in"].as_bool() {
                known += 1;
                ready += usize::from(value);
            }
        }
        json!({"ready_observed":ready,"status_observed":known,"status_unknown":quests.len()-known})
    });
    json!({"client":s.client,"character":s.character,"captured_at":s.captured_at,"live":false,
      "experience_progress":s.sections.get("experience").map(|xp|
          json!({"observed":xp,"xp_to_next_level":xp_to_next_level(xp)})),
      "spellbook_slots_observed":s.sections.get("spellbook").and_then(|book|book["entries"].as_object()).map(|slots|slots.len()),
      "equipped_slots_observed":occupied,"carried_item_slots_observed":container_slots(s,"bags"),
      "carried_bag_space":carried_bag_space(s),
      "instance_context":s.sections.get("instance_context"),
      "character_bank_item_slots_observed":container_slots(s,"character_bank"),
      "account_bank_item_slots_observed":container_slots(s,"account_bank"),
      "reputation_factions_observed":s.sections.get("reputation").and_then(|v|v["entries"].as_object()).map(|v|v.len()),
      "currency_balances_observed":s.sections.get("currency_balances").and_then(|v|v["entries"].as_object()).map(|v|v.len()),
      "tracked_achievements_observed":s.sections.get("tracked_achievements").and_then(|v|v["entries"].as_object()).map(|v|v.len()),
      "quest_turn_in":quest_turn_in,
      "quest_checklist":quests.map(quest_checklist),
      "quest_waypoint_maps":waypoint_maps,
      "captured_map_id":captured_map,
      "captured_map_name":captured_map.and_then(|_| s.sections["stats"]["map_name"].as_str()),
      "captured_map_parent_id":captured_map.and_then(|_| s.sections["stats"]["map_parent_id"].as_u64()),
      "captured_map_min_level":captured_map.and_then(|_| s.sections["stats"]["map_min_level"].as_u64()),
      "captured_map_max_level":captured_map.and_then(|_| s.sections["stats"]["map_max_level"].as_u64()),
      "new_mail_indicator":s.sections.get("stats").filter(|_| is_forever).and_then(|stats| stats["new_mail_indicator"].as_bool()),
      "hearthstone_bind_name":s.sections.get("stats").filter(|_| is_forever).and_then(|stats| stats["hearthstone_bind_name"].as_str()),
      "hearthstone_item_id":s.sections.get("stats").filter(|_| is_forever).and_then(|stats| stats["hearthstone_item_id"].as_u64()),
      "hearthstone_cooldown_seconds":s.sections.get("stats").filter(|_| is_forever).and_then(|stats| stats["hearthstone_cooldown_seconds"].as_u64()),
      "quests_with_waypoint_on_captured_map":quests_on_captured_map,
      "quest_money":quest_money,
      "repair_status":repair_status,
      "observed_quest_timers":timed_quests,
      "observed_quest_difficulty":group_quests,
      "carried_item_review":carried_items(s),
      "unspent_talent_points_reported":unspent,"available_sections":s.sections.keys().collect::<Vec<_>>(),
      "unavailable":s.unavailable,"limitations":["Counts describe exported entries, not item quality or optimal gear","No bank/mail inventory inferred","No enchant, socket or talent legality inferred from missing fields"]})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Snapshot {
        serde_json::from_value(json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z","client":{"family":"retail","variant":"live","build":"123","interface":120105},"character":{"name":"Example","realm":"Test"},"sections":{"equipment":{"16":{"item_id":1}}}})).unwrap()
    }
    #[test]
    fn compare_identity_missing_and_changed() {
        let a = fixture();
        let mut b = a.clone();
        b.sections.get_mut("equipment").unwrap()["16"]["item_id"] = json!(2);
        assert_eq!(
            compare(&a, &b).unwrap()["sections"]["equipment"]["changed_paths"],
            json!(["/16/item_id"])
        );
        b.sections.remove("equipment");
        assert_eq!(
            compare(&a, &b).unwrap()["sections"]["equipment"]["status"],
            "not_comparable"
        );
        b.client.build = "124".into();
        assert!(compare(&a, &b).is_err());
        b = a.clone();
        b.character.name = "Other".into();
        assert!(compare(&a, &b).is_err());
    }
    #[test]
    fn output_bounded_and_missing_not_zero() {
        let a = fixture();
        assert!(summary(&a)["carried_item_slots_observed"].is_null());
        let mut b = a.clone();
        for n in 0..500 {
            b.sections.get_mut("equipment").unwrap()[n.to_string()] = json!({"item_id":n});
        }
        let result = compare(&a, &b).unwrap();
        assert_eq!(
            result["sections"]["equipment"]["changed_paths"]
                .as_array()
                .unwrap()
                .len(),
            100
        );
        assert_eq!(result["sections"]["equipment"]["truncated"], true);
    }

    #[test]
    fn unavailable_sections_and_malformed_bags_remain_unknown() {
        let mut a = fixture();
        a.unavailable
            .insert("talents".into(), "API unavailable".into());
        let result = compare(&a, &a).unwrap();
        assert_eq!(result["sections"]["talents"]["status"], "not_comparable");
        a.sections.insert("bags".into(), json!({"0": {}, "1": []}));
        assert!(summary(&a)["carried_item_slots_observed"].is_null());
        a.sections.insert("bags".into(), json!({"0": {}}));
        assert_eq!(summary(&a)["carried_item_slots_observed"], 0);
    }

    #[test]
    fn carried_review_counts_locations_without_claiming_stack_compatibility() {
        let mut s = fixture();
        assert!(summary(&s)["carried_item_review"].is_null());
        s.sections.insert(
            "bags".into(),
            json!({"0":{"1":{"item_id":5,"count":3,"link":"variant-a"}},
            "1":{"2":{"item_id":5,"count":7,"link":"variant-b"},"3":{"item_id":6,"count":1}}}),
        );
        s.validate().unwrap();
        let review = summary(&s)["carried_item_review"].clone();
        assert_eq!(review["quantity_observed"], 11);
        assert_eq!(review["distinct_item_ids_observed"], 2);
        assert_eq!(review["repeated_item_ids"][0]["quantity_observed"], 10);
        assert_eq!(
            review["repeated_item_ids"][0]["locations"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(review["stack_compatibility"], "unknown");
        assert_eq!(review["game_execution"], false);
        s.sections.insert("bags".into(), json!({"0":{}}));
        assert_eq!(summary(&s)["carried_item_review"]["quantity_observed"], 0);
        s.sections
            .insert("bags".into(), json!({"0":{"1":{"item_id":1}}}));
        assert!(summary(&s)["carried_item_review"].is_null());
        let mut slots = serde_json::Map::new();
        for slot in 1..=462 {
            slots.insert(slot.to_string(), json!({"item_id":(slot-1)/22+1,"count":1}));
        }
        s.sections.insert("bags".into(), json!({"0":slots}));
        s.validate().unwrap();
        let bounded = summary(&s)["carried_item_review"].clone();
        assert_eq!(bounded["quantity_observed"], 462);
        assert_eq!(bounded["repeated_item_id_count"], 21);
        assert_eq!(bounded["repeated_item_ids"].as_array().unwrap().len(), 20);
        assert_eq!(bounded["groups_truncated"], true);
        assert_eq!(
            bounded["repeated_item_ids"][0]["locations"]
                .as_array()
                .unwrap()
                .len(),
            20
        );
        assert_eq!(bounded["repeated_item_ids"][0]["locations_truncated"], true);
    }
}
