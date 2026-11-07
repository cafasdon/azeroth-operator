//! Version-one payload validation matching the Lua export contract.
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Item<'a> {
    item_id: u32,
    #[serde(borrow)]
    link: Option<&'a str>,
    count: Option<u32>,
    guid: Option<&'a str>,
}

fn index(key: &str, minimum: u32, maximum: u32) -> Result<()> {
    let value: u32 = key.parse()?;
    ensure!(
        value >= minimum && value <= maximum && value.to_string() == key,
        "Invalid slot index"
    );
    Ok(())
}

fn items(value: &Value, equipment: bool) -> Result<()> {
    let slots = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Slots must be an object"))?;
    for (slot, value) in slots {
        index(slot, 1, if equipment { 19 } else { 1024 })?;
        let item = Item::deserialize(value)?;
        ensure!(item.item_id > 0, "Invalid item ID");
        ensure!(
            item.guid.is_none_or(|s| !s.is_empty()
                && s.len() <= 128
                && !s.chars().any(char::is_control)),
            "Invalid item GUID"
        );
        ensure!(
            item.link.as_ref().is_none_or(|s| s.len() <= 4096),
            "Item link too long"
        );
        ensure!(
            !equipment || item.count.is_none(),
            "Equipment cannot carry a stack count"
        );
        ensure!(
            equipment || item.count.is_some_and(|n| n > 0),
            "Missing or invalid stack count"
        );
    }
    Ok(())
}

// Bounds describe accepted export data, not limits of any particular game client.
fn object(value: &Value, fields: &[(&str, &str, bool)]) -> Result<()> {
    let map = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected object"))?;
    ensure!(
        map.keys()
            .all(|key| fields.iter().any(|(name, _, _)| name == key)),
        "Unknown field"
    );
    for (name, kind, required) in fields {
        let Some(v) = map.get(*name).filter(|v| !v.is_null()) else {
            ensure!(!required, "Missing required field: {name}");
            continue;
        };
        let valid = match *kind {
            "text" => v.as_str().is_some_and(|s| s.len() <= 16384),
            "uint" => v.as_u64().is_some_and(|n| n <= u32::MAX as u64),
            "wallet_copper" => v.as_u64().is_some_and(|n| n <= 9_999_999_999_999),
            "faction_tag" => matches!(v.as_str(), Some("Alliance" | "Horde")),
            "positive" => v.as_u64().is_some_and(|n| n > 0 && n <= u32::MAX as u64),
            "int" => v.as_i64().is_some_and(|n| i32::try_from(n).is_ok()),
            "bool" => v.is_boolean(),
            "icon" => v.as_u64().is_some() || v.as_str().is_some_and(|s| s.len() <= 4096),
            "map" => v.is_object(),
            literal => v.as_str() == Some(literal),
        };
        ensure!(valid, "Invalid field: {name}");
    }
    Ok(())
}

fn entries(value: &Value, fields: &[(&str, &str, bool)]) -> Result<()> {
    let map = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected entry map"))?;
    ensure!(map.len() <= 10000, "Too many entries");
    for (key, entry) in map {
        index(key, 1, u32::MAX)?;
        object(entry, fields)?;
    }
    Ok(())
}

