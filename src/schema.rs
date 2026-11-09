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
    sell_price_copper: Option<u64>,
    equip_location: Option<&'a str>,
    native_stats: Option<std::collections::BTreeMap<String, f64>>,
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
        if value.get("sell_price_copper").is_some() {
            ensure!(
                item.link.is_some_and(|link| !link.is_empty())
                    && item
                        .sell_price_copper
                        .is_some_and(|price| price <= 9_999_999_999_999),
                "Invalid linked item sell price"
            );
        }
        if value.get("equip_location").is_some() {
            let location = item
                .equip_location
                .ok_or_else(|| anyhow::anyhow!("Invalid native equip location"))?;
            ensure!(
                item.link.is_some_and(|link| !link.is_empty())
                    && location.len() <= 64
                    && (location.is_empty()
                        || location.strip_prefix("INVTYPE_").is_some_and(|token| !token
                            .is_empty()
                            && token.bytes().all(|b| b.is_ascii_uppercase()
                                || b.is_ascii_digit()
                                || b == b'_'))),
                "Invalid native equip location"
            );
        }
        if value.get("native_stats").is_some() {
            let stats = item
                .native_stats
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Invalid native item stats"))?;
            ensure!(
                item.link.is_some_and(|link| !link.is_empty()) && stats.len() <= 128,
                "Invalid native item stats"
            );
            for (key, number) in stats {
                let token = key
                    .strip_prefix("ITEM_MOD_")
                    .and_then(|s| s.strip_suffix("_SHORT"));
                let socket = key.strip_prefix("EMPTY_SOCKET_");
                let resistance = key
                    .strip_prefix("RESISTANCE")
                    .and_then(|s| s.strip_suffix("_NAME"));
                let valid = token.is_some_and(|s| {
                    !s.is_empty()
                        && s.bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                }) || socket.is_some_and(|s| {
                    !s.is_empty() && s.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
                }) || resistance
                    .is_some_and(|s| s.len() == 1 && matches!(s.as_bytes()[0], b'0'..=b'6'));
                ensure!(
                    key.len() <= 96
                        && valid
                        && number.is_finite()
                        && (-4294967295.0..=4294967295.0).contains(number),
                    "Invalid native item stat"
                );
            }
        }
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
            "unit_interval" => v
                .as_f64()
                .is_some_and(|n| n.is_finite() && (0.0..=1.0).contains(&n)),
            "percent" => v
                .as_f64()
                .is_some_and(|n| n.is_finite() && (0.0..=100.0).contains(&n)),
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

fn reference_locations(source: &Value, cache: &Value) -> Result<()> {
    object(
        cache,
        &[
            ("coordinate_system", "text", true),
            ("reported_npc_count", "uint", true),
            ("reported_object_count", "uint", true),
            ("npcs", "map", true),
            ("objects", "map", true),
        ],
    )?;
    ensure!(
        cache["coordinate_system"] == "questiedb_area_percent",
        "Invalid reference coordinate system"
    );
    for (kind, count_field, fields) in [
        ("npcs", "reported_npc_count", &["npc_drops", "vendors"][..]),
        ("objects", "reported_object_count", &["object_drops"][..]),
    ] {
        let mut ids = std::collections::BTreeSet::<u32>::new();
        for row in source["entries"]
            .as_object()
            .unwrap()
            .values()
            .filter(|row| row["status"] == "reported")
        {
            for field in fields {
                if let Some(refs) = row[*field]["entries"].as_object() {
                    for id in refs.keys() {
                        ids.insert(id.parse()?);
                    }
                }
            }
        }
        let rows = cache[kind].as_object().unwrap();
        let expected: std::collections::BTreeSet<_> =
            ids.iter().take(32).map(u32::to_string).collect();
        ensure!(
            cache[count_field].as_u64() == Some(ids.len() as u64)
                && rows
                    .keys()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>()
                    == expected,
            "Invalid retained reference entity coverage"
        );
        for row in rows.values() {
            if row["status"] != "reported" {
                object(row, &[("status", "text", true)])?;
                ensure!(
                    matches!(
                        row["status"].as_str(),
                        Some("not_in_provider" | "unavailable")
                    ),
                    "Invalid entity reference status"
                );
                continue;
            }
            object(row, &[("status", "text", true), ("spawns", "map", false)])?;
            let Some(spawns) = row.get("spawns") else {
                continue;
            };
            object(
                spawns,
                &[
                    ("reported_point_count", "uint", true),
                    ("truncated", "bool", true),
                    ("entries", "map", true),
                ],
            )?;
            let count = spawns["reported_point_count"].as_u64().unwrap();
            let points = spawns["entries"].as_object().unwrap();
            ensure!(
                count <= 4096
                    && points.len() == count.min(16) as usize
                    && spawns["truncated"].as_bool() == Some(count > 16),
                "Invalid reference point coverage"
            );
            for (slot, point) in points {
                index(slot, 1, points.len() as u32)?;
                match point["kind"].as_str() {
                    Some("coordinate") => {
                        object(
                            point,
                            &[
                                ("kind", "text", true),
                                ("area_id", "positive", true),
                                ("x_percent", "percent", true),
                                ("y_percent", "percent", true),
                                ("provider_phase_id", "positive", false),
                            ],
                        )?;
                        ensure!(
                            point.get("provider_phase_id").is_none_or(|n| n
                                .as_u64()
                                .is_some_and(|n| (1..=u64::from(u32::MAX)).contains(&n))),
                            "Invalid reference phase ID"
                        );
                    }
                    Some("instance_presence") => object(
                        point,
                        &[("kind", "text", true), ("area_id", "positive", true)],
                    )?,
                    _ => anyhow::bail!("Invalid reference point kind"),
                }
            }
        }
    }
    Ok(())
}

