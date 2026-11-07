//! Snapshot-only analysis. Missing data is not an empty inventory.
use crate::Snapshot;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn review_recipe(snapshot: &Snapshot, crafts: u32) -> Result<Value> {
    ensure!(
        snapshot.client.family == crate::Family::Forever,
        "Expected a Forever snapshot"
    );
    ensure!((1..=1000).contains(&crafts), "Craft count out of range");
    let recipe = snapshot
        .sections
        .get("selected_recipe")
        .ok_or_else(|| anyhow::anyhow!("Selected recipe was not captured"))?;
    let fixed = recipe["requirements"] == "fixed_items";
    let storage = ["bags", "character_bank", "account_bank"];
    let mut observed: BTreeMap<&str, u64> = BTreeMap::new();
    let mut missing_storage = Vec::new();
    for name in storage {
        let Some(containers) = snapshot.sections.get(name).and_then(Value::as_object) else {
            missing_storage.push(name);
            continue;
        };
        for item in containers
            .values()
            .filter_map(Value::as_object)
            .flat_map(|slots| slots.values())
        {
            if let (Some(id), Some(count)) = (item["item_id"].as_u64(), item["count"].as_u64())
                && let Some((key, _)) = recipe["reagents"]
                    .as_object()
                    .and_then(|map| map.get_key_value(&id.to_string()))
            {
                *observed.entry(key).or_default() += count;
            }
        }
    }
    let mut materials = Vec::new();
    if fixed {
        for (id, quantity) in recipe["reagents"]
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid reagent data"))?
        {
            let required = quantity
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Invalid reagent quantity"))?
                * u64::from(crafts);
            let available = observed.get(id.as_str()).copied();
            materials.push(json!({"item_id":id.parse::<u32>()?, "required":required,
                "quantity_observed":if missing_storage.len()==storage.len() {None} else {Some(available.unwrap_or(0))},
                "shortfall_vs_observed":if missing_storage.len()==storage.len() {None} else {Some(required.saturating_sub(available.unwrap_or(0)))}}));
        }
    }
    Ok(
        json!({"captured_at":snapshot.captured_at,"recipe":recipe,"requested_crafts":crafts,
        "materials":materials,"unobserved_storage":missing_storage,"live":false,
        "craftability_verified":false,"game_execution":false}),
    )
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
    Ok(
        json!({"client":after.client,"character":after.character,"before_captured_at":before.captured_at,"after_captured_at":after.captured_at,"live":false,"sections":sections}),
    )
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
    let quest_turn_in = s.sections.get("quests").and_then(Value::as_object).map(|quests| {
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
