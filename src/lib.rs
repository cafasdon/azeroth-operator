//! Data-only client contracts and snapshot storage. No game execution.
pub mod action_operation;
pub mod analysis;
pub mod bag_operation;
pub mod binding_operation;
pub mod equipment_operation;
pub mod equipment_set_operation;
pub mod forever_catalogue;
pub mod forever_talent_identity;
pub mod gossip_option_filter;
pub mod import;
pub mod loot_observation;
pub mod macro_operation;
pub mod planning;
pub mod saved_resets;
mod schema;
pub mod skill_line_filter;
pub mod sync;
pub mod talent_operation;
pub mod upload;
use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_BYTES: u64 = 4 * 1024 * 1024;
pub const SNAPSHOT_SECTIONS: &[&str] = &[
    "equipment",
    "equipment_sets",
    "bags",
    "character_bank",
    "account_bank",
    "selected_recipe",
    "tracked_recipes",
    "reputation",
    "currency_balances",
    "wallet",
    "experience",
    "player_resources",
    "player_auras",
    "loot_window",
    "player_trade_offer",
    "player_pvp_progression",
    "group_context",
    "bag_item_cooldowns",
    "gossip_quests",
    "gossip_options",
    "spellbook",
    "flight_map",
    "durability_alerts",
    "equipment_durability",
    "weapon_enchants",
    "repair_quote",
    "merchant_stock",
    "merchant_buyback",
    "trainer_services",
    "pet_care",
    "pet_action_bar",
    "mount_collection",
    "toy_collection_view",
    "saved_instances",
    "saved_world_bosses",
    "profession_recipe_view",
    "profession_spells",
    "profession_recipe_materials",
    "recipe_item_sources",
    "mail_attachments",
    "pet_collection_view",
    "skill_lines",
    "bag_capacity",
    "instance_context",
    "tracked_achievements",
    "talents",
    "stats",
    "professions",
    "quests",
    "currencies",
    "achievements",
    "macros",
    "bindings",
    "action_bars",
    "operation_receipts",
    "binding_state",
    "action_bar_observation",
];

