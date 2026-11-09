//! Data-only client contracts and snapshot storage. No game execution.
pub mod action_operation;
pub mod analysis;
pub mod bag_operation;
pub mod binding_operation;
pub mod equipment_operation;
pub mod equipment_set_operation;
pub mod forever_catalogue;
pub mod import;
pub mod macro_operation;
pub mod planning;
mod schema;
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
const SECTIONS: &[&str] = &[
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
    "bag_item_cooldowns",
    "gossip_quests",
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Retail,
    Classic,
    Forever,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
                !matches!(
                    name.as_str(),
                    "selected_recipe"
                        | "tracked_recipes"
                        | "reputation"
                        | "currency_balances"
                        | "tracked_achievements"
                        | "wallet"
                        | "experience"
                        | "player_resources"
                        | "player_auras"
                        | "loot_window"
                        | "bag_item_cooldowns"
                        | "gossip_quests"
                        | "spellbook"
                        | "flight_map"
                        | "durability_alerts"
                        | "equipment_durability"
                        | "weapon_enchants"
                        | "repair_quote"
                        | "merchant_stock"
                        | "merchant_buyback"
                        | "trainer_services"
                        | "pet_care"
                        | "pet_action_bar"
                        | "mount_collection"
                        | "toy_collection_view"
                        | "saved_instances"
                        | "saved_world_bosses"
                        | "profession_recipe_view"
                        | "profession_spells"
                        | "profession_recipe_materials"
                        | "recipe_item_sources"
                        | "mail_attachments"
                        | "pet_collection_view"
                        | "skill_lines"
                        | "bag_capacity"
                        | "instance_context"
                ) || self.client.family == Family::Forever,
                "Observation requires Forever"
            );
            schema::validate(name, value)?;
            ensure!(SECTIONS.contains(&name.as_str()), "Unknown section");
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
                SECTIONS.contains(&name.as_str()) && !self.sections.contains_key(name),
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
pub struct SavedInstanceFilter {
    /// Exact case-insensitive captured instance name.
    pub name: String,
    pub difficulty_id: Option<u32>,
    pub locked: Option<bool>,
    pub extended: Option<bool>,
    pub is_raid: Option<bool>,
    /// Only nonnegative reset timers at or below this value, as recorded at capture.
    pub maximum_reset_seconds: Option<u32>,
    /// Maximum age of the last instance-info update at capture, not age today.
    pub maximum_update_age_seconds: Option<u32>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SavedWorldBossFilter {
    /// Exact case-insensitive captured boss name.
    pub name: String,
    /// Match only nonnegative reset timers at or below this captured value.
    pub maximum_reset_seconds: Option<u32>,
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
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PetSpeciesFilter {
    pub species_id: std::num::NonZeroU32,
    /// Minimum owned rows in the captured filtered view, not the whole account.
    pub minimum_owned_rows: Option<u32>,
    pub minimum_unowned_rows: Option<u32>,
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
                for container in section.values() {
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
        Some(json!({
            "item_id": self.item_id,
            "storage": self.storage,
            "observed_quantity": quantity,
            "basis": "same_snapshot_single_storage",
            "complete_ownership_verified": false,
            "current_access_verified": false,
            "shared_storage_scope_verified": false,
            "cross_character_aggregation_performed": false,
            "game_execution": false,
        }))
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

#[derive(Default)]
pub struct CaptureFilters<'a> {
    pub captured_from: Option<&'a str>,
    pub captured_through: Option<&'a str>,
    pub latest_per_character: bool,
    pub inventory_item: Option<InventoryItemFilter>,
    pub recipe: RecipeCaptureFilters,
    pub tracked_recipe_id: Option<std::num::NonZeroU32>,
    pub tracked_recipe_kind: Option<TrackedRecipeKind>,
    pub tracked_recipe_learning: Option<RecipeLearning>,
    pub profession_skill_line: Option<std::num::NonZeroU32>,
    pub profession_min_rank: Option<u32>,
    pub instance: InstanceCaptureFilters<'a>,
    pub player_map_id: Option<std::num::NonZeroU32>,
    pub player_parent_map_id: Option<std::num::NonZeroU32>,
    pub repair_max_quote_copper: Option<u64>,
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
    pub saved_instance: Option<SavedInstanceFilter>,
    pub saved_world_boss: Option<SavedWorldBossFilter>,
    pub profession_recipe: Option<ProfessionRecipeFilter>,
    pub mail_attachment: Option<MailAttachmentFilter>,
    pub pet_species: Option<PetSpeciesFilter>,
    pub active_pet: Option<ActivePetCaptureFilter>,
    pub spellbook_spell: Option<SpellbookSpellFilter>,
    pub minimum_rested_xp: Option<u32>,
    pub xp_gain_enabled_only: bool,
    pub quest_id: Option<std::num::NonZeroU32>,
    pub quest_readiness: Option<analysis::QuestReadiness>,
    pub quest_waypoint_map_id: Option<std::num::NonZeroU32>,
    pub merchant_item_id: Option<std::num::NonZeroU32>,
    pub merchant_currency_id: Option<std::num::NonZeroU32>,
    pub merchant_pet_food_snapshot_id: Option<&'a str>,
    pub merchant_target_quantity: Option<std::num::NonZeroU32>,
    pub merchant_cheapest_first: bool,
    pub merchant_max_total_copper: Option<u64>,
    pub merchant_max_price_copper: Option<u64>,
    pub merchant_in_stock_only: bool,
    pub merchant_copper_only: bool,
    pub merchant_affordable_only: bool,
    pub trainer_service_name: Option<&'a str>,
    pub trainer_service_status: Option<&'a str>,
    pub trainer_max_price_copper: Option<u64>,
    pub trainer_max_price_points: Option<u32>,
    pub trainer_affordable_only: bool,
    pub flight_destination_node_id: Option<std::num::NonZeroU32>,
    pub flight_origin_node_id: Option<std::num::NonZeroU32>,
    pub flight_max_fare_copper: Option<u64>,
    pub flight_departure_map_id: Option<std::num::NonZeroU32>,
    pub flight_affordable_only: bool,
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
    pub currency_id: std::num::NonZeroU32,
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
    pub faction_id: std::num::NonZeroU32,
    /// Inclusive minimum standard reputation reaction, from 1 to 8.
    #[schemars(range(min = 1, max = 8))]
    pub minimum_reaction: u8,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenownFilter {
    pub faction_id: std::num::NonZeroU32,
    pub minimum_level: u32,
    pub capped: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParagonFilter {
    pub faction_id: std::num::NonZeroU32,
    pub reward_pending: Option<bool>,
    pub level_too_low: Option<bool>,
    /// Match the linked reward quest's readiness in the same capture.
    pub reward_quest_readiness: Option<analysis::QuestReadiness>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrackedAchievementFilter {
    pub achievement_id: std::num::NonZeroU32,
    pub completed: Option<bool>,
    pub earned_by_character: Option<bool>,
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
            captured_from,
            captured_through,
            latest_per_character,
            inventory_item,
            recipe: filters,
            tracked_recipe_id,
            tracked_recipe_kind,
            tracked_recipe_learning,
            profession_skill_line,
            profession_min_rank,
            instance,
            player_map_id,
            player_parent_map_id,
            repair_max_quote_copper,
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
            saved_instance,
            saved_world_boss,
            profession_recipe,
            mail_attachment,
            pet_species,
            active_pet,
            spellbook_spell,
            minimum_rested_xp,
            xp_gain_enabled_only,
            quest_id,
            quest_readiness,
            quest_waypoint_map_id,
            merchant_item_id,
            merchant_currency_id,
            merchant_pet_food_snapshot_id,
            merchant_target_quantity,
            merchant_cheapest_first,
            merchant_max_total_copper,
            merchant_max_price_copper,
            merchant_in_stock_only,
            merchant_copper_only,
            merchant_affordable_only,
            trainer_service_name,
            trainer_service_status,
            trainer_max_price_copper,
            trainer_max_price_points,
            trainer_affordable_only,
            flight_destination_node_id,
            flight_origin_node_id,
            flight_max_fare_copper,
            flight_departure_map_id,
            flight_affordable_only,
        } = query;
        if let Some(filter) = &inventory_item {
            filter.validate(&family)?;
        }
        ensure!(
            collected_mount.is_none() || family == Family::Forever,
            "Mount filtering requires Forever"
        );
        let mount_key = collected_mount
            .as_ref()
            .map(|filter| filter.mount_id.to_string());
        ensure!(
            saved_instance
                .as_ref()
                .is_none_or(|filter| family == Family::Forever
                    && !filter.name.trim().is_empty()
                    && filter.name.len() <= 256),
            "Saved instance filtering requires Forever and a bounded name"
        );
        let saved_instance_name = saved_instance
            .as_ref()
            .map(|filter| filter.name.to_lowercase());
        ensure!(
            saved_world_boss
                .as_ref()
                .is_none_or(|filter| family == Family::Forever
                    && !filter.name.trim().is_empty()
                    && filter.name.len() <= 256),
            "Saved world boss filtering requires Forever and a bounded name"
        );
        let saved_world_boss_name = saved_world_boss
            .as_ref()
            .map(|filter| filter.name.to_lowercase());
        ensure!(
            profession_recipe.is_none() || family == Family::Forever,
            "Profession recipe view filtering requires Forever"
        );
        let profession_recipe_key = profession_recipe
            .as_ref()
            .map(|filter| filter.recipe_id.to_string());
        ensure!(
            mail_attachment.is_none() || family == Family::Forever,
            "Pending mail attachment filtering requires Forever"
        );
        ensure!(
            pet_species.is_none() || family == Family::Forever,
            "Pet species view filtering requires Forever"
        );
        let pet_species_key = pet_species
            .as_ref()
            .map(|filter| filter.species_id.to_string());
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
        if let Some(filter) = &spellbook_spell {
            ensure!(
                filter.spell_id.is_some() || filter.resources.is_some(),
                "Spell ID or resource filter required"
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
        ensure!(
            flight_departure_map_id.is_none() || flight_destination_node_id.is_some(),
            "Flight departure map filtering requires a destination"
        );
        ensure!(
            !flight_affordable_only || flight_destination_node_id.is_some(),
            "Flight affordability filtering requires a destination"
        );
        ensure!(
            flight_max_fare_copper.is_none_or(|fare| {
                flight_destination_node_id.is_some() && fare <= 9_999_999_999_999
            }),
            "Flight fare filtering requires a destination and a valid copper amount"
        );
        ensure!(
            flight_origin_node_id.is_none() || flight_destination_node_id.is_some(),
            "Flight origin filtering requires a destination"
        );
        ensure!(
            flight_destination_node_id.is_none() || family == Family::Forever,
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
            !(merchant_in_stock_only || merchant_copper_only || merchant_affordable_only)
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
                && (profession_min_rank.is_none() || profession_skill_line.is_some()),
            "Profession filtering requires Forever and a profession skill line"
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
        let filter_player_map = player_map_id.is_some() || player_parent_map_id.is_some();
        let filter_repair = repair_max_quote_copper.is_some() || repair_affordable_only;
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
            currency_balance.is_none() || family == Family::Forever,
            "Currency balance filtering requires Forever"
        );
        let currency_key = currency_balance
            .as_ref()
            .map(|filter| filter.currency_id.to_string());
        ensure!(
            reputation_rank.as_ref().is_none_or(|filter| {
                family == Family::Forever && (1..=8).contains(&filter.minimum_reaction)
            }),
            "Reputation filtering requires Forever and a standard reaction from 1 to 8"
        );
        let reputation_key = reputation_rank
            .as_ref()
            .map(|filter| filter.faction_id.to_string());
        ensure!(
            renown.is_none() || family == Family::Forever,
            "Renown filtering requires Forever"
        );
        let renown_key = renown.as_ref().map(|filter| filter.faction_id.to_string());
        ensure!(
            paragon.is_none() || family == Family::Forever,
            "Paragon filtering requires Forever"
        );
        let paragon_key = paragon.as_ref().map(|filter| filter.faction_id.to_string());
        ensure!(
            tracked_achievement.is_none() || family == Family::Forever,
            "Tracked achievement filtering requires Forever"
        );
        let achievement_key = tracked_achievement
            .as_ref()
            .map(|filter| filter.achievement_id.to_string());
        ensure!(
            (quest_id.is_none() && quest_readiness.is_none() && quest_waypoint_map_id.is_none())
                || family == Family::Forever,
            "Quest filtering requires Forever"
        );
        let quest_key = quest_id.map(|id| id.to_string());
        ensure!(
            !filter_experience || family == Family::Forever,
            "Experience filtering requires Forever"
        );
        ensure!(
            (!filter_repair || family == Family::Forever)
                && repair_max_quote_copper.is_none_or(|price| price <= 9_999_999_999_999),
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
        if !self.root.exists() {
            return Ok(json!({"snapshots":[],"has_more":false,"invalid_count":0}));
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
                    let pet_species_match=pet_species.as_ref().zip(pet_species_key.as_ref()).and_then(|(filter,key)| {
                        let view=s.sections.get("pet_collection_view")?;
                        let row=view["entries"].get(key)?;
                        (filter.minimum_owned_rows.is_none_or(|minimum|row["owned_rows"].as_u64().unwrap()>=u64::from(minimum))
                            && filter.minimum_unowned_rows.is_none_or(|minimum|row["unowned_rows"].as_u64().unwrap()>=u64::from(minimum)))
                            .then(||json!({"coverage":view["coverage"],"displayed_rows":view["displayed_rows"],
                                "species_id":filter.species_id,"name":row["name"],
                                "owned_rows":row["owned_rows"],"unowned_rows":row["unowned_rows"]}))
                    });
                    if pet_species.is_some() && pet_species_match.is_none() {
                        continue;
                    }
                    let active_pet_match = active_pet
                        .as_ref()
                        .and_then(|filter| filter.observation_match(s.sections.get("pet_care")?));
                    if active_pet.is_some() && active_pet_match.is_none() {
                        continue;
                    }
                    let spellbook_match=spellbook_spell.as_ref().and_then(|filter| {
                        let book=s.sections.get("spellbook")?;
                        let requested=filter.spell_id.map(|id|u64::from(id.get()));
                        let mut resource_unknown=0;
                        let mut resource_nonmatching=0;
                        let mut matches=book["entries"].as_object()?.iter().filter(|(_,row)| {
                            (requested.is_none() || row["base_spell_id"].as_u64()==requested || row["spell_id"].as_u64()==requested)
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
                        let (kind,field)=match filter.kind {
                            MailAttachmentKind::Item=>("item","item_id"),
                            MailAttachmentKind::Currency=>("currency","currency_id"),
                        };
                        let mut locations=Vec::new();
                        for (message,attachments) in mail["messages"].as_object()?.iter() {
                            for (slot,row) in attachments.as_object()?.iter() {
                                if row["kind"]==kind && row[field].as_u64()==Some(u64::from(filter.id.get())) {
                                    locations.push((message.parse::<u32>().unwrap(),slot.parse::<u32>().unwrap(),row["count"].as_u64().unwrap()));
                                }
                            }
                        }
                        // Validated coverage bounds this sum to 100 * 16 * u32::MAX.
                        let quantity:u64=locations.iter().map(|(_,_,count)| count).sum();
                        if locations.is_empty() || quantity<filter.minimum_quantity.unwrap_or(1) {return None;}
                        locations.sort_by_key(|(message,slot,_)| (*message,*slot));
                        Some(json!({"coverage":mail["coverage"],"loaded_messages":mail["loaded_messages"],
                            "total_messages":mail["total_messages"],"kind":filter.kind,"id":filter.id,
                            "quantity":quantity,"matched_locations":locations.len(),"truncated":locations.len()>20,
                            "locations":locations.iter().take(20).map(|(message,slot,count)|json!({
                                "message_slot":message,"attachment_slot":slot,"count":count})).collect::<Vec<_>>()}))
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
                    let saved_world_boss_match = saved_world_boss.as_ref().zip(saved_world_boss_name.as_ref()).and_then(|(filter, name)| {
                        let saved = s.sections.get("saved_world_bosses")?;
                        if !filter.maximum_update_age_seconds.is_none_or(|maximum|
                            saved["update_age_seconds"].as_u64().is_some_and(|age| age <= u64::from(maximum))) {
                            return None;
                        }
                        let mut matches: Vec<_> = saved["entries"].as_object()?.iter().filter(|(_, row)| {
                            row["name"].as_str().is_some_and(|value| value.to_lowercase() == *name)
                                && filter.maximum_reset_seconds.is_none_or(|maximum|
                                    row["reset_seconds"].as_u64().is_some_and(|seconds| seconds <= u64::from(maximum)))
                        }).collect();
                        matches.sort_by_key(|(slot, _)| slot.parse::<u32>().unwrap());
                        (!matches.is_empty()).then(|| json!({"update_age_seconds":saved["update_age_seconds"],
                            "matched_count":matches.len(),"truncated":matches.len()>20,
                            "entries":matches.iter().take(20).map(|(slot,row)| json!({"slot":slot.parse::<u32>().unwrap(),"data":row})).collect::<Vec<_>>() }))
                    });
                    if saved_world_boss.is_some() && saved_world_boss_match.is_none() {
                        continue;
                    }
                    let saved_instance_match = saved_instance.as_ref().zip(saved_instance_name.as_ref()).and_then(|(filter, name)| {
                        let saved = s.sections.get("saved_instances")?;
                        if !filter.maximum_update_age_seconds.is_none_or(|maximum|
                            saved["update_age_seconds"].as_u64().is_some_and(|age| age <= u64::from(maximum))) {
                            return None;
                        }
                        let mut matches: Vec<_> = saved["entries"].as_object()?.iter().filter(|(_, row)| {
                            row["name"].as_str().is_some_and(|value| value.to_lowercase() == *name)
                                && filter.difficulty_id.is_none_or(|id| row["difficulty_id"].as_u64() == Some(u64::from(id)))
                                && filter.locked.is_none_or(|flag| row["locked"].as_bool() == Some(flag))
                                && filter.extended.is_none_or(|flag| row["extended"].as_bool() == Some(flag))
                                && filter.is_raid.is_none_or(|flag| row["is_raid"].as_bool() == Some(flag))
                                && filter.maximum_reset_seconds.is_none_or(|maximum|
                                    row["reset_seconds"].as_u64().is_some_and(|seconds| seconds <= u64::from(maximum)))
                        }).collect();
                        matches.sort_by_key(|(slot, _)| slot.parse::<u32>().unwrap());
                        (!matches.is_empty()).then(|| json!({"update_age_seconds":saved["update_age_seconds"],
                            "matched_count":matches.len(),"truncated":matches.len()>20,
                            "entries":matches.iter().take(20).map(|(slot,row)| json!({"slot":slot.parse::<u32>().unwrap(),"data":row})).collect::<Vec<_>>() }))
                    });
                    if saved_instance.is_some() && saved_instance_match.is_none() {
                        continue;
                    }
                    let mount_match = collected_mount.as_ref().zip(mount_key.as_ref()).and_then(
                        |(filter, key)| {
                            let row = s.sections.get("mount_collection")?["entries"].get(key)?;
                            (filter
                                .usable
                                .is_none_or(|expected| row["usable"].as_bool() == Some(expected))
                                && filter.favorite.is_none_or(|expected| {
                                    row["favorite"].as_bool() == Some(expected)
                                }))
                            .then_some(row)
                        },
                    );
                    if collected_mount.is_some() && mount_match.is_none() {
                        continue;
                    }
                    let achievement_match = tracked_achievement
                        .as_ref()
                        .zip(achievement_key.as_ref())
                        .and_then(|(filter, key)| {
                            let row =
                                s.sections.get("tracked_achievements")?["entries"].get(key)?;
                            (filter
                                .completed
                                .is_none_or(|value| row["completed"].as_bool() == Some(value))
                                && filter.earned_by_character.is_none_or(|value| {
                                    row["earned_by_character"].as_bool() == Some(value)
                                }))
                            .then_some(row)
                        });
                    if tracked_achievement.is_some() && achievement_match.is_none() {
                        continue;
                    }
                    let paragon_match =
                        paragon
                            .as_ref()
                            .zip(paragon_key.as_ref())
                            .and_then(|(filter, key)| {
                                let row = s.sections.get("reputation")?["entries"].get(key)?;
                                let detail = row["paragon"].as_object()?;
                                let readiness = detail
                                    .get("reward_quest_id")
                                    .and_then(Value::as_u64)
                                    .and_then(|id| s.sections.get("quests")?.get(id.to_string()))
                                    .map(analysis::quest_readiness);
                                (row["kind"] == "paragon"
                                    && filter.reward_pending.is_none_or(|flag| {
                                        detail["reward_pending"].as_bool() == Some(flag)
                                    })
                                    && filter.level_too_low.is_none_or(|flag| {
                                        detail["level_too_low"].as_bool() == Some(flag)
                                    })
                                    && filter
                                        .reward_quest_readiness
                                        .is_none_or(|expected| readiness == Some(expected)))
                                .then_some((row, readiness))
                            });
                    if paragon.is_some() && paragon_match.is_none() {
                        continue;
                    }
                    let renown_match =
                        renown
                            .as_ref()
                            .zip(renown_key.as_ref())
                            .and_then(|(filter, key)| {
                                let row = s.sections.get("reputation")?["entries"].get(key)?;
                                (row["kind"] == "renown"
                                    && row["renown"]["level"].as_u64()?
                                        >= u64::from(filter.minimum_level)
                                    && filter.capped.is_none_or(|capped| {
                                        row["renown"]["capped"].as_bool() == Some(capped)
                                    }))
                                .then_some(row)
                            });
                    if renown.is_some() && renown_match.is_none() {
                        continue;
                    }
                    let reputation_match = reputation_rank
                        .as_ref()
                        .zip(reputation_key.as_ref())
                        .and_then(|(filter, key)| {
                            let row = s.sections.get("reputation")?["entries"].get(key)?;
                            (row["kind"] == "standard"
                                && row["reaction"].as_u64()? >= u64::from(filter.minimum_reaction))
                            .then_some(row)
                        });
                    if reputation_rank.is_some() && reputation_match.is_none() {
                        continue;
                    }
                    let currency_match = currency_balance
                        .as_ref()
                        .zip(currency_key.as_ref())
                        .and_then(|(filter, key)| {
                            let row = s.sections.get("currency_balances")?["entries"].get(key)?;
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
                            .then_some(row)
                        });
                    if currency_balance.is_some() && currency_match.is_none() {
                        continue;
                    }
                    let matches_quest = |quest: &Value| {
                        quest_readiness
                            .is_none_or(|expected| analysis::quest_readiness(quest) == expected)
                            && quest_waypoint_map_id.is_none_or(|map| {
                                quest["next_waypoint_map_id"].as_u64() == Some(u64::from(map.get()))
                            })
                    };
                    let quest_match = quest_key.as_ref().and_then(|key| {
                        let quest = s.sections.get("quests")?.get(key)?;
                        matches_quest(quest).then(|| analysis::quest_readiness(quest))
                    });
                    if quest_key.is_some() && quest_match.is_none() {
                        continue;
                    }
                    let quest_count = (quest_key.is_none()
                        && (quest_readiness.is_some() || quest_waypoint_map_id.is_some()))
                    .then(|| {
                        s.sections
                            .get("quests")
                            .and_then(Value::as_object)
                            .map_or(0, |quests| {
                                quests.values().filter(|quest| matches_quest(quest)).count()
                            })
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
                    let repair_quote = if filter_repair {
                        let quote = s
                            .sections
                            .get("repair_quote")
                            .and_then(|row| row["copper"].as_u64());
                        let Some(quote) = quote.filter(|quote| {
                            repair_max_quote_copper.is_none_or(|maximum| *quote <= maximum)
                                && (!repair_affordable_only
                                    || s.sections
                                        .get("wallet")
                                        .and_then(|wallet| wallet["copper"].as_u64())
                                        .is_some_and(|balance| *quote <= balance))
                        }) else {
                            continue;
                        };
                        Some(quote)
                    } else {
                        None
                    };
                    let player_map = s.sections.get("stats").filter(|stats| {
                        player_map_id
                            .is_none_or(|id| stats["map_id"].as_u64() == Some(u64::from(id.get())))
                            && player_parent_map_id.is_none_or(|id| {
                                stats["map_parent_id"].as_u64() == Some(u64::from(id.get()))
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
                            profession_min_rank.is_none_or(|minimum| {
                                row["rank"]
                                    .as_u64()
                                    .is_some_and(|rank| rank >= u64::from(minimum))
                            })
                        })
                    });
                    if profession_skill_line.is_some() && profession_match.is_none() {
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
                            .map_or((0, None), |offers| {
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
                                    return (matching.count(), None);
                                };
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
                                (count, best.map(|(_, plan)| plan))
                            })
                    });
                    if matches!(merchant_matches, Some((0, _))) {
                        continue;
                    }
                    let trainer_match = filter_trainer.then(|| {
                        let trainer = s.sections.get("trainer_services")?;
                        let (balance_field, balance) = if trainer["kind"] == "pet" {
                            ("price_training_points", trainer["pet_training_points_available"].as_u64())
                        } else {
                            ("price_copper", s.sections.get("wallet").and_then(|wallet| wallet["copper"].as_u64()))
                        };
                        let count = trainer["entries"].as_object()?.values()
                            .filter(|row| {
                                trainer_service_status.is_none_or(|status| row["status"] == status)
                                    && (!trainer_affordable_only || row[balance_field].as_u64()
                                        .zip(balance).is_some_and(|(price, balance)| price <= balance))
                                    && trainer_price.is_none_or(|(field, maximum)| {
                                        row[field].as_u64().is_some_and(|price| price <= maximum)
                                    })
                                    && trainer_service_name.as_ref().is_none_or(|name| {
                                        row["name"].as_str().is_some_and(|lesson| lesson.to_lowercase().contains(name))
                                    })
                            })
                            .count();
                        (count > 0).then(|| json!({"lesson_count":count,"kind":trainer["kind"],
                            "captured_player_map_id":s.sections.get("stats").and_then(|stats| stats["map_id"].as_u64())}))
                    }).flatten();
                    if filter_trainer && trainer_match.is_none() {
                        continue;
                    }
                    let flight_match = flight_destination_node_id.and_then(|node| {
                        let player_map = s
                            .sections
                            .get("stats")
                            .and_then(|stats| stats["map_id"].as_u64());
                        if flight_departure_map_id
                            .is_some_and(|map| player_map != Some(u64::from(map.get())))
                        {
                            return None;
                        }
                        analysis::captured_flight_route(s.sections.get("flight_map")?, node.get())
                            .filter(|route| {
                                let fare = route["fare_copper"].as_u64();
                                flight_origin_node_id.is_none_or(|origin| {
                                    route["origin_node_id"].as_u64()
                                        == Some(u64::from(origin.get()))
                                }) && flight_max_fare_copper
                                    .is_none_or(|maximum| fare.is_some_and(|fare| fare <= maximum))
                                    && (!flight_affordable_only
                                        || s.sections
                                            .get("wallet")
                                            .and_then(|wallet| wallet["copper"].as_u64())
                                            .zip(fare)
                                            .is_some_and(|(balance, fare)| fare <= balance))
                            })
                            .map(|mut route| {
                                if flight_departure_map_id.is_some() {
                                    route["captured_departure_map_id"] = json!(player_map);
                                }
                                route
                            })
                    });
                    if flight_destination_node_id.is_some() && flight_match.is_none() {
                        continue;
                    }
                    // Retain metadata only: listing must not hold every inventory payload.
                    let mut summary = json!({"snapshot_id":id,"character":s.character,"client":s.client,"captured_at":s.captured_at,"source":s.source});
                    if let Some((filter, row)) = tracked_achievement.as_ref().zip(achievement_match)
                    {
                        summary["tracked_achievement_match"] = json!({
                            "achievement_id":filter.achievement_id,"completed":row["completed"],
                            "earned_by_character":row["earned_by_character"],"account_wide":row["account_wide"],
                        });
                    }
                    if let Some((filter, row)) = renown.as_ref().zip(renown_match) {
                        summary["renown_match"] = json!({
                            "faction_id":filter.faction_id,"level":row["renown"]["level"],
                            "capped":row["renown"]["capped"],"account_wide":row["account_wide"],
                        });
                    }
                    if let Some((filter, (row, readiness))) = paragon.as_ref().zip(paragon_match) {
                        summary["paragon_match"] = json!({
                            "faction_id":filter.faction_id,"reward_pending":row["paragon"]["reward_pending"],
                            "level_too_low":row["paragon"]["level_too_low"],
                            "reward_quest_id":row["paragon"]["reward_quest_id"],
                            "account_wide":row["account_wide"],
                        });
                        if filter.reward_quest_readiness.is_some() {
                            summary["paragon_match"]["reward_quest_readiness"] = json!(readiness);
                        }
                    }
                    if let Some((filter, row)) = reputation_rank.as_ref().zip(reputation_match) {
                        summary["reputation_rank_match"] = json!({
                            "faction_id":filter.faction_id,"reaction":row["reaction"],
                            "account_wide":row["account_wide"],
                        });
                    }
                    if let Some((filter, row)) = currency_balance.as_ref().zip(currency_match) {
                        summary["currency_balance_match"] = json!({
                            "currency_id":filter.currency_id,"quantity":row["quantity"],
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
                    if let Some(map) = quest_waypoint_map_id {
                        summary["quest_match"]["waypoint_map_id"] = json!(map);
                    }
                    if let Some(observation) = saved_instance_match {
                        summary["saved_instance_match"] = observation;
                    }
                    if let Some(observation) = saved_world_boss_match {
                        summary["saved_world_boss_match"] = observation;
                    }
                    if let Some(observation) = profession_recipe_match {
                        summary["profession_recipe_match"] = observation;
                    }
                    if let Some(observation) = mail_attachment_match {
                        summary["mail_attachment_match"] = observation;
                    }
                    if let Some(observation) = pet_species_match {
                        summary["pet_species_match"] = observation;
                    }
                    if let Some(observation) = inventory_item_match {
                        summary["inventory_item_match"] = observation;
                    }
                    if let Some(observation) = active_pet_match {
                        summary["active_pet_match"] = observation;
                    }
                    if let Some(observation) = spellbook_match {
                        summary["spellbook_spell_match"] = observation;
                    }
                    if let Some((filter, row)) = collected_mount.as_ref().zip(mount_match) {
                        summary["collected_mount_match"] = json!({"mount_id":filter.mount_id,
                            "spell_id":row["spell_id"],"usable":row["usable"],"favorite":row["favorite"]});
                    }
                    if let Some(space) = bag_space {
                        summary["carried_bag_space"] = space;
                    }
                    if let Some(quote) = repair_quote {
                        summary["repair_quote_match"] = json!({"copper":quote});
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
                    }
                    if let Some(trainer) = trainer_match {
                        summary["trainer_match"] = trainer;
                    }
                    if let Some(route) = flight_match {
                        summary["flight_match"] = route;
                    }
                    if let Some((count, quantity_plan)) = merchant_matches {
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
                    rows.push((s.captured_at, id.to_owned(), summary));
                }
                Ok(_) => (),
                Err(_) => invalid += 1,
            }
        }
        if latest_per_character {
            let ids: std::collections::BTreeSet<_> =
                latest.values().map(|(_, id)| id.as_str()).collect();
            rows.retain(|(_, id, _)| ids.contains(id.as_str()));
        }
        let purchase_cost = |summary: &Value| {
            summary["merchant_match"]["lowest_cost_quantity_plan"]["plan"]["required_copper"]
                .as_u64()
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
            .map(|(_, _, summary)| summary)
            .collect();
        Ok(json!({"snapshots":snapshots,"has_more":has_more,"invalid_count":invalid}))
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
        let s = self.read(id)?;
        let mut result = json!({"snapshot_id": id, "client": s.client, "character": s.character,
            "source": s.source, "captured_at": s.captured_at, "age_seconds": (Utc::now()-s.captured_at).num_seconds(), "live": false});
        if let Some(name) = section {
            ensure!(SECTIONS.contains(&name), "Unknown section");
            result["available"] = json!(s.sections.contains_key(name));
            result["data"] = s.sections.get(name).cloned().unwrap_or(Value::Null);
            result["unavailable_reason"] = json!(s.unavailable.get(name));
        } else {
            result["available_sections"] = json!(s.sections.keys().collect::<Vec<_>>());
            result["unavailable"] = json!(s.unavailable);
        }
        Ok(result)
    }
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
