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
            materials.push(json!({"item_id":item_id, "required":required,
                "quantity_observed":(missing_storage.len()<storage.len()).then_some(available),
                "shortfall_vs_observed":(missing_storage.len()<storage.len()).then_some(required.saturating_sub(available)),
                "quantity_by_storage":by_storage,
                "shortfall_vs_bags":by_storage["bags"].map(|count| required.saturating_sub(count)),
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
    let mut review = json!({"captured_at":snapshot.captured_at,"recipe":recipe,"requested_crafts":crafts,
        "materials":materials,"unobserved_storage":missing_storage,"live":false,
        "material_capacity":{
            "bags":capacity(&bag_limits, !missing_storage.contains(&"bags")),
            "all_observed_storage":capacity(&observed_limits, missing_storage.len() < storage.len())
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
        "stack_compatibility":"unknown","free_slots":"not_captured",
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
    pub achievement_id: Option<u32>,
    pub profession_skill_line: Option<u32>,
    pub wallet: bool,
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
        comparison["reputation_change"] = json!({
            "faction_id":faction_id,"before":previous,"after":current,"standing_delta":standing_delta
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
        let (before_status, previous) = profession_by_skill_line(before, skill_line);
        let (after_status, current) = profession_by_skill_line(after, skill_line);
        let rank_delta = previous
            .zip(current)
            .and_then(|(a, b)| Some(b["rank"].as_i64()? - a["rank"].as_i64()?));
        comparison["profession_change"] = json!({
            "skill_line":skill_line,"before_status":before_status,"after_status":after_status,
            "before":previous,"after":current,"rank_delta":rank_delta
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

fn profession_by_skill_line(
    snapshot: &Snapshot,
    skill_line: u32,
) -> (&'static str, Option<&Value>) {
    let Some(professions) = snapshot
        .sections
        .get("professions")
        .and_then(Value::as_object)
    else {
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

fn quest_checklist(quests: &serde_json::Map<String, Value>) -> Value {
    let (mut ready, mut not_ready, mut failed, mut unknown) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (key, quest) in quests {
        let Ok(id) = key.parse::<u32>() else {
            continue;
        };
        if quest["failed"].as_bool() == Some(true) {
            failed.push(id);
        } else {
            match quest["ready_for_turn_in"].as_bool() {
                Some(true) => ready.push(id),
                Some(false) => not_ready.push(id),
                None => unknown.push(id),
            }
        }
    }
    for ids in [&mut ready, &mut not_ready, &mut failed, &mut unknown] {
        ids.sort_unstable();
    }
    json!({"ready_for_turn_in":ready,"turn_in_not_ready":not_ready,"failed":failed,"turn_in_unknown":unknown})
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
      "equipped_slots_observed":occupied,"carried_item_slots_observed":container_slots(s,"bags"),
      "character_bank_item_slots_observed":container_slots(s,"character_bank"),
      "account_bank_item_slots_observed":container_slots(s,"account_bank"),
      "reputation_factions_observed":s.sections.get("reputation").and_then(|v|v["entries"].as_object()).map(|v|v.len()),
      "currency_balances_observed":s.sections.get("currency_balances").and_then(|v|v["entries"].as_object()).map(|v|v.len()),
      "tracked_achievements_observed":s.sections.get("tracked_achievements").and_then(|v|v["entries"].as_object()).map(|v|v.len()),
      "quest_turn_in":quest_turn_in,
      "quest_checklist":quests.map(quest_checklist),
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