pub const FOREVER_ONLY_OBSERVATIONS: &[&str] = &[
    "selected_recipe",
    "tracked_recipes",
    "reputation",
    "currency_balances",
    "tracked_achievements",
    "wallet",
    "experience",
    "player_resources",
    "player_auras",
    "loot_window",
    "player_trade_offer",
    "player_pvp_progression",
    "group_context",
    "bag_item_cooldowns",
    "gossip_quests",
    "gossip_options",
    "spellbook",
    "flight_map",
    "durability_alerts",
    "equipment_durability",
    "weapon_enchants",
    "repair_quote",
    "merchant_stock",
    "merchant_buyback",
    "trainer_services",
    "pet_care",
    "pet_action_bar",
    "mount_collection",
    "toy_collection_view",
    "saved_instances",
    "saved_world_bosses",
    "profession_recipe_view",
    "profession_spells",
    "profession_recipe_materials",
    "recipe_item_sources",
    "mail_attachments",
    "pet_collection_view",
    "skill_lines",
    "bag_capacity",
    "instance_context",
];

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Retail,
    Classic,
    Forever,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClientIdentity {
    pub family: Family,
    pub variant: String,
    pub build: String,
    pub interface: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Character {
    pub name: String,
    pub realm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Addon,
    OfficialApi,
    Fixture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub source: Source,
    pub captured_at: DateTime<Utc>,
    pub client: ClientIdentity,
    pub character: Character,
    pub sections: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub unavailable: BTreeMap<String, String>,
}

impl Snapshot {
    pub(crate) fn section_capture_status(&self, name: &str) -> &'static str {
        if self.sections.contains_key(name) {
            "observed"
        } else if self.unavailable.contains_key(name) {
            "reported_unavailable"
        } else {
            "not_recorded"
        }
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema_version == 1, "Unsupported schema version");
        ensure!(self.client.interface > 0, "Missing interface version");
        for text in [
            &self.client.variant,
            &self.client.build,
            &self.character.name,
            &self.character.realm,
        ] {
            ensure!(
                !text.trim().is_empty() && text.len() <= 128,
                "Invalid identity field"
            );
        }
        ensure!(
            self.captured_at <= Utc::now(),
            "Capture timestamp is in the future"
        );
        for (name, value) in &self.sections {
            if name == "pet_care"
                && let Some(food) = value.get("carried_food").and_then(Value::as_object)
            {
                let bags = self
                    .sections
                    .get("bags")
                    .and_then(Value::as_object)
                    .ok_or_else(|| anyhow::anyhow!("Carried pet food requires captured bags"))?;
                let mut totals = std::collections::BTreeMap::<u64, u64>::new();
                for item in bags
                    .values()
                    .filter_map(Value::as_object)
                    .flat_map(|bag| bag.values())
                {
                    if let Some((id, count)) = item["item_id"].as_u64().zip(item["count"].as_u64())
                    {
                        let total = totals.entry(id).or_default();
                        *total = total
                            .checked_add(count)
                            .ok_or_else(|| anyhow::anyhow!("Invalid carried item count"))?;
                    }
                }
                ensure!(
                    food.iter().all(|(id, count)| id
                        .parse::<u64>()
                        .ok()
                        .and_then(|id| totals.get(&id).copied())
                        == count.as_u64()),
                    "Carried pet food does not match bags"
                );
            }
            if name == "stats"
                && (value.get("hearthstone_item_id").is_some()
                    || value.get("hearthstone_cooldown_seconds").is_some())
            {
                ensure!(
                    self.client.family == Family::Forever,
                    "Hearthstone observation requires Forever"
                );
            }
            if name == "stats"
                && let Some(location) = value.get("hearthstone_bind_name")
            {
                ensure!(
                    self.client.family == Family::Forever
                        && location
                            .as_str()
                            .is_some_and(|name| !name.is_empty() && name.len() <= 256),
                    "Invalid Forever Hearthstone bind name"
                );
            }
            if name == "stats"
                && let Some(indicator) = value.get("new_mail_indicator")
            {
                ensure!(
                    self.client.family == Family::Forever && indicator.is_boolean(),
                    "Invalid Forever new mail indicator"
                );
            }
            ensure!(
                !FOREVER_ONLY_OBSERVATIONS.contains(&name.as_str())
                    || self.client.family == Family::Forever,
                "Observation requires Forever"
            );
            schema::validate(name, value)?;
            if name == "talents" && schema::has_talent_definition_observation(value) {
                ensure!(
                    self.client.family == Family::Forever,
                    "Talent definition observations require Forever"
                );
            }
            ensure!(
                SNAPSHOT_SECTIONS.contains(&name.as_str()),
                "Unknown section"
            );
            ensure!(
                value.is_array() || value.is_object(),
                "Section must be an array or object"
            );
        }
        if let Some(cooldowns) = self.sections.get("bag_item_cooldowns") {
            schema::bag_item_cooldowns(cooldowns, self.sections.get("bags"))?;
        }
        if self.sections.contains_key("character_bank")
            || self.sections.contains_key("account_bank")
        {
            ensure!(
                self.client.family == Family::Forever,
                "Bank snapshot requires Forever"
            );
            let mut containers = std::collections::BTreeSet::new();
            for name in ["bags", "character_bank", "account_bank"] {
                if let Some(bags) = self.sections.get(name).and_then(Value::as_object) {
                    for bag in bags.keys() {
                        ensure!(
                            containers.insert(bag),
                            "Container appears in multiple storage sections"
                        );
                    }
                }
            }
        }
        for (name, reason) in &self.unavailable {
            ensure!(
                SNAPSHOT_SECTIONS.contains(&name.as_str()) && !self.sections.contains_key(name),
                "Invalid unavailable section"
            );
            ensure!(
                !reason.trim().is_empty() && reason.len() <= 256,
                "Invalid unavailable reason"
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Store {
    pub root: PathBuf,
}

#[derive(Default)]
pub struct RecipeCaptureFilters {
    pub recipe_id: Option<std::num::NonZeroU32>,
    pub output_item_id: Option<std::num::NonZeroU32>,
    pub profession_id: Option<std::num::NonZeroU32>,
    pub cooldown_state: Option<analysis::RecipeCooldownState>,
}

#[derive(Default)]
pub struct InstanceCaptureFilters<'a> {
    pub kind: Option<&'a str>,
    pub name: Option<&'a str>,
    pub difficulty_id: Option<u32>,
    pub map_id: Option<std::num::NonZeroU32>,
}

#[derive(Clone, Copy, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecipeLearning {
    Learned,
    Unlearned,
    Unknown,
}

impl RecipeLearning {
    pub fn observed(self) -> Option<bool> {
        match self {
            Self::Learned => Some(true),
            Self::Unlearned => Some(false),
            Self::Unknown => None,
        }
    }
}

#[derive(Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrackedRecipeKind {
    Craft,
    Recraft,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BagFamilySpaceFilter {
    /// Exact captured family mask. Zero selects general-purpose bags.
    pub family_mask: u32,
    /// Inclusive minimum free slots for this observed family.
    pub minimum_free_slots: u32,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CollectedMountFilter {
    pub mount_id: std::num::NonZeroU32,
    pub usable: Option<bool>,
    pub favorite: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToyViewFilter {
    pub item_id: std::num::NonZeroU32,
    pub collected: Option<bool>,
    pub favorite: Option<bool>,
}

impl ToyViewFilter {
    pub fn match_view(&self, view: &Value) -> Option<Value> {
        let row = view["entries"].get(self.item_id.to_string())?;
        (self
            .collected
            .is_none_or(|flag| row["collected"].as_bool() == Some(flag))
            && self
                .favorite
                .is_none_or(|flag| row["favorite"].as_bool() == Some(flag)))
        .then(|| {
            json!({"item_id":self.item_id,"collected":row["collected"],
                "favorite":row["favorite"],"coverage":view["coverage"]})
        })
    }
}

impl CollectedMountFilter {
    pub fn match_view(&self, view: &Value) -> Option<Value> {
        let row = view["entries"].get(self.mount_id.to_string())?;
        (self
            .usable
            .is_none_or(|expected| row["usable"].as_bool() == Some(expected))
            && self
                .favorite
                .is_none_or(|expected| row["favorite"].as_bool() == Some(expected)))
        .then(|| {
            json!({"mount_id":self.mount_id,"spell_id":row["spell_id"],
                "usable":row["usable"],"favorite":row["favorite"]})
        })
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SavedInstanceFilter {
    /// Exact case-insensitive captured instance name.
    pub name: Option<String>,
    pub difficulty_id: Option<u32>,
    pub locked: Option<bool>,
    pub extended: Option<bool>,
    pub is_raid: Option<bool>,
    /// Match the reported instance group-size cap.
    pub max_players: Option<std::num::NonZeroU32>,
    /// Only nonnegative reset timers at or below this value, as recorded at capture.
    pub maximum_reset_seconds: Option<u32>,
    /// Inclusive historical reset deadline (RFC 3339). Unknown projections do not match.
    pub projected_reset_by: Option<chrono::DateTime<chrono::Utc>>,
    /// Inclusive start of a historical reset window (RFC 3339).
    pub projected_reset_from: Option<chrono::DateTime<chrono::Utc>>,
    /// Maximum age of the last instance-info update at capture, not age today.
    pub maximum_update_age_seconds: Option<u32>,
}

#[derive(Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MerchantStockState {
    Unlimited,
    LimitedInStock,
    SoldOut,
}

impl MerchantStockState {
    pub fn matches(self, available: Option<i64>) -> bool {
        match self {
            Self::Unlimited => available == Some(-1),
            Self::LimitedInStock => matches!(available, Some(1..)),
            Self::SoldOut => available == Some(0),
        }
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SavedWorldBossFilter {
    /// Exact case-insensitive captured boss name.
    pub name: Option<String>,
    /// Match only nonnegative reset timers at or below this captured value.
    pub maximum_reset_seconds: Option<u32>,
    /// Inclusive historical reset deadline (RFC 3339). Unknown projections do not match.
    pub projected_reset_by: Option<chrono::DateTime<chrono::Utc>>,
    /// Inclusive start of a historical reset window (RFC 3339).
    pub projected_reset_from: Option<chrono::DateTime<chrono::Utc>>,
    /// Maximum instance-info cache age at capture, not age today.
    pub maximum_update_age_seconds: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfessionRecipeFilter {
    pub recipe_id: std::num::NonZeroU32,
    pub profession_id: Option<std::num::NonZeroU32>,
    /// Unknown requires an observed row with no readable learning flag.
    pub learning: Option<RecipeLearning>,
    /// Require an observed skill-up flag; unknown does not match false.
    pub can_skill_up: Option<bool>,
    /// Require an observed reported skill-up count at or above this bound.
    pub minimum_skill_ups_reported: Option<u32>,
}

#[derive(Clone, Copy, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MailAttachmentKind {
    Item,
    Currency,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MailAttachmentFilter {
    pub kind: MailAttachmentKind,
    pub id: std::num::NonZeroU32,
    /// Minimum summed quantity in the loaded inbox only; defaults to one.
    pub minimum_quantity: Option<u64>,
    /// Currency only. Require a known hypothetical overflow no greater than this quantity.
    pub maximum_quantity_above_recorded_headroom: Option<u64>,
    /// Items only. Require a known fit in recorded general bag slots without stack merges.
    #[serde(default)]
    pub require_recorded_bag_space: bool,
    /// Require zero unloaded messages and zero unknown attachment slots in the saved capture.
    #[serde(default)]
    pub require_fully_readable_capture: bool,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PetSpeciesFilter {
    pub species_id: std::num::NonZeroU32,
    /// Minimum owned rows in the captured filtered view, not the whole account.
    pub minimum_owned_rows: Option<u32>,
    pub minimum_unowned_rows: Option<u32>,
    /// Maximum owned rows in the captured filtered view.
    pub maximum_owned_rows: Option<u32>,
    /// Maximum unowned rows in the captured filtered view.
    pub maximum_unowned_rows: Option<u32>,
}

impl PetSpeciesFilter {
    pub fn validate(&self) -> Result<()> {
        for (minimum, maximum) in [
            (self.minimum_owned_rows, self.maximum_owned_rows),
            (self.minimum_unowned_rows, self.maximum_unowned_rows),
        ] {
            ensure!(
                minimum.zip(maximum).is_none_or(|(min, max)| min <= max),
                "Pet species row bounds are reversed"
            );
        }
        Ok(())
    }

    pub fn match_view(&self, view: &Value) -> Option<Value> {
        let row = view["entries"].get(self.species_id.to_string())?;
        let owned = row["owned_rows"].as_u64()?;
        let unowned = row["unowned_rows"].as_u64()?;
        (self
            .minimum_owned_rows
            .is_none_or(|min| owned >= u64::from(min))
            && self
                .minimum_unowned_rows
                .is_none_or(|min| unowned >= u64::from(min))
            && self
                .maximum_owned_rows
                .is_none_or(|max| owned <= u64::from(max))
            && self
                .maximum_unowned_rows
                .is_none_or(|max| unowned <= u64::from(max)))
        .then(|| {
            json!({"coverage":view["coverage"],"displayed_rows":view["displayed_rows"],
                "species_id":self.species_id,"name":row["name"],
                "owned_rows":owned,"unowned_rows":unowned})
        })
    }
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpellResourceFilter {
    pub minimum_charges: Option<u32>,
    pub maximum_charges: Option<u32>,
    pub minimum_reported_cast_count: Option<u32>,
    pub maximum_reported_cast_count: Option<u32>,
    pub cooldown_enabled: Option<bool>,
    pub cooldown_active: Option<bool>,
}

impl SpellResourceFilter {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.minimum_charges.is_some()
                || self.maximum_charges.is_some()
                || self.minimum_reported_cast_count.is_some()
                || self.maximum_reported_cast_count.is_some()
                || self.cooldown_enabled.is_some()
                || self.cooldown_active.is_some(),
            "Empty spell resource filter"
        );
        ensure!(
            self.minimum_charges
                .zip(self.maximum_charges)
                .is_none_or(|(a, b)| a <= b)
                && self
                    .minimum_reported_cast_count
                    .zip(self.maximum_reported_cast_count)
                    .is_none_or(|(a, b)| a <= b),
            "Reversed spell resource bounds"
        );
        Ok(())
    }

    /// Conjunction of captured predicates: false dominates unknown; never castability.
    pub fn matches(&self, row: &Value) -> Option<bool> {
        let charges = row["charge_count"]["current"].as_u64();
        let casts = row["reported_cast_count"].as_u64();
        let checks: Vec<_> = [
            self.minimum_charges
                .map(|n| charges.map(|v| v >= u64::from(n))),
            self.maximum_charges
                .map(|n| charges.map(|v| v <= u64::from(n))),
            self.minimum_reported_cast_count
                .map(|n| casts.map(|v| v >= u64::from(n))),
            self.maximum_reported_cast_count
                .map(|n| casts.map(|v| v <= u64::from(n))),
            self.cooldown_enabled
                .map(|v| row["cooldown_state"]["enabled"].as_bool().map(|n| n == v)),
            self.cooldown_active
                .map(|v| row["cooldown_state"]["active"].as_bool().map(|n| n == v)),
        ]
        .into_iter()
        .flatten()
        .collect();
        if checks.contains(&Some(false)) {
            Some(false)
        } else if checks.contains(&None) {
            None
        } else {
            Some(true)
        }
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpellbookSpellFilter {
    /// Match either the base or reported spell ID in one saved player spellbook row.
    pub spell_id: Option<std::num::NonZeroU32>,
    /// Match the exact case-insensitive saved spell name.
    pub name: Option<String>,
    /// Captured resource predicates on the same row; allows resource-only discovery.
    pub resources: Option<SpellResourceFilter>,
    pub passive: Option<bool>,
    pub off_spec: Option<bool>,
    /// Recorded API usability, not current cooldown/target eligibility.
    pub usable: Option<bool>,
    pub insufficient_power: Option<bool>,
    /// Inclusive reported learn-level bound; level linking may affect this observation.
    pub minimum_level_learned: Option<u32>,
    pub maximum_level_learned: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionBarSpellResourceFilter {
    /// Exact reported spell ID shared by the action slot and spellbook row.
    pub spell_id: std::num::NonZeroU32,
    /// All predicates must match the same saved spellbook row.
    pub resources: SpellResourceFilter,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActivePetCaptureFilter {
    /// Recorded pet presence, not current game state.
    pub present: Option<bool>,
    pub family_id: Option<std::num::NonZeroU32>,
    pub minimum_level: Option<std::num::NonZeroU32>,
    pub maximum_level: Option<std::num::NonZeroU32>,
    /// Inclusive signed total-minus-used bound; requires observed counters.
    pub minimum_training_points_balance: Option<i64>,
    pub maximum_training_points_balance: Option<i64>,
}

impl ActivePetCaptureFilter {
    fn validate(&self) -> Result<()> {
        let details = self.family_id.is_some()
            || self.minimum_level.is_some()
            || self.maximum_level.is_some()
            || self.minimum_training_points_balance.is_some()
            || self.maximum_training_points_balance.is_some();
        ensure!(
            self.present.is_some() || details,
            "Select an active-pet observation filter"
        );
        ensure!(
            self.present != Some(false) || !details,
            "Absent-pet filtering cannot require pet details"
        );
        ensure!(
            self.minimum_level
                .zip(self.maximum_level)
                .is_none_or(|(a, b)| a <= b),
            "Reversed pet level bounds"
        );
        for bound in [
            self.minimum_training_points_balance,
            self.maximum_training_points_balance,
        ]
        .into_iter()
        .flatten()
        {
            ensure!(
                (-(i64::from(u32::MAX))..=i64::from(u32::MAX)).contains(&bound),
                "Pet training balance bound out of range"
            );
        }
        ensure!(
            self.minimum_training_points_balance
                .zip(self.maximum_training_points_balance)
                .is_none_or(|(a, b)| a <= b),
            "Reversed pet training balance bounds"
        );
        Ok(())
    }

    fn observation_match(&self, pet: &Value) -> Option<Value> {
        let present = pet["present"].as_bool()?;
        if self.present.is_some_and(|expected| present != expected)
            || (!present && self.present != Some(false))
        {
            return None;
        }
        let level = pet["level"].as_u64();
        let balance = analysis::pet_training_points_balance(pet);
        if !self
            .family_id
            .is_none_or(|id| pet["family_id"].as_u64() == Some(u64::from(id.get())))
            || !self
                .minimum_level
                .is_none_or(|minimum| level.is_some_and(|n| n >= u64::from(minimum.get())))
            || !self
                .maximum_level
                .is_none_or(|maximum| level.is_some_and(|n| n <= u64::from(maximum.get())))
            || !self
                .minimum_training_points_balance
                .is_none_or(|minimum| balance.is_some_and(|n| n >= minimum))
            || !self
                .maximum_training_points_balance
                .is_none_or(|maximum| balance.is_some_and(|n| n <= maximum))
        {
            return None;
        }
        Some(
            json!({"present":present,"family_id":pet["family_id"],"level":pet["level"],"training_points_balance":balance}),
        )
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InventoryStorage {
    Equipment,
    Bags,
    CharacterBank,
    AccountBank,
}

impl InventoryStorage {
    fn section_name(self) -> &'static str {
        match self {
            Self::Equipment => "equipment",
            Self::Bags => "bags",
            Self::CharacterBank => "character_bank",
            Self::AccountBank => "account_bank",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InventoryItemFilter {
    pub item_id: std::num::NonZeroU32,
    pub storage: InventoryStorage,
    /// Restrict quantities to one observed bag or bank container.
    #[schemars(range(min = 0, max = 64))]
    pub container_id: Option<u8>,
    pub minimum_quantity: u64,
    pub maximum_quantity: Option<u64>,
}

impl InventoryItemFilter {
    pub fn validate(&self, family: &Family) -> Result<()> {
        ensure!(
            !matches!(
                self.storage,
                InventoryStorage::CharacterBank | InventoryStorage::AccountBank
            ) || *family == Family::Forever,
            "Bank inventory filtering requires Forever"
        );
        ensure!(
            self.maximum_quantity
                .is_none_or(|maximum| self.minimum_quantity <= maximum),
            "Inventory minimum quantity exceeds maximum quantity"
        );
        ensure!(
            self.container_id
                .is_none_or(|id| id <= 64 && !matches!(self.storage, InventoryStorage::Equipment)),
            "Container filtering requires a bag or bank container ID from 0 to 64"
        );
        Ok(())
    }

    // The store validates the complete source snapshot before applying filters.
    // Absence means unknown; an observed empty section means zero.
    fn observed_quantity(&self, snapshot: &Snapshot) -> Option<u64> {
        let section = snapshot
            .sections
            .get(self.storage.section_name())?
            .as_object()?;
        let requested = u64::from(self.item_id.get());
        let mut quantity = 0_u64;
        match self.storage {
            InventoryStorage::Equipment => {
                for item in section.values() {
                    if item["item_id"].as_u64()? == requested {
                        quantity = quantity.checked_add(1)?;
                    }
                }
            }
            _ => {
                let selected = self.container_id.map(|id| id.to_string());
                if let Some(id) = &selected {
                    section.get(id)?;
                }
                for (_, container) in section
                    .iter()
                    .filter(|(id, _)| selected.as_ref().is_none_or(|selected| *id == selected))
                {
                    for item in container.as_object()?.values() {
                        if item["item_id"].as_u64()? == requested {
                            let count = item["count"].as_u64()?;
                            if count == 0 || count > u64::from(u32::MAX) {
                                return None;
                            }
                            quantity = quantity.checked_add(count)?;
                        }
                    }
                }
            }
        }
        Some(quantity)
    }

    fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        let quantity = self.observed_quantity(snapshot)?;
        if quantity < self.minimum_quantity
            || self
                .maximum_quantity
                .is_some_and(|maximum| quantity > maximum)
        {
            return None;
        }
        let mut observation = json!({
            "item_id": self.item_id,
            "storage": self.storage,
            "observed_quantity": quantity,
            "basis": "same_snapshot_single_storage",
            "complete_ownership_verified": false,
            "current_access_verified": false,
            "shared_storage_scope_verified": false,
            "cross_character_aggregation_performed": false,
            "game_execution": false,
        });
        if let Some(id) = self.container_id {
            observation["container_id"] = json!(id);
            observation["basis"] = json!("same_snapshot_single_container");
        }
        Some(observation)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InventoryRequirement {
    pub item_id: std::num::NonZeroU32,
    pub quantity: std::num::NonZeroU64,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InventoryRequirementsFilter {
    pub storage: InventoryStorage,
    /// Require all quantities in one recorded bag or bank container.
    #[schemars(range(min = 0, max = 64))]
    pub container_id: Option<u8>,
    pub items: Vec<InventoryRequirement>,
}

impl InventoryRequirementsFilter {
    pub fn review_observed_stock(
        &self,
        snapshot: &Snapshot,
        bank_withdrawal_offset: Option<usize>,
    ) -> Value {
        self.review_stock(
            snapshot,
            bank_withdrawal_offset.is_some(),
            bank_withdrawal_offset,
        )
    }

    fn review_stock(
        &self,
        snapshot: &Snapshot,
        bank_plan: bool,
        bank_withdrawal_offset: Option<usize>,
    ) -> Value {
        let quantities = self.observed_quantities(snapshot);
        let items: Vec<_> = self.items.iter().map(|row| {
            let required = row.quantity.get();
            let observed = quantities.as_ref().map(|stock| stock[&u64::from(row.item_id.get())]);
            json!({"item_id":row.item_id,"required_quantity":required,"observed_quantity":observed,
                "shortfall_vs_observed":observed.map(|count| required.saturating_sub(count))})
        }).collect();
        let short_item_count = quantities.as_ref().map(|_| {
            items
                .iter()
                .filter(|row| row["shortfall_vs_observed"].as_u64().is_some_and(|n| n > 0))
                .count()
        });
        let mut report = json!({"storage":self.storage,"items":items,"storage_observed":quantities.is_some(),
            "short_item_count":short_item_count,
            "all_requirements_covered_by_observed_quantities":short_item_count.map(|count| count == 0),
            "quest_item_binding_verified":false,"quest_readiness_verified":false,"live":false});
        if let Some(id) = self.container_id {
            report["container_id"] = json!(id);
        }
        if bank_plan && self.container_id.is_none() {
            report["bank_withdrawal_plan"] = json!(quantities.as_ref().map(|bags| {
                let bank_stock = |storage| Self { storage, container_id:None, items:self.items.clone() }
                    .observed_quantities(snapshot);
                let character = bank_stock(InventoryStorage::CharacterBank);
                let account = bank_stock(InventoryStorage::AccountBank);
                let rows: Vec<_> = self.items.iter().filter_map(|requirement| {
                    let id = u64::from(requirement.item_id.get());
                    let required = requirement.quantity.get();
                    if bags[&id] >= required {
                        return None;
                    }
                    let stock = json!({"bags":bags[&id],
                        "character_bank":character.as_ref().map(|items| items[&id]),
                        "account_bank":account.as_ref().map(|items| items[&id])});
                    let (shortfall, withdrawals) = analysis::bag_restock(&stock, required);
                    let withdrawals = withdrawals.unwrap();
                    let remaining = shortfall.unwrap() - withdrawals["character_bank"].as_u64().unwrap_or(0)
                        - withdrawals["account_bank"].as_u64().unwrap_or(0);
                    Some(json!({"item_id":id,"bag_shortfall":shortfall,
                        "suggested_withdrawals":withdrawals,"remaining_after_observed_withdrawals":remaining}))
                }).collect();
                let mut plan = json!({"rows":rows,"character_bank_observed":character.is_some(),
                    "account_bank_observed":account.is_some(),"withdrawal_eligibility_verified":false});
                if let Some(offset) = bank_withdrawal_offset {
                    let amounts: Vec<_> = rows.iter().map(|row| json!({
                        "item_id":row["item_id"],
                        "character_bank":row["suggested_withdrawals"]["character_bank"],
                        "account_bank":row["suggested_withdrawals"]["account_bank"],
                    })).collect();
                    plan["suggested_locations"] = analysis::bank_withdrawal_locations(snapshot, &amounts, offset);
                }
                plan
            }));
        }
        report
    }

    pub fn validate(&self, family: &Family) -> Result<()> {
        ensure!(
            !matches!(
                self.storage,
                InventoryStorage::CharacterBank | InventoryStorage::AccountBank
            ) || *family == Family::Forever,
            "Bank inventory filtering requires Forever"
        );
        ensure!(
            (1..=64).contains(&self.items.len()),
            "Inventory requirements must contain 1..64 items"
        );
        let mut ids = std::collections::BTreeSet::new();
        ensure!(
            self.items.iter().all(|item| ids.insert(item.item_id)),
            "Inventory requirements contain duplicate item IDs"
        );
        ensure!(
            self.container_id
                .is_none_or(|id| id <= 64 && !matches!(self.storage, InventoryStorage::Equipment)),
            "Container filtering requires a bag or bank container ID from 0 to 64"
        );
        Ok(())
    }

    // Scan one validated storage section once, retaining only requested IDs.
    fn observed_quantities(&self, snapshot: &Snapshot) -> Option<BTreeMap<u64, u64>> {
        let section = snapshot
            .sections
            .get(self.storage.section_name())?
            .as_object()?;
        let mut quantities: BTreeMap<u64, u64> = self
            .items
            .iter()
            .map(|item| (u64::from(item.item_id.get()), 0))
            .collect();
        match self.storage {
            InventoryStorage::Equipment => {
                for item in section.values() {
                    if let Some(quantity) = quantities.get_mut(&item["item_id"].as_u64()?) {
                        *quantity = quantity.checked_add(1)?;
                    }
                }
            }
            _ => {
                let selected = self.container_id.map(|id| id.to_string());
                if let Some(id) = &selected {
                    section.get(id)?;
                }
                for (_, container) in section
                    .iter()
                    .filter(|(id, _)| selected.as_ref().is_none_or(|selected| *id == selected))
                {
                    for item in container.as_object()?.values() {
                        if let Some(quantity) = quantities.get_mut(&item["item_id"].as_u64()?) {
                            let count = item["count"].as_u64()?;
                            if count == 0 || count > u64::from(u32::MAX) {
                                return None;
                            }
                            *quantity = quantity.checked_add(count)?;
                        }
                    }
                }
            }
        }
        Some(quantities)
    }

    fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        let quantities = self.observed_quantities(snapshot)?;
        let mut items = Vec::with_capacity(self.items.len());
        for requirement in &self.items {
            let observed = *quantities.get(&u64::from(requirement.item_id.get()))?;
            if observed < requirement.quantity.get() {
                return None;
            }
            items.push(json!({
                "item_id": requirement.item_id,
                "required_quantity": requirement.quantity,
                "observed_quantity": observed,
            }));
        }
        let mut observation = json!({
            "storage": self.storage,
            "basis": "same_snapshot_single_storage",
            "items": items,
            "all_requirements_covered_by_observed_quantities": true,
            "complete_ownership_verified": false,
            "current_access_verified": false,
            "shared_storage_scope_verified": false,
            "cross_character_aggregation_performed": false,
            "craftability_verified": false,
            "transferability_verified": false,
            "game_execution": false,
        });
        if let Some(id) = self.container_id {
            observation["container_id"] = json!(id);
            observation["basis"] = json!("same_snapshot_single_container");
        }
        Some(observation)
    }
}

#[cfg(test)]
mod inventory_item_filter_tests {
    use super::*;

    fn filter(
        storage: InventoryStorage,
        minimum: u64,
        maximum: Option<u64>,
    ) -> InventoryItemFilter {
        InventoryItemFilter {
            item_id: std::num::NonZeroU32::new(123).unwrap(),
            storage,
            container_id: None,
            minimum_quantity: minimum,
            maximum_quantity: maximum,
        }
    }

    fn snapshot(sections: Value) -> Snapshot {
        let snapshot: Snapshot = serde_json::from_value(json!({
            "schema_version": 1,
            "source": "fixture",
            "captured_at": "2025-01-01T00:00:00Z",
            "client": {"family": "forever", "variant": "test", "build": "123", "interface": 123},
            "character": {"name": "Example", "realm": "Test"},
            "sections": sections,
        }))
        .unwrap();
        snapshot.validate().unwrap();
        snapshot
    }

    #[test]
    fn required_fields_and_unknown_fields_are_rejected() {
        let valid = json!({"item_id": 123, "storage": "bags", "minimum_quantity": 0});
        assert!(serde_json::from_value::<InventoryItemFilter>(valid.clone()).is_ok());
        for field in ["item_id", "storage", "minimum_quantity"] {
            let mut missing = valid.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<InventoryItemFilter>(missing).is_err());
        }
        for (field, value) in [
            ("item_id", json!(0)),
            ("storage", json!("all")),
            ("minimum_quantity", json!(-1)),
            ("minimum_quantity", json!(1.5)),
            ("aggregate_characters", json!(true)),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert!(serde_json::from_value::<InventoryItemFilter>(invalid).is_err());
        }
    }

    #[test]
    fn banks_require_forever_and_quantity_bounds_are_ordered() {
        for storage in [
            InventoryStorage::CharacterBank,
            InventoryStorage::AccountBank,
        ] {
            let query = filter(storage, 0, Some(0));
            assert!(query.validate(&Family::Forever).is_ok());
            assert!(query.validate(&Family::Retail).is_err());
            assert!(query.validate(&Family::Classic).is_err());
        }
        for storage in [InventoryStorage::Equipment, InventoryStorage::Bags] {
            for family in [Family::Forever, Family::Retail, Family::Classic] {
                assert!(filter(storage, 0, Some(0)).validate(&family).is_ok());
                assert!(filter(storage, 2, Some(1)).validate(&family).is_err());
            }
        }
    }

    #[test]
    fn missing_or_unavailable_is_unknown_but_empty_is_observed_zero() {
        for storage in [
            InventoryStorage::Equipment,
            InventoryStorage::Bags,
            InventoryStorage::CharacterBank,
            InventoryStorage::AccountBank,
        ] {
            let query = filter(storage, 0, Some(0));
            let mut absent = snapshot(json!({}));
            assert_eq!(query.observed_quantity(&absent), None);
            assert!(query.observation_match(&absent).is_none());
            absent
                .unavailable
                .insert(storage.section_name().into(), "Not captured".into());
            absent.validate().unwrap();
            assert!(query.observation_match(&absent).is_none());
            let mut sections = serde_json::Map::new();
            sections.insert(storage.section_name().into(), json!({}));
            let empty = snapshot(Value::Object(sections));
            assert_eq!(query.observed_quantity(&empty), Some(0));
            assert!(query.observation_match(&empty).is_some());
        }
    }

    #[test]
    fn equipment_counts_locations_and_emits_only_match_metadata() {
        let capture = snapshot(json!({"equipment": {
            "11": {"item_id": 123, "guid": "private-guid", "link": "private-link"},
            "12": {"item_id": 123},
            "16": {"item_id": 456}
        }}));
        let query = filter(InventoryStorage::Equipment, 2, Some(2));
        assert_eq!(query.observed_quantity(&capture), Some(2));
        assert_eq!(
            query.observation_match(&capture).unwrap(),
            json!({
                "item_id": 123,
                "storage": "equipment",
                "observed_quantity": 2,
                "basis": "same_snapshot_single_storage",
                "complete_ownership_verified": false,
                "current_access_verified": false,
                "shared_storage_scope_verified": false,
                "cross_character_aggregation_performed": false,
                "game_execution": false
            })
        );
        assert!(
            filter(InventoryStorage::Equipment, 3, None)
                .observation_match(&capture)
                .is_none()
        );
        assert!(
            filter(InventoryStorage::Equipment, 0, Some(1))
                .observation_match(&capture)
                .is_none()
        );
    }

    #[test]
    fn container_stacks_use_u64_and_never_add_other_storage() {
        let capture = snapshot(json!({
            "equipment": {"11": {"item_id": 123}},
            "bags": {"0": {"1": {"item_id": 123, "count": u32::MAX}},
                     "1": {"1": {"item_id": 123, "count": u32::MAX},
                           "2": {"item_id": 456, "count": 999}}},
            "character_bank": {"5": {"1": {"item_id": 123, "count": 7}}},
            "account_bank": {"10": {"1": {"item_id": 123, "count": 11}}}
        }));
        for (storage, expected) in [
            (InventoryStorage::Equipment, 1),
            (InventoryStorage::Bags, 2 * u64::from(u32::MAX)),
            (InventoryStorage::CharacterBank, 7),
            (InventoryStorage::AccountBank, 11),
        ] {
            let query = filter(storage, expected, Some(expected));
            assert_eq!(query.observed_quantity(&capture), Some(expected));
            assert!(query.observation_match(&capture).is_some());
        }
    }

    #[test]
    fn nonmatching_items_are_zero_without_cross_snapshot_aggregation() {
        let first = snapshot(json!({"bags": {"0": {"1": {"item_id": 123, "count": 2}}}}));
        let second = snapshot(json!({"bags": {"0": {"1": {"item_id": 123, "count": 3}}}}));
        let query = filter(InventoryStorage::Bags, 5, None);
        assert!(query.observation_match(&first).is_none());
        assert!(query.observation_match(&second).is_none());
        let nonmatching = snapshot(json!({"bags": {"0": {"1": {"item_id": 456, "count": 99}}}}));
        assert_eq!(query.observed_quantity(&nonmatching), Some(0));
        assert!(
            filter(InventoryStorage::Bags, 0, Some(0))
                .observation_match(&nonmatching)
                .is_some()
        );
    }

    #[test]
    fn malformed_matching_counts_fail_closed() {
        let mut capture = snapshot(json!({"bags": {"0": {"1": {"item_id": 123, "count": 1}}}}));
        let query = filter(InventoryStorage::Bags, 0, None);
        for count in [json!(0), json!(u64::from(u32::MAX) + 1), Value::Null] {
            capture.sections.get_mut("bags").unwrap()["0"]["1"]["count"] = count;
            assert!(capture.validate().is_err());
            assert!(query.observation_match(&capture).is_none());
        }
    }
}

#[cfg(test)]
mod inventory_requirements_filter_tests {
    use super::*;

    fn filter(storage: InventoryStorage, items: &[(u32, u64)]) -> InventoryRequirementsFilter {
        InventoryRequirementsFilter {
            storage,
            container_id: None,
            items: items
                .iter()
                .map(|&(id, quantity)| InventoryRequirement {
                    item_id: std::num::NonZeroU32::new(id).unwrap(),
                    quantity: std::num::NonZeroU64::new(quantity).unwrap(),
                })
                .collect(),
        }
    }

    fn snapshot(sections: Value) -> Snapshot {
        let snapshot: Snapshot = serde_json::from_value(json!({
            "schema_version": 1,
            "source": "fixture",
            "captured_at": "2025-01-01T00:00:00Z",
            "client": {"family": "forever", "variant": "test", "build": "123", "interface": 123},
            "character": {"name": "Example", "realm": "Test"},
            "sections": sections,
        }))
        .unwrap();
        snapshot.validate().unwrap();
        snapshot
    }

    #[test]
    fn one_to_sixty_four_distinct_requirements_are_allowed() {
        assert!(
            filter(InventoryStorage::Bags, &[])
                .validate(&Family::Forever)
                .is_err()
        );
        assert!(
            filter(InventoryStorage::Bags, &[(1, 1)])
                .validate(&Family::Forever)
                .is_ok()
        );
        let boundary: Vec<_> = (1..=64).map(|id| (id, u64::MAX)).collect();
        assert!(
            filter(InventoryStorage::Bags, &boundary)
                .validate(&Family::Forever)
                .is_ok()
        );
        let excessive: Vec<_> = (1..=65).map(|id| (id, 1)).collect();
        assert!(
            filter(InventoryStorage::Bags, &excessive)
                .validate(&Family::Forever)
                .is_err()
        );
        assert!(
            filter(InventoryStorage::Bags, &[(1, 1), (1, 2)])
                .validate(&Family::Forever)
                .is_err()
        );
    }

    #[test]
    fn required_positive_fields_and_unknown_fields_are_rejected() {
        for invalid in [
            json!({"item_id": 0, "quantity": 1}),
            json!({"item_id": 1, "quantity": 0}),
            json!({"item_id": 1, "quantity": -1}),
            json!({"item_id": 1, "quantity": 1.5}),
            json!({"item_id": 1}),
            json!({"quantity": 1}),
            json!({"item_id": 1, "quantity": 1, "other": true}),
        ] {
            assert!(serde_json::from_value::<InventoryRequirement>(invalid).is_err());
        }
        for invalid in [
            json!({"items": [{"item_id": 1, "quantity": 1}]}),
            json!({"storage": "bags"}),
            json!({"storage": "all", "items": [{"item_id": 1, "quantity": 1}]}),
            json!({"storage": "bags", "items": [{"item_id": 1, "quantity": 1}], "other": true}),
        ] {
            assert!(serde_json::from_value::<InventoryRequirementsFilter>(invalid).is_err());
        }
    }

    #[test]
    fn bank_scopes_require_forever() {
        for storage in [InventoryStorage::Equipment, InventoryStorage::Bags] {
            for family in [Family::Forever, Family::Retail, Family::Classic] {
                assert!(filter(storage, &[(1, 1)]).validate(&family).is_ok());
            }
        }
        for storage in [
            InventoryStorage::CharacterBank,
            InventoryStorage::AccountBank,
        ] {
            let query = filter(storage, &[(1, 1)]);
            assert!(query.validate(&Family::Forever).is_ok());
            assert!(query.validate(&Family::Retail).is_err());
            assert!(query.validate(&Family::Classic).is_err());
        }
    }

    #[test]
    fn every_requirement_must_be_covered_in_the_same_capture() {
        let query = filter(InventoryStorage::Bags, &[(123, 2), (456, 3)]);
        let first = snapshot(json!({"bags": {"0": {"1": {"item_id": 123, "count": 2}}}}));
        let second = snapshot(json!({"bags": {"0": {"1": {"item_id": 456, "count": 3}}}}));
        assert!(query.observation_match(&first).is_none());
        assert!(query.observation_match(&second).is_none());
        assert_eq!(
            query.observed_quantities(&first).unwrap().get(&456),
            Some(&0)
        );
        let shortfall = snapshot(json!({"bags": {"0": {
            "1": {"item_id": 123, "count": 2}, "2": {"item_id": 456, "count": 2}
        }}}));
        assert!(query.observation_match(&shortfall).is_none());
        let complete = snapshot(json!({"bags": {"0": {
            "1": {"item_id": 123, "count": 2}, "2": {"item_id": 456, "count": 3}
        }}}));
        assert!(query.observation_match(&complete).is_some());
    }

    #[test]
    fn missing_unavailable_and_empty_storage_never_match() {
        for storage in [
            InventoryStorage::Equipment,
            InventoryStorage::Bags,
            InventoryStorage::CharacterBank,
            InventoryStorage::AccountBank,
        ] {
            let query = filter(storage, &[(123, 1)]);
            let mut missing = snapshot(json!({}));
            assert!(query.observed_quantities(&missing).is_none());
            assert!(query.observation_match(&missing).is_none());
            missing
                .unavailable
                .insert(storage.section_name().into(), "Not observed".into());
            missing.validate().unwrap();
            assert!(query.observation_match(&missing).is_none());
            let mut sections = serde_json::Map::new();
            sections.insert(storage.section_name().into(), json!({}));
            let empty = snapshot(Value::Object(sections));
            assert_eq!(
                query.observed_quantities(&empty).unwrap().get(&123),
                Some(&0)
            );
            assert!(query.observation_match(&empty).is_none());
        }
    }

    #[test]
    fn different_storage_sections_are_never_pooled() {
        let capture = snapshot(json!({
            "equipment": {"11": {"item_id": 123}},
            "bags": {"0": {"1": {"item_id": 123, "count": 2}}},
            "character_bank": {"5": {"1": {"item_id": 456, "count": 3}}},
            "account_bank": {"10": {"1": {"item_id": 456, "count": 4}}}
        }));
        for storage in [
            InventoryStorage::Equipment,
            InventoryStorage::Bags,
            InventoryStorage::CharacterBank,
            InventoryStorage::AccountBank,
        ] {
            assert!(
                filter(storage, &[(123, 1), (456, 1)])
                    .observation_match(&capture)
                    .is_none()
            );
        }
        assert!(
            filter(InventoryStorage::Bags, &[(123, 3)])
                .observation_match(&capture)
                .is_none()
        );
        assert!(
            filter(InventoryStorage::AccountBank, &[(456, 7)])
                .observation_match(&capture)
                .is_none()
        );
    }

    #[test]
    fn equipment_counts_each_observed_location_once() {
        let capture = snapshot(json!({"equipment": {
            "11": {"item_id": 123}, "12": {"item_id": 123}, "16": {"item_id": 456}
        }}));
        let query = filter(InventoryStorage::Equipment, &[(123, 2), (456, 1)]);
        assert_eq!(
            query.observed_quantities(&capture).unwrap(),
            BTreeMap::from([(123, 2), (456, 1)])
        );
        assert!(query.observation_match(&capture).is_some());
        assert!(
            filter(InventoryStorage::Equipment, &[(123, 3)])
                .observation_match(&capture)
                .is_none()
        );
    }

    #[test]
    fn stack_totals_use_u64_and_preserve_request_order_without_raw_items() {
        let quantity = 2 * u64::from(u32::MAX);
        let capture = snapshot(json!({"bags": {
            "0": {"1": {"item_id": 123, "count": u32::MAX, "guid": "private", "link": "private"},
                  "2": {"item_id": 456, "count": 3},
                  "3": {"item_id": 999, "count": 99}},
            "1": {"1": {"item_id": 123, "count": u32::MAX}}
        }}));
        let query = filter(InventoryStorage::Bags, &[(456, 2), (123, quantity)]);
        assert_eq!(
            query.observed_quantities(&capture).unwrap(),
            BTreeMap::from([(123, quantity), (456, 3)])
        );
        assert_eq!(
            query.observation_match(&capture).unwrap(),
            json!({
                "storage": "bags",
                "basis": "same_snapshot_single_storage",
                "items": [
                    {"item_id": 456, "required_quantity": 2, "observed_quantity": 3},
                    {"item_id": 123, "required_quantity": quantity, "observed_quantity": quantity}
                ],
                "all_requirements_covered_by_observed_quantities": true,
                "complete_ownership_verified": false,
                "current_access_verified": false,
                "shared_storage_scope_verified": false,
                "cross_character_aggregation_performed": false,
                "craftability_verified": false,
                "transferability_verified": false,
                "game_execution": false
            })
        );
        assert!(
            filter(InventoryStorage::Bags, &[(123, u64::MAX)])
                .observation_match(&capture)
                .is_none()
        );
    }

    #[test]
    fn malformed_matching_counts_fail_closed() {
        let mut capture = snapshot(json!({"bags": {"0": {"1": {"item_id": 123, "count": 1}}}}));
        let query = filter(InventoryStorage::Bags, &[(123, 1)]);
        for count in [
            json!(0),
            json!(u64::from(u32::MAX) + 1),
            json!(-1),
            json!(1.5),
            Value::Null,
        ] {
            capture.sections.get_mut("bags").unwrap()["0"]["1"]["count"] = count;
            assert!(capture.validate().is_err());
            assert!(query.observation_match(&capture).is_none());
        }
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum PvpRankCap {
    MaxRank,
    CurrentWeekMaxRank,
    PreviousWeekMaxRank,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum PlayerResource {
    Health,
    PrimaryPower,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayerResourceFilter {
    resource: PlayerResource,
    minimum_percent: Option<u8>,
    maximum_percent: Option<u8>,
    power_type: Option<u8>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum PlayerAuraGroup {
    Helpful,
    Harmful,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlayerAuraFilter {
    group: PlayerAuraGroup,
    spell_id: std::num::NonZeroU32,
    minimum_applications: Option<u32>,
    maximum_applications: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum WeaponEnchantHand {
    MainHand,
    OffHand,
    Ranged,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WeaponEnchantFilter {
    hand: WeaponEnchantHand,
    enchant_id: Option<std::num::NonZeroU32>,
    enchant_type: Option<WeaponEnchantType>,
    minimum_charges: Option<u32>,
    maximum_charges: Option<u32>,
    maximum_remaining_milliseconds: Option<u32>,
    /// Match temporary enchants with a recorded timer projecting at or before this timestamp.
    projected_expiry_by: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MerchantBuybackFilter {
    item_id: std::num::NonZeroU32,
    minimum_quantity: Option<std::num::NonZeroU32>,
    maximum_price_copper: Option<u64>,
    minimum_remaining_wallet_copper: Option<u64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PetActionSpellFilter {
    spell_id: std::num::NonZeroU32,
    is_active: Option<bool>,
    autocast_allowed: Option<bool>,
    autocast_enabled: Option<bool>,
    in_range: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PetActionCommandFilter {
    command_token: String,
    is_active: Option<bool>,
}

pub(crate) fn valid_pet_action_command(token: &str) -> bool {
    token.len() <= 64
        && token.strip_prefix("PET_").is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        })
}

fn match_pet_action_rows(section: &Value, predicate: impl Fn(&Value) -> bool) -> Option<Value> {
    let mut matches: BTreeMap<u32, &Value> = BTreeMap::new();
    for (slot, row) in section["entries"].as_object()? {
        if predicate(row) {
            matches.insert(slot.parse().ok()?, row);
        }
    }
    (!matches.is_empty()).then(|| {
        let rows: Vec<_> = matches
            .into_iter()
            .map(|(slot, row)| json!({"slot":slot,"observation":row}))
            .collect();
        json!({"matches":rows,"cast_eligibility_verified":false,"live":false})
    })
}

impl PetActionSpellFilter {
    fn observation_match(&self, section: &Value) -> Option<Value> {
        match_pet_action_rows(section, |row| {
            row["kind"] == "spell"
                && row["spell_id"].as_u64() == Some(u64::from(self.spell_id.get()))
                && [
                    ("is_active", self.is_active),
                    ("autocast_allowed", self.autocast_allowed),
                    ("autocast_enabled", self.autocast_enabled),
                    ("in_range", self.in_range),
                ]
                .into_iter()
                .all(|(field, expected)| {
                    expected.is_none_or(|flag| row[field].as_bool() == Some(flag))
                })
        })
    }
}

impl PetActionCommandFilter {
    fn observation_match(&self, section: &Value) -> Option<Value> {
        match_pet_action_rows(section, |row| {
            row["kind"] == "command"
                && row["command_token"] == self.command_token.as_str()
                && self
                    .is_active
                    .is_none_or(|flag| row["is_active"].as_bool() == Some(flag))
        })
    }
}

impl MerchantBuybackFilter {
    fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        let section = snapshot.sections.get("merchant_buyback")?;
        let spendable = if let Some(reserve) = self.minimum_remaining_wallet_copper {
            Some(
                snapshot.sections.get("wallet")?["copper"]
                    .as_u64()?
                    .checked_sub(reserve)?,
            )
        } else {
            None
        };
        let mut matches: BTreeMap<u32, &Value> = BTreeMap::new();
        for (slot, row) in section["entries"].as_object()? {
            if row["item_id"].as_u64() == Some(u64::from(self.item_id.get()))
                && self.minimum_quantity.is_none_or(|min| {
                    row["count"]
                        .as_u64()
                        .is_some_and(|n| n >= u64::from(min.get()))
                })
                && self.maximum_price_copper.is_none_or(|max| {
                    row["buyback_price_copper"]
                        .as_u64()
                        .is_some_and(|n| n <= max)
                })
                && spendable.is_none_or(|available| {
                    row["buyback_price_copper"]
                        .as_u64()
                        .is_some_and(|price| price <= available)
                })
            {
                matches.insert(slot.parse().ok()?, row);
            }
        }
        (!matches.is_empty()).then(|| {
            let rows: Vec<_> = matches
                .into_iter()
                .map(|(slot, row)| json!({"slot":slot,"observation":row}))
                .collect();
            json!({"matches":rows,"included_in_owned_inventory":false,
                "buyback_eligibility_verified":false,"live":false})
        })
    }
}

#[derive(Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum WeaponEnchantType {
    Permanent,
    Temporary,
    Imbue,
}

impl WeaponEnchantFilter {
    fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        let section = snapshot.sections.get("weapon_enchants")?;
        let hand = match self.hand {
            WeaponEnchantHand::MainHand => "main_hand",
            WeaponEnchantHand::OffHand => "off_hand",
            WeaponEnchantHand::Ranged => "ranged",
        };
        let enchant_type = self.enchant_type.map(|kind| match kind {
            WeaponEnchantType::Permanent => "permanent",
            WeaponEnchantType::Temporary => "temporary",
            WeaponEnchantType::Imbue => "imbue",
        });
        let mut matches: Vec<_> = section["hands"][hand]["entries"]
            .as_object()?
            .iter()
            .filter_map(|(slot, row)| {
                (row["has_enchant"] == true
                    && enchant_type.is_none_or(|kind| row["enchant_type"].as_str() == Some(kind))
                    && self
                        .enchant_id
                        .is_none_or(|id| row["enchant_id"].as_u64() == Some(u64::from(id.get())))
                    && self.minimum_charges.is_none_or(|min| {
                        row["charges"].as_u64().is_some_and(|n| n >= u64::from(min))
                    })
                    && self.maximum_charges.is_none_or(|max| {
                        row["charges"].as_u64().is_some_and(|n| n <= u64::from(max))
                    })
                    && self.maximum_remaining_milliseconds.is_none_or(|max| {
                        row["remaining_milliseconds"]
                            .as_u64()
                            .is_some_and(|n| n <= u64::from(max))
                    })
                    && self.projected_expiry_by.is_none_or(|deadline| {
                        analysis::temporary_weapon_enchant_expiry(row, snapshot.captured_at)
                            .is_some_and(|expiry| expiry <= deadline)
                    }))
                .then(|| slot.parse::<u32>().unwrap())
            })
            .collect();
        matches.sort_unstable();
        (!matches.is_empty())
            .then(|| json!({"hand":hand,"matching_observed_rows":matches,"live":false}))
    }
}

impl PlayerAuraFilter {
    fn observation_match(&self, section: &Value) -> Option<Value> {
        let group = match self.group {
            PlayerAuraGroup::Helpful => "helpful",
            PlayerAuraGroup::Harmful => "harmful",
        };
        let spell_id = u64::from(self.spell_id.get());
        let count = section["groups"][group]["entries"]
            .as_object()?
            .values()
            .filter(|row| row["spell_id"].as_u64() == Some(spell_id))
            .filter(|row| {
                let applications = row["applications"].as_u64();
                self.minimum_applications
                    .is_none_or(|min| applications.is_some_and(|n| n >= u64::from(min)))
                    && self
                        .maximum_applications
                        .is_none_or(|max| applications.is_some_and(|n| n <= u64::from(max)))
            })
            .count();
        (count > 0).then(|| {
            json!({"group":group,"spell_id":spell_id,
            "matching_observed_rows":count,"effectiveness_verified":false})
        })
    }
}

impl PlayerResourceFilter {
    fn observation_match(&self, section: &Value) -> Option<Value> {
        let resource = match self.resource {
            PlayerResource::Health => "health",
            PlayerResource::PrimaryPower => "primary_power",
        };
        let row = section.get(resource)?;
        if self
            .power_type
            .is_some_and(|kind| row["power_type"].as_u64() != Some(u64::from(kind)))
        {
            return None;
        }
        let current = u128::from(row["current"].as_u64()?);
        let maximum = u128::from(row["maximum"].as_u64()?);
        if self.minimum_percent.is_some() || self.maximum_percent.is_some() {
            let min = u128::from(self.minimum_percent.unwrap_or(0));
            let max = u128::from(self.maximum_percent.unwrap_or(100));
            if maximum == 0 || current * 100 < maximum * min || current * 100 > maximum * max {
                return None;
            }
        }
        Some(json!({"resource":resource,"observation":row,"readiness_verified":false}))
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PvpCapDistanceFilter {
    cap: PvpRankCap,
    maximum_remaining_points: u32,
}

impl PvpCapDistanceFilter {
    fn observation_match(&self, progression: &Value) -> Option<Value> {
        let cap = match self.cap {
            PvpRankCap::MaxRank => "max_rank",
            PvpRankCap::CurrentWeekMaxRank => "current_week_max_rank",
            PvpRankCap::PreviousWeekMaxRank => "previous_week_max_rank",
        };
        let report = analysis::player_pvp_progression_observation(progression);
        let remaining = report["remaining_points_to_reported_caps"][cap].as_u64()?;
        (remaining <= u64::from(self.maximum_remaining_points)).then(
            || json!({"cap":cap,"reported_cap_rank":progression[cap],"remaining_points":remaining}),
        )
    }
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum PvpTargetDistanceBasis {
    #[default]
    TotalThreshold,
    NextRankThreshold,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PvpTargetDistanceFilter {
    target_rank: u32,
    maximum_remaining_points: u32,
    #[serde(default)]
    basis: PvpTargetDistanceBasis,
}

impl PvpTargetDistanceFilter {
    fn observation_match(&self, progression: &Value) -> Option<Value> {
        let projection = analysis::pvp_rank_target_projection(progression, self.target_rank)?;
        let field = match self.basis {
            PvpTargetDistanceBasis::TotalThreshold => "remaining_points_using_total_threshold",
            PvpTargetDistanceBasis::NextRankThreshold => {
                "remaining_points_using_next_rank_threshold"
            }
        };
        let remaining = projection[field].as_u64()?;
        (remaining <= u64::from(self.maximum_remaining_points)).then(
            || json!({"basis":self.basis,"remaining_points":remaining,"projection":projection}),
        )
    }
}

#[derive(Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GroupContextCategory {
    Home,
    Instance,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupContextFilter {
    category: GroupContextCategory,
    in_group: Option<bool>,
    in_raid: Option<bool>,
    player_is_leader: Option<bool>,
    minimum_reported_member_count: Option<u32>,
    maximum_reported_member_count: Option<u32>,
}

impl GroupContextFilter {
    fn observation_match(&self, snapshot: &Snapshot) -> Option<Value> {
        let category = match self.category {
            GroupContextCategory::Home => "home",
            GroupContextCategory::Instance => "instance",
        };
        let section = snapshot.sections.get("group_context")?;
        let row = section["categories"].get(category)?;
        for (key, expected) in [
            ("in_group", self.in_group),
            ("in_raid", self.in_raid),
            ("player_is_leader", self.player_is_leader),
        ] {
            if expected.is_some_and(|flag| row[key].as_bool() != Some(flag)) {
                return None;
            }
        }
        let count = row["reported_member_count"].as_u64()?;
        if self
            .minimum_reported_member_count
            .is_some_and(|min| count < u64::from(min))
            || self
                .maximum_reported_member_count
                .is_some_and(|max| count > u64::from(max))
        {
            return None;
        }
        Some(
            json!({"category":category,"observation":row,"coverage":section["coverage"],
            "current_group_verified":false,"group_identity_verified":false}),
        )
    }
}

#[derive(Default)]
pub struct CaptureFilters<'a> {
    pub exact_client: Option<&'a ClientIdentity>,
    pub captured_from: Option<&'a str>,
    pub captured_through: Option<&'a str>,
    pub latest_per_character: bool,
    pub required_sections: Option<&'a [String]>,
    pub any_missing_sections: Option<&'a [String]>,
    pub coverage_summary_sections: Option<&'a [String]>,
    pub inventory_item: Option<InventoryItemFilter>,
    pub inventory_requirements: Option<InventoryRequirementsFilter>,
    pub inventory_requirements_shortfall_only: bool,
    pub inventory_requirements_bank_plan: bool,
    pub loot_observation: Option<loot_observation::LootObservationFilter>,
    pub skill_line: Option<skill_line_filter::SkillLineFilter>,
    pub gossip_option: Option<gossip_option_filter::GossipOptionFilter>,
    pub outgoing_trade_item_id: Option<std::num::NonZeroU32>,
    pub pvp_rank_range: (Option<u32>, Option<u32>),
    pub pvp_season_id: Option<std::num::NonZeroU32>,
    pub maximum_pvp_points_to_next_threshold: Option<u32>,
    pub pvp_cap_distance: Option<PvpCapDistanceFilter>,
    pub pvp_target_distance: Option<PvpTargetDistanceFilter>,
    pub player_resource: Option<PlayerResourceFilter>,
    pub player_aura: Option<PlayerAuraFilter>,
    pub weapon_enchant: Option<WeaponEnchantFilter>,
    pub merchant_buyback: Option<MerchantBuybackFilter>,
    pub pet_action_spell: Option<PetActionSpellFilter>,
    pub pet_action_command: Option<PetActionCommandFilter>,
    pub group_context: Option<GroupContextFilter>,
    pub profession_spell_id: Option<std::num::NonZeroU32>,
    pub recipe: RecipeCaptureFilters,
    pub tracked_recipe_id: Option<std::num::NonZeroU32>,
    pub tracked_recipe_kind: Option<TrackedRecipeKind>,
    pub tracked_recipe_learning: Option<RecipeLearning>,
    pub profession_skill_line: Option<std::num::NonZeroU32>,
    pub profession_min_rank: Option<u32>,
    pub profession_max_rank: Option<u32>,
    pub profession_target_rank: Option<std::num::NonZeroU32>,
    pub profession_target_max_remaining: Option<u32>,
    pub profession_target_within_cap_only: bool,
    pub instance: InstanceCaptureFilters<'a>,
    pub player_map_id: Option<std::num::NonZeroU32>,
    pub player_map_name: Option<&'a str>,
    pub player_parent_map_id: Option<std::num::NonZeroU32>,
    pub repair_max_quote_copper: Option<u64>,
    pub repair_min_remaining_copper: Option<u64>,
    pub equipment_durability_max_percent: Option<u8>,
    pub repair_affordable_only: bool,
    pub minimum_free_bag_slots: Option<u32>,
    pub minimum_general_free_bag_slots: Option<u32>,
    pub bag_family_space: Option<BagFamilySpaceFilter>,
    pub pet_food: Option<PetFoodCaptureFilter>,
    pub currency_balance: Option<CurrencyBalanceFilter>,
    pub reputation_rank: Option<ReputationRankFilter>,
    pub renown: Option<RenownFilter>,
    pub paragon: Option<ParagonFilter>,
    pub tracked_achievement: Option<TrackedAchievementFilter>,
    pub collected_mount: Option<CollectedMountFilter>,
    pub collected_mount_name: Option<&'a str>,
    pub collected_mount_spell_id: Option<std::num::NonZeroU32>,
    pub toy_in_view: Option<ToyViewFilter>,
    pub toy_name: Option<&'a str>,
    pub saved_instance: Option<SavedInstanceFilter>,
    pub saved_world_boss: Option<SavedWorldBossFilter>,
    pub instance_lock_warning: Option<saved_resets::InstanceLockWarningFilter>,
    pub profession_recipe: Option<ProfessionRecipeFilter>,
    pub mail_attachment: Option<MailAttachmentFilter>,
    pub pet_species: Option<PetSpeciesFilter>,
    pub pet_species_name: Option<&'a str>,
    pub active_pet: Option<ActivePetCaptureFilter>,
    pub spellbook_spell: Option<SpellbookSpellFilter>,
    pub action_bar_spell_resources: Option<ActionBarSpellResourceFilter>,
    pub minimum_rested_xp: Option<u32>,
    pub xp_gain_enabled_only: bool,
    pub quest_id: Option<std::num::NonZeroU32>,
    pub quest_title: Option<&'a str>,
    pub quest_reward_item_id: Option<std::num::NonZeroU32>,
    pub quest_reward_currency_id: Option<std::num::NonZeroU32>,
    pub quest_reward_renown_faction_id: Option<std::num::NonZeroU32>,
    pub quest_difficulty: Option<analysis::QuestDifficulty>,
    pub quest_max_suggested_group_size: Option<std::num::NonZeroU32>,
    pub quest_min_reward_xp: Option<u64>,
    pub quest_min_reward_honor: Option<u64>,
    pub quest_max_required_copper: Option<u64>,
    pub quest_within_captured_wallet: bool,
    pub quest_objective_kind: Option<&'a str>,
    pub quest_objective_finished: Option<bool>,
    pub quest_objective_remaining: (Option<u32>, Option<u32>),
    pub quest_readiness: Option<analysis::QuestReadiness>,
    pub quest_waypoint_map_id: Option<std::num::NonZeroU32>,
    pub quest_on_captured_map: bool,
    pub local_quest_offset: Option<usize>,
    pub local_quest_soonest_first: bool,
    pub local_quest_highest_xp_first: bool,
    pub quest_max_remaining_seconds: Option<u32>,
    pub quest_projected_expiry_by: Option<chrono::DateTime<chrono::Utc>>,
    pub merchant_item_id: Option<std::num::NonZeroU32>,
    pub merchant_currency_id: Option<std::num::NonZeroU32>,
    pub merchant_pet_food_snapshot_id: Option<&'a str>,
    pub merchant_target_quantity: Option<std::num::NonZeroU32>,
    pub merchant_cheapest_first: bool,
    pub merchant_combine_bundles: bool,
    pub merchant_max_total_copper: Option<u64>,
    pub merchant_max_price_copper: Option<u64>,
    pub merchant_in_stock_only: bool,
    pub merchant_stock_state: Option<MerchantStockState>,
    pub merchant_copper_only: bool,
    pub merchant_affordable_only: bool,
    pub trainer_service_name: Option<&'a str>,
    pub trainer_service_status: Option<&'a str>,
    pub trainer_max_price_copper: Option<u64>,
    pub trainer_max_price_points: Option<u32>,
    pub trainer_affordable_only: bool,
    pub trainer_balance_reserve: Option<u64>,
    pub flight_destination_node_id: Option<std::num::NonZeroU32>,
    pub flight_destination_name: Option<&'a str>,
    pub flight_origin_node_id: Option<std::num::NonZeroU32>,
    pub flight_max_fare_copper: Option<u64>,
    pub flight_departure_map_id: Option<std::num::NonZeroU32>,
    pub flight_affordable_only: bool,
    pub flight_wallet_reserve_copper: Option<u64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PetFoodCaptureFilter {
    /// Count only this base item ID when supplied.
    pub item_id: Option<std::num::NonZeroU32>,
    /// Inclusive minimum accepted units in the captured carried bags.
    pub minimum_quantity: u32,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CurrencyBalanceFilter {
    pub currency_id: Option<std::num::NonZeroU32>,
    /// Exact case-insensitive saved currency name.
    pub currency_name: Option<String>,
    /// Inclusive minimum recorded quantity.
    pub minimum_quantity: u64,
    /// Inclusive minimum room below an observed weekly cap.
    pub minimum_weekly_remaining: Option<u64>,
    /// Inclusive minimum room below an observed total cap.
    pub minimum_maximum_remaining: Option<u64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReputationRankFilter {
    pub faction_id: Option<std::num::NonZeroU32>,
    /// Exact case-insensitive saved faction name.
    pub faction_name: Option<String>,
    /// Inclusive minimum standard reputation reaction, from 1 to 8.
    #[schemars(range(min = 1, max = 8))]
    pub minimum_reaction: u8,
}

fn select_named_entry<'a>(
    entries: &'a serde_json::Map<String, Value>,
    key: Option<&str>,
    name: Option<&str>,
    in_scope: impl Fn(&Value) -> bool,
) -> Option<(u32, &'a Value)> {
    let matches = |row: &Value| {
        in_scope(row)
            && name.is_none_or(|name| {
                row["name"]
                    .as_str()
                    .is_some_and(|label| label.trim().to_lowercase() == name)
            })
    };
    let (id, row) = if let Some(key) = key {
        entries.get_key_value(key).filter(|(_, row)| matches(row))?
    } else {
        let mut matching = entries.iter().filter(|(_, row)| matches(row));
        let first = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        first
    };
    Some((id.parse().ok()?, row))
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenownFilter {
    pub faction_id: Option<std::num::NonZeroU32>,
    /// Exact case-insensitive saved faction name.
    pub faction_name: Option<String>,
    pub minimum_level: u32,
    pub capped: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParagonFilter {
    pub faction_id: Option<std::num::NonZeroU32>,
    /// Exact case-insensitive saved faction name.
    pub faction_name: Option<String>,
    pub reward_pending: Option<bool>,
    pub level_too_low: Option<bool>,
    /// Match the linked reward quest's readiness in the same capture.
    pub reward_quest_readiness: Option<analysis::QuestReadiness>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrackedAchievementFilter {
    pub achievement_id: Option<std::num::NonZeroU32>,
    /// Exact case-insensitive saved tracked achievement name.
    pub achievement_name: Option<String>,
    pub completed: Option<bool>,
    pub earned_by_character: Option<bool>,
    /// Inclusive limit on unfinished criteria for the selected saved achievement.
    pub maximum_unfinished_criteria: Option<u32>,
}

impl Store {
    pub fn list(&self, family: Family, offset: usize, limit: usize) -> Result<Value> {
        self.list_matching(family, None, offset, limit)
    }

    pub fn list_matching(
        &self,
        family: Family,
        character: Option<&Character>,
        offset: usize,
        limit: usize,
    ) -> Result<Value> {
        self.list_matching_recipe_output(family, character, None, offset, limit)
    }

    pub fn list_matching_recipe_output(
        &self,
        family: Family,
        character: Option<&Character>,
        recipe_output_item_id: Option<std::num::NonZeroU32>,
        offset: usize,
        limit: usize,
    ) -> Result<Value> {
        self.list_matching_recipes(
            family,
            character,
            RecipeCaptureFilters {
                output_item_id: recipe_output_item_id,
                ..Default::default()
            },
            offset,
            limit,
        )
    }

    pub fn list_matching_recipes(
        &self,
        family: Family,
        character: Option<&Character>,
        filters: RecipeCaptureFilters,
        offset: usize,
        limit: usize,
    ) -> Result<Value> {
        self.list_matching_captures(
            family,
            character,
            filters,
            InstanceCaptureFilters::default(),
            offset,
            limit,
        )
    }

    pub fn list_matching_captures(
        &self,
        family: Family,
        character: Option<&Character>,
        filters: RecipeCaptureFilters,
        instance: InstanceCaptureFilters<'_>,
        offset: usize,
        limit: usize,
    ) -> Result<Value> {
        self.list_filtered(
            family,
            character,
            CaptureFilters {
                recipe: filters,
                instance,
                ..Default::default()
            },
            offset,
            limit,
        )
    }

    pub fn list_filtered(
        &self,
        family: Family,
        character: Option<&Character>,
        query: CaptureFilters<'_>,
        offset: usize,
        limit: usize,
    ) -> Result<Value> {
        let CaptureFilters {
            exact_client,
            captured_from,
            captured_through,
            latest_per_character,
            required_sections,
            any_missing_sections,
            coverage_summary_sections,
            inventory_item,
            inventory_requirements,
            inventory_requirements_shortfall_only,
            inventory_requirements_bank_plan,
            loot_observation,
            skill_line,
            gossip_option,
            outgoing_trade_item_id,
            pvp_rank_range: (minimum_pvp_rank, maximum_pvp_rank),
            pvp_season_id,
            maximum_pvp_points_to_next_threshold,
            pvp_cap_distance,
            pvp_target_distance,
            player_resource,
            player_aura,
            weapon_enchant,
            merchant_buyback,
            pet_action_spell,
            pet_action_command,
            group_context,
            profession_spell_id,
            recipe: filters,
            tracked_recipe_id,
            tracked_recipe_kind,
            tracked_recipe_learning,
            profession_skill_line,
            profession_min_rank,
            profession_max_rank,
            profession_target_rank,
            profession_target_max_remaining,
            profession_target_within_cap_only,
            instance,
            player_map_id,
            player_map_name,
            player_parent_map_id,
            repair_max_quote_copper,
            repair_min_remaining_copper,
            equipment_durability_max_percent,
            repair_affordable_only,
            minimum_free_bag_slots,
            minimum_general_free_bag_slots,
            bag_family_space,
            pet_food,
            currency_balance,
            reputation_rank,
            renown,
            paragon,
            tracked_achievement,
            collected_mount,
            collected_mount_name,
            collected_mount_spell_id,
            toy_in_view,
            toy_name,
            saved_instance,
            saved_world_boss,
            instance_lock_warning,
            profession_recipe,
            mail_attachment,
            pet_species,
            pet_species_name,
            active_pet,
            spellbook_spell,
            action_bar_spell_resources,
            minimum_rested_xp,
            xp_gain_enabled_only,
            quest_id,
            quest_title,
            quest_reward_item_id,
            quest_reward_currency_id,
            quest_reward_renown_faction_id,
            quest_difficulty,
            quest_max_suggested_group_size,
            quest_min_reward_xp,
            quest_min_reward_honor,
            quest_max_required_copper,
            quest_within_captured_wallet,
            quest_objective_kind,
            quest_objective_finished,
            quest_objective_remaining,
            quest_readiness,
            quest_waypoint_map_id,
            quest_on_captured_map,
            local_quest_offset,
            local_quest_soonest_first,
            local_quest_highest_xp_first,
            quest_max_remaining_seconds,
            quest_projected_expiry_by,
            merchant_item_id,
            merchant_currency_id,
            merchant_pet_food_snapshot_id,
            merchant_target_quantity,
            merchant_cheapest_first,
            merchant_combine_bundles,
            merchant_max_total_copper,
            merchant_max_price_copper,
            merchant_in_stock_only,
            merchant_stock_state,
            merchant_copper_only,
            merchant_affordable_only,
            trainer_service_name,
            trainer_service_status,
            trainer_max_price_copper,
            trainer_max_price_points,
            trainer_affordable_only,
            trainer_balance_reserve,
            flight_destination_node_id,
            flight_destination_name,
            flight_origin_node_id,
            flight_max_fare_copper,
            flight_departure_map_id,
            flight_affordable_only,
            flight_wallet_reserve_copper,
        } = query;
        for sections in [
            required_sections,
            any_missing_sections,
            coverage_summary_sections,
        ]
        .into_iter()
        .flatten()
        {
            validate_section_names(sections)?;
        }
        ensure!(
            inventory_item.is_none() || inventory_requirements.is_none(),
            "Inventory item and inventory requirements filters are mutually exclusive"
        );
        ensure!(
            !inventory_requirements_shortfall_only || inventory_requirements.is_some(),
            "Inventory shortfall discovery requires inventory_requirements"
        );
        ensure!(
            !inventory_requirements_bank_plan
                || (family == Family::Forever
                    && inventory_requirements_shortfall_only
                    && inventory_requirements
                        .as_ref()
                        .is_some_and(|filter| matches!(filter.storage, InventoryStorage::Bags)
                            && filter.container_id.is_none())),
            "Inventory bank planning requires a Forever bag shortfall search"
        );
        if let Some(filter) = &inventory_requirements {
            filter.validate(&family)?;
        }
        if let Some(filter) = &inventory_item {
            filter.validate(&family)?;
        }
        ensure!(
            (collected_mount.is_none() && collected_mount_spell_id.is_none())
                || family == Family::Forever,
            "Mount filtering requires Forever"
        );
        let collected_mount_name = collected_mount_name.map(str::trim);
        ensure!(
            collected_mount_name.is_none_or(|name| family == Family::Forever
                && !name.is_empty()
                && name.len() <= 256),
            "Mount name filtering requires Forever and a valid name"
        );
        let collected_mount_name = collected_mount_name.map(str::to_lowercase);
        ensure!(
            toy_in_view.is_none() || family == Family::Forever,
            "Toy view filtering requires Forever"
        );
        let toy_name = toy_name.map(str::trim);
        ensure!(
            toy_name.is_none_or(|name| family == Family::Forever
                && !name.is_empty()
                && name.len() <= 256),
            "Toy name filtering requires Forever and a valid name"
        );
        let toy_name = toy_name.map(str::to_lowercase);
        ensure!(
            saved_instance.as_ref().is_none_or(|filter| {
                family == Family::Forever
                    && filter
                        .name
                        .as_ref()
                        .is_none_or(|name| !name.trim().is_empty() && name.len() <= 256)
                    && (filter.name.is_some()
                        || filter.difficulty_id.is_some()
                        || filter.locked.is_some()
                        || filter.extended.is_some()
                        || filter.is_raid.is_some()
                        || filter.max_players.is_some()
                        || filter.maximum_reset_seconds.is_some()
                        || filter.projected_reset_from.is_some()
                        || filter.projected_reset_by.is_some()
                        || filter.maximum_update_age_seconds.is_some())
            }),
            "Saved instance filtering requires Forever and at least one valid criterion"
        );
        ensure!(
            instance_lock_warning.is_none() || family == Family::Forever,
            "Instance lock warning filtering requires Forever"
        );
        if let Some(filter) = &instance_lock_warning {
            filter.validate()?;
        }
        let saved_instance_name = saved_instance
            .as_ref()
            .and_then(|filter| filter.name.as_ref())
            .map(|name| name.to_lowercase());
        ensure!(
            saved_world_boss.as_ref().is_none_or(|filter| {
                family == Family::Forever
                    && filter
                        .name
                        .as_ref()
                        .is_none_or(|name| !name.trim().is_empty() && name.len() <= 256)
                    && (filter.name.is_some()
                        || filter.maximum_reset_seconds.is_some()
                        || filter.projected_reset_from.is_some()
                        || filter.projected_reset_by.is_some()
                        || filter.maximum_update_age_seconds.is_some())
            }),
            "Saved world boss filtering requires Forever and at least one valid criterion"
        );
        let saved_world_boss_name = saved_world_boss
            .as_ref()
            .and_then(|filter| filter.name.as_ref())
            .map(|name| name.to_lowercase());
        for (from, through) in saved_instance
            .iter()
            .map(|filter| (filter.projected_reset_from, filter.projected_reset_by))
            .chain(
                saved_world_boss
                    .iter()
                    .map(|filter| (filter.projected_reset_from, filter.projected_reset_by)),
            )
        {
            ensure!(
                from.zip(through)
                    .is_none_or(|(from, through)| from <= through),
                "Projected reset window is reversed"
            );
        }
        ensure!(
            profession_recipe.is_none() || family == Family::Forever,
            "Profession recipe view filtering requires Forever"
        );
        let profession_recipe_key = profession_recipe
            .as_ref()
            .map(|filter| filter.recipe_id.to_string());
        ensure!(
            loot_observation.is_none() || family == Family::Forever,
            "Loot observation filtering requires Forever"
        );
        if let Some(filter) = &loot_observation {
            filter.validate()?;
        }
        ensure!(
            skill_line.is_none() || family == Family::Forever,
            "Skill-line filtering requires Forever"
        );
        if let Some(filter) = &skill_line {
            filter.validate()?;
        }
        ensure!(
            gossip_option.is_none() || family == Family::Forever,
            "Gossip option filtering requires Forever"
        );
        ensure!(
            outgoing_trade_item_id.is_none() || family == Family::Forever,
            "Outgoing trade filtering requires Forever"
        );
        let filter_pvp = minimum_pvp_rank.is_some()
            || maximum_pvp_rank.is_some()
            || pvp_season_id.is_some()
            || maximum_pvp_points_to_next_threshold.is_some()
            || pvp_cap_distance.is_some()
            || pvp_target_distance.is_some();
        ensure!(
            player_aura.is_none() || family == Family::Forever,
            "Player aura search requires Forever"
        );
        ensure!(
            player_aura
                .as_ref()
                .and_then(|filter| filter.minimum_applications.zip(filter.maximum_applications))
                .is_none_or(|(min, max)| min <= max),
            "Invalid player aura application range"
        );
        ensure!(
            weapon_enchant.is_none() || family == Family::Forever,
            "Weapon enchant search requires Forever"
        );
        ensure!(
            weapon_enchant
                .as_ref()
                .and_then(|filter| filter.minimum_charges.zip(filter.maximum_charges))
                .is_none_or(|(min, max)| min <= max),
            "Invalid weapon enchant charge range"
        );
        ensure!(
            merchant_buyback.as_ref().is_none_or(|filter| {
                family == Family::Forever
                    && [
                        filter.maximum_price_copper,
                        filter.minimum_remaining_wallet_copper,
                    ]
                    .into_iter()
                    .flatten()
                    .all(|copper| copper <= 9_999_999_999_999)
            }),
            "Merchant buyback search requires Forever and a valid copper limit"
        );
        ensure!(
            pet_action_spell.is_none() || family == Family::Forever,
            "Pet action spell search requires Forever"
        );
        ensure!(
            pet_action_command.as_ref().is_none_or(|filter| {
                family == Family::Forever && valid_pet_action_command(&filter.command_token)
            }),
            "Pet action command search requires Forever and an exact PET_ command token"
        );
        if let Some(filter) = &player_resource {
            ensure!(
                family == Family::Forever,
                "Player resource search requires Forever"
            );
            ensure!(
                filter.minimum_percent.is_some()
                    || filter.maximum_percent.is_some()
                    || filter.power_type.is_some(),
                "Player resource search requires a percentage bound or power type"
            );
            let min = filter.minimum_percent.unwrap_or(0);
            let max = filter.maximum_percent.unwrap_or(100);
            ensure!(
                min <= max && max <= 100,
                "Invalid player resource percentage range"
            );
            ensure!(
                filter.power_type.is_none_or(|kind| matches!(
                    filter.resource,
                    PlayerResource::PrimaryPower
                ) && kind <= 63),
                "Power type requires primary_power and a value from 0 to 63"
            );
        }
        ensure!(
            !filter_pvp || family == Family::Forever,
            "PvP filtering requires Forever"
        );
        ensure!(
            minimum_pvp_rank
                .zip(maximum_pvp_rank)
                .is_none_or(|(min, max)| min <= max),
            "Reversed PvP rank bounds"
        );
        ensure!(
            group_context.is_none() || family == Family::Forever,
            "Group context filtering requires Forever"
        );
        ensure!(
            group_context
                .as_ref()
                .and_then(|filter| {
                    filter
                        .minimum_reported_member_count
                        .zip(filter.maximum_reported_member_count)
                })
                .is_none_or(|(min, max)| min <= max),
            "Invalid group member count range"
        );
        ensure!(
            profession_spell_id.is_none() || family == Family::Forever,
            "Profession spell filtering requires Forever"
        );
        if let Some(filter) = &gossip_option {
            filter.validate()?;
        }
        ensure!(
            mail_attachment.is_none() || family == Family::Forever,
            "Pending mail attachment filtering requires Forever"
        );
        ensure!(
            mail_attachment.as_ref().is_none_or(|filter| {
                filter.maximum_quantity_above_recorded_headroom.is_none()
                    || matches!(filter.kind, MailAttachmentKind::Currency)
            }),
            "Mail cap headroom filtering requires a currency attachment"
        );
        ensure!(
            mail_attachment.as_ref().is_none_or(|filter| {
                !filter.require_recorded_bag_space
                    || matches!(filter.kind, MailAttachmentKind::Item)
            }),
            "Mail bag space filtering requires an item attachment"
        );
        ensure!(
            pet_species.is_none() || family == Family::Forever,
            "Pet species view filtering requires Forever"
        );
        let pet_species_name = pet_species_name.map(str::trim);
        ensure!(
            pet_species_name.is_none_or(|name| family == Family::Forever
                && !name.is_empty()
                && name.len() <= 256),
            "Pet species name filtering requires Forever and a valid name"
        );
        let pet_species_name = pet_species_name.map(str::to_lowercase);
        if let Some(filter) = &pet_species {
            filter.validate()?;
        }
        ensure!(
            active_pet.is_none() || family == Family::Forever,
            "Active-pet filtering requires Forever"
        );
        if let Some(filter) = &active_pet {
            filter.validate()?;
        }
        ensure!(
            spellbook_spell.is_none() || family == Family::Forever,
            "Spellbook filtering requires Forever"
        );
        if let Some(filter) = &action_bar_spell_resources {
            ensure!(
                family == Family::Forever,
                "Action-bar spell resource filtering requires Forever"
            );
            filter.resources.validate()?;
        }
        if let Some(filter) = &spellbook_spell {
            ensure!(
                filter.spell_id.is_some() || filter.resources.is_some() || filter.name.is_some(),
                "Spell ID, name or resource filter required"
            );
            ensure!(
                filter
                    .name
                    .as_deref()
                    .map(str::trim)
                    .is_none_or(|name| !name.is_empty() && name.len() <= 256),
                "Spell name requires one to 256 bytes after trimming"
            );
            if let Some(resources) = &filter.resources {
                resources.validate()?;
            }
            ensure!(
                filter
                    .minimum_level_learned
                    .zip(filter.maximum_level_learned)
                    .is_none_or(|(a, b)| a <= b),
                "Reversed reported spell learn-level bounds"
            );
        }
        let parse_bound = |value: &str| -> Result<DateTime<Utc>> {
            ensure!(value.len() <= 64, "Capture timestamp is too long");
            Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
        };
        let captured_from = captured_from.map(parse_bound).transpose()?;
        let captured_through = captured_through.map(parse_bound).transpose()?;
        ensure!(
            captured_from
                .zip(captured_through)
                .is_none_or(|(from, through)| from <= through),
            "Capture time bounds are reversed"
        );
        let trainer_service_name = trainer_service_name.map(str::trim);
        ensure!(
            trainer_max_price_copper.is_none_or(
                |copper| copper <= 9_999_999_999_999 && trainer_max_price_points.is_none()
            ),
            "Specify one valid trainer price ceiling"
        );
        let trainer_price = trainer_max_price_copper
            .map(|amount| ("price_copper", amount))
            .or_else(|| {
                trainer_max_price_points.map(|amount| ("price_training_points", u64::from(amount)))
            });
        ensure!(
            trainer_balance_reserve.is_none() || trainer_affordable_only,
            "Trainer balance reserve requires the affordability filter"
        );
        let filter_trainer = trainer_service_name.is_some()
            || trainer_service_status.is_some()
            || trainer_price.is_some()
            || trainer_affordable_only;
        ensure!(
            (!filter_trainer || family == Family::Forever)
                && trainer_service_name.is_none_or(|name| !name.is_empty() && name.len() <= 256)
                && trainer_service_status
                    .is_none_or(|status| matches!(status, "available" | "unavailable" | "used")),
            "Trainer filtering requires Forever and valid lesson filters"
        );
        let trainer_service_name = trainer_service_name.map(str::to_lowercase);
        let flight_destination_name = flight_destination_name.map(str::trim);
        ensure!(
            flight_destination_name.is_none_or(|name| !name.is_empty() && name.len() <= 256),
            "Flight destination name requires one to 256 bytes after trimming"
        );
        let flight_destination_name = flight_destination_name.map(str::to_lowercase);
        let filter_flight_destination =
            flight_destination_node_id.is_some() || flight_destination_name.is_some();
        ensure!(
            flight_departure_map_id.is_none() || filter_flight_destination,
            "Flight departure map filtering requires a destination"
        );
        ensure!(
            !flight_affordable_only || filter_flight_destination,
            "Flight affordability filtering requires a destination"
        );
        ensure!(
            flight_wallet_reserve_copper.is_none_or(|reserve| {
                filter_flight_destination && reserve <= 9_999_999_999_999
            }),
            "Flight wallet reserve requires a destination and a valid copper amount"
        );
        ensure!(
            flight_max_fare_copper
                .is_none_or(|fare| { filter_flight_destination && fare <= 9_999_999_999_999 }),
            "Flight fare filtering requires a destination and a valid copper amount"
        );
        ensure!(
            flight_origin_node_id.is_none() || filter_flight_destination,
            "Flight origin filtering requires a destination"
        );
        ensure!(
            !filter_flight_destination || family == Family::Forever,
            "Flight filtering requires Forever"
        );
        ensure!(
            [
                merchant_item_id.is_some(),
                merchant_currency_id.is_some(),
                merchant_pet_food_snapshot_id.is_some()
            ]
            .into_iter()
            .filter(|selected| *selected)
            .count()
                <= 1,
            "Use one merchant product or pet food selector"
        );
        let merchant_product = merchant_item_id
            .map(|id| ("item_id", id.get()))
            .or_else(|| merchant_currency_id.map(|id| ("currency_id", id.get())));
        let filter_merchant = merchant_product.is_some() || merchant_pet_food_snapshot_id.is_some();
        ensure!(
            !merchant_combine_bundles
                || (merchant_product.is_some() && merchant_target_quantity.is_some()),
            "Merchant bundle combinations require an exact item or currency and target quantity"
        );
        ensure!(
            merchant_target_quantity
                .is_none_or(|quantity| filter_merchant && quantity.get() <= 1000),
            "Merchant quantity filtering requires an offer selector and up to 1000 units"
        );
        ensure!(
            !(merchant_cheapest_first || merchant_max_total_copper.is_some())
                || merchant_target_quantity.is_some(),
            "Merchant cost ordering and total cost limits require a target quantity"
        );
        ensure!(
            !filter_merchant || family == Family::Forever,
            "Merchant filtering requires Forever"
        );
        ensure!(
            !(merchant_in_stock_only
                || merchant_copper_only
                || merchant_affordable_only
                || merchant_stock_state.is_some())
                || filter_merchant,
            "Merchant stock and cost filters require an offer selector"
        );
        ensure!(
            merchant_max_price_copper
                .is_none_or(|price| filter_merchant && price <= 9_999_999_999_999),
            "Merchant price filtering requires an offer selector and a valid copper amount"
        );
        let food_capture = merchant_pet_food_snapshot_id
            .map(|id| self.read(id))
            .transpose()?;
        let accepted_food = food_capture
            .as_ref()
            .map(|capture| {
                ensure!(
                    capture.client.family == Family::Forever,
                    "Expected Forever pet food capture"
                );
                capture
                    .sections
                    .get("pet_care")
                    .and_then(|pet| pet["carried_food"].as_object())
                    .ok_or_else(|| anyhow::anyhow!("Missing food classification"))
            })
            .transpose()?;
        let filter_recipes = filters.recipe_id.is_some()
            || filters.output_item_id.is_some()
            || filters.profession_id.is_some()
            || filters.cooldown_state.is_some();
        let filter_tracking = tracked_recipe_id.is_some()
            || tracked_recipe_kind.is_some()
            || tracked_recipe_learning.is_some();
        ensure!(
            !(filter_recipes || filter_tracking) || family == Family::Forever,
            "Recipe filtering requires Forever"
        );
        let tracked_recipe_key = tracked_recipe_id.map(|id| id.to_string());
        let matches_learning = |row: &Value| {
            tracked_recipe_learning
                .is_none_or(|learning| row["learned"].as_bool() == learning.observed())
        };
        let craft_enabled = !matches!(tracked_recipe_kind, Some(TrackedRecipeKind::Recraft));
        let recraft_enabled = !matches!(tracked_recipe_kind, Some(TrackedRecipeKind::Craft));
        ensure!(
            profession_skill_line.is_none_or(|_| family == Family::Forever)
                && (profession_skill_line.is_some()
                    || (profession_min_rank.is_none()
                        && profession_max_rank.is_none()
                        && profession_target_rank.is_none()))
                && profession_min_rank
                    .zip(profession_max_rank)
                    .is_none_or(|(min, max)| min <= max),
            "Profession filtering requires Forever, a skill line and ordered rank bounds"
        );
        ensure!(
            profession_target_rank.is_some()
                || (profession_target_max_remaining.is_none()
                    && !profession_target_within_cap_only),
            "Profession target filtering requires a target rank"
        );
        let filter_instance = instance.kind.is_some()
            || instance.name.is_some()
            || instance.difficulty_id.is_some()
            || instance.map_id.is_some();
        ensure!(
            !filter_instance || family == Family::Forever,
            "Instance filtering requires Forever"
        );
        ensure!(
            instance.kind.is_none_or(|kind| matches!(
                kind,
                "none" | "party" | "raid" | "pvp" | "arena" | "scenario"
            )) && instance
                .name
                .is_none_or(|name| !name.trim().is_empty() && name.len() <= 256)
                && (instance.kind != Some("none")
                    || (instance.name.is_none()
                        && instance.difficulty_id.is_none()
                        && instance.map_id.is_none())),
            "Invalid instance filters"
        );
        let instance_name = instance.name.map(str::to_lowercase);
        let player_map_name = player_map_name.map(str::trim);
        ensure!(
            player_map_name.is_none_or(|name| !name.is_empty() && name.len() <= 256),
            "Invalid captured player map name"
        );
        let player_map_name = player_map_name.map(str::to_lowercase);
        let filter_player_map =
            player_map_id.is_some() || player_parent_map_id.is_some() || player_map_name.is_some();
        let filter_repair = repair_max_quote_copper.is_some()
            || repair_affordable_only
            || repair_min_remaining_copper.is_some();
        ensure!(
            equipment_durability_max_percent
                .is_none_or(|maximum| family == Family::Forever && maximum <= 100),
            "Durability filtering requires Forever and a percentage from 0 to 100"
        );
        let bag_family_space = bag_family_space.map(|filter| {
            (
                filter.family_mask.to_string(),
                u64::from(filter.minimum_free_slots),
            )
        });
        let filter_bag_space = minimum_free_bag_slots.is_some()
            || minimum_general_free_bag_slots.is_some()
            || bag_family_space.is_some();
        ensure!(
            !filter_bag_space || family == Family::Forever,
            "Bag space filtering requires Forever"
        );
        ensure!(
            pet_food.is_none() || family == Family::Forever,
            "Pet food filtering requires Forever"
        );
        let pet_food_key = pet_food
            .as_ref()
            .and_then(|filter| filter.item_id.map(|id| id.to_string()));
        let filter_experience = minimum_rested_xp.is_some() || xp_gain_enabled_only;
        ensure!(
            currency_balance
                .as_ref()
                .is_none_or(|filter| family == Family::Forever
                    && (filter.currency_id.is_some() || filter.currency_name.is_some())
                    && filter
                        .currency_name
                        .as_deref()
                        .map(str::trim)
                        .is_none_or(|name| !name.is_empty() && name.len() <= 256)),
            "Currency balance filtering requires Forever and a currency ID or valid name"
        );
        let currency_key = currency_balance
            .as_ref()
            .and_then(|filter| filter.currency_id.map(|id| id.to_string()));
        let currency_name = currency_balance
            .as_ref()
            .and_then(|filter| filter.currency_name.as_deref())
            .map(str::trim)
            .map(str::to_lowercase);
        ensure!(
            reputation_rank.as_ref().is_none_or(|filter| {
                family == Family::Forever
                    && (1..=8).contains(&filter.minimum_reaction)
                    && (filter.faction_id.is_some() || filter.faction_name.is_some())
                    && filter
                        .faction_name
                        .as_deref()
                        .map(str::trim)
                        .is_none_or(|name| !name.is_empty() && name.len() <= 256)
            }),
            "Reputation filtering requires Forever, a faction ID or name and a standard reaction from 1 to 8"
        );
        let reputation_key = reputation_rank
            .as_ref()
            .and_then(|filter| filter.faction_id.map(|id| id.to_string()));
        let reputation_name = reputation_rank
            .as_ref()
            .and_then(|filter| filter.faction_name.as_deref())
            .map(str::trim)
            .map(str::to_lowercase);
        let special_faction = |id: Option<std::num::NonZeroU32>, name: Option<&str>| {
            let name = name.map(str::trim);
            ensure!(
                family == Family::Forever
                    && (id.is_some() || name.is_some())
                    && name.is_none_or(|name| !name.is_empty() && name.len() <= 256),
                "Renown and paragon filtering require Forever and a faction ID or valid name"
            );
            Ok((id.map(|id| id.to_string()), name.map(str::to_lowercase)))
        };
        let (renown_key, renown_name) = renown
            .as_ref()
            .map(|filter| special_faction(filter.faction_id, filter.faction_name.as_deref()))
            .transpose()?
            .unwrap_or_default();
        let (paragon_key, paragon_name) = paragon
            .as_ref()
            .map(|filter| special_faction(filter.faction_id, filter.faction_name.as_deref()))
            .transpose()?
            .unwrap_or_default();
        ensure!(
            tracked_achievement
                .as_ref()
                .is_none_or(|filter| family == Family::Forever
                    && (filter.achievement_id.is_some() || filter.achievement_name.is_some())
                    && filter
                        .achievement_name
                        .as_deref()
                        .map(str::trim)
                        .is_none_or(|name| !name.is_empty() && name.len() <= 256)),
            "Tracked achievement filtering requires Forever and an achievement ID or valid name"
        );
        let achievement_key = tracked_achievement
            .as_ref()
            .and_then(|filter| filter.achievement_id.map(|id| id.to_string()));
        let achievement_name = tracked_achievement
            .as_ref()
            .and_then(|filter| filter.achievement_name.as_deref())
            .map(str::trim)
            .map(str::to_lowercase);
        let quest_title = quest_title.map(str::trim);
        ensure!(
            quest_title.is_none_or(|title| !title.is_empty() && title.len() <= 256),
            "Invalid saved quest title"
        );
        let quest_title = quest_title.map(str::to_lowercase);
        let (objective_min, objective_max) = quest_objective_remaining;
        let filter_quest_objectives = quest_objective_kind.is_some()
            || quest_objective_finished.is_some()
            || objective_min.is_some()
            || objective_max.is_some();
        ensure!(
            quest_objective_kind.is_none_or(|kind| !kind.is_empty() && kind.len() <= 64)
                && objective_min
                    .zip(objective_max)
                    .is_none_or(|(min, max)| min <= max),
            "Invalid saved quest objective filter"
        );
        let filter_quest_group =
            quest_difficulty.is_some() || quest_max_suggested_group_size.is_some();
        let filter_quest_budget = quest_min_reward_xp.is_some()
            || quest_min_reward_honor.is_some()
            || quest_max_required_copper.is_some()
            || quest_within_captured_wallet;
        let filter_quest_timer =
            quest_max_remaining_seconds.is_some() || quest_projected_expiry_by.is_some();
        ensure!(
            (quest_id.is_none()
                && quest_title.is_none()
                && quest_reward_item_id.is_none()
                && quest_reward_currency_id.is_none()
                && quest_reward_renown_faction_id.is_none()
                && quest_readiness.is_none()
                && quest_waypoint_map_id.is_none()
                && !quest_on_captured_map
                && !filter_quest_timer
                && !filter_quest_objectives
                && !filter_quest_group
                && !filter_quest_budget)
                || family == Family::Forever,
            "Quest filtering requires Forever"
        );
        ensure!(
            !quest_on_captured_map || quest_waypoint_map_id.is_none(),
            "Choose either a waypoint map ID or the captured player map"
        );
        ensure!(
            (local_quest_offset.is_none()
                && !local_quest_soonest_first
                && !local_quest_highest_xp_first)
                || quest_on_captured_map,
            "Local quest paging and ordering require the captured player map filter"
        );
        ensure!(
            !local_quest_soonest_first || !local_quest_highest_xp_first,
            "Choose either timer or XP ordering for local quests"
        );
        let quest_key = quest_id.map(|id| id.to_string());
        ensure!(
            !filter_experience || family == Family::Forever,
            "Experience filtering requires Forever"
        );
        ensure!(
            (!filter_repair || family == Family::Forever)
                && repair_max_quote_copper.is_none_or(|price| price <= 9_999_999_999_999)
                && repair_min_remaining_copper.is_none_or(|reserve| reserve <= 9_999_999_999_999),
            "Repair quote filtering requires Forever and a valid copper amount"
        );
        ensure!(
            !filter_player_map || family == Family::Forever,
            "Player map filtering requires Forever"
        );
        ensure!((1..=100).contains(&limit), "Limit must be 1..100");
        if let Some(character) = character {
            ensure!(
                [&character.name, &character.realm]
                    .into_iter()
                    .all(|value| !value.trim().is_empty() && value.len() <= 128),
                "Invalid character identity"
            );
        }
        let coverage_summary = |observed: &[[usize; 3]], matching_count| {
            coverage_summary_sections.map(|names| {
                let sections: BTreeMap<_, _> = names.iter().enumerate().map(|(index, name)| {
                    let [observed, unavailable, missing] = observed.get(index).copied().unwrap_or_default();
                    (name, json!({"observed":observed,"reported_unavailable":unavailable,"not_recorded":missing}))
                }).collect();
                json!({"matching_snapshot_count":matching_count,"sections":sections})
            })
        };
        if !self.root.exists() {
            let mut result = json!({"snapshots":[],"has_more":false,"invalid_count":0});
            if let Some(summary) = coverage_summary(&[], 0) {
                result["section_coverage_summary"] = summary;
            }
            return Ok(result);
        }
        // Resolve once per scan; every file still gets containment/hash/schema checks.
        let root = self.root.canonicalize()?;
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(&root)? {
            let path = entry?.path();
            if path.extension().is_some_and(|x| x == "json") {
                paths.push(path);
            }
            ensure!(
                paths.len() <= 10000,
                "Store scan limit reached; archive older snapshots"
            );
        }
        paths.sort();
        if let Some(client) = exact_client {
            ensure!(
                client.family == family
                    && client.interface > 0
                    && [&client.variant, &client.build]
                        .iter()
                        .all(|text| !text.trim().is_empty() && text.len() <= 128),
                "Invalid exact client identity"
            );
        }
        let mut rows = Vec::new();
        let mut latest = BTreeMap::new();
        let mut invalid = 0;
        for path in paths {
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                invalid += 1;
                continue;
            };
            let snapshot = Self::read_in_root(&root, id);
            if latest_per_character
                && let Ok(snapshot) = &snapshot
                && snapshot.client.family == family
                && exact_client.is_none_or(|client| snapshot.client == *client)
                && captured_from.is_none_or(|from| snapshot.captured_at >= from)
                && captured_through.is_none_or(|through| snapshot.captured_at <= through)
            {
                let key = (
                    snapshot.character.name.to_lowercase(),
                    snapshot.character.realm.to_lowercase(),
                    snapshot.client.variant.clone(),
                );
                let candidate = (snapshot.captured_at, id.to_owned());
                let current = latest.entry(key).or_insert_with(|| candidate.clone());
                if candidate.0 > current.0 || (candidate.0 == current.0 && candidate.1 < current.1)
                {
                    *current = candidate;
                }
            }
            match snapshot {
                Ok(s)
                    if s.client.family == family
                        && exact_client.is_none_or(|client| s.client == *client)
                        && captured_from.is_none_or(|from| s.captured_at >= from)
                        && captured_through.is_none_or(|through| s.captured_at <= through)
                        && (!filter_instance
                            || s.sections.get("instance_context").is_some_and(|context| {
                                instance
                                    .kind
                                    .is_none_or(|kind| context["kind"].as_str() == Some(kind))
                                    && instance_name.as_ref().is_none_or(|name| {
                                        context["name"]
                                            .as_str()
                                            .is_some_and(|value| value.to_lowercase() == *name)
                                    })
                                    && instance.difficulty_id.is_none_or(|id| {
                                        context["difficulty_id"].as_u64() == Some(u64::from(id))
                                    })
                                    && instance.map_id.is_none_or(|id| {
                                        context["instance_map_id"].as_u64()
                                            == Some(u64::from(id.get()))
                                    })
                            }))
                        && filters.recipe_id.is_none_or(|id| {
                            s.sections.get("selected_recipe").is_some_and(|recipe| {
                                recipe["recipe_id"].as_u64() == Some(u64::from(id.get()))
                            })
                        })
                        && filters.cooldown_state.is_none_or(|state| {
                            s.sections.get("selected_recipe").is_some_and(|recipe| {
                                analysis::recipe_cooldown_state(recipe) == state
                            })
                        })
                        && filters.output_item_id.is_none_or(|id| {
                            s.sections.get("selected_recipe").is_some_and(|recipe| {
                                recipe["output_item_id"].as_u64() == Some(u64::from(id.get()))
                            })
                        })
                        && filters.profession_id.is_none_or(|id| {
                            s.sections.get("selected_recipe").is_some_and(|recipe| {
                                recipe["profession"]["profession_id"].as_u64()
                                    == Some(u64::from(id.get()))
                            })
                        })
                        && character.is_none_or(|target| {
                            s.character.name.to_lowercase() == target.name.to_lowercase()
                                && s.character.realm.to_lowercase() == target.realm.to_lowercase()
                        }) =>
                {
                    let inventory_item_match = inventory_item
                        .as_ref()
                        .and_then(|filter| filter.observation_match(&s));
                    if inventory_item.is_some() && inventory_item_match.is_none() {
                        continue;
                    }
                    let inventory_requirements_match =
                        inventory_requirements.as_ref().and_then(|filter| {
                            if !inventory_requirements_shortfall_only {
                                return filter.observation_match(&s);
                            }
                            let review =
                                filter.review_stock(&s, inventory_requirements_bank_plan, None);
                            (review["all_requirements_covered_by_observed_quantities"] == false)
                                .then_some(review)
                        });
                    if inventory_requirements.is_some() && inventory_requirements_match.is_none() {
                        continue;
                    }
                    let loot_observation_match = loot_observation
                        .as_ref()
                        .and_then(|filter| filter.observation_match(&s));
                    if loot_observation.is_some() && loot_observation_match.is_none() {
                        continue;
                    }
                    let skill_line_match = skill_line
                        .as_ref()
                        .and_then(|filter| filter.observation_match(&s));
                    if skill_line.is_some() && skill_line_match.is_none() {
                        continue;
                    }
                    let gossip_option_match = gossip_option
                        .as_ref()
                        .and_then(|filter| filter.observation_match(&s));
                    if gossip_option.is_some() && gossip_option_match.is_none() {
                        continue;
                    }
                    let outgoing_trade_item_match = outgoing_trade_item_id.and_then(|id| {
                        let section = s.sections.get("player_trade_offer")?;
                        let (slots, quantities) = analysis::observed_trade_offer_quantities(section);
                        let quantity = quantities.get(&id.get())?;
                        Some(json!({"item_id":id,"observed_offered_quantity":quantity,
                            "readable_item_slots":slots,"unreadable_or_empty_slots":6-slots,
                            "trade_completion_verified":false,"counts_toward_owned_inventory":false}))
                    });
                    if outgoing_trade_item_id.is_some() && outgoing_trade_item_match.is_none() {
                        continue;
                    }
                    let profession_spell_match = if let Some(id) = profession_spell_id {
                        let Some(section) = s.sections.get("profession_spells") else {
                            continue;
                        };
                        if section["entries"].get(id.to_string()).is_none() {
                            continue;
                        }
                        Some(
                            json!({"spell_id":id,"profession_id":section["profession_id"],
                            "current_knowledge_verified":false}),
                        )
                    } else {
                        None
                    };
                    if required_sections.is_some_and(|sections| {
                        sections.iter().any(|name| !s.sections.contains_key(name))
                    }) {
                        continue;
                    }
                    let missing_section_matches = any_missing_sections.map(|sections| {
                        sections
                            .iter()
                            .filter(|name| !s.sections.contains_key(*name))
                            .map(|name| (name, s.section_capture_status(name)))
                            .collect::<BTreeMap<_, _>>()
                    });
                    if missing_section_matches
                        .as_ref()
                        .is_some_and(BTreeMap::is_empty)
                    {
                        continue;
                    }
                    let group_context_match = group_context
                        .as_ref()
                        .and_then(|filter| filter.observation_match(&s));
                    if group_context.is_some() && group_context_match.is_none() {
                        continue;
                    }
                    let player_resource_match = player_resource.as_ref().and_then(|filter| {
                        filter.observation_match(s.sections.get("player_resources")?)
                    });
                    if player_resource.is_some() && player_resource_match.is_none() {
                        continue;
                    }
                    let player_aura_match = player_aura.as_ref().and_then(|filter| {
                        filter.observation_match(s.sections.get("player_auras")?)
                    });
                    if player_aura.is_some() && player_aura_match.is_none() {
                        continue;
                    }
                    let weapon_enchant_match = weapon_enchant
                        .as_ref()
                        .and_then(|filter| filter.observation_match(&s));
                    if weapon_enchant.is_some() && weapon_enchant_match.is_none() {
                        continue;
                    }
                    let buyback_match = merchant_buyback
                        .as_ref()
                        .and_then(|filter| filter.observation_match(&s));
                    if merchant_buyback.is_some() && buyback_match.is_none() {
                        continue;
                    }
                    let pet_action_match = pet_action_spell.as_ref().and_then(|filter| {
                        filter.observation_match(s.sections.get("pet_action_bar")?)
                    });
                    if pet_action_spell.is_some() && pet_action_match.is_none() {
                        continue;
                    }
                    let pet_command_match = pet_action_command.as_ref().and_then(|filter| {
                        filter.observation_match(s.sections.get("pet_action_bar")?)
                    });
                    if pet_action_command.is_some() && pet_command_match.is_none() {
                        continue;
                    }
                    let pvp_rank_match = if filter_pvp {
                        let Some(progression) = s.sections.get("player_pvp_progression") else {
                            continue;
                        };
                        let rank = progression["rank"].as_u64().unwrap();
                        if minimum_pvp_rank.is_some_and(|min| rank < u64::from(min))
                            || maximum_pvp_rank.is_some_and(|max| rank > u64::from(max))
                            || pvp_season_id.is_some_and(|id| {
                                progression["reported_season_id"].as_u64()
                                    != Some(u64::from(id.get()))
                            })
                        {
                            continue;
                        }
                        let mut observation = json!({"rank":rank,"faction_id":progression["faction_id"],
                            "coverage":progression["coverage"],"current_progression_verified":false});
                        if let Some(season) = progression.get("reported_season_id") {
                            observation["reported_season_id"] = season.clone();
                        }
                        if let Some(maximum) = maximum_pvp_points_to_next_threshold {
                            let Some(remaining) =
                                analysis::remaining_pvp_points_to_next_threshold(progression)
                                    .filter(|points| *points <= u64::from(maximum))
                            else {
                                continue;
                            };
                            observation["remaining_points_to_reported_next_threshold"] =
                                json!(remaining);
                        }
                        if let Some(filter) = &pvp_cap_distance {
                            let Some(cap) = filter.observation_match(progression) else {
                                continue;
                            };
                            observation["cap_distance"] = cap;
                        }
                        if let Some(filter) = &pvp_target_distance {
                            let Some(target) = filter.observation_match(progression) else {
                                continue;
                            };
                            observation["target_distance"] = target;
                        }
                        Some(observation)
                    } else {
                        None
                    };
                    let pet_species_match = pet_species.as_ref().and_then(|filter| {
                        filter.match_view(s.sections.get("pet_collection_view")?)
                    });
                    if pet_species.is_some() && pet_species_match.is_none() {
                        continue;
                    }
                    let pet_name_match = pet_species_name.as_ref().and_then(|name| {
                        let view = s.sections.get("pet_collection_view")?;
                        let mut matches: Vec<_> = view["entries"].as_object()?.iter().filter_map(|(id, row)| {
                            let id = id.parse::<u32>().ok()?;
                            (pet_species.as_ref().is_none_or(|filter| filter.species_id.get() == id)
                                && row["name"].as_str().is_some_and(|observed| observed.trim().to_lowercase() == *name))
                                .then_some((id, row))
                        }).collect();
                        matches.sort_unstable_by_key(|(id, _)| *id);
                        (!matches.is_empty()).then(|| json!({
                            "coverage":view["coverage"],"matched_count":matches.len(),"truncated":matches.len()>20,
                            "species":matches.into_iter().take(20).map(|(id,row)| json!({
                                "species_id":id,"name":row["name"],"owned_rows":row["owned_rows"],
                                "unowned_rows":row["unowned_rows"],
                            })).collect::<Vec<_>>(),
                        }))
                    });
                    if pet_species_name.is_some() && pet_name_match.is_none() {
                        continue;
                    }
                    let active_pet_match = active_pet
                        .as_ref()
                        .and_then(|filter| filter.observation_match(s.sections.get("pet_care")?));
                    if active_pet.is_some() && active_pet_match.is_none() {
                        continue;
                    }
                    let action_bar_spell_resource_match = action_bar_spell_resources.as_ref().and_then(|filter| {
                        let actions = s.sections.get("action_bar_observation")?;
                        let report = analysis::action_bar_spell_resource_matches(
                            actions, s.sections.get("spellbook"), &filter.resources,
                            Some(u64::from(filter.spell_id.get())),
                        )?;
                        let matched = report["matches"].get(filter.spell_id.to_string())?;
                        Some(serde_json::json!({
                            "spell_id": filter.spell_id,
                            "action_slots": matched["action_slots"],
                            "matching_spellbook_slots": matched["matching_spellbook_slots"],
                            "nonmatching_spellbook_slots": matched["nonmatching_spellbook_slots"],
                            "unknown_spellbook_slots": matched["unknown_spellbook_slots"],
                            "cast_eligibility_verified": false
                        }))
                    });
                    if action_bar_spell_resources.is_some()
                        && action_bar_spell_resource_match.is_none()
                    {
                        continue;
                    }
                    let spellbook_match=spellbook_spell.as_ref().and_then(|filter| {
                        let book=s.sections.get("spellbook")?;
                        let requested=filter.spell_id.map(|id|u64::from(id.get()));
                        let name=filter.name.as_deref().map(str::trim).map(str::to_lowercase);
                        let mut resource_unknown=0;
                        let mut resource_nonmatching=0;
                        let mut matches=book["entries"].as_object()?.iter().filter(|(_,row)| {
                            (requested.is_none() || row["base_spell_id"].as_u64()==requested || row["spell_id"].as_u64()==requested)
                                && name.as_ref().is_none_or(|name| row["name"].as_str()
                                    .is_some_and(|label| label.trim().to_lowercase() == *name))
                                && filter.passive.is_none_or(|flag|row["passive"].as_bool()==Some(flag))
                                && filter.off_spec.is_none_or(|flag|row["off_spec"].as_bool()==Some(flag))
                                && filter.usable.is_none_or(|flag|row["usability"]["usable"].as_bool()==Some(flag))
                                && filter.insufficient_power.is_none_or(|flag|row["usability"]["insufficient_power"].as_bool()==Some(flag))
                                && filter.minimum_level_learned.is_none_or(|minimum|row["level_learned"].as_u64().is_some_and(|n|n>=u64::from(minimum)))
                                && filter.maximum_level_learned.is_none_or(|maximum|row["level_learned"].as_u64().is_some_and(|n|n<=u64::from(maximum)))
                        }).filter(|(_,row)| {
                            match filter.resources.as_ref().map(|resources|resources.matches(row)) {
                                Some(Some(false)) => {resource_nonmatching+=1;false},
                                Some(None) => {resource_unknown+=1;false},
                                _ => true,
                            }
                        }).map(|(slot,row)|(slot.parse::<u32>().unwrap(),row)).collect::<Vec<_>>();
                        if matches.is_empty() {return None;}
                        matches.sort_by_key(|(slot,_)|*slot);
                        let mut matched=json!({"coverage":book["coverage"],"spell_id":filter.spell_id,
                            "matched_slots":matches.len(),"truncated":matches.len()>20,
                            "entries":matches.iter().take(20).map(|(slot,row)| {
                                let mut observed=json!({"slot":slot,"base_spell_id":row["base_spell_id"],"spell_id":row["spell_id"],
                                    "passive":row["passive"],"off_spec":row["off_spec"],"level_learned":row["level_learned"]});
                                if let Some(usability)=row.get("usability") {observed["usability"]=usability.clone();}
                                if filter.resources.is_some() {
                                    for field in ["charge_count","reported_cast_count","cooldown_state"] {observed[field]=row[field].clone();}
                                }
                                observed
                            }).collect::<Vec<_>>()});
                        if let Some(resources)=&filter.resources {
                            matched["resources"]=json!(resources);
                            matched["resource_unknown_slots"]=json!(resource_unknown);
                            matched["resource_nonmatching_slots"]=json!(resource_nonmatching);
                            matched["cast_eligibility_verified"]=json!(false);
                        }
                        Some(matched)
                    });
                    if spellbook_spell.is_some() && spellbook_match.is_none() {
                        continue;
                    }
                    let mail_attachment_match = mail_attachment.as_ref().and_then(|filter| {
                        let mail=s.sections.get("mail_attachments")?;
                        let unloaded = analysis::unloaded_mail_messages(mail);
                        if filter.require_fully_readable_capture && unloaded != 0 {
                            return None;
                        }
                        let (kind,field)=match filter.kind {
                            MailAttachmentKind::Item=>("item","item_id"),
                            MailAttachmentKind::Currency=>("currency","currency_id"),
                        };
                        let mut locations=Vec::new();
                        let mut unknown_slots=0;
                        for (message,attachments) in mail["messages"].as_object()?.iter() {
                            for (slot,row) in attachments.as_object()?.iter() {
                                unknown_slots += usize::from(row["kind"] == "unknown");
                                if row["kind"]==kind && row[field].as_u64()==Some(u64::from(filter.id.get())) {
                                    locations.push((message.parse::<u32>().unwrap(),slot.parse::<u32>().unwrap(),row["count"].as_u64().unwrap()));
                                }
                            }
                        }
                        // Validated coverage bounds this sum to 100 * 16 * u32::MAX.
                        let quantity:u64=locations.iter().map(|(_,_,count)| count).sum();
                        if filter.require_fully_readable_capture && unknown_slots != 0 {
                            return None;
                        }
                        if locations.is_empty() || quantity<filter.minimum_quantity.unwrap_or(1) {return None;}
                        let projection = if let Some(maximum) = filter.maximum_quantity_above_recorded_headroom {
                            let balance = s.sections.get("currency_balances")?["entries"].get(filter.id.to_string())?;
                            let caps = analysis::currency_entry_cap_headroom(balance)?;
                            let projection = analysis::currency_reward_projection(quantity, balance, &caps);
                            if projection["quantity_above_recorded_headroom"].as_u64()? > maximum {
                                return None;
                            }
                            Some(projection)
                        } else {
                            None
                        };
                        let bag_space = if filter.require_recorded_bag_space {
                            let space = analysis::incoming_items_bag_space(
                                &s, std::iter::once((filter.id.get(), quantity)),
                                "matching_loaded_mail_items_only", "known_pending_quantity",
                            );
                            if space["fits_recorded_general_slots"] != true {
                                return None;
                            }
                            Some(space)
                        } else {
                            None
                        };
                        locations.sort_by_key(|(message,slot,_)| (*message,*slot));
                        let mut report = json!({"coverage":mail["coverage"],"loaded_messages":mail["loaded_messages"],
                            "total_messages":mail["total_messages"],"kind":filter.kind,"id":filter.id,
                            "unloaded_messages":unloaded,
                            "unknown_attachment_slots":unknown_slots,
                            "quantity":quantity,"matched_locations":locations.len(),"truncated":locations.len()>20,
                            "locations":locations.iter().take(20).map(|(message,slot,count)|json!({
                                "message_slot":message,"attachment_slot":slot,"count":count})).collect::<Vec<_>>()});
                        if let Some(projection) = projection {
                            report["hypothetical_cap_projection"] = projection;
                            report["mail_cap_application_verified"] = json!(false);
                        }
                        if let Some(space) = bag_space {
                            report["bag_space_projection"] = space;
                            report["retrieval_eligibility_verified"] = json!(false);
                        }
                        Some(report)
                    });
                    if mail_attachment.is_some() && mail_attachment_match.is_none() {
                        continue;
                    }
                    let profession_recipe_match = profession_recipe.as_ref().zip(profession_recipe_key.as_ref()).and_then(|(filter, key)| {
                        let view = s.sections.get("profession_recipe_view")?;
                        let row = view["entries"].get(key)?;
                        (filter.profession_id.is_none_or(|id| view["profession_id"].as_u64() == Some(u64::from(id.get())))
                            && filter.learning.is_none_or(|state| row["learned"].as_bool() == state.observed())
                            && filter.can_skill_up.is_none_or(|flag| row["can_skill_up"].as_bool()==Some(flag))
                            && filter.minimum_skill_ups_reported.is_none_or(|minimum|row["skill_ups_reported"].as_u64().is_some_and(|n|n>=u64::from(minimum))))
                            .then(|| {
                                let mut observation=json!({"coverage":view["coverage"],"profession_id":view["profession_id"],
                                    "recipe_id":filter.recipe_id,"name":row["name"],"learned":row["learned"]});
                                for field in ["relative_difficulty","can_skill_up","skill_ups_reported"] {
                                    if let Some(value)=row.get(field) { observation[field]=value.clone(); }
                                }
                                observation
                            })
                    });
                    if profession_recipe.is_some() && profession_recipe_match.is_none() {
                        continue;
                    }
                    let instance_lock_warning_match =
                        instance_lock_warning.as_ref().and_then(|filter| {
                            filter.match_observation(s.sections.get("instance_context")?)
                        });
                    if instance_lock_warning.is_some() && instance_lock_warning_match.is_none() {
                        continue;
                    }
                    let saved_world_boss_match = saved_world_boss.as_ref().and_then(|filter| {
                        let saved = s.sections.get("saved_world_bosses")?;
                        if !filter.maximum_update_age_seconds.is_none_or(|maximum| {
                            saved["update_age_seconds"]
                                .as_u64()
                                .is_some_and(|age| age <= u64::from(maximum))
                        }) {
                            return None;
                        }
                        let matches = saved["entries"]
                            .as_object()?
                            .iter()
                            .filter(|(_, row)| {
                                saved_world_boss_name.as_ref().is_none_or(|name| {
                                    row["name"]
                                        .as_str()
                                        .is_some_and(|value| value.to_lowercase() == *name)
                                }) && filter.maximum_reset_seconds.is_none_or(|maximum| {
                                    row["reset_seconds"]
                                        .as_u64()
                                        .is_some_and(|seconds| seconds <= u64::from(maximum))
                                }) && saved_resets::matches_projected_window(
                                    row["reset_seconds"].as_i64(),
                                    s.captured_at,
                                    filter.projected_reset_from,
                                    filter.projected_reset_by,
                                )
                            })
                            .collect();
                        saved_resets::discovery_report(
                            matches,
                            saved,
                            s.captured_at,
                            filter.projected_reset_from.is_some()
                                || filter.projected_reset_by.is_some(),
                        )
                    });
                    if saved_world_boss.is_some() && saved_world_boss_match.is_none() {
                        continue;
                    }
                    let saved_instance_match =
                        saved_instance.as_ref().and_then(|filter| {
                            let saved = s.sections.get("saved_instances")?;
                            if !filter.maximum_update_age_seconds.is_none_or(|maximum| {
                                saved["update_age_seconds"]
                                    .as_u64()
                                    .is_some_and(|age| age <= u64::from(maximum))
                            }) {
                                return None;
                            }
                            let matches =
                                saved["entries"]
                                    .as_object()?
                                    .iter()
                                    .filter(|(_, row)| {
                                        saved_instance_name.as_ref().is_none_or(|name| {
                                            row["name"]
                                                .as_str()
                                                .is_some_and(|value| value.to_lowercase() == *name)
                                        }) && filter.max_players.is_none_or(|size| {
                                            row["max_players"].as_u64()
                                                == Some(u64::from(size.get()))
                                        }) && filter.difficulty_id.is_none_or(|id| {
                                            row["difficulty_id"].as_u64() == Some(u64::from(id))
                                        }) && filter.locked.is_none_or(|flag| {
                                            row["locked"].as_bool() == Some(flag)
                                        }) && filter.extended.is_none_or(|flag| {
                                            row["extended"].as_bool() == Some(flag)
                                        }) && filter.is_raid.is_none_or(|flag| {
                                            row["is_raid"].as_bool() == Some(flag)
                                        }) && filter.maximum_reset_seconds.is_none_or(|maximum| {
                                            row["reset_seconds"].as_u64().is_some_and(|seconds| {
                                                seconds <= u64::from(maximum)
                                            })
                                        }) && saved_resets::matches_projected_window(
                                            row["reset_seconds"].as_i64(),
                                            s.captured_at,
                                            filter.projected_reset_from,
                                            filter.projected_reset_by,
                                        )
                                    })
                                    .collect();
                            saved_resets::discovery_report(
                                matches,
                                saved,
                                s.captured_at,
                                filter.projected_reset_from.is_some()
                                    || filter.projected_reset_by.is_some(),
                            )
                        });
                    if saved_instance.is_some() && saved_instance_match.is_none() {
                        continue;
                    }
                    let mount_match = collected_mount
                        .as_ref()
                        .and_then(|filter| filter.match_view(s.sections.get("mount_collection")?));
                    if collected_mount.is_some() && mount_match.is_none() {
                        continue;
                    }
                    let search_mount_description =
                        collected_mount_name.is_some() || collected_mount_spell_id.is_some();
                    let mount_description_match = search_mount_description
                        .then(|| {
                            let view = s.sections.get("mount_collection")?;
                            let mut matches: Vec<_> = view["entries"]
                                .as_object()?
                                .iter()
                                .filter_map(|(id, row)| {
                                    let id = id.parse::<u32>().ok()?;
                                    (collected_mount
                                        .as_ref()
                                        .is_none_or(|filter| filter.mount_id.get() == id)
                                        && collected_mount_name.as_ref().is_none_or(|name| {
                                            row["name"].as_str().is_some_and(|label| {
                                                label.trim().to_lowercase() == *name
                                            })
                                        })
                                        && collected_mount_spell_id.is_none_or(|id| {
                                            row["spell_id"].as_u64() == Some(u64::from(id.get()))
                                        }))
                                    .then_some((id, row))
                                })
                                .collect();
                            matches.sort_unstable_by_key(|(id, _)| *id);
                            (!matches.is_empty()).then(|| {
                                json!({
                                    "matched_count":matches.len(),"truncated":matches.len()>20,
                                    "mounts":matches.into_iter().take(20).map(|(id,row)| json!({
                                        "mount_id":id,"name":row["name"],"spell_id":row["spell_id"],
                                        "usable":row["usable"],"favorite":row["favorite"],
                                    })).collect::<Vec<_>>(),
                                })
                            })
                        })
                        .flatten();
                    if search_mount_description && mount_description_match.is_none() {
                        continue;
                    }
                    let toy_match = toy_in_view.as_ref().and_then(|filter| {
                        filter.match_view(s.sections.get("toy_collection_view")?)
                    });
                    if toy_in_view.is_some() && toy_match.is_none() {
                        continue;
                    }
                    let toy_name_match = toy_name.as_ref().and_then(|name| {
                        let view = s.sections.get("toy_collection_view")?;
                        let mut matches: Vec<_> = view["entries"].as_object()?.iter().filter_map(|(id, row)| {
                            let id = id.parse::<u32>().ok()?;
                            (toy_in_view.as_ref().is_none_or(|filter| filter.item_id.get() == id)
                                && row["name"].as_str().is_some_and(|label| label.trim().to_lowercase() == *name))
                                .then_some((id, row))
                        }).collect();
                        matches.sort_unstable_by_key(|(id, _)| *id);
                        (!matches.is_empty()).then(|| json!({
                            "coverage":view["coverage"],"matched_count":matches.len(),"truncated":matches.len()>20,
                            "toys":matches.into_iter().take(20).map(|(id,row)| json!({
                                "item_id":id,"name":row["name"],"collected":row["collected"],"favorite":row["favorite"],
                            })).collect::<Vec<_>>(),
                        }))
                    });
                    if toy_name.is_some() && toy_name_match.is_none() {
                        continue;
                    }
                    let achievement_match = tracked_achievement.as_ref().and_then(|filter| {
                        let entries =
                            s.sections.get("tracked_achievements")?["entries"].as_object()?;
                        let (id, row) = select_named_entry(
                            entries,
                            achievement_key.as_deref(),
                            achievement_name.as_deref(),
                            |_| true,
                        )?;
                        let unfinished = if let Some(maximum) = filter.maximum_unfinished_criteria {
                            let count = analysis::unfinished_achievement_criteria_count(row)?;
                            if count > maximum as usize {
                                return None;
                            }
                            Some(count)
                        } else {
                            None
                        };
                        (filter
                            .completed
                            .is_none_or(|value| row["completed"].as_bool() == Some(value))
                            && filter.earned_by_character.is_none_or(|value| {
                                row["earned_by_character"].as_bool() == Some(value)
                            }))
                        .then_some((id, row, unfinished))
                    });
                    if tracked_achievement.is_some() && achievement_match.is_none() {
                        continue;
                    }
                    let paragon_match = paragon.as_ref().and_then(|filter| {
                        let entries = s.sections.get("reputation")?["entries"].as_object()?;
                        let (id, row) = select_named_entry(
                            entries,
                            paragon_key.as_deref(),
                            paragon_name.as_deref(),
                            |row| row["kind"] == "paragon",
                        )?;
                        let detail = row["paragon"].as_object()?;
                        let readiness = detail
                            .get("reward_quest_id")
                            .and_then(Value::as_u64)
                            .and_then(|id| s.sections.get("quests")?.get(id.to_string()))
                            .map(analysis::quest_readiness);
                        (filter
                            .reward_pending
                            .is_none_or(|flag| detail["reward_pending"].as_bool() == Some(flag))
                            && filter
                                .level_too_low
                                .is_none_or(|flag| detail["level_too_low"].as_bool() == Some(flag))
                            && filter
                                .reward_quest_readiness
                                .is_none_or(|expected| readiness == Some(expected)))
                        .then_some((id, row, readiness))
                    });
                    if paragon.is_some() && paragon_match.is_none() {
                        continue;
                    }
                    let renown_match = renown.as_ref().and_then(|filter| {
                        let entries = s.sections.get("reputation")?["entries"].as_object()?;
                        let (id, row) = select_named_entry(
                            entries,
                            renown_key.as_deref(),
                            renown_name.as_deref(),
                            |row| row["kind"] == "renown",
                        )?;
                        (row["renown"]["level"].as_u64()? >= u64::from(filter.minimum_level)
                            && filter.capped.is_none_or(|capped| {
                                row["renown"]["capped"].as_bool() == Some(capped)
                            }))
                        .then_some((id, row))
                    });
                    if renown.is_some() && renown_match.is_none() {
                        continue;
                    }
                    let reputation_match = reputation_rank.as_ref().and_then(|filter| {
                        let entries = s.sections.get("reputation")?["entries"].as_object()?;
                        let (id, row) = select_named_entry(
                            entries,
                            reputation_key.as_deref(),
                            reputation_name.as_deref(),
                            |row| row["kind"] == "standard",
                        )?;
                        if row["reaction"].as_u64()? < u64::from(filter.minimum_reaction) {
                            return None;
                        }
                        Some((id, row))
                    });
                    if reputation_rank.is_some() && reputation_match.is_none() {
                        continue;
                    }
                    let currency_match = currency_balance.as_ref().and_then(|filter| {
                        let entries =
                            s.sections.get("currency_balances")?["entries"].as_object()?;
                        let (id, row) = select_named_entry(
                            entries,
                            currency_key.as_deref(),
                            currency_name.as_deref(),
                            |_| true,
                        )?;
                        let headroom =
                            analysis::currency_entry_cap_headroom(row).unwrap_or(Value::Null);
                        (row["quantity"].as_u64()? >= filter.minimum_quantity
                            && filter.minimum_weekly_remaining.is_none_or(|minimum| {
                                headroom["weekly_remaining"]
                                    .as_u64()
                                    .is_some_and(|room| room >= minimum)
                            })
                            && filter.minimum_maximum_remaining.is_none_or(|minimum| {
                                headroom["maximum_remaining"]
                                    .as_u64()
                                    .is_some_and(|room| room >= minimum)
                            }))
                        .then_some((id, row))
                    });
                    if currency_balance.is_some() && currency_match.is_none() {
                        continue;
                    }
                    let waypoint_map = if quest_on_captured_map {
                        let Some(map) = s
                            .sections
                            .get("stats")
                            .and_then(|stats| stats["map_id"].as_u64())
                        else {
                            continue;
                        };
                        Some(map)
                    } else {
                        quest_waypoint_map_id.map(|id| u64::from(id.get()))
                    };
                    let has_reward = |quest: &Value, field: &str, key: &str, id: u32| {
                        quest[field].as_object().is_some_and(|rows| {
                            rows.values()
                                .any(|row| row[key].as_u64() == Some(u64::from(id)))
                        })
                    };
                    let quest_wallet = s
                        .sections
                        .get("wallet")
                        .and_then(|wallet| wallet["copper"].as_u64());
                    let matches_quest = |quest: &Value| {
                        quest_title.as_ref().is_none_or(|title| {
                            quest["title"]
                                .as_str()
                                .is_some_and(|observed| observed.trim().to_lowercase() == *title)
                        }) && quest_readiness
                            .is_none_or(|expected| analysis::quest_readiness(quest) == expected)
                            && waypoint_map.is_none_or(|map| {
                                quest["next_waypoint_map_id"].as_u64() == Some(map)
                            })
                            && quest_max_remaining_seconds.is_none_or(|maximum| {
                                quest["timer"]["remaining_seconds"]
                                    .as_u64()
                                    .is_some_and(|seconds| seconds <= u64::from(maximum))
                            })
                            && quest_reward_item_id.is_none_or(|id| {
                                has_reward(quest, "item_rewards", "item_id", id.get())
                                    || has_reward(quest, "item_choices", "item_id", id.get())
                            })
                            && quest_reward_currency_id.is_none_or(|id| {
                                has_reward(quest, "currency_rewards", "currency_id", id.get())
                                    || has_reward(
                                        quest,
                                        "currency_choices",
                                        "currency_id",
                                        id.get(),
                                    )
                            })
                            && quest_reward_renown_faction_id.is_none_or(|id| {
                                has_reward(quest, "renown_rewards", "faction_id", id.get())
                            })
                            && quest_difficulty
                                .is_none_or(|difficulty| difficulty.matches(quest) == Some(true))
                            && quest_max_suggested_group_size.is_none_or(|maximum| {
                                quest["suggested_group_size"]
                                    .as_u64()
                                    .is_some_and(|size| size <= u64::from(maximum.get()))
                            })
                            && quest_min_reward_xp.is_none_or(|minimum| {
                                quest["reward_amounts"]["xp"]
                                    .as_u64()
                                    .is_some_and(|xp| xp >= minimum)
                            })
                            && quest_min_reward_honor.is_none_or(|minimum| {
                                quest["reward_amounts"]["honor"]
                                    .as_u64()
                                    .is_some_and(|honor| honor >= minimum)
                            })
                            && quest_max_required_copper.is_none_or(|maximum| {
                                quest["required_copper"]
                                    .as_u64()
                                    .is_some_and(|cost| cost <= maximum)
                            })
                            && (!quest_within_captured_wallet
                                || quest["required_copper"]
                                    .as_u64()
                                    .zip(quest_wallet)
                                    .is_some_and(|(cost, balance)| cost <= balance))
                            && (!filter_quest_objectives
                                || analysis::quest_objective_matches(
                                    quest,
                                    quest_objective_kind,
                                    quest_objective_finished,
                                    objective_min,
                                    objective_max,
                                ) == Some(true))
                            && saved_resets::matches_projected_window(
                                quest["timer"]["remaining_seconds"].as_i64(),
                                s.captured_at,
                                None,
                                quest_projected_expiry_by,
                            )
                    };
                    let quest_match = quest_key.as_ref().and_then(|key| {
                        let quest = s.sections.get("quests")?.get(key)?;
                        matches_quest(quest).then(|| analysis::quest_readiness(quest))
                    });
                    if quest_key.is_some() && quest_match.is_none() {
                        continue;
                    }
                    let quest_count = (quest_key.is_none()
                        && (quest_title.is_some()
                            || quest_reward_item_id.is_some()
                            || quest_reward_currency_id.is_some()
                            || quest_reward_renown_faction_id.is_some()
                            || quest_readiness.is_some()
                            || waypoint_map.is_some()
                            || filter_quest_timer
                            || filter_quest_objectives
                            || filter_quest_group
                            || filter_quest_budget))
                        .then(|| {
                            s.sections.get("quests").and_then(Value::as_object).map_or(
                                0,
                                |quests| {
                                    quests.values().filter(|quest| matches_quest(quest)).count()
                                },
                            )
                        });
                    if quest_count == Some(0) {
                        continue;
                    }
                    let experience_match = if filter_experience {
                        let Some(xp) = s.sections.get("experience").filter(|xp| {
                            xp["status"] == "progress"
                                && (!xp_gain_enabled_only || xp["xp_disabled"] == false)
                                && minimum_rested_xp.is_none_or(|minimum| {
                                    xp["rested_xp"]
                                        .as_u64()
                                        .is_some_and(|rested| rested >= u64::from(minimum))
                                })
                        }) else {
                            continue;
                        };
                        Some(json!({"rested_xp":xp["rested_xp"],"xp_disabled":xp["xp_disabled"]}))
                    } else {
                        None
                    };
                    if food_capture.as_ref().is_some_and(|food| {
                        analysis::ensure_same_character_and_client(food, &s).is_err()
                    }) {
                        continue;
                    }
                    let pet_food_match = if let Some(filter) = &pet_food {
                        let Some(food) = s
                            .sections
                            .get("pet_care")
                            .and_then(|pet| pet["carried_food"].as_object())
                        else {
                            continue;
                        };
                        let quantity = match &pet_food_key {
                            Some(key) => food.get(key).and_then(Value::as_u64).unwrap_or(0),
                            None => food.values().filter_map(Value::as_u64).sum(),
                        };
                        if quantity < u64::from(filter.minimum_quantity) {
                            continue;
                        }
                        Some(json!({"item_id":filter.item_id,"accepted_quantity":quantity}))
                    } else {
                        None
                    };
                    let bag_space = if filter_bag_space {
                        let Some(space) = analysis::carried_bag_space(&s).filter(|space| {
                            minimum_free_bag_slots.is_none_or(|minimum| {
                                space["free_slots"]
                                    .as_u64()
                                    .is_some_and(|free| free >= u64::from(minimum))
                            }) && minimum_general_free_bag_slots.is_none_or(|minimum| {
                                space["known_general_free_slots"]
                                    .as_u64()
                                    .is_some_and(|free| free >= u64::from(minimum))
                            }) && bag_family_space.as_ref().is_none_or(|(mask, minimum)| {
                                space["free_slots_by_family_mask"][mask]
                                    .as_u64()
                                    .is_some_and(|free| free >= *minimum)
                            })
                        }) else {
                            continue;
                        };
                        Some(space)
                    } else {
                        None
                    };
                    let durability_matches = if let Some(maximum) = equipment_durability_max_percent
                    {
                        let Some(slots) = s
                            .sections
                            .get("equipment_durability")
                            .and_then(|section| {
                                analysis::equipment_durability_matching_slots(section, maximum)
                            })
                            .filter(|slots| !slots.is_empty())
                        else {
                            continue;
                        };
                        Some(slots)
                    } else {
                        None
                    };
                    let repair_quote = if filter_repair {
                        let quote = s
                            .sections
                            .get("repair_quote")
                            .and_then(|row| row["copper"].as_u64());
                        let Some(quote) = quote.filter(|quote| {
                            repair_max_quote_copper.is_none_or(|maximum| *quote <= maximum)
                        }) else {
                            continue;
                        };
                        let remaining = s
                            .sections
                            .get("wallet")
                            .and_then(|wallet| wallet["copper"].as_u64())
                            .and_then(|balance| balance.checked_sub(quote));
                        if (repair_affordable_only || repair_min_remaining_copper.is_some())
                            && !remaining.is_some_and(|balance| {
                                balance >= repair_min_remaining_copper.unwrap_or(0)
                            })
                        {
                            continue;
                        }
                        Some((quote, remaining))
                    } else {
                        None
                    };
                    let player_map = s.sections.get("stats").filter(|stats| {
                        player_map_id
                            .is_none_or(|id| stats["map_id"].as_u64() == Some(u64::from(id.get())))
                            && player_parent_map_id.is_none_or(|id| {
                                stats["map_parent_id"].as_u64() == Some(u64::from(id.get()))
                            })
                            && player_map_name.as_ref().is_none_or(|name| {
                                stats["map_name"]
                                    .as_str()
                                    .is_some_and(|observed| observed.to_lowercase() == *name)
                            })
                    });
                    if filter_player_map && player_map.is_none() {
                        continue;
                    }
                    let tracked_match = tracked_recipe_key.as_ref().and_then(|key| {
                        let tracking = s.sections.get("tracked_recipes")?;
                        let craft = tracking["crafts"].get(key);
                        let recraft = tracking["recrafts"].get(key);
                        let matched = (craft_enabled && craft.is_some_and(matches_learning))
                            || (recraft_enabled && recraft.is_some_and(matches_learning));
                        matched.then_some((craft, recraft))
                    });
                    let tracked_counts = (filter_tracking && tracked_recipe_key.is_none())
                        .then(|| {
                            let tracking = s.sections.get("tracked_recipes")?;
                            let count = |group: &str, enabled: bool| {
                                tracking[group].as_object().map(|rows| {
                                    rows.values()
                                        .filter(|row| enabled && matches_learning(row))
                                        .count()
                                })
                            };
                            let crafts = count("crafts", craft_enabled)?;
                            let recrafts = count("recrafts", recraft_enabled)?;
                            (crafts > 0 || recrafts > 0).then_some((crafts, recrafts))
                        })
                        .flatten();
                    if filter_tracking && tracked_match.is_none() && tracked_counts.is_none() {
                        continue;
                    }
                    let profession_match = profession_skill_line.and_then(|skill_line| {
                        let (_, row) = analysis::profession_by_skill_line(
                            s.sections.get("professions"),
                            skill_line.get(),
                        );
                        row.filter(|row| {
                            let rank = row["rank"].as_u64();
                            profession_min_rank.is_none_or(|minimum| {
                                rank.is_some_and(|rank| rank >= u64::from(minimum))
                            }) && profession_max_rank.is_none_or(|maximum| {
                                rank.is_some_and(|rank| rank <= u64::from(maximum))
                            })
                        })
                    });
                    if profession_skill_line.is_some() && profession_match.is_none() {
                        continue;
                    }
                    let profession_progress =
                        profession_match
                            .zip(profession_target_rank)
                            .map(|(profession, target)| {
                                analysis::profession_target_progress(Some(profession), target.get())
                            });
                    if !profession_target_max_remaining.is_none_or(|maximum| {
                        profession_progress
                            .as_ref()
                            .and_then(|row| row["points_to_target"].as_u64())
                            .is_some_and(|points| points <= u64::from(maximum))
                    }) || (profession_target_within_cap_only
                        && !profession_progress.as_ref().is_some_and(|row| {
                            row["target_above_observed_cap"].as_bool() == Some(false)
                        }))
                    {
                        continue;
                    }
                    let merchant_matches = filter_merchant.then(|| {
                        let wallet = s
                            .sections
                            .get("wallet")
                            .and_then(|wallet| wallet["copper"].as_u64());
                        s.sections
                            .get("merchant_stock")
                            .and_then(Value::as_object)
                            .map_or((0, None, None), |offers| {
                                let matching = offers
                                    .iter()
                                    .filter(|(_, offer)| {
                                        merchant_product.is_none_or(|(field, id)| {
                                            offer[field].as_u64() == Some(u64::from(id))
                                        }) && accepted_food.is_none_or(|food| {
                                            offer["item_id"].as_u64().is_some_and(|id| {
                                                food.contains_key(&id.to_string())
                                            })
                                        }) && (!(merchant_in_stock_only
                                            || merchant_affordable_only)
                                            || matches!(
                                                offer["available"].as_i64(),
                                                Some(-1 | 1..)
                                            ))
                                            && (!(merchant_copper_only || merchant_affordable_only)
                                                || offer["extended_cost"] == false)
                                            && merchant_stock_state.is_none_or(|state|
                                                state.matches(offer["available"].as_i64()))
                                            && (!merchant_affordable_only
                                                || merchant_target_quantity.is_some()
                                                || wallet
                                                    .zip(offer["price_copper"].as_u64())
                                                    .is_some_and(|(balance, price)| {
                                                        price <= balance
                                                    }))
                                            && merchant_max_price_copper.is_none_or(|maximum| {
                                                offer["price_copper"]
                                                    .as_u64()
                                                    .is_some_and(|price| price <= maximum)
                                            })
                                    });
                                let Some(quantity) = merchant_target_quantity else {
                                    return (matching.count(), None, None);
                                };
                                if merchant_combine_bundles {
                                    let eligible = matching.filter(|(_, offer)| {
                                        offer["available"] == -1 && offer["extended_cost"] == false
                                    });
                                    let count = eligible.clone().count();
                                    let mix = analysis::cheapest_merchant_bundle_mix(eligible, quantity.get())
                                        .filter(|plan| plan["required_copper"].as_u64().is_some_and(|cost|
                                            merchant_max_total_copper.is_none_or(|maximum| cost <= maximum)
                                            && (!merchant_affordable_only || wallet.is_some_and(|balance| cost <= balance))));
                                    return (if mix.is_some() { count } else { 0 }, None, mix);
                                }
                                let mut count = 0;
                                let mut best = None;
                                for (slot, offer) in matching {
                                    if offer["available"] != -1 || offer["extended_cost"] != false {
                                        continue;
                                    }
                                    let Some(plan) = analysis::merchant_quantity_row(offer, u64::from(quantity.get())) else {
                                        continue;
                                    };
                                    let Some(cost) = plan["required_copper"].as_u64() else {
                                        continue;
                                    };
                                    if merchant_max_total_copper.is_some_and(|maximum| cost > maximum) {
                                        continue;
                                    }
                                    if merchant_affordable_only && !wallet.is_some_and(|balance| cost <= balance) {
                                        continue;
                                    }
                                    let Ok(slot) = slot.parse::<u32>() else { continue };
                                    count += 1;
                                    let key = (cost, slot);
                                    if best.as_ref().is_none_or(|(previous, _)| key < *previous) {
                                        best = Some((key, json!({"slot":slot,"target_quantity":quantity,
                                            "item_id":offer["item_id"],"currency_id":offer["currency_id"],
                                            "plan":plan})));
                                    }
                                }
                                (count, best.map(|(_, plan)| plan), None)
                            })
                    });
                    if matches!(merchant_matches, Some((0, _, _))) {
                        continue;
                    }
                    let trainer_match = filter_trainer.then(|| {
                        let trainer = s.sections.get("trainer_services")?;
                        let (balance_field, balance) = if trainer["kind"] == "pet" {
                            ("price_training_points", trainer["pet_training_points_available"].as_u64())
                        } else {
                            ("price_copper", s.sections.get("wallet").and_then(|wallet| wallet["copper"].as_u64()))
                        };
                        let spendable = balance.and_then(|balance| {
                            balance.checked_sub(trainer_balance_reserve.unwrap_or_default())
                        });
                        let count = trainer["entries"].as_object()?.values()
                            .filter(|row| {
                                trainer_service_status.is_none_or(|status| row["status"] == status)
                                    && (!trainer_affordable_only || row[balance_field].as_u64()
                                        .zip(spendable).is_some_and(|(price, spendable)| price <= spendable))
                                    && trainer_price.is_none_or(|(field, maximum)| {
                                        row[field].as_u64().is_some_and(|price| price <= maximum)
                                    })
                                    && trainer_service_name.as_ref().is_none_or(|name| {
                                        row["name"].as_str().is_some_and(|lesson| lesson.to_lowercase().contains(name))
                                    })
                            })
                            .count();
                        (count > 0).then(|| {
                            let mut result = json!({"lesson_count":count,"kind":trainer["kind"],
                                "captured_player_map_id":s.sections.get("stats").and_then(|stats| stats["map_id"].as_u64())});
                            if let Some(reserve) = trainer_balance_reserve {
                                result["balance_after_reserve"] = json!({
                                    "unit":if balance_field == "price_copper" {"copper"} else {"training_points"},
                                    "reserve":reserve,"spendable":spendable,
                                });
                            }
                            result
                        })
                    }).flatten();
                    if filter_trainer && trainer_match.is_none() {
                        continue;
                    }
                    let flight_match = (|| {
                        if !filter_flight_destination {
                            return None;
                        }
                        let player_map = s
                            .sections
                            .get("stats")
                            .and_then(|stats| stats["map_id"].as_u64());
                        if flight_departure_map_id
                            .is_some_and(|map| player_map != Some(u64::from(map.get())))
                        {
                            return None;
                        }
                        let map = s.sections.get("flight_map")?;
                        let node = match flight_destination_node_id {
                            Some(id) => id.get(),
                            None => analysis::named_flight_destination(
                                map,
                                flight_destination_name.as_deref()?,
                            )?,
                        };
                        let mut route = analysis::captured_flight_route(map, node)?;
                        if flight_destination_name.as_ref().is_some_and(|name| {
                            !route["destination_name"]
                                .as_str()
                                .is_some_and(|label| label.trim().to_lowercase() == *name)
                        }) {
                            return None;
                        }
                        let fare = route["fare_copper"].as_u64();
                        if flight_origin_node_id.is_some_and(|origin| {
                            route["origin_node_id"].as_u64() != Some(u64::from(origin.get()))
                        }) || flight_max_fare_copper
                            .is_some_and(|maximum| fare.is_none_or(|fare| fare > maximum))
                        {
                            return None;
                        }
                        if flight_affordable_only || flight_wallet_reserve_copper.is_some() {
                            let remaining = s.sections.get("wallet")?["copper"]
                                .as_u64()?
                                .checked_sub(fare?)?;
                            if let Some(reserve) = flight_wallet_reserve_copper {
                                if remaining < reserve {
                                    return None;
                                }
                                route["remaining_wallet_copper"] = json!(remaining);
                                route["required_wallet_reserve_copper"] = json!(reserve);
                            }
                        }
                        if flight_departure_map_id.is_some() {
                            route["captured_departure_map_id"] = json!(player_map);
                        }
                        Some(route)
                    })();
                    if filter_flight_destination && flight_match.is_none() {
                        continue;
                    }
                    // Retain metadata only: listing must not hold every inventory payload.
                    let mut summary = json!({"snapshot_id":id,"character":s.character,"client":s.client,"captured_at":s.captured_at,"source":s.source});
                    if let Some(matches) = missing_section_matches {
                        summary["missing_section_matches"] = json!(matches);
                    }
                    if let Some((id, row, unfinished)) = achievement_match {
                        summary["tracked_achievement_match"] = json!({
                            "achievement_id":id,"completed":row["completed"],
                            "earned_by_character":row["earned_by_character"],"account_wide":row["account_wide"],
                        });
                        if let Some(count) = unfinished {
                            summary["tracked_achievement_match"]["unfinished_criteria"] =
                                json!(count);
                        }
                    }
                    if let Some((id, row)) = renown_match {
                        summary["renown_match"] = json!({
                            "faction_id":id,"level":row["renown"]["level"],
                            "capped":row["renown"]["capped"],"account_wide":row["account_wide"],
                        });
                    }
                    if let Some((filter, (id, row, readiness))) =
                        paragon.as_ref().zip(paragon_match)
                    {
                        summary["paragon_match"] = json!({
                            "faction_id":id,"reward_pending":row["paragon"]["reward_pending"],
                            "level_too_low":row["paragon"]["level_too_low"],
                            "reward_quest_id":row["paragon"]["reward_quest_id"],
                            "account_wide":row["account_wide"],
                        });
                        if filter.reward_quest_readiness.is_some() {
                            summary["paragon_match"]["reward_quest_readiness"] = json!(readiness);
                        }
                    }
                    if let Some((id, row)) = reputation_match {
                        summary["reputation_rank_match"] = json!({
                            "faction_id":id,"reaction":row["reaction"],
                            "account_wide":row["account_wide"],
                        });
                    }
                    if let Some((filter, (id, row))) = currency_balance.as_ref().zip(currency_match)
                    {
                        summary["currency_balance_match"] = json!({
                            "currency_id":id,"quantity":row["quantity"],
                            "account_wide":row["account_wide"],"inactive":row["inactive"],
                        });
                        if filter.minimum_weekly_remaining.is_some()
                            || filter.minimum_maximum_remaining.is_some()
                        {
                            summary["currency_balance_match"]["cap_headroom"] =
                                analysis::currency_entry_cap_headroom(row).unwrap_or(Value::Null);
                        }
                    }
                    if let Some(food) = pet_food_match {
                        summary["pet_food_match"] = food;
                    }
                    if let Some(experience) = experience_match {
                        summary["experience_match"] = experience;
                    }
                    if let Some((id, readiness)) = quest_id.zip(quest_match) {
                        summary["quest_match"] = json!({"quest_id":id,"readiness":readiness});
                    }
                    if let Some(count) = quest_count {
                        summary["quest_match"] =
                            json!({"readiness":quest_readiness,"quest_count":count});
                    }
                    if let Some(map) = waypoint_map {
                        summary["quest_match"]["waypoint_map_id"] = json!(map);
                    }
                    if quest_on_captured_map
                        || quest_title.is_some()
                        || filter_quest_objectives
                        || quest_reward_item_id.is_some()
                        || quest_reward_currency_id.is_some()
                        || quest_reward_renown_faction_id.is_some()
                        || filter_quest_group
                        || filter_quest_budget
                    {
                        let mut local: Vec<_> = s
                            .sections
                            .get("quests")
                            .and_then(Value::as_object)
                            .into_iter()
                            .flatten()
                            .filter(|(key, quest)| {
                                quest_key.as_ref().is_none_or(|wanted| wanted == *key)
                                    && matches_quest(quest)
                            })
                            .filter_map(|(key, quest)| Some((key.parse::<u32>().ok()?, quest)))
                            .collect();
                        local.sort_unstable_by_key(|(id, quest)| {
                            let timer = local_quest_soonest_first
                                .then(|| quest["timer"]["remaining_seconds"].as_u64())
                                .flatten();
                            let xp = local_quest_highest_xp_first
                                .then(|| quest["reward_amounts"]["xp"].as_u64())
                                .flatten();
                            (timer.is_none(), timer, std::cmp::Reverse(xp), *id)
                        });
                        summary["quest_match"]["matching_quests_truncated"] =
                            json!(local.len() > 20);
                        if let Some(offset) = local_quest_offset {
                            summary["quest_match"]["matching_quests_offset"] = json!(offset);
                            summary["quest_match"]["matching_quests_has_more"] =
                                json!(local.len().saturating_sub(offset) > 20);
                        }
                        summary["quest_match"]["matching_quests"] = json!(
                            local
                                .into_iter()
                                .skip(local_quest_offset.unwrap_or_default()).take(20)
                                .map(|(id, quest)| {
                                let mut row = json!({"quest_id":id,"title":quest["title"].as_str().filter(|title| title.len() <= 256),
                                    "readiness":analysis::quest_readiness(quest)});
                                if let Some(item) = quest_reward_item_id {
                                    row["recorded_reward_match"] = json!({"item_id":item,
                                        "fixed":has_reward(quest, "item_rewards", "item_id", item.get()),
                                        "choice":has_reward(quest, "item_choices", "item_id", item.get()),
                                        "reward_eligibility_verified":false});
                                }
                                if let Some(currency) = quest_reward_currency_id {
                                    row["recorded_currency_reward_match"] = json!({"currency_id":currency,
                                        "fixed":has_reward(quest, "currency_rewards", "currency_id", currency.get()),
                                        "choice":has_reward(quest, "currency_choices", "currency_id", currency.get()),
                                        "reward_eligibility_verified":false});
                                }
                                if let Some(faction) = quest_reward_renown_faction_id {
                                    row["recorded_renown_reward_match"] = json!({
                                        "faction_id":faction,"reward_eligibility_verified":false,
                                    });
                                }
                                if filter_quest_group {
                                    row["elite"] = json!(quest["elite"].as_bool());
                                    row["suggested_group_size"] = json!(quest["suggested_group_size"].as_u64());
                                }
                                if filter_quest_budget || local_quest_highest_xp_first {
                                    row["reward_xp"] = json!(quest["reward_amounts"]["xp"].as_u64());
                                }
                                if quest_min_reward_honor.is_some() {
                                    row["reward_honor"] = json!(quest["reward_amounts"]["honor"].as_u64());
                                }
                                if filter_quest_budget {
                                    row["required_copper"] = json!(quest["required_copper"].as_u64());
                                }
                                if local_quest_soonest_first {
                                    row["remaining_seconds"] = json!(quest["timer"]["remaining_seconds"].as_u64());
                                }
                                row
                            })
                                .collect::<Vec<_>>()
                        );
                        if quest_on_captured_map {
                            summary["quest_match"]["on_captured_player_map"] = json!(true);
                        }
                    }
                    if filter_quest_timer {
                        let shortest = s.sections.get("quests")
                            .and_then(Value::as_object)
                            .into_iter()
                            .flatten()
                            .filter(|(key, quest)| {
                                quest_key.as_ref().is_none_or(|wanted| wanted == *key)
                                    && matches_quest(quest)
                            })
                            .filter_map(|(key, quest)| {
                                Some((quest["timer"]["remaining_seconds"].as_i64()?,
                                    key.parse::<u32>().ok()?))
                            })
                            .min()
                            .map(|(seconds, id)| {
                                json!({"quest_id":id,"remaining_seconds":seconds,
                                    "projected_expiry_at":saved_resets::projected_reset_at(seconds, s.captured_at)})
                            });
                        summary["quest_match"]["timer_filter"] = json!({
                            "maximum_remaining_seconds":quest_max_remaining_seconds,
                            "projected_expiry_by":quest_projected_expiry_by,
                            "shortest_matching_timer":shortest,
                            "current_expiry_verified":false,
                        });
                    }
                    if let Some(observation) = saved_instance_match {
                        summary["saved_instance_match"] = observation;
                    }
                    if let Some(observation) = saved_world_boss_match {
                        summary["saved_world_boss_match"] = observation;
                    }
                    if let Some(observation) = instance_lock_warning_match {
                        summary["instance_lock_warning_match"] = observation;
                    }
                    if let Some(observation) = profession_recipe_match {
                        summary["profession_recipe_match"] = observation;
                    }
                    if let Some(observation) = mail_attachment_match {
                        summary["mail_attachment_match"] = observation;
                    }
                    if let Some(observation) = pet_name_match {
                        summary["pet_species_name_match"] = observation;
                    }
                    if let Some(observation) = pet_species_match {
                        summary["pet_species_match"] = observation;
                    }
                    if let Some(observation) = inventory_item_match {
                        summary["inventory_item_match"] = observation;
                    }
                    if let Some(observation) = inventory_requirements_match {
                        summary["inventory_requirements_match"] = observation;
                    }
                    if let Some(observation) = loot_observation_match {
                        summary["loot_observation_match"] = observation;
                    }
                    if let Some(observation) = skill_line_match {
                        summary["skill_line_match"] = observation;
                    }
                    if let Some(observation) = gossip_option_match {
                        summary["gossip_option_match"] = observation;
                    }
                    if let Some(observation) = outgoing_trade_item_match {
                        summary["outgoing_trade_item_match"] = observation;
                    }
                    if let Some(observation) = pvp_rank_match {
                        summary["pvp_rank_match"] = observation;
                    }
                    if let Some(observation) = player_resource_match {
                        summary["player_resource_match"] = observation;
                    }
                    if let Some(observation) = player_aura_match {
                        summary["player_aura_match"] = observation;
                    }
                    if let Some(observation) = weapon_enchant_match {
                        summary["weapon_enchant_match"] = observation;
                    }
                    if let Some(observation) = buyback_match {
                        summary["merchant_buyback_match"] = observation;
                    }
                    if let Some(observation) = pet_action_match {
                        summary["pet_action_spell_match"] = observation;
                    }
                    if let Some(observation) = pet_command_match {
                        summary["pet_action_command_match"] = observation;
                    }
                    if let Some(observation) = group_context_match {
                        summary["group_context_match"] = observation;
                    }
                    if let Some(observation) = profession_spell_match {
                        summary["profession_spell_match"] = observation;
                    }
                    if let Some(observation) = active_pet_match {
                        summary["active_pet_match"] = observation;
                    }
                    if let Some(observation) = spellbook_match {
                        summary["spellbook_spell_match"] = observation;
                    }
                    if let Some(observation) = action_bar_spell_resource_match {
                        summary["action_bar_spell_resource_match"] = observation;
                    }
                    if let Some(observation) = mount_match {
                        summary["collected_mount_match"] = observation;
                    }
                    if let Some(observation) = mount_description_match {
                        if collected_mount_name.is_some() {
                            summary["collected_mount_name_match"] = observation.clone();
                        }
                        if collected_mount_spell_id.is_some() {
                            summary["collected_mount_spell_match"] = observation;
                        }
                    }
                    if let Some(observation) = toy_match {
                        summary["toy_in_view_match"] = observation;
                    }
                    if let Some(observation) = toy_name_match {
                        summary["toy_name_match"] = observation;
                    }
                    if let Some(space) = bag_space {
                        summary["carried_bag_space"] = space;
                    }
                    if let Some((quote, remaining)) = repair_quote {
                        summary["repair_quote_match"] = json!({"copper":quote});
                        if repair_min_remaining_copper.is_some() {
                            summary["repair_quote_match"]["wallet_remaining_copper"] =
                                json!(remaining);
                        }
                    }
                    if let Some(slots) = durability_matches {
                        summary["equipment_durability_matching_slots"] = json!(slots);
                    }
                    if filter_player_map && let Some(stats) = player_map {
                        summary["player_map_match"] = json!({
                            "map_id":stats["map_id"],"map_name":stats["map_name"],
                            "map_parent_id":stats["map_parent_id"],
                        });
                    }
                    if let Some((recipe_id, (craft, recraft))) =
                        tracked_recipe_id.zip(tracked_match)
                    {
                        summary["tracked_recipe_match"] = json!({
                            "recipe_id":recipe_id,
                            "tracked_as_craft":craft.is_some(),"tracked_as_recraft":recraft.is_some(),
                            "craft_learned":craft.and_then(|row| row["learned"].as_bool()),
                            "recraft_learned":recraft.and_then(|row| row["learned"].as_bool()),
                        });
                    }
                    if let Some((crafts, recrafts)) = tracked_counts {
                        summary["tracked_recipe_counts"] =
                            json!({"crafts":crafts,"recrafts":recrafts});
                    }
                    if let Some(profession) = profession_match {
                        summary["profession_match"] = json!({
                            "skill_line":profession["skill_line"],"rank":profession["rank"],
                            "max_rank":profession["max_rank"],
                        });
                        if let Some(progress) = profession_progress {
                            summary["profession_target_progress"] = progress;
                        }
                    }
                    if let Some(trainer) = trainer_match {
                        summary["trainer_match"] = trainer;
                    }
                    if let Some(route) = flight_match {
                        summary["flight_match"] = route;
                    }
                    if let Some((count, quantity_plan, bundle_mix)) = merchant_matches {
                        let stats = s.sections.get("stats");
                        summary["merchant_match"] = json!({
                            "offer_count":count,
                            "captured_player_map_id":stats.and_then(|stats| stats["map_id"].as_u64()),
                            "captured_player_map_name":stats.and_then(|stats| stats["map_name"].as_str()),
                        });
                        if let Some((field, product_id)) = merchant_product {
                            summary["merchant_match"][field] = json!(product_id);
                        }
                        if let Some(food_id) = merchant_pet_food_snapshot_id {
                            summary["merchant_match"]["pet_food_snapshot_id"] = json!(food_id);
                        }
                        if let Some(plan) = quantity_plan {
                            summary["merchant_match"]["lowest_cost_quantity_plan"] = plan;
                        }
                        if let Some(mix) = bundle_mix {
                            summary["merchant_match"]["lowest_cost_copper_only_bundle_mix"] = mix;
                        }
                    }
                    if filter_instance {
                        summary["instance_context"] = s.sections["instance_context"].clone();
                        if let Some(context) = summary["instance_context"].as_object_mut() {
                            context.remove("lock_warning");
                        }
                    }
                    if filter_recipes && let Some(recipe) = s.sections.get("selected_recipe") {
                        summary["selected_recipe"] = json!({
                            "recipe_id":recipe["recipe_id"],"learned":recipe["learned"],
                            "output_item_id":recipe["output_item_id"]
                        });
                        if filters.profession_id.is_some() {
                            summary["selected_recipe"]["profession_id"] =
                                recipe["profession"]["profession_id"].clone();
                        }
                        if filters.cooldown_state.is_some() {
                            summary["selected_recipe"]["cooldown_state"] =
                                json!(analysis::recipe_cooldown_state(recipe));
                            summary["selected_recipe"]["cooldown"] = recipe["cooldown"].clone();
                        }
                    }
                    let coverage: Vec<_> = coverage_summary_sections
                        .into_iter()
                        .flatten()
                        .map(|name| s.section_capture_status(name))
                        .collect();
                    rows.push((s.captured_at, id.to_owned(), summary, coverage));
                }
                Ok(_) => (),
                Err(_) => invalid += 1,
            }
        }
        if latest_per_character {
            let ids: std::collections::BTreeSet<_> =
                latest.values().map(|(_, id)| id.as_str()).collect();
            rows.retain(|(_, id, _, _)| ids.contains(id.as_str()));
        }
        let coverage = coverage_summary_sections.map(|names| {
            let mut counts = vec![[0; 3]; names.len()];
            for (_, _, _, statuses) in &rows {
                for (count, status) in counts.iter_mut().zip(statuses) {
                    let index = match *status {
                        "observed" => 0,
                        "reported_unavailable" => 1,
                        _ => 2,
                    };
                    count[index] += 1;
                }
            }
            counts
        });
        let coverage = coverage_summary(coverage.as_deref().unwrap_or_default(), rows.len());
        let purchase_cost = |summary: &Value| {
            summary["merchant_match"]["lowest_cost_quantity_plan"]["plan"]["required_copper"]
                .as_u64()
                .or_else(|| summary["merchant_match"]["lowest_cost_copper_only_bundle_mix"]["required_copper"].as_u64())
                .unwrap_or(u64::MAX)
        };
        rows.sort_by(|a, b| {
            let cost_order = if merchant_cheapest_first {
                purchase_cost(&a.2).cmp(&purchase_cost(&b.2))
            } else {
                std::cmp::Ordering::Equal
            };
            cost_order
                .then_with(|| b.0.cmp(&a.0))
                .then_with(|| a.1.cmp(&b.1))
        });
        let has_more = rows.len().saturating_sub(offset) > limit;
        let snapshots: Vec<_> = rows
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|(_, _, summary, _)| summary)
            .collect();
        let mut result = json!({"snapshots":snapshots,"has_more":has_more,"invalid_count":invalid});
        if let Some(summary) = coverage {
            result["section_coverage_summary"] = summary;
        }
        Ok(result)
    }
    pub fn ingest(&self, snapshot: &Snapshot) -> Result<String> {
        snapshot.validate()?;
        let raw = serde_json::to_vec(snapshot)?;
        ensure!(raw.len() as u64 <= MAX_BYTES, "Snapshot exceeds size limit");
        let id = format!("{:x}", Sha256::digest(&raw));
        std::fs::create_dir_all(&self.root)?;
        let path = self.root.join(format!("{id}.json"));
        if path.exists() {
            self.read(&id)?;
            return Ok(id);
        }
        let mut pending = tempfile::NamedTempFile::new_in(&self.root)?;
        pending.write_all(&raw)?;
        pending.as_file().sync_all()?;
        match pending.persist_noclobber(&path) {
            Ok(_) => (),
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                self.read(&id)?;
            }
            Err(e) => return Err(e.error.into()),
        }
        Ok(id)
    }

    pub fn read(&self, id: &str) -> Result<Snapshot> {
        let root = self.root.canonicalize()?;
        Self::read_in_root(&root, id)
    }

    fn read_in_root(root: &Path, id: &str) -> Result<Snapshot> {
        ensure!(
            id.len() == 64
                && id
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
            "Invalid snapshot id"
        );
        let path = root.join(format!("{id}.json")).canonicalize()?;
        ensure!(path.parent() == Some(root), "Snapshot path escaped store");
        let mut raw = Vec::new();
        File::open(path)?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut raw)?;
        ensure!(raw.len() as u64 <= MAX_BYTES, "Snapshot exceeds size limit");
        ensure!(
            format!("{:x}", Sha256::digest(&raw)) == id,
            "Snapshot content hash mismatch"
        );
        let snapshot: Snapshot = serde_json::from_slice(&raw)?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn inspect(&self, id: &str, section: Option<&str>) -> Result<Value> {
        self.inspect_with_coverage(id, section, None)
    }

    pub fn inspect_with_coverage(
        &self,
        id: &str,
        section: Option<&str>,
        coverage_sections: Option<&[String]>,
    ) -> Result<Value> {
        if let Some(names) = coverage_sections {
            ensure!(
                section.is_none(),
                "Coverage inspection excludes section data"
            );
            validate_section_names(names)?;
        }
        let s = self.read(id)?;
        let mut result = json!({"snapshot_id": id, "client": s.client, "character": s.character,
            "source": s.source, "captured_at": s.captured_at, "age_seconds": (Utc::now()-s.captured_at).num_seconds(), "live": false});
        if let Some(name) = section {
            ensure!(SNAPSHOT_SECTIONS.contains(&name), "Unknown section");
            result["available"] = json!(s.sections.contains_key(name));
            result["data"] = s.sections.get(name).cloned().unwrap_or(Value::Null);
            result["unavailable_reason"] = json!(s.unavailable.get(name));
            if name == "pet_care" && s.client.family == Family::Forever {
                result["pet_food_locations"] = json!(analysis::pet_food_locations(&s));
            }
            if name == "repair_quote" && s.client.family == Family::Forever {
                result["repair_status"] = analysis::repair_status(&s);
            }
            if name == "player_pvp_progression" && s.client.family == Family::Forever {
                result["player_pvp_progression_observation"] = json!(
                    s.sections
                        .get(name)
                        .map(analysis::player_pvp_progression_observation)
                );
            }
        } else if let Some(names) = coverage_sections {
            let statuses: BTreeMap<_, _> = names
                .iter()
                .map(|name| (name, s.section_capture_status(name)))
                .collect();
            result["section_coverage"] = json!(statuses);
        } else {
            result["available_sections"] = json!(s.sections.keys().collect::<Vec<_>>());
            result["unavailable"] = json!(s.unavailable);
        }
        Ok(result)
    }
}

pub fn validate_section_names(sections: &[String]) -> Result<()> {
    ensure!(
        !sections.is_empty()
            && sections.len() <= SNAPSHOT_SECTIONS.len()
            && sections
                .iter()
                .all(|name| SNAPSHOT_SECTIONS.contains(&name.as_str()))
            && sections
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == sections.len(),
        "Section filters require distinct known section names"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Snapshot {
        serde_json::from_value(json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z","client":{"family":"retail","variant":"live","build":"123","interface":120105},"character":{"name":"Example","realm":"Test"},"sections":{"equipment":{}}})).unwrap()
    }
    #[test]
    fn roundtrip_and_integrity() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store {
            root: dir.path().into(),
        };
        let id = store.ingest(&fixture()).unwrap();
        assert_eq!(id, store.ingest(&fixture()).unwrap());
        assert_eq!(store.inspect(&id, None).unwrap()["live"], false);
        assert!(store.read("../secret").is_err());
        std::fs::write(dir.path().join(format!("{id}.json")), "{}").unwrap();
        assert!(store.read(&id).is_err());
    }
    #[test]
    fn invalid_envelope() {
        let mut s = fixture();
        s.client.interface = 0;
        assert!(s.validate().is_err());
        let mut s = fixture();
        s.sections.insert("execute".into(), json!({}));
        assert!(s.validate().is_err());
        let mut s = fixture();
        s.captured_at = Utc::now() + chrono::Duration::days(1);
        assert!(s.validate().is_err());
        let mut v = serde_json::to_value(fixture()).unwrap();
        v["client"]["family"] = json!("unknown");
        assert!(serde_json::from_value::<Snapshot>(v).is_err());
    }
    #[test]
    fn concurrent_publication() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store {
            root: dir.path().into(),
        };
        let jobs: Vec<_> = (0..8)
            .map(|_| {
                let store = store.clone();
                std::thread::spawn(move || store.ingest(&fixture()).unwrap())
            })
            .collect();
        let ids: Vec<_> = jobs.into_iter().map(|j| j.join().unwrap()).collect();
        assert!(ids.iter().all(|id| id == &ids[0]));
        assert!(store.read(&ids[0]).is_ok());
    }

    #[test]
    fn listing_preserves_validation_family_order_and_pages() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store {
            root: dir.path().into(),
        };
        let mut expected = Vec::new();
        for name in ["A", "B", "C"] {
            let mut snapshot = fixture();
            snapshot.character.name = name.into();
            expected.push(store.ingest(&snapshot).unwrap());
        }
        expected.sort(); // Equal capture times use ascending content ID.
        let mut other = fixture();
        other.client.family = Family::Forever;
        store.ingest(&other).unwrap();
        let mut invalid = serde_json::to_value(fixture()).unwrap();
        invalid["sections"]["equipment"] = json!({"16":{"item_id":0}});
        let raw = serde_json::to_vec(&invalid).unwrap();
        let id = format!("{:x}", Sha256::digest(&raw));
        std::fs::write(dir.path().join(format!("{id}.json")), raw).unwrap();
        std::fs::write(dir.path().join(format!("{}.json", "a".repeat(64))), "{}").unwrap();
        let first = store.list(Family::Retail, 0, 2).unwrap();
        let last = store.list(Family::Retail, 2, 2).unwrap();
        assert_eq!(first["invalid_count"], 2);
        assert_eq!(first["has_more"], true);
        assert_eq!(last["has_more"], false);
        assert_eq!(first["snapshots"][0]["snapshot_id"], expected[0]);
        assert_eq!(first["snapshots"][1]["snapshot_id"], expected[1]);
        assert_eq!(last["snapshots"][0]["snapshot_id"], expected[2]);
        assert_eq!(
            store.list(Family::Forever, 0, 2).unwrap()["snapshots"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            store.list(Family::Retail, usize::MAX, 2).unwrap()["snapshots"],
            json!([])
        );
        std::fs::write(dir.path().join(format!("{}.json", expected[0])), "{}").unwrap();
        let fresh = store.list(Family::Retail, 0, 100).unwrap();
        assert_eq!(fresh["invalid_count"], 3);
        assert_eq!(fresh["snapshots"].as_array().unwrap().len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn listing_rejects_hash_valid_symlink_outside_store() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("store");
        std::fs::create_dir(&root).unwrap();
        let raw = serde_json::to_vec(&fixture()).unwrap();
        let id = format!("{:x}", Sha256::digest(&raw));
        let outside = dir.path().join("outside.json");
        std::fs::write(&outside, raw).unwrap();
        std::os::unix::fs::symlink(&outside, root.join(format!("{id}.json"))).unwrap();
        let store = Store { root };
        assert!(store.read(&id).is_err());
        let listing = store.list(Family::Retail, 0, 20).unwrap();
        assert_eq!(listing["snapshots"], json!([]));
        assert_eq!(listing["invalid_count"], 1);
    }
}
