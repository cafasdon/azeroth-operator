use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs::File, io::Read, path::Path};

const MAX_BYTES: u64 = 8 * 1024 * 1024;
const PIN: &str = include_str!("../clients/forever/catalogue.lock.json");

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Items,
    Enchants,
    RandomSuffixes,
    Zones,
    Npcs,
    Factions,
    Encounters,
    Consumables,
    SpellEffects,
}

impl Section {
    fn fields(&self) -> (&'static str, &'static str) {
        match self {
            Self::Items => ("items", "id"),
            Self::Enchants => ("enchants", "effectId"),
            Self::RandomSuffixes => ("randomSuffixes", "id"),
            Self::Zones => ("zones", "id"),
            Self::Npcs => ("npcs", "id"),
            Self::Factions => ("factions", "id"),
            Self::Encounters => ("encounters", "path"),
            Self::Consumables => ("consumables", "id"),
            Self::SpellEffects => ("spellEffects", "id"),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub section: Section,
    /// Exact numeric ID (enchant effect ID for enchants), encounter path, or name substring.
    /// Omit all filters to get the section count without records.
    pub key: Option<String>,
    /// Restrict items or enchants to a crafted source with this recipe spell ID.
    pub recipe_spell_id: Option<u32>,
    /// Restrict items to this pinned catalogue set ID.
    pub set_id: Option<std::num::NonZeroU32>,
    /// Restrict items to records with a drop source for this NPC ID.
    pub drop_npc_id: Option<std::num::NonZeroU32>,
    /// Restrict items to drop sources in this zone. With an NPC filter, both must match the same source.
    pub drop_zone_id: Option<std::num::NonZeroU32>,
    /// Restrict items to records with a reward source for this quest ID.
    pub reward_quest_id: Option<std::num::NonZeroU32>,
    /// Restrict items to records with a sale source for this vendor NPC ID.
    pub vendor_npc_id: Option<std::num::NonZeroU32>,
    /// Restrict items to records with a reputation source for this reputation faction ID.
    pub reputation_faction_id: Option<std::num::NonZeroU32>,
    /// Skip this many matching records before returning up to 20 records. Requires a filter to return records.
    pub offset: Option<usize>,
}

fn load(path: &Path) -> Result<(Value, Value)> {
    let provenance: Value = serde_json::from_str(PIN)?;
    ensure!(
        path.metadata()?.is_file(),
        "Catalogue must be a regular file"
    );
    let mut data = Vec::new();
    File::open(path)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut data)?;
    ensure!(data.len() as u64 <= MAX_BYTES, "Catalogue too large");
    ensure!(
        provenance["sha256"] == format!("{:x}", Sha256::digest(&data)),
        "Catalogue checksum mismatch"
    );
    Ok((serde_json::from_slice(&data)?, provenance))
}

pub fn lookup(path: &Path, query: &Query) -> Result<Value> {
    let (db, provenance) = load(path)?;
    let (section, identity) = query.section.fields();
    let rows = db[section]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing catalogue section"))?;
    let mut records = Vec::new();
    let mut has_more = false;
    let key = query.key.as_deref().map(str::trim);
    if let Some(key) = key {
        ensure!(!key.is_empty() && key.len() <= 160, "Invalid catalogue key");
    }
    if let Some(recipe) = query.recipe_spell_id {
        ensure!(
            recipe != 0 && matches!(query.section, Section::Items | Section::Enchants),
            "Expected a positive recipe spell ID for items or enchants"
        );
    }
    let item_filter = query.set_id.is_some()
        || query.drop_npc_id.is_some()
        || query.drop_zone_id.is_some()
        || query.reward_quest_id.is_some()
        || query.vendor_npc_id.is_some()
        || query.reputation_faction_id.is_some();
    ensure!(
        !item_filter || matches!(query.section, Section::Items),
        "Set and acquisition source filters are supported for items only"
    );
    if key.is_some() || query.recipe_spell_id.is_some() || item_filter {
        let id = key.and_then(|key| key.parse::<u32>().ok());
        let name = key.map(str::to_lowercase);
        let mut remaining_offset = query.offset.unwrap_or_default();
        for row in rows {
            let matches = if key.is_none() {
                true
            } else if identity == "path" {
                row[identity].as_str() == key
            } else if let Some(id) = id {
                row[identity].as_u64() == Some(u64::from(id))
            } else {
                row["name"]
                    .as_str()
                    .zip(name.as_deref())
                    .is_some_and(|(s, name)| s.to_lowercase().contains(name))
            };
            let matches_recipe = query.recipe_spell_id.is_none_or(|recipe| {
                row["sources"].as_array().is_some_and(|sources| {
                    sources.iter().any(|source| {
                        source["crafted"]["spellId"].as_u64() == Some(u64::from(recipe))
                    })
                })
            });
            let matches_set = query
                .set_id
                .is_none_or(|id| row["setId"].as_u64() == Some(u64::from(id.get())));
            let matches_drop = (query.drop_npc_id.is_none() && query.drop_zone_id.is_none())
                || row["sources"].as_array().is_some_and(|sources| {
                    sources.iter().any(|source| {
                        let drop = &source["drop"];
                        query
                            .drop_npc_id
                            .is_none_or(|id| drop["npcId"].as_u64() == Some(u64::from(id.get())))
                            && query.drop_zone_id.is_none_or(|id| {
                                drop["zoneId"].as_u64() == Some(u64::from(id.get()))
                            })
                    })
                });
            let matches_quest = query.reward_quest_id.is_none_or(|id| {
                row["sources"].as_array().is_some_and(|sources| {
                    sources
                        .iter()
                        .any(|source| source["quest"]["id"].as_u64() == Some(u64::from(id.get())))
                })
            });
            let matches_vendor = query.vendor_npc_id.is_none_or(|id| {
                row["sources"].as_array().is_some_and(|sources| {
                    sources.iter().any(|source| {
                        source["soldBy"]["npcId"].as_u64() == Some(u64::from(id.get()))
                    })
                })
            });
            let matches_reputation = query.reputation_faction_id.is_none_or(|id| {
                row["sources"].as_array().is_some_and(|sources| {
                    sources.iter().any(|source| {
                        source["rep"]["repFactionId"].as_u64() == Some(u64::from(id.get()))
                    })
                })
            });
            if matches
                && matches_recipe
                && matches_set
                && matches_drop
                && matches_quest
                && matches_vendor
                && matches_reputation
            {
                if remaining_offset > 0 {
                    remaining_offset -= 1;
                    continue;
                }
                if records.len() == 20 {
                    has_more = true;
                    break;
                }
                records.push(row);
            }
        }
    }
    Ok(
        json!({"family":"forever","source":provenance,"section":section,
        "section_count":rows.len(),"records":records,"has_more":has_more,
        "acquisition_verified":false}),
    )
}

/// Equipment categories from the pinned Forever catalogue's ItemType enum.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GearType {
    Head = 1,
    Neck = 2,
    Shoulder = 3,
    Back = 4,
    Chest = 5,
    Wrist = 6,
    Hands = 7,
    Waist = 8,
    Legs = 9,
    Feet = 10,
    Finger = 11,
    Trinket = 12,
    Weapon = 13,
    Ranged = 14,
}

