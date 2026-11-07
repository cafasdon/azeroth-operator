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
#[serde(rename_all = "snake_case")]
pub enum ItemSource {
    Crafted,
    Drop,
    Quest,
    Vendor,
    Reputation,
    Pvp,
}

impl ItemSource {
    fn field(&self) -> &'static str {
        match self {
            Self::Crafted => "crafted",
            Self::Drop => "drop",
            Self::Quest => "quest",
            Self::Vendor => "soldBy",
            Self::Reputation => "rep",
            Self::Pvp => "pvp",
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
    /// Restrict items to records with this type of acquisition source.
    pub source_kind: Option<ItemSource>,
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
        || query.reputation_faction_id.is_some()
        || query.source_kind.is_some();
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
            let matches_source = query.source_kind.as_ref().is_none_or(|kind| {
                row["sources"].as_array().is_some_and(|sources| {
                    sources
                        .iter()
                        .any(|source| source[kind.field()].is_object())
                })
            });
            if matches
                && matches_recipe
                && matches_set
                && matches_drop
                && matches_quest
                && matches_vendor
                && matches_reputation
                && matches_source
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

// Read only the fields shared with the Forever WoWSims exporter.
fn link_modifiers(item: &Value) -> Option<Value> {
    let link = item["link"].as_str()?;
    let payload = if let Some(payload) = link.strip_prefix("item:") {
        payload
    } else {
        let (prefix, rest) = link.split_once("|Hitem:")?;
        if !prefix.is_empty()
            && !prefix.strip_prefix("|c").is_some_and(|color| {
                color.len() == 8 && color.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        {
            return None;
        }
        let (payload, display) = rest.split_once("|h")?;
        display.split_once("|h")?;
        payload
    };
    let mut fields = payload.split(':');
    let id = fields.next()?.parse::<u64>().ok()?;
    if item["item_id"].as_u64()? != id {
        return None;
    }
    let enchant = fields.next()?;
    let suffix = fields.nth(4)?;
    let enchant = if enchant.is_empty() {
        None
    } else {
        Some(enchant.parse::<u32>().ok()?)
    };
    let suffix = if suffix.is_empty() {
        None
    } else {
        Some(suffix.parse::<i32>().ok()?)
    };
    Some(json!({"enchant_id":enchant,"random_suffix_id":suffix}))
}

// Mirrors the pinned simulator's itemfit.EnchantFits. Distinct item
// categories have disjoint slots, and weapon enchants cover both hands.
fn enchant_type_fits(enchant: &Value, item: &Value, item_type: u64) -> bool {
    let weapon = GearType::Weapon as u64;
    let ranged = GearType::Ranged as u64;
    let hand = item["handType"].as_u64().unwrap_or_default();
    let weapon_type = item["weaponType"].as_u64().unwrap_or_default();
    let ranged_type = item["rangedWeaponType"].as_u64().unwrap_or_default();
    let enchant_type = enchant["enchantType"].as_u64().unwrap_or_default();
    let primary = enchant["type"].as_u64();
    let fits_category = primary == Some(item_type)
        || enchant["extraTypes"]
            .as_array()
            .is_some_and(|types| types.iter().any(|kind| kind.as_u64() == Some(item_type)));
    fits_category
        && (item_type != weapon || (1..=4).contains(&hand))
        && (enchant_type != 1 || hand == 4) // Two-handed.
        && (enchant_type != 4 || weapon_type == 8) // Staff.
        && ((enchant_type == 2) == (weapon_type == 7)) // Shield.
        && ((enchant_type == 5) == (weapon_type == 5)) // Held off-hand.
        && if primary == Some(ranged) {
            (1..=3).contains(&ranged_type) // Bow, crossbow or gun.
        } else {
            matches!(ranged_type, 0 | 5) // No ranged subtype, or wand.
        }
}

fn modifier_candidates(db: &Value, modifiers: &Value, item: Option<&Value>) -> Value {
    let matches = |section: &str, field: &str, id: Option<i64>| -> Vec<&Value> {
        let Some(id) = id.filter(|id| *id != 0) else {
            return Vec::new();
        };
        db[section]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|record| record[field].as_i64() == Some(id))
            .collect()
    };
    let enchants = matches("enchants", "effectId", modifiers["enchant_id"].as_i64());
    let fitting_enchants = item.and_then(|item| {
        let item_type = item["type"]
            .as_u64()
            .filter(|kind| (1..=14).contains(kind))?;
        Some(
            enchants
                .iter()
                .copied()
                .filter(|enchant| enchant_type_fits(enchant, item, item_type))
                .collect::<Vec<_>>(),
        )
    });
    let suffixes = matches(
        "randomSuffixes",
        "id",
        modifiers["random_suffix_id"].as_i64(),
    );
    let suffix_effects = item.and_then(|item| match suffixes.as_slice() {
        [suffix] => scaled_suffix_effects(item, suffix),
        _ => None,
    });
    json!({
        "enchants":enchants,
        "fitting_enchants":fitting_enchants,
        "random_suffixes":suffixes,
        "default_scaling_suffix_allocation_delta":suffix_effects.as_ref().map(|(stats, _)| stats),
        "default_scaling_suffix_extra_effects":suffix_effects.as_ref().map(|(_, extra)| extra)
    })
}

fn scaled_suffix_effects(item: &Value, suffix: &Value) -> Option<(Value, Value)> {
    let id = suffix["id"].as_i64()?;
    let listed = item["randomSuffixOptions"]
        .as_array()?
        .iter()
        .any(|option| option.as_i64() == Some(id));
    if !listed {
        return None;
    }
    let scaling = &item["scalingOptions"]["0"];
    let points = scaling["randPropPoints"].as_i64()?;
    let mut deltas = BTreeMap::new();
    for (stat, allocation) in suffix["stats"].as_array()?.iter().enumerate() {
        let delta = (allocation.as_f64()? * points as f64 / 10_000.0).round();
        if delta != 0.0 {
            deltas.insert(stat, delta);
        }
    }
    let pseudo_stats: BTreeMap<_, _> = suffix["pseudoStats"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(id, value)| {
            value
                .as_f64()
                .filter(|value| *value != 0.0)
                .map(|_| (id, value))
        })
        .collect();
    let weapon_damage = (suffix["reducedCasterDps"] == true)
        .then_some(&scaling["reducedCasterWeaponDamage"])
        .filter(|damage| damage.is_object());
    Some((
        json!({"random_property_points":points,"by_simulator_stat_id":deltas}),
        json!({"by_simulator_pseudo_stat_id":pseudo_stats,"replacement_weapon_damage":weapon_damage}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_listed_suffix_allocations_and_rounds_halves_away_from_zero() {
        let item = json!({"randomSuffixOptions":[7],"scalingOptions":{"0":{
            "randPropPoints":6,"reducedCasterWeaponDamage":{"weaponDamageMin":44}
        }}});
        let suffix = json!({"id":7,"stats":[47500,-47500,0],
            "pseudoStats":[0,0,0,0,1],"reducedCasterDps":true});
        let (stats, extra) = scaled_suffix_effects(&item, &suffix).unwrap();
        assert_eq!(
            stats,
            json!({"random_property_points":6,"by_simulator_stat_id":{"0":29.0,"1":-29.0}})
        );
        assert_eq!(
            extra,
            json!({"by_simulator_pseudo_stat_id":{"4":1},"replacement_weapon_damage":{"weaponDamageMin":44}})
        );
        assert!(scaled_suffix_effects(&item, &json!({"id":8,"stats":[47500]})).is_none());
    }

    #[test]
    fn distinguishes_weapon_enchant_rows_with_the_same_effect_id() {
        let db = json!({"enchants":[
            {"effectId":851,"type":13,"enchantType":1},
            {"effectId":851,"type":13,"enchantType":2},
            {"effectId":851,"type":6}
        ]});
        let modifiers = json!({"enchant_id":851});
        let sword = json!({"type":13,"handType":4,"weaponType":9});
        let shield = json!({"type":13,"handType":3,"weaponType":7});
        for (item, index) in [(&sword, 0), (&shield, 1)] {
            let result = modifier_candidates(&db, &modifiers, Some(item));
            assert_eq!(result["enchants"], db["enchants"]);
            assert_eq!(result["fitting_enchants"], json!([db["enchants"][index]]));
        }
        assert!(modifier_candidates(&db, &modifiers, None)["fitting_enchants"].is_null());
    }

    #[test]
    fn reads_modifier_positions_without_guessing_from_short_or_mismatched_links() {
        for (link, expected) in [
            (
                "item:744:15:1:2:3:4:-12",
                Some(json!({"enchant_id":15,"random_suffix_id":-12})),
            ),
            (
                "|cffaabbcc|Hitem:744:0:0:0:0:0:123:99|h[Example]|h|r",
                Some(json!({"enchant_id":0,"random_suffix_id":123})),
            ),
            (
                "|Hitem:744::::::|h[Example]|h",
                Some(json!({"enchant_id":null,"random_suffix_id":null})),
            ),
            ("item:744:15", None),
            ("item:745:15:0:0:0:0:-12", None),
            ("item:744:x:0:0:0:0:-12", None),
            ("|Hitem:744:15:0:0:0:0:-12|h[Example]", None),
        ] {
            assert_eq!(
                link_modifiers(&json!({"item_id":744,"link":link})),
                expected,
                "{link}"
            );
        }
    }
}

pub fn review_owned(
    path: &Path,
    snapshot: &Snapshot,
    offset: usize,
    limit: usize,
    item_type: Option<GearType>,
    set_id: Option<std::num::NonZeroU32>,
    observed_item_id: Option<std::num::NonZeroU32>,
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
            let wrong_item =
                observed_item_id.is_some_and(|id| item_id != Some(u64::from(id.get())));
            if wrong_type || wrong_set || wrong_item {
                return;
            }
            if included >= offset && entries.len() < limit {
                let modifiers = link_modifiers(item);
                entries.push(json!({"storage":storage,"container":container,"slot":slot,"observed":item,
                    "catalogue_modifier_candidates":modifiers.as_ref().map(|fields| modifier_candidates(&db, fields, reference.copied())),
                    "link_modifiers":modifiers,
                    "catalogue_base_item":reference.map(|item| json!({
                        "id":item["id"],"name":item["name"],"type":item["type"],
                        "armorType":item["armorType"],"weaponType":item["weaponType"],
                        "handType":item["handType"],"rangedWeaponType":item["rangedWeaponType"],
                        "reqLvl":item["reqLvl"],"classAllowlist":item["classAllowlist"],
                        "factionRestriction":item["factionRestriction"],"requiredProfession":item["requiredProfession"],
                        "unique":item["unique"],"limitCategory":item["limitCategory"],
                        "setId":item["setId"],"setName":item["setName"],
                        "randomSuffixOptions":item["randomSuffixOptions"],
                        "simulator_default_scaling":item["scalingOptions"]["0"],
                        "weaponSpeed":item["weaponSpeed"],
                        "itemEffects":item["itemEffects"]
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
