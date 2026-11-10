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
pub enum GossipQuestList {
    Available,
    Active,
}

impl GossipQuestList {
    fn field(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Active => "active",
        }
    }
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
    /// Up to eight saved learned recipe captures for confirmed consumable restock shortages.
    pub restock_recipe_snapshot_ids: Option<Vec<String>>,
    /// Up to eight selected or exact bulk recipes for confirmed restock shortages; replaces the legacy capture-ID list.
    pub restock_recipes: Option<Vec<RestockRecipe>>,
    /// Price combined restock reagent shortfalls against captured merchant offers.
    #[serde(default)]
    pub restock_merchant_reagent_plan: bool,
    /// Optional same-character, exact-client/build offer capture; wallet remains the acquisition inventory's.
    pub restock_merchant_snapshot_id: Option<String>,
    /// Match the exact groupKey recorded by the pinned simulator for consumable buffs.
    pub consumable_group_key: Option<String>,
    /// Restrict items or enchants to a crafted source with this recipe spell ID.
    pub recipe_spell_id: Option<u32>,
    /// Restrict items or enchants to a crafted source with this pinned simulator Profession enum ID.
    pub crafted_profession_id: Option<std::num::NonZeroU32>,
    /// Restrict enchant effects to simulator fits for this exact catalogue base item ID.
    pub enchant_target_item_id: Option<std::num::NonZeroU32>,
    /// Include this listed random suffix in the target's regular-stat preview.
    pub enchant_target_random_suffix_id: Option<std::num::NonZeroI32>,
    /// Restrict enchant effects to those linked to this exact enchant application item ID.
    pub enchant_application_item_id: Option<std::num::NonZeroU32>,
    /// List random suffix records referenced by this exact base item ID.
    pub suffix_target_item_id: Option<std::num::NonZeroU32>,
    /// List items that explicitly reference this random suffix ID.
    pub item_random_suffix_id: Option<std::num::NonZeroI32>,
    /// Match crafted sources to consistent positive selected/list/bulk/tracked learning or reported known profession spells.
    #[serde(default)]
    pub only_observed_learned_recipe: bool,
    /// Restrict items to this pinned catalogue set ID.
    pub set_id: Option<std::num::NonZeroU32>,
    /// Case-insensitive substring of a recorded item set name. Items only.
    pub set_name: Option<String>,
    /// Restrict items to catalogue placements for this simulator equipment slot.
    pub simulator_slot: Option<SimulatorSlot>,
    /// Preserve any recorded set membership of the saved item in the requested slot.
    #[serde(default)]
    pub preserve_equipped_set: bool,
    /// Restrict items to at least this recorded item level at default scaling.
    pub minimum_item_level: Option<std::num::NonZeroU32>,
    /// Minimum recorded character level required to equip the item, inclusive.
    pub minimum_required_level: Option<u32>,
    /// Maximum recorded character level required to equip the item, inclusive.
    pub maximum_required_level: Option<u32>,
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
    /// Restrict quest reward sources to IDs observed in this saved gossip list; requires acquisition_snapshot_id.
    pub gossip_quest_list: Option<GossipQuestList>,
    /// Match an explicitly observed completion flag on the same gossip quest row. Requires gossip_quest_list.
    pub gossip_quest_is_complete: Option<bool>,
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
    /// Owner-scoped Forever snapshot for stock counts, item source progress or learned enchant filtering.
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
    /// Override the movement interval and distance for the Default/Movement preset.
    pub movement: Option<MovementSettings>,
    /// Override the wave timing for the Default/Dynamic Adds preset.
    pub dynamic_adds: Option<DynamicAddsSettings>,
    /// Enable custom boss attacks for the Default/Custom Boss preset.
    pub custom_boss: Option<CustomBossSettings>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MovementSettings {
    /// Seconds between moves, from 1 to 3600.
    pub interval_seconds: f64,
    /// Distance per move, from 0 to 100 yards.
    pub distance_yards: f64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DynamicAddsSettings {
    /// Seconds until the first wave, from 0 to 3600.
    pub spawn_delay_seconds: f64,
    /// Seconds each wave stays active, from 1 to 3600.
    pub lifetime_seconds: f64,
    /// Seconds between despawn and respawn, from 0 to 3600. Zero respawns immediately.
    pub respawn_seconds: f64,
    /// Chance that a despawning add counts as your kill, from 0 to 100 percent.
    pub kill_share_percent: Option<f64>,
    /// Apply configured raid debuffs to newly spawned adds.
    pub apply_raid_debuffs: Option<bool>,
    /// Seconds before configured raid debuffs land, from 0 to 3600.
    pub debuff_delay_seconds: Option<f64>,
    /// Maximum variation in the debuff delay, from 0 to 3600 seconds.
    pub debuff_jitter_seconds: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CustomBossSettings {
    pub thrash: Option<BossThrash>,
    pub cleave: Option<BossCleave>,
    pub magic_spell: Option<BossMagicSpell>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BossMagicSchool {
    Arcane,
    Fire,
    Frost,
    Holy,
    Nature,
    Shadow,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BossMagicSpell {
    pub school: BossMagicSchool,
    /// Cast time in whole seconds, from 0 to 3600.
    pub cast_seconds: u32,
    /// Cooldown in whole seconds, from 1 to 3600.
    pub cooldown_seconds: u32,
    pub minimum_damage: u32,
    pub maximum_damage: u32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BossThrash {
    /// Extra swings per proc, from 1 to 10.
    pub extra_attacks: u32,
    /// Proc chance as a percentage, from 0 to 100.
    pub proc_chance_percent: f64,
    /// Internal cooldown in whole seconds, from 1 to 3600.
    pub cooldown_seconds: u32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BossCleave {
    /// Internal cooldown in whole seconds, from 1 to 3600.
    pub cooldown_seconds: u32,
    /// Additional weapon damage as a percentage, from 0 to 1000.
    pub bonus_damage_percent: f64,
}

fn set_boolean_target_input(
    target: &mut serde_json::Map<String, Value>,
    index: usize,
    label: &str,
    value: bool,
) -> Result<()> {
    let input = target
        .get_mut("targetInputs")
        .and_then(Value::as_array_mut)
        .and_then(|inputs| inputs.get_mut(index))
        .filter(|input| {
            input["label"] == label && (input.get("inputType").is_none() || input["inputType"] == 0)
        })
        .ok_or_else(|| anyhow::anyhow!("Unexpected encounter boolean input layout"))?;
    input["boolValue"] = json!(value);
    Ok(())
}

fn set_numeric_target_inputs(
    target: &mut serde_json::Map<String, Value>,
    updates: &[(usize, &str, f64)],
) -> Result<()> {
    let inputs = target
        .get_mut("targetInputs")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| anyhow::anyhow!("Missing encounter inputs"))?;
    for &(index, label, amount) in updates {
        let input = inputs
            .get_mut(index)
            .filter(|input| input["label"] == label && input["inputType"] == 1)
            .ok_or_else(|| anyhow::anyhow!("Unexpected encounter input layout"))?;
        input["numberValue"] = json!(amount);
    }
    Ok(())
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
    let mut targets: Vec<_> = targets
        .iter()
        .map(|entry| {
            entry["target"]
                .as_object()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Invalid encounter target"))
        })
        .collect::<Result<_>>()?;
    if let Some(movement) = &settings.movement {
        ensure!(
            record["path"] == "Default/Movement"
                && (1.0..=3600.0).contains(&movement.interval_seconds)
                && (0.0..=100.0).contains(&movement.distance_yards),
            "Movement overrides require the movement preset and bounded interval and distance"
        );
        let [target] = targets.as_mut_slice() else {
            anyhow::bail!("Expected one movement target");
        };
        set_numeric_target_inputs(
            target,
            &[
                (0, "Movement Interval", movement.interval_seconds),
                (2, "Yards", movement.distance_yards),
            ],
        )?;
    }
    if let Some(adds) = &settings.dynamic_adds {
        ensure!(
            record["path"] == "Default/Dynamic Adds"
                && (0.0..=3600.0).contains(&adds.spawn_delay_seconds)
                && (1.0..=3600.0).contains(&adds.lifetime_seconds)
                && (0.0..=3600.0).contains(&adds.respawn_seconds)
                && adds
                    .kill_share_percent
                    .is_none_or(|value| (0.0..=100.0).contains(&value))
                && [adds.debuff_delay_seconds, adds.debuff_jitter_seconds]
                    .into_iter()
                    .flatten()
                    .all(|value| (0.0..=3600.0).contains(&value))
                && (adds.apply_raid_debuffs != Some(false)
                    || (adds.debuff_delay_seconds.is_none()
                        && adds.debuff_jitter_seconds.is_none())),
            "Dynamic adds overrides require the dynamic adds preset and compatible bounded settings"
        );
        let [boss, _] = targets.as_mut_slice() else {
            anyhow::bail!("Expected one boss and one dynamic add");
        };
        let mut numeric = vec![
            (0, "Add(s) respawn Time", adds.respawn_seconds),
            (1, "Add(s) lifetime", adds.lifetime_seconds),
            (2, "Add(s) spawn delay", adds.spawn_delay_seconds),
        ];
        numeric.extend(
            [
                (3, "Kill share %", adds.kill_share_percent),
                (5, "Debuff delay", adds.debuff_delay_seconds),
                (6, "Debuff jitter", adds.debuff_jitter_seconds),
            ]
            .into_iter()
            .filter_map(|(index, label, value)| Some((index, label, value?))),
        );
        set_numeric_target_inputs(boss, &numeric)?;
        if let Some(apply) = adds.apply_raid_debuffs {
            set_boolean_target_input(boss, 4, "Apply debuffs to adds", apply)?;
        }
    }
    if let Some(custom) = &settings.custom_boss {
        ensure!(
            record["path"] == "Default/Custom Boss"
                && (custom.thrash.is_some()
                    || custom.cleave.is_some()
                    || custom.magic_spell.is_some()),
            "Custom attacks require the custom boss preset and at least one attack"
        );
        let [boss] = targets.as_mut_slice() else {
            anyhow::bail!("Expected one custom boss");
        };
        if let Some(thrash) = &custom.thrash {
            ensure!(
                (1..=10).contains(&thrash.extra_attacks)
                    && (0.0..=100.0).contains(&thrash.proc_chance_percent)
                    && (1..=3600).contains(&thrash.cooldown_seconds),
                "Invalid thrash settings"
            );
            set_numeric_target_inputs(
                boss,
                &[
                    (1, "Extra attack count", f64::from(thrash.extra_attacks)),
                    (2, "Extra attack proc chance %", thrash.proc_chance_percent),
                    (
                        3,
                        "Extra attack cooldown",
                        f64::from(thrash.cooldown_seconds),
                    ),
                ],
            )?;
            set_boolean_target_input(boss, 0, "Has Extra Attacks", true)?;
        }
        if let Some(cleave) = &custom.cleave {
            ensure!(
                (1..=3600).contains(&cleave.cooldown_seconds)
                    && (0.0..=1000.0).contains(&cleave.bonus_damage_percent),
                "Invalid cleave settings"
            );
            set_numeric_target_inputs(
                boss,
                &[
                    (5, "Cleave cooldown", f64::from(cleave.cooldown_seconds)),
                    (6, "Cleave damage multiplier", cleave.bonus_damage_percent),
                ],
            )?;
            set_boolean_target_input(boss, 4, "Has Cleave", true)?;
        }
        if let Some(spell) = &custom.magic_spell {
            ensure!(
                spell.cast_seconds <= 3600
                    && (1..=3600).contains(&spell.cooldown_seconds)
                    && spell.minimum_damage <= spell.maximum_damage
                    && spell.maximum_damage <= 1_000_000,
                "Invalid custom boss magic spell settings"
            );
            set_numeric_target_inputs(
                boss,
                &[
                    (8, "Magic spell cast time", f64::from(spell.cast_seconds)),
                    (9, "Magic spell cooldown", f64::from(spell.cooldown_seconds)),
                    (
                        10,
                        "Magic spell min damage",
                        f64::from(spell.minimum_damage),
                    ),
                    (
                        11,
                        "Magic spell max damage",
                        f64::from(spell.maximum_damage),
                    ),
                ],
            )?;
            let (index, label) = match spell.school {
                BossMagicSchool::Arcane => (0, "Arcane"),
                BossMagicSchool::Fire => (1, "Fire"),
                BossMagicSchool::Frost => (2, "Frost"),
                BossMagicSchool::Holy => (3, "Holy"),
                BossMagicSchool::Nature => (4, "Nature"),
                BossMagicSchool::Shadow => (5, "Shadow"),
            };
            let school = boss
                .get_mut("targetInputs")
                .and_then(Value::as_array_mut)
                .and_then(|inputs| inputs.get_mut(12))
                .filter(|input| {
                    input["label"] == "Magic Spell School"
                        && input["inputType"] == 2
                        && input["enumOptions"][index] == label
                })
                .ok_or_else(|| anyhow::anyhow!("Unexpected magic spell school layout"))?;
            school["enumValue"] = json!(index);
            set_boolean_target_input(boss, 7, "Has Magic spell", true)?;
        }
    }
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

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RestockRecipe {
    pub snapshot_id: String,
    /// Omit for the selected schematic; otherwise select this exact bulk-observed recipe.
    pub recipe_id: Option<std::num::NonZeroU32>,
}

pub fn lookup(path: &Path, query: &Query, snapshot: Option<&Snapshot>) -> Result<Value> {
    ensure!(
        query.restock_merchant_snapshot_id.is_none() || query.restock_merchant_reagent_plan,
        "Restock merchant snapshot requires a reagent quote plan"
    );
    ensure!(
        !query.restock_merchant_reagent_plan
            || query.restock_recipes.is_some()
            || query.restock_recipe_snapshot_ids.is_some(),
        "Restock reagent quotes require restock recipes"
    );
    ensure!(
        query.restock_recipes.is_none() || query.restock_recipe_snapshot_ids.is_none(),
        "Supply one restock recipe selector list"
    );
    if let Some(recipes) = &query.restock_recipes {
        ensure!(
            query.consumable_targets.is_some() && (1..=8).contains(&recipes.len()),
            "Restock recipes require consumable targets and one to eight selectors"
        );
    }
    if let Some(ids) = &query.restock_recipe_snapshot_ids {
        ensure!(
            query.consumable_targets.is_some() && (1..=8).contains(&ids.len()),
            "Restock recipes require consumable targets and one to eight captures"
        );
    }
    let (db, provenance) = load(path)?;
    let (section, identity) = query.section.fields();
    let has_inventory_context = matches!(query.section, Section::Items | Section::Consumables);
    let rows = db[section]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing catalogue section"))?;
    ensure!(
        query.enchant_application_item_id.is_none() || matches!(query.section, Section::Enchants),
        "Enchant application item filters require enchants"
    );
    let enchant_target = if let Some(id) = query.enchant_target_item_id {
        ensure!(
            matches!(query.section, Section::Enchants),
            "Enchant target requires enchants"
        );
        let item = db["items"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|item| item["id"].as_u64() == Some(u64::from(id.get())))
            .ok_or_else(|| anyhow::anyhow!("Unknown enchant target item"))?;
        let kind = item["type"]
            .as_u64()
            .filter(|kind| (1..=14).contains(kind))
            .ok_or_else(|| anyhow::anyhow!("Unknown enchant target item type"))?;
        Some((item, kind))
    } else {
        None
    };
    let enchant_target_suffix = if let Some(id) = query.enchant_target_random_suffix_id {
        let (item, _) =
            enchant_target.ok_or_else(|| anyhow::anyhow!("Suffix requires an enchant target"))?;
        let id = i64::from(id.get());
        ensure!(
            lists_random_suffix(item, id),
            "Suffix is not listed for the enchant target"
        );
        let mut matches = db["randomSuffixes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|suffix| suffix["id"].as_i64() == Some(id));
        let suffix = matches
            .next()
            .ok_or_else(|| anyhow::anyhow!("Unknown target suffix"))?;
        ensure!(matches.next().is_none(), "Ambiguous target suffix");
        Some(suffix)
    } else {
        None
    };
    let mut records = Vec::new();
    let suffix_target = if let Some(id) = query.suffix_target_item_id {
        ensure!(
            matches!(query.section, Section::RandomSuffixes),
            "Suffix target requires random suffixes"
        );
        Some(
            db["items"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|item| item["id"].as_u64() == Some(u64::from(id.get())))
                .ok_or_else(|| anyhow::anyhow!("Unknown suffix target item"))?,
        )
    } else {
        None
    };
    let mut has_more = false;
    let key = query.key.as_deref().map(str::trim);
    if let Some(key) = key {
        ensure!(!key.is_empty() && key.len() <= 160, "Invalid catalogue key");
    }
    let set_name = query.set_name.as_deref().map(str::trim);
    if let Some(name) = set_name {
        ensure!(
            !name.is_empty() && name.len() <= 160,
            "Invalid catalogue set name"
        );
    }
    let set_name = set_name.map(str::to_lowercase);
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
    ensure!(
        query.crafted_profession_id.is_none()
            || matches!(query.section, Section::Items | Section::Enchants),
        "Crafted profession filters require items or enchants"
    );
    let learned_enchant_filter =
        query.only_observed_learned_recipe && matches!(query.section, Section::Enchants);
    let learned_snapshot = if query.only_observed_learned_recipe {
        ensure!(
            matches!(query.section, Section::Items | Section::Enchants)
                && query.acquisition_snapshot_id.is_some(),
            "Learned recipe filtering requires items or enchants and an acquisition snapshot"
        );
        let snapshot = snapshot.ok_or_else(|| anyhow::anyhow!("Missing acquisition snapshot"))?;
        ensure!(
            snapshot.client.family == Family::Forever,
            "Expected a Forever snapshot"
        );
        Some(snapshot)
    } else {
        None
    };
    let crafted_filter = query.recipe_spell_id.is_some()
        || query.crafted_profession_id.is_some()
        || query.only_observed_learned_recipe;
    let drop_label = query.drop_label.as_deref().map(str::trim);
    if let Some(label) = drop_label {
        ensure!(
            !label.is_empty() && label.len() <= 160,
            "Invalid drop label"
        );
    }
    ensure!(
        query.gossip_quest_is_complete.is_none() || query.gossip_quest_list.is_some(),
        "Gossip completion filtering requires a gossip list"
    );
    let gossip_rows = if let Some(list) = query.gossip_quest_list {
        ensure!(
            matches!(query.section, Section::Items) && query.acquisition_snapshot_id.is_some(),
            "Gossip reward filtering requires items and a saved capture"
        );
        let snapshot = snapshot
            .filter(|s| s.client.family == Family::Forever)
            .ok_or_else(|| anyhow::anyhow!("Gossip reward filtering requires Forever"))?;
        snapshot
            .sections
            .get("gossip_quests")
            .and_then(|section| section[list.field()].as_object())
    } else {
        None
    };
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
    let required_level_filter =
        query.minimum_required_level.is_some() || query.maximum_required_level.is_some();
    let matches_required_level =
        required_level_check(query.minimum_required_level, query.maximum_required_level)?;
    let inventory_filter = item_ids.is_some() || ownership.is_some();
    let item_filter = check_requirements.is_some()
        || query.set_id.is_some()
        || set_name.is_some()
        || query.simulator_slot.is_some()
        || query.minimum_item_level.is_some()
        || query.item_random_suffix_id.is_some()
        || required_level_filter
        || weapon_filter.iter().any(Option::is_some)
        || effect_filter
        || query.order_by.is_some()
        || drop_filter
        || query.reward_quest_id.is_some()
        || query.gossip_quest_list.is_some()
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
        || enchant_target.is_some()
        || suffix_target.is_some()
        || query.enchant_application_item_id.is_some()
        || crafted_filter
        || item_filter
        || inventory_filter
        || consumable_group.is_some();
    ensure!(
        !query.include_source_references || (matches!(query.section, Section::Items) && filtered),
        "Source references require a filtered item lookup"
    );
    ensure!(
        query.acquisition_snapshot_id.is_none()
            || (has_inventory_context && filtered)
            || learned_enchant_filter,
        "Snapshot context requires a filtered item or consumable lookup, or learned enchant filtering"
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
            let matches_recipe = !crafted_filter
                || row["sources"].as_array().is_some_and(|sources| {
                    sources.iter().any(|source| {
                        source["crafted"].is_object()
                            && query.recipe_spell_id.is_none_or(|recipe| {
                                source["crafted"]["spellId"].as_u64() == Some(u64::from(recipe))
                            })
                            && query.crafted_profession_id.is_none_or(|profession| {
                                source["crafted"]["profession"].as_u64()
                                    == Some(u64::from(profession.get()))
                            })
                            && (!query.only_observed_learned_recipe
                                || learned_snapshot.is_some_and(|snapshot| {
                                    source["crafted"]["spellId"].as_u64().is_some_and(|id| {
                                        let output = if learned_enchant_filter {
                                            &row["itemId"]
                                        } else {
                                            &row["id"]
                                        };
                                        recipe_learning(
                                            snapshot,
                                            id,
                                            output.as_u64().filter(|id| *id > 0),
                                        )
                                        .learned
                                            == Some(true)
                                    })
                                }))
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
                && matches_required_level(Some(row))
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
                && set_name.as_deref().is_none_or(|name| {
                    recorded_set_id(row).is_some()
                        && row["setName"]
                            .as_str()
                            .is_some_and(|recorded| recorded.to_lowercase().contains(name))
                })
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
            let matches_quest = (query.reward_quest_id.is_none()
                && query.gossip_quest_list.is_none())
                || row["sources"].as_array().is_some_and(|sources| {
                    sources.iter().any(|source| {
                        source["quest"]["id"].as_u64().is_some_and(|id| {
                            query
                                .reward_quest_id
                                .is_none_or(|wanted| id == u64::from(wanted.get()))
                                && (query.gossip_quest_list.is_none()
                                    || gossip_rows
                                        .and_then(|rows| rows.get(&id.to_string()))
                                        .is_some_and(|quest| {
                                            query.gossip_quest_is_complete.is_none_or(|wanted| {
                                                quest["is_complete"].as_bool() == Some(wanted)
                                            })
                                        }))
                        })
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
                && suffix_target.is_none_or(|item| {
                    row["id"]
                        .as_i64()
                        .is_some_and(|id| lists_random_suffix(item, id))
                })
                && query
                    .item_random_suffix_id
                    .is_none_or(|id| lists_random_suffix(row, i64::from(id.get())))
                && enchant_target.is_none_or(|(item, kind)| enchant_type_fits(row, item, kind))
                && query
                    .enchant_application_item_id
                    .is_none_or(|id| row["itemId"].as_u64() == Some(u64::from(id.get())))
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
    if let Some((item, _)) = enchant_target {
        let base_stats = default_regular_stats(item).and_then(|base| {
            if let Some(suffix) = enchant_target_suffix {
                let (allocation, _) = scaled_suffix_effects(item, suffix)?;
                combine_regular_stats(base, &allocation["by_simulator_stat_id"])
            } else {
                Some(Value::Object(base))
            }
        });
        let variants: Vec<_> = records
            .iter()
            .map(|enchant| {
                json!({
                    "effect_id": enchant["effectId"],
                    "default_scaling_regular_stats_with_enchant": base_stats.as_ref()
                        .and_then(|base| add_enchant_regular_stats(base, enchant))
                })
            })
            .collect();
        result["enchant_target_preview"] = json!({
            "item_id": item["id"], "suffix_id": query.enchant_target_random_suffix_id,
            "default_scaling_regular_stats_before_enchant": base_stats, "variants": variants
        });
    }
    if let Some(item) = suffix_target {
        let base_stats = default_regular_stats(item);
        let variants: Vec<_> = records.iter().map(|suffix| {
            let effects = scaled_suffix_effects(item, suffix);
            let regular_stats = effects.as_ref().and_then(|(allocation, _)| {
                combine_regular_stats(base_stats.clone()?, &allocation["by_simulator_stat_id"])
            });
            json!({
                "suffix_id": suffix["id"],
                "default_scaling_regular_stats_before_enchant": regular_stats,
                "default_scaling_suffix_allocation_delta": effects.as_ref().map(|(stats, _)| stats),
                "default_scaling_suffix_extra_effects": effects.as_ref().map(|(_, extra)| extra)
            })
        }).collect();
        result["suffix_target_preview"] = json!({"item_id":item["id"],"variants":variants});
    }
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
    if let Some(list) = query.gossip_quest_list {
        result["gossip_quest_filter"] = json!({"list":list.field(),"list_observed":gossip_rows.is_some(),
            "observed_quest_count":gossip_rows.map(|rows|rows.len()),"requested_is_complete":query.gossip_quest_is_complete,
            "eligibility_verified":false});
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
    let helpful = snapshot
        .sections
        .get("player_auras")
        .and_then(|section| section["groups"]["helpful"]["entries"].as_object());
    let mut effects = serde_json::Map::new();
    for record in records.as_array().unwrap() {
        let Some(item_id) = record["id"]
            .as_u64()
            .filter(|id| *id > 0 && *id <= u32::MAX as u64)
        else {
            continue;
        };
        let reference_ids = recorded_effect_ids(record);
        let matched = reference_ids
            .as_ref()
            .filter(|ids| !ids.is_empty())
            .zip(helpful)
            .map(|(ids, rows)| {
                let mut count = 0;
                let mut matching_ids = BTreeSet::new();
                for row in rows.values() {
                    if let Some(id) = row["spell_id"].as_u64().filter(|id| ids.contains(id)) {
                        count += 1;
                        matching_ids.insert(id);
                    }
                }
                (count, matching_ids)
            });
        effects.insert(item_id.to_string(), json!({
            "catalogue_effect_spell_id_count":reference_ids.as_ref().map(BTreeSet::len),
            "catalogue_effect_spell_ids":reference_ids.as_ref().map(|ids|ids.iter().take(50).collect::<Vec<_>>()),
            "catalogue_effect_spell_ids_truncated":reference_ids.as_ref().map(|ids|ids.len()>50),
            "observed_id_match":matched.as_ref().map(|(count,_)|*count>0),
            "matching_aura_row_count":matched.as_ref().map(|(count,_)|count),
            "matching_aura_spell_id_count":matched.as_ref().map(|(_,ids)|ids.len()),
            "matching_aura_spell_ids":matched.as_ref().map(|(_,ids)|ids.iter().take(50).collect::<Vec<_>>()),
            "matching_aura_spell_ids_truncated":matched.as_ref().map(|(_,ids)|ids.len()>50)
        }));
    }
    result["captured_helpful_aura_matches"] = json!({
        "helpful_list_observed":helpful.is_some(),"by_item_id":effects,
        "item_use_verified":false,"effectiveness_verified":false
    });
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

#[derive(serde::Serialize)]
struct RecipeLearning {
    selected_recipe_observed: bool,
    selected_recipe_learned: Option<bool>,
    recipe_list_observed: bool,
    recipe_list_learned: Option<bool>,
    profession_spell_reported: bool,
    bulk_recipe_observed: bool,
    bulk_material_status: Option<String>,
    bulk_recipe_learned: Option<bool>,
    tracked_craft_learned: Option<bool>,
    tracked_recraft_learned: Option<bool>,
    learning_conflict: bool,
    selected_output_matches_catalogue: Option<bool>,
    bulk_output_matches_catalogue: Option<bool>,
    learned: Option<bool>,
}

fn recipe_learning(snapshot: &Snapshot, recipe_id: u64, output_id: Option<u64>) -> RecipeLearning {
    let selected = snapshot
        .sections
        .get("selected_recipe")
        .filter(|row| row["recipe_id"].as_u64() == Some(recipe_id));
    let listed = snapshot
        .sections
        .get("profession_recipe_view")
        .and_then(|view| view["entries"].get(recipe_id.to_string()));
    let selected_learned = selected.and_then(|row| row["learned"].as_bool());
    let listed_learned = listed.and_then(|row| row["learned"].as_bool());
    let reported = snapshot
        .sections
        .get("profession_spells")
        .is_some_and(|section| section["entries"].get(recipe_id.to_string()).is_some());
    let bulk = snapshot
        .sections
        .get("profession_recipe_materials")
        .and_then(|section| section["entries"].get(recipe_id.to_string()));
    let bulk_status = bulk.and_then(|row| row["status"].as_str());
    let bulk_learned = matches!(
        bulk_status,
        Some("fixed_items" | "unsupported_required_slots")
    )
    .then_some(true);
    let tracked = |group: &str| {
        snapshot
            .sections
            .get("tracked_recipes")
            .and_then(|section| section[group].get(recipe_id.to_string()))
            .and_then(|row| row["learned"].as_bool())
    };
    let tracked_craft = tracked("crafts");
    let tracked_recraft = tracked("recrafts");
    let output_matches = selected
        .and_then(|row| row["output_item_id"].as_u64())
        .zip(output_id)
        .map(|(observed, expected)| observed == expected);
    let bulk_output_matches = bulk
        .and_then(|row| row["output_item_id"].as_u64())
        .zip(output_id)
        .map(|(observed, expected)| observed == expected);
    let flags = [
        selected_learned,
        listed_learned,
        reported.then_some(true),
        bulk_learned,
        tracked_craft,
        tracked_recraft,
    ];
    let conflict = flags.contains(&Some(true)) && flags.contains(&Some(false));
    RecipeLearning {
        selected_recipe_observed: selected.is_some(),
        selected_recipe_learned: selected_learned,
        recipe_list_observed: listed.is_some(),
        recipe_list_learned: listed_learned,
        profession_spell_reported: reported,
        bulk_recipe_observed: bulk.is_some(),
        bulk_material_status: bulk_status.map(str::to_owned),
        bulk_recipe_learned: bulk_learned,
        tracked_craft_learned: tracked_craft,
        tracked_recraft_learned: tracked_recraft,
        learning_conflict: conflict,
        selected_output_matches_catalogue: output_matches,
        bulk_output_matches_catalogue: bulk_output_matches,
        learned: if conflict || output_matches == Some(false) || bulk_output_matches == Some(false)
        {
            None
        } else {
            flags.into_iter().flatten().next()
        },
    }
}

pub fn acquisition_progress(snapshot: &Snapshot, records: &Value) -> Result<Value> {
    ensure!(
        snapshot.client.family == Family::Forever,
        "Expected a Forever snapshot"
    );
    let quests = snapshot.sections.get("quests").and_then(Value::as_object);
    let gossip_available = snapshot
        .sections
        .get("gossip_quests")
        .and_then(|section| section["available"].as_object());
    let gossip_active = snapshot
        .sections
        .get("gossip_quests")
        .and_then(|section| section["active"].as_object());
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
    let mut bulk_recipe_reviews = BTreeMap::<u64, Value>::new();
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
                    "gossip_available_row_observed":gossip_available.map(|rows|rows.contains_key(&id.to_string())),
                    "gossip_active_row_observed":gossip_active.map(|rows|rows.contains_key(&id.to_string())),
                    "gossip_available_is_complete":gossip_available.and_then(|rows|rows.get(&id.to_string())).and_then(|quest|quest["is_complete"].as_bool()),
                    "gossip_active_is_complete":gossip_active.and_then(|rows|rows.get(&id.to_string())).and_then(|quest|quest["is_complete"].as_bool()),
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
                let learning = recipe_learning(snapshot, id, item["id"].as_u64());
                let bulk_recipe_review = snapshot.sections.get("profession_recipe_materials")
                    .and_then(|section|section["entries"].get(&key)).map(|row| {
                        if learning.learning_conflict {
                            return json!({"status":"learning_conflict","review":Value::Null});
                        }
                        if learning.selected_output_matches_catalogue == Some(false)
                            || learning.bulk_output_matches_catalogue == Some(false) {
                            return json!({"status":"catalogue_output_mismatch","review":Value::Null});
                        }
                        if !matches!(row["status"].as_str(), Some("fixed_items" | "unsupported_required_slots")) {
                            return json!({"status":row["status"],"review":Value::Null});
                        }
                        if learning.learned != Some(true) {
                            return json!({"status":"review_unavailable","review":Value::Null});
                        }
                        bulk_recipe_reviews.entry(id).or_insert_with(|| {
                            let recipe_id = u32::try_from(id).ok().and_then(std::num::NonZeroU32::new);
                            let result = recipe_id.ok_or_else(||anyhow::anyhow!("Recipe ID unavailable"))
                                .and_then(|id|crate::analysis::plan_recipe_by_id(snapshot,Some(id),Some(1),None,None));
                            match result {
                                Ok(review) => json!({"status":"reviewed","review":review}),
                                Err(_) => json!({"status":"review_unavailable","review":Value::Null}),
                            }
                        }).clone()
                    });
                sources.push(json!({
                    "item_id":item["id"],"source_index":index+1,"recipe_spell_id":id,
                    "recipe_learning":learning,"bulk_recipe_review":bulk_recipe_review,
                    "recipe_observed":recipe.is_some(),"output_matches":output_matches,
                    "tracked_as_craft":tracked("crafts"),"tracked_as_recraft":tracked("recrafts"),
                    "learned":matched_recipe.and_then(|row| row["learned"].as_bool()),
                    "material_capacity":review.and_then(|value| value.get("material_capacity")),
                    "material_capacity_basis":review.and_then(|value| value.get("material_capacity_basis")),
                    "material_and_charge_capacity":review.and_then(|value| value.get("material_and_charge_capacity")),
                    "captured_cooldown_state":review.and_then(|value| value.get("captured_cooldown_state")),
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
        "gossip_available_list_observed":gossip_available.is_some(),"gossip_active_list_observed":gossip_active.is_some(),
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

fn lists_random_suffix(item: &Value, id: i64) -> bool {
    item["randomSuffixOptions"]
        .as_array()
        .is_some_and(|options| options.iter().any(|option| option.as_i64() == Some(id)))
}

fn scaled_suffix_effects(item: &Value, suffix: &Value) -> Option<(Value, Value)> {
    if !lists_random_suffix(item, suffix["id"].as_i64()?) {
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
    #[test]
    fn chain_source_references_bound_and_validate_leaves() {
        let db: &'static serde_json::Value = Box::leak(Box::new(serde_json::json!({
            "items":[{"id":1,"name":"Public","sources":(1..=10).map(|id|serde_json::json!({"soldBy":{"npcId":id}})).collect::<Vec<_>>()},
                {"id":2,"sources":[]},{"id":3}],
            "npcs":[{"id":1,"name":"One"},{"id":9,"name":"Nine"}]
        })));
        let items = Box::leak(Box::new(
            db["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| (row["id"].as_u64().unwrap(), row))
                .collect(),
        ));
        let catalogue = super::EquipmentCatalogue { db, items };
        let chain = serde_json::json!({"draft_complete":true,"external_material_requirements":[
            {"item_id":1,"quantity":u64::MAX},{"item_id":2,"quantity":2},
            {"item_id":3,"quantity":3},{"item_id":4,"quantity":4}]});
        let result = catalogue.reagent_chain_sources(&chain).unwrap();
        assert_eq!(result["matched_item_count"], 3);
        assert_eq!(result["unmatched_item_count"], 1);
        assert_eq!(result["items_with_recorded_sources"], 1);
        let rows = result["records"].as_array().unwrap();
        assert_eq!(rows[0]["quantity"], u64::MAX);
        assert_eq!(rows[0]["source_count"], 10);
        assert_eq!(rows[0]["sources"].as_array().unwrap().len(), 8);
        assert_eq!(rows[0]["sources_truncated"], true);
        assert_eq!(rows[0]["source_references"].as_array().unwrap().len(), 8);
        assert!(!rows[0].to_string().contains("Nine"));
        assert_eq!(rows[1]["status"], "no_sources_recorded");
        assert_eq!(rows[1]["sources"], serde_json::json!([]));
        assert_eq!(rows[2]["status"], "source_metadata_missing");
        assert!(rows[2]["sources"].is_null());
        assert_eq!(rows[3]["status"], "item_not_in_catalogue");
        assert!(
            catalogue
                .reagent_chain_sources(&serde_json::json!({"draft_complete":false}))
                .unwrap()
                .is_null()
        );
        for leaves in [
            serde_json::json!([{"item_id":0,"quantity":1}]),
            serde_json::json!([{"item_id":1,"quantity":0}]),
            serde_json::json!([{"item_id":1,"quantity":1},{"item_id":1,"quantity":2}]),
            serde_json::json!([{"item_id":4294967296u64,"quantity":1}]),
            serde_json::json!([{"item_id":1,"quantity":true}]),
            serde_json::json!(vec![serde_json::json!({"item_id":1,"quantity":1}); 513]),
        ] {
            assert!(
                catalogue
                    .reagent_chain_sources(&serde_json::json!({"draft_complete":true,
                "external_material_requirements":leaves}))
                    .is_err()
            );
        }
    }
    use super::*;

    #[test]
    fn consumable_aura_matches_preserve_unknown_references_and_bound_full_counts() {
        let rows: serde_json::Map<_, _> = (1..=60)
            .map(|i| (i.to_string(), json!({"spell_id":1000+i})))
            .chain(std::iter::once(("61".into(), json!({"spell_id":1001}))))
            .collect();
        let snapshot:Snapshot=serde_json::from_value(json!({
            "schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
            "client":{"family":"forever","variant":"live","build":"70862","interface":16001},
            "character":{"name":"Example","realm":"Test"},"sections":{"player_auras":{
                "coverage":"readable_nonprivate_player_aura_lists","groups":{"helpful":{"entries":rows}}}}
        })).unwrap();
        snapshot.validate().unwrap();
        let mut effects: Vec<_> = (1001..=1059).map(|id| json!({"buffId":id})).collect();
        effects.push(json!({"stackingAura":{"buffId":1060}}));
        effects.push(json!({"buffId":1001}));
        let records = json!([{"id":1},{"id":2,"itemEffects":{}},{"id":3,"itemEffects":effects},
            {"id":4,"itemEffects":[{"buffId":1001}]}]);
        let result = consumable_inventory(&snapshot, &records, None).unwrap();
        let observations = &result["captured_helpful_aura_matches"];
        let items = &observations["by_item_id"];
        assert_eq!(items["1"]["catalogue_effect_spell_ids"], json!([]));
        assert!(items["1"]["observed_id_match"].is_null());
        assert!(items["2"]["catalogue_effect_spell_ids"].is_null());
        assert!(items["2"]["observed_id_match"].is_null());
        assert_eq!(items["3"]["catalogue_effect_spell_id_count"], 60);
        assert_eq!(items["3"]["matching_aura_row_count"], 61);
        assert_eq!(items["3"]["matching_aura_spell_id_count"], 60);
        assert_eq!(
            items["3"]["matching_aura_spell_ids"],
            json!((1001..=1050).collect::<Vec<_>>())
        );
        assert_eq!(items["3"]["catalogue_effect_spell_ids_truncated"], true);
        assert_eq!(items["3"]["matching_aura_spell_ids_truncated"], true);
        assert_eq!(items["4"]["matching_aura_row_count"], 2);
        assert_eq!(observations["item_use_verified"], false);
        assert_eq!(observations["effectiveness_verified"], false);
    }

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

fn native_slot_category_conflict(item: &Value, target: u32) -> Option<Value> {
    let category = crate::planning::native_slot_category(item, target);
    (category["target_slot_category_matches"]==false).then(||json!({"slot":target,"item_id":item["item_id"],
        "observed_equip_location":category["observed_equip_location"],"candidate_equipment_slots":category["candidate_equipment_slots"]}))
}

fn native_level_shortfall(
    snapshot: &Snapshot,
    selection: &crate::planning::EquipmentSelection,
) -> Option<Value> {
    use crate::planning::ItemLocation;
    if selection.source
        == (ItemLocation::Equipment {
            slot: selection.target_slot,
        })
    {
        return None;
    }
    let observed = snapshot.sections.get("stats")?["level"]
        .as_u64()
        .filter(|level| *level > 0)?;
    let required = selection.source.observed_in(snapshot)?["required_level"].as_u64()?;
    (observed < required).then(|| {
        json!({"slot":selection.target_slot,"item_id":selection.expected_item_id,
        "observed_level":observed,"required_level":required,"levels_remaining":required-observed})
    })
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
        let level_shortfall = level
            .zip(item["reqLvl"].as_u64())
            .filter(|(level, required)| level < required);
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
        (level_shortfall.is_some()
            || class_conflict
            || missing_skill_line.is_some()
            || faction_conflict.is_some()
            || proficiency.is_some())
        .then(|| {
            let mut conflict = json!({
                "below_required_level":level_shortfall.is_some(),"class_excluded":class_conflict
            });
            if let Some((observed, required)) = level_shortfall {
                conflict["level_shortfall"] = json!({
                    "observed_level": observed,
                    "required_level": required,
                    "levels_remaining": required - observed
                });
            }
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

/// A verified catalogue for reviewing one request's equipment drafts.
pub struct EquipmentCatalogue {
    db: &'static Value,
    items: &'static BTreeMap<u64, &'static Value>,
}

impl EquipmentCatalogue {
    pub fn load(path: &Path) -> Result<Self> {
        static ITEMS: OnceLock<BTreeMap<u64, &'static Value>> = OnceLock::new();
        let (db, _) = load(path)?;
        let rows = db["items"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing catalogue items"))?;
        let items = ITEMS.get_or_init(|| {
            let mut index = BTreeMap::new();
            for row in rows {
                if let Some(id) = row["id"].as_u64() {
                    index.entry(id).or_insert(row);
                }
            }
            index
        });
        Ok(Self { db, items })
    }

    /// Public reference data for already-net chain leaves, independent of owned stock.
    pub fn reagent_chain_sources(&self, chain: &Value) -> Result<Value> {
        if chain["draft_complete"] != true {
            return Ok(Value::Null);
        }
        let leaves = chain["external_material_requirements"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing chain external materials"))?;
        ensure!(leaves.len() <= 512, "Too many chain external materials");
        let mut seen = BTreeMap::new();
        for leaf in leaves {
            let id = leaf["item_id"]
                .as_u64()
                .filter(|n| *n > 0 && *n <= u64::from(u32::MAX))
                .ok_or_else(|| anyhow::anyhow!("Invalid chain material ID"))?;
            let quantity = leaf["quantity"]
                .as_u64()
                .filter(|n| *n > 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid chain material quantity"))?;
            ensure!(
                seen.insert(id, quantity).is_none(),
                "Duplicate chain material"
            );
        }
        self.material_source_records(seen, "chain_external_material_shortfalls")
    }

    pub fn quest_inventory_sources(&self, review: &Value) -> Result<Value> {
        let mut report =
            self.inventory_shortfall_sources(review, "user_supplied_quest_item_shortfalls")?;
        if report.is_object() {
            report["quest_item_binding_verified"] = json!(false);
        }
        Ok(report)
    }

    pub fn inventory_shortfall_sources(&self, review: &Value, basis: &str) -> Result<Value> {
        if review["storage_observed"] != true {
            return Ok(Value::Null);
        }
        let needs = review["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| {
                let quantity = row["shortfall_vs_observed"].as_u64().filter(|n| *n > 0)?;
                Some((row["item_id"].as_u64()?, quantity))
            })
            .collect();
        self.material_source_records(needs, basis)
    }

    fn material_source_records(&self, seen: BTreeMap<u64, u64>, basis: &str) -> Result<Value> {
        let mut matched = 0;
        let mut with_sources = 0;
        let records = seen
            .into_iter()
            .map(|(id, quantity)| {
                let item = self.items.get(&id).copied();
                let sources = item.and_then(|item| item["sources"].as_array());
                if item.is_some() {
                    matched += 1;
                }
                if sources.is_some_and(|rows| !rows.is_empty()) {
                    with_sources += 1;
                }
                let retained = sources.map(|rows| rows.iter().take(8).cloned().collect::<Vec<_>>());
                let references = retained.as_ref().map(|rows| {
                    let projection = json!({"id":id,"sources":rows});
                    source_references(self.db, &[&projection])
                });
                json!({"item_id":id,"quantity":quantity,
                "status":if item.is_none() { "item_not_in_catalogue" }
                    else if sources.is_none() { "source_metadata_missing" }
                    else if sources.unwrap().is_empty() { "no_sources_recorded" }
                    else { "sources_recorded" },
                "name":item.map(|row| &row["name"]),
                "sources":retained,"source_count":sources.map(Vec::len),
                "returned_source_count":sources.map(|rows| rows.len().min(8)),
                "sources_truncated":sources.map(|rows| rows.len() > 8),
                "source_references":references})
            })
            .collect::<Vec<_>>();
        let provenance: Value = serde_json::from_str(PIN)?;
        Ok(json!({"basis":basis,"catalogue_source":provenance,
            "external_item_count":records.len(),"matched_item_count":matched,
            "unmatched_item_count":records.len()-matched,"items_with_recorded_sources":with_sources,
            "source_limit_per_item":8,"records":records,
            "complete_world_sources_verified":false,"current_availability_verified":false,
            "acquisition_eligibility_verified":false,"ownership_verified":false,
            "live":false,"game_execution":false}))
    }

    pub fn quest_item_references(&self, quest: &Value, snapshot: &Snapshot) -> Value {
        let equipment = snapshot
            .sections
            .get("equipment")
            .and_then(Value::as_object);
        let equipped_slots = |item: &Value| {
            let reward_level = default_item_level(item);
            equipment.map(|equipment| {
                (1..=19)
                    .filter_map(|index| {
                        let slot = simulator_slot_for_equipment(index)?;
                        if !simulator_slots(item).contains(&slot) {
                            return None;
                        }
                        let id = equipment
                            .get(&index.to_string())
                            .and_then(|row| row["item_id"].as_u64());
                        let equipped = id.and_then(|id| self.items.get(&id)).copied();
                        let level = equipped.and_then(default_item_level);
                        let delta = reward_level
                            .zip(level)
                            .map(|(reward, equipped)| reward - equipped);
                        Some(
                            json!({"equipment_slot":index,"simulator_slot":slot,"item_id":id,
                        "name":equipped.map(|row| &row["name"]),"default_item_level":level,
                        "reward_item_level_delta":delta}),
                        )
                    })
                    .collect::<Vec<_>>()
            })
        };
        let references = |field: &str| {
            quest[field].as_object().map(|rows| {
                rows.iter()
                    .map(|(position, row)| {
                        let item = row["item_id"].as_u64().and_then(|id| self.items.get(&id));
                        (position.clone(), item.map(|item| json!({
                        "item_id":item["id"],"name":item["name"],"required_level":item["reqLvl"],
                        "default_item_level":default_item_level(item),
                        "simulator_slots":simulator_slots(item),"equipped_slots":equipped_slots(item)
                    })))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
        };
        json!({"item_rewards":references("item_rewards"),"item_choices":references("item_choices"),
            "spell_recipe_references":self.quest_spell_recipe_references(quest, snapshot)})
    }

    fn quest_spell_recipe_references(&self, quest: &Value, snapshot: &Snapshot) -> Value {
        let Some(rows) = quest["spell_rewards"].as_object() else {
            return Value::Null;
        };
        let mut matches = rows
            .values()
            .filter_map(|row| row["spell_id"].as_u64())
            .map(|id| (id, (0usize, Vec::new())))
            .collect::<BTreeMap<_, _>>();
        if !matches.is_empty() {
            for (section, identity) in [("items", "id"), ("enchants", "effectId")] {
                for record in self.db[section].as_array().into_iter().flatten() {
                    let recipe_ids = record["sources"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|source| source["crafted"]["spellId"].as_u64())
                        .collect::<std::collections::BTreeSet<_>>();
                    for id in recipe_ids {
                        if let Some((count, references)) = matches.get_mut(&id) {
                            *count += 1;
                            if references.len() < 8 {
                                references.push(json!({"section":section,"id":record[identity],"name":record["name"]}));
                            }
                        }
                    }
                }
            }
        }
        json!(
            rows.iter()
                .filter_map(|(position, row)| {
                    let id = row["spell_id"].as_u64()?;
                    let (count, references) = matches.get(&id)?;
                    let selected = snapshot.sections.get("selected_recipe")
                        .filter(|recipe| recipe["recipe_id"].as_u64() == Some(id));
                    let tracking = crate::analysis::recipe_tracking(snapshot, id);
                    let tracked = tracking["tracked_as_craft"].as_bool() == Some(true)
                        || tracking["tracked_as_recraft"].as_bool() == Some(true);
                    let observed_recipe = (*count > 0 || selected.is_some() || tracked).then(|| json!({
                        "selected_recipe_learned":selected.and_then(|recipe| recipe["learned"].as_bool()),
                        "tracking":tracking
                    }));
                    Some((
                        position,
                        json!({"spell_id":id,"match_count":count,"references":references,"observed_recipe":observed_recipe}),
                    ))
                })
                .collect::<BTreeMap<_, _>>()
        )
    }
}

/// Checks catalogue slot fit and two-handed pairing in a validated equipment draft.
pub fn review_equipment_catalogue(
    catalogue: &EquipmentCatalogue,
    snapshot: &Snapshot,
    selections: &[crate::planning::EquipmentSelection],
    allow_two_hand_off_hand_clear: bool,
) -> Result<Value> {
    review_equipment_catalogue_with_layout(
        catalogue,
        snapshot,
        selections,
        allow_two_hand_off_hand_clear,
        false,
    )
}

pub fn review_equipment_catalogue_with_layout(
    catalogue: &EquipmentCatalogue,
    snapshot: &Snapshot,
    selections: &[crate::planning::EquipmentSelection],
    allow_two_hand_off_hand_clear: bool,
    positional: bool,
) -> Result<Value> {
    review_equipment_catalogue_with_clears(
        catalogue,
        snapshot,
        selections,
        allow_two_hand_off_hand_clear,
        positional,
        &[],
    )
}

pub fn review_equipment_catalogue_with_clears(
    catalogue: &EquipmentCatalogue,
    snapshot: &Snapshot,
    selections: &[crate::planning::EquipmentSelection],
    allow_two_hand_off_hand_clear: bool,
    positional: bool,
    clear_slots: &[u32],
) -> Result<Value> {
    use crate::planning::ItemLocation;

    let equipment = snapshot
        .sections
        .get("equipment")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Equipment observation unavailable"))?;
    let clears: BTreeSet<_> = clear_slots.iter().copied().collect();
    ensure!(
        clear_slots.is_empty() || (snapshot.client.family == Family::Forever && positional),
        "Explicit clears require positional Forever equipment"
    );
    ensure!(
        clear_slots.len() <= 17
            && clears.len() == clear_slots.len()
            && clears
                .iter()
                .all(|slot| simulator_slot_for_equipment(*slot).is_some()
                    && equipment.contains_key(&slot.to_string())
                    && !selections
                        .iter()
                        .any(|selection| selection.target_slot == *slot)),
        "Invalid, duplicate, unobserved or conflicting clear slot"
    );
    let selected_observation = |slot| {
        if clears.contains(&slot) {
            return None;
        }
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
    let db = catalogue.db;
    let find = |id| catalogue.items.get(&id).copied();
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
    let collect_set_pieces = |ids: Vec<u64>| {
        let mut sets = BTreeMap::<u64, BTreeSet<u64>>::new();
        let mut unmatched = 0;
        for id in ids {
            if let Some(item) = find(id) {
                if let Some(set_id) = recorded_set_id(item) {
                    sets.entry(set_id).or_default().insert(id);
                }
            } else {
                unmatched += 1;
            }
        }
        (sets, unmatched)
    };
    let (before_sets, before_unmatched) = collect_set_pieces(
        equipment
            .values()
            .filter_map(|item| item["item_id"].as_u64())
            .collect(),
    );
    let (after_sets, after_unmatched) =
        collect_set_pieces((1..=19).filter_map(projected_item).collect());
    let set_changes: Vec<_> = before_sets
        .keys()
        .chain(after_sets.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|id| {
            let before = before_sets.get(id);
            let after = after_sets.get(id);
            (before != after).then(|| {
                json!({"set_id":id,
                "before_distinct_item_ids":before.cloned().unwrap_or_default(),
                "projected_distinct_item_ids":after.cloned().unwrap_or_default()})
            })
        })
        .collect();
    let native_slot_conflicts: Vec<_> = (1..=19)
        .filter(|slot| {
            snapshot.client.family == Family::Forever
                && simulator_slot_for_equipment(*slot).is_some()
        })
        .filter_map(|slot| native_slot_category_conflict(projected_observation(slot)?, slot))
        .collect();
    let projected_weapon_skills: Vec<_> = [16, 17, 18]
        .into_iter()
        .filter_map(|slot| {
            let id = projected_item(slot)?;
            let item = find(id)?;
            let target = simulator_slot_for_equipment(slot)?;
            if !simulator_slots(item).contains(&target) {
                return None;
            }
            let skill = weapon_skill(snapshot, item)?;
            Some(json!({"slot":slot,"item_id":id,"weapon_skill":skill}))
        })
        .collect();
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
    let native_shortfalls: Vec<_> = selections
        .iter()
        .filter_map(|selection| native_level_shortfall(snapshot, selection))
        .collect();
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
                SimulatorSlot::Finger2 if !positional && projected_item(11).is_none() => {
                    return None;
                }
                SimulatorSlot::Trinket2 if !positional && projected_item(13).is_none() => {
                    return None;
                }
                SimulatorSlot::OffHand
                    if !positional
                        && item["handType"] == 2
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
        && native_slot_conflicts.is_empty()
        && native_shortfalls.is_empty()
        && requirement_conflicts.is_empty()
        && pairing != "two_hand_conflict"
        && duplicates.is_empty()
        && shared_categories.is_empty())
    .then(|| {
        let mut spec = json!({"items":specs});
        if positional {
            spec["preserveSlots"] = json!(true);
        }
        spec
    });
    let mut result = json!({
        "catalogue_two_hand_pairing":pairing,
        "catalogue_slot_fit":fit,
        "catalogue_projected_weapon_skills":projected_weapon_skills,
        "catalogue_projected_set_changes":{
            "changes":set_changes,
            "before_uncatalogued_equipped_slots":before_unmatched,
            "projected_uncatalogued_equipped_slots":after_unmatched,
            "native_set_bonuses_verified":false
        },
        "catalogue_requirement_conflicts":requirement_conflicts,
        "native_required_level_shortfalls":native_shortfalls,
        "native_slot_category_conflicts":native_slot_conflicts,
        "catalogue_unique_item_duplicates":duplicates,
        "catalogue_shared_limit_categories":shared_categories,
        "simulator_equipment_spec":equipment_spec,
        "simulator_cleared_equipment_slots": clears.iter().copied().chain(clear_off_hand.then_some(17)).collect::<BTreeSet<_>>(),
        "simulator_equipment_unresolved_slots":blockers
    });
    if positional {
        result["simulator_equipment_contract"] = json!("azeroth_equipment_preserve_slots_v1");
    }
    Ok(result)
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

fn weapon_skill_line_id(item: &Value) -> Option<u32> {
    // The pinned simulator weapon enums and Forever SkillsFrame use different IDs.
    match item["type"].as_u64()? {
        13 => match (item["weaponType"].as_u64()?, item["handType"].as_u64()) {
            (1, Some(1..=3)) => Some(44),
            (1, Some(4)) => Some(172),
            (2, _) => Some(173),
            (3, _) => Some(162),
            (4, Some(1..=3)) => Some(54),
            (4, Some(4)) => Some(160),
            (6, _) => Some(229),
            (8, _) => Some(136),
            (9, Some(1..=3)) => Some(43),
            (9, Some(4)) => Some(55),
            _ => None,
        },
        14 => match item["rangedWeaponType"].as_u64()? {
            1 => Some(45),
            2 => Some(226),
            3 => Some(46),
            4 => Some(176),
            5 => Some(228),
            _ => None,
        },
        _ => None,
    }
}

fn weapon_skill(snapshot: &Snapshot, item: &Value) -> Option<Value> {
    let id = weapon_skill_line_id(item)?;
    let observed = snapshot
        .sections
        .get("skill_lines")
        .and_then(|lines| lines.get(id.to_string()));
    let points_to_cap =
        observed.and_then(|row| row["max_rank"].as_u64()?.checked_sub(row["rank"].as_u64()?));
    Some(json!({"skill_line_id":id,"observed":observed,"base_points_to_cap":points_to_cap}))
}

fn required_level_check(
    minimum: Option<u32>,
    maximum: Option<u32>,
) -> Result<impl Fn(Option<&Value>) -> bool> {
    ensure!(
        minimum.zip(maximum).is_none_or(|(min, max)| min <= max),
        "Expected an ordered required character level range"
    );
    Ok(move |item: Option<&Value>| {
        (minimum.is_none() && maximum.is_none())
            || item
                .and_then(|item| item["reqLvl"].as_u64())
                .is_some_and(|level| {
                    minimum.is_none_or(|min| level >= u64::from(min))
                        && maximum.is_none_or(|max| level <= u64::from(max))
                })
    })
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GearStorage {
    Equipment,
    Bags,
    CharacterBank,
    AccountBank,
}

impl GearStorage {
    fn section(self) -> &'static str {
        match self {
            Self::Equipment => "equipment",
            Self::Bags => "bags",
            Self::CharacterBank => "character_bank",
            Self::AccountBank => "account_bank",
        }
    }
}

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Deserialize,
    serde::Serialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorItemResolution {
    CatalogueItemUnavailable,
    ItemLinkUnresolved,
    ModifierValuesUnavailable,
    Resolved,
    SuffixReferenceUnresolved,
    EnchantReferenceUnresolved,
    SimulatorItemUnresolved,
}

fn item_resolution(
    reference: Option<&&Value>,
    modifiers: Option<&Value>,
    candidates: Option<&Value>,
) -> SimulatorItemResolution {
    match (reference, modifiers, candidates) {
        (None, _, _) => SimulatorItemResolution::CatalogueItemUnavailable,
        (_, None, _) => SimulatorItemResolution::ItemLinkUnresolved,
        (_, Some(fields), _)
            if fields["enchant_id"].is_null() || fields["random_suffix_id"].is_null() =>
        {
            SimulatorItemResolution::ModifierValuesUnavailable
        }
        (_, _, Some(review)) if !review["simulator_item_spec"].is_null() => {
            SimulatorItemResolution::Resolved
        }
        (_, Some(fields), Some(review))
            if fields["random_suffix_id"]
                .as_i64()
                .is_some_and(|id| id != 0)
                && review["default_scaling_suffix_allocation_delta"].is_null() =>
        {
            SimulatorItemResolution::SuffixReferenceUnresolved
        }
        (_, Some(fields), Some(review))
            if fields["enchant_id"].as_u64().is_some_and(|id| id != 0)
                && review["fitting_enchants"]
                    .as_array()
                    .is_some_and(|rows| rows.len() != 1) =>
        {
            SimulatorItemResolution::EnchantReferenceUnresolved
        }
        _ => SimulatorItemResolution::SimulatorItemUnresolved,
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
    required_levels: (Option<u32>, Option<u32>),
    observed_enchant_effect_id: Option<std::num::NonZeroU32>,
    observed_random_suffix_id: Option<i32>,
    simulator_input_resolved: Option<bool>,
    catalogue_matched: Option<bool>,
    resolution_filter: Option<SimulatorItemResolution>,
    include_resolution_counts: bool,
    exclude_observed_requirement_conflicts: bool,
    storage_filter: Option<GearStorage>,
    container_filter: Option<u8>,
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
    let matches_required_level = required_level_check(required_levels.0, required_levels.1)?;
    ensure!(
        container_filter
            .is_none_or(|id| id <= 64 && !matches!(storage_filter, Some(GearStorage::Equipment))),
        "Expected a bag or bank container ID from 0 to 64"
    );
    let container_filter = container_filter.map(|id| id.to_string());
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
    let mut unmatched_ids = std::collections::BTreeSet::new();
    let mut observed_storage = Vec::new();
    let mut unavailable_storage = Vec::new();
    let mut included = 0usize;
    let mut resolution_counts = BTreeMap::<SimulatorItemResolution, usize>::new();
    let mut resolution_counts_by_storage =
        BTreeMap::<&str, BTreeMap<SimulatorItemResolution, usize>>::new();
    let mut set_locations = BTreeMap::<u64, BTreeMap<&str, usize>>::new();
    let target_slot = simulator_slot.and_then(|target| {
        (1..=19).find(|slot| simulator_slot_for_equipment(*slot) == Some(target))
    });
    let check_requirements = requirement_check(snapshot);
    let check_item_requirements = item_requirement_check(snapshot);
    let durability = snapshot
        .sections
        .get("equipment_durability")
        .and_then(|section| section["entries"].as_object());
    for storage in ["equipment", "bags", "character_bank", "account_bank"] {
        let Some(section) = snapshot.sections.get(storage).and_then(Value::as_object) else {
            unavailable_storage.push(storage);
            coverage.insert(
                storage.into(),
                json!({"status":"unavailable","reason":snapshot.unavailable.get(storage)}),
            );
            continue;
        };
        observed_storage.push(storage);
        let mut count = 0usize;
        let mut record = |container: Option<&str>, slot: &str, item: &Value| {
            let item_id = item["item_id"].as_u64();
            let reference = item_id.and_then(|id| catalogue.get(&id));
            total += 1;
            count += 1;
            matched += usize::from(reference.is_some());
            if reference.is_none()
                && let Some(id) = item_id
            {
                unmatched_ids.insert(id);
            }
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
            if wrong_type
                || wrong_set
                || wrong_item
                || wrong_slot
                || catalogue_matched.is_some_and(|expected| reference.is_some() != expected)
                || storage_filter.is_some_and(|wanted| wanted.section() != storage)
                || container_filter
                    .as_deref()
                    .is_some_and(|wanted| container != Some(wanted))
                || !matches_required_level(reference.copied())
            {
                return;
            }
            let on_page = included >= offset && entries.len() < limit;
            let item_conflict = (on_page || exclude_observed_requirement_conflicts)
                .then(|| reference.and_then(|item| check_item_requirements(item)))
                .flatten();
            if exclude_observed_requirement_conflicts && item_conflict.is_some() {
                return;
            }
            let modifiers = (on_page
                || simulator_input_resolved.is_some()
                || resolution_filter.is_some()
                || include_resolution_counts
                || observed_enchant_effect_id.is_some()
                || observed_random_suffix_id.is_some())
            .then(|| link_modifiers(item))
            .flatten();
            if observed_enchant_effect_id.is_some_and(|id| {
                modifiers
                    .as_ref()
                    .and_then(|fields| fields["enchant_id"].as_u64())
                    != Some(u64::from(id.get()))
            }) || observed_random_suffix_id.is_some_and(|id| {
                modifiers
                    .as_ref()
                    .and_then(|fields| fields["random_suffix_id"].as_i64())
                    != Some(i64::from(id))
            }) {
                return;
            }
            let candidates = modifiers
                .as_ref()
                .filter(|_| {
                    on_page
                        || simulator_input_resolved.is_some()
                        || resolution_filter.is_some()
                        || include_resolution_counts
                })
                .map(|fields| modifier_candidates(db, fields, reference.copied()));
            if simulator_input_resolved.is_some_and(|expected| {
                candidates
                    .as_ref()
                    .is_some_and(|review| !review["simulator_item_spec"].is_null())
                    != expected
            }) {
                return;
            }
            let simulator_item_resolution =
                (on_page || resolution_filter.is_some() || include_resolution_counts)
                    .then(|| item_resolution(reference, modifiers.as_ref(), candidates.as_ref()));
            if resolution_filter.is_some_and(|expected| simulator_item_resolution != Some(expected))
            {
                return;
            }
            if include_resolution_counts && let Some(status) = simulator_item_resolution {
                *resolution_counts.entry(status).or_default() += 1;
                *resolution_counts_by_storage
                    .entry(storage)
                    .or_default()
                    .entry(status)
                    .or_default() += 1;
            }
            if on_page {
                let draft_selection = target_slot
                    .and_then(|target| draft_selection(target, storage, container, slot, item_id?));
                let requirement_conflict = draft_selection
                    .as_ref()
                    .zip(reference)
                    .and_then(|(selection, item)| check_requirements(item, selection));
                let weapon_skill = reference.and_then(|item| weapon_skill(snapshot, item));
                let native_shortfall = draft_selection
                    .as_ref()
                    .and_then(|selection| native_level_shortfall(snapshot, selection));
                let native_slot_conflict = draft_selection.as_ref().and_then(|selection| {
                    native_slot_category_conflict(item, selection.target_slot)
                });
                let mut entry = json!({"storage":storage,"container":container,"slot":slot,"observed":item,
                "weapon_skill":weapon_skill,
                "catalogue_item_requirement_conflict":item_conflict,
                "catalogue_requirement_conflict":requirement_conflict,
                "native_required_level_shortfall":native_shortfall,
                "native_slot_category_conflict":native_slot_conflict,
                "draft_selection":draft_selection,
                "catalogue_modifier_candidates":candidates,
                "simulator_item_resolution":simulator_item_resolution,
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
                }))});
                if storage == "equipment" {
                    if let Some(condition) = durability.and_then(|slots| slots.get(slot)) {
                        entry["observed_equipped_durability"] = condition.clone();
                    }
                    if let Some(enchants) =
                        crate::analysis::equipped_weapon_hand_enchants(snapshot, slot)
                    {
                        entry["observed_weapon_hand_enchants"] = enchants;
                    }
                }
                entries.push(entry);
            }
            included += 1;
        };
        let mut observed_container_occupied_slots = BTreeMap::new();
        for (key, value) in section {
            if storage == "equipment" {
                record(None, key, value);
            } else if let Some(slots) = value.as_object() {
                observed_container_occupied_slots.insert(key, slots.len());
                for (slot, item) in slots {
                    record(Some(key), slot, item);
                }
            }
        }
        let mut storage_coverage = json!({"status":"observed","occupied_slots":count});
        if storage != "equipment" {
            storage_coverage["observed_container_occupied_slots"] =
                json!(observed_container_occupied_slots);
        }
        coverage.insert(storage.into(), storage_coverage);
    }
    let mut result = json!({"family":"forever","captured_at":snapshot.captured_at,"catalogue_source":provenance,
        "coverage":coverage,"observed_item_locations":total,"base_item_matches":matched,
        "items":entries,"filtered_item_locations":included,"has_more":included.saturating_sub(offset)>limit,
        "live":false,"ranked":false,"equip_legality_verified":false});
    if include_resolution_counts {
        result["simulator_item_resolution_counts"] = json!(resolution_counts);
        result["simulator_item_resolution_counts_by_storage"] = json!(resolution_counts_by_storage);
    }
    result["inventory_evidence"] = json!({
        "scope":"all_observed_storage_before_filters",
        "observed_storage_sections":observed_storage,
        "unavailable_storage_sections":unavailable_storage,
        "all_storage_sections_observed":unavailable_storage.is_empty(),
        "catalogue_unmatched_item_locations":total - matched,
        "catalogue_unmatched_distinct_item_ids":unmatched_ids.len(),
        "catalogue_unmatched_item_ids":unmatched_ids.iter().take(50).collect::<Vec<_>>(),
        "catalogue_unmatched_item_ids_truncated":unmatched_ids.len() > 50,
        "complete_ownership_verified":false,
        "complete_simulator_resolution_verified":false,
        "best_in_slot_verified":false
    });
    if let Some(set_id) = set_id {
        let distinct_pieces_by_storage: BTreeMap<_, _> =
            ["equipment", "bags", "character_bank", "account_bank"]
                .into_iter()
                .map(|storage| {
                    let count = snapshot.sections.contains_key(storage).then(|| {
                        set_locations
                            .values()
                            .filter(|locations| locations.contains_key(storage))
                            .count()
                    });
                    (storage, count)
                })
                .collect();
        let pieces: Vec<_> = catalogue
            .iter()
            .filter(|(_, item)| item["setId"].as_u64() == Some(u64::from(set_id.get())))
            .map(|(id, item)| {
                json!({"item_id":id,"name":item["name"],"observed":set_locations.contains_key(id),
                    "observed_locations_by_storage":set_locations.get(id).cloned().unwrap_or_default()})
            })
            .collect();
        let observed_stored_not_equipped_item_ids =
            snapshot.sections.contains_key("equipment").then(|| {
                set_locations
                    .iter()
                    .filter(|(_, locations)| !locations.contains_key("equipment"))
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>()
            });
        let unobserved_catalogue_item_ids: Vec<_> = pieces
            .iter()
            .filter(|piece| piece["observed"] == false)
            .map(|piece| &piece["item_id"])
            .collect();
        result["set_observation"] = json!({"set_id":set_id.get(),"catalogue_pieces":pieces,
            "observed_distinct_pieces":set_locations.len(),
            "observed_distinct_pieces_by_storage":distinct_pieces_by_storage,
            "observed_stored_not_equipped_item_ids":observed_stored_not_equipped_item_ids,
            "unobserved_catalogue_item_ids":unobserved_catalogue_item_ids});
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
