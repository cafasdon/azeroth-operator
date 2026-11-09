//! Player-reviewed desired equipment, never an executable instruction stream.
use crate::Snapshot;
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub fn captured_loadout(snapshot: &Snapshot) -> Result<Value> {
    snapshot.validate()?;
    let mut review = json!({"client":snapshot.client,"character":snapshot.character,
        "captured_at":snapshot.captured_at,"source":snapshot.source,"live":false,"game_execution":false,
        "status":"unavailable","requires_player_review":true,
        "compatibility":"not_live_verified","optimisation":"not_assessed"});
    if snapshot.client.family != crate::Family::Retail {
        review["reason"] = json!("client_not_supported");
        return Ok(review);
    }
    match snapshot
        .sections
        .get("talents")
        .and_then(|value| value.get("loadout_export"))
    {
        Some(export) if export["status"] == "available" => {
            review["status"] = json!("captured_string_for_review");
            review["loadout"] = export.clone();
            review["limitations"] = json!([
                "Original captured API string; not a newly generated or optimised build",
                "Confirm current character, specialisation and game build before using the in-game talent import UI",
                "String syntax checked; semantic header, tree hash and game acceptance not verified"
            ]);
        }
        Some(export) => review["reason"] = export["reason"].clone(),
        None => review["reason"] = json!("not_captured"),
    }
    Ok(review)
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemLocation {
    Equipment { slot: u32 },
    Bag { bag: u32, slot: u32 },
    CharacterBank { container: u32, slot: u32 },
    AccountBank { container: u32, slot: u32 },
}

impl ItemLocation {
    pub(crate) fn observed_in<'a>(&self, snapshot: &'a Snapshot) -> Option<&'a Value> {
        let (section, container, slot) = match *self {
            Self::Equipment { slot } => {
                return snapshot.sections.get("equipment")?.get(slot.to_string());
            }
            Self::Bag { bag, slot } => ("bags", bag, slot),
            Self::CharacterBank { container, slot }
                if snapshot.client.family == crate::Family::Forever =>
            {
                ("character_bank", container, slot)
            }
            Self::AccountBank { container, slot }
                if snapshot.client.family == crate::Family::Forever =>
            {
                ("account_bank", container, slot)
            }
            _ => return None,
        };
        snapshot
            .sections
            .get(section)?
            .get(container.to_string())?
            .get(slot.to_string())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EquipmentSelection {
    pub target_slot: u32,
    pub source: ItemLocation,
    /// Expected item at this exact exported location, not an arbitrary catalogue ID.
    pub expected_item_id: u32,
}

pub fn equipment_review(snapshot: &Snapshot, selections: &[EquipmentSelection]) -> Result<Value> {
    snapshot.validate()?;
    ensure!(
        selections.len() <= 19
            && (!selections.is_empty() || snapshot.client.family == crate::Family::Forever),
        "Expected 1 to 19 selections, or an empty Forever baseline"
    );
    let equipment = snapshot
        .sections
        .get("equipment")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Equipment observation unavailable"))?;
    let mut targets = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut moved_equipment = BTreeSet::new();
    let mut changes = Vec::new();
    for selection in selections {
        ensure!(
            (1..=19).contains(&selection.target_slot),
            "Invalid equipment slot"
        );
        ensure!(
            targets.insert(selection.target_slot),
            "Duplicate target slot"
        );
        ensure!(
            sources.insert(selection.source.clone()),
            "Source location reused"
        );
        if let ItemLocation::Equipment { slot } = selection.source {
            moved_equipment.insert(slot);
        }
        let item = selection
            .source
            .observed_in(snapshot)
            .ok_or_else(|| anyhow::anyhow!("Source item not observed"))?;
        ensure!(
            item["item_id"].as_u64() == Some(selection.expected_item_id as u64),
            "Source item mismatch"
        );
        let previous = equipment.get(&selection.target_slot.to_string());
        let same_location = selection.source
            == (ItemLocation::Equipment {
                slot: selection.target_slot,
            });
        let native_stat_comparison=previous.and_then(|old|old.get("native_stats")).zip(item.get("native_stats")).map(|(old,new)| {
            let old=old.as_object().unwrap();let new=new.as_object().unwrap();
            let shared:std::collections::BTreeMap<_,_>=old.iter().filter_map(|(key,first)| {
                let second=new.get(key)?;
                Some((key,json!({"before":first,"after":second,"delta":second.as_f64().unwrap()-first.as_f64().unwrap()})))
            }).collect();
            json!({"shared_tokens":shared,
                "observed_only_before":old.iter().filter(|(key,_)|!new.contains_key(*key)).collect::<std::collections::BTreeMap<_,_>>(),
                "observed_only_after":new.iter().filter(|(key,_)|!old.contains_key(*key)).collect::<std::collections::BTreeMap<_,_>>(),
                "effect_on_performance":"unverified"})
        });
        changes.push(
            json!({"target_slot":selection.target_slot,"source":selection.source,
            "expected_item_id":selection.expected_item_id,"selected_item_observed":item,
            "previous_item_observed":previous,"native_stat_comparison":native_stat_comparison,
            "native_slot_category":if snapshot.client.family==crate::Family::Forever {native_slot_category(item,selection.target_slot)} else {Value::Null},
            "status":if same_location {"already_selected"} else {"requested_change"}}),
        );
    }
    changes.sort_by_key(|row| row["target_slot"].as_u64().unwrap());
    let vacated: Vec<_> = moved_equipment.difference(&targets).copied().collect();
    let captured_level = snapshot
        .sections
        .get("stats")
        .and_then(|stats| stats["level"].as_u64())
        .filter(|level| *level > 0);
    Ok(
        json!({"format":"azeroth_operator.equipment_review.v1","captured_character_level":captured_level,
        "captured_professions":snapshot.sections.get("professions"),
        "captured_talent_observation":snapshot.sections.get("talents")
            .filter(|talents| snapshot.client.family==crate::Family::Forever && talents.get("observed_tree_ids").is_some())
            .map(|talents| {
                let mut fields=serde_json::Map::new();
                for key in ["config_id","nodes","observed_tree_ids","observed_node_count","staged_changes_observed","definition_allocation"] {
                    if let Some(value)=talents.get(key) {fields.insert(key.into(),value.clone());}
                }
                Value::Object(fields)
            }),
        "captured_active_pet":snapshot.sections.get("pet_care").map(|pet| json!({
            "present":pet["present"],"family_id":pet["family_id"],"level":pet["level"]})),
        "captured_character_identity":{
            "class":snapshot.sections.get("stats").and_then(|stats| stats.get("class")),
            "race":snapshot.sections.get("stats").and_then(|stats| stats.get("race"))},
        "client":snapshot.client,"character":snapshot.character,"captured_at":snapshot.captured_at,
        "source":snapshot.source,"live":false,"game_execution":false,"requires_player_review":true,
        "status":"draft_unverified_compatibility","changes":changes,"vacated_equipment_slots":vacated,
        "checks":{"selected_locations_observed":true,"item_ids_match_snapshot":true,
          "unique_sources_and_targets":true,"slot_compatibility":"unknown","class_level_requirements":"unknown",
          "unique_equip_restrictions":"unknown","weapon_pairing":"unknown","inventory_capacity":"unknown",
          "current_ownership":"unknown","performance_improvement":"unknown"},
        "limitations":["Observed locations are bound to this capture and may have changed",
          "No item GUIDs or account/region identity are available in this snapshot schema",
          "This is a desired selection review, not a game import, macro, move order or equipment optimisation",
          "Displaced items have no assigned destination; review inventory capacity in the game"]}),
    )
}

pub(crate) fn native_slot_category(item: &Value, target: u32) -> Value {
    let location = item["equip_location"].as_str();
    let slots: Option<&[u32]> = match location {
        Some("INVTYPE_HEAD") => Some(&[1]),
        Some("INVTYPE_NECK") => Some(&[2]),
        Some("INVTYPE_SHOULDER") => Some(&[3]),
        Some("INVTYPE_BODY") => Some(&[4]),
        Some("INVTYPE_CHEST" | "INVTYPE_ROBE") => Some(&[5]),
        Some("INVTYPE_WAIST") => Some(&[6]),
        Some("INVTYPE_LEGS") => Some(&[7]),
        Some("INVTYPE_FEET") => Some(&[8]),
        Some("INVTYPE_WRIST") => Some(&[9]),
        Some("INVTYPE_HAND") => Some(&[10]),
        Some("INVTYPE_FINGER") => Some(&[11, 12]),
        Some("INVTYPE_TRINKET") => Some(&[13, 14]),
        Some("INVTYPE_CLOAK") => Some(&[15]),
        Some("INVTYPE_WEAPONMAINHAND") => Some(&[16]),
        Some("INVTYPE_WEAPONOFFHAND" | "INVTYPE_SHIELD" | "INVTYPE_HOLDABLE") => Some(&[17]),
        Some("INVTYPE_WEAPON" | "INVTYPE_2HWEAPON") => Some(&[16, 17]),
        Some("INVTYPE_RANGED" | "INVTYPE_RANGEDRIGHT" | "INVTYPE_THROWN" | "INVTYPE_RELIC") => {
            Some(&[18])
        }
        Some("INVTYPE_TABARD") => Some(&[19]),
        _ => None,
    };
    let matches = slots.and_then(|slots| {
        if location == Some("INVTYPE_2HWEAPON") && target == 17 {
            None
        } else {
            Some(slots.contains(&target))
        }
    });
    json!({"observed_equip_location":location,"candidate_equipment_slots":slots,"target_slot_category_matches":matches,
        "eligibility_verified":false})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Snapshot {
        serde_json::from_value(
            json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
          "client":{"family":"retail","variant":"live","build":"123","interface":120105},
          "character":{"name":"Example","realm":"Test"},
          "sections":{"equipment":{"11":{"item_id":10},"12":{"item_id":20}},
          "bags":{"0":{"1":{"item_id":30,"count":1},"2":{"item_id":30,"count":1}}}}}),
        )
        .unwrap()
    }
    #[test]
    fn captured_loadouts_preserve_strings_and_report_missing_or_unsupported() {
        let mut snapshot = fixture();
        assert_eq!(
            captured_loadout(&snapshot).unwrap()["reason"],
            "not_captured"
        );
        snapshot.sections.insert("talents".into(), json!({"config_id":7,"nodes":{},"loadout_export":{
            "status":"available","format":"blizzard_traits","import_string":"QUJDRA==","spec_id":263,
            "serialization_version":2,"staged_changes":false}}));
        let review = captured_loadout(&snapshot).unwrap();
        assert_eq!(review["loadout"]["import_string"], "QUJDRA==");
        assert_eq!(review["game_execution"], false);
        assert_eq!(review["compatibility"], "not_live_verified");
        for invalid in [
            json!("/run bad"),
            json!(""),
            json!(123),
            json!("A".repeat(16385)),
        ] {
            let mut malformed = snapshot.clone();
            malformed.sections.get_mut("talents").unwrap()["loadout_export"]["import_string"] =
                invalid;
            assert!(captured_loadout(&malformed).is_err());
        }
        let mut malformed = snapshot.clone();
        malformed.sections.get_mut("talents").unwrap()["loadout_export"]["staged_changes"] =
            json!(true);
        assert!(captured_loadout(&malformed).is_err());
        snapshot.client.family = crate::Family::Classic;
        assert_eq!(
            captured_loadout(&snapshot).unwrap()["reason"],
            "client_not_supported"
        );
    }
    fn bag(target: u32, slot: u32) -> EquipmentSelection {
        EquipmentSelection {
            target_slot: target,
            source: ItemLocation::Bag { bag: 0, slot },
            expected_item_id: 30,
        }
    }
    #[test]
    fn bound_observations_and_unknowns_for_all_families() {
        let mut snapshot = fixture();
        for family in [
            crate::Family::Retail,
            crate::Family::Classic,
            crate::Family::Forever,
        ] {
            snapshot.client.family = family;
            let review = equipment_review(&snapshot, &[bag(11, 1), bag(12, 2)]).unwrap();
            assert_eq!(
                review["changes"][0]["previous_item_observed"]["item_id"],
                10
            );
            assert_eq!(
                review["changes"][0]["selected_item_observed"]["item_id"],
                30
            );
            assert_eq!(review["checks"]["performance_improvement"], "unknown");
            assert_eq!(review["game_execution"], false);
            assert_eq!(
                review["client"]["family"],
                serde_json::to_value(&snapshot.client.family).unwrap()
            );
        }
    }
    #[test]
    fn swaps_and_vacancies_do_not_duplicate_items() {
        let snapshot = fixture();
        let a = EquipmentSelection {
            target_slot: 12,
            source: ItemLocation::Equipment { slot: 11 },
            expected_item_id: 10,
        };
        let b = EquipmentSelection {
            target_slot: 11,
            source: ItemLocation::Equipment { slot: 12 },
            expected_item_id: 20,
        };
        assert_eq!(
            equipment_review(&snapshot, std::slice::from_ref(&a)).unwrap()["vacated_equipment_slots"],
            json!([11])
        );
        assert_eq!(
            equipment_review(&snapshot, &[a, b]).unwrap()["vacated_equipment_slots"],
            json!([])
        );
    }
    #[test]
    fn rejects_unobserved_reused_mismatched_or_missing_data() {
        let mut snapshot = fixture();
        for selections in [
            vec![],
            vec![bag(0, 1)],
            vec![bag(20, 1)],
            vec![bag(11, 9)],
            vec![bag(11, 1), bag(11, 2)],
            vec![bag(11, 1), bag(12, 1)],
        ] {
            assert!(equipment_review(&snapshot, &selections).is_err());
        }
        let mut wrong = bag(11, 1);
        wrong.expected_item_id = 99;
        assert!(equipment_review(&snapshot, &[wrong]).is_err());
        snapshot.sections.remove("bags");
        assert!(equipment_review(&snapshot, &[bag(11, 1)]).is_err());
        snapshot.sections.remove("equipment");
        assert!(equipment_review(&snapshot, &[bag(11, 1)]).is_err());
    }
}