pub fn bag_item_cooldowns(value: &Value, bags: Option<&Value>) -> Result<()> {
    object(
        value,
        &[
            ("coverage", "text", true),
            ("captured_item_locations", "uint", true),
            ("entries", "map", true),
        ],
    )?;
    ensure!(
        value["coverage"] == "captured_carried_items_only",
        "Invalid bag cooldown coverage"
    );
    let expected = value["captured_item_locations"]
        .as_u64()
        .filter(|n| *n <= 4096)
        .ok_or_else(|| anyhow::anyhow!("Invalid bag cooldown count"))?;
    let source = bags
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Source bags unavailable"))?;
    let entries = value["entries"].as_object().unwrap();
    ensure!(
        entries.len() <= 65 && entries.keys().eq(source.keys()),
        "Bag cooldown source mismatch"
    );
    let mut count = 0_u64;
    for (bag, rows) in entries {
        index(bag, 0, 64)?;
        let rows = rows
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid bag cooldown slots"))?;
        let captured = source[bag]
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid source bag"))?;
        ensure!(
            rows.keys().eq(captured.keys()),
            "Bag cooldown slot mismatch"
        );
        for (slot, row) in rows {
            count += 1;
            ensure!(count <= 4096, "Bag cooldown budget exceeded");
            index(slot, 1, 1024)?;
            match row["status"].as_str() {
                Some("reported") => object(
                    row,
                    &[
                        ("item_id", "positive", true),
                        ("status", "text", true),
                        ("enabled", "bool", true),
                        ("remaining_seconds", "uint", true),
                    ],
                )?,
                Some("unavailable") => object(
                    row,
                    &[("item_id", "positive", true), ("status", "text", true)],
                )?,
                _ => anyhow::bail!("Invalid bag cooldown status"),
            }
            ensure!(
                row["item_id"]
                    .as_u64()
                    .is_some_and(|id| id <= u64::from(u32::MAX))
                    && row["item_id"] == captured[slot]["item_id"]
                    && row
                        .get("remaining_seconds")
                        .is_none_or(|n| n.as_u64().is_some_and(|n| n <= u64::from(u32::MAX))),
                "Invalid bag cooldown item or timer"
            );
        }
    }
    ensure!(count == expected, "Bag cooldown count mismatch");
    Ok(())
}

pub fn has_talent_definition_observation(value: &Value) -> bool {
    [
        "observed_tree_ids",
        "observed_node_count",
        "staged_changes_observed",
        "definition_allocation",
    ]
    .iter()
    .any(|key| value.get(*key).is_some())
        || value["nodes"].as_object().is_some_and(|nodes| {
            nodes.values().any(|node| {
                ["definition_id", "purchased_rank", "max_rank"]
                    .iter()
                    .any(|key| node.get(*key).is_some())
            })
        })
}

