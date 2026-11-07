use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::Path,
    sync::OnceLock,
};

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

#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DropInstanceKind {
    Dungeon = 1,
    Raid = 2,
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceFaction {
    Alliance = 1,
    Horde = 2,
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReputationTier {
    Hated = 1,
    Hostile = 2,
    Unfriendly = 3,
    Neutral = 4,
    Friendly = 5,
    Honored = 6,
    Revered = 7,
    Exalted = 8,
}

#[derive(Clone, Copy, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservedOwnership {
    Owned,
    Absent,
    Unknown,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatGain {
    /// Regular stat ID from the pinned simulator, including 0 for Strength.
    stat_id: u32,
    /// Positive gain over the saved slot's default base stat.
    minimum_gain: f64,
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ItemOrder {
    ItemLevelDesc,
    StatGainDesc,
    WeaponDpsDesc,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ItemEffectTrigger {
    OnUse,
    Proc,
    StackProc,
    StackingAura,
}

impl ItemEffectTrigger {
    fn field(&self) -> &'static str {
        match self {
            Self::OnUse => "onUse",
            Self::Proc => "proc",
            Self::StackProc => "stackProc",
            Self::StackingAura => "stackingAura",
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
    /// Up to 100 distinct exact item IDs for items or consumables, in place of key. Results follow catalogue order.
    pub item_ids: Option<Vec<std::num::NonZeroU32>>,
    /// Plan bag stock for up to 20 distinct consumables from the supplied snapshot.
    pub consumable_targets: Option<Vec<ConsumableTarget>>,
    /// Match the exact groupKey recorded by the pinned simulator for consumable buffs.
    pub consumable_group_key: Option<String>,
    /// Restrict items or enchants to a crafted source with this recipe spell ID.
    pub recipe_spell_id: Option<u32>,
    /// Restrict items to this pinned catalogue set ID.
    pub set_id: Option<std::num::NonZeroU32>,
    /// Restrict items to catalogue placements for this simulator equipment slot.
    pub simulator_slot: Option<SimulatorSlot>,
    /// Preserve any recorded set membership of the saved item in the requested slot.
    #[serde(default)]
    pub preserve_equipped_set: bool,
    /// Restrict items to at least this recorded item level at default scaling.
    pub minimum_item_level: Option<std::num::NonZeroU32>,
    /// Minimum recorded default weapon DPS.
    pub minimum_weapon_dps: Option<f64>,
    /// Minimum recorded weapon swing time in seconds (inclusive).
    pub minimum_weapon_speed: Option<f64>,
    /// Maximum recorded weapon swing time in seconds (inclusive).
    pub maximum_weapon_speed: Option<f64>,
    /// Restrict items to a recorded effect with this buff spell ID.
    pub item_effect_buff_id: Option<std::num::NonZeroU32>,
    /// Restrict items to this recorded effect trigger, on the same effect as any buff ID.
    pub item_effect_trigger: Option<ItemEffectTrigger>,
    /// Require this gain over the saved slot's default item level. Requires a known equipped baseline.
    pub minimum_item_level_gain: Option<std::num::NonZeroU32>,
    /// Require a gain in one regular base stat over the saved equipped slot.
    pub minimum_stat_gain: Option<StatGain>,
    /// Order matching item records before paging. Stat gain ordering requires minimum_stat_gain.
    pub order_by: Option<ItemOrder>,
    /// Restrict items to records with a drop source for this NPC ID.
    pub drop_npc_id: Option<std::num::NonZeroU32>,
    /// Restrict items to drop sources in this zone. With an NPC filter, both must match the same source.
    pub drop_zone_id: Option<std::num::NonZeroU32>,
    /// Restrict drops to the catalogue dungeon or raid category on the same source.
    pub drop_instance_kind: Option<DropInstanceKind>,
    /// Exact case-insensitive drop label, such as Trash or World Drop.
    pub drop_label: Option<String>,
    /// Restrict items to records with a reward source for this quest ID.
    pub reward_quest_id: Option<std::num::NonZeroU32>,
    /// Restrict items to records with a sale source for this vendor NPC ID.
    pub vendor_npc_id: Option<std::num::NonZeroU32>,
    /// Restrict items to sale sources in this zone. With an NPC filter, both must match the same sale.
    pub vendor_zone_id: Option<std::num::NonZeroU32>,
    /// Restrict items to sale sources with this recorded faction tag.
    pub vendor_faction: Option<SourceFaction>,
    /// Restrict items to records with a reputation source for this reputation faction ID.
    pub reputation_faction_id: Option<std::num::NonZeroU32>,
    /// Exact required rank on the same reputation reward source.
    pub reputation_required_tier: Option<ReputationTier>,
    /// Alliance or Horde tag recorded on the same reputation reward source.
    pub reputation_source_faction: Option<SourceFaction>,
    /// Restrict items to records with this type of acquisition source.
    pub source_kind: Option<ItemSource>,
    /// Join explicit NPC, zone and faction IDs on returned item sources to catalogue names.
    #[serde(default)]
    pub include_source_references: bool,
    /// Optional owner-scoped Forever snapshot for stock counts and, for items, observed source progress.
    pub acquisition_snapshot_id: Option<String>,
    /// Filter by base item presence in the supplied snapshot. Absent requires all four storage sections.
    pub observed_ownership: Option<ObservedOwnership>,
    /// Keep gear without a known conflict against the supplied saved character.
    #[serde(default)]
    pub exclude_observed_requirement_conflicts: bool,
    /// Skip this many matching records before returning up to 20 records. Requires a filter to return records.
    pub offset: Option<usize>,
    /// Build simulator inputs for an exact encounter preset path.
    pub simulation: Option<SimulationSettings>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SimulationSettings {
    pub duration_seconds: u32,
    pub duration_variation_seconds: u32,
    pub execute_proportion_20: f64,
    pub execute_proportion_35: f64,
    pub iterations: u32,
    pub seed: std::num::NonZeroU32,
}

fn encounter_input(record: &Value, settings: &SimulationSettings) -> Result<Value> {
    ensure!(
        (1..=3600).contains(&settings.duration_seconds)
            && settings.duration_variation_seconds < settings.duration_seconds
            && (0.0..=1.0).contains(&settings.execute_proportion_35)
            && (0.0..=settings.execute_proportion_35).contains(&settings.execute_proportion_20)
            && (1..=100_000).contains(&settings.iterations),
        "Invalid simulation settings"
    );
    let targets = record["targets"]
        .as_array()
        .filter(|targets| !targets.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Encounter targets unavailable"))?;
    let targets: Vec<_> = targets
        .iter()
        .map(|entry| {
            entry["target"]
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Invalid encounter target"))
        })
        .collect::<Result<_>>()?;
    Ok(json!({
        "encounter":{"duration":settings.duration_seconds,
            "durationVariation":settings.duration_variation_seconds,
            "executeProportion20":settings.execute_proportion_20,
            "executeProportion35":settings.execute_proportion_35,"targets":targets},
        "simOptions":{"iterations":settings.iterations,"randomSeed":settings.seed.get().to_string()}
    }))
}

fn load(path: &Path) -> Result<(&'static Value, Value)> {
    static PARSED: OnceLock<Value> = OnceLock::new();
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
    // Every caller verifies the configured file against the same embedded pin.
    let db = match PARSED.get() {
        Some(db) => db,
        None => {
            let parsed = serde_json::from_slice(&data)?;
            PARSED.get_or_init(|| parsed)
        }
    };
    Ok((db, provenance))
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConsumableTarget {
    item_id: std::num::NonZeroU32,
    quantity: std::num::NonZeroU32,
}

pub fn lookup(path: &Path, query: &Query, snapshot: Option<&Snapshot>) -> Result<Value> {
    let (db, provenance) = load(path)?;
    let (section, identity) = query.section.fields();
    let has_inventory_context = matches!(query.section, Section::Items | Section::Consumables);
    let rows = db[section]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing catalogue section"))?;
    let mut records = Vec::new();
    let mut has_more = false;
    let key = query.key.as_deref().map(str::trim);
    if let Some(key) = key {
        ensure!(!key.is_empty() && key.len() <= 160, "Invalid catalogue key");
    }
    let consumable_group = query.consumable_group_key.as_deref().map(str::trim);
    if let Some(group) = consumable_group {
        ensure!(
            matches!(query.section, Section::Consumables)
                && query.consumable_targets.is_none()
                && !group.is_empty()
                && group.len() <= 160,
            "Consumable group filtering requires a group key and no restock targets"
        );
    }
    let item_ids = if let Some(targets) = &query.consumable_targets {
        ensure!(
            matches!(query.section, Section::Consumables)
                && snapshot.is_some()
                && key.is_none()
                && query.item_ids.is_none()
                && query.offset.is_none()
                && query.observed_ownership.is_none()
                && (1..=20).contains(&targets.len()),
            "Consumable targets require a saved snapshot and replace lookup filters and paging"
        );
        let distinct: BTreeSet<_> = targets
            .iter()
            .map(|target| u64::from(target.item_id.get()))
            .collect();
        ensure!(
            distinct.len() == targets.len(),
            "Consumable target IDs must be distinct"
        );
        Some(distinct)
    } else if let Some(ids) = &query.item_ids {
        ensure!(
            has_inventory_context && key.is_none() && (1..=100).contains(&ids.len()),
            "Expected 1 to 100 item IDs in place of a catalogue key"
        );
        let distinct: BTreeSet<_> = ids.iter().map(|id| u64::from(id.get())).collect();
        ensure!(distinct.len() == ids.len(), "Item IDs must be distinct");
        Some(distinct)
    } else {
        None
    };
    if let Some(recipe) = query.recipe_spell_id {
        ensure!(
            recipe != 0 && matches!(query.section, Section::Items | Section::Enchants),
            "Expected a positive recipe spell ID for items or enchants"
        );
    }
    let drop_label = query.drop_label.as_deref().map(str::trim);
    if let Some(label) = drop_label {
        ensure!(
            !label.is_empty() && label.len() <= 160,
            "Invalid drop label"
        );
    }
    let drop_filter = query.drop_npc_id.is_some()
        || query.drop_zone_id.is_some()
        || query.drop_instance_kind.is_some()
        || drop_label.is_some();
    let vendor_filter = query.vendor_npc_id.is_some()
        || query.vendor_zone_id.is_some()
        || query.vendor_faction.is_some();
    let reputation_filter = query.reputation_faction_id.is_some()
        || query.reputation_required_tier.is_some()
        || query.reputation_source_faction.is_some();
    let ownership = if let Some(wanted) = query.observed_ownership {
        let snapshot = snapshot
            .filter(|snapshot| snapshot.client.family == Family::Forever)
            .ok_or_else(|| {
                anyhow::anyhow!("Ownership filtering requires a saved Forever snapshot")
            })?;
        let (owned, complete) = observed_item_ids(snapshot);
        Some((wanted, owned, complete))
    } else {
        None
    };
    let check_requirements = if query.exclude_observed_requirement_conflicts {
        let snapshot = snapshot
            .filter(|snapshot| snapshot.client.family == Family::Forever)
            .ok_or_else(|| {
                anyhow::anyhow!("Requirement filtering requires a saved Forever snapshot")
            })?;
        Some(item_requirement_check(snapshot))
    } else {
        None
    };
    let equipped = query
        .simulator_slot
        .zip(snapshot)
        .filter(|(_, snapshot)| snapshot.client.family == Family::Forever)
        .map(|(slot, snapshot)| equipped_slot(rows, snapshot, slot));
    let protected_set = if query.preserve_equipped_set {
        equipped
            .as_ref()
            .filter(|slot| slot.observed && (slot.item_id.is_none() || slot.item_catalogued))
            .ok_or_else(|| {
                anyhow::anyhow!("Set preservation requires a known saved equipment slot")
            })?
            .set_id
    } else {
        None
    };
    let minimum_upgrade_level = if let Some(gain) = query.minimum_item_level_gain {
        Some(
            equipped
                .as_ref()
                .and_then(|baseline| baseline.level)
                .and_then(|level| level.checked_add(i64::from(gain.get())))
                .ok_or_else(|| {
                    anyhow::anyhow!("Item level gain requires a known equipped baseline")
                })?,
        )
    } else {
        None
    };
    let minimum_stat = if let Some(gain) = &query.minimum_stat_gain {
        ensure!(
            gain.stat_id <= 38 && gain.minimum_gain.is_finite() && gain.minimum_gain > 0.0,
            "Expected a known regular stat ID and positive gain"
        );
        let id = gain.stat_id.to_string();
        let baseline = equipped
            .as_ref()
            .and_then(|slot| slot.regular_stats.as_ref())
            .and_then(|stats| stats.get(&id).map_or(Some(0.0), Value::as_f64))
            .ok_or_else(|| anyhow::anyhow!("Stat gain requires known equipped base stats"))?;
        Some((id, baseline, gain.minimum_gain))
    } else {
        None
    };
    let weapon_filter = [
        query.minimum_weapon_dps,
        query.minimum_weapon_speed,
        query.maximum_weapon_speed,
    ];
    ensure!(
        weapon_filter
            .iter()
            .flatten()
            .all(|value| value.is_finite() && *value > 0.0)
            && query
                .minimum_weapon_speed
                .zip(query.maximum_weapon_speed)
                .is_none_or(|(min, max)| min <= max),
        "Expected positive weapon filters and an ordered speed range"
    );
    let effect_filter = query.item_effect_buff_id.is_some() || query.item_effect_trigger.is_some();
    let inventory_filter = item_ids.is_some() || ownership.is_some();
    let item_filter = check_requirements.is_some()
        || query.set_id.is_some()
        || query.simulator_slot.is_some()
        || query.minimum_item_level.is_some()
        || weapon_filter.iter().any(Option::is_some)
        || effect_filter
        || query.order_by.is_some()
        || drop_filter
        || query.reward_quest_id.is_some()
        || vendor_filter
        || reputation_filter
        || query.source_kind.is_some();
    ensure!(
        !item_filter || matches!(query.section, Section::Items),
        "Item filters are supported for items only"
    );
    ensure!(
        !inventory_filter || has_inventory_context,
        "Inventory filters require items or consumables"
    );
    let filtered = key.is_some()
        || query.recipe_spell_id.is_some()
        || item_filter
        || inventory_filter
        || consumable_group.is_some();
    ensure!(
        !query.include_source_references || (matches!(query.section, Section::Items) && filtered),
        "Source references require a filtered item lookup"
    );
    ensure!(
        query.acquisition_snapshot_id.is_none() || (has_inventory_context && filtered),
        "Snapshot context requires a filtered item or consumable lookup"
    );
    if filtered {
        let id = key.and_then(|key| key.parse::<u32>().ok());
        let name = key.map(str::to_lowercase);
        let mut remaining_offset = query.offset.unwrap_or_default();
        for row in rows {
            if let Some((wanted, owned, complete)) = &ownership {
                let Some(id) = row["id"].as_u64() else {
                    continue;
                };
                let observed = if owned.contains(&id) {
                    ObservedOwnership::Owned
                } else if *complete {
                    ObservedOwnership::Absent
                } else {
                    ObservedOwnership::Unknown
                };
                if observed != *wanted {
                    continue;
                }
            }
            if let Some(ids) = &item_ids
                && !row["id"].as_u64().is_some_and(|id| ids.contains(&id))
            {
                continue;
            }
            if consumable_group.is_some_and(|group| row["groupKey"].as_str() != Some(group)) {
                continue;
            }
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
            let matches_gear = query
                .simulator_slot
                .is_none_or(|slot| simulator_slots(row).contains(&slot))
                && query.minimum_item_level.is_none_or(|minimum| {
                    row["scalingOptions"]["0"]["ilvl"]
                        .as_u64()
                        .is_some_and(|level| level >= u64::from(minimum.get()))
                })
                && minimum_upgrade_level.is_none_or(|minimum| {
                    default_item_level(row).is_some_and(|level| level >= minimum)
                })
                && minimum_stat.as_ref().is_none_or(|(id, baseline, minimum)| {
                    default_regular_stat(row, id).is_some_and(|candidate| {
                        let delta = candidate - baseline;
                        delta.is_finite() && delta >= *minimum
                    })
                });
            let matches_weapon = query
                .minimum_weapon_dps
                .is_none_or(|minimum| default_weapon_dps(row).is_some_and(|dps| dps >= minimum))
                && query
                    .minimum_weapon_speed
                    .is_none_or(|minimum| weapon_speed(row).is_some_and(|speed| speed >= minimum))
                && query
                    .maximum_weapon_speed
                    .is_none_or(|maximum| weapon_speed(row).is_some_and(|speed| speed <= maximum));
            let matches_effect = !effect_filter
                || row["itemEffects"].as_array().is_some_and(|effects| {
                    effects.iter().any(|effect| {
                        query.item_effect_buff_id.is_none_or(|id| {
                            effect_buff_ids(effect).any(|buff| buff == u64::from(id.get()))
                        }) && query
                            .item_effect_trigger
                            .as_ref()
                            .is_none_or(|trigger| effect[trigger.field()].is_object())
                    })
                });
            let matches_set = query
                .set_id
                .is_none_or(|id| recorded_set_id(row) == Some(u64::from(id.get())))
                && protected_set.is_none_or(|id| recorded_set_id(row) == Some(id));
            let matches_drop =
                !drop_filter
                    || row["sources"].as_array().is_some_and(|sources| {
                        sources.iter().any(|source| {
                            let drop = &source["drop"];
                            query.drop_npc_id.is_none_or(|id| {
                                drop["npcId"].as_u64() == Some(u64::from(id.get()))
                            }) && query.drop_zone_id.is_none_or(|id| {
                                drop["zoneId"].as_u64() == Some(u64::from(id.get()))
                            }) && query
                                .drop_instance_kind
                                .is_none_or(|kind| drop["difficulty"].as_u64() == Some(kind as u64))
                                && drop_label.is_none_or(|label| {
                                    ["category", "otherName"].iter().any(|field| {
                                        drop[*field]
                                            .as_str()
                                            .is_some_and(|value| value.eq_ignore_ascii_case(label))
                                    })
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
            let matches_vendor =
                !vendor_filter
                    || row["sources"].as_array().is_some_and(|sources| {
                        sources.iter().any(|source| {
                            let sale = &source["soldBy"];
                            query.vendor_npc_id.is_none_or(|id| {
                                sale["npcId"].as_u64() == Some(u64::from(id.get()))
                            }) && query.vendor_zone_id.is_none_or(|id| {
                                sale["zoneId"].as_u64() == Some(u64::from(id.get()))
                            }) && query.vendor_faction.is_none_or(|faction| {
                                sale["faction"].as_u64() == Some(faction as u64)
                            })
                        })
                    });
            let matches_reputation = !reputation_filter
                || row["sources"].as_array().is_some_and(|sources| {
                    sources.iter().any(|source| {
                        let rep = &source["rep"];
                        query.reputation_faction_id.is_none_or(|id| {
                            rep["repFactionId"].as_u64() == Some(u64::from(id.get()))
                        }) && query
                            .reputation_required_tier
                            .is_none_or(|tier| rep["repLevel"].as_u64() == Some(tier as u64))
                            && query.reputation_source_faction.is_none_or(|faction| {
                                rep["factionId"].as_u64() == Some(faction as u64)
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
                && matches_gear
                && matches_weapon
                && matches_effect
                && matches_drop
                && matches_quest
                && matches_vendor
                && matches_reputation
                && matches_source
                && check_requirements
                    .as_ref()
                    .is_none_or(|check| !simulator_slots(row).is_empty() && check(row).is_none())
            {
                if query.order_by.is_none() {
                    if remaining_offset > 0 {
                        remaining_offset -= 1;
                        continue;
                    }
                    if records.len() == 20 {
                        has_more = true;
                        break;
                    }
                }
                records.push(row);
            }
        }
    }
    if let Some(order) = query.order_by {
        match order {
            ItemOrder::ItemLevelDesc => {
                records.sort_by_cached_key(|item| std::cmp::Reverse(default_item_level(item)));
            }
            ItemOrder::WeaponDpsDesc => {
                records.sort_by(|a, b| {
                    default_weapon_dps(b)
                        .unwrap_or(0.0)
                        .total_cmp(&default_weapon_dps(a).unwrap_or(0.0))
                });
            }
            ItemOrder::StatGainDesc => {
                let (id, baseline, _) = minimum_stat.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Stat gain ordering requires a stat gain filter")
                })?;
                records.sort_by(|a, b| {
                    let value =
                        |item: &Value| default_regular_stat(item, id).unwrap_or(0.0) - baseline;
                    value(b).total_cmp(&value(a))
                });
            }
        }
        let offset = query.offset.unwrap_or_default().min(records.len());
        has_more = records.len() - offset > 20;
        records.drain(..offset);
        records.truncate(20);
    }
    let mut result = json!({"family":"forever","source":provenance,"section":section,
        "section_count":rows.len(),"records":records,"has_more":has_more,
        "acquisition_verified":false});
    if matches!(query.section, Section::Consumables) && filtered {
        result["recorded_shared_cooldowns"] = shared_consumable_cooldowns(&records);
        result["simulator_exclusive_buff_groups"] = exclusive_consumable_groups(&records);
    }
    if let Some(equipped) = equipped {
        result["equipped_slot_comparison"] = equipped.comparison(&records);
    }
    if query.include_source_references {
        result["source_references"] = source_references(db, &records);
    }
    if let Some(settings) = &query.simulation {
        ensure!(
            matches!(query.section, Section::Encounters)
                && key.is_some()
                && query.offset.unwrap_or_default() == 0,
            "Simulation settings require an exact encounter path without pagination"
        );
        let [record] = records.as_slice() else {
            anyhow::bail!("Expected one matching encounter preset");
        };
        result["simulation_input"] = encounter_input(record, settings)?;
    }
    Ok(result)
}

fn exclusive_consumable_groups(records: &[&Value]) -> Value {
    let mut groups = BTreeMap::<&str, BTreeSet<u64>>::new();
    for row in records {
        if let Some((key, id)) = row["groupKey"]
            .as_str()
            .filter(|key| !key.is_empty())
            .zip(row["id"].as_u64())
        {
            groups.entry(key).or_default().insert(id);
        }
    }
    json!(
        groups
            .into_iter()
            .filter(|(_, items)| items.len() > 1)
            .map(|(key, items)| { json!({"group_key":key,"item_ids":items}) })
            .collect::<Vec<_>>()
    )
}

fn shared_consumable_cooldowns(records: &[&Value]) -> Value {
    let mut categories = BTreeMap::<u64, BTreeMap<u64, BTreeSet<u64>>>::new();
    for record in records {
        let Some(item_id) = record["id"].as_u64() else {
            continue;
        };
        for effect in record["itemEffects"].as_array().into_iter().flatten() {
            let on_use = &effect["onUse"];
            let Some(category_id) = on_use["categoryId"].as_u64().filter(|id| *id > 0) else {
                continue;
            };
            let Some(duration_ms) = on_use["categoryCooldownMs"].as_u64().filter(|ms| *ms > 0)
            else {
                continue;
            };
            categories
                .entry(category_id)
                .or_default()
                .entry(item_id)
                .or_default()
                .insert(duration_ms);
        }
    }
    json!(
        categories
            .into_iter()
            .filter(|(_, items)| items.len() > 1)
            .map(|(category_id, items)| {
                json!({"category_id":category_id,"duration_ms_by_item_id":items})
            })
            .collect::<Vec<_>>()
    )
}

struct EquippedSlot {
    slot: SimulatorSlot,
    observed: bool,
    item_id: Option<u64>,
    item_catalogued: bool,
    level: Option<i64>,
    weapon_dps: Option<f64>,
    effect_ids: Option<BTreeSet<u64>>,
    set_id: Option<u64>,
    observed_set_counts: Option<BTreeMap<u64, usize>>,
    uncatalogued_equipped_items: Option<usize>,
    regular_stats: Option<serde_json::Map<String, Value>>,
}

fn effect_buff_ids(effect: &Value) -> impl Iterator<Item = u64> {
    [
        effect["buffId"].as_u64(),
        effect["stackingAura"]["buffId"].as_u64(),
    ]
    .into_iter()
    .flatten()
    .filter(|id| *id > 0)
}

fn recorded_effect_ids(item: &Value) -> Option<BTreeSet<u64>> {
    match item.get("itemEffects") {
        Some(effects) => Some(
            effects
                .as_array()?
                .iter()
                .flat_map(effect_buff_ids)
                .collect(),
        ),
        None => Some(BTreeSet::new()),
    }
}

fn default_item_level(item: &Value) -> Option<i64> {
    item["scalingOptions"]["0"]["ilvl"]
        .as_i64()
        .filter(|level| *level > 0)
}

fn weapon_speed(item: &Value) -> Option<f64> {
    item["weaponSpeed"]
        .as_f64()
        .filter(|speed| speed.is_finite() && *speed > 0.0)
}

fn default_weapon_dps(item: &Value) -> Option<f64> {
    item["scalingOptions"]["0"]["unroundedDps"]
        .as_f64()
        .filter(|dps| dps.is_finite() && *dps > 0.0)
}

fn default_regular_stats(item: &Value) -> Option<serde_json::Map<String, Value>> {
    let scaling = item["scalingOptions"]["0"].as_object()?;
    match scaling.get("stats") {
        Some(stats) => stats.as_object().cloned(),
        None => Some(serde_json::Map::new()),
    }
}

fn default_regular_stat(item: &Value, id: &str) -> Option<f64> {
    let scaling = item["scalingOptions"]["0"].as_object()?;
    let stat = match scaling.get("stats") {
        Some(stats) => stats.as_object()?.get(id),
        None => None,
    };
    stat.map_or(Some(0.0), Value::as_f64)
}

fn regular_stat_delta(
    equipped: &serde_json::Map<String, Value>,
    candidate: &serde_json::Map<String, Value>,
) -> Option<BTreeMap<String, f64>> {
    let mut deltas = BTreeMap::new();
    for id in equipped
        .keys()
        .chain(candidate.keys())
        .collect::<BTreeSet<_>>()
    {
        let before = equipped.get(id).map_or(Some(0.0), Value::as_f64)?;
        let after = candidate.get(id).map_or(Some(0.0), Value::as_f64)?;
        let delta = after - before;
        if !delta.is_finite() {
            return None;
        }
        if delta != 0.0 {
            deltas.insert(id.clone(), delta);
        }
    }
    Some(deltas)
}

fn recorded_set_id(item: &Value) -> Option<u64> {
    item["setId"].as_u64().filter(|id| *id > 0)
}

fn equipped_slot(catalogue: &[Value], snapshot: &Snapshot, slot: SimulatorSlot) -> EquippedSlot {
    let equipment = snapshot
        .sections
        .get("equipment")
        .and_then(Value::as_object);
    let item_id = (1..=19)
        .find(|index| simulator_slot_for_equipment(*index) == Some(slot))
        .and_then(|index| equipment?.get(&index.to_string()))
        .and_then(|item| item["item_id"].as_u64());
    let equipped = item_id.and_then(|id| {
        catalogue
            .iter()
            .find(|item| item["id"].as_u64() == Some(id))
    });
    let set_observation = equipment.map(|equipment| {
        let mut counts = BTreeMap::new();
        let mut uncatalogued = 0;
        for item in equipment.values() {
            let reference = item["item_id"]
                .as_u64()
                .and_then(|id| catalogue.iter().find(|row| row["id"].as_u64() == Some(id)));
            if let Some(reference) = reference {
                if let Some(set_id) = recorded_set_id(reference) {
                    *counts.entry(set_id).or_default() += 1;
                }
            } else {
                uncatalogued += 1;
            }
        }
        (counts, uncatalogued)
    });
    let (observed_set_counts, uncatalogued_equipped_items) = set_observation.unzip();
    EquippedSlot {
        slot,
        observed: equipment.is_some(),
        item_id,
        item_catalogued: equipped.is_some(),
        level: equipped.and_then(default_item_level),
        weapon_dps: equipped.and_then(default_weapon_dps),
        effect_ids: equipped.and_then(recorded_effect_ids),
        set_id: equipped.and_then(recorded_set_id),
        observed_set_counts,
        uncatalogued_equipped_items,
        regular_stats: equipped.and_then(default_regular_stats),
    }
}

impl EquippedSlot {
    fn comparison(&self, records: &[&Value]) -> Value {
        let items: Vec<_> = records
            .iter()
            .map(|item| {
                let level = default_item_level(item);
                let weapon_dps = default_weapon_dps(item);
                let weapon_dps_delta = weapon_dps.zip(self.weapon_dps)
                    .map(|(candidate, equipped)| candidate - equipped);
                let stat_delta = self.regular_stats.as_ref().and_then(|equipped| {
                    let candidate = default_regular_stats(item)?;
                    regular_stat_delta(equipped, &candidate)
                });
                let effect_changes = self.effect_ids.as_ref().and_then(|equipped| {
                    let candidate = recorded_effect_ids(item)?;
                    Some(json!({
                        "added_buff_ids":candidate.difference(equipped).collect::<Vec<_>>(),
                        "removed_buff_ids":equipped.difference(&candidate).collect::<Vec<_>>(),
                    }))
                });
                let candidate_set = recorded_set_id(item);
                let set_projection = self.observed_set_counts.as_ref().map(|counts| {
                    [self.set_id, candidate_set].into_iter().flatten().collect::<BTreeSet<_>>()
                        .into_iter().map(|set_id| {
                            let before = counts.get(&set_id).copied().unwrap_or_default();
                            let after = before.saturating_sub(usize::from(self.set_id == Some(set_id)))
                                + usize::from(candidate_set == Some(set_id));
                            json!({"set_id":set_id,"observed_equipped_pieces":before,
                                "projected_equipped_pieces":after})
                        }).collect::<Vec<_>>()
                });
                json!({"item_id":item["id"],"default_item_level":level,
                    "delta_from_equipped":level.zip(self.level).and_then(|(candidate, equipped)| candidate.checked_sub(equipped)),
                    "default_regular_stat_delta":stat_delta,
                    "default_weapon_dps":weapon_dps,"default_weapon_dps_delta":weapon_dps_delta,
                    "catalogue_effect_buff_changes":effect_changes,
                    "single_slot_set_projection":set_projection})
            })
            .collect();
        json!({"simulator_slot":self.slot,"equipment_observed":self.observed,"equipped_item_id":self.item_id,
            "equipped_default_item_level":self.level,"equipped_default_weapon_dps":self.weapon_dps,
            "uncatalogued_equipped_items":self.uncatalogued_equipped_items,"items":items})
    }
}

fn observed_item_ids(snapshot: &Snapshot) -> (BTreeSet<u64>, bool) {
    let mut owned = BTreeSet::new();
    let mut complete = true;
    for storage in ["equipment", "bags", "character_bank", "account_bank"] {
        let Some(rows) = snapshot.sections.get(storage).and_then(Value::as_object) else {
            complete = false;
            continue;
        };
        if storage == "equipment" {
            owned.extend(rows.values().filter_map(|item| item["item_id"].as_u64()));
        } else {
            owned.extend(
                rows.values()
                    .filter_map(Value::as_object)
                    .flat_map(|slots| slots.values())
                    .filter_map(|item| item["item_id"].as_u64()),
            );
        }
    }
    (owned, complete)
}

fn source_references(db: &Value, items: &[&Value]) -> Value {
    let mut references = BTreeMap::new();
    for (source_kind, id_key, section) in [
        ("drop", "npcId", "npcs"),
        ("drop", "zoneId", "zones"),
        ("soldBy", "npcId", "npcs"),
        ("soldBy", "zoneId", "zones"),
        ("rep", "repFactionId", "factions"),
    ] {
        let names: BTreeMap<_, _> = db[section]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| Some((row["id"].as_u64()?, row["name"].as_str()?)))
            .collect();
        for item in items {
            for (index, source) in item["sources"].as_array().into_iter().flatten().enumerate() {
                if let Some(id) = source[source_kind][id_key].as_u64().filter(|id| *id > 0) {
                    references
                        .entry((item["id"].as_u64(), index + 1))
                        .or_insert_with(Vec::new)
                        .push(json!({"section":section,"id":id,"name":names.get(&id)}));
                }
            }
        }
    }
    json!(
        references
            .into_iter()
            .map(|((item_id, source_index), references)| json!({
                "item_id":item_id,"source_index":source_index,"references":references
            }))
            .collect::<Vec<_>>()
    )
}

pub fn consumable_inventory(
    snapshot: &Snapshot,
    records: &Value,
    targets: Option<&[ConsumableTarget]>,
) -> Result<Value> {
    ensure!(
        snapshot.client.family == Family::Forever,
        "Expected a Forever snapshot"
    );
    let quantities = owned_quantities(snapshot, records)?;
    let plan = targets.map(|targets| {
        targets.iter().map(|target| {
            let item_id = target.item_id.get();
            let required = u64::from(target.quantity.get());
            let Some(stock) = quantities.get(&item_id.to_string()) else {
                return json!({"item_id":item_id,"target_bag_quantity":required,"catalogue_match":false});
            };
            let (bag_shortfall, withdrawals) = crate::analysis::bag_restock(stock, required);
            let accessible = ["bags", "character_bank", "account_bank"].map(|key| stock[key].as_u64());
            let remaining = accessible.iter().flatten().fold(required, |left, count| left.saturating_sub(*count));
            let confirmed_shortfall = (remaining == 0 || accessible.iter().all(Option::is_some))
                .then_some(remaining);
            json!({
                "item_id":item_id,"target_bag_quantity":required,"catalogue_match":true,
                "shortfall_vs_bags":bag_shortfall,"suggested_bank_withdrawals":withdrawals,
                "shortfall_after_observed_storage":remaining,"confirmed_shortfall":confirmed_shortfall
            })
        }).collect::<Vec<_>>()
    });
    let mut result =
        json!({"captured_at":snapshot.captured_at,"owned_quantities_by_item_id":quantities});
    if let Some(plan) = plan {
        result["restock_plan"] = json!(plan);
    }
    Ok(result)
}

fn owned_quantities(
    snapshot: &Snapshot,
    records: &Value,
) -> Result<serde_json::Map<String, Value>> {
    let rows = records
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid item records"))?;
    let mut quantities = serde_json::Map::new();
    for row in rows {
        if let Some(id) = row["id"].as_u64().and_then(|id| u32::try_from(id).ok())
            && id > 0
        {
            let mut found = crate::analysis::find_item(snapshot, id, 0, 1)?;
            quantities.insert(id.to_string(), found["quantity_by_storage"].take());
        }
    }
    Ok(quantities)
}

pub fn acquisition_progress(snapshot: &Snapshot, records: &Value) -> Result<Value> {
    ensure!(
        snapshot.client.family == Family::Forever,
        "Expected a Forever snapshot"
    );
    let quests = snapshot.sections.get("quests").and_then(Value::as_object);
    let factions = snapshot
        .sections
        .get("reputation")
        .and_then(|section| section["entries"].as_object());
    let selected_recipe = snapshot.sections.get("selected_recipe");
    let tracked_recipes = snapshot.sections.get("tracked_recipes");
    let observed_faction = snapshot
        .sections
        .get("stats")
        .and_then(|stats| stats["faction"].as_str())
        .filter(|faction| matches!(*faction, "Alliance" | "Horde"));
    let source_faction = |value: &Value| match value.as_u64() {
        Some(1) => Some("Alliance"),
        Some(2) => Some("Horde"),
        _ => None,
    };
    let faction_matches = |recorded| {
        observed_faction
            .zip(recorded)
            .map(|(observed, recorded)| observed == recorded)
    };
    let mut recipe_review = None;
    let mut sources = Vec::new();
    let owned_quantities = owned_quantities(snapshot, records)?;
    let mut requirement_conflicts = serde_json::Map::new();
    let check_requirements = item_requirement_check(snapshot);
    for item in records
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid item records"))?
    {
        if let Some(id) = item["id"].as_u64().and_then(|id| u32::try_from(id).ok())
            && id > 0
        {
            if !simulator_slots(item).is_empty()
                && let Some(conflict) = check_requirements(item)
            {
                requirement_conflicts.insert(id.to_string(), conflict);
            }
        }
        for (index, source) in item["sources"].as_array().into_iter().flatten().enumerate() {
            if let Some(id) = source["quest"]["id"].as_u64().filter(|id| *id > 0) {
                let row = quests.and_then(|rows| rows.get(&id.to_string()));
                sources.push(json!({
                    "item_id":item["id"],"source_index":index+1,"quest_id":id,
                    "quest_observed":row.is_some(),
                    "ready_for_turn_in":row.and_then(|quest| quest["ready_for_turn_in"].as_bool()),
                    "failed":row.and_then(|quest| quest["failed"].as_bool()),
                    "unfinished_objectives":crate::analysis::unfinished_quest_objectives(row)
                }));
            }
            if let Some(id) = source["rep"]["repFactionId"].as_u64().filter(|id| *id > 0) {
                let row = factions.and_then(|rows| rows.get(&id.to_string()));
                // The pinned RepLevel enum uses the same 1–8 standard tiers as the client.
                let required_reaction = source["rep"]["repLevel"]
                    .as_u64()
                    .filter(|reaction| (1..=8).contains(reaction));
                let observed_reaction = row
                    .filter(|row| row["kind"] == "standard")
                    .and_then(|row| row["reaction"].as_u64())
                    .filter(|reaction| (1..=8).contains(reaction));
                let recorded_faction = source_faction(&source["rep"]["factionId"]);
                let requirement_met = required_reaction
                    .zip(observed_reaction)
                    .map(|(required, observed)| observed >= required);
                sources.push(json!({
                    "item_id":item["id"],"source_index":index+1,"faction_id":id,
                    "reputation_observed":row.is_some(),
                    "required_standard_reaction":required_reaction,
                    "observed_reputation_requirement_met":requirement_met,
                    "catalogue_source_faction":recorded_faction,
                    "observed_character_faction_matches":faction_matches(recorded_faction),
                    "standing":row.and_then(|faction| faction["standing"].as_i64()),
                    "reaction":row.and_then(|faction| faction["reaction"].as_u64()),
                    "kind":row.and_then(|faction| faction["kind"].as_str()),
                    "account_wide":row.and_then(|faction| faction["account_wide"].as_bool())
                }));
            }
            if let Some(id) = source["soldBy"]["npcId"].as_u64().filter(|id| *id > 0) {
                let recorded_faction = source_faction(&source["soldBy"]["faction"]);
                sources.push(json!({
                    "item_id":item["id"],"source_index":index+1,"vendor_npc_id":id,
                    "catalogue_source_faction":recorded_faction,
                    "observed_character_faction_matches":faction_matches(recorded_faction)
                }));
            }
            if let Some(id) = source["crafted"]["spellId"].as_u64().filter(|id| *id > 0) {
                let key = id.to_string();
                let tracked = |group: &str| {
                    tracked_recipes
                        .and_then(|section| section[group].as_object())
                        .map(|rows| rows.contains_key(&key))
                };
                let recipe = selected_recipe.filter(|row| row["recipe_id"].as_u64() == Some(id));
                let output_matches = recipe
                    .and_then(|row| row["output_item_id"].as_u64())
                    .map(|output| item["id"].as_u64() == Some(output));
                let matched_recipe = recipe.filter(|_| output_matches != Some(false));
                let review = if matched_recipe.is_some() {
                    if recipe_review.is_none() {
                        recipe_review = Some(crate::analysis::review_recipe(snapshot, 1)?);
                    }
                    recipe_review.as_ref()
                } else {
                    None
                };
                sources.push(json!({
                    "item_id":item["id"],"source_index":index+1,"recipe_spell_id":id,
                    "recipe_observed":recipe.is_some(),"output_matches":output_matches,
                    "tracked_as_craft":tracked("crafts"),"tracked_as_recraft":tracked("recrafts"),
                    "learned":matched_recipe.and_then(|row| row["learned"].as_bool()),
                    "material_capacity":review.and_then(|value| value.get("material_capacity")),
                    "unobserved_storage":review.and_then(|value| value.get("unobserved_storage")),
                    "base_output_if_completed":review.and_then(|value| value.get("base_output_if_completed")),
                    "observed_cooldown":matched_recipe.and_then(|row| row.get("cooldown")),
                    "unmet_condition_count":review.and_then(|value| value["unmet_crafting_conditions"].as_object()).map(|rows| rows.len())
                }));
            }
        }
    }
    Ok(
        json!({"captured_at":snapshot.captured_at,"quests_available":quests.is_some(),
        "reputation_available":factions.is_some(),"selected_recipe_available":selected_recipe.is_some(),
        "tracked_recipes_available":tracked_recipes.is_some(),
        "sources":sources,"owned_quantities_by_item_id":owned_quantities,
        "observed_requirement_conflicts_by_item_id":requirement_conflicts}),
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

/// Slot names from the pinned simulator's SIML format.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, serde::Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorSlot {
    Head,
    Neck,
    Shoulder,
    Back,
    Chest,
    Wrist,
    Hands,
    Waist,
    Legs,
    Feet,
    Finger1,
    Finger2,
    Trinket1,
    Trinket2,
    MainHand,
    OffHand,
    Ranged,
}

fn simulator_slots(item: &Value) -> &'static [SimulatorSlot] {
    use SimulatorSlot::*;
    match item["type"].as_u64() {
        Some(1) => &[Head],
        Some(2) => &[Neck],
        Some(3) => &[Shoulder],
        Some(4) => &[Back],
        Some(5) => &[Chest],
        Some(6) => &[Wrist],
        Some(7) => &[Hands],
        Some(8) => &[Waist],
        Some(9) => &[Legs],
        Some(10) => &[Feet],
        Some(11) => &[Finger1, Finger2],
        Some(12) => &[Trinket1, Trinket2],
        Some(13) => match item["handType"].as_u64() {
            Some(1 | 4) => &[MainHand],
            Some(2) => &[MainHand, OffHand],
            Some(3) => &[OffHand],
            _ => &[],
        },
        Some(14) => &[Ranged],
        _ => &[],
    }
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
    let regular_stats = item.and_then(|item| {
        let base = default_regular_stats(item)?;
        match modifiers["random_suffix_id"].as_i64()? {
            0 => Some(Value::Object(base)),
            _ => {
                let (allocation, _) = suffix_effects.as_ref()?;
                combine_regular_stats(base, &allocation["by_simulator_stat_id"])
            }
        }
    });
    let regular_stats_with_enchant = regular_stats.as_ref().and_then(|stats| {
        if modifiers["enchant_id"].as_u64()? == 0 {
            return Some(stats.clone());
        }
        let [enchant] = fitting_enchants.as_deref()? else {
            return None;
        };
        add_enchant_regular_stats(stats, enchant)
    });
    let simulator_item_spec = item.and_then(|item| {
        if simulator_slots(item).is_empty() || regular_stats_with_enchant.is_none() {
            return None;
        }
        let id = i32::try_from(item["id"].as_i64()?).ok()?;
        let enchant = i32::try_from(modifiers["enchant_id"].as_i64()?).ok()?;
        let suffix = i32::try_from(modifiers["random_suffix_id"].as_i64()?).ok()?;
        Some(json!({"id":id,"enchant":enchant,"randomSuffix":suffix}))
    });
    json!({
        "enchants":enchants,
        "fitting_enchants":fitting_enchants,
        "random_suffixes":suffixes,
        "simulator_item_spec":simulator_item_spec,
        "default_scaling_regular_stats_before_enchant":regular_stats,
        "default_scaling_regular_stats_with_enchant":regular_stats_with_enchant,
        "default_scaling_suffix_allocation_delta":suffix_effects.as_ref().map(|(stats, _)| stats),
        "default_scaling_suffix_extra_effects":suffix_effects.as_ref().map(|(_, extra)| extra)
    })
}

fn add_enchant_regular_stats(base: &Value, enchant: &Value) -> Option<Value> {
    let mut stats = base.as_object()?.clone();
    for (id, delta) in enchant["stats"].as_array()?.iter().enumerate() {
        let delta = delta.as_f64()?;
        if delta != 0.0 {
            let id = id.to_string();
            let base_value = stats.get(&id).map_or(Some(0.0), Value::as_f64)?;
            stats.insert(id, json!(base_value + delta));
        }
    }
    Some(Value::Object(stats))
}

fn combine_regular_stats(
    mut stats: serde_json::Map<String, Value>,
    suffix: &Value,
) -> Option<Value> {
    for (id, delta) in suffix.as_object()? {
        let base_value = stats.get(id).map_or(Some(0.0), Value::as_f64)?;
        stats.insert(id.clone(), json!(base_value + delta.as_f64()?));
    }
    Some(Value::Object(stats))
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
    fn crafted_sources_distinguish_missing_and_observed_recipe_tracking() {
        let mut snapshot: Snapshot = serde_json::from_value(json!({
            "schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
            "client":{"family":"forever","variant":"live","build":"70862","interface":16001},
            "character":{"name":"Example","realm":"Test"},"sections":{}
        }))
        .unwrap();
        let records = json!([{"id":2870,"sources":[{"crafted":{"spellId":2675}}]}]);
        let absent = acquisition_progress(&snapshot, &records).unwrap();
        assert_eq!(absent["tracked_recipes_available"], false);
        assert!(absent["sources"][0]["tracked_as_craft"].is_null());
        snapshot.sections.insert(
            "tracked_recipes".into(),
            json!({
                "coverage":"player_tracked_recipes_only",
                "crafts":{"2675":{"name":"Fixture only"}},"recrafts":{"2676":{}}
            }),
        );
        let observed = acquisition_progress(&snapshot, &records).unwrap();
        assert_eq!(observed["tracked_recipes_available"], true);
        assert_eq!(observed["sources"][0]["tracked_as_craft"], true);
        assert_eq!(observed["sources"][0]["tracked_as_recraft"], false);
        assert!(observed["sources"][0]["learned"].is_null());
        assert!(!observed.to_string().contains("Fixture only"));
    }

    #[test]
    fn crafted_sources_report_constraints_only_for_compatible_selected_recipe() {
        let snapshot: Snapshot = serde_json::from_value(json!({
            "schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
            "client":{"family":"forever","variant":"live","build":"70862","interface":16001},
            "character":{"name":"Example","realm":"Test"},"sections":{"selected_recipe":{
                "recipe_id":2675,"name":"Fixture recipe","learned":true,"output_item_id":2870,
                "requirements":"fixed_items","reagents":{},"base_output_quantity":{"minimum":2,"maximum":3},
                "cooldown":{"charges":0,"max_charges":3},"crafting_conditions":{
                    "1":{"name":"Private condition","kind":"area","met":false}
                }
            }}
        })).unwrap();
        let source = json!({"crafted":{"spellId":2675}});
        let records = json!([{"id":2870,"sources":[source]},{"id":2871,"sources":[source]}]);
        let result = acquisition_progress(&snapshot, &records).unwrap();
        let matched = &result["sources"][0];
        assert_eq!(
            matched["base_output_if_completed"],
            json!({"item_id":2870,"minimum":2,"maximum":3})
        );
        assert_eq!(
            matched["observed_cooldown"],
            json!({"charges":0,"max_charges":3})
        );
        assert_eq!(matched["unmet_condition_count"], 1);
        for field in [
            "base_output_if_completed",
            "observed_cooldown",
            "unmet_condition_count",
        ] {
            assert!(result["sources"][1][field].is_null());
        }
        assert!(!result.to_string().contains("Private condition"));
    }

    #[test]
    fn class_proficiency_checks_known_types_and_preserves_unknowns() {
        assert_eq!(
            proficiency_conflict(3, &json!({"type":5,"armorType":4})),
            Some("armor_type")
        );
        assert_eq!(
            proficiency_conflict(3, &json!({"type":13,"weaponType":4})),
            Some("weapon_type")
        );
        assert_eq!(
            proficiency_conflict(4, &json!({"type":13,"weaponType":1,"handType":4})),
            Some("two_hand_weapon_type")
        );
        assert_eq!(
            proficiency_conflict(3, &json!({"type":14,"rangedWeaponType":5})),
            Some("ranged_weapon_type")
        );
        assert_eq!(
            proficiency_conflict(3, &json!({"type":13,"weaponType":8,"handType":4})),
            None
        );
        assert_eq!(proficiency_conflict(3, &json!({"type":13})), None);
        assert_eq!(
            proficiency_conflict(0, &json!({"type":5,"armorType":4})),
            None
        );
    }

    #[test]
    fn simulator_slot_names_match_pinned_siml() {
        let slots = |item| json!(simulator_slots(&item));
        assert_eq!(slots(json!({"type":11})), json!(["finger1", "finger2"]));
        assert_eq!(slots(json!({"type":12})), json!(["trinket1", "trinket2"]));
        assert_eq!(
            slots(json!({"type":13,"handType":2})),
            json!(["main_hand", "off_hand"])
        );
        assert_eq!(slots(json!({"type":13,"handType":4})), json!(["main_hand"]));
    }

    #[test]
    fn adds_regular_enchant_stats_only_for_a_unique_fit_or_explicit_zero() {
        let db = json!({"enchants":[
            {"effectId":7,"type":5,"stats":[2,0,4]},
            {"effectId":8,"type":5,"stats":[7]},
            {"effectId":8,"type":5,"stats":[8]},
            {"effectId":9,"type":6,"stats":[10]}
        ]});
        let item = json!({"id":42,"type":5,"scalingOptions":{"0":{"stats":{"0":3}}}});
        for (enchant, expected) in [
            (json!(7), json!({"0":5.0,"2":4.0})),
            (json!(0), json!({"0":3})),
            (json!(8), Value::Null),
            (json!(9), Value::Null),
            (json!(10), Value::Null),
            (Value::Null, Value::Null),
        ] {
            let modifiers = json!({"enchant_id":enchant,"random_suffix_id":0});
            let result = modifier_candidates(&db, &modifiers, Some(&item));
            assert_eq!(
                result["default_scaling_regular_stats_with_enchant"],
                expected
            );
            if expected.is_null() {
                assert!(result["simulator_item_spec"].is_null());
            } else {
                assert_eq!(
                    result["simulator_item_spec"],
                    json!({"id":42,"enchant":enchant,"randomSuffix":0})
                );
            }
        }
    }

    #[test]
    fn combines_base_and_suffix_regular_stat_ids() {
        assert_eq!(
            combine_regular_stats(
                json!({"0":4,"2":9}).as_object().unwrap().clone(),
                &json!({"0":7,"1":3})
            ),
            Some(json!({"0":11.0,"1":3.0,"2":9}))
        );
        assert_eq!(
            combine_regular_stats(serde_json::Map::new(), &json!({"0":7})),
            Some(json!({"0":7.0}))
        );
    }

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

fn proficiency_conflict(class: u64, item: &Value) -> Option<&'static str> {
    // Pinned capabilities_auto_gen.ts: max armour, melee, two-hand melee, ranged.
    let (armour, melee, two_hand, ranged): (u64, &[u64], &[u64], &[u64]) = match class {
        1 => (
            4,
            &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            &[1, 4, 6, 8, 9],
            &[1, 2, 3, 4],
        ),
        2 => (4, &[1, 4, 5, 6, 7, 9], &[1, 4, 6, 9], &[7]),
        3 => (3, &[1, 2, 3, 6, 8, 9], &[1, 6, 8, 9], &[1, 2, 3]),
        4 => (2, &[1, 2, 3, 4, 5, 9], &[], &[1, 2, 3, 4]),
        5 => (1, &[2, 4, 5, 8], &[8], &[5]),
        7 => (3, &[1, 2, 3, 4, 5, 7, 8], &[1, 4, 8], &[8]),
        8 | 9 => (1, &[2, 5, 8, 9], &[8], &[5]),
        11 => (2, &[2, 3, 4, 5, 6, 8], &[4, 6, 8], &[6]),
        _ => return None,
    };
    match item["type"].as_u64()? {
        13 => {
            let kind = item["weaponType"].as_u64()?;
            if !(1..=9).contains(&kind) {
                return None;
            }
            if !melee.contains(&kind) {
                return Some("weapon_type");
            }
            (item["handType"] == 4 && !two_hand.contains(&kind)).then_some("two_hand_weapon_type")
        }
        14 => {
            let kind = item["rangedWeaponType"].as_u64()?;
            ((1..=9).contains(&kind) && !ranged.contains(&kind)).then_some("ranged_weapon_type")
        }
        1..=10 => {
            let kind = item["armorType"].as_u64()?;
            ((1..=4).contains(&kind) && kind > armour).then_some("armor_type")
        }
        _ => None,
    }
}

fn requirement_check(
    snapshot: &Snapshot,
) -> impl Fn(&Value, &crate::planning::EquipmentSelection) -> Option<Value> {
    use crate::planning::ItemLocation;
    let check = item_requirement_check(snapshot);
    move |item, selection| {
        if selection.source
            == (ItemLocation::Equipment {
                slot: selection.target_slot,
            })
        {
            return None;
        }
        let mut conflict = check(item)?;
        conflict["slot"] = json!(selection.target_slot);
        conflict["item_id"] = json!(selection.expected_item_id);
        Some(conflict)
    }
}

fn item_requirement_check(snapshot: &Snapshot) -> impl Fn(&Value) -> Option<Value> {
    let stats = snapshot.sections.get("stats").unwrap_or(&Value::Null);
    let level = stats["level"].as_u64().filter(|level| *level > 0);
    let faction = stats["faction"].as_str();
    // Class enum values from the pinned simulator common.proto.
    let class_id = match stats["class"].as_str() {
        Some("WARRIOR") => Some(1),
        Some("PALADIN") => Some(2),
        Some("HUNTER") => Some(3),
        Some("ROGUE") => Some(4),
        Some("PRIEST") => Some(5),
        Some("SHAMAN") => Some(7),
        Some("MAGE") => Some(8),
        Some("WARLOCK") => Some(9),
        Some("DRUID") => Some(11),
        _ => None,
    };
    let professions = snapshot
        .sections
        .get("professions")
        .and_then(Value::as_object);
    move |item| {
        let level_conflict = level
            .zip(item["reqLvl"].as_u64())
            .is_some_and(|(level, required)| level < required);
        let class_conflict =
            class_id
                .zip(item["classAllowlist"].as_array())
                .is_some_and(|(class, allowed)| {
                    !allowed.is_empty() && !allowed.iter().any(|id| id.as_u64() == Some(class))
                });
        // Reverse the pinned simulator's DBC skill-line to Profession mapping.
        let required_skill_line = match item["requiredProfession"].as_u64() {
            Some(1) => Some(171),
            Some(2) => Some(164),
            Some(3) => Some(333),
            Some(4) => Some(202),
            Some(5) => Some(182),
            Some(6) => Some(773),
            Some(8) => Some(165),
            Some(9) => Some(186),
            Some(10) => Some(393),
            Some(11) => Some(197),
            _ => None,
        };
        let missing_skill_line = required_skill_line.filter(|id| {
            professions.is_some_and(|rows| {
                !rows
                    .values()
                    .any(|row| row["skill_line"].as_u64() == Some(*id))
            })
        });
        // FactionRestriction from the pinned simulator UIItem proto.
        let required_faction = match item["factionRestriction"].as_u64() {
            Some(1) => Some("Alliance"),
            Some(2) => Some("Horde"),
            _ => None,
        };
        let faction_conflict =
            required_faction.filter(|required| faction.is_some_and(|actual| actual != *required));
        let proficiency = class_id.and_then(|class| proficiency_conflict(class, item));
        (level_conflict
            || class_conflict
            || missing_skill_line.is_some()
            || faction_conflict.is_some()
            || proficiency.is_some())
        .then(|| {
            let mut conflict = json!({
                "below_required_level":level_conflict,"class_excluded":class_conflict
            });
            if let Some(id) = missing_skill_line {
                conflict["required_profession_not_observed"] = json!(id);
            }
            if let Some(required) = faction_conflict {
                conflict["required_faction"] = json!(required);
            }
            if let Some(kind) = proficiency {
                conflict["class_proficiency_conflict"] = json!(kind);
            }
            conflict
        })
    }
}

/// Checks catalogue slot fit and two-handed pairing in a validated equipment draft.
pub fn review_equipment_catalogue(
    path: &Path,
    snapshot: &Snapshot,
    selections: &[crate::planning::EquipmentSelection],
    allow_two_hand_off_hand_clear: bool,
) -> Result<Value> {
    use crate::planning::ItemLocation;

    let equipment = snapshot
        .sections
        .get("equipment")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Equipment observation unavailable"))?;
    let selected_observation = |slot| {
        if let Some(selection) = selections
            .iter()
            .find(|selection| selection.target_slot == slot)
        {
            return selection.source.observed_in(snapshot);
        }
        if selections
            .iter()
            .any(|selection| selection.source == (ItemLocation::Equipment { slot }))
        {
            return None;
        }
        equipment.get(&slot.to_string())
    };
    let (db, _) = load(path)?;
    let items = db["items"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing catalogue items"))?;
    let find = |id| items.iter().find(|item| item["id"].as_u64() == Some(id));
    let clear_off_hand = allow_two_hand_off_hand_clear
        && selected_observation(17).is_some()
        && selected_observation(16)
            .and_then(|item| item["item_id"].as_u64())
            .and_then(find)
            .is_some_and(|item| item["handType"] == 4);
    let projected_observation = |slot| {
        if clear_off_hand && slot == 17 {
            None
        } else {
            selected_observation(slot)
        }
    };
    let projected_item = |slot| projected_observation(slot)?.get("item_id")?.as_u64();
    let pairing = match (projected_item(16), projected_item(17)) {
        (Some(main), Some(_)) => match find(main).and_then(|item| item["handType"].as_u64()) {
            Some(4) => "two_hand_conflict",
            Some(1 | 2 | 3) => "no_two_hand_conflict_observed",
            _ => "unknown",
        },
        _ => "no_two_hand_conflict_observed",
    };
    let fit: Vec<_> = selections
        .iter()
        .map(|selection| {
            let target = simulator_slot_for_equipment(selection.target_slot);
            let candidates = find(u64::from(selection.expected_item_id)).map(simulator_slots);
            let status = match (target, candidates) {
                (Some(target), Some(slots)) if !slots.is_empty() => {
                    if slots.contains(&target) {
                        "matches"
                    } else {
                        "mismatch"
                    }
                }
                _ => "unknown",
            };
            json!({"target_slot":selection.target_slot,"simulator_slot":target,"status":status})
        })
        .collect();
    let check_requirements = requirement_check(snapshot);
    let requirement_conflicts: Vec<_> = selections
        .iter()
        .filter_map(|selection| {
            check_requirements(find(u64::from(selection.expected_item_id))?, selection)
        })
        .collect();
    let mut unique_slots = BTreeMap::<u64, Vec<u32>>::new();
    let mut limit_categories = BTreeMap::<u64, Vec<Value>>::new();
    for slot in 1..=19 {
        if let Some(id) = projected_item(slot)
            && let Some(item) = find(id)
        {
            if item["unique"] == true {
                unique_slots.entry(id).or_default().push(slot);
            }
            if let Some(category) = item["limitCategory"].as_u64().filter(|id| *id != 0) {
                limit_categories
                    .entry(category)
                    .or_default()
                    .push(json!({"slot":slot,"item_id":id}));
            }
        }
    }
    let duplicates: Vec<_> = unique_slots
        .into_iter()
        .filter(|(_, slots)| slots.len() > 1)
        .map(|(id, slots)| json!({"item_id":id,"slots":slots}))
        .collect();
    let shared_categories: Vec<_> = limit_categories
        .into_iter()
        .filter(|(_, items)| items.len() > 1)
        .map(|(category, items)| json!({"limit_category":category,"items":items}))
        .collect();
    // EquipmentSpec is positional in simulator ItemSlot order. Shirt and
    // tabard have no simulator slots.
    let mut blockers: Vec<_> = selections
        .iter()
        .filter(|selection| simulator_slot_for_equipment(selection.target_slot).is_none())
        .map(|selection| json!({"slot":selection.target_slot,"item_id":selection.expected_item_id}))
        .collect();
    let mut specs = Vec::new();
    for slot in [1, 2, 3, 15, 5, 9, 10, 6, 7, 8, 11, 12, 13, 14, 16, 17, 18] {
        let Some(observed) = projected_observation(slot) else {
            specs.push(json!({}));
            continue;
        };
        let id = observed["item_id"].as_u64();
        let spec = id.and_then(find).and_then(|item| {
            let target = simulator_slot_for_equipment(slot)?;
            if !simulator_slots(item).contains(&target) {
                return None;
            }
            // The engine fills the first free paired slot and ignores empty
            // placeholders during equip. Withhold inputs it would relocate.
            match target {
                SimulatorSlot::Finger2 if projected_item(11).is_none() => return None,
                SimulatorSlot::Trinket2 if projected_item(13).is_none() => return None,
                SimulatorSlot::OffHand
                    if item["handType"] == 2
                        && item["weaponType"] != 7
                        && projected_item(16).is_none() =>
                {
                    return None;
                }
                _ => {}
            }
            let modifiers = link_modifiers(observed)?;
            let mut candidates = modifier_candidates(db, &modifiers, Some(item));
            candidates
                .as_object_mut()?
                .remove("simulator_item_spec")
                .filter(|spec| !spec.is_null())
        });
        if let Some(spec) = spec {
            specs.push(spec);
        } else {
            blockers.push(json!({"slot":slot,"item_id":id}));
        }
    }
    let equipment_spec = (blockers.is_empty()
        && requirement_conflicts.is_empty()
        && pairing != "two_hand_conflict"
        && duplicates.is_empty()
        && shared_categories.is_empty())
    .then(|| json!({"items":specs}));
    Ok(json!({
        "catalogue_two_hand_pairing":pairing,
        "catalogue_slot_fit":fit,
        "catalogue_requirement_conflicts":requirement_conflicts,
        "catalogue_unique_item_duplicates":duplicates,
        "catalogue_shared_limit_categories":shared_categories,
        "simulator_equipment_spec":equipment_spec,
        "simulator_cleared_equipment_slots": if clear_off_hand { vec![17] } else { vec![] },
        "simulator_equipment_unresolved_slots":blockers
    }))
}

fn simulator_slot_for_equipment(slot: u32) -> Option<SimulatorSlot> {
    use SimulatorSlot::*;
    // The game inventory has a shirt and tabard; SIML has neither.
    match slot {
        1 => Some(Head),
        2 => Some(Neck),
        3 => Some(Shoulder),
        5 => Some(Chest),
        6 => Some(Waist),
        7 => Some(Legs),
        8 => Some(Feet),
        9 => Some(Wrist),
        10 => Some(Hands),
        11 => Some(Finger1),
        12 => Some(Finger2),
        13 => Some(Trinket1),
        14 => Some(Trinket2),
        15 => Some(Back),
        16 => Some(MainHand),
        17 => Some(OffHand),
        18 => Some(Ranged),
        _ => None,
    }
}

pub fn review_owned(
    path: &Path,
    snapshot: &Snapshot,
    offset: usize,
    limit: usize,
    item_type: Option<GearType>,
    set_id: Option<std::num::NonZeroU32>,
    observed_item_ids: &[std::num::NonZeroU32],
    simulator_slot: Option<SimulatorSlot>,
) -> Result<Value> {
    ensure!(
        snapshot.client.family == Family::Forever,
        "Expected a Forever snapshot"
    );
    ensure!(
        (1..=50).contains(&limit),
        "Expected a limit between 1 and 50"
    );
    let item_filter: std::collections::BTreeSet<_> = observed_item_ids
        .iter()
        .map(|id| u64::from(id.get()))
        .collect();
    ensure!(
        observed_item_ids.len() <= 16 && item_filter.len() == observed_item_ids.len(),
        "Expected at most sixteen distinct item IDs"
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
    let target_slot = simulator_slot.and_then(|target| {
        (1..=19).find(|slot| simulator_slot_for_equipment(*slot) == Some(target))
    });
    let check_requirements = requirement_check(snapshot);
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
                !item_filter.is_empty() && !item_id.is_some_and(|id| item_filter.contains(&id));
            let wrong_slot = simulator_slot.is_some_and(|slot| {
                !reference.is_some_and(|item| simulator_slots(item).contains(&slot))
            });
            if wrong_type || wrong_set || wrong_item || wrong_slot {
                return;
            }
            if included >= offset && entries.len() < limit {
                let modifiers = link_modifiers(item);
                let draft_selection = target_slot
                    .and_then(|target| draft_selection(target, storage, container, slot, item_id?));
                let requirement_conflict = draft_selection
                    .as_ref()
                    .zip(reference)
                    .and_then(|(selection, item)| check_requirements(item, selection));
                entries.push(json!({"storage":storage,"container":container,"slot":slot,"observed":item,
                    "catalogue_requirement_conflict":requirement_conflict,
                    "draft_selection":draft_selection,
                    "catalogue_modifier_candidates":modifiers.as_ref().map(|fields| modifier_candidates(db, fields, reference.copied())),
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
                        "simulator_slots":simulator_slots(item),
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

fn draft_selection(
    target_slot: u32,
    storage: &str,
    container: Option<&str>,
    slot: &str,
    item_id: u64,
) -> Option<crate::planning::EquipmentSelection> {
    use crate::planning::{EquipmentSelection, ItemLocation};

    let slot = slot.parse().ok()?;
    let source = match storage {
        "equipment" => ItemLocation::Equipment { slot },
        "bags" => ItemLocation::Bag {
            bag: container?.parse().ok()?,
            slot,
        },
        "character_bank" => ItemLocation::CharacterBank {
            container: container?.parse().ok()?,
            slot,
        },
        "account_bank" => ItemLocation::AccountBank {
            container: container?.parse().ok()?,
            slot,
        },
        _ => return None,
    };
    Some(EquipmentSelection {
        target_slot,
        source,
        expected_item_id: item_id.try_into().ok()?,
    })
}