pub fn review_owned(
    path: &Path,
    snapshot: &Snapshot,
    offset: usize,
    limit: usize,
    item_type: Option<GearType>,
    set_id: Option<std::num::NonZeroU32>,
) -> Result<Value> {
    ensure!(
        snapshot.client.family == Family::Forever,
        "Expected a Forever snapshot"
    );
    ensure!(
        (1..=50).contains(&limit),
        "Expected a limit between 1 and 50"
    );
    let (db, provenance) = load(path)?;
    let items = db["items"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing catalogue items"))?;
    let catalogue: BTreeMap<_, _> = items
        .iter()
        .filter_map(|item| Some((item["id"].as_u64()?, item)))
        .collect();
    ensure!(
        catalogue.len() == items.len(),
        "Catalogue IDs are missing or ambiguous"
    );
    let mut entries = Vec::new();
    let mut coverage = serde_json::Map::new();
    let mut total = 0usize;
    let mut matched = 0usize;
    let mut included = 0usize;
    let mut set_locations = BTreeMap::<u64, BTreeMap<&str, usize>>::new();
    for storage in ["equipment", "bags", "character_bank", "account_bank"] {
        let Some(section) = snapshot.sections.get(storage).and_then(Value::as_object) else {
            coverage.insert(
                storage.into(),
                json!({"status":"unavailable","reason":snapshot.unavailable.get(storage)}),
            );
            continue;
        };
        let mut count = 0usize;
        let mut record = |container: Option<&str>, slot: &str, item: &Value| {
            let item_id = item["item_id"].as_u64();
            let reference = item_id.and_then(|id| catalogue.get(&id));
            total += 1;
            count += 1;
            matched += usize::from(reference.is_some());
            let wrong_type = item_type.is_some_and(|kind| {
                reference.and_then(|item| item["type"].as_u64()) != Some(kind as u64)
            });
            let wrong_set = set_id.is_some_and(|id| {
                reference.and_then(|item| item["setId"].as_u64()) != Some(u64::from(id.get()))
            });
            if set_id.is_some()
                && !wrong_set
                && let Some(id) = item_id
            {
                *set_locations
                    .entry(id)
                    .or_default()
                    .entry(storage)
                    .or_default() += 1;
            }
            if wrong_type || wrong_set {
                return;
            }
            if included >= offset && entries.len() < limit {
                entries.push(json!({"storage":storage,"container":container,"slot":slot,"observed":item,
                    "catalogue_base_item":reference.map(|item| json!({
                        "id":item["id"],"name":item["name"],"type":item["type"],
                        "armorType":item["armorType"],"weaponType":item["weaponType"],
                        "handType":item["handType"],"rangedWeaponType":item["rangedWeaponType"],
                        "reqLvl":item["reqLvl"],"classAllowlist":item["classAllowlist"],
                        "factionRestriction":item["factionRestriction"],"requiredProfession":item["requiredProfession"],
                        "unique":item["unique"],"limitCategory":item["limitCategory"],
                        "setId":item["setId"],"setName":item["setName"],
                        "randomSuffixOptions":item["randomSuffixOptions"]
                    }))}));
            }
            included += 1;
        };
        for (key, value) in section {
            if storage == "equipment" {
                record(None, key, value);
            } else if let Some(slots) = value.as_object() {
                for (slot, item) in slots {
                    record(Some(key), slot, item);
                }
            }
        }
        coverage.insert(
            storage.into(),
            json!({"status":"observed","occupied_slots":count}),
        );
    }
    let mut result = json!({"family":"forever","captured_at":snapshot.captured_at,"catalogue_source":provenance,
        "coverage":coverage,"observed_item_locations":total,"base_item_matches":matched,
        "items":entries,"filtered_item_locations":included,"has_more":included.saturating_sub(offset)>limit,
        "live":false,"ranked":false,"equip_legality_verified":false});
    if let Some(set_id) = set_id {
        let pieces: Vec<_> = catalogue
            .iter()
            .filter(|(_, item)| item["setId"].as_u64() == Some(u64::from(set_id.get())))
            .map(|(id, item)| {
                json!({"item_id":id,"name":item["name"],"observed":set_locations.contains_key(id),
                    "observed_locations_by_storage":set_locations.get(id).cloned().unwrap_or_default()})
            })
            .collect();
        result["set_observation"] = json!({"set_id":set_id.get(),"catalogue_pieces":pieces,
            "observed_distinct_pieces":set_locations.len()});
    }
    Ok(result)
}
