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
    required_level: Option<u32>,
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
            item.required_level.is_none() || item.link.is_some_and(|link| !link.is_empty()),
            "Required level needs an observed item link"
        );
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
        "pet_collection_view" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("displayed_rows", "uint", true),
                    ("entries", "map", true),
                ],
            )?;
            ensure!(
                value["coverage"] == "filtered_journal_species"
                    && value["displayed_rows"].as_u64().unwrap() <= 10000,
                "Invalid pet journal coverage"
            );
            entries(
                &value["entries"],
                &[
                    ("name", "text", true),
                    ("owned_rows", "uint", true),
                    ("unowned_rows", "uint", true),
                ],
            )?;
            let mut count = 0u64;
            for row in value["entries"].as_object().unwrap().values() {
                let total =
                    row["owned_rows"].as_u64().unwrap() + row["unowned_rows"].as_u64().unwrap();
                ensure!(
                    total > 0
                        && total <= 10000
                        && row["name"]
                            .as_str()
                            .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                    "Invalid pet journal species"
                );
                count += total;
            }
            ensure!(
                count == value["displayed_rows"].as_u64().unwrap(),
                "Inconsistent pet journal row count"
            );
        }
        "mail_attachments" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("loaded_messages", "uint", true),
                    ("total_messages", "uint", true),
                    ("messages", "map", true),
                ],
            )?;
            let loaded = value["loaded_messages"].as_u64().unwrap();
            let messages = value["messages"].as_object().unwrap();
            ensure!(
                value["coverage"] == "loaded_inbox_attachments"
                    && loaded <= 100
                    && value["total_messages"].as_u64().unwrap() >= loaded
                    && messages.len() as u64 == loaded,
                "Invalid inbox coverage"
            );
            for message in 1..=loaded {
                ensure!(messages.contains_key(&message.to_string()), "Sparse inbox");
            }
            for (message, attachments) in messages {
                index(message, 1, 100)?;
                let rows = attachments
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("Invalid inbox attachments"))?;
                ensure!(rows.len() <= 16, "Too many inbox attachments");
                for (slot, row) in rows {
                    index(slot, 1, 16)?;
                    match row["kind"].as_str() {
                        Some("item") => object(
                            row,
                            &[
                                ("kind", "text", true),
                                ("item_id", "positive", true),
                                ("count", "positive", true),
                            ],
                        )?,
                        Some("currency") => object(
                            row,
                            &[
                                ("kind", "text", true),
                                ("currency_id", "positive", true),
                                ("count", "positive", true),
                            ],
                        )?,
                        Some("unknown") => object(row, &[("kind", "text", true)])?,
                        _ => anyhow::bail!("Invalid inbox attachment kind"),
                    }
                }
            }
        }
        "profession_recipe_view" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("profession_id", "positive", true),
                    ("entries", "map", true),
                ],
            )?;
            ensure!(
                value["coverage"] == "filtered_local_recipe_list",
                "Invalid profession recipe coverage"
            );
            entries(
                &value["entries"],
                &[("name", "text", false), ("learned", "bool", false)],
            )?;
            let rows = value["entries"].as_object().unwrap();
            ensure!(
                rows.len() <= 4096
                    && rows.values().all(|row| row["name"]
                        .as_str()
                        .is_none_or(|name| !name.is_empty() && name.len() <= 256)),
                "Invalid profession recipe rows"
            );
        }
        "saved_world_bosses" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("update_age_seconds", "uint", true),
                    ("entries", "map", true),
                ],
            )?;
            let entries = value["entries"].as_object().unwrap();
            ensure!(
                value["coverage"] == "updated_cache" && entries.len() <= 128,
                "Invalid saved world boss coverage"
            );
            for slot in 1..=entries.len() {
                ensure!(
                    entries.contains_key(&slot.to_string()),
                    "Sparse saved world bosses"
                );
            }
            for (slot, row) in entries {
                index(slot, 1, 128)?;
                object(
                    row,
                    &[("name", "text", true), ("reset_seconds", "int", true)],
                )?;
                ensure!(
                    row["name"]
                        .as_str()
                        .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                    "Invalid saved world boss name"
                );
            }
        }
        "saved_instances" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("update_age_seconds", "uint", true),
                    ("entries", "map", true),
                ],
            )?;
            let entries = value["entries"].as_object().unwrap();
            ensure!(
                value["coverage"] == "updated_cache" && entries.len() <= 128,
                "Invalid saved instance coverage"
            );
            for slot in 1..=entries.len() {
                ensure!(
                    entries.contains_key(&slot.to_string()),
                    "Sparse saved instances"
                );
            }
            for (slot, row) in entries {
                index(slot, 1, 128)?;
                object(
                    row,
                    &[
                        ("name", "text", true),
                        ("reset_seconds", "int", true),
                        ("difficulty_id", "uint", true),
                        ("locked", "bool", true),
                        ("extended", "bool", true),
                        ("is_raid", "bool", true),
                        ("max_players", "uint", true),
                        ("difficulty_name", "text", false),
                    ],
                )?;
                for field in ["name", "difficulty_name"] {
                    ensure!(
                        row[field].is_null()
                            || row[field]
                                .as_str()
                                .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                        "Invalid saved instance name"
                    );
                }
            }
        }
        "toy_collection_view" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("displayed_count", "uint", true),
                    ("entries", "map", true),
                ],
            )?;
            let entries = value["entries"].as_object().unwrap();
            ensure!(
                value["coverage"] == "filtered_journal"
                    && entries.len() <= 4096
                    && value["displayed_count"].as_u64() == Some(entries.len() as u64),
                "Invalid toy view coverage"
            );
            for (id, entry) in entries {
                index(id, 1, u32::MAX)?;
                object(
                    entry,
                    &[
                        ("name", "text", true),
                        ("collected", "bool", true),
                        ("favorite", "bool", true),
                    ],
                )?;
                ensure!(
                    entry["name"]
                        .as_str()
                        .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                    "Invalid toy name"
                );
            }
        }
        "mount_collection" => {
            object(
                value,
                &[
                    ("catalogue_count", "positive", true),
                    ("entries", "map", true),
                ],
            )?;
            let count = value["catalogue_count"].as_u64().unwrap();
            let entries = value["entries"].as_object().unwrap();
            ensure!(
                count <= 4096 && entries.len() as u64 <= count,
                "Invalid mount catalogue size"
            );
            for (id, entry) in entries {
                index(id, 1, u32::MAX)?;
                object(
                    entry,
                    &[
                        ("name", "text", true),
                        ("spell_id", "positive", true),
                        ("usable", "bool", true),
                        ("favorite", "bool", true),
                    ],
                )?;
                ensure!(
                    entry["name"]
                        .as_str()
                        .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                    "Invalid mount name"
                );
            }
        }
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
                    ("map_min_level", "positive", false),
                    ("map_max_level", "positive", false),
                    ("new_mail_indicator", "bool", false),
                    ("hearthstone_bind_name", "text", false),
                    ("hearthstone_item_id", "positive", false),
                    ("hearthstone_cooldown_seconds", "uint", false),
                    ("client_version", "text", true),
                ],
            )?;
            let map_id = value["map_id"].as_u64();
            ensure!(
                value
                    .get("hearthstone_item_id")
                    .is_none_or(|id| !id.is_null())
                    && value
                        .get("hearthstone_cooldown_seconds")
                        .is_none_or(|seconds| {
                            !seconds.is_null() && value["hearthstone_item_id"].as_u64().is_some()
                        }),
                "Invalid Hearthstone cooldown observation"
            );
            let levels = (
                value["map_min_level"].as_u64(),
                value["map_max_level"].as_u64(),
            );
            ensure!(
                (value.get("map_min_level").is_none() && value.get("map_max_level").is_none())
                    || matches!(levels, (Some(min), Some(max)) if map_id.is_some() && min <= max && max <= 255),
                "Invalid captured map level range"
            );
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
        "wallet" | "repair_quote" => object(value, &[("copper", "wallet_copper", true)])?,
        "skill_lines" => {
            entries(
                value,
                &[
                    ("name", "text", true),
                    ("rank", "uint", true),
                    ("max_rank", "uint", true),
                    ("category_id", "positive", false),
                    ("modifier", "int", false),
                    ("temporary_points", "int", false),
                ],
            )?;
            let rows = value.as_object().unwrap();
            ensure!(rows.len() <= 512, "Too many skill lines");
            for row in rows.values() {
                ensure!(
                    row["name"]
                        .as_str()
                        .is_some_and(|name| !name.is_empty() && name.len() <= 256)
                        && row["rank"].as_u64() <= row["max_rank"].as_u64()
                        && ["category_id", "modifier", "temporary_points"]
                            .iter()
                            .all(|field| row.get(*field).is_none_or(|value| !value.is_null())),
                    "Invalid skill line"
                );
            }
        }
        "pet_care" => {
            object(
                value,
                &[
                    ("present", "bool", true),
                    ("family_id", "positive", false),
                    ("level", "positive", false),
                    ("training_points", "map", false),
                    ("happiness", "uint", false),
                    ("loyalty", "text", false),
                    ("food_types", "map", false),
                    ("carried_food", "map", false),
                ],
            )?;
            let present = value["present"] == true;
            if let Some(points) = value.get("training_points") {
                ensure!(present, "Pet training points require a present pet");
                object(points, &[("total", "int", true), ("used", "int", true)])?;
            }
            for field in ["family_id", "level"] {
                ensure!(
                    value.get(field).is_none_or(|v| present && !v.is_null()),
                    "Invalid active pet details"
                );
            }
            ensure!(
                value
                    .get("happiness")
                    .is_none_or(|h| present && h.as_u64().is_some_and(|h| h <= 255))
                    && value.get("loyalty").is_none_or(|name| present
                        && name
                            .as_str()
                            .is_some_and(|name| !name.is_empty() && name.len() <= 256)),
                "Invalid pet care data"
            );
            if let Some(foods) = value.get("food_types") {
                let foods = foods
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("Invalid pet food types"))?;
                ensure!(present && foods.len() <= 16, "Invalid pet food types");
                for (key, name) in foods {
                    index(key, 1, foods.len() as u32)?;
                    ensure!(
                        name.as_str()
                            .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                        "Invalid pet food type"
                    );
                }
            }
            if let Some(foods) = value.get("carried_food") {
                let foods = foods
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("Invalid carried pet food"))?;
                ensure!(present && foods.len() <= 256, "Invalid carried pet food");
                for (key, count) in foods {
                    index(key, 1, u32::MAX)?;
                    ensure!(
                        count
                            .as_u64()
                            .is_some_and(|count| (1..=u64::from(u32::MAX)).contains(&count)),
                        "Invalid carried pet food quantity"
                    );
                }
            }
        }
        "trainer_services" => {
            object(
                value,
                &[
                    ("kind", "text", true),
                    ("entries", "map", true),
                    ("pet_training_points_available", "uint", false),
                    ("pet_level", "positive", false),
                    ("primary_profession_slots_full", "bool", false),
                ],
            )?;
            let pet = value["kind"] == "pet";
            ensure!(
                value
                    .get("primary_profession_slots_full")
                    .is_none_or(|v| !pet && !v.is_null()),
                "Invalid trainer profession slots"
            );
            ensure!(
                matches!(
                    value["kind"].as_str(),
                    Some("general" | "tradeskills" | "pet")
                ) && value
                    .get("pet_training_points_available")
                    .is_none_or(|points| pet && !points.is_null())
                    && value.get("pet_level").is_none_or(|level| {
                        pet && level
                            .as_u64()
                            .is_some_and(|level| (1..=255).contains(&level))
                    }),
                "Invalid trainer kind"
            );
            let rows = value["entries"].as_object().unwrap();
            ensure!(
                (1..=512).contains(&rows.len()),
                "Invalid trainer service count"
            );
            for (key, row) in rows {
                index(key, 1, rows.len() as u32)?;
                object(
                    row,
                    &[
                        ("name", "text", true),
                        ("status", "text", true),
                        ("required_level", "positive", false),
                        ("rank_label", "text", false),
                        ("is_profession", "bool", false),
                        ("skill_requirement", "map", false),
                        ("ability_requirements", "map", false),
                        ("price_copper", "wallet_copper", false),
                        ("price_training_points", "uint", false),
                    ],
                )?;
                ensure!(
                    row.get("is_profession")
                        .is_none_or(|v| !pet && !v.is_null() && row["status"] != "used"),
                    "Invalid trainer profession flag"
                );
                ensure!(
                    matches!(
                        row["status"].as_str(),
                        Some("available" | "unavailable" | "used")
                    ) && row["name"]
                        .as_str()
                        .is_some_and(|name| !name.is_empty() && name.len() <= 256)
                        && row.get("rank_label").is_none_or(|rank| rank
                            .as_str()
                            .is_some_and(|rank| !rank.is_empty() && rank.len() <= 256))
                        && row
                            .get("required_level")
                            .is_none_or(|level| level.as_u64().is_some_and(|level| level <= 255))
                        && row.get("price_copper").is_none_or(|price| !pet
                            && !price.is_null()
                            && row["status"] != "used")
                        && row
                            .get("price_training_points")
                            .is_none_or(|price| pet && !price.is_null() && row["status"] != "used"),
                    "Invalid trainer service"
                );
                if let Some(skill) = row.get("skill_requirement") {
                    object(
                        skill,
                        &[
                            ("name", "text", true),
                            ("required_rank", "uint", true),
                            ("met", "bool", true),
                        ],
                    )?;
                    ensure!(
                        row["status"] != "used"
                            && skill["name"]
                                .as_str()
                                .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                        "Invalid trainer skill requirement"
                    );
                }
                if let Some(abilities) = row.get("ability_requirements") {
                    let abilities = abilities
                        .as_object()
                        .ok_or_else(|| anyhow::anyhow!("Invalid trainer ability requirements"))?;
                    ensure!(
                        row["status"] != "used" && abilities.len() <= 16,
                        "Invalid trainer ability requirements"
                    );
                    for (key, ability) in abilities {
                        index(key, 1, abilities.len() as u32)?;
                        object(ability, &[("name", "text", true), ("met", "bool", true)])?;
                        ensure!(
                            ability["name"]
                                .as_str()
                                .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                            "Invalid trainer ability name"
                        );
                    }
                }
            }
        }
        "merchant_stock" => {
            let rows = value
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Expected merchant stock map"))?;
            ensure!(
                (1..=256).contains(&rows.len()),
                "Invalid merchant stock count"
            );
            for (key, row) in rows {
                index(key, 1, rows.len() as u32)?;
                object(
                    row,
                    &[
                        ("item_id", "positive", false),
                        ("currency_id", "positive", false),
                        ("price_copper", "wallet_copper", true),
                        ("bundle_size", "positive", true),
                        ("available", "int", true),
                        ("extended_cost", "bool", true),
                        ("cost_components", "map", false),
                    ],
                )?;
                ensure!(
                    (row.get("item_id").is_some() ^ row.get("currency_id").is_some())
                        && row["item_id"]
                            .as_u64()
                            .or_else(|| row["currency_id"].as_u64())
                            .is_some()
                        && row["available"].as_i64().is_some_and(|stock| stock >= -1),
                    "Invalid merchant offer"
                );
                if let Some(costs) = row.get("cost_components") {
                    let costs = costs
                        .as_object()
                        .ok_or_else(|| anyhow::anyhow!("Invalid merchant costs"))?;
                    ensure!(
                        row["extended_cost"].as_bool() == Some(true)
                            && (1..=16).contains(&costs.len()),
                        "Invalid merchant costs"
                    );
                    for (key, cost) in costs {
                        index(key, 1, costs.len() as u32)?;
                        object(
                            cost,
                            &[
                                ("quantity", "positive", true),
                                ("item_id", "positive", false),
                                ("currency_name", "text", false),
                            ],
                        )?;
                        ensure!(
                            (cost.get("item_id").is_some() ^ cost.get("currency_name").is_some())
                                && (cost["item_id"].as_u64().is_some()
                                    || cost["currency_name"]
                                        .as_str()
                                        .is_some_and(|name| !name.is_empty() && name.len() <= 256)),
                            "Invalid merchant cost component"
                        );
                    }
                }
            }
        }
        "instance_context" => match value["kind"].as_str() {
            Some("none") => object(value, &[("kind", "none", true)])?,
            Some("party" | "raid" | "pvp" | "arena" | "scenario") => {
                object(
                    value,
                    &[
                        ("kind", "text", true),
                        ("name", "text", true),
                        ("difficulty_id", "uint", true),
                        ("instance_map_id", "positive", false),
                        ("lock_warning", "map", false),
                    ],
                )?;
                ensure!(
                    value["name"]
                        .as_str()
                        .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                    "Invalid instance name"
                );
                ensure!(
                    value.get("instance_map_id").is_none_or(|id| !id.is_null()),
                    "Invalid instance map ID"
                );
                if let Some(warning) = value.get("lock_warning") {
                    ensure!(
                        matches!(value["kind"].as_str(), Some("party" | "raid")),
                        "Unexpected instance lock warning"
                    );
                    object(
                        warning,
                        &[
                            ("remaining_seconds", "uint", true),
                            ("encounters_total", "positive", true),
                            ("encounters_completed", "uint", true),
                            ("encounters", "map", false),
                        ],
                    )?;
                    ensure!(
                        warning["encounters_total"]
                            .as_u64()
                            .zip(warning["encounters_completed"].as_u64())
                            .is_some_and(|(total, completed)| total <= 256 && completed <= total),
                        "Invalid instance lock progress"
                    );
                    if let Some(rows) = warning.get("encounters") {
                        let total = warning["encounters_total"].as_u64().unwrap() as u32;
                        let completed = warning["encounters_completed"].as_u64().unwrap();
                        let rows = rows
                            .as_object()
                            .ok_or_else(|| anyhow::anyhow!("Invalid lock encounter rows"))?;
                        ensure!(
                            completed > 0 && rows.len() == total as usize,
                            "Incomplete lock encounter rows"
                        );
                        let mut killed = 0;
                        for (key, row) in rows {
                            index(key, 1, total)?;
                            object(row, &[("name", "text", true), ("killed", "bool", true)])?;
                            ensure!(
                                row["name"]
                                    .as_str()
                                    .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                                "Invalid lock encounter name"
                            );
                            killed += u64::from(row["killed"].as_bool() == Some(true));
                        }
                        ensure!(killed == completed, "Lock encounter count mismatch");
                    }
                }
            }
            _ => anyhow::bail!("Invalid instance type"),
        },
        "bag_capacity" => {
            let bags = value
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Expected bag capacity map"))?;
            ensure!((5..=6).contains(&bags.len()), "Invalid carried bag range");
            for (key, bag) in bags {
                index(key, 0, (bags.len() - 1) as u32)?;
                object(
                    bag,
                    &[
                        ("total_slots", "uint", true),
                        ("free_slots", "uint", true),
                        ("family_mask", "uint", false),
                    ],
                )?;
                let total = bag["total_slots"].as_u64().unwrap();
                let free = bag["free_slots"].as_u64().unwrap();
                ensure!(
                    total <= 1024
                        && free <= total
                        && (key != "0" || total > 0)
                        && bag
                            .get("family_mask")
                            .is_none_or(|mask| total > 0 && !mask.is_null()),
                    "Invalid bag capacity"
                );
            }
        }
        "durability_alerts" => {
            let alerts = value
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Expected durability alert map"))?;
            for (slot, alert) in alerts {
                ensure!(
                    matches!(
                        slot.as_str(),
                        "head"
                            | "shoulders"
                            | "chest"
                            | "waist"
                            | "legs"
                            | "feet"
                            | "wrists"
                            | "hands"
                            | "main_hand"
                            | "off_hand"
                            | "ranged"
                    ) && matches!(alert.as_str(), Some("low" | "broken")),
                    "Invalid durability alert"
                );
            }
        }
        "flight_map" => {
            object(
                value,
                &[("map_id", "positive", true), ("entries", "map", true)],
            )?;
            entries(
                &value["entries"],
                &[
                    ("node_id", "positive", true),
                    ("name", "text", true),
                    ("state", "text", true),
                    ("map_transition", "bool", true),
                    ("fare_copper", "wallet_copper", false),
                ],
            )?;
            let rows = value["entries"].as_object().unwrap();
            ensure!(rows.len() <= 512, "Too many flight map observations");
            for (key, row) in rows {
                index(key, 1, 4096)?;
                ensure!(
                    row["name"]
                        .as_str()
                        .is_some_and(|name| !name.is_empty() && name.len() <= 256)
                        && matches!(
                            row["state"].as_str(),
                            Some("current" | "reachable" | "unreachable")
                        )
                        && (row["fare_copper"].is_null()
                            || (row["state"] == "reachable" && row["map_transition"] == false)),
                    "Invalid flight destination"
                );
            }
        }
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
                    ("level_learned", "uint", false),
                ],
            )?;
            let rows = value["entries"].as_object().unwrap();
            ensure!(rows.len() <= 1024, "Too many spellbook observations");
            for (key, row) in rows {
                index(key, 1, 4096)?;
                ensure!(
                    row.get("level_learned")
                        .is_none_or(|level| !level.is_null()),
                    "Invalid spell learned-level observation"
                );
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
                    ("resting", "bool", false),
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
                        && value["rested_xp"].is_null()
                        && value["resting"].is_null(),
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