fn talent_definition_observation(value: &Value) -> Result<()> {
    if !has_talent_definition_observation(value) {
        return Ok(());
    }
    let trees = value["observed_tree_ids"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Missing observed talent trees"))?;
    ensure!(
        (1..=16).contains(&trees.len()),
        "Invalid observed talent tree count"
    );
    for (tree, observed) in trees {
        index(tree, 1, u32::MAX)?;
        ensure!(observed == true, "Invalid observed talent tree");
    }
    let nodes = value["nodes"].as_object().unwrap();
    ensure!(
        value["observed_node_count"]
            .as_u64()
            .is_some_and(|count| count <= 4096 && count >= nodes.len() as u64),
        "Invalid observed talent node count"
    );
    if let Some(staged) = value.get("staged_changes_observed") {
        ensure!(staged.is_boolean(), "Invalid observed staged changes");
    }
    if let Some(allocation) = value.get("definition_allocation") {
        object(
            allocation,
            &[
                ("coverage", "text", true),
                ("queried_node_count", "uint", true),
                ("queried_entry_count", "uint", true),
                ("definitions", "map", true),
            ],
        )?;
        let definitions = allocation["definitions"].as_object().unwrap();
        ensure!(
            allocation["coverage"] == "queried_config_tree_entries"
                && allocation["queried_node_count"] == value["observed_node_count"]
                && allocation["queried_entry_count"].as_u64() == Some(definitions.len() as u64)
                && definitions.len() <= 4096,
            "Invalid definition allocation coverage"
        );
        let mut entries = std::collections::BTreeSet::new();
        let mut node_trees = std::collections::BTreeMap::new();
        let mut node_entries = std::collections::BTreeMap::<u64, u32>::new();
        let mut locations = std::collections::BTreeMap::new();
        for (definition, row) in definitions {
            index(definition, 1, u32::MAX)?;
            object(
                row,
                &[
                    ("node_id", "positive", true),
                    ("entry_id", "positive", true),
                    ("tree_id", "positive", true),
                    ("purchased_rank", "uint", true),
                    ("max_rank", "positive", true),
                ],
            )?;
            let node = row["node_id"].as_u64().unwrap();
            let entry = row["entry_id"].as_u64().unwrap();
            let tree = row["tree_id"].as_u64().unwrap();
            let count = node_entries.entry(node).or_default();
            *count += 1;
            ensure!(*count <= 64, "Definition node entry budget exceeded");
            ensure!(
                trees.contains_key(&tree.to_string())
                    && entries.insert(entry)
                    && row["purchased_rank"].as_u64() <= row["max_rank"].as_u64(),
                "Invalid definition allocation row"
            );
            if let Some(previous) = node_trees.insert(node, tree) {
                ensure!(previous == tree, "Definition node tree mismatch");
            }
            locations.insert((node, entry), (definition, row));
            if row["purchased_rank"].as_u64().unwrap() > 0 {
                let active = nodes
                    .get(&node.to_string())
                    .ok_or_else(|| anyhow::anyhow!("Purchased definition has no active entry"))?;
                ensure!(
                    active["entry_id"].as_u64() == Some(entry)
                        && active["tree_id"].as_u64() == Some(tree),
                    "Purchased definition active entry mismatch"
                );
            }
        }
        ensure!(
            node_trees.len() as u64 <= value["observed_node_count"].as_u64().unwrap(),
            "Definition node budget mismatch"
        );
        for (node, active) in nodes {
            let (definition, row) = locations
                .get(&(node.parse::<u64>()?, active["entry_id"].as_u64().unwrap()))
                .ok_or_else(|| {
                    anyhow::anyhow!("Active entry missing from definition allocation")
                })?;
            ensure!(
                row["tree_id"] == active["tree_id"],
                "Active definition tree mismatch"
            );
            if active.get("definition_id").is_some() {
                ensure!(
                    active["definition_id"].as_u64() == Some(definition.parse::<u64>()?)
                        && active["purchased_rank"] == row["purchased_rank"]
                        && active["max_rank"] == row["max_rank"],
                    "Active definition metadata mismatch"
                );
            }
        }
    }
    for node in nodes.values() {
        ensure!(
            node["tree_id"]
                .as_u64()
                .is_some_and(|tree| trees.contains_key(&tree.to_string())),
            "Talent tree observation mismatch"
        );
        let fields = ["definition_id", "purchased_rank", "max_rank"];
        let count = fields
            .iter()
            .filter(|field| node.get(**field).is_some())
            .count();
        ensure!(
            count == 0 || count == fields.len(),
            "Incomplete talent definition observation"
        );
        if count > 0 {
            ensure!(
                node["definition_id"]
                    .as_u64()
                    .is_some_and(|id| id > 0 && id <= u64::from(u32::MAX))
                    && node["max_rank"].as_u64().is_some_and(|maximum| maximum > 0
                        && maximum <= u64::from(u32::MAX)
                        && node["purchased_rank"]
                            .as_u64()
                            .is_some_and(|rank| rank <= maximum)),
                "Invalid talent definition observation"
            );
        }
    }
    Ok(())
}

pub fn validate(name: &str, value: &Value) -> Result<()> {
    match name {
        "bag_item_cooldowns" => {} // Validated with its source bags by Snapshot::validate.
        "recipe_item_sources" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("provider", "text", true),
                    ("provider_mode", "text", true),
                    ("provider_contract_version", "positive", true),
                    ("profession_id", "positive", true),
                    ("reported_item_count", "uint", true),
                    ("entries", "map", true),
                    ("entity_locations", "map", false),
                ],
            )?;
            let count = value["reported_item_count"].as_u64().unwrap();
            let rows = value["entries"].as_object().unwrap();
            ensure!(
                value["coverage"] == "first_reported_fixed_recipe_reagents"
                    && value["provider"] == "questiedb"
                    && matches!(value["provider_mode"].as_str(), Some("source" | "baked"))
                    && count <= 16384
                    && rows.len() == count.min(64) as usize,
                "Invalid reference coverage"
            );
            for (id, row) in rows {
                index(id, 1, u32::MAX)?;
                if row["status"] != "reported" {
                    object(row, &[("status", "text", true)])?;
                    ensure!(
                        matches!(
                            row["status"].as_str(),
                            Some("not_in_provider" | "unavailable")
                        ),
                        "Invalid reference status"
                    );
                    continue;
                }
                object(
                    row,
                    &[
                        ("status", "text", true),
                        ("npc_drops", "map", false),
                        ("object_drops", "map", false),
                        ("container_drops", "map", false),
                        ("quest_rewards", "map", false),
                        ("vendors", "map", false),
                    ],
                )?;
                for field in [
                    "npc_drops",
                    "object_drops",
                    "container_drops",
                    "quest_rewards",
                    "vendors",
                ] {
                    let Some(reference) = row.get(field) else {
                        continue;
                    };
                    object(
                        reference,
                        &[
                            ("entries", "map", true),
                            ("distinct_id_count", "uint", true),
                            ("truncated", "bool", true),
                        ],
                    )?;
                    let entries = reference["entries"].as_object().unwrap();
                    let count = reference["distinct_id_count"].as_u64().unwrap();
                    ensure!(
                        count <= 4096
                            && entries.len() == count.min(8) as usize
                            && reference["truncated"].as_bool() == Some(count > 8),
                        "Invalid reference count"
                    );
                    for (id, flag) in entries {
                        index(id, 1, u32::MAX)?;
                        ensure!(*flag == true, "Invalid reference entry");
                    }
                }
            }
            if let Some(cache) = value.get("entity_locations") {
                reference_locations(value, cache)?;
            }
        }
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
        "profession_recipe_materials" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("profession_id", "positive", true),
                    ("entries", "map", true),
                ],
            )?;
            ensure!(
                value["coverage"] == "reported_known_spell_schematics",
                "Invalid recipe material coverage"
            );
            let rows = value["entries"].as_object().unwrap();
            ensure!(rows.len() <= 4096, "Too many recipe material rows");
            let mut retained = 0usize;
            for (id, row) in rows {
                index(id, 1, u32::MAX)?;
                if row["status"] != "fixed_items" {
                    object(row, &[("status", "text", true)])?;
                    ensure!(
                        matches!(
                            row["status"].as_str(),
                            Some("unavailable" | "unsupported_required_slots" | "budget_exhausted")
                        ),
                        "Invalid recipe material status"
                    );
                    continue;
                }
                object(
                    row,
                    &[
                        ("status", "text", true),
                        ("reagents", "map", true),
                        ("output_item_id", "positive", false),
                        ("base_output_quantity", "map", false),
                        ("reported_craftable_count", "uint", false),
                        ("cooldown", "map", false),
                    ],
                )?;
                let reagents = row["reagents"].as_object().unwrap();
                if let Some(cooldown) = row.get("cooldown") {
                    object(
                        cooldown,
                        &[
                            ("remaining_seconds", "uint", false),
                            ("daily", "bool", false),
                            ("charges", "uint", false),
                            ("max_charges", "positive", false),
                        ],
                    )?;
                    ensure!(
                        cooldown
                            .as_object()
                            .unwrap()
                            .values()
                            .all(|value| !value.is_null()),
                        "Invalid bulk recipe cooldown"
                    );
                    let seconds = cooldown["remaining_seconds"].as_u64();
                    let charges = cooldown["charges"].as_u64();
                    let maximum = cooldown["max_charges"].as_u64();
                    ensure!(
                        (seconds.is_some() || charges.is_some())
                            && (!cooldown.as_object().unwrap().contains_key("daily")
                                || seconds.is_some())
                            && charges.is_some() == maximum.is_some()
                            && charges <= maximum
                            && [seconds, charges, maximum]
                                .into_iter()
                                .flatten()
                                .all(|value| value <= u64::from(u32::MAX)),
                        "Invalid bulk recipe cooldown"
                    );
                }
                if let Some(count) = row.get("reported_craftable_count") {
                    ensure!(
                        count.as_u64().is_some_and(|n| n <= u64::from(u32::MAX)),
                        "Invalid reported recipe craft count"
                    );
                }
                ensure!(reagents.len() <= 64, "Too many recipe reagents");
                retained += reagents.len();
                ensure!(retained <= 16384, "Recipe material budget exceeded");
                for (item, quantity) in reagents {
                    index(item, 1, u32::MAX)?;
                    ensure!(
                        quantity
                            .as_u64()
                            .is_some_and(|n| (1..=u32::MAX as u64).contains(&n)),
                        "Invalid recipe material quantity"
                    );
                }
                if let Some(output) = row.get("output_item_id") {
                    ensure!(!output.is_null(), "Null recipe output");
                }
                if let Some(quantity) = row.get("base_output_quantity") {
                    object(
                        quantity,
                        &[("minimum", "positive", true), ("maximum", "positive", true)],
                    )?;
                    ensure!(
                        row["output_item_id"].as_u64().is_some()
                            && quantity["minimum"].as_u64() <= quantity["maximum"].as_u64(),
                        "Invalid recipe material output range"
                    );
                }
            }
        }
        "profession_spells" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("profession_id", "positive", true),
                    ("entries", "map", true),
                ],
            )?;
            ensure!(
                value["coverage"] == "reported_known_profession_spells",
                "Invalid profession spell coverage"
            );
            entries(&value["entries"], &[])?;
            ensure!(
                value["entries"].as_object().unwrap().len() <= 4096,
                "Too many profession spells"
            );
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
                &[
                    ("name", "text", false),
                    ("learned", "bool", false),
                    ("relative_difficulty", "text", false),
                    ("can_skill_up", "bool", false),
                    ("skill_ups_reported", "uint", false),
                ],
            )?;
            let rows = value["entries"].as_object().unwrap();
            ensure!(
                rows.len() <= 4096
                    && rows.values().all(|row| row["name"]
                        .as_str()
                        .is_none_or(|name| !name.is_empty() && name.len() <= 256)),
                "Invalid profession recipe rows"
            );
            for row in rows.values() {
                ensure!(
                    row.get("relative_difficulty").is_none_or(|v| matches!(
                        v.as_str(),
                        Some("optimal" | "medium" | "easy" | "trivial")
                    )) && row.get("can_skill_up").is_none_or(Value::is_boolean)
                        && row
                            .get("skill_ups_reported")
                            .is_none_or(|v| v.as_u64().is_some_and(|n| n <= u32::MAX as u64)),
                    "Invalid recipe skill-up observation"
                );
            }
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
        "group_context" => {
            object(
                value,
                &[("coverage", "text", true), ("categories", "map", true)],
            )?;
            ensure!(
                value["coverage"] == "reported_player_group_context",
                "Invalid group coverage"
            );
            let categories = value["categories"]
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Invalid group categories"))?;
            ensure!(
                (1..=2).contains(&categories.len()),
                "Invalid group category count"
            );
            for (category, row) in categories {
                ensure!(
                    matches!(category.as_str(), "home" | "instance"),
                    "Invalid group category"
                );
                object(
                    row,
                    &[
                        ("in_group", "bool", true),
                        ("in_raid", "bool", true),
                        ("reported_member_count", "uint", true),
                        ("player_is_leader", "bool", false),
                    ],
                )?;
                ensure!(
                    row["reported_member_count"]
                        .as_u64()
                        .is_some_and(|n| n <= u64::from(u32::MAX)),
                    "Invalid group member count"
                );
                if let Some(leader) = row.get("player_is_leader") {
                    ensure!(leader.is_boolean(), "Invalid group leader flag");
                }
            }
        }
        "player_pvp_progression" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("faction_id", "positive", true),
                    ("rank", "uint", true),
                    ("rank_points", "uint", true),
                    ("next_rank_threshold", "uint", true),
                    ("max_rank", "uint", false),
                    ("current_week_max_rank", "uint", false),
                    ("previous_week_max_rank", "uint", false),
                    ("total_points_by_rank", "map", false),
                ],
            )?;
            ensure!(
                value["coverage"] == "reported_pvp_rank_faction" && value["faction_id"] == 2800,
                "Invalid PvP progression source"
            );
            for key in [
                "rank",
                "rank_points",
                "next_rank_threshold",
                "max_rank",
                "current_week_max_rank",
                "previous_week_max_rank",
            ] {
                if let Some(number) = value.get(key) {
                    ensure!(
                        number.as_u64().is_some_and(|n| n <= u64::from(u32::MAX)),
                        "Invalid PvP progression value"
                    );
                }
            }
            if let Some(totals) = value.get("total_points_by_rank") {
                let totals = totals
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("Invalid PvP total points"))?;
                ensure!(
                    (1..=4).contains(&totals.len()),
                    "Invalid PvP threshold count"
                );
                for (rank, total) in totals {
                    let number = rank.parse::<u32>()?;
                    ensure!(
                        number.to_string() == *rank
                            && [
                                "rank",
                                "max_rank",
                                "current_week_max_rank",
                                "previous_week_max_rank"
                            ]
                            .iter()
                            .any(|key| value[*key].as_u64() == Some(u64::from(number))),
                        "Unreferenced PvP threshold rank"
                    );
                    ensure!(
                        total.as_u64().is_some_and(|n| n <= u64::from(u32::MAX)),
                        "Invalid PvP total points"
                    );
                }
            }
        }
        "player_trade_offer" => {
            object(
                value,
                &[("coverage", "text", true), ("entries", "map", true)],
            )?;
            ensure!(
                value["coverage"] == "visible_player_trade_normal_slots",
                "Invalid player trade coverage"
            );
            let rows = value["entries"].as_object().unwrap();
            ensure!(rows.len() == 6, "Incomplete player trade slots");
            for (slot, row) in rows {
                index(slot, 1, 6)?;
                match row["status"].as_str() {
                    Some("item") => {
                        object(
                            row,
                            &[
                                ("status", "text", true),
                                ("item_id", "positive", true),
                                ("quantity", "positive", true),
                            ],
                        )?;
                        ensure!(
                            ["item_id", "quantity"].iter().all(|key| row[*key]
                                .as_u64()
                                .is_some_and(|n| (1..=u64::from(u32::MAX)).contains(&n))),
                            "Invalid player trade item"
                        );
                    }
                    Some("unreadable_or_empty") => object(row, &[("status", "text", true)])?,
                    _ => anyhow::bail!("Invalid player trade slot"),
                }
            }
        }
        "loot_window" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("slot_count", "uint", true),
                    ("entries", "map", true),
                ],
            )?;
            ensure!(
                value["coverage"] == "visible_loot_window_slots",
                "Invalid loot window coverage"
            );
            let count = value["slot_count"]
                .as_u64()
                .filter(|n| *n <= 64)
                .ok_or_else(|| anyhow::anyhow!("Invalid loot slot count"))?;
            let entries = value["entries"].as_object().unwrap();
            ensure!(entries.len() as u64 == count, "Incomplete loot slots");
            for (slot, row) in entries {
                index(slot, 1, count as u32)?;
                match row["status"].as_str() {
                    Some("item") => {
                        object(
                            row,
                            &[
                                ("status", "text", true),
                                ("item_id", "uint", true),
                                ("quantity", "uint", true),
                            ],
                        )?;
                        ensure!(
                            row["item_id"]
                                .as_u64()
                                .is_some_and(|n| (1..=u64::from(u32::MAX)).contains(&n))
                                && row["quantity"]
                                    .as_u64()
                                    .is_some_and(|n| n <= u64::from(u32::MAX)),
                            "Invalid loot item"
                        );
                    }
                    Some("unreadable_or_non_item") => object(row, &[("status", "text", true)])?,
                    Some("money") => object(row, &[("status", "text", true)])?,
                    Some("reported_currency") => object(
                        row,
                        &[
                            ("status", "text", true),
                            ("reported_currency_id", "positive", true),
                            ("reported_quantity", "uint", true),
                        ],
                    )?,
                    _ => anyhow::bail!("Invalid loot slot status"),
                }
            }
        }
        "merchant_buyback" => {
            object(
                value,
                &[("coverage", "text", true), ("entries", "map", true)],
            )?;
            ensure!(
                value["coverage"] == "visible_merchant_buyback",
                "Invalid buyback coverage"
            );
            entries(
                &value["entries"],
                &[
                    ("item_id", "positive", true),
                    ("count", "positive", true),
                    ("buyback_price_copper", "wallet_copper", false),
                ],
            )?;
            let rows = value["entries"].as_object().unwrap();
            ensure!(rows.len() <= 12, "Too many buyback entries");
            for (key, row) in rows {
                index(key, 1, rows.len() as u32)?;
                ensure!(
                    row.get("buyback_price_copper")
                        .is_none_or(|v| v.as_u64().is_some()),
                    "Invalid buyback price"
                );
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
            let mut inventory_slots = std::collections::BTreeSet::new();
            for (key, bag) in bags {
                index(key, 0, (bags.len() - 1) as u32)?;
                object(
                    bag,
                    &[
                        ("total_slots", "uint", true),
                        ("free_slots", "uint", true),
                        ("family_mask", "uint", false),
                        ("equipped_item", "map", false),
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
                if let Some(item) = bag.get("equipped_item") {
                    object(
                        item,
                        &[
                            ("item_id", "positive", true),
                            ("inventory_slot", "positive", true),
                        ],
                    )?;
                    let slot = item["inventory_slot"].as_u64().unwrap();
                    ensure!(
                        key != "0" && total > 0 && slot <= 255 && inventory_slots.insert(slot),
                        "Invalid equipped bag identity"
                    );
                }
            }
        }
        "pet_action_bar" => {
            object(
                value,
                &[
                    ("coverage", "readable_pet_action_slots", true),
                    ("pet_present", "bool", true),
                    ("has_action_bar", "bool", true),
                    ("entries", "map", true),
                ],
            )?;
            ensure!(
                value["pet_present"] == true || value["has_action_bar"] == false,
                "Invalid pet action context"
            );
            let rows = value["entries"].as_object().unwrap();
            ensure!(
                value["has_action_bar"] == true || rows.is_empty(),
                "Unexpected pet action rows"
            );
            entries(
                &value["entries"],
                &[
                    ("kind", "text", true),
                    ("spell_id", "positive", false),
                    ("command_token", "text", false),
                    ("is_active", "bool", false),
                    ("autocast_allowed", "bool", false),
                    ("autocast_enabled", "bool", false),
                    ("range_check_performed", "bool", false),
                    ("in_range", "bool", false),
                ],
            )?;
            for (slot, row) in rows {
                index(slot, 1, 10)?;
                match row["kind"].as_str() {
                    Some("spell") => ensure!(
                        row["spell_id"]
                            .as_u64()
                            .is_some_and(|n| n > 0 && n <= u32::MAX as u64)
                            && row.get("command_token").is_none(),
                        "Invalid pet action spell"
                    ),
                    Some("command") => ensure!(
                        row.get("spell_id").is_none()
                            && row["command_token"]
                                .as_str()
                                .is_some_and(|token| token.len() <= 64
                                    && token.strip_prefix("PET_").is_some_and(|s| !s.is_empty()
                                        && s.bytes().all(|b| b.is_ascii_uppercase()
                                            || b.is_ascii_digit()
                                            || b == b'_'))),
                        "Invalid pet action command"
                    ),
                    _ => anyhow::bail!("Invalid pet action kind"),
                }
                for field in [
                    "is_active",
                    "autocast_allowed",
                    "autocast_enabled",
                    "range_check_performed",
                    "in_range",
                ] {
                    ensure!(
                        row.get(field).is_none_or(Value::is_boolean),
                        "Invalid pet action flag"
                    );
                }
                ensure!(
                    (row["range_check_performed"] != true || row.get("in_range").is_some())
                        && (row.get("in_range").is_none() || row["range_check_performed"] == true),
                    "Invalid pet action range observation"
                );
            }
        }
        "weapon_enchants" => {
            object(
                value,
                &[
                    ("coverage", "public_weapon_enchant_lists", true),
                    ("hands", "map", true),
                ],
            )?;
            let hands = value["hands"].as_object().unwrap();
            ensure!(
                hands
                    .keys()
                    .all(|hand| matches!(hand.as_str(), "main_hand" | "off_hand" | "ranged")),
                "Invalid weapon hand"
            );
            for hand in hands.values() {
                object(hand, &[("entries", "map", true)])?;
                entries(
                    &hand["entries"],
                    &[
                        ("has_enchant", "bool", true),
                        ("enchant_type", "text", true),
                        ("enchant_id", "positive", false),
                        ("charges", "uint", false),
                        ("remaining_milliseconds", "uint", false),
                    ],
                )?;
                let rows = hand["entries"].as_object().unwrap();
                ensure!(
                    rows.len() <= 32 && (1..=rows.len()).all(|i| rows.contains_key(&i.to_string())),
                    "Invalid weapon enchant list"
                );
                for row in rows.values() {
                    ensure!(
                        matches!(
                            row["enchant_type"].as_str(),
                            Some("none" | "permanent" | "temporary" | "imbue")
                        ),
                        "Invalid weapon enchant type"
                    );
                    for field in ["enchant_id", "charges", "remaining_milliseconds"] {
                        if let Some(v) = row.get(field) {
                            ensure!(
                                row["has_enchant"] == true
                                    && v.as_u64().is_some_and(|n| n <= u32::MAX as u64
                                        && (field != "enchant_id" || n > 0)),
                                "Invalid weapon enchant detail"
                            );
                        }
                    }
                }
            }
        }
        "gossip_options" => {
            object(
                value,
                &[("coverage", "text", true), ("entries", "map", true)],
            )?;
            ensure!(
                value["coverage"] == "visible_gossip_options",
                "Invalid gossip option coverage"
            );
            let rows = value["entries"].as_object().unwrap();
            ensure!(
                rows.len() <= 128 && (1..=rows.len()).all(|i| rows.contains_key(&i.to_string())),
                "Invalid gossip option slots"
            );
            for row in rows.values() {
                object(
                    row,
                    &[
                        ("name", "text", true),
                        ("option_id", "uint", false),
                        ("spell_id", "uint", false),
                        ("reported_status", "uint", false),
                        ("reported_flags", "uint", false),
                        ("reported_order_index", "int", false),
                    ],
                )?;
                ensure!(
                    row["name"]
                        .as_str()
                        .is_some_and(|text| !text.is_empty() && text.len() <= 512),
                    "Invalid gossip option name"
                );
                if let Some(number) = row.get("reported_order_index") {
                    ensure!(
                        number.as_i64().is_some_and(|n| i32::try_from(n).is_ok()),
                        "Invalid gossip option order"
                    );
                }
                for field in ["option_id", "spell_id", "reported_status", "reported_flags"] {
                    if let Some(number) = row.get(field) {
                        ensure!(
                            number.as_u64().is_some_and(|n| n <= u64::from(u32::MAX)
                                && (!matches!(field, "option_id" | "spell_id") || n > 0)),
                            "Invalid gossip option number"
                        );
                    }
                }
            }
        }
        "gossip_quests" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("available", "map", false),
                    ("active", "map", false),
                ],
            )?;
            ensure!(
                value["coverage"] == "visible_gossip_quest_lists"
                    && (value.get("available").is_some() || value.get("active").is_some()),
                "Invalid gossip quest coverage"
            );
            for field in ["available", "active"] {
                if let Some(rows) = value.get(field) {
                    entries(
                        rows,
                        &[
                            ("repeatable", "bool", false),
                            ("is_complete", "bool", false),
                            ("is_trivial", "bool", false),
                            ("is_ignored", "bool", false),
                        ],
                    )?;
                    ensure!(
                        rows.as_object().unwrap().len() <= 128,
                        "Too many gossip quests"
                    );
                    for row in rows.as_object().unwrap().values() {
                        ensure!(
                            row.as_object().unwrap().values().all(Value::is_boolean),
                            "Invalid gossip quest flag"
                        );
                    }
                }
            }
        }
        "player_auras" => {
            object(
                value,
                &[("coverage", "text", true), ("groups", "map", true)],
            )?;
            ensure!(
                value["coverage"] == "readable_nonprivate_player_aura_lists",
                "Invalid player aura coverage"
            );
            object(
                &value["groups"],
                &[("helpful", "map", false), ("harmful", "map", false)],
            )?;
            let groups = value["groups"].as_object().unwrap();
            ensure!(!groups.is_empty(), "No observed aura groups");
            for group in groups.values() {
                object(group, &[("entries", "map", true)])?;
                entries(
                    &group["entries"],
                    &[
                        ("spell_id", "positive", true),
                        ("applications", "uint", false),
                    ],
                )?;
                let rows = group["entries"].as_object().unwrap();
                ensure!(rows.len() <= 256, "Too many player aura rows");
                for (key, row) in rows {
                    index(key, 1, rows.len() as u32)?;
                    ensure!(
                        row.get("applications").is_none_or(|v| v.as_u64().is_some()),
                        "Invalid aura applications"
                    );
                }
            }
        }
        "player_resources" => {
            object(
                value,
                &[
                    ("coverage", "text", true),
                    ("health", "map", false),
                    ("primary_power", "map", false),
                ],
            )?;
            ensure!(
                value["coverage"] == "readable_player_resources"
                    && (value.get("health").is_some() || value.get("primary_power").is_some()),
                "Invalid player resource coverage"
            );
            for field in ["health", "primary_power"] {
                if let Some(row) = value.get(field) {
                    if field == "health" {
                        object(row, &[("current", "uint", true), ("maximum", "uint", true)])?;
                    } else {
                        object(
                            row,
                            &[
                                ("current", "uint", true),
                                ("maximum", "uint", true),
                                ("power_type", "uint", true),
                                ("units", "text", true),
                            ],
                        )?;
                        ensure!(
                            row["power_type"].as_u64().unwrap() <= 63
                                && row["units"] == "unmodified",
                            "Invalid primary power context"
                        );
                    }
                    ensure!(
                        row["current"].as_u64().unwrap() <= row["maximum"].as_u64().unwrap(),
                        "Invalid player resource pair"
                    );
                }
            }
        }
        "equipment_durability" => {
            object(
                value,
                &[
                    ("coverage", "readable_equipped_slots_only", true),
                    ("entries", "map", true),
                ],
            )?;
            entries(
                &value["entries"],
                &[("current", "uint", true), ("maximum", "positive", true)],
            )?;
            for (slot, row) in value["entries"].as_object().unwrap() {
                index(slot, 1, 19)?;
                ensure!(
                    row["current"].as_u64().unwrap() <= row["maximum"].as_u64().unwrap(),
                    "Invalid equipment durability"
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
                    ("map_position", "map", false),
                    ("observed_route_node_ids", "map", false),
                    ("fare_copper", "wallet_copper", false),
                ],
            )?;
            let rows = value["entries"].as_object().unwrap();
            ensure!(rows.len() <= 512, "Too many flight map observations");
            for (key, row) in rows {
                index(key, 1, 4096)?;
                if let Some(path) = row.get("observed_route_node_ids") {
                    let path = path
                        .as_object()
                        .ok_or_else(|| anyhow::anyhow!("Invalid flight route nodes"))?;
                    ensure!(
                        (2..=65).contains(&path.len())
                            && row["state"] == "reachable"
                            && row["map_transition"] == false,
                        "Invalid flight route nodes"
                    );
                    let origins: Vec<_> = rows
                        .values()
                        .filter(|node| {
                            node["state"] == "current" && node["map_transition"] == false
                        })
                        .collect();
                    ensure!(
                        origins.len() == 1
                            && path.get("1") == origins[0].get("node_id")
                            && path.get(&path.len().to_string()) == row.get("node_id"),
                        "Invalid flight route endpoints"
                    );
                    let mut seen = std::collections::BTreeSet::new();
                    for position in 1..=path.len() {
                        let id = path
                            .get(&position.to_string())
                            .and_then(Value::as_u64)
                            .filter(|id| *id > 0 && *id <= u32::MAX as u64)
                            .ok_or_else(|| anyhow::anyhow!("Invalid flight route node"))?;
                        let matching: Vec<_> = rows
                            .values()
                            .filter(|node| node["node_id"].as_u64() == Some(id))
                            .collect();
                        ensure!(
                            seen.insert(id)
                                && matching.len() == 1
                                && matching[0]["map_transition"] == false,
                            "Ambiguous flight route node"
                        );
                    }
                }
                if let Some(position) = row.get("map_position") {
                    object(
                        position,
                        &[("x", "unit_interval", true), ("y", "unit_interval", true)],
                    )?;
                }
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
                    ("usability", "map", false),
                    ("charge_count", "map", false),
                    ("reported_cast_count", "uint", false),
                    ("cooldown_state", "map", false),
                ],
            )?;
            let rows = value["entries"].as_object().unwrap();
            ensure!(rows.len() <= 1024, "Too many spellbook observations");
            for (key, row) in rows {
                index(key, 1, 4096)?;
                if let Some(charges) = row.get("charge_count") {
                    object(
                        charges,
                        &[("current", "uint", true), ("maximum", "positive", true)],
                    )?;
                    ensure!(
                        charges["current"]
                            .as_u64()
                            .zip(charges["maximum"].as_u64())
                            .is_some_and(|(current, maximum)| current <= maximum
                                && maximum <= u64::from(u32::MAX)),
                        "Invalid spell charge observation"
                    );
                }
                if let Some(count) = row.get("reported_cast_count") {
                    ensure!(
                        count.as_u64().is_some_and(|n| n <= u64::from(u32::MAX)),
                        "Invalid spell cast count observation"
                    );
                }
                if let Some(cooldown) = row.get("cooldown_state") {
                    object(
                        cooldown,
                        &[("enabled", "bool", true), ("active", "bool", true)],
                    )?;
                }
                if let Some(usability) = row.get("usability") {
                    object(
                        usability,
                        &[
                            ("usable", "bool", true),
                            ("insufficient_power", "bool", true),
                        ],
                    )?;
                }
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
                    ("observed_tree_ids", "map", false),
                    ("observed_node_count", "uint", false),
                    ("staged_changes_observed", "bool", false),
                    ("definition_allocation", "map", false),
                ],
            )?;
            entries(
                &value["nodes"],
                &[
                    ("entry_id", "positive", true),
                    ("rank", "positive", true),
                    ("tree_id", "positive", true),
                    ("definition_id", "positive", false),
                    ("purchased_rank", "uint", false),
                    ("max_rank", "positive", false),
                ],
            )?;
            talent_definition_observation(value)?;
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