pub fn validate(name: &str, value: &Value) -> Result<()> {
    match name {
        "binding_state" => {
            object(
                value,
                &[
                    ("binding_set", "uint", true),
                    ("coverage", "bound_keys_only", true),
                ],
            )?;
            ensure!(
                matches!(value["binding_set"].as_u64(), Some(1 | 2)),
                "Invalid binding set"
            );
        }
        "operation_receipts" => {
            let receipts = value
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Expected receipt map"))?;
            ensure!(receipts.len() <= 128, "Too many receipts");
            for (id, receipt) in receipts {
                ensure!(
                    id.len() == 64
                        && id
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                    "Invalid operation ID"
                );
                object(receipt, &[("status", "text", true), ("kind", "text", true)])?;
                ensure!(
                    matches!(
                        receipt["status"].as_str(),
                        Some(
                            "applied"
                                | "outcome_unknown"
                                | "copied_source_removal_unconfirmed"
                                | "already_selected"
                        )
                    ),
                    "Invalid receipt status"
                );
                ensure!(
                    matches!(
                        receipt["kind"].as_str(),
                        Some(
                            "create"
                                | "edit"
                                | "delete"
                                | "move"
                                | "set_binding"
                                | "equip_bag_item"
                                | "move_bag_item"
                                | "split_bag_item"
                                | "load_talent_config"
                                | "create_talent_config"
                                | "import_talent_config"
                                | "rename_talent_config"
                                | "delete_talent_config"
                                | "move_action_slot"
                                | "copy_action_slot"
                                | "swap_bag_items"
                                | "modify_equipment_set"
                                | "delete_equipment_set"
                                | "use_equipment_set"
                                | "assign_equipment_set_spec"
                                | "unassign_equipment_set_spec"
                                | "create_equipment_set"
                                | "save_equipment_set"
                        )
                    ),
                    "Invalid receipt kind"
                );
            }
        }
        "equipment" => items(value, true)?,
        "equipment_sets" => {
            object(
                value,
                &[
                    ("coverage", "native_catalogue_raw_slots", true),
                    ("can_use", "bool", true),
                    ("entries", "map", true),
                    ("specializations", "map", false),
                ],
            )?;
            if let Some(specs) = value.get("specializations") {
                let specs = specs.as_object().unwrap();
                ensure!(
                    !specs.is_empty() && specs.len() <= 64,
                    "Invalid specialisation catalogue size"
                );
                let mut ids = std::collections::BTreeSet::new();
                for i in 1..=specs.len() {
                    let id = specs
                        .get(&i.to_string())
                        .and_then(Value::as_u64)
                        .ok_or_else(|| anyhow::anyhow!("Incomplete specialisation catalogue"))?;
                    ensure!(
                        id > 0 && id <= u32::MAX as u64 && ids.insert(id),
                        "Invalid or duplicate specialisation ID"
                    );
                }
            }
            let sets = value["entries"].as_object().unwrap();
            ensure!(sets.len() <= 128, "Too many equipment sets");
            for (id, entry) in sets {
                if let (Some(specs), Some(index)) =
                    (value.get("specializations"), entry.get("assigned_spec"))
                {
                    ensure!(
                        specs.get(index.to_string()).is_some(),
                        "Assigned specialisation not captured"
                    );
                }
                index(id, 0, u32::MAX)?;
                object(
                    entry,
                    &[
                        ("name", "text", true),
                        ("icon", "uint", true),
                        ("is_equipped", "bool", true),
                        ("num_items", "uint", true),
                        ("num_equipped", "uint", true),
                        ("num_in_inventory", "uint", true),
                        ("num_lost", "uint", true),
                        ("num_ignored", "uint", true),
                        ("assigned_spec", "positive", false),
                        ("slots", "map", true),
                    ],
                )?;
                ensure!(
                    entry["name"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty() && s.len() <= 1024),
                    "Invalid set name"
                );
                for key in [
                    "num_items",
                    "num_equipped",
                    "num_in_inventory",
                    "num_lost",
                    "num_ignored",
                ] {
                    ensure!(
                        entry[key].as_u64().unwrap() <= 19,
                        "Invalid equipment set count"
                    );
                }
                ensure!(
                    entry
                        .get("assigned_spec")
                        .is_none_or(|s| s.as_u64().is_some_and(|n| (1..=64).contains(&n))),
                    "Invalid assigned spec"
                );
                let slots = entry["slots"].as_object().unwrap();
                ensure!(slots.len() == 19, "Incomplete equipment set slots");
                for (slot, data) in slots {
                    index(slot, 1, 19)?;
                    let fields = data
                        .as_object()
                        .ok_or_else(|| anyhow::anyhow!("Invalid set slot"))?;
                    ensure!(
                        fields.keys().all(|k| matches!(
                            k.as_str(),
                            "ignored" | "raw_item_id" | "raw_location" | "observed_item"
                        )),
                        "Unknown set slot field"
                    );
                    for (key, field) in fields {
                        let valid = match key.as_str() {
                            "observed_item" => {
                                object(
                                    field,
                                    &[
                                        ("kind", "text", true),
                                        ("slot", "positive", true),
                                        ("bag", "uint", false),
                                        ("item_id", "positive", true),
                                        ("guid", "text", true),
                                        ("locked", "bool", true),
                                    ],
                                )?;
                                ensure!(
                                    data["ignored"] != true
                                        && data["raw_item_id"] == field["item_id"]
                                        && data["raw_location"].as_i64().is_some_and(|n| n >= 0),
                                    "Unbound set item observation"
                                );
                                ensure!(
                                    field["guid"].as_str().is_some_and(|s| !s.is_empty()
                                        && s.len() <= 128
                                        && !s.chars().any(char::is_control)),
                                    "Invalid set item GUID"
                                );
                                match field["kind"].as_str() {
                                    Some("equipment") => {
                                        field.get("bag").is_none()
                                            && field["slot"].as_u64().unwrap() <= 19
                                    }
                                    Some("bag") => {
                                        field["bag"].as_u64().is_some_and(|n| n <= 64)
                                            && field["slot"].as_u64().unwrap() <= 1024
                                    }
                                    _ => false,
                                }
                            }
                            "ignored" => field.is_boolean(),
                            "raw_item_id" => field
                                .as_i64()
                                .is_some_and(|n| (-2147483648..=4294967295).contains(&n)),
                            "raw_location" => field
                                .as_i64()
                                .is_some_and(|n| (-2147483648..=4294967295).contains(&n)),
                            _ => false,
                        };
                        ensure!(valid, "Invalid raw set slot field");
                    }
                }
            }
        }
        "bags" | "character_bank" | "account_bank" => {
            let bags = value
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Bags must be an object"))?;
            for (bag, slots) in bags {
                index(bag, 0, 64)?;
                items(slots, false)?;
            }
        }
        "stats" => {
            object(
                value,
                &[
                    ("level", "int", true),
                    ("class", "text", true),
                    ("race", "text", true),
                    ("faction", "faction_tag", false),
                    ("map_id", "positive", false),
                    ("map_name", "text", false),
                    ("map_parent_id", "positive", false),
                    ("client_version", "text", true),
                ],
            )?;
            let map_id = value["map_id"].as_u64();
            if let Some(name) = value["map_name"].as_str() {
                ensure!(
                    map_id.is_some() && !name.is_empty() && name.len() <= 256,
                    "Invalid captured map name"
                );
            }
            if let Some(parent) = value["map_parent_id"].as_u64() {
                ensure!(
                    map_id.is_some_and(|id| id != parent),
                    "Invalid captured parent map"
                );
            }
        }
        "macros" => {
            entries(
                value,
                &[
                    ("name", "text", true),
                    ("icon", "icon", false),
                    ("body", "text", false),
                    ("scope", "text", true),
                ],
            )?;
            for entry in value.as_object().unwrap().values() {
                ensure!(
                    matches!(entry["scope"].as_str(), Some("account" | "character")),
                    "Invalid macro scope"
                );
            }
        }
        "bindings" => entries(
            value,
            &[
                ("command", "text", true),
                ("category", "text", false),
                ("key1", "text", false),
                ("key2", "text", false),
                ("context", "uint", false),
            ],
        )?,
        "action_bar_observation" => {
            object(
                value,
                &[
                    ("coverage", "loaded_builtin_buttons_only", true),
                    ("slots", "map", true),
                    ("observed_buttons", "positive", true),
                    ("missing_buttons", "uint", true),
                    ("button_hotkeys", "map", false),
                ],
            )?;
            let observed = value["observed_buttons"].as_u64().unwrap();
            let missing = value["missing_buttons"].as_u64().unwrap();
            ensure!(observed + missing <= 2048, "Too many action buttons");
            let slots = value["slots"].as_object().unwrap();
            ensure!(
                !slots.is_empty() && slots.len() as u64 <= observed,
                "Invalid action coverage"
            );
            if let Some(hotkeys) = value.get("button_hotkeys").filter(|v| !v.is_null()) {
                entries(
                    hotkeys,
                    &[
                        ("button", "text", true),
                        ("slot", "positive", true),
                        ("key1", "text", false),
                        ("key2", "text", false),
                    ],
                )?;
                let mut buttons = std::collections::BTreeSet::new();
                for (key, row) in hotkeys.as_object().unwrap() {
                    index(key, 1, observed as u32)?;
                    let button = row["button"].as_str().unwrap();
                    let slot = row["slot"].as_u64().unwrap();
                    ensure!(
                        slots.contains_key(&slot.to_string())
                            && buttons.insert(button)
                            && !button.is_empty()
                            && button.len() <= 132
                            && button
                                .bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                            && ["key1", "key2"].iter().all(|key| row[*key]
                                .as_str()
                                .is_none_or(|value| !value.is_empty() && value.len() <= 128)),
                        "Invalid observed action button hotkeys"
                    );
                }
            }
            for (slot, entry) in slots {
                index(slot, 1, 4096)?;
                object(
                    entry,
                    &[
                        ("empty", "bool", true),
                        ("kind", "text", false),
                        ("id", "positive", false),
                        ("sub_type", "text", false),
                    ],
                )?;
                if entry["empty"] == true {
                    ensure!(
                        entry.as_object().unwrap().len() == 1,
                        "Empty action contains data"
                    );
                } else {
                    ensure!(
                        entry["kind"].as_str().is_some_and(|s| !s.is_empty()),
                        "Missing action kind"
                    );
                }
            }
        }
        "action_bars" => entries(
            value,
            &[
                ("kind", "text", true),
                ("id", "positive", false),
                ("sub_type", "text", false),
            ],
        )?,
        "professions" => entries(
            value,
            &[
                ("name", "text", true),
                ("rank", "uint", true),
                ("max_rank", "uint", true),
                ("skill_line", "positive", true),
            ],
        )?,
        "tracked_recipes" => {
            object(
                value,
                &[
                    ("coverage", "player_tracked_recipes_only", true),
                    ("crafts", "map", true),
                    ("recrafts", "map", true),
                ],
            )?;
            for kind in ["crafts", "recrafts"] {
                let recipes = &value[kind];
                entries(
                    recipes,
                    &[("name", "text", false), ("learned", "bool", false)],
                )?;
                let recipes = recipes.as_object().unwrap();
                ensure!(recipes.len() <= 128, "Too many tracked recipes");
                ensure!(
                    recipes.values().all(|recipe| recipe["name"]
                        .as_str()
                        .is_none_or(|name| !name.is_empty() && name.len() <= 256)),
                    "Invalid tracked recipe name"
                );
            }
        }
        "selected_recipe" => {
            object(
                value,
                &[
                    ("recipe_id", "positive", true),
                    ("name", "text", true),
                    ("learned", "bool", true),
                    ("relative_difficulty", "text", false),
                    ("can_skill_up", "bool", false),
                    ("skill_ups_reported", "uint", false),
                    ("profession", "map", false),
                    ("cooldown", "map", false),
                    ("crafting_conditions", "map", false),
                    ("output_item_id", "positive", false),
                    ("base_output_quantity", "map", false),
                    ("requirements", "text", true),
                    ("reagents", "map", true),
                ],
            )?;
            let name = value["name"].as_str().unwrap();
            ensure!(!name.is_empty() && name.len() <= 256, "Invalid recipe name");
            if let Some(profession) = value.get("profession").filter(|v| !v.is_null()) {
                object(
                    profession,
                    &[
                        ("profession_id", "positive", true),
                        ("name", "text", true),
                        ("rank", "uint", true),
                        ("max_rank", "uint", true),
                    ],
                )?;
                ensure!(
                    profession["name"]
                        .as_str()
                        .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                    "Invalid recipe profession name"
                );
            }
            ensure!(
                value["relative_difficulty"].is_null()
                    || matches!(
                        value["relative_difficulty"].as_str(),
                        Some("optimal" | "medium" | "easy" | "trivial")
                    ),
                "Invalid recipe difficulty"
            );
            if let Some(quantity) = value
                .get("base_output_quantity")
                .filter(|value| !value.is_null())
            {
                object(
                    quantity,
                    &[("minimum", "positive", true), ("maximum", "positive", true)],
                )?;
                ensure!(
                    value["output_item_id"].as_u64().is_some()
                        && quantity["minimum"].as_u64() <= quantity["maximum"].as_u64(),
                    "Invalid recipe output range"
                );
            }
            if let Some(cooldown) = value.get("cooldown").filter(|value| !value.is_null()) {
                object(
                    cooldown,
                    &[
                        ("remaining_seconds", "uint", false),
                        ("daily", "bool", false),
                        ("charges", "uint", false),
                        ("max_charges", "positive", false),
                    ],
                )?;
                let seconds = cooldown["remaining_seconds"].as_u64();
                let charges = cooldown["charges"].as_u64();
                let maximum = cooldown["max_charges"].as_u64();
                ensure!(
                    (seconds.is_some() || charges.is_some())
                        && (cooldown["daily"].is_null() || seconds.is_some())
                        && charges.is_some() == maximum.is_some()
                        && charges <= maximum,
                    "Invalid recipe cooldown observation"
                );
            }
            if let Some(conditions) = value
                .get("crafting_conditions")
                .filter(|value| !value.is_null())
            {
                entries(
                    conditions,
                    &[
                        ("name", "text", true),
                        ("met", "bool", true),
                        ("kind", "text", true),
                    ],
                )?;
                let conditions = conditions.as_object().unwrap();
                ensure!(conditions.len() <= 32, "Too many crafting conditions");
                for (key, condition) in conditions {
                    index(key, 1, conditions.len() as u32)?;
                    ensure!(
                        condition["name"]
                            .as_str()
                            .is_some_and(|name| !name.is_empty() && name.len() <= 256)
                            && matches!(
                                condition["kind"].as_str(),
                                Some("spell_focus" | "tool" | "area")
                            ),
                        "Invalid crafting condition"
                    );
                }
            }
            let reagents = value["reagents"].as_object().unwrap();
            ensure!(reagents.len() <= 64, "Too many required reagents");
            ensure!(
                value["requirements"] == "fixed_items"
                    || (value["requirements"] == "unsupported_required_slots"
                        && reagents.is_empty()),
                "Invalid recipe requirements"
            );
            for (id, quantity) in reagents {
                index(id, 1, u32::MAX)?;
                ensure!(
                    quantity
                        .as_u64()
                        .is_some_and(|n| (1..=u32::MAX as u64).contains(&n)),
                    "Invalid reagent quantity"
                );
            }
        }
        "reputation" => {
            object(
                value,
                &[
                    ("coverage", "visible_faction_rows_only", true),
                    ("entries", "map", true),
                ],
            )?;
            entries(
                &value["entries"],
                &[
                    ("name", "text", true),
                    ("kind", "text", true),
                    ("account_wide", "bool", true),
                    ("reaction", "positive", true),
                    ("standing", "int", true),
                    ("threshold", "int", true),
                    ("next_threshold", "int", true),
                    ("renown", "map", false),
                    ("friendship", "map", false),
                    ("paragon", "map", false),
                ],
            )?;
            let factions = value["entries"].as_object().unwrap();
            ensure!(factions.len() <= 512, "Too many reputation observations");
            for faction in factions.values() {
                ensure!(
                    matches!(
                        faction["kind"].as_str(),
                        Some("standard" | "renown" | "friendship" | "paragon" | "unknown")
                    ) && faction["reaction"].as_u64().is_some_and(|n| n <= 8)
                        && faction["name"]
                            .as_str()
                            .is_some_and(|s| !s.is_empty() && s.len() <= 256),
                    "Invalid reputation observation"
                );
                if let Some(renown) = faction.get("renown").filter(|value| !value.is_null()) {
                    ensure!(faction["kind"] == "renown", "Unexpected renown observation");
                    object(
                        renown,
                        &[
                            ("level", "uint", true),
                            ("earned_in_level", "uint", true),
                            ("level_threshold", "uint", true),
                            ("capped", "bool", true),
                        ],
                    )?;
                }
                if let Some(friendship) = faction.get("friendship").filter(|value| !value.is_null())
                {
                    ensure!(
                        faction["kind"] == "friendship",
                        "Unexpected friendship observation"
                    );
                    object(
                        friendship,
                        &[
                            ("standing", "int", true),
                            ("threshold", "int", true),
                            ("next_threshold", "int", false),
                        ],
                    )?;
                }
                if let Some(paragon) = faction.get("paragon").filter(|value| !value.is_null()) {
                    ensure!(
                        faction["kind"] == "paragon",
                        "Unexpected paragon observation"
                    );
                    object(
                        paragon,
                        &[
                            ("current_value", "uint", true),
                            ("reward_threshold", "positive", true),
                            ("reward_pending", "bool", true),
                            ("level_too_low", "bool", true),
                            ("reward_quest_id", "positive", false),
                        ],
                    )?;
                }
            }
        }
        "quests" => {
            entries(
                value,
                &[
                    ("title", "text", true),
                    ("level", "int", true),
                    ("ready_for_turn_in", "bool", false),
                    ("failed", "bool", false),
                    ("next_waypoint_map_id", "positive", false),
                    ("required_copper", "wallet_copper", false),
                    ("timer", "map", false),
                    ("elite", "bool", false),
                    ("suggested_group_size", "uint", false),
                    ("currency_rewards", "map", false),
                    ("currency_choices", "map", false),
                    ("item_choices", "map", false),
                    ("item_rewards", "map", false),
                    ("reward_amounts", "map", false),
                    ("renown_rewards", "map", false),
                    ("spell_rewards", "map", false),
                    ("objectives", "map", false),
                ],
            )?;
            for quest in value.as_object().unwrap().values() {
                if let Some(timer) = quest.get("timer").filter(|v| !v.is_null()) {
                    object(
                        timer,
                        &[
                            ("total_seconds", "positive", true),
                            ("remaining_seconds", "uint", true),
                        ],
                    )?;
                    ensure!(
                        timer["remaining_seconds"].as_u64() <= timer["total_seconds"].as_u64(),
                        "Invalid quest timer"
                    );
                }
                if let Some(spells) = quest.get("spell_rewards").filter(|v| !v.is_null()) {
                    entries(
                        spells,
                        &[
                            ("spell_id", "positive", true),
                            ("reward_type", "uint", false),
                            ("is_spell_learned", "bool", false),
                            ("name", "text", false),
                            ("in_player_spellbook", "bool", false),
                        ],
                    )?;
                    let spells = spells.as_object().unwrap();
                    ensure!(spells.len() <= 32, "Too many quest spell rewards");
                    for (key, spell) in spells {
                        index(key, 1, spells.len() as u32)?;
                        ensure!(
                            spell["name"]
                                .as_str()
                                .is_none_or(|name| !name.is_empty() && name.len() <= 256),
                            "Invalid quest reward spell name"
                        );
                    }
                }
                if let Some(amounts) = quest.get("reward_amounts").filter(|v| !v.is_null()) {
                    object(
                        amounts,
                        &[
                            ("copper", "wallet_copper", false),
                            ("xp", "uint", false),
                            ("honor", "uint", false),
                        ],
                    )?;
                    ensure!(
                        ["copper", "xp", "honor"]
                            .iter()
                            .any(|key| amounts[*key].is_u64()),
                        "No observed quest reward amounts"
                    );
                }
                for (field, id, is_choice) in [
                    ("currency_rewards", "currency_id", false),
                    ("currency_choices", "currency_id", true),
                    ("item_rewards", "item_id", false),
                    ("item_choices", "item_id", true),
                    ("renown_rewards", "faction_id", false),
                ] {
                    let Some(rewards) = quest.get(field).filter(|value| !value.is_null()) else {
                        continue;
                    };
                    entries(
                        rewards,
                        &[(id, "positive", true), ("quantity", "positive", true)],
                    )?;
                    let rewards = rewards.as_object().unwrap();
                    ensure!(rewards.len() <= 32, "Too many quest rewards");
                    let max_index = if is_choice { 32 } else { rewards.len() as u32 };
                    for key in rewards.keys() {
                        index(key, 1, max_index)?;
                    }
                }
                if let Some(objectives) = quest.get("objectives").filter(|value| !value.is_null()) {
                    entries(
                        objectives,
                        &[
                            ("kind", "text", true),
                            ("finished", "bool", true),
                            ("fulfilled", "uint", false),
                            ("required", "uint", false),
                        ],
                    )?;
                    let objectives = objectives.as_object().unwrap();
                    ensure!(objectives.len() <= 64, "Too many quest objectives");
                    for (key, objective) in objectives {
                        index(key, 1, objectives.len() as u32)?;
                        ensure!(
                            objective["kind"]
                                .as_str()
                                .is_some_and(|value| !value.is_empty() && value.len() <= 64)
                                && objective["fulfilled"].is_null()
                                    == objective["required"].is_null(),
                            "Invalid quest objective"
                        );
                    }
                }
            }
        }
        "currencies" => {
            object(
                value,
                &[
                    ("coverage", "visible_currency_list_only", true),
                    ("entries", "map", true),
                ],
            )?;
            entries(
                &value["entries"],
                &[("name", "text", true), ("quantity", "uint", true)],
            )?;
        }
        "wallet" => object(value, &[("copper", "wallet_copper", true)])?,
        "spellbook" => {
            object(
                value,
                &[
                    ("coverage", "visible_player_spell_slots", true),
                    ("entries", "map", true),
                ],
            )?;
            entries(
                &value["entries"],
                &[
                    ("base_spell_id", "positive", true),
                    ("spell_id", "positive", true),
                    ("name", "text", false),
                    ("passive", "bool", false),
                    ("off_spec", "bool", false),
                ],
            )?;
            let rows = value["entries"].as_object().unwrap();
            ensure!(rows.len() <= 1024, "Too many spellbook observations");
            for (key, row) in rows {
                index(key, 1, 4096)?;
                ensure!(
                    row["name"]
                        .as_str()
                        .is_none_or(|name| !name.is_empty() && name.len() <= 256),
                    "Invalid spellbook spell name"
                );
            }
        }
        "experience" => {
            object(
                value,
                &[
                    ("status", "text", true),
                    ("current_xp", "uint", false),
                    ("level_threshold_xp", "positive", false),
                    ("xp_disabled", "bool", false),
                    ("rested_xp", "uint", false),
                ],
            )?;
            let current = value["current_xp"].as_u64();
            let threshold = value["level_threshold_xp"].as_u64();
            match value["status"].as_str() {
                Some("progress") => ensure!(
                    current
                        .zip(threshold)
                        .is_some_and(|(current, threshold)| current <= threshold),
                    "Invalid XP progress"
                ),
                Some("trial_capped" | "level_capped") => ensure!(
                    current.is_none()
                        && threshold.is_none()
                        && value["xp_disabled"].is_null()
                        && value["rested_xp"].is_null(),
                    "Unexpected capped XP details"
                ),
                _ => anyhow::bail!("Unknown XP status"),
            }
        }
        "currency_balances" => {
            object(
                value,
                &[
                    ("coverage", "visible_currency_ids_only", true),
                    ("entries", "map", true),
                ],
            )?;
            entries(
                &value["entries"],
                &[
                    ("name", "text", true),
                    ("quantity", "uint", true),
                    ("account_wide", "bool", true),
                    ("inactive", "bool", true),
                    ("max_quantity", "uint", true),
                    ("max_weekly_quantity", "uint", true),
                    ("earned_this_week", "uint", true),
                    ("total_earned", "uint", true),
                    ("cap_uses_total_earned", "bool", true),
                    ("can_earn_per_week", "bool", false),
                ],
            )?;
            let balances = value["entries"].as_object().unwrap();
            ensure!(balances.len() <= 512, "Too many currency observations");
            ensure!(
                balances.values().all(|entry| entry["name"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty() && s.len() <= 256)),
                "Invalid currency name"
            );
        }
        "achievements" => object(
            value,
            &[
                ("coverage", "points_only", true),
                ("total_points", "uint", true),
            ],
        )?,
        "tracked_achievements" => {
            object(
                value,
                &[
                    ("coverage", "tracked_player_achievements_only", true),
                    ("entries", "map", true),
                ],
            )?;
            entries(
                &value["entries"],
                &[
                    ("name", "text", true),
                    ("points", "uint", true),
                    ("completed", "bool", true),
                    ("earned_by_character", "bool", true),
                    ("account_wide", "bool", true),
                    ("criteria", "map", true),
                ],
            )?;
            let achievements = value["entries"].as_object().unwrap();
            ensure!(achievements.len() <= 32, "Too many tracked achievements");
            for achievement in achievements.values() {
                ensure!(
                    achievement["name"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty() && s.len() <= 256),
                    "Invalid achievement name"
                );
                let criteria = &achievement["criteria"];
                entries(
                    criteria,
                    &[
                        ("completed", "bool", true),
                        ("criterion_id", "positive", false),
                        ("quantity", "uint", false),
                        ("required", "uint", false),
                    ],
                )?;
                let criteria = criteria.as_object().unwrap();
                ensure!(criteria.len() <= 128, "Too many achievement criteria");
                for (key, criterion) in criteria {
                    index(key, 1, criteria.len() as u32)?;
                    ensure!(
                        criterion["quantity"].is_null() == criterion["required"].is_null(),
                        "Incomplete criterion quantities"
                    );
                }
            }
        }
        "talents" => {
            object(
                value,
                &[
                    ("config_id", "positive", true),
                    ("name", "text", false),
                    ("nodes", "map", true),
                    ("has_unspent", "bool", false),
                    ("unspent_class", "uint", false),
                    ("unspent_spec", "uint", false),
                    ("loadout_export", "map", false),
                    ("saved_loadouts", "map", false),
                ],
            )?;
            entries(
                &value["nodes"],
                &[
                    ("entry_id", "positive", true),
                    ("rank", "positive", true),
                    ("tree_id", "positive", true),
                ],
            )?;
            if let Some(catalog) = value.get("saved_loadouts") {
                object(
                    catalog,
                    &[
                        ("spec_id", "positive", true),
                        ("selected_config_id", "positive", false),
                        ("entries", "map", true),
                    ],
                )?;
                let loadouts = catalog["entries"].as_object().unwrap();
                ensure!(loadouts.len() <= 128, "Too many saved loadouts");
                for (id, loadout) in loadouts {
                    index(id, 1, u32::MAX)?;
                    object(
                        loadout,
                        &[
                            ("name", "text", true),
                            ("import_string", "text", true),
                            ("serialization_version", "uint", true),
                            ("uses_shared_action_bars", "bool", true),
                        ],
                    )?;
                    let text = loadout["import_string"].as_str().unwrap();
                    ensure!(
                        !text.is_empty()
                            && text
                                .bytes()
                                .all(|c| c.is_ascii_alphanumeric() || b"+/=".contains(&c)),
                        "Invalid saved loadout string"
                    );
                }
            }
            if let Some(export) = value.get("loadout_export") {
                if export["status"] == "available" {
                    object(
                        export,
                        &[
                            ("status", "available", true),
                            ("format", "blizzard_traits", true),
                            ("import_string", "text", true),
                            ("spec_id", "positive", true),
                            ("serialization_version", "uint", true),
                            ("staged_changes", "bool", true),
                        ],
                    )?;
                    let text = export["import_string"].as_str().unwrap();
                    ensure!(
                        !text.is_empty()
                            && text
                                .bytes()
                                .all(|c| c.is_ascii_alphanumeric() || b"+/=".contains(&c)),
                        "Invalid loadout alphabet"
                    );
                    ensure!(
                        export["staged_changes"] == false,
                        "Staged loadout cannot be exported as committed"
                    );
                } else {
                    object(
                        export,
                        &[("status", "unavailable", true), ("reason", "text", true)],
                    )?;
                    ensure!(
                        matches!(
                            export["reason"].as_str(),
                            Some(
                                "client_not_supported"
                                    | "api_unavailable"
                                    | "staged_changes"
                                    | "restricted_or_failed"
                            )
                        ),
                        "Unknown loadout refusal"
                    );
                }
            }
        }
        _ => anyhow::bail!("Unknown section"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rejects_ambiguous_or_malformed_inventory() {
        for value in [
            json!([]),
            json!({"01":{"item_id":1}}),
            json!({"20":{"item_id":1}}),
            json!({"1":{"item_id":0}}),
            json!({"1":{"item_id":1,"extra":true}}),
        ] {
            assert!(validate("equipment", &value).is_err());
        }
        for value in [
            json!({"0":[]}),
            json!({"0":{"1":{"item_id":1}}}),
            json!({"0":{"1":{"item_id":1,"count":-1}}}),
        ] {
            assert!(validate("bags", &value).is_err());
        }
        validate("equipment", &json!({"16":{"item_id":123,"link":null}})).unwrap();
        validate("bags", &json!({"0":{"1":{"item_id":123,"count":2}}})).unwrap();
    }
}
