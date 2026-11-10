//! Snapshot-only analysis. Missing data is not an empty inventory.
use crate::Snapshot;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MapLevelPosition {
    Below,
    Within,
    Above,
}

pub fn map_level_position(stats: &Value) -> Option<MapLevelPosition> {
    map_level_position_at(stats, stats["level"].as_i64()?)
}

pub fn map_level_position_at(stats: &Value, level: i64) -> Option<MapLevelPosition> {
    let minimum = stats["map_min_level"].as_i64()?;
    let maximum = stats["map_max_level"].as_i64()?;
    Some(if level < minimum {
        MapLevelPosition::Below
    } else if level > maximum {
        MapLevelPosition::Above
    } else {
        MapLevelPosition::Within
    })
}

#[derive(Default)]
struct ObservedReagent {
    quantities: [u64; 3],
    stack_count: usize,
    locations: Vec<Value>,
}

pub fn equipment_durability_matching_slots(section: &Value, maximum: u8) -> Option<BTreeSet<u32>> {
    section["entries"].as_object().map(|entries| {
        entries
            .iter()
            .filter_map(|(slot, row)| {
                let current = row["current"].as_u64().unwrap();
                let capacity = row["maximum"].as_u64().unwrap();
                (current * 100 <= capacity * u64::from(maximum))
                    .then(|| slot.parse::<u32>().unwrap())
            })
            .collect()
    })
}

pub(crate) fn temporary_weapon_enchant_expiry(
    row: &Value,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    (row["has_enchant"] == true && row["enchant_type"] == "temporary").then_some(())?;
    let duration = chrono::TimeDelta::try_milliseconds(row["remaining_milliseconds"].as_i64()?)?;
    captured_at.checked_add_signed(duration)
}

pub(crate) fn equipped_weapon_hand_enchants(snapshot: &Snapshot, slot: &str) -> Option<Value> {
    let hand = match slot {
        "16" => "main_hand",
        "17" => "off_hand",
        "18" => "ranged",
        _ => return None,
    };
    let entries =
        snapshot.sections.get("weapon_enchants")?["hands"][hand]["entries"].as_object()?;
    Some(json!({"hand":hand,"entries":entries}))
}

pub fn weapon_enchant_expiry_review(
    section: &Value,
    captured_at: chrono::DateTime<chrono::Utc>,
    reference: chrono::DateTime<chrono::Utc>,
) -> Option<Value> {
    let hands = section["hands"].as_object()?;
    let projections: BTreeMap<_, _> = ["main_hand", "off_hand", "ranged"].into_iter().map(|hand| {
        let rows = hands.get(hand).and_then(|view| view["entries"].as_object()).map(|entries| {
            entries.iter().filter(|(_, row)| row["has_enchant"] == true && row["enchant_type"] == "temporary")
                .map(|(slot, row)| {
                    let expiry = temporary_weapon_enchant_expiry(row, captured_at);
                    (slot.parse::<u32>().unwrap(), json!({"enchant_id":row["enchant_id"],
                        "reported_remaining_milliseconds":row["remaining_milliseconds"],
                        "projected_expiry_at":expiry,"projected_expiry_at_or_before_reference":expiry.map(|expiry| expiry <= reference)}))
                }).collect::<BTreeMap<_, _>>()
        });
        (hand, rows)
    }).collect();
    Some(
        json!({"reference_at":reference,"hands":projections,"current_enchant_verified":false,"live":false}),
    )
}

#[derive(
    Clone, Copy, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RecipeSearchOrder {
    #[default]
    RecipeId,
    ObservedItemCapacityDesc,
    ObservedSoleShortfallAsc,
    ReportedSkillUpsDesc,
    ReportedSkillUpsPerReagentDesc,
    ReportedCooldownRemainingAsc,
    ReportedBaseOutputMinimumDesc,
    ReportedBaseOutputPerReagentDesc,
}

#[derive(
    Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RecipeCooldownState {
    Available,
    Waiting,
    Unknown,
}

fn estimated_recipe_cooldown_end(
    recipe: &Value,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    recipe["cooldown"]["remaining_seconds"]
        .as_i64()
        .filter(|seconds| *seconds > 0)
        .and_then(chrono::TimeDelta::try_seconds)
        .and_then(|remaining| captured_at.checked_add_signed(remaining))
}

pub fn recipe_cooldown_state(recipe: &Value) -> RecipeCooldownState {
    if let Some(charges) = recipe["cooldown"]["charges"].as_u64() {
        return if charges > 0 {
            RecipeCooldownState::Available
        } else {
            RecipeCooldownState::Waiting
        };
    }
    match recipe["cooldown"]["remaining_seconds"].as_u64() {
        Some(0) => RecipeCooldownState::Available,
        Some(_) => RecipeCooldownState::Waiting,
        None => RecipeCooldownState::Unknown,
    }
}

pub fn review_recipe(snapshot: &Snapshot, crafts: u32) -> Result<Value> {
    review_recipe_with_inventory(snapshot, crafts, None)
}

pub fn recipe_tracking(snapshot: &Snapshot, recipe_id: u64) -> Value {
    let tracked = snapshot.sections.get("tracked_recipes");
    let key = recipe_id.to_string();
    let observation = |group: &str| tracked?.get(group)?.as_object().map(|rows| rows.get(&key));
    let craft = observation("crafts");
    let recraft = observation("recrafts");
    json!({
        "tracked_as_craft":craft.map(|row| row.is_some()),
        "tracked_as_recraft":recraft.map(|row| row.is_some()),
        "tracked_craft_learned":craft.flatten().and_then(|row| row["learned"].as_bool()),
        "tracked_recraft_learned":recraft.flatten().and_then(|row| row["learned"].as_bool())
    })
}

pub fn plan_recipe(
    snapshot: &Snapshot,
    crafts: Option<u32>,
    target_output_quantity: Option<std::num::NonZeroU32>,
    inventory: Option<&Snapshot>,
) -> Result<Value> {
    match (crafts, target_output_quantity) {
        (Some(crafts), None) => review_recipe_with_inventory(snapshot, crafts, inventory),
        (None, Some(target)) => {
            let minimum = snapshot
                .sections
                .get("selected_recipe")
                .and_then(|recipe| recipe["base_output_quantity"]["minimum"].as_u64())
                .and_then(|quantity| u32::try_from(quantity).ok())
                .filter(|quantity| *quantity > 0)
                .ok_or_else(|| anyhow::anyhow!("Recipe base output range unavailable"))?;
            let crafts = target.get().div_ceil(minimum);
            let mut review = review_recipe_with_inventory(snapshot, crafts, inventory)?;
            review["target_output_quantity"] = json!(target.get());
            Ok(review)
        }
        _ => anyhow::bail!("Supply one craft count or target output quantity"),
    }
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecipeMaterialStatus {
    FixedItems,
    Unavailable,
    UnsupportedRequiredSlots,
    BudgetExhausted,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecipeMaterialScope {
    Bags,
    AllObservedStorage,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecipeMaterialFilter {
    pub scope: RecipeMaterialScope,
    pub minimum_crafts: std::num::NonZeroU32,
    /// Return known observed shortfalls instead of recipes that meet the craft target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insufficient_only: Option<bool>,
    /// Match only recipes whose sole observed reagent shortfall is this item. Requires insufficient_only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sole_shortfall_item_id: Option<std::num::NonZeroU32>,
    /// Inclusive missing quantity limit for the selected sole shortfall item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_sole_shortfall_quantity: Option<u64>,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecipeViewFilter {
    pub learned: Option<bool>,
    pub can_skill_up: Option<bool>,
    pub minimum_skill_ups_reported: Option<u32>,
    /// Inclusive upper bound on the client-reported skill-up count.
    pub maximum_skill_ups_reported: Option<u32>,
    /// Exact client-reported difficulty. Missing observations are unevaluable.
    pub relative_difficulty: Option<RecipeRelativeDifficulty>,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecipeRelativeDifficulty {
    Optimal,
    Medium,
    Easy,
    Trivial,
}

impl RecipeRelativeDifficulty {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Optimal => "optimal",
            Self::Medium => "medium",
            Self::Easy => "easy",
            Self::Trivial => "trivial",
        }
    }
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecipeCooldownFilter {
    pub state: Option<RecipeCooldownState>,
    /// Match an explicitly reported daily flag. An absent flag is unknown.
    pub daily: Option<bool>,
    pub minimum_charges: Option<u32>,
    pub maximum_remaining_seconds: Option<u32>,
    /// Inclusive RFC 3339 deadline for a positive reported timer's estimated end.
    /// Zero and missing timers have no estimate.
    pub estimated_end_by: Option<chrono::DateTime<chrono::Utc>>,
}

fn recipe_cooldown_matches(
    row: &Value,
    filter: &RecipeCooldownFilter,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> Option<bool> {
    let checks: Vec<_> = [
        filter
            .state
            .map(|state| Some(recipe_cooldown_state(row) == state)),
        filter.daily.map(|expected| {
            row["cooldown"]["daily"]
                .as_bool()
                .map(|daily| daily == expected)
        }),
        filter.minimum_charges.map(|minimum| {
            row["cooldown"]["charges"]
                .as_u64()
                .map(|charges| charges >= u64::from(minimum))
        }),
        filter.maximum_remaining_seconds.map(|maximum| {
            row["cooldown"]["remaining_seconds"]
                .as_u64()
                .map(|seconds| seconds <= u64::from(maximum))
        }),
        filter.estimated_end_by.map(|deadline| {
            estimated_recipe_cooldown_end(row, captured_at).map(|end| end <= deadline)
        }),
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

fn recipe_view_matches(row: Option<&Value>, filter: &RecipeViewFilter) -> Option<bool> {
    let checks: Vec<_> = [
        filter.learned.map(|expected| {
            row.and_then(|r| r["learned"].as_bool())
                .map(|value| value == expected)
        }),
        filter.can_skill_up.map(|expected| {
            row.and_then(|r| r["can_skill_up"].as_bool())
                .map(|value| value == expected)
        }),
        filter.minimum_skill_ups_reported.map(|minimum| {
            row.and_then(|r| r["skill_ups_reported"].as_u64())
                .map(|value| value >= u64::from(minimum))
        }),
        filter.maximum_skill_ups_reported.map(|maximum| {
            row.and_then(|r| r["skill_ups_reported"].as_u64())
                .map(|value| value <= u64::from(maximum))
        }),
        filter.relative_difficulty.map(|expected| {
            row.and_then(|r| r["relative_difficulty"].as_str())
                .map(|value| value == expected.as_str())
        }),
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

struct RecipeMaterialStock {
    quantities: BTreeMap<u64, u64>,
    observed: Vec<&'static str>,
    missing: Vec<&'static str>,
}

impl RecipeMaterialStock {
    fn from_snapshot(snapshot: &Snapshot, scope: RecipeMaterialScope) -> Result<Self> {
        let storages: &[&'static str] = match scope {
            RecipeMaterialScope::Bags => &["bags"],
            RecipeMaterialScope::AllObservedStorage => &["bags", "character_bank", "account_bank"],
        };
        let mut stock = Self {
            quantities: BTreeMap::new(),
            observed: Vec::new(),
            missing: Vec::new(),
        };
        for &storage in storages {
            let Some(containers) = snapshot.sections.get(storage).and_then(Value::as_object) else {
                stock.missing.push(storage);
                continue;
            };
            stock.observed.push(storage);
            for item in containers
                .values()
                .filter_map(Value::as_object)
                .flat_map(|slots| slots.values())
            {
                if let (Some(id), Some(count)) = (item["item_id"].as_u64(), item["count"].as_u64())
                {
                    let quantity = stock.quantities.entry(id).or_default();
                    *quantity = quantity
                        .checked_add(count)
                        .ok_or_else(|| anyhow::anyhow!("Observed material quantity overflow"))?;
                }
            }
        }
        Ok(stock)
    }

    fn support(&self, row: &Value, minimum: u32) -> Option<Value> {
        if row["status"] != "fixed_items" {
            return None;
        }
        let reagents = row["reagents"].as_object()?;
        if !reagents.is_empty() && self.observed.is_empty() {
            return None;
        }
        let mut capacity: Option<u64> = None;
        for (id, quantity) in reagents {
            let id = id.parse::<u32>().ok()?;
            let required = quantity.as_u64().filter(|value| *value > 0)?;
            let supported = self.quantities.get(&u64::from(id)).copied().unwrap_or(0) / required;
            capacity = Some(capacity.map_or(supported, |current| current.min(supported)));
        }
        Some(
            json!({"supported_crafts":capacity,"no_required_items":reagents.is_empty(),
            "covers_minimum_crafts":capacity.is_none_or(|value|value >= u64::from(minimum))}),
        )
    }

    fn shortfalls(&self, row: &Value, crafts: u32) -> Option<BTreeMap<u32, Value>> {
        let mut shortfalls = BTreeMap::new();
        for (id, quantity) in row["reagents"].as_object()? {
            let id = id.parse::<u32>().ok()?;
            let required = quantity.as_u64()?.checked_mul(u64::from(crafts))?;
            let observed = self.quantities.get(&u64::from(id)).copied().unwrap_or(0);
            if observed < required {
                shortfalls.insert(
                    id,
                    json!({"required":required,"observed":observed,"missing":required-observed}),
                );
            }
        }
        Some(shortfalls)
    }
}

/// Search only observed rows; unknown fields do not satisfy positive filters.
pub fn search_recipe_materials(
    snapshot: &Snapshot,
    recipe_id: Option<std::num::NonZeroU32>,
    output_item_id: Option<std::num::NonZeroU32>,
    reagent_item_id: Option<std::num::NonZeroU32>,
    status: Option<RecipeMaterialStatus>,
    offset: usize,
    limit: usize,
) -> Result<Value> {
    search_recipe_materials_with_filter(
        snapshot,
        recipe_id,
        output_item_id,
        reagent_item_id,
        status,
        offset,
        limit,
        None,
    )
}

pub fn search_recipe_materials_with_filter(
    snapshot: &Snapshot,
    recipe_id: Option<std::num::NonZeroU32>,
    output_item_id: Option<std::num::NonZeroU32>,
    reagent_item_id: Option<std::num::NonZeroU32>,
    status: Option<RecipeMaterialStatus>,
    offset: usize,
    limit: usize,
    material_filter: Option<&RecipeMaterialFilter>,
) -> Result<Value> {
    search_recipe_materials_with_observations(
        snapshot,
        recipe_id,
        output_item_id,
        reagent_item_id,
        status,
        offset,
        limit,
        material_filter,
        None,
    )
}

pub fn search_recipe_materials_with_observations(
    snapshot: &Snapshot,
    recipe_id: Option<std::num::NonZeroU32>,
    output_item_id: Option<std::num::NonZeroU32>,
    reagent_item_id: Option<std::num::NonZeroU32>,
    status: Option<RecipeMaterialStatus>,
    offset: usize,
    limit: usize,
    material_filter: Option<&RecipeMaterialFilter>,
    recipe_view_filter: Option<&RecipeViewFilter>,
) -> Result<Value> {
    search_recipe_materials_with_inventory(
        snapshot,
        recipe_id,
        output_item_id,
        reagent_item_id,
        status,
        offset,
        limit,
        material_filter,
        recipe_view_filter,
        None,
    )
}

pub fn search_recipe_materials_with_inventory(
    snapshot: &Snapshot,
    recipe_id: Option<std::num::NonZeroU32>,
    output_item_id: Option<std::num::NonZeroU32>,
    reagent_item_id: Option<std::num::NonZeroU32>,
    status: Option<RecipeMaterialStatus>,
    offset: usize,
    limit: usize,
    material_filter: Option<&RecipeMaterialFilter>,
    recipe_view_filter: Option<&RecipeViewFilter>,
    inventory: Option<&Snapshot>,
) -> Result<Value> {
    search_recipe_materials_with_cooldowns(
        snapshot,
        recipe_id,
        output_item_id,
        reagent_item_id,
        status,
        offset,
        limit,
        material_filter,
        recipe_view_filter,
        None,
        inventory,
    )
}

pub fn search_recipe_materials_with_cooldowns(
    snapshot: &Snapshot,
    recipe_id: Option<std::num::NonZeroU32>,
    output_item_id: Option<std::num::NonZeroU32>,
    reagent_item_id: Option<std::num::NonZeroU32>,
    status: Option<RecipeMaterialStatus>,
    offset: usize,
    limit: usize,
    material_filter: Option<&RecipeMaterialFilter>,
    recipe_view_filter: Option<&RecipeViewFilter>,
    cooldown_filter: Option<&RecipeCooldownFilter>,
    inventory: Option<&Snapshot>,
) -> Result<Value> {
    search_recipe_materials_ordered(
        snapshot,
        recipe_id,
        output_item_id,
        reagent_item_id,
        status,
        offset,
        limit,
        material_filter,
        recipe_view_filter,
        cooldown_filter,
        inventory,
        RecipeSearchOrder::RecipeId,
        None,
    )
}

pub fn search_recipe_materials_ordered(
    snapshot: &Snapshot,
    recipe_id: Option<std::num::NonZeroU32>,
    output_item_id: Option<std::num::NonZeroU32>,
    reagent_item_id: Option<std::num::NonZeroU32>,
    status: Option<RecipeMaterialStatus>,
    offset: usize,
    limit: usize,
    material_filter: Option<&RecipeMaterialFilter>,
    recipe_view_filter: Option<&RecipeViewFilter>,
    cooldown_filter: Option<&RecipeCooldownFilter>,
    inventory: Option<&Snapshot>,
    order: RecipeSearchOrder,
    minimum_base_output: Option<std::num::NonZeroU32>,
) -> Result<Value> {
    ensure!(
        order != RecipeSearchOrder::ObservedItemCapacityDesc || material_filter.is_some(),
        "Observed item capacity ordering requires material filtering"
    );
    ensure!(
        order != RecipeSearchOrder::ObservedSoleShortfallAsc
            || material_filter.is_some_and(|filter| filter.sole_shortfall_item_id.is_some()),
        "Observed sole shortfall ordering requires a sole shortfall item ID"
    );
    ensure!(
        material_filter.is_none_or(|filter| filter.sole_shortfall_item_id.is_none()
            || filter.insufficient_only == Some(true)),
        "A sole reagent shortfall requires insufficient_only"
    );
    ensure!(
        material_filter.is_none_or(|filter| filter.maximum_sole_shortfall_quantity.is_none()
            || filter.sole_shortfall_item_id.is_some()),
        "A sole shortfall quantity limit requires a sole shortfall item ID"
    );
    ensure!(
        order != RecipeSearchOrder::ReportedBaseOutputPerReagentDesc
            || (output_item_id.is_some() && reagent_item_id.is_some()),
        "Reagent efficiency ordering requires output and reagent item IDs"
    );
    ensure!(
        order != RecipeSearchOrder::ReportedSkillUpsPerReagentDesc || reagent_item_id.is_some(),
        "Skill-up efficiency ordering requires a reagent item ID"
    );
    ensure!(
        snapshot.client.family == crate::Family::Forever,
        "Expected a Forever snapshot"
    );
    ensure!(
        (1..=50).contains(&limit),
        "Recipe search page size out of range"
    );
    if let Some(inventory) = inventory {
        ensure!(
            material_filter.is_some(),
            "Inventory capture requires material filtering"
        );
        ensure_comparable(snapshot, inventory)?;
    }
    let section = snapshot.sections.get("profession_recipe_materials");
    let rows = section.and_then(|value| value["entries"].as_object());
    if let Some(filter) = cooldown_filter {
        ensure!(
            filter.state.is_some()
                || filter.daily.is_some()
                || filter.minimum_charges.is_some()
                || filter.maximum_remaining_seconds.is_some()
                || filter.estimated_end_by.is_some(),
            "Cooldown filter requires at least one predicate"
        );
    }
    let mut cooldown_unknown = 0;
    let mut cooldown_nonmatching = 0;
    if let Some(filter) = recipe_view_filter {
        ensure!(
            filter.learned.is_some()
                || filter.can_skill_up.is_some()
                || filter.minimum_skill_ups_reported.is_some()
                || filter.maximum_skill_ups_reported.is_some()
                || filter.relative_difficulty.is_some(),
            "Recipe view filter requires at least one predicate"
        );
        ensure!(
            filter
                .minimum_skill_ups_reported
                .zip(filter.maximum_skill_ups_reported)
                .is_none_or(|(minimum, maximum)| minimum <= maximum),
            "Recipe skill-up bounds must be ordered"
        );
    }
    let view = snapshot.sections.get("profession_recipe_view");
    let profession_matches = view
        .zip(section)
        .map(|(view, bulk)| view["profession_id"] == bulk["profession_id"]);
    let compatible_view = view.filter(|_| profession_matches == Some(true));
    let mut view_unknown = 0;
    let mut view_nonmatching = 0;
    let stock = material_filter
        .map(|filter| {
            RecipeMaterialStock::from_snapshot(inventory.unwrap_or(snapshot), filter.scope)
        })
        .transpose()?;
    let mut unknown = 0;
    let mut insufficient = 0;
    let insufficient_only =
        material_filter.is_some_and(|filter| filter.insufficient_only == Some(true));
    let status_value = status.map(|value| json!(value));
    let mut matches: Vec<_> = rows
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter(|(id, row)| {
            recipe_id.is_none_or(|value| id.as_str() == value.to_string())
                && output_item_id.is_none_or(|value| {
                    row["output_item_id"].as_u64() == Some(u64::from(value.get()))
                })
                && minimum_base_output.is_none_or(|value| {
                    row["base_output_quantity"]["minimum"]
                        .as_u64()
                        .is_some_and(|minimum| minimum >= u64::from(value.get()))
                })
                && reagent_item_id
                    .is_none_or(|value| row["reagents"].get(value.to_string()).is_some())
                && status_value
                    .as_ref()
                    .is_none_or(|value| row["status"] == *value)
        })
        .filter_map(|(id, row)| {
            if let Some(filter) = cooldown_filter {
                match recipe_cooldown_matches(row, filter, snapshot.captured_at) {
                    Some(true) => {}
                    Some(false) => {
                        cooldown_nonmatching += 1;
                        return None;
                    }
                    None => {
                        cooldown_unknown += 1;
                        return None;
                    }
                }
            }
            let observation = compatible_view.and_then(|view| view["entries"].get(id));
            if let Some(filter) = recipe_view_filter {
                match recipe_view_matches(observation, filter) {
                    Some(true) => {}
                    Some(false) => {
                        view_nonmatching += 1;
                        return None;
                    }
                    None => {
                        view_unknown += 1;
                        return None;
                    }
                }
            }
            let support = if let (Some(filter), Some(stock)) = (material_filter, stock.as_ref()) {
                match stock.support(row, filter.minimum_crafts.get()) {
                    Some(mut support) => {
                        let short = support["covers_minimum_crafts"] == false;
                        insufficient += usize::from(short);
                        if short != insufficient_only {
                            return None;
                        }
                        if let Some(item) = filter.sole_shortfall_item_id {
                            let shortfalls = stock.shortfalls(row, filter.minimum_crafts.get())?;
                            if shortfalls.len() != 1 {
                                return None;
                            }
                            let missing = shortfalls.get(&item.get())?["missing"].as_u64()?;
                            if filter
                                .maximum_sole_shortfall_quantity
                                .is_some_and(|maximum| missing > maximum)
                            {
                                return None;
                            }
                            if order == RecipeSearchOrder::ObservedSoleShortfallAsc {
                                support["sole_shortfall_quantity"] = json!(missing);
                            }
                        }
                        Some(support)
                    }
                    None => {
                        unknown += 1;
                        return None;
                    }
                }
            } else {
                None
            };
            Some((id, row, support, observation))
        })
        .collect();
    if order == RecipeSearchOrder::ObservedItemCapacityDesc {
        matches.sort_unstable_by_key(|(id, _, support, _)| {
            let capacity = support
                .as_ref()
                .and_then(|value| value["supported_crafts"].as_u64());
            (
                capacity.is_none(),
                std::cmp::Reverse(capacity.unwrap_or(0)),
                id.parse::<u32>().unwrap(),
            )
        });
    } else if order == RecipeSearchOrder::ObservedSoleShortfallAsc {
        matches.sort_unstable_by_key(|(id, _, support, _)| {
            (
                support
                    .as_ref()
                    .and_then(|row| row["sole_shortfall_quantity"].as_u64()),
                id.parse::<u32>().unwrap(),
            )
        });
    } else if order == RecipeSearchOrder::ReportedSkillUpsDesc {
        matches.sort_unstable_by_key(|(id, _, _, observation)| {
            let reported = observation.and_then(|row| row["skill_ups_reported"].as_u64());
            (
                reported.is_none(),
                std::cmp::Reverse(reported.unwrap_or(0)),
                id.parse::<u32>().unwrap(),
            )
        });
    } else if order == RecipeSearchOrder::ReportedCooldownRemainingAsc {
        matches.sort_unstable_by_key(|(id, row, _, _)| {
            let remaining = row["cooldown"]["remaining_seconds"].as_u64();
            (
                remaining.is_none(),
                remaining.unwrap_or(0),
                id.parse::<u32>().unwrap(),
            )
        });
    } else if matches!(
        order,
        RecipeSearchOrder::ReportedBaseOutputPerReagentDesc
            | RecipeSearchOrder::ReportedSkillUpsPerReagentDesc
    ) {
        let reagent_key = reagent_item_id.unwrap().to_string();
        let ratio = |row: &Value, observation: Option<&Value>| {
            let reported = if order == RecipeSearchOrder::ReportedSkillUpsPerReagentDesc {
                observation.and_then(|view| view["skill_ups_reported"].as_u64())
            } else {
                row["base_output_quantity"]["minimum"].as_u64()
            };
            reported.zip(row["reagents"][&reagent_key].as_u64())
        };
        matches.sort_unstable_by(
            |(left_id, left, _, left_view), (right_id, right, _, right_view)| {
                let order = match (ratio(left, *left_view), ratio(right, *right_view)) {
                    (Some((left_output, left_cost)), Some((right_output, right_cost))) => {
                        (right_output * left_cost).cmp(&(left_output * right_cost))
                    }
                    (left, right) => right.is_some().cmp(&left.is_some()),
                };
                order.then_with(|| {
                    left_id
                        .parse::<u32>()
                        .unwrap()
                        .cmp(&right_id.parse::<u32>().unwrap())
                })
            },
        );
    } else if order == RecipeSearchOrder::ReportedBaseOutputMinimumDesc {
        matches.sort_unstable_by_key(|(id, row, _, _)| {
            let minimum = row["base_output_quantity"]["minimum"].as_u64();
            (std::cmp::Reverse(minimum), id.parse::<u32>().unwrap())
        });
    } else {
        matches.sort_unstable_by_key(|(id, _, _, _)| id.parse::<u32>().unwrap());
    }
    let records: Vec<_> = matches
        .iter()
        .skip(offset)
        .take(limit)
        .map(|(id, row, support, observation)| {
            let mut record = (*row).clone();
            record["recipe_id"] = json!(id.parse::<u32>().unwrap());
            if cooldown_filter.is_some() || order == RecipeSearchOrder::ReportedCooldownRemainingAsc {
                record["captured_cooldown_state"] = json!(recipe_cooldown_state(row));
                record["estimated_cooldown_end_at"] =
                    json!(estimated_recipe_cooldown_end(row, snapshot.captured_at));
            }
            if let Some(support) = support {
                record["material_support"] = support.clone();
            }
            if insufficient_only && let Some((stock, filter)) = stock.as_ref().zip(material_filter) {
                record["observed_reagent_shortfalls"] = json!(stock.shortfalls(row, filter.minimum_crafts.get()));
            }
            if let Some(observation) = observation.filter(|_| recipe_view_filter.is_some() || matches!(order,
                RecipeSearchOrder::ReportedSkillUpsDesc | RecipeSearchOrder::ReportedSkillUpsPerReagentDesc)) {
                record["recipe_view_observation"] = json!({"source":"profession_recipe_view",
                    "learned":observation.get("learned"),"can_skill_up":observation.get("can_skill_up"),
                    "skill_ups_reported":observation.get("skill_ups_reported"),
                    "relative_difficulty":observation.get("relative_difficulty")});
            }
            record
        })
        .collect();
    let next = if offset < matches.len() && records.len() < matches.len() - offset {
        Some(offset + records.len())
    } else {
        None
    };
    let mut result = json!({"section_observed":section.is_some(),"coverage":section.map(|s|&s["coverage"]),
        "profession_id":section.map(|s|&s["profession_id"]),
        "captured_spell_count":rows.map(|rows|rows.len()),
        "matched_row_count":rows.map(|_|matches.len()),"offset":offset,"limit":limit,
        "records":records,"next_offset":next,
        "filters":{"recipe_id":recipe_id,"output_item_id":output_item_id,"reagent_item_id":reagent_item_id,"status":status},
        "complete_recipe_knowledge_verified":false,"craftability_verified":false,"live":false,"game_execution":false});
    if let Some(minimum) = minimum_base_output {
        result["filters"]["minimum_base_output"] = json!(minimum);
    }
    if let Some(rank) = compatible_view.and_then(|view| view.get("rank_observation")) {
        result["recipe_view_rank_observation"] = rank.clone();
    }
    if order != RecipeSearchOrder::RecipeId {
        result["order"] = json!(order);
        result["ordering_basis"] = json!(match order {
            RecipeSearchOrder::ObservedItemCapacityDesc =>
                "independent_observed_required_item_capacity_only",
            RecipeSearchOrder::ObservedSoleShortfallAsc =>
                "independent_observed_sole_reagent_shortfall_quantity",
            RecipeSearchOrder::ReportedCooldownRemainingAsc =>
                "captured_cooldown_remaining_seconds_only",
            RecipeSearchOrder::ReportedSkillUpsDesc => "captured_same_profession_skill_ups_only",
            RecipeSearchOrder::ReportedSkillUpsPerReagentDesc =>
                "captured_same_profession_skill_ups_per_selected_required_reagent",
            RecipeSearchOrder::ReportedBaseOutputMinimumDesc =>
                "captured_base_output_minimum_per_craft_only",
            RecipeSearchOrder::ReportedBaseOutputPerReagentDesc =>
                "captured_base_output_minimum_per_selected_required_reagent",
            RecipeSearchOrder::RecipeId => "recipe_id",
        });
        result["shared_material_allocation_performed"] = json!(false);
        result["skill_gain_prediction_performed"] = json!(false);
    }
    if let Some(inventory) = inventory {
        result["recipe_captured_at"] = json!(snapshot.captured_at);
        result["inventory_captured_at"] = json!(inventory.captured_at);
    }
    if let Some(filter) = cooldown_filter {
        result["filters"]["cooldown_filter"] = json!(filter);
        result["cooldown_unknown_count"] = json!(rows.map(|_| cooldown_unknown));
        result["cooldown_nonmatching_count"] = json!(rows.map(|_| cooldown_nonmatching));
    }
    if cooldown_filter.is_some() || order == RecipeSearchOrder::ReportedCooldownRemainingAsc {
        result["cooldown_captured_at"] = json!(snapshot.captured_at);
    }
    if let Some(filter) = material_filter {
        let stock = stock.as_ref().unwrap();
        result["filters"]["material_filter"] = json!(filter);
        result["material_support_unknown_count"] = json!(rows.map(|_| unknown));
        result["material_support_insufficient_observed_count"] = json!(rows.map(|_| insufficient));
        result["material_inventory_coverage"] = json!({"observed_storage":stock.observed,
            "unobserved_storage":stock.missing,"complete_ownership_verified":false});
    }
    if let Some(filter) = recipe_view_filter {
        result["filters"]["recipe_view_filter"] = json!(filter);
        result["recipe_view_unknown_count"] = json!(rows.map(|_| view_unknown));
        result["recipe_view_nonmatching_count"] = json!(rows.map(|_| view_nonmatching));
        result["recipe_view_coverage"] = json!({"section_observed":view.is_some(),
            "profession_matches_bulk":profession_matches,"coverage":view.map(|view|&view["coverage"]),
            "learning_verified":false,"skill_gains_verified":false});
    }
    Ok(result)
}

/// Explicit IDs use the bulk schematic, with compatible same-capture selected conditions.
pub fn plan_recipe_by_id(
    snapshot: &Snapshot,
    recipe_id: Option<std::num::NonZeroU32>,
    crafts: Option<u32>,
    target_output_quantity: Option<std::num::NonZeroU32>,
    inventory: Option<&Snapshot>,
) -> Result<Value> {
    let Some(id) = recipe_id else {
        return plan_recipe(snapshot, crafts, target_output_quantity, inventory);
    };
    ensure!(
        snapshot.client.family == crate::Family::Forever,
        "Expected a Forever snapshot"
    );
    let section = snapshot
        .sections
        .get("profession_recipe_materials")
        .ok_or_else(|| anyhow::anyhow!("Bulk recipe materials were not captured"))?;
    let row = section["entries"]
        .get(id.to_string())
        .ok_or_else(|| anyhow::anyhow!("Requested bulk recipe was not captured"))?;
    let fixed = row["status"] == "fixed_items";
    ensure!(
        fixed || row["status"] == "unsupported_required_slots",
        "Requested bulk recipe materials unavailable"
    );
    let requirements = if fixed {
        "fixed_items"
    } else {
        "unsupported_required_slots"
    };
    let mut recipe = json!({"recipe_id":id.get(),"learned":true,"requirements":requirements,
        "reagents":if fixed {row["reagents"].clone()} else {json!({})}});
    for field in ["output_item_id", "base_output_quantity", "cooldown"] {
        if let Some(value) = row.get(field) {
            recipe[field] = value.clone();
        }
    }
    if let Some(selected) = snapshot
        .sections
        .get("selected_recipe")
        .filter(|recipe| recipe["recipe_id"].as_u64() == Some(u64::from(id.get())))
    {
        ensure!(
            selected["learned"] == true && selected["requirements"] == requirements,
            "Conflicting captured recipe observations"
        );
        if fixed {
            ensure!(
                selected["reagents"] == recipe["reagents"],
                "Conflicting captured recipe materials"
            );
        }
        for field in ["output_item_id", "base_output_quantity"] {
            if let (Some(a), Some(b)) = (
                selected.get(field).filter(|v| !v.is_null()),
                recipe.get(field),
            ) {
                ensure!(a == b, "Conflicting captured recipe outputs");
            }
        }
        if let Some(profession) = selected.get("profession").filter(|v| !v.is_null()) {
            ensure!(
                profession["profession_id"] == section["profession_id"],
                "Conflicting captured recipe profession"
            );
        }
        if let Some(conditions) = selected.get("crafting_conditions") {
            recipe["crafting_conditions"] = conditions.clone();
        }
    }
    if let Some(view) = snapshot.sections.get("profession_recipe_view") {
        ensure!(
            view["entries"][id.to_string()]["learned"] != false,
            "Conflicting captured recipe learning"
        );
    }
    let tracking = recipe_tracking(snapshot, u64::from(id.get()));
    ensure!(
        tracking["tracked_craft_learned"] != false && tracking["tracked_recraft_learned"] != false,
        "Conflicting captured recipe learning"
    );
    let count = match (crafts, target_output_quantity) {
        (Some(count), None) => count,
        (None, Some(target)) => {
            let minimum = recipe["base_output_quantity"]["minimum"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .filter(|n| *n > 0)
                .ok_or_else(|| anyhow::anyhow!("Recipe base output range unavailable"))?;
            target.get().div_ceil(minimum)
        }
        _ => anyhow::bail!("Supply one craft count or target output quantity"),
    };
    let mut review = review_recipe_observation(snapshot, &recipe, count, inventory)?;
    review["recipe_observation_source"] = json!("profession_recipe_materials");
    review["recipe_profession_id"] = section["profession_id"].clone();
    review["recipe_material_status"] = row["status"].clone();
    if let Some(reported) = row.get("reported_craftable_count").and_then(Value::as_u64) {
        review["reported_craftable_count_observation"] = json!({"count":reported,
            "captured_at":snapshot.captured_at,"basis":"recipe_capture_public_api_default_level",
            "shortfall_vs_requested_crafts":u64::from(count).saturating_sub(reported),
            "current_craftability_verified":false});
    }
    if let Some(target) = target_output_quantity {
        review["target_output_quantity"] = json!(target.get());
    }
    Ok(review)
}

pub fn review_recipe_with_inventory(
    snapshot: &Snapshot,
    crafts: u32,
    inventory: Option<&Snapshot>,
) -> Result<Value> {
    ensure!(
        snapshot.client.family == crate::Family::Forever,
        "Expected a Forever snapshot"
    );
    ensure!((1..=1000).contains(&crafts), "Craft count out of range");
    let recipe = snapshot
        .sections
        .get("selected_recipe")
        .ok_or_else(|| anyhow::anyhow!("Selected recipe was not captured"))?;
    review_recipe_observation(snapshot, recipe, crafts, inventory)
}

/// Draft a bounded producer chain; simulated acquisition and output are not owned stock.
pub fn recipe_reagent_chain(
    snapshot: &Snapshot,
    recipe_id: std::num::NonZeroU32,
    crafts: u32,
    inventory: Option<&Snapshot>,
) -> Result<Value> {
    recipe_reagent_chain_with_choices(snapshot, recipe_id, crafts, inventory, &[])
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecipeProducerChoice {
    pub item_id: std::num::NonZeroU32,
    pub recipe_id: std::num::NonZeroU32,
}

pub fn recipe_reagent_chain_with_choices(
    snapshot: &Snapshot,
    recipe_id: std::num::NonZeroU32,
    crafts: u32,
    inventory: Option<&Snapshot>,
    choices: &[RecipeProducerChoice],
) -> Result<Value> {
    ensure!(choices.len() <= 8, "Too many reagent producer choices");
    ensure!(
        snapshot.client.family == crate::Family::Forever && (1..=1000).contains(&crafts),
        "Invalid recipe chain request"
    );
    if let Some(stock) = inventory {
        ensure_comparable(snapshot, stock)?;
    }
    let stock = RecipeMaterialStock::from_snapshot(
        inventory.unwrap_or(snapshot),
        RecipeMaterialScope::AllObservedStorage,
    )?;
    let section = snapshot
        .sections
        .get("profession_recipe_materials")
        .ok_or_else(|| anyhow::anyhow!("Bulk recipes unavailable"))?;
    let mut producers = BTreeMap::<u64, Vec<std::num::NonZeroU32>>::new();
    for (id, row) in section["entries"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Bulk recipes unavailable"))?
    {
        if let Some(output) = fixed_recipe_output(row) {
            producers.entry(output).or_default().push(
                std::num::NonZeroU32::new(id.parse()?)
                    .ok_or_else(|| anyhow::anyhow!("Invalid recipe ID"))?,
            );
        }
    }
    for choices in producers.values_mut() {
        choices.sort_unstable();
    }
    let mut selected = BTreeMap::new();
    for choice in choices {
        let item = u64::from(choice.item_id.get());
        ensure!(
            selected.insert(item, choice.recipe_id).is_none(),
            "Duplicate reagent producer choice"
        );
        ensure!(
            producers
                .get(&item)
                .is_some_and(|candidates| candidates.contains(&choice.recipe_id)),
            "Producer choice must match a captured fixed recipe output"
        );
    }
    let mut chain = RecipeChainDraft {
        snapshot,
        inventory,
        producers,
        choices: selected,
        used_choices: BTreeSet::new(),
        balance: stock.quantities,
        inventory_observed: !stock.observed.is_empty(),
        active: BTreeSet::new(),
        steps: Vec::new(),
        external: BTreeMap::new(),
        relevant: BTreeSet::new(),
        charges: RecipeChargeLedger::default(),
    };
    let blocked = chain.craft(recipe_id, crafts).err();
    let complete = blocked.is_none();
    let external: Vec<_> = chain
        .external
        .iter()
        .map(|(id, quantity)| {
            json!({"item_id":id,"quantity":quantity,
        "basis":"no_captured_fixed_producer"})
        })
        .collect();
    let remaining: Vec<_> = chain
        .relevant
        .iter()
        .map(|id| {
            json!({"item_id":id,
        "quantity":chain.balance.get(id).copied().unwrap_or(0)})
        })
        .collect();
    let mut result = json!({"basis":"captured_fixed_recipes_observed_stock_and_reported_minimum_yields",
        "root_recipe_id":recipe_id,"root_crafts":crafts,"maximum_steps":8,
        "status":if complete {"draft_complete"} else {"blocked"},"draft_complete":complete,"blocked_reason":blocked,
        "steps":chain.steps,"external_material_requirements":complete.then_some(external),
        "requires_external_materials":complete.then_some(!chain.external.is_empty()),
        "modeled_remaining_items_if_completed":complete.then_some(remaining),
        "observed_storage":stock.observed,"unobserved_storage":stock.missing,
        "recipe_captured_at":snapshot.captured_at,"inventory_captured_at":inventory.unwrap_or(snapshot).captured_at,
        "complete_ownership_verified":false,"minimum_yields_verified":false,"optimisation_verified":false,
        "craftability_verified":false,"live":false,"game_execution":false});
    if !choices.is_empty() {
        result["requested_producer_choices"] = json!(choices);
        result["used_producer_choices"] = json!(
            chain
                .used_choices
                .iter()
                .map(|item| json!({
            "item_id":item,"recipe_id":chain.choices[item]}))
                .collect::<Vec<_>>()
        );
    }
    Ok(result)
}

fn fixed_recipe_output(row: &Value) -> Option<u64> {
    (row["status"] == "fixed_items")
        .then(|| row["output_item_id"].as_u64())
        .flatten()
}

/// Replan the first unresolved producer choice with a bounded set of explicit alternatives.
pub fn preview_reagent_producers(
    chain: &Value,
    snapshot: &Snapshot,
    recipe_id: std::num::NonZeroU32,
    crafts: u32,
    inventory: Option<&Snapshot>,
    choices: &[RecipeProducerChoice],
    include_stock_plan: bool,
    offset: usize,
) -> Result<Value> {
    let blocked = &chain["blocked_reason"];
    if blocked["reason"] != "ambiguous_captured_producer" {
        return Ok(Value::Null);
    }
    let item_id: std::num::NonZeroU32 = serde_json::from_value(blocked["item_id"].clone())?;
    let mut ids: Vec<std::num::NonZeroU32> =
        snapshot.sections["profession_recipe_materials"]["entries"]
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(_, row)| fixed_recipe_output(row) == Some(u64::from(item_id.get())))
            .map(|(id, _)| id.parse())
            .collect::<std::result::Result<_, _>>()?;
    ids.sort_unstable();
    let total = ids.len();
    let mut previews = Vec::new();
    for candidate in ids.into_iter().skip(offset).take(4) {
        let mut selections = choices.to_vec();
        selections.push(RecipeProducerChoice {
            item_id,
            recipe_id: candidate,
        });
        let preview =
            recipe_reagent_chain_with_choices(snapshot, recipe_id, crafts, inventory, &selections)?;
        let reason = &preview["blocked_reason"];
        let steps: Vec<_> = preview["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|step| json!({"recipe_id":step["recipe_id"],"crafts":step["crafts"]}))
            .collect();
        let mut result = json!({"recipe_id":candidate,"draft_complete":preview["draft_complete"],
            "blocked_reason":reason["reason"],"blocked_item_id":reason["item_id"],
            "blocked_recipe_id":reason["recipe_id"],"steps":steps,
            "external_material_requirements":preview["external_material_requirements"]});
        if include_stock_plan {
            let stock = reagent_chain_stock_plan(&preview, snapshot, inventory, 0)?;
            result["stock_plan_summary"] = if stock.is_null() {
                Value::Null
            } else {
                json!({
                    "material_coverage":stock["material_coverage"],
                    "captured_chain_checks_pass":stock["captured_chain_checks_pass"],
                    "acquisition_shortfalls_vs_observed_storage":stock["acquisition_shortfalls_vs_observed_storage"],
                    "suggested_bank_withdrawal_list":stock["suggested_bank_withdrawal_list"],
                })
            };
        }
        previews.push(result);
    }
    Ok(json!({"item_id":item_id,"truncated":total > previews.len(),
        "offset":offset,"total_candidates":total,"has_more":total.saturating_sub(offset) > previews.len(),
        "candidate_previews":previews,"craftability_verified":false}))
}

struct RecipeChainDraft<'a> {
    snapshot: &'a Snapshot,
    inventory: Option<&'a Snapshot>,
    producers: BTreeMap<u64, Vec<std::num::NonZeroU32>>,
    choices: BTreeMap<u64, std::num::NonZeroU32>,
    used_choices: BTreeSet<u64>,
    balance: BTreeMap<u64, u64>,
    inventory_observed: bool,
    active: BTreeSet<u32>,
    steps: Vec<Value>,
    external: BTreeMap<u64, u64>,
    relevant: BTreeSet<u64>,
    charges: RecipeChargeLedger,
}

/// Per-recipe historical charge accounting; all callers have at most eight
/// validated 1..=1000-craft steps from the same recipe capture. No recharge or
/// relationship between different recipes' cooldown pools is inferred.
#[derive(Default)]
struct RecipeChargeLedger {
    requested: BTreeMap<u64, u64>,
}

impl RecipeChargeLedger {
    fn consume(&mut self, review: &Value) -> Value {
        let recipe = &review["recipe"];
        let id = recipe["recipe_id"].as_u64().unwrap();
        let requested = self.requested.entry(id).or_default();
        let before = *requested;
        let step_crafts = review["requested_crafts"].as_u64().unwrap();
        *requested += step_crafts;
        let charges = recipe["cooldown"]["charges"].as_u64();
        let before_step = charges.map(|n| n.saturating_sub(before));
        let covered_crafts = before_step.map(|n| n.min(step_crafts));
        json!({"requested_crafts_through_step":*requested,
            "observed_charges":charges,
            "observed_charges_before_step":before_step,
            "step_crafts_covered_by_observed_charges":covered_crafts,
            "base_output_for_charge_covered_crafts_if_completed":covered_crafts
                .and_then(|count| recipe_base_output(recipe, count)),
            "observed_charges_remaining_if_completed":charges.map(|n| n.saturating_sub(*requested)),
            "shortfall_vs_cumulative_crafts":charges.map(|n| requested.saturating_sub(n)),
            "covers_cumulative_crafts":charges.map(|n| n >= *requested)})
    }
}

fn recipe_base_output(recipe: &Value, count: u64) -> Option<Value> {
    recipe["base_output_quantity"]["minimum"]
        .as_u64()
        .zip(recipe["base_output_quantity"]["maximum"].as_u64())
        .map(|(minimum, maximum)| {
            json!({"item_id":recipe["output_item_id"],"minimum":minimum * count,
                "maximum":maximum * count})
        })
}

/// Reuse ordered-batch accounting for the exact printed chain steps, without external acquisitions.
pub fn reagent_chain_stock_plan(
    chain: &Value,
    snapshot: &Snapshot,
    inventory: Option<&Snapshot>,
    withdrawal_offset: usize,
) -> Result<Value> {
    if chain["draft_complete"] != true {
        return Ok(Value::Null);
    }
    let steps = chain["steps"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing chain steps"))?;
    ensure!((1..=8).contains(&steps.len()), "Invalid chain step count");
    let mut reviews = Vec::new();
    for step in steps {
        let id = step["recipe_id"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .and_then(std::num::NonZeroU32::new)
            .ok_or_else(|| anyhow::anyhow!("Invalid chain recipe ID"))?;
        let crafts = step["crafts"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid chain craft count"))?;
        let review = plan_recipe_by_id(snapshot, Some(id), Some(crafts), None, inventory)?;
        let required: Vec<_> = review["materials"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| json!({"item_id":row["item_id"],"quantity":row["required"]}))
            .collect();
        ensure!(
            review["recipe"]["requirements"] == "fixed_items"
                && review["base_output_if_completed"]["item_id"] == step["output_item_id"]
                && review["base_output_if_completed"]["minimum"]
                    == step["minimum_output_if_completed"]
                && json!(required) == step["required_reagents"],
            "Chain step changed"
        );
        reviews.push(review);
    }
    let combined = combine_recipe_materials(reviews.iter())?;
    ensure!(combined.is_array(), "Unsupported chain materials");
    let (materials, checkpoint) = ordered_recipe_materials(reviews.iter(), &combined)?;
    let material_coverage = ordered_recipe_material_coverage(&materials);
    let acquisition_shortfalls: Vec<_> = materials
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let quantity = row["shortfall_vs_observed"].as_u64()?;
            (quantity > 0).then(|| json!({"item_id":row["item_id"],"quantity":quantity}))
        })
        .collect();
    let bank_withdrawals: Vec<_> = materials
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let shortfall = row["shortfall_vs_bags"].as_u64()?;
            let bank = &row["suggested_bank_withdrawals"];
            let character = bank["character_bank"].as_u64().unwrap_or(0);
            let account = bank["account_bank"].as_u64().unwrap_or(0);
            (character > 0 || account > 0).then(|| json!({
                "item_id":row["item_id"],"character_bank":character,"account_bank":account,
                "remaining_bag_shortfall_after_suggested_withdrawals":shortfall - character - account,
            }))
        })
        .collect();
    let bank_locations = bank_withdrawal_locations(
        inventory.unwrap_or(snapshot),
        &bank_withdrawals,
        withdrawal_offset,
    );
    let nonmaterial = ordered_recipe_requirements_with_charges(reviews.iter(), true);
    let checkpoints = [
        ("materials", &checkpoint["first_uncovered_recipe"]),
        ("nonmaterial", &nonmaterial["first_observed_shortfall"]),
    ];
    let first_review_step = checkpoints
        .iter()
        .filter_map(|(_, row)| row["recipe_number"].as_u64())
        .min()
        .map(|number| {
            let reasons: BTreeMap<_, _> = checkpoints
                .into_iter()
                .filter(|(_, row)| row["recipe_number"].as_u64() == Some(number))
                .collect();
            json!({"recipe_number":number,"checkpoints":reasons})
        });
    let captured_checks_pass = |storage: &str| {
        let checks = [
            material_coverage[storage].as_bool(),
            nonmaterial["observed_charges_cover_chain"].as_bool(),
            nonmaterial["observed_chain_conditions"]["all_met"].as_bool(),
        ];
        if checks.contains(&Some(false)) {
            Some(false)
        } else {
            checks.iter().all(Option::is_some).then_some(true)
        }
    };
    Ok(
        json!({"basis":"minimum_starting_items_for_printed_chain_order_and_reported_minimum_yields",
        "material_coverage":material_coverage,
        "captured_chain_checks_pass":{"bags":captured_checks_pass("bags"),
            "all_observed_storage":captured_checks_pass("all_observed_storage")},
        "starting_materials":materials,"material_checkpoint":checkpoint,
        "suggested_bank_withdrawal_list":bank_withdrawals,
        "suggested_bank_withdrawal_locations":bank_locations,
        "acquisition_shortfalls_vs_observed_storage":acquisition_shortfalls,
        "first_step_to_review":first_review_step,
        "nonmaterial_requirements":nonmaterial,
        "recipe_captured_at":snapshot.captured_at,"inventory_captured_at":inventory.unwrap_or(snapshot).captured_at,
        "withdrawal_eligibility_verified":false,"current_stock_verified":false,"minimum_yields_verified":false,
        "craftability_verified":false,"live":false,"game_execution":false}),
    )
}

pub(crate) fn bank_withdrawal_locations(
    stock: &Snapshot,
    withdrawals: &[Value],
    offset: usize,
) -> Value {
    let mut locations = Vec::new();
    let mut total_locations = 0;
    for storage in ["character_bank", "account_bank"] {
        let mut remaining: BTreeMap<_, _> = withdrawals
            .iter()
            .filter_map(|row| {
                let quantity = row[storage].as_u64()?;
                (quantity > 0).then(|| (row["item_id"].as_u64().unwrap(), quantity))
            })
            .collect();
        if remaining.is_empty() {
            continue;
        }
        let Some(containers) = stock.sections.get(storage).and_then(Value::as_object) else {
            continue;
        };
        let mut candidates = Vec::new();
        for (container, slots) in containers {
            for (slot, item) in slots.as_object().into_iter().flatten() {
                if let Some(id) = item["item_id"].as_u64()
                    && remaining.contains_key(&id)
                {
                    candidates.push((
                        std::cmp::Reverse(item["count"].as_u64().unwrap()),
                        container.parse::<u32>().unwrap(),
                        slot.parse::<u32>().unwrap(),
                        id,
                    ));
                }
            }
        }
        candidates.sort_unstable();
        for (std::cmp::Reverse(count), container, slot, id) in candidates {
            let quantity = remaining.get_mut(&id).unwrap();
            let take = count.min(*quantity);
            if take == 0 {
                continue;
            }
            *quantity -= take;
            if total_locations >= offset && locations.len() < 50 {
                locations.push(json!({"storage":storage,"container":container.to_string(),
                    "slot":slot.to_string(),"item_id":id,"observed_quantity":count,"suggested_quantity":take}));
            }
            total_locations += 1;
        }
    }
    json!({"locations":locations,"total_locations":total_locations,
        "offset":offset,"has_more":total_locations.saturating_sub(offset) > locations.len(),
        "truncated":total_locations > locations.len()})
}

impl RecipeChainDraft<'_> {
    fn craft(&mut self, id: std::num::NonZeroU32, crafts: u32) -> std::result::Result<(), Value> {
        if self.active.contains(&id.get()) {
            return Err(json!({"reason":"cyclic_uncovered_demand","recipe_id":id}));
        }
        if self.steps.len() + self.active.len() >= 8 {
            return Err(json!({"reason":"step_budget","recipe_id":id}));
        }
        if !(1..=1000).contains(&crafts) {
            return Err(json!({"reason":"craft_count_budget","recipe_id":id}));
        }
        let review = plan_recipe_by_id(self.snapshot, Some(id), Some(crafts), None, self.inventory)
            .map_err(
                |_| json!({"reason":"recipe_observation_conflict_or_unavailable","recipe_id":id}),
            )?;
        if review["recipe"]["requirements"] != "fixed_items" {
            return Err(json!({"reason":"unsupported_materials","recipe_id":id}));
        }
        let output = review["base_output_if_completed"]["item_id"].as_u64();
        let quantity = review["base_output_if_completed"]["minimum"]
            .as_u64()
            .filter(|n| *n > 0);
        let (Some(output), Some(quantity)) = (output, quantity) else {
            return Err(json!({"reason":"output_or_minimum_yield_unavailable","recipe_id":id}));
        };
        self.active.insert(id.get());
        self.relevant.insert(output);
        let mut required = Vec::new();
        for material in review["materials"].as_array().unwrap() {
            let item = material["item_id"].as_u64().unwrap();
            let demand = material["required"].as_u64().unwrap();
            self.relevant.insert(item);
            if !self.inventory_observed {
                return Err(json!({"reason":"inventory_unobserved","item_id":item}));
            }
            let available = self.balance.get(&item).copied().unwrap_or(0);
            if available < demand {
                let deficit = demand - available;
                let choices = self.producers.get(&item).map(Vec::as_slice).unwrap_or(&[]);
                let explicit = self.choices.get(&item).copied();
                if explicit.is_none() && choices.len() > 1 {
                    let rows = &self.snapshot.sections["profession_recipe_materials"]["entries"];
                    let estimates: Vec<_> = choices
                        .iter()
                        .take(20)
                        .map(|id| {
                            let row = &rows[id.to_string()];
                            let minimum = row["base_output_quantity"]["minimum"]
                                .as_u64()
                                .filter(|n| *n > 0);
                            let crafts =
                                minimum.map(|yield_per_craft| deficit.div_ceil(yield_per_craft));
                            let charges = row["cooldown"]["charges"].as_u64();
                            let direct_reagents =
                                crafts.filter(|count| *count <= 1000).and_then(|count| {
                                    row["reagents"].as_object().map(|reagents| {
                                        reagents
                                            .iter()
                                            .map(|(item_id, quantity)| {
                                                (item_id, quantity.as_u64().unwrap() * count)
                                            })
                                            .collect::<BTreeMap<_, _>>()
                                    })
                                });
                            let direct_shortfalls = direct_reagents.as_ref().map(|reagents| {
                                reagents
                                    .iter()
                                    .filter_map(|(item_id, required)| {
                                        let available = self
                                            .balance
                                            .get(&item_id.parse::<u64>().unwrap())
                                            .copied()
                                            .unwrap_or_default();
                                        let shortfall = required.saturating_sub(available);
                                        (shortfall > 0).then_some((*item_id, shortfall))
                                    })
                                    .collect::<BTreeMap<_, _>>()
                            });
                            json!({"recipe_id":id,"reported_minimum_output_per_craft":minimum,
                            "already_active_in_chain":self.active.contains(&id.get()),
                            "crafts_for_current_deficit":crafts,
                            "fits_step_craft_limit":crafts.map(|count| count <= 1000),
                            "direct_reagents_for_current_deficit":direct_reagents,
                            "direct_reagent_shortfalls_vs_chain_balance":direct_shortfalls,
                            "captured_charges":charges,
                            "captured_charges_cover_current_deficit":crafts.zip(charges)
                                .map(|(count, available)| count <= available)})
                        })
                        .collect();
                    return Err(
                        json!({"reason":"ambiguous_captured_producer","item_id":item,
                    "current_deficit":deficit,"candidate_estimates":estimates,
                    "candidate_recipe_ids":choices.iter().take(20).collect::<Vec<_>>(),"candidate_count":choices.len(),
                    "candidates_truncated":choices.len()>20}),
                    );
                }
                if let Some(producer) = explicit.or_else(|| choices.first().copied()) {
                    if explicit.is_some() {
                        self.used_choices.insert(item);
                    }
                    let row = &self.snapshot.sections["profession_recipe_materials"]["entries"]
                        [producer.to_string()];
                    let minimum = row["base_output_quantity"]["minimum"].as_u64().filter(|n|*n > 0)
                        .ok_or_else(||json!({"reason":"output_or_minimum_yield_unavailable","recipe_id":producer}))?;
                    let count = u32::try_from(deficit.div_ceil(minimum))
                        .ok()
                        .filter(|n| *n <= 1000)
                        .ok_or_else(
                            || json!({"reason":"craft_count_budget","recipe_id":producer}),
                        )?;
                    self.craft(producer, count)?;
                } else {
                    let total = self.external.entry(item).or_default();
                    *total = total
                        .checked_add(deficit)
                        .ok_or_else(|| json!({"reason":"quantity_overflow"}))?;
                    self.balance.insert(item, demand);
                }
            }
            let amount = self
                .balance
                .get(&item)
                .copied()
                .unwrap_or(0)
                .checked_sub(demand)
                .ok_or_else(|| json!({"reason":"producer_did_not_cover_demand","item_id":item}))?;
            self.balance.insert(item, amount);
            required.push(json!({"item_id":item,"quantity":demand}));
        }
        let amount = self
            .balance
            .get(&output)
            .copied()
            .unwrap_or(0)
            .checked_add(quantity)
            .ok_or_else(|| json!({"reason":"quantity_overflow"}))?;
        self.balance.insert(output, amount);
        let charges = self.charges.consume(&review);
        self.steps.push(json!({"recipe_id":id,"crafts":crafts,"output_item_id":output,
            "minimum_output_if_completed":quantity,"required_reagents":required,
            "captured_charge_accounting":charges,
            "unmet_crafting_conditions":review["unmet_crafting_conditions"],
            "reported_craftable_count_observation":review["reported_craftable_count_observation"],
            "recipe_tracking":review["recipe_tracking"],
            "captured_cooldown_state":review["captured_cooldown_state"],
            "estimated_cooldown_end_at":review["estimated_cooldown_end_at"],
            "captured_nonmaterial_checks":{"recipe_learned":review["captured_preparation"]["recipe_learned"],
                "observed_conditions_met":review["captured_preparation"]["observed_conditions_met"],
                "observed_charges_cover_cumulative_crafts":charges["covers_cumulative_crafts"],
                "observed_charges_cover_request":review["captured_preparation"]["observed_charges_cover_request"]}}));
        self.active.remove(&id.get());
        Ok(())
    }
}

fn review_recipe_observation(
    snapshot: &Snapshot,
    recipe: &Value,
    crafts: u32,
    inventory: Option<&Snapshot>,
) -> Result<Value> {
    ensure!(
        snapshot.client.family == crate::Family::Forever,
        "Expected a Forever snapshot"
    );
    ensure!((1..=1000).contains(&crafts), "Craft count out of range");
    if let Some(inventory) = inventory {
        ensure_comparable(snapshot, inventory)?;
    }
    let stock = inventory.unwrap_or(snapshot);
    let fixed = recipe["requirements"] == "fixed_items";
    let storage = ["bags", "character_bank", "account_bank"];
    let mut observed: BTreeMap<&str, ObservedReagent> = BTreeMap::new();
    let mut missing_storage = Vec::new();
    for (index, name) in storage.into_iter().enumerate() {
        let Some(containers) = stock.sections.get(name).and_then(Value::as_object) else {
            missing_storage.push(name);
            continue;
        };
        for (container, items) in containers {
            let Some(slots) = items.as_object() else {
                continue;
            };
            for (slot, item) in slots {
                if let (Some(id), Some(count)) = (item["item_id"].as_u64(), item["count"].as_u64())
                    && let Some((key, _)) = recipe["reagents"]
                        .as_object()
                        .and_then(|map| map.get_key_value(&id.to_string()))
                {
                    let reagent = observed.entry(key).or_default();
                    reagent.quantities[index] += count;
                    reagent.stack_count += 1;
                    if reagent.locations.len() < 20 {
                        reagent.locations.push(json!({
                            "storage":name,"container":container,"slot":slot,"count":count
                        }));
                    }
                }
            }
        }
    }
    let mut materials = Vec::new();
    let mut bag_limits = Vec::new();
    let mut observed_limits = Vec::new();
    let mut capacity_by_item = BTreeMap::new();
    if fixed {
        for (id, quantity) in recipe["reagents"]
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid reagent data"))?
        {
            let per_craft = quantity
                .as_u64()
                .filter(|quantity| *quantity > 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid reagent quantity"))?;
            let item_id = id.parse::<u32>()?;
            let required = per_craft * u64::from(crafts);
            let reagent = observed.remove(id.as_str()).unwrap_or_default();
            let quantities = reagent.quantities;
            let available: u64 = quantities.iter().sum();
            let bags_capacity = quantities[0] / per_craft;
            let observed_capacity = available / per_craft;
            bag_limits.push((item_id, bags_capacity));
            observed_limits.push((item_id, observed_capacity));
            let bags_observed = !missing_storage.contains(&"bags");
            let any_storage_observed = missing_storage.len() < storage.len();
            let bags_capacity = bags_observed.then_some(bags_capacity);
            let observed_capacity = any_storage_observed.then_some(observed_capacity);
            let craft_shortfall = |capacity: u64| u64::from(crafts).saturating_sub(capacity);
            capacity_by_item.insert(
                item_id,
                json!({
                    "bags":bags_capacity,
                    "all_observed_storage":observed_capacity,
                    "craft_shortfall_vs_bags":bags_capacity.map(craft_shortfall),
                    "craft_shortfall_vs_observed_storage":observed_capacity.map(craft_shortfall),
                    "reagent_units_left_in_bags":bags_observed.then_some(quantities[0]).and_then(|count| count.checked_sub(required)),
                    "reagent_units_left_in_observed_storage":any_storage_observed.then_some(available).and_then(|count| count.checked_sub(required))
                }),
            );
            let by_storage: BTreeMap<_, _> = storage
                .into_iter()
                .zip(quantities)
                .map(|(name, count)| (name, (!missing_storage.contains(&name)).then_some(count)))
                .collect();
            let by_storage = json!(by_storage);
            let (bag_shortfall, withdrawals) = bag_restock(&by_storage, required);
            materials.push(json!({"item_id":item_id, "required":required,
                "quantity_observed":(missing_storage.len()<storage.len()).then_some(available),
                "shortfall_vs_observed":(missing_storage.len()<storage.len()).then_some(required.saturating_sub(available)),
                "quantity_by_storage":by_storage,
                "shortfall_vs_bags":bag_shortfall,"suggested_bank_withdrawals":withdrawals,
                "observed_stack_count":(missing_storage.len()<storage.len()).then_some(reagent.stack_count),
                "observed_locations":reagent.locations}));
        }
    }
    let capacity = |limits: &[(u32, u64)], observed| {
        if observed && let Some(maximum) = limits.iter().map(|(_, count)| *count).min() {
            let mut limiting_item_ids: Vec<_> = limits
                .iter()
                .filter_map(|(id, count)| (*count == maximum).then_some(*id))
                .collect();
            limiting_item_ids.sort_unstable();
            return json!({"supported_crafts":maximum,"limiting_item_ids":limiting_item_ids});
        }
        Value::Null
    };
    let recipe_id = recipe["recipe_id"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid recipe ID"))?;
    let charges = recipe["cooldown"]["charges"].as_u64();
    let estimated_cooldown_end = estimated_recipe_cooldown_end(recipe, snapshot.captured_at);
    let bags_capacity = capacity(&bag_limits, !missing_storage.contains(&"bags"));
    let observed_capacity = capacity(&observed_limits, missing_storage.len() < storage.len());
    let no_required_items = fixed && bag_limits.is_empty();
    let capacity_basis = |materials: &Value| {
        if no_required_items {
            "no_required_items"
        } else if materials["supported_crafts"].as_u64().is_some() {
            "observed_required_items"
        } else {
            "unavailable"
        }
    };
    let material_coverage = |materials: &Value| {
        if no_required_items {
            return Some(true);
        }
        materials["supported_crafts"]
            .as_u64()
            .map(|limit| limit >= u64::from(crafts))
    };
    let preparation_checks = [
        ("recipe_learned", recipe["learned"].as_bool()),
        (
            "observed_conditions_met",
            recipe["crafting_conditions"]
                .as_object()
                .map(|conditions| conditions.values().all(|row| row["met"] == true)),
        ),
        (
            "observed_charges_cover_request",
            charges.map(|available| available >= u64::from(crafts)),
        ),
        (
            "bags_cover_fixed_reagents",
            material_coverage(&bags_capacity),
        ),
        (
            "observed_storage_covers_fixed_reagents",
            material_coverage(&observed_capacity),
        ),
    ];
    let combined_capacity = |materials: &Value| {
        let supported = if no_required_items {
            charges
        } else {
            materials["supported_crafts"]
                .as_u64()
                .zip(charges)
                .map(|(items, charges)| items.min(charges))
        };
        supported.map(|supported| {
            json!({"supported_crafts":supported,
            "shortfall_vs_requested":u64::from(crafts).saturating_sub(supported)})
        })
    };
    let mut review = json!({"captured_at":snapshot.captured_at,"recipe":recipe,"requested_crafts":crafts,
        "captured_cooldown_state":recipe_cooldown_state(recipe),
        "recipe_tracking":recipe_tracking(snapshot, recipe_id),
        "captured_preparation":preparation_checks.into_iter().collect::<BTreeMap<_, _>>(),
        "profession_points_to_observed_cap":recipe["profession"]["max_rank"].as_u64().filter(|cap| *cap > 0)
            .zip(recipe["profession"]["rank"].as_u64()).and_then(|(cap, rank)| cap.checked_sub(rank)),
        "base_output_if_completed":recipe_base_output(recipe, u64::from(crafts)),
        "base_output_for_charge_covered_crafts_if_completed":charges
            .and_then(|available| recipe_base_output(recipe, available.min(u64::from(crafts)))),
        "estimated_cooldown_end_at":estimated_cooldown_end,
        "shortfall_vs_observed_recipe_charges":charges
            .map(|charges| u64::from(crafts).saturating_sub(charges)),
        "unmet_crafting_conditions":recipe["crafting_conditions"].as_object().map(|conditions|
            conditions.iter().filter(|(_, condition)| condition["met"].as_bool() == Some(false))
                .collect::<BTreeMap<_, _>>()),
        "materials":materials,"unobserved_storage":missing_storage,"live":false,
        "material_capacity":{
            "bags":bags_capacity,
            "all_observed_storage":observed_capacity
        },
        "material_capacity_basis":{
            "bags":capacity_basis(&bags_capacity),
            "all_observed_storage":capacity_basis(&observed_capacity)
        },
        "material_and_charge_capacity":{
            "bags":combined_capacity(&bags_capacity),
            "all_observed_storage":combined_capacity(&observed_capacity)
        },
        "craftability_verified":false,"game_execution":false});
    if let Some(inventory) = inventory {
        review["inventory_captured_at"] = json!(inventory.captured_at);
    }
    review["material_capacity_by_item"] = json!(fixed.then_some(capacity_by_item));
    review["pending_mail_material_observations"] = stock.sections.get("mail_attachments")
        .filter(|_|fixed).map(|mail| {
            let (totals,unknown)=mail_attachment_totals(mail);
            let ids:BTreeSet<u64>=recipe["reagents"].as_object().unwrap().keys()
                .map(|id|id.parse().unwrap()).collect();
            let mut items=Vec::new();
            let mut unobserved=Vec::new();
            for id in ids {
                if let Some((quantity,slots))=totals.get(&("item",id)) {
                    items.push(json!({"item_id":id,"known_quantity":quantity,"known_attachment_slots":slots}));
                } else { unobserved.push(id); }
            }
            json!({"coverage":"loaded_inbox_attachments","captured_at":stock.captured_at,
                "loaded_messages":mail["loaded_messages"],"total_messages":mail["total_messages"],
                "unloaded_messages":unloaded_mail_messages(mail),
                "unknown_attachment_slots":unknown,"counts_toward_material_capacity":false,
                "items":items,"unobserved_reagent_ids":unobserved})
        }).unwrap_or(Value::Null);
    Ok(review)
}

/// Combine demands from reviews already validated against the same inventory capture.
pub fn combine_recipe_materials<'a>(reviews: impl Iterator<Item = &'a Value>) -> Result<Value> {
    let mut totals = BTreeMap::<u64, (u64, Option<u64>, &Value)>::new();
    for review in reviews {
        if review["recipe"]["requirements"] != "fixed_items" {
            return Ok(Value::Null);
        }
        for material in review["materials"].as_array().into_iter().flatten() {
            let id = material["item_id"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing reagent ID"))?;
            let required = material["required"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing reagent quantity"))?;
            let observed = material["quantity_observed"].as_u64();
            let by_storage = &material["quantity_by_storage"];
            let total = totals.entry(id).or_insert((0, observed, by_storage));
            ensure!(
                total.1 == observed && total.2 == by_storage,
                "Inventory observations differ"
            );
            total.0 = total
                .0
                .checked_add(required)
                .ok_or_else(|| anyhow::anyhow!("Reagent quantity overflow"))?;
        }
    }
    Ok(json!(
        totals
            .into_iter()
            .map(|(item_id, (required, observed, by_storage))| json!({
                "item_id":item_id,"required":required,"quantity_observed":observed,
                "shortfall_vs_observed":observed.map(|quantity| required.saturating_sub(quantity)),
                "quantity_by_storage":by_storage,
                "shortfall_vs_bags":by_storage["bags"].as_u64().map(|count| required.saturating_sub(count))
            }))
            .collect::<Vec<_>>()
    ))
}

/// Model minimum yields in supplied recipe order, consuming reagents before producing output.
pub fn ordered_recipe_materials<'a>(
    reviews: impl Iterator<Item = &'a Value>,
    combined_materials: &Value,
) -> Result<(Value, Value)> {
    let Some(materials) = combined_materials.as_array() else {
        return Ok((Value::Null, Value::Null));
    };
    let mut balance = BTreeMap::<u64, i64>::new();
    let mut starting_stock = BTreeMap::<u64, u64>::new();
    let mut covered_prefix = 0usize;
    let mut first_uncovered = None;
    for (index, review) in reviews.enumerate() {
        let mut blocking_reagents = BTreeMap::new();
        let mut insufficient_stock = false;
        for material in review["materials"].as_array().into_iter().flatten() {
            let item = material["item_id"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing reagent ID"))?;
            let required = material["required"]
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("Missing reagent quantity"))?;
            let available = balance.entry(item).or_default();
            *available = available
                .checked_sub(required)
                .ok_or_else(|| anyhow::anyhow!("Reagent balance overflow"))?;
            let minimum = starting_stock.entry(item).or_default();
            *minimum = (*minimum).max((*available).min(0).unsigned_abs());
            if first_uncovered.is_none() && *available < 0 {
                let needed = available.unsigned_abs();
                let shortfall = material["quantity_observed"]
                    .as_u64()
                    .map(|count| needed.saturating_sub(count));
                if shortfall != Some(0) {
                    insufficient_stock |= shortfall.is_some();
                    let (bag_shortfall, withdrawals) =
                        bag_restock(&material["quantity_by_storage"], needed);
                    blocking_reagents.insert(
                        item,
                        json!({
                            "starting_units_needed_through_recipe":needed,
                            "shortfall_vs_observed":shortfall,
                            "shortfall_vs_bags":bag_shortfall,
                            "suggested_bank_withdrawals":withdrawals
                        }),
                    );
                }
            }
        }
        if first_uncovered.is_none() {
            if !blocking_reagents.is_empty() {
                first_uncovered = Some(json!({"recipe_number":index + 1,
                    "recipe_id":review["recipe"]["recipe_id"],
                    "reason":if insufficient_stock {"shortfall_vs_observed_stock"} else {"stock_unknown"},
                    "blocking_reagents_by_item_id":blocking_reagents}));
            } else {
                covered_prefix += 1;
            }
        }
        let output = &review["base_output_if_completed"];
        if let Some((item, minimum)) = output["item_id"].as_u64().zip(output["minimum"].as_i64()) {
            let available = balance.entry(item).or_default();
            *available = available
                .checked_add(minimum)
                .ok_or_else(|| anyhow::anyhow!("Output balance overflow"))?;
        }
    }
    let mut starting_materials = Vec::new();
    for material in materials {
        let item = material["item_id"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing combined reagent ID"))?;
        let required = *starting_stock
            .get(&item)
            .ok_or_else(|| anyhow::anyhow!("Missing starting stock estimate"))?;
        let observed = material["quantity_observed"].as_u64();
        let by_storage = &material["quantity_by_storage"];
        let (bag_shortfall, withdrawals) = bag_restock(by_storage, required);
        let remaining = observed
            .filter(|count| *count >= required)
            .and_then(|count| count.checked_add_signed(*balance.get(&item)?));
        starting_materials.push(json!({
            "item_id":item,"required_before_batch":required,"quantity_observed":observed,
            "shortfall_vs_observed":observed.map(|quantity| required.saturating_sub(quantity)),
            "shortfall_vs_bags":bag_shortfall,"suggested_bank_withdrawals":withdrawals,
            "estimated_units_after_batch_in_observed_storage":remaining
        }));
    }
    Ok((
        json!(starting_materials),
        json!({"covered_prefix_recipes":covered_prefix,
        "first_uncovered_recipe":first_uncovered}),
    ))
}

pub fn ordered_recipe_material_coverage(materials: &Value) -> Value {
    let Some(rows) = materials.as_array() else {
        return Value::Null;
    };
    let covered = |field: &str| {
        let mut all_known = true;
        for row in rows.iter().filter(|row| row["required_before_batch"] != 0) {
            match row[field].as_u64() {
                Some(0) => {}
                Some(_) => return Some(false),
                None => all_known = false,
            }
        }
        all_known.then_some(true)
    };
    json!({"bags":covered("shortfall_vs_bags"),
        "all_observed_storage":covered("shortfall_vs_observed")})
}

pub fn recipe_dependency_order<'a>(
    reviews: impl Iterator<Item = &'a Value>,
    combined_materials: &Value,
) -> Result<Value> {
    let reviews: Vec<_> = reviews.collect();
    let mut unresolved_inputs = Vec::new();
    let mut unresolved_outputs = Vec::new();
    let mut edges = Vec::new();
    let mut successors = vec![BTreeSet::new(); reviews.len()];
    let mut incoming = vec![0; reviews.len()];
    for (index, review) in reviews.iter().enumerate() {
        if review["recipe"]["requirements"] != "fixed_items" {
            unresolved_inputs.push(index + 1);
        }
        if review["base_output_if_completed"]["item_id"]
            .as_u64()
            .is_none()
            || review["base_output_if_completed"]["minimum"]
                .as_u64()
                .is_none()
        {
            unresolved_outputs.push(index + 1);
        }
    }
    for (producer, review) in reviews.iter().enumerate() {
        let output = &review["base_output_if_completed"];
        let Some(item) = output["item_id"]
            .as_u64()
            .filter(|_| output["minimum"].as_u64().is_some())
        else {
            continue;
        };
        for (consumer, other) in reviews.iter().enumerate() {
            if other["recipe"]["requirements"] == "fixed_items"
                && other["materials"].as_array().is_some_and(|materials| {
                    materials
                        .iter()
                        .any(|row| row["item_id"].as_u64() == Some(item))
                })
            {
                edges.push(json!({"producer_recipe_number":producer + 1,
                    "consumer_recipe_number":consumer + 1,"item_id":item}));
                if successors[producer].insert(consumer) {
                    incoming[consumer] += 1;
                }
            }
        }
    }
    let mut ready: BTreeSet<_> = incoming
        .iter()
        .enumerate()
        .filter_map(|(index, count)| (*count == 0).then_some(index))
        .collect();
    let mut order = Vec::new();
    while let Some(index) = ready.pop_first() {
        order.push(index);
        for &consumer in &successors[index] {
            incoming[consumer] -= 1;
            if incoming[consumer] == 0 {
                ready.insert(consumer);
            }
        }
    }
    let status = if !unresolved_inputs.is_empty() || !unresolved_outputs.is_empty() {
        "incomplete_observations"
    } else if order.len() != reviews.len() {
        "cyclic_dependencies"
    } else {
        "suggested"
    };
    let suggested = status == "suggested";
    let mut result = json!({"status":status,"dependencies":edges,
        "unresolved_input_recipe_numbers":unresolved_inputs,
        "unresolved_output_recipe_numbers":unresolved_outputs,
        "blocked_recipe_numbers":incoming.iter().enumerate().filter_map(|(index, count)|
            (*count > 0).then_some(index + 1)).collect::<Vec<_>>(),
        "suggested_recipe_numbers":suggested.then(|| order.iter().map(|index| index+1).collect::<Vec<_>>()),
        "starting_materials":Value::Null,"material_checkpoint":Value::Null,
        "nonmaterial_requirements":Value::Null,"material_coverage":Value::Null});
    if suggested {
        let ordered = || order.iter().map(|index| reviews[*index]);
        let remap_checkpoint = |checkpoint: &mut Value, field: &str| {
            if let Some(row) = checkpoint.get_mut(field).and_then(Value::as_object_mut)
                && let Some(step) = row.get("recipe_number").and_then(Value::as_u64)
                && let Some(original) = step
                    .checked_sub(1)
                    .and_then(|index| order.get(index as usize))
            {
                row.insert("suggested_step".to_string(), json!(step));
                row.insert("recipe_number".to_string(), json!(original + 1));
            }
        };
        let (materials, mut checkpoint) = ordered_recipe_materials(ordered(), combined_materials)?;
        remap_checkpoint(&mut checkpoint, "first_uncovered_recipe");
        let mut requirements = ordered_recipe_requirements(ordered());
        remap_checkpoint(&mut requirements, "first_observed_shortfall");
        result["material_coverage"] = ordered_recipe_material_coverage(&materials);
        result["starting_materials"] = materials;
        result["material_checkpoint"] = checkpoint;
        result["nonmaterial_requirements"] = requirements;
    }
    Ok(result)
}

pub fn ordered_recipe_requirements<'a>(reviews: impl Iterator<Item = &'a Value>) -> Value {
    ordered_recipe_requirements_with_charges(reviews, false)
}

fn ordered_recipe_requirements_with_charges<'a>(
    reviews: impl Iterator<Item = &'a Value>,
    cumulative: bool,
) -> Value {
    let mut checks = vec![
        "recipe_learned",
        "observed_conditions_met",
        "observed_charges_cover_request",
    ];
    if cumulative {
        checks.push("observed_charges_cover_cumulative_crafts");
    }
    let mut first_shortfall = None;
    let mut recipes_with_unknown_checks = 0;
    let mut ledger = RecipeChargeLedger::default();
    let mut charge_accounting = Vec::new();
    let mut condition_checks = (0, 0, 0);
    for (index, review) in reviews.enumerate() {
        let mut preparation = review["captured_preparation"].clone();
        if cumulative {
            match preparation["observed_conditions_met"].as_bool() {
                Some(true) => condition_checks.0 += 1,
                Some(false) => condition_checks.1 += 1,
                None => condition_checks.2 += 1,
            }
            let mut charges = ledger.consume(review);
            preparation["observed_charges_cover_cumulative_crafts"] =
                charges["covers_cumulative_crafts"].clone();
            charges["recipe_number"] = json!(index + 1);
            charges["recipe_id"] = review["recipe"]["recipe_id"].clone();
            charge_accounting.push(charges);
        }
        if checks.iter().any(|check| preparation[*check].is_null()) {
            recipes_with_unknown_checks += 1;
        }
        if first_shortfall.is_none() {
            let failed: Vec<_> = checks
                .iter()
                .filter(|check| preparation[**check] == false)
                .collect();
            if !failed.is_empty() {
                first_shortfall = Some(json!({
                    "recipe_number":index+1,"recipe_id":review["recipe"]["recipe_id"],
                    "failed_checks":failed,
                }));
            }
        }
    }
    let mut result = json!({"first_observed_shortfall":first_shortfall,
        "recipes_with_unknown_checks":recipes_with_unknown_checks});
    if cumulative {
        let (met, unmet, unknown) = condition_checks;
        result["observed_chain_conditions"] = json!({
            "steps_met":met,"steps_unmet":unmet,"steps_unknown":unknown,
            "all_met":if unmet > 0 { Some(false) }
                else if unknown == 0 && met > 0 { Some(true) }
                else { None },
        });
        let first_charge_shortfall = charge_accounting
            .iter()
            .find(|row| row["covers_cumulative_crafts"] == false);
        result["first_observed_charge_shortfall"] = json!(first_charge_shortfall);
        result["observed_charges_cover_chain"] = json!(if first_charge_shortfall.is_some() {
            Some(false)
        } else if charge_accounting.is_empty()
            || charge_accounting
                .iter()
                .any(|row| row["covers_cumulative_crafts"].is_null())
        {
            None
        } else {
            Some(true)
        });
        result["charge_accounting"] = json!(charge_accounting);
    }
    result
}

pub fn bag_restock(by_storage: &Value, required: u64) -> (Option<u64>, Option<Value>) {
    let bag_shortfall = by_storage["bags"]
        .as_u64()
        .map(|count| required.saturating_sub(count));
    let withdrawals = bag_shortfall.map(|shortfall| {
        let character_bank = by_storage["character_bank"]
            .as_u64()
            .map(|count| count.min(shortfall));
        let remaining = shortfall - character_bank.unwrap_or(0);
        let account_bank = by_storage["account_bank"]
            .as_u64()
            .map(|count| count.min(remaining));
        json!({"character_bank":character_bank,"account_bank":account_bank})
    });
    (bag_shortfall, withdrawals)
}

pub fn combine_recipe_outputs<'a>(reviews: impl Iterator<Item = &'a Value>) -> Value {
    let mut totals = BTreeMap::<u64, Option<(u64, u64)>>::new();
    let mut recipes_without_observed_output_item = 0;
    for review in reviews {
        let Some(item_id) = review["recipe"]["output_item_id"].as_u64() else {
            recipes_without_observed_output_item += 1;
            continue;
        };
        let output = &review["base_output_if_completed"];
        let range = output["minimum"].as_u64().zip(output["maximum"].as_u64());
        let total = totals.entry(item_id).or_insert(Some((0, 0)));
        *total = total
            .zip(range)
            .and_then(|((a_min, a_max), (b_min, b_max))| {
                Some((a_min.checked_add(b_min)?, a_max.checked_add(b_max)?))
            });
    }
    let items: Vec<_> = totals
        .into_iter()
        .map(|(item_id, range)| {
            json!({"item_id":item_id,"minimum":range.map(|(minimum, _)| minimum),
                "maximum":range.map(|(_, maximum)| maximum)})
        })
        .collect();
    json!({"items":items,"recipes_without_observed_output_item":recipes_without_observed_output_item})
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LootRequirement {
    pub item_id: std::num::NonZeroU32,
    pub quantity: std::num::NonZeroU64,
}

pub fn validate_loot_requirements(requirements: &[LootRequirement]) -> Result<()> {
    ensure!(
        (1..=64).contains(&requirements.len()),
        "Expected one to 64 loot requirements"
    );
    let ids: BTreeSet<_> = requirements.iter().map(|row| row.item_id).collect();
    ensure!(
        ids.len() == requirements.len(),
        "Duplicate loot requirement item ID"
    );
    Ok(())
}

/// Conditional comparison against readable displayed rows, never inventory credit.
pub fn review_loot_requirements(
    snapshot: &Snapshot,
    requirements: &[LootRequirement],
) -> Result<Value> {
    validate_loot_requirements(requirements)?;
    ensure!(
        snapshot.client.family == crate::Family::Forever,
        "Expected a Forever loot capture"
    );
    let Some(section) = snapshot.sections.get("loot_window") else {
        return Ok(Value::Null);
    };
    let entries = section["entries"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Loot rows unavailable"))?;
    let non_item = entries
        .values()
        .filter(|row| row["status"] != "item")
        .count();
    let unknown = entries
        .values()
        .filter(|row| row["status"] == "unreadable_or_non_item")
        .count();
    let known_non_item = non_item - unknown;
    let mut all_covered = true;
    let mut rows = Vec::new();
    for requirement in requirements {
        let item_id = requirement.item_id.get();
        let requested = requirement.quantity.get();
        let mut matching: Vec<_> = entries
            .iter()
            .filter(|(_, row)| {
                row["status"] == "item" && row["item_id"].as_u64() == Some(u64::from(item_id))
            })
            .map(|(slot, row)| {
                Ok((
                    slot.parse::<u32>()?,
                    row["quantity"]
                        .as_u64()
                        .ok_or_else(|| anyhow::anyhow!("Loot quantity unavailable"))?,
                ))
            })
            .collect::<Result<_>>()?;
        matching.sort_unstable_by_key(|(slot, _)| *slot);
        let readable = matching.iter().try_fold(0u64, |sum, (_, quantity)| {
            sum.checked_add(*quantity)
                .ok_or_else(|| anyhow::anyhow!("Loot quantity overflow"))
        })?;
        all_covered &= readable >= requested;
        rows.push(json!({"item_id":item_id,"requested_quantity":requested,"readable_quantity":readable,
            "matching_slots":matching.iter().map(|(slot,quantity)|json!({"slot":slot,"quantity":quantity})).collect::<Vec<_>>(),
            "covered_quantity_if_readable_items_acquired":readable.min(requested),
            "remaining_request_if_readable_items_acquired":requested.saturating_sub(readable),
            "readable_slots_cover_request":readable >= requested}));
    }
    let mut report = json!({"basis":"captured_readable_loot_rows_only","grouping":"base_item_id_only",
        "captured_at":snapshot.captured_at,"slot_count":section["slot_count"],
        "unreadable_or_non_item_slots":non_item,"additional_matching_quantities_unknown":unknown > 0,
        "all_requests_covered_by_readable_slots":all_covered,"items":rows,
        "counts_toward_owned_inventory":false,"ownership_verified":false,"lootability_verified":false,
        "game_execution":false,"live":false});
    if known_non_item > 0 {
        report["known_non_item_slots"] = json!(known_non_item);
        report["unreadable_slots"] = json!(unknown);
    }
    Ok(report)
}

pub fn loot_requirement_bag_space(snapshot: &Snapshot, review: &Value) -> Value {
    let items = review["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let item_id = u32::try_from(row["item_id"].as_u64()?).ok()?;
            let quantity = row["covered_quantity_if_readable_items_acquired"].as_u64()?;
            Some((item_id, quantity))
        });
    incoming_items_bag_space(
        snapshot,
        items,
        "readable_requested_loot_only",
        "readable_requested_quantity",
    )
}

pub(crate) fn incoming_items_bag_space(
    snapshot: &Snapshot,
    quantities: impl Iterator<Item = (u32, u64)>,
    scope: &str,
    quantity_field: &str,
) -> Value {
    let items: Vec<_> = quantities
        .map(|(item_id, quantity)| {
            let mut estimate = carried_item_quantity_bag_space(snapshot, item_id, quantity);
            estimate["item_id"] = json!(item_id);
            estimate[quantity_field] = json!(quantity);
            estimate
        })
        .collect();
    let free =
        carried_bag_space(snapshot).and_then(|space| space["known_general_free_slots"].as_u64());
    let total = |field: &str| {
        items
            .iter()
            .try_fold(0u64, |sum, row| sum.checked_add(row[field].as_u64()?))
    };
    let slots = total("estimated_new_stack_slots");
    let merged = total("estimated_new_stack_slots_if_same_item_stacks_merge");
    json!({"scope":scope,"items":items,
        "recorded_general_free_slots":free,"estimated_new_stack_slots":slots,
        "fits_recorded_general_slots":slots.zip(free).map(|(needed, free)| needed <= free),
        "estimated_new_stack_slots_if_same_item_stacks_merge":merged,
        "fits_recorded_general_slots_if_same_item_stacks_merge":merged.zip(free)
            .map(|(needed, free)| needed <= free),
        "same_item_stack_merge_verified":false,"placement_verified":false,"live":false})
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum BagCooldownStatus {
    Reported,
    Unavailable,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BagCooldownFilter {
    pub item_ids: Option<Vec<std::num::NonZeroU32>>,
    pub status: Option<BagCooldownStatus>,
    pub enabled: Option<bool>,
    pub minimum_remaining_seconds: Option<u32>,
    pub maximum_remaining_seconds: Option<u32>,
    /// Inclusive projected deadline for an enabled saved cooldown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projected_end_by: Option<chrono::DateTime<chrono::Utc>>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl BagCooldownFilter {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.item_ids.is_some()
                || self.status.is_some()
                || self.enabled.is_some()
                || self.minimum_remaining_seconds.is_some()
                || self.maximum_remaining_seconds.is_some()
                || self.projected_end_by.is_some()
                || self.offset.is_some()
                || self.limit.is_some(),
            "Bag cooldown filter must contain at least one field"
        );
        if let Some(ids) = &self.item_ids {
            ensure!(
                (1..=64).contains(&ids.len())
                    && ids.iter().copied().collect::<BTreeSet<_>>().len() == ids.len(),
                "Bag cooldown item IDs must contain 1 to 64 unique IDs"
            );
        }
        if let (Some(minimum), Some(maximum)) = (
            self.minimum_remaining_seconds,
            self.maximum_remaining_seconds,
        ) {
            ensure!(minimum <= maximum, "Bag cooldown timer bounds are reversed");
        }
        ensure!(
            (1..=50).contains(&self.limit.unwrap_or(20)),
            "Bag cooldown limit must be between 1 and 50"
        );
        Ok(())
    }
}

fn bag_cooldown_matches(
    row: &Value,
    filter: &BagCooldownFilter,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> Option<bool> {
    let checks: Vec<_> = [
        filter.item_ids.as_ref().map(|ids| {
            Some(
                ids.iter()
                    .any(|id| row["item_id"].as_u64() == Some(u64::from(id.get()))),
            )
        }),
        filter.status.map(|status| {
            Some(
                row["status"]
                    == match status {
                        BagCooldownStatus::Reported => "reported",
                        BagCooldownStatus::Unavailable => "unavailable",
                    },
            )
        }),
        filter
            .enabled
            .map(|expected| row["enabled"].as_bool().map(|actual| actual == expected)),
        filter.minimum_remaining_seconds.map(|minimum| {
            row["remaining_seconds"]
                .as_u64()
                .map(|seconds| seconds >= u64::from(minimum))
        }),
        filter.maximum_remaining_seconds.map(|maximum| {
            row["remaining_seconds"]
                .as_u64()
                .map(|seconds| seconds <= u64::from(maximum))
        }),
        filter.projected_end_by.map(|deadline| {
            bag_cooldown_projected_end(row, captured_at).map(|end| end <= deadline)
        }),
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

/// Filters validated, captured bag cooldown rows without inferring present usability.
pub fn filter_bag_cooldowns(
    section: Option<&Value>,
    filter: &BagCooldownFilter,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> Result<Value> {
    filter.validate()?;
    let offset = filter.offset.unwrap_or(0);
    let limit = filter.limit.unwrap_or(20);
    let mut result = json!({
        "basis":"captured_bag_item_cooldowns", "filter":filter,
        "rows":null, "captured_item_locations":null, "matched_item_locations":null,
        "unknown_filter_item_locations":null, "nonmatching_item_locations":null,
        "offset":offset, "limit":limit, "has_more":null,
        "current_usability_verified":false, "game_execution":false
    });
    let Some(section) = section else {
        return Ok(result);
    };
    let entries = section["entries"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Bag cooldown entries unavailable"))?;
    let mut matching = Vec::new();
    let (mut captured, mut unknown, mut nonmatching) = (0usize, 0usize, 0usize);
    for (bag, slots) in entries {
        let bag = bag.parse::<u32>()?;
        let slots = slots
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Bag cooldown slots unavailable"))?;
        for (slot, row) in slots {
            captured += 1;
            match bag_cooldown_matches(row, filter, captured_at) {
                Some(true) => matching.push((bag, slot.parse::<u32>()?, row)),
                Some(false) => nonmatching += 1,
                None => unknown += 1,
            }
        }
    }
    matching.sort_unstable_by_key(|(bag, slot, _)| (*bag, *slot));
    let rows: Vec<_> = matching
        .iter()
        .skip(offset)
        .take(limit)
        .map(|(bag, slot, row)| {
            let mut flattened = json!({"bag":bag, "slot":slot,
            "item_id":row["item_id"], "status":row["status"]});
            for key in ["enabled", "remaining_seconds"] {
                if let Some(value) = row.get(key) {
                    flattened[key] = value.clone();
                }
            }
            if filter.projected_end_by.is_some() {
                flattened["projected_cooldown_end_at"] =
                    json!(bag_cooldown_projected_end(row, captured_at));
            }
            flattened
        })
        .collect();
    result["rows"] = json!(rows);
    result["captured_item_locations"] = json!(captured);
    result["matched_item_locations"] = json!(matching.len());
    result["unknown_filter_item_locations"] = json!(unknown);
    result["nonmatching_item_locations"] = json!(nonmatching);
    result["has_more"] = json!(matching.len().saturating_sub(offset) > limit);
    Ok(result)
}

fn bag_cooldown_projected_end(
    row: &Value,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    (row["enabled"] == true).then_some(())?;
    crate::saved_resets::projected_reset_at(row["remaining_seconds"].as_i64()?, captured_at)
}

pub fn project_bag_cooldown_reference(
    review: &mut Value,
    captured_at: chrono::DateTime<chrono::Utc>,
    reference: chrono::DateTime<chrono::Utc>,
) {
    review["reference_at"] = json!(reference);
    for row in review["rows"].as_array_mut().into_iter().flatten() {
        let end = bag_cooldown_projected_end(row, captured_at);
        row["projected_cooldown_end_at"] = json!(end);
        row["projected_end_at_or_before_reference"] = json!(end.map(|end| end <= reference));
    }
}

#[cfg(test)]
mod bag_cooldown_filter_tests {
    use super::*;

    fn filter_bag_cooldowns(section: Option<&Value>, filter: &BagCooldownFilter) -> Result<Value> {
        super::filter_bag_cooldowns(section, filter, chrono::DateTime::UNIX_EPOCH)
    }

    fn filter(value: Value) -> BagCooldownFilter {
        serde_json::from_value(value).unwrap()
    }

    fn source() -> Value {
        json!({"coverage":"captured_carried_items_only", "captured_item_locations":4,
        "entries":{
            "10":{"1":{"item_id":100,"status":"reported","enabled":true,"remaining_seconds":0}},
            "2":{
                "10":{"item_id":100,"status":"reported","enabled":false,"remaining_seconds":10},
                "2":{"item_id":200,"status":"unavailable"},
                "1":{"item_id":100,"status":"unavailable"}
            }
        }})
    }

    fn assert_counts(result: &Value, matched: usize, unknown: usize, nonmatching: usize) {
        assert_eq!(result["matched_item_locations"], matched);
        assert_eq!(result["unknown_filter_item_locations"], unknown);
        assert_eq!(result["nonmatching_item_locations"], nonmatching);
        assert_eq!(
            result["captured_item_locations"],
            matched + unknown + nonmatching
        );
    }

    #[test]
    fn conjunctive_predicates_preserve_unknown_and_false_dominates() {
        let source = source();
        let query = filter(
            json!({"item_ids":[100],"enabled":false,"minimum_remaining_seconds":10,"maximum_remaining_seconds":10}),
        );
        let result = filter_bag_cooldowns(Some(&source), &query).unwrap();
        assert_counts(&result, 1, 1, 2);
        assert_eq!(result["rows"][0]["slot"], 10);
        let result = filter_bag_cooldowns(
            Some(&source),
            &filter(json!({"status":"reported","enabled":true})),
        )
        .unwrap();
        assert_counts(&result, 1, 0, 3);
        let result = filter_bag_cooldowns(
            Some(&source),
            &filter(json!({"minimum_remaining_seconds":0,"maximum_remaining_seconds":0})),
        )
        .unwrap();
        assert_counts(&result, 1, 2, 1);
    }

    #[test]
    fn unavailable_status_is_known_but_its_timer_and_enabled_are_unknown() {
        let source = source();
        let result =
            filter_bag_cooldowns(Some(&source), &filter(json!({"status":"unavailable"}))).unwrap();
        assert_counts(&result, 2, 0, 2);
        assert!(result["rows"][0].get("enabled").is_none());
        assert!(result["rows"][0].get("remaining_seconds").is_none());
        for predicate in [
            json!({"status":"unavailable","enabled":false}),
            json!({"status":"unavailable","maximum_remaining_seconds":0}),
        ] {
            let result = filter_bag_cooldowns(Some(&source), &filter(predicate)).unwrap();
            assert_counts(&result, 0, 2, 2);
        }
    }

    #[test]
    fn pagination_only_sorts_numerically_and_does_not_create_unknowns() {
        let source = source();
        let result = filter_bag_cooldowns(Some(&source), &filter(json!({"limit":2}))).unwrap();
        assert_counts(&result, 4, 0, 0);
        assert_eq!(result["rows"][0]["bag"], 2);
        assert_eq!(result["rows"][0]["slot"], 1);
        assert_eq!(result["rows"][1]["slot"], 2);
        assert_eq!(result["has_more"], true);
        let result =
            filter_bag_cooldowns(Some(&source), &filter(json!({"offset":2,"limit":2}))).unwrap();
        assert_eq!(result["rows"][0]["bag"], 2);
        assert_eq!(result["rows"][0]["slot"], 10);
        assert_eq!(result["rows"][1]["bag"], 10);
        assert_eq!(result["has_more"], false);
        let result =
            filter_bag_cooldowns(Some(&source), &filter(json!({"offset":usize::MAX}))).unwrap();
        assert_counts(&result, 4, 0, 0);
        assert_eq!(result["rows"], json!([]));
        assert_eq!(result["has_more"], false);
        assert_eq!(result["limit"], 20);
    }

    #[test]
    fn missing_section_is_distinct_from_known_empty_and_still_validates() {
        let query = filter(json!({"offset":0}));
        let missing = filter_bag_cooldowns(None, &query).unwrap();
        for field in [
            "rows",
            "captured_item_locations",
            "matched_item_locations",
            "unknown_filter_item_locations",
            "nonmatching_item_locations",
            "has_more",
        ] {
            assert!(missing[field].is_null(), "{field}");
        }
        assert_eq!(missing["current_usability_verified"], false);
        assert_eq!(missing["game_execution"], false);
        let empty = json!({"coverage":"captured_carried_items_only","captured_item_locations":0,"entries":{}});
        let result = filter_bag_cooldowns(Some(&empty), &query).unwrap();
        assert_counts(&result, 0, 0, 0);
        assert_eq!(result["rows"], json!([]));
        assert_eq!(result["has_more"], false);
        assert!(filter_bag_cooldowns(None, &filter(json!({}))).is_err());
    }

    #[test]
    fn malformed_filters_and_limits_are_rejected() {
        for value in [
            json!({"bogus":1}),
            json!({"status":"ready"}),
            json!({"item_ids":[0]}),
            json!({"item_ids":[4294967296_u64]}),
            json!({"enabled":1}),
            json!({"offset":-1}),
            json!({"minimum_remaining_seconds":-1}),
            json!({"maximum_remaining_seconds":1.5}),
        ] {
            assert!(serde_json::from_value::<BagCooldownFilter>(value).is_err());
        }
        for value in [
            json!({}),
            json!({"enabled":null}),
            json!({"item_ids":[]}),
            json!({"item_ids":[1,1]}),
            json!({"item_ids":(1..=65).collect::<Vec<u32>>()}),
            json!({"limit":0}),
            json!({"limit":51}),
            json!({"minimum_remaining_seconds":2,"maximum_remaining_seconds":1}),
        ] {
            assert!(filter(value).validate().is_err());
        }
        for value in [
            json!({"item_ids":(1..=64).collect::<Vec<u32>>()}),
            json!({"limit":50}),
            json!({"enabled":false}),
            json!({"minimum_remaining_seconds":u32::MAX,"maximum_remaining_seconds":u32::MAX}),
        ] {
            assert!(filter(value).validate().is_ok());
        }
    }

    #[test]
    fn maximum_capture_keeps_counts_before_bounded_pagination() {
        let mut entries = serde_json::Map::new();
        for bag in 0..4 {
            let mut slots = serde_json::Map::new();
            for slot in 1..=1024 {
                slots.insert(
                    slot.to_string(),
                    json!({"item_id":1,"status":"unavailable"}),
                );
            }
            entries.insert(bag.to_string(), Value::Object(slots));
        }
        let source = json!({"coverage":"captured_carried_items_only","captured_item_locations":4096,"entries":entries});
        let result = filter_bag_cooldowns(Some(&source), &filter(json!({"limit":50}))).unwrap();
        assert_counts(&result, 4096, 0, 0);
        assert_eq!(result["rows"].as_array().unwrap().len(), 50);
        assert_eq!(result["rows"][49]["slot"], 50);
        assert_eq!(result["has_more"], true);
    }
}

pub fn find_item(snapshot: &Snapshot, item_id: u32, offset: usize, limit: usize) -> Result<Value> {
    find_item_with_container_quantities(snapshot, item_id, offset, limit, false, None, None)
}

pub fn find_item_with_container_quantities(
    snapshot: &Snapshot,
    item_id: u32,
    offset: usize,
    limit: usize,
    include_container_quantities: bool,
    cooldown_reference: Option<chrono::DateTime<chrono::Utc>>,
    cooldown_target: Option<std::num::NonZeroU64>,
) -> Result<Value> {
    find_item_page(
        snapshot,
        item_id,
        (offset, limit),
        include_container_quantities,
        cooldown_reference,
        cooldown_target,
        None,
    )
}

pub(crate) fn find_item_preferring_saved_link(
    snapshot: &Snapshot,
    item_id: u32,
    limit: usize,
    preferred_link: Option<&str>,
) -> Result<Value> {
    find_item_page(
        snapshot,
        item_id,
        (0, limit),
        false,
        None,
        None,
        preferred_link,
    )
}

fn find_item_page(
    snapshot: &Snapshot,
    item_id: u32,
    (offset, limit): (usize, usize),
    include_container_quantities: bool,
    cooldown_reference: Option<chrono::DateTime<chrono::Utc>>,
    cooldown_target: Option<std::num::NonZeroU64>,
    preferred_link: Option<&str>,
) -> Result<Value> {
    ensure!(
        item_id > 0 && (1..=50).contains(&limit),
        "Invalid item query"
    );
    let mut locations = Vec::new();
    let mut preferred_locations = Vec::new();
    let mut preferred_match_count = 0;
    let mut matched_locations = 0usize;
    let mut quantities = BTreeMap::new();
    let mut container_quantities = BTreeMap::new();
    let cooldowns = snapshot.sections.get("bag_item_cooldowns");
    let durability = snapshot
        .sections
        .get("equipment_durability")
        .and_then(|section| section["entries"].as_object());
    ensure!(
        cooldown_reference.is_none() || snapshot.client.family == crate::Family::Forever,
        "Item cooldown projections require Forever"
    );
    let (mut ended, mut pending, mut unknown_end) = (0u64, 0u64, 0u64);
    let mut copies_by_end = BTreeMap::new();
    let (mut reported, mut unavailable, mut disabled, mut positive_timer) =
        (0usize, 0usize, 0usize, 0usize);
    for storage in ["equipment", "bags", "character_bank", "account_bank"] {
        if storage.ends_with("_bank") && snapshot.client.family != crate::Family::Forever {
            continue;
        }
        let Some(section) = snapshot.sections.get(storage).and_then(Value::as_object) else {
            quantities.insert(storage, None);
            if include_container_quantities && storage != "equipment" {
                container_quantities.insert(storage, None);
            }
            continue;
        };
        let mut quantity = 0;
        let mut container_totals = BTreeMap::new();
        let mut record = |container: Option<&str>, slot: &str, item: &Value, count: u64| {
            if item["item_id"].as_u64() == Some(u64::from(item_id)) {
                quantity += count;
                let cooldown = if storage == "bags" {
                    cooldowns
                        .and_then(|section| {
                            section["entries"]
                                .get(container?)
                                .and_then(|bag| bag.get(slot))
                        })
                        .filter(|row| row["item_id"].as_u64() == Some(u64::from(item_id)))
                } else {
                    None
                };
                if let Some(row) = cooldown {
                    if row["status"] == "reported" {
                        reported += 1;
                        disabled += usize::from(row["enabled"] == false);
                        positive_timer +=
                            usize::from(row["remaining_seconds"].as_u64().is_some_and(|n| n > 0));
                    } else {
                        unavailable += 1;
                    }
                }
                let projection =
                    cooldown_reference
                        .filter(|_| storage == "bags")
                        .map(|reference| {
                            let end = cooldown.and_then(|row| {
                                bag_cooldown_projected_end(row, snapshot.captured_at)
                            });
                            let ended_by_reference = end.map(|end| end <= reference);
                            if let Some(end) = end.filter(|_| cooldown_target.is_some()) {
                                *copies_by_end.entry(end).or_insert(0u64) += count;
                            }
                            match ended_by_reference {
                                Some(true) => ended += count,
                                Some(false) => pending += count,
                                None => unknown_end += count,
                            }
                            (end, ended_by_reference)
                        });
                let preferred =
                    preferred_link.is_some_and(|link| item["link"].as_str() == Some(link));
                preferred_match_count += usize::from(preferred);
                let page = if preferred {
                    &mut preferred_locations
                } else {
                    &mut locations
                };
                if matched_locations >= offset && page.len() < limit {
                    let mut location = json!({
                        "storage":storage,"container":container,"slot":slot,"count":count
                    });
                    if let Some(limit) = item.get("reported_stack_limit") {
                        location["reported_stack_limit"] = limit.clone();
                    }
                    if let Some(row) = cooldown {
                        location["captured_cooldown"] = row.clone();
                    }
                    if storage == "equipment" {
                        if let Some(condition) = durability.and_then(|slots| slots.get(slot)) {
                            location["observed_equipped_durability"] = condition.clone();
                        }
                        if let Some(enchants) = equipped_weapon_hand_enchants(snapshot, slot) {
                            location["observed_weapon_hand_enchants"] = enchants;
                        }
                    }
                    if let Some((end, ended_by_reference)) = projection {
                        location["projected_cooldown_end_at"] = json!(end);
                        location["projected_end_at_or_before_reference"] =
                            json!(ended_by_reference);
                    }
                    page.push(location);
                }
                matched_locations += 1;
                return count;
            }
            0
        };
        for (key, item) in section {
            if storage == "equipment" {
                record(None, key, item, 1);
            } else if let Some(slots) = item.as_object() {
                let mut container_total = 0;
                for (slot, observed) in slots {
                    if let Some(count) = observed["count"].as_u64() {
                        container_total += record(Some(key), slot, observed, count);
                    }
                }
                if include_container_quantities {
                    container_totals.insert(key, container_total);
                }
            }
        }
        if include_container_quantities && storage != "equipment" {
            container_quantities.insert(storage, Some(container_totals));
        }
        quantities.insert(storage, Some(quantity));
    }
    if !preferred_locations.is_empty() {
        preferred_locations.extend(
            locations
                .into_iter()
                .take(limit - preferred_locations.len()),
        );
        locations = preferred_locations;
    }
    let mut result = json!({
        "captured_at":snapshot.captured_at,"item_id":item_id,"live":false,
        "locations":locations,"matched_locations":matched_locations,
        "quantity_by_storage":quantities,"has_more":matched_locations.saturating_sub(offset)>limit
    });
    if preferred_link.is_some() {
        result["exact_saved_link_location_count"] = json!(preferred_match_count);
    }
    if include_container_quantities {
        result["quantity_by_container"] = json!(container_quantities);
    }
    if let Some(reference) = cooldown_reference {
        result["bag_cooldown_reference_at"] = json!(reference);
        result["bag_cooldown_quantity_projection"] = json!(quantities["bags"].map(|_| json!({
            "quantity_with_projected_end_at_or_before_reference":ended,
            "quantity_with_projected_end_after_reference":pending,
            "quantity_with_unknown_projected_end":unknown_end,
            "current_usability_verified":false,
        })));
    }
    if let Some(target) = cooldown_target {
        let target = target.get();
        result["bag_cooldown_target_projection"] = json!(quantities["bags"].map(|_| {
            let mut cumulative = 0u64;
            let earliest = copies_by_end.into_iter().find_map(|(end, copies)| {
                cumulative += copies;
                (cumulative >= target).then_some(end)
            });
            let covered = if ended >= target { Some(true) }
                else if ended + unknown_end < target { Some(false) } else { None };
            json!({"target_quantity":target,"projected_end_by_reference_covers_target":covered,
                "shortfall_at_reference_from_known_timers":target.saturating_sub(ended),
                "earliest_known_projected_end_covering_target":earliest,"current_usability_verified":false})
        }));
    }
    if cooldowns.is_some() {
        result["bag_cooldown_observation"] = json!({"basis":"same_snapshot_exact_bag_slot_item",
            "scope":"matching_bag_locations_before_pagination","reported_item_locations":reported,
            "unavailable_item_locations":unavailable,"disabled_item_locations":disabled,
            "positive_timer_item_locations":positive_timer,"current_usability_verified":false,
            "game_execution":false});
    }
    Ok(result)
}

pub fn observed_bag_stack_groups(
    snapshot: &Snapshot,
    item_id: u32,
    assumed_max_stack_size: Option<std::num::NonZeroU32>,
    use_reported_stack_limits: bool,
    storage: Option<crate::InventoryStorage>,
    container_id: Option<u8>,
    group_offset: Option<usize>,
) -> Option<Value> {
    let section = storage
        .unwrap_or(crate::InventoryStorage::Bags)
        .section_name();
    let bags = snapshot.sections.get(section)?.as_object()?;
    let selected = container_id.map(|id| id.to_string());
    if let Some(id) = &selected {
        bags.get(id)?;
    }
    let mut by_link = BTreeMap::<&str, Vec<(u32, u32, u64, Option<u64>)>>::new();
    let mut locations_without_link = 0;
    for (bag, slots) in bags {
        if selected.as_ref().is_some_and(|id| bag != id) {
            continue;
        }
        let bag = bag.parse().ok()?;
        for (slot, item) in slots.as_object()? {
            if item["item_id"].as_u64() != Some(u64::from(item_id)) {
                continue;
            }
            if let Some(link) = item["link"].as_str().filter(|link| !link.trim().is_empty()) {
                by_link.entry(link).or_default().push((
                    bag,
                    slot.parse().ok()?,
                    item["count"].as_u64()?,
                    item["reported_stack_limit"].as_u64(),
                ));
            } else {
                locations_without_link += 1;
            }
        }
    }
    let mut groups: Vec<_> = by_link
        .into_values()
        .filter(|rows| rows.len() > 1)
        .collect();
    for rows in &mut groups {
        rows.sort_unstable();
    }
    groups.sort_unstable_by_key(|rows| rows[0]);
    let assumed_maximum = assumed_max_stack_size.map(|size| u64::from(size.get()));
    let mut shown = Vec::new();
    let mut estimated_groups = 0;
    let mut maximum_slots_freed = 0;
    let mut disjoint_pairs = 0;
    let stack_location = |(container, slot, count, _)| {
        if section == "bags" {
            json!({"bag":container,"slot":slot,"count":count})
        } else {
            json!({"container":container,"slot":slot,"count":count})
        }
    };
    let offset = group_offset.unwrap_or(0);
    for (index, rows) in groups.iter().enumerate() {
        let visible = index >= offset && index - offset < 20;
        if !visible && assumed_maximum.is_none() && !use_reported_stack_limits {
            continue;
        }
        let reported_maximum = if use_reported_stack_limits {
            rows[0]
                .3
                .filter(|limit| rows.iter().all(|row| row.3 == Some(*limit)))
        } else {
            None
        };
        let maximum = assumed_maximum.or(reported_maximum);
        let quantity = rows.iter().map(|row| row.2).sum::<u64>();
        let minimum = maximum
            .filter(|maximum| rows.iter().all(|row| row.2 <= *maximum))
            .map(|maximum| quantity.div_ceil(maximum));
        let freed = minimum.map(|count| rows.len() as u64 - count);
        let mut pair_examples = Vec::new();
        let pairs = maximum.filter(|_| minimum.is_some()).map(|maximum| {
            let mut by_count = rows.clone();
            by_count.sort_unstable_by_key(|&(bag, slot, count, _)| (count, bag, slot));
            let (mut low, mut high) = (0, by_count.len() - 1);
            let mut pairs = 0;
            while low < high {
                let combined = by_count[low].2 + by_count[high].2;
                if combined <= maximum {
                    if visible && pair_examples.len() < 5 {
                        pair_examples.push(json!({"stacks":[stack_location(by_count[low]),
                            stack_location(by_count[high])],"combined_count":combined}));
                    }
                    pairs += 1;
                    low += 1;
                }
                high -= 1;
            }
            pairs
        });
        disjoint_pairs += pairs.unwrap_or(0);
        if let Some(freed) = freed {
            estimated_groups += 1;
            maximum_slots_freed += freed;
        }
        if !visible {
            continue;
        }
        let locations: Vec<_> = rows.iter().take(10).copied().map(stack_location).collect();
        let mut group = json!({"stack_count":rows.len(),"total_quantity":quantity,
            "locations":locations,"locations_truncated":rows.len()>locations.len()});
        if maximum.is_some() {
            group["capacity_projection"] = json!({"observed_counts_fit_assumption":minimum.is_some(),
                "minimum_stacks_under_assumption":minimum,
                "maximum_slots_freed_under_assumption":freed,
                "maximum_disjoint_whole_stack_pairs":pairs,
                "whole_stack_pair_examples_truncated":pairs.map(|count| count>pair_examples.len()),
                "whole_stack_pair_examples":pairs.map(|_| pair_examples)});
        }
        if use_reported_stack_limits {
            group["reported_stack_limit"] = json!(reported_maximum);
            group["reported_stack_limit_status"] = json!(if reported_maximum.is_some() {
                "agreed"
            } else if rows.iter().any(|row| row.3.is_none()) {
                "incomplete"
            } else {
                "conflicting"
            });
        }
        shown.push(group);
    }
    let mut report = json!({"group_count":groups.len(),"groups":shown,"groups_truncated":groups.len()>shown.len(),
        "locations_without_link":locations_without_link,
        "stack_compatibility_verified":false,"game_execution":false});
    if storage.is_some() || container_id.is_some() {
        report["storage"] = json!(section);
    }
    if let Some(id) = container_id {
        report["container_id"] = json!(id);
    }
    if group_offset.is_some() {
        report["group_offset"] = json!(offset);
        report["has_more_groups"] = json!(groups.len().saturating_sub(offset) > shown.len());
    }
    if let Some(maximum) = assumed_maximum {
        report["assumed_max_stack_size"] = json!(maximum);
        report["capacity_projection_summary"] = json!({
            "estimated_group_count":estimated_groups,
            "inconsistent_group_count":groups.len()-estimated_groups,
            "maximum_slots_freed_across_estimated_groups":maximum_slots_freed,
            "maximum_disjoint_whole_stack_pairs_across_estimated_groups":disjoint_pairs
        });
    }
    if use_reported_stack_limits {
        report["reported_capacity_projection_summary"] = json!({
            "estimated_group_count":estimated_groups,
            "unestimated_group_count":groups.len()-estimated_groups,
            "maximum_slots_freed_across_estimated_groups":maximum_slots_freed,
            "maximum_disjoint_whole_stack_pairs_across_estimated_groups":disjoint_pairs
        });
    }
    Some(report)
}

fn compare_item(
    before: &Snapshot,
    after: &Snapshot,
    item_id: u32,
    by_container: bool,
) -> Result<Value> {
    let before =
        find_item_with_container_quantities(before, item_id, 0, 1, by_container, None, None)?;
    let after =
        find_item_with_container_quantities(after, item_id, 0, 1, by_container, None, None)?;
    let quantities = after["quantity_by_storage"].as_object().unwrap();
    let mut net_delta = Some(0i64);
    let mut by_storage = BTreeMap::new();
    for (storage, current) in quantities {
        let previous = &before["quantity_by_storage"][storage];
        let delta = previous.as_i64().zip(current.as_i64()).map(|(a, b)| b - a);
        net_delta = net_delta.zip(delta).map(|(total, change)| total + change);
        by_storage.insert(
            storage,
            json!({"before":previous,"after":current,"delta":delta}),
        );
    }
    let mut result = json!({"item_id":item_id,"by_storage":by_storage,"net_delta":net_delta});
    if by_container {
        let mut changes = serde_json::Map::new();
        for storage in quantities
            .keys()
            .filter(|name| name.as_str() != "equipment")
        {
            let previous = before["quantity_by_container"][storage].as_object();
            let current = after["quantity_by_container"][storage].as_object();
            let ids: BTreeSet<_> = previous
                .into_iter()
                .chain(current)
                .flat_map(|map| map.keys())
                .collect();
            let entries: BTreeMap<_, _> = ids
                .into_iter()
                .map(|id| {
                    let a = previous.and_then(|map| map.get(id)).and_then(Value::as_i64);
                    let b = current.and_then(|map| map.get(id)).and_then(Value::as_i64);
                    (
                        id,
                        json!({"before":a,"after":b,"delta":a.zip(b).map(|(a,b)| b-a)}),
                    )
                })
                .collect();
            changes.insert(storage.clone(), json!({
                "before_observed":previous.is_some(),"after_observed":current.is_some(),"containers":entries,
            }));
        }
        result["by_container"] = json!(changes);
    }
    Ok(result)
}

#[derive(Clone, Copy)]
pub enum RestockStackLimitSource {
    Supplied,
    ReportedBags,
    ReportedBagsAndBanks,
}

pub fn reported_inventory_stack_limits(
    snapshot: &Snapshot,
    include_banks: bool,
) -> BTreeMap<u32, std::num::NonZeroU32> {
    let mut limits = BTreeMap::new();
    let storages = ["bags", "character_bank", "account_bank"];
    for storage in storages.into_iter().take(if include_banks { 3 } else { 1 }) {
        let Some(bags) = snapshot.sections.get(storage).and_then(Value::as_object) else {
            continue;
        };
        for item in bags
            .values()
            .filter_map(Value::as_object)
            .flat_map(|slots| slots.values())
        {
            let Some(id) = item["item_id"]
                .as_u64()
                .and_then(|id| u32::try_from(id).ok())
            else {
                continue;
            };
            let limit = item["reported_stack_limit"]
                .as_u64()
                .and_then(|limit| u32::try_from(limit).ok())
                .and_then(std::num::NonZeroU32::new);
            limits
                .entry(id)
                .and_modify(|previous| {
                    if *previous != limit {
                        *previous = None;
                    }
                })
                .or_insert(limit);
        }
    }
    limits
        .into_iter()
        .filter_map(|(id, limit)| Some((id, limit?)))
        .collect()
}

pub fn restock_new_stack_space(
    snapshot: &Snapshot,
    review: &Value,
    stack_limits: &BTreeMap<u32, std::num::NonZeroU32>,
    merchant_quote: Option<&Value>,
    limit_source: RestockStackLimitSource,
) -> Option<Value> {
    if review["storage_observed"] != true {
        return None;
    }
    let mut bag_stacks = BTreeMap::<u32, (u64, u64)>::new();
    if let Some(bags) = snapshot.sections.get("bags").and_then(Value::as_object) {
        for item in bags
            .values()
            .filter_map(Value::as_object)
            .flat_map(|slots| slots.values())
        {
            if let Some((id, count)) = item["item_id"]
                .as_u64()
                .and_then(|id| u32::try_from(id).ok())
                .zip(item["count"].as_u64())
                && let Some(limit) = stack_limits.get(&id)
            {
                let (largest, headroom) = bag_stacks.entry(id).or_default();
                *largest = (*largest).max(count);
                *headroom += u64::from(limit.get()).saturating_sub(count);
            }
        }
    }
    let quoted_surplus: BTreeMap<_, _> = merchant_quote
        .and_then(|quote| quote["reagents"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|row| {
            Some((
                row["item_id"].as_u64()?,
                row["lowest_cost_copper_only_bundle_mix"]["extra_units"].as_u64()?,
            ))
        })
        .collect();
    let residual: BTreeMap<_, _> = review["bank_withdrawal_plan"]["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            Some((
                row["item_id"].as_u64()?,
                row["remaining_after_observed_withdrawals"].as_u64()?,
            ))
        })
        .collect();
    let mut total = Some(0u64);
    let mut merged_total = Some(0u64);
    let mut items = Vec::new();
    for row in review["items"].as_array()? {
        let shortfall = row["shortfall_vs_observed"].as_u64()?;
        if shortfall == 0 {
            continue;
        }
        let item_id = u32::try_from(row["item_id"].as_u64()?).ok()?;
        let limit = stack_limits.get(&item_id).copied();
        let stack = bag_stacks.get(&item_id).copied();
        let largest = stack.map(|(largest, _)| largest);
        let contradicted = limit
            .zip(largest)
            .map(|(limit, largest)| largest > u64::from(limit.get()));
        let incoming = if merchant_quote.is_some() && residual.get(&u64::from(item_id)) != Some(&0)
        {
            quoted_surplus
                .get(&u64::from(item_id))
                .and_then(|extra| shortfall.checked_add(*extra))
        } else {
            Some(shortfall)
        };
        let slots = limit
            .filter(|_| contradicted != Some(true))
            .zip(incoming)
            .map(|(limit, quantity)| quantity.div_ceil(u64::from(limit.get())));
        total = total
            .zip(slots)
            .and_then(|(sum, slots)| sum.checked_add(slots));
        let headroom = limit
            .filter(|_| contradicted != Some(true))
            .map(|_| stack.map_or(0, |(_, headroom)| headroom));
        let merged_slots =
            limit
                .zip(headroom)
                .zip(incoming)
                .map(|((limit, headroom), quantity)| {
                    quantity
                        .saturating_sub(headroom)
                        .div_ceil(u64::from(limit.get()))
                });
        merged_total = merged_total
            .zip(merged_slots)
            .and_then(|(sum, slots)| sum.checked_add(slots));
        let mut item = json!({"item_id":item_id,"bag_shortfall":shortfall,
            "assumed_stack_limit":limit,"largest_observed_bag_stack":largest,
            "assumption_below_observed_stack":contradicted,"new_stack_slots":slots,
            "assumed_same_item_stack_headroom":headroom,"new_stack_slots_if_same_item_stacks_merge":merged_slots});
        if merchant_quote.is_some() {
            item["incoming_quantity_with_quoted_bundles"] = json!(incoming);
        }
        items.push(item);
    }
    let general_free =
        carried_bag_space(snapshot).and_then(|space| space["known_general_free_slots"].as_u64());
    Some(json!({
        "basis":match (merchant_quote.is_some(), limit_source) {
            (true,RestockStackLimitSource::ReportedBagsAndBanks) => "bank_and_quoted_bundles_in_new_stacks_using_reported_inventory_limits",
            (false,RestockStackLimitSource::ReportedBagsAndBanks) => "requested_shortfall_in_new_stacks_using_reported_inventory_limits",
            (true,RestockStackLimitSource::ReportedBags) => "bank_and_quoted_bundles_in_new_stacks_using_reported_bag_limits",
            (false,RestockStackLimitSource::ReportedBags) => "requested_shortfall_in_new_stacks_using_reported_bag_limits",
            (true,RestockStackLimitSource::Supplied) => "bank_and_quoted_bundles_in_new_stacks_using_supplied_limits",
            (false,RestockStackLimitSource::Supplied) => "requested_shortfall_in_new_stacks_using_supplied_limits",
        },
        "items":items,"new_stack_slots":total,"observed_general_free_slots":general_free,
        "fits_observed_general_slots_under_assumptions":total.zip(general_free)
            .map(|(needed, free)| needed <= free),
        "new_stack_slots_if_same_item_stacks_merge":merged_total,
        "fits_observed_general_slots_if_same_item_stacks_merge":merged_total.zip(general_free)
            .map(|(needed, free)| needed <= free),
        "same_item_stack_merge_verified":false,
        "stack_limits_verified":false,"placement_verified":false,"live":false,
    }))
}

pub(crate) fn carried_bag_space(snapshot: &Snapshot) -> Option<Value> {
    let bags = snapshot.sections.get("bag_capacity")?.as_object()?;
    let mut total = 0;
    let mut free = 0;
    let mut by_family = BTreeMap::new();
    let mut unknown = 0;
    for bag in bags.values() {
        let slots = bag["free_slots"].as_u64()?;
        total += bag["total_slots"].as_u64()?;
        free += slots;
        if let Some(mask) = bag["family_mask"].as_u64() {
            *by_family.entry(mask).or_insert(0) += slots;
        } else {
            unknown += slots;
        }
    }
    Some(json!({"total_slots":total,"free_slots":free,
        "known_general_free_slots":by_family.get(&0).copied().unwrap_or_default(),
        "free_slots_by_family_mask":by_family,"unknown_family_free_slots":unknown}))
}

pub(crate) fn pet_food_locations(snapshot: &Snapshot) -> Option<Value> {
    let foods = snapshot.sections.get("pet_care")?["carried_food"].as_object()?;
    let bags = snapshot.sections.get("bags")?.as_object()?;
    let mut locations = Vec::new();
    let mut matching_stacks = 0usize;
    let mut bag_totals = BTreeMap::<String, (u64, usize)>::new();
    for (bag, slots) in bags {
        for (slot, item) in slots.as_object()? {
            let id = item["item_id"].as_u64()?;
            let key = id.to_string();
            if foods.contains_key(&key) {
                let (quantity, stacks) = bag_totals.entry(key).or_default();
                *quantity = quantity.checked_add(item["count"].as_u64()?)?;
                *stacks += 1;
                matching_stacks += 1;
                if locations.len() < 50 {
                    locations
                        .push(json!({"bag":bag,"slot":slot,"item_id":id,"count":item["count"]}));
                }
            }
        }
    }
    let quantities: BTreeMap<_, _> = bag_totals
        .into_iter()
        .map(|(id, (quantity, stacks))| {
            (
                id,
                json!({"observed_bag_quantity":quantity,"matching_stacks":stacks}),
            )
        })
        .collect();
    Some(json!({
        "matching_stacks":matching_stacks,
        "quantity_by_item_id":quantities,
        "locations_truncated":matching_stacks>locations.len(),
        "locations":locations
    }))
}

fn carried_items(snapshot: &Snapshot) -> Option<Value> {
    let bags = snapshot.sections.get("bags")?.as_object()?;
    let mut groups: BTreeMap<u64, (u64, usize, Vec<Value>)> = BTreeMap::new();
    let mut total = 0u64;
    for (bag, slots) in bags {
        for (slot, item) in slots.as_object()? {
            let id = item.get("item_id")?.as_u64()?;
            let count = item.get("count")?.as_u64()?;
            total = total.checked_add(count)?;
            let group = groups.entry(id).or_default();
            group.0 = group.0.checked_add(count)?;
            group.1 += 1;
            if group.2.len() < 20 {
                group.2.push(json!({"bag":bag,"slot":slot,"count":count}));
            }
        }
    }
    let repeated_count = groups.values().filter(|group| group.1 > 1).count();
    let repeated: Vec<_> = groups
        .iter()
        .filter(|(_, group)| group.1 > 1)
        .take(20)
        .map(|(id, group)| {
            json!({"item_id":id,"quantity_observed":group.0,
            "occupied_slots_observed":group.1,"locations":group.2,
            "locations_truncated":group.1>group.2.len()})
        })
        .collect();
    Some(
        json!({"quantity_observed":total,"distinct_item_ids_observed":groups.len(),
        "repeated_item_id_count":repeated_count,"repeated_item_ids":repeated,
        "groups_truncated":repeated_count>repeated.len(),
        "grouping":"item_id_only_not_item_identity",
        "stack_compatibility":"unknown",
        "free_slots":if snapshot.sections.contains_key("bag_capacity") {"captured"} else {"not_captured"},
        "game_execution":false}),
    )
}

pub fn compare(before: &Snapshot, after: &Snapshot) -> Result<Value> {
    compare_with_item(before, after, None)
}

pub fn compare_with_item(
    before: &Snapshot,
    after: &Snapshot,
    item_id: Option<u32>,
) -> Result<Value> {
    compare_observations(
        before,
        after,
        ComparisonSelectors {
            item_id,
            ..ComparisonSelectors::default()
        },
    )
}

#[derive(Default)]
pub struct ComparisonSelectors<'a> {
    pub item_container_changes: bool,
    pub profession_spell_changes: bool,
    pub player_resource_changes: bool,
    pub player_aura_changes: bool,
    pub weapon_enchant_changes: bool,
    pub pet_action_bar_changes: bool,
    pub bag_cooldown_item_id: Option<std::num::NonZeroU32>,
    pub merchant_buyback_changes: bool,
    pub merchant_offer_item_id: Option<std::num::NonZeroU32>,
    pub merchant_offer_currency_id: Option<std::num::NonZeroU32>,
    pub trainer_lesson_name: Option<&'a str>,
    pub flight_destination_node_id: Option<std::num::NonZeroU32>,
    pub flight_wallet_reserve_copper: Option<u64>,
    pub repair_quote_changes: bool,
    pub equipment_durability_max_percent: Option<u8>,
    pub player_aura_spell_id: Option<std::num::NonZeroU32>,
    pub group_context_changes: bool,
    pub owned_auction_changes: bool,
    pub battlefield_queue_changes: bool,
    pub player_life_state_changes: bool,
    pub instance_group_size_reference: bool,
    pub instance_group_size_max_cache_age_seconds: Option<u32>,
    pub group_member_target: Option<std::num::NonZeroU32>,
    pub player_pvp_progression_changes: bool,
    pub pvp_target_rank: Option<u32>,
    pub player_trade_offer_changes: bool,
    pub player_trade_offer_item_id: Option<std::num::NonZeroU32>,
    pub player_trade_offer_compare_carried: bool,
    pub item_id: Option<u32>,
    pub currency_id: Option<u32>,
    pub currency_name: Option<&'a str>,
    pub faction_id: Option<u32>,
    pub faction_name: Option<&'a str>,
    pub quest_id: Option<u32>,
    pub quest_title: Option<&'a str>,
    pub quest_waypoints: bool,
    pub quest_readiness_changes: bool,
    pub quest_objective_changes: bool,
    pub achievement_status_changes: bool,
    pub selected_recipe_changes: bool,
    pub selected_recipe_preparation_crafts: Option<u32>,
    pub mount_collection_changes: bool,
    pub toy_view_changes: bool,
    pub saved_instance_changes: bool,
    pub instance_lock_warning_changes: bool,
    pub saved_world_boss_changes: bool,
    pub profession_recipe_changes: bool,
    pub recipe_cooldown_changes: bool,
    pub loot_window_changes: bool,
    pub gossip_option_changes: bool,
    pub gossip_quest_changes: bool,
    pub pet_view_changes: bool,
    pub mail_attachment_changes: bool,
    pub mail_attachment_item_id: Option<std::num::NonZeroU32>,
    pub mail_attachment_compare_carried_item: bool,
    pub mail_attachment_currency_id: Option<std::num::NonZeroU32>,
    pub mail_attachment_compare_currency_balance: bool,
    pub achievement_id: Option<u32>,
    pub achievement_name: Option<&'a str>,
    pub profession_skill_line: Option<u32>,
    pub profession_target_rank: Option<std::num::NonZeroU32>,
    pub wallet: bool,
    pub experience: bool,
    pub spellbook: bool,
    pub action_bar_slots: bool,
    pub action_bar_spell_id: Option<std::num::NonZeroU32>,
    pub action_bar_item_id: Option<std::num::NonZeroU32>,
}

pub(crate) fn observed_trade_offer_quantities(section: &Value) -> (usize, BTreeMap<u32, u64>) {
    let rows = section["entries"].as_object().unwrap();
    let mut quantities = BTreeMap::new();
    let mut items = 0;
    for row in rows.values().filter(|row| row["status"] == "item") {
        items += 1;
        *quantities
            .entry(row["item_id"].as_u64().unwrap() as u32)
            .or_default() += row["quantity"].as_u64().unwrap();
    }
    (items, quantities)
}

/// Compare expected quantities against the validated saved outgoing offer.
pub fn review_trade_offer(section: &Value, requirements: &[LootRequirement]) -> Value {
    if section.is_null() {
        return Value::Null;
    }
    let (slots, mut quantities) = observed_trade_offer_quantities(section);
    let unreadable_slots = 6 - slots;
    let mut short_item_count = 0;
    let mut minimum_additional_snapshot_slots = 0;
    let mut excess = false;
    let items: Vec<_> = requirements
        .iter()
        .map(|row| {
            let expected = row.quantity.get();
            let observed = quantities.remove(&row.item_id.get()).unwrap_or_default();
            let shortfall = expected.saturating_sub(observed);
            let minimum_slots = shortfall.div_ceil(u64::from(u32::MAX));
            short_item_count += usize::from(shortfall > 0);
            minimum_additional_snapshot_slots += minimum_slots;
            excess |= observed > expected;
            json!({"item_id":row.item_id,"expected_quantity":expected,
                "observed_quantity":observed,"observed_shortfall":shortfall,
                "observed_excess":observed.saturating_sub(expected),
                "minimum_additional_snapshot_slots":minimum_slots})
        })
        .collect();
    // Each unclassified normal slot can cover a shortfall for at most one item ID.
    let exact_match = if excess || !quantities.is_empty() || short_item_count > unreadable_slots {
        Some(false)
    } else {
        (unreadable_slots == 0).then_some(true)
    };
    let snapshot_slots_can_fit_shortfalls =
        minimum_additional_snapshot_slots <= unreadable_slots as u64;
    json!({"items":items,"unexpected_observed_quantities":quantities,
        "unreadable_or_empty_slots":unreadable_slots,"short_item_id_count":short_item_count,
        "minimum_additional_snapshot_slots":minimum_additional_snapshot_slots,
        "snapshot_slots_can_fit_shortfalls":snapshot_slots_can_fit_shortfalls,
        "exact_match_possible_in_snapshot_format":exact_match != Some(false) && snapshot_slots_can_fit_shortfalls,
        "captured_offer_matches_expected":exact_match,
        "current_offer_verified":false,"trade_completion_verified":false,"game_execution":false})
}

pub fn trade_offer_carried_comparison(
    snapshot: &Snapshot,
    requirements: &[LootRequirement],
) -> Result<Value> {
    let mut rows = Vec::with_capacity(requirements.len());
    for requirement in requirements {
        let id = requirement.item_id.get();
        let expected = requirement.quantity.get();
        let found = find_item(snapshot, id, 0, 1)?;
        let carried = found["quantity_by_storage"]["bags"].as_u64();
        rows.push(json!({"item_id":id,"expected_quantity":expected,"observed_carried_quantity":carried,
            "shortfall_vs_observed_carried":carried.map(|quantity| expected.saturating_sub(quantity))}));
    }
    Ok(json!({"scope":"carried_stock_only","items":rows,
        "offer_and_carried_overlap_verified":false,"trade_eligibility_verified":false,"live":false}))
}

pub(crate) fn remaining_pvp_points_to_next_threshold(section: &Value) -> Option<u64> {
    let points = section["rank_points"].as_u64()?;
    let threshold = section["next_rank_threshold"].as_u64()?;
    (threshold > 0).then_some(threshold)?.checked_sub(points)
}

const PVP_CAP_FIELDS: [&str; 3] = [
    "max_rank",
    "current_week_max_rank",
    "previous_week_max_rank",
];

pub fn instance_group_size_reference(snapshot: &Snapshot, maximum_cache_age: Option<u32>) -> Value {
    let instance = snapshot.sections.get("instance_context");
    let is_raid = match instance.and_then(|row| row["kind"].as_str()) {
        Some("raid") => Some(true),
        Some("party") => Some(false),
        _ => None,
    };
    let cache = snapshot.sections.get("saved_instances");
    let cache_age = cache.and_then(|row| row["update_age_seconds"].as_u64());
    let within_limit = maximum_cache_age
        .zip(cache_age)
        .map(|(maximum, age)| age <= u64::from(maximum));
    let matches = is_raid
        .zip(instance)
        .zip(cache)
        .and_then(|((is_raid, instance), cache)| {
            Some(
                cache["entries"]
                    .as_object()?
                    .values()
                    .filter(|row| {
                        row["name"] == instance["name"]
                            && row["difficulty_id"] == instance["difficulty_id"]
                            && row["is_raid"].as_bool() == Some(is_raid)
                    })
                    .collect::<Vec<_>>(),
            )
        });
    let size = matches
        .as_deref()
        .filter(|_| maximum_cache_age.is_none() || within_limit == Some(true))
        .and_then(|matches| match matches {
            [row] => row["max_players"].as_u64().filter(|size| *size > 0),
            _ => None,
        });
    let count = snapshot
        .sections
        .get("group_context")
        .and_then(|section| section["categories"]["instance"]["reported_member_count"].as_u64());
    let mut report = json!({
        "matching_saved_lockout_rows":matches.as_ref().map(Vec::len),
        "saved_lockout_cache_age_seconds":cache_age,
        "reported_size_reference":size,"reported_instance_group_count":count,
        "shortfall_to_size_reference":size.zip(count).map(|(size, count)| size.saturating_sub(count)),
        "excess_over_size_reference":size.zip(count).map(|(size, count)| count.saturating_sub(size)),
        "instance_identity_verified":false,"instance_capacity_verified":false,
        "current_group_verified":false,"live":false
    });
    if let Some(maximum) = maximum_cache_age {
        report["maximum_cache_age_seconds_at_capture"] = json!(maximum);
        report["cache_age_within_limit_at_capture"] = json!(within_limit);
    }
    report
}

fn compare_instance_group_size_references(
    before: &Snapshot,
    after: &Snapshot,
    maximum_cache_age: Option<u32>,
) -> Value {
    let identity = |snapshot: &Snapshot| {
        let row = snapshot.sections.get("instance_context")?;
        Some((
            row["kind"].as_str()?.to_owned(),
            row["name"].as_str()?.to_owned(),
            row["difficulty_id"].as_u64()?,
            row["instance_map_id"].as_u64()?,
        ))
    };
    let same_context = identity(before)
        .zip(identity(after))
        .map(|(old, new)| old == new);
    let old = instance_group_size_reference(before, maximum_cache_age);
    let new = instance_group_size_reference(after, maximum_cache_age);
    let same_size = old["reported_size_reference"]
        .as_u64()
        .zip(new["reported_size_reference"].as_u64())
        .map(|(old, new)| old == new);
    let delta = |field: &str| {
        if same_context != Some(true) || same_size != Some(true) {
            return None;
        }
        Some(i128::from(new[field].as_u64()?) - i128::from(old[field].as_u64()?))
    };
    json!({"before":old,"after":new,"reported_instance_context_matches":same_context,
        "reported_size_references_match":same_size,
        "shortfall_to_size_reference_delta":delta("shortfall_to_size_reference"),
        "excess_over_size_reference_delta":delta("excess_over_size_reference"),
        "instance_identity_verified":false,"group_identity_verified":false,"live":false})
}

pub fn group_member_target_projection(section: &Value, target: u32) -> Value {
    let target = u64::from(target);
    let categories = ["home", "instance"]
        .map(|category| {
            let count = section["categories"][category]["reported_member_count"].as_u64();
            (
                category,
                json!({"reported_member_count":count,
            "shortfall":count.map(|count| target.saturating_sub(count)),
            "excess":count.map(|count| count.saturating_sub(target)),
            "target_met_at_capture":count.map(|count| count >= target)}),
            )
        })
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    json!({"target_reported_member_count":target,"categories":categories,
        "complete_roster_verified":false,"current_group_verified":false,"live":false})
}

fn compare_group_member_target(before: &Snapshot, after: &Snapshot, target: u32) -> Value {
    let project = |snapshot: &Snapshot| {
        group_member_target_projection(
            snapshot
                .sections
                .get("group_context")
                .unwrap_or(&Value::Null),
            target,
        )
    };
    let old = project(before);
    let new = project(after);
    let categories = ["home", "instance"]
        .map(|category| {
            let before = &old["categories"][category];
            let after = &new["categories"][category];
            let met = before["target_met_at_capture"]
                .as_bool()
                .zip(after["target_met_at_capture"].as_bool());
            (
                category,
                json!({"before":before,"after":after,
            "crossed_up":met.map(|(old, new)| !old && new),
            "crossed_down":met.map(|(old, new)| old && !new)}),
            )
        })
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    json!({"target_reported_member_count":target,"categories":categories,
        "group_identity_verified":false,"membership_events_verified":false,"live":false})
}

pub fn pvp_rank_target_projection(section: &Value, target: u32) -> Option<Value> {
    let rank = section["rank"].as_u64()?;
    let points = section["rank_points"].as_u64()?;
    let totals = &section["total_points_by_rank"];
    let target_total = totals[target.to_string()].as_u64();
    let current_total = totals[rank.to_string()]
        .as_u64()
        .and_then(|base| base.checked_add(points));
    let total_distance = current_total
        .zip(target_total.filter(|threshold| *threshold > 0))
        .map(|(current, threshold)| threshold.saturating_sub(current));
    let next_distance = (rank.checked_add(1) == Some(u64::from(target)))
        .then(|| remaining_pvp_points_to_next_threshold(section))
        .flatten();
    let caps: BTreeMap<_, _> = PVP_CAP_FIELDS
        .into_iter()
        .map(|field| {
            (
                field,
                section[field].as_u64().map(|cap| u64::from(target) > cap),
            )
        })
        .collect();
    Some(json!({"target_rank":target,"captured_rank":rank,
        "captured_rank_at_or_above_target":rank >= u64::from(target),
        "reported_total_rank_points":current_total,"reported_target_total_points":target_total,
        "remaining_points_using_total_threshold":total_distance,
        "remaining_points_using_next_rank_threshold":next_distance,
        "target_above_reported_caps":caps,"rank_eligibility_verified":false,"live":false}))
}

pub(crate) fn player_pvp_progression_observation(section: &Value) -> Value {
    let mut report = section.clone();
    report["remaining_points_to_reported_next_threshold"] =
        json!(remaining_pvp_points_to_next_threshold(section));
    if let Some(totals) = section.get("total_points_by_rank") {
        let lookup = |field: &str| {
            section[field]
                .as_u64()
                .and_then(|rank| totals[rank.to_string()].as_u64())
        };
        let current = lookup("rank").map(|base| base + section["rank_points"].as_u64().unwrap());
        report["reported_total_rank_points"] = json!(current);
        let mut caps = serde_json::Map::new();
        for field in PVP_CAP_FIELDS {
            let remaining = current
                .zip(lookup(field))
                .filter(|(_, cap)| *cap > 0)
                .map(|(current, cap)| cap.saturating_sub(current));
            caps.insert(field.to_string(), json!(remaining));
        }
        report["remaining_points_to_reported_caps"] = Value::Object(caps);
        report["cap_distance_basis"] =
            json!("captured_total_thresholds_and_within_rank_points_only");
    }
    report["current_progression_verified"] = json!(false);
    report["rank_eligibility_verified"] = json!(false);
    report["reward_eligibility_verified"] = json!(false);
    report["game_execution"] = json!(false);
    report
}

fn resolve_comparison_id(
    before: &Snapshot,
    after: &Snapshot,
    section: &str,
    entries_key: Option<&str>,
    label_key: &str,
    id: Option<u32>,
    name: Option<&str>,
) -> Result<Option<u32>> {
    let Some(name) = name.map(str::trim) else {
        return Ok(id);
    };
    ensure!(
        before.client.family == crate::Family::Forever && !name.is_empty() && name.len() <= 256,
        "Named comparison requires Forever and a valid name"
    );
    let name = name.to_lowercase();
    let ids: BTreeSet<_> = [before, after]
        .into_iter()
        .filter_map(|snapshot| {
            let section = snapshot.sections.get(section)?;
            match entries_key {
                Some(key) => section.get(key)?.as_object(),
                None => section.as_object(),
            }
        })
        .flat_map(|entries| entries.iter())
        .filter_map(|(key, row)| {
            let matched = key.parse::<u32>().ok()?;
            (id.is_none_or(|wanted| matched == wanted)
                && row[label_key]
                    .as_str()
                    .is_some_and(|label| label.trim().to_lowercase() == name))
            .then_some(matched)
        })
        .collect();
    ensure!(ids.len() == 1, "Name is absent or ambiguous");
    Ok(ids.first().copied())
}

pub fn compare_observations(
    before: &Snapshot,
    after: &Snapshot,
    selectors: ComparisonSelectors<'_>,
) -> Result<Value> {
    ensure_comparable(before, after)?;
    ensure!(
        !selectors.item_container_changes || selectors.item_id.is_some(),
        "Item container changes require an item ID"
    );
    let names: BTreeSet<_> = before
        .sections
        .keys()
        .chain(after.sections.keys())
        .chain(before.unavailable.keys())
        .chain(after.unavailable.keys())
        .collect();
    let mut sections = serde_json::Map::new();
    for name in names {
        let result = match (before.sections.get(name), after.sections.get(name)) {
            (Some(a), Some(b)) => {
                let mut paths = Vec::new();
                let mut count = 0;
                diff(a, b, "", &mut paths, &mut count);
                json!({"status":if count==0 {"unchanged"} else {"changed"},"changed_paths":paths,"change_count":count,"truncated":count>paths.len()})
            }
            _ => {
                json!({"status":"not_comparable",
                    "before_available":before.sections.contains_key(name),
                    "after_available":after.sections.contains_key(name),
                    "before_capture_status":before.section_capture_status(name),
                    "after_capture_status":after.section_capture_status(name)})
            }
        };
        sections.insert(name.clone(), result);
    }
    let mut comparison = json!({"client":after.client,"character":after.character,
        "before_captured_at":before.captured_at,"after_captured_at":after.captured_at,
        "live":false,"sections":sections});
    if before.client.family == crate::Family::Forever {
        let changes: Vec<_> = crate::SNAPSHOT_SECTIONS
            .iter()
            .filter_map(|name| {
                let old_status = before.section_capture_status(name);
                let new_status = after.section_capture_status(name);
                let scope = |snapshot: &'_ Snapshot| {
                    snapshot
                        .sections
                        .get(*name)
                        .and_then(|section| section["coverage"].as_str())
                        .map(str::to_owned)
                };
                let old_scope = scope(before);
                let new_scope = scope(after);
                (old_status != new_status || old_scope != new_scope).then(|| json!({
                "section":name,"before_status":old_status,"after_status":new_status,
                "before_reported_scope":old_scope,"after_reported_scope":new_scope,
                "reported_scope_changed":old_scope.zip(new_scope).map(|(old,new)| old != new)
            }))
            })
            .collect();
        comparison["capture_coverage_changes"] = json!(changes);
    }
    ensure!(
        !selectors.player_trade_offer_compare_carried
            || selectors.player_trade_offer_item_id.is_some(),
        "Carried comparison requires an exact outgoing trade item"
    );
    if selectors.player_trade_offer_changes || selectors.player_trade_offer_item_id.is_some() {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Trade offer comparison requires Forever"
        );
        let before_offer = before.sections.get("player_trade_offer");
        let after_offer = after.sections.get("player_trade_offer");
        let old = before_offer.map(observed_trade_offer_quantities);
        let new = after_offer.map(observed_trade_offer_quantities);
        let ids: BTreeSet<_> = selectors.player_trade_offer_item_id.map_or_else(
            || {
                old.iter()
                    .chain(new.iter())
                    .flat_map(|(_, quantities)| quantities.keys().copied())
                    .collect()
            },
            |id| BTreeSet::from([id.get()]),
        );
        let quantity = |side: &Option<(usize, BTreeMap<u32, u64>)>, id| {
            let (slots, quantities) = side.as_ref()?;
            quantities
                .get(&id)
                .copied()
                .or_else(|| (*slots == 6).then_some(0))
        };
        let fully_read = |side: &Option<(usize, BTreeMap<u32, u64>)>| {
            side.as_ref().is_some_and(|(slots, _)| *slots == 6)
        };
        let items: Vec<_> = ids.into_iter().map(|id| {
            let before_quantity = quantity(&old, id);
            let after_quantity = quantity(&new, id);
            let delta = if fully_read(&old) && fully_read(&new) {
                before_quantity.zip(after_quantity).map(|(a,b)| b as i64 - a as i64)
            } else { None };
            json!({"item_id":id,"before_observed_quantity":before_quantity,"after_observed_quantity":after_quantity,"quantity_delta":delta})
        }).collect();
        comparison["player_trade_offer_changes"] = json!({"items":items,
            "before_readable_item_slots":old.as_ref().map(|(slots, _)| *slots),
            "after_readable_item_slots":new.as_ref().map(|(slots, _)| *slots),
            "basis":"captured_normal_offer_slots_by_base_item_id",
            "same_trade_verified":false,"trade_completion_verified":false,"live":false});
        if selectors.player_trade_offer_item_id.is_none() {
            let changes = before_offer.zip(after_offer).map(|(before, after)| {
                (1..=6)
                    .filter_map(|slot| {
                        let key = slot.to_string();
                        let old = &before["entries"][&key];
                        let new = &after["entries"][&key];
                        (old["status"] == "item" && new["status"] == "item" && old != new).then(
                            || {
                                json!({"slot":slot,
                                "before":{"item_id":old["item_id"],"quantity":old["quantity"]},
                                "after":{"item_id":new["item_id"],"quantity":new["quantity"]}})
                            },
                        )
                    })
                    .collect::<Vec<_>>()
            });
            comparison["player_trade_offer_changes"]["readable_slot_changes"] = json!(changes);
        }
        if let Some(id) = selectors
            .player_trade_offer_item_id
            .filter(|_| selectors.player_trade_offer_compare_carried)
        {
            let mut carried = compare_carried_item_quantities(before, after, id.get())?;
            carried["offers_count_toward_carried_stock"] = json!(false);
            comparison["player_trade_offer_changes"]["carried_comparison"] = carried;
        }
    }
    if let Some(target) = selectors.pvp_target_rank {
        ensure!(
            before.client.family == crate::Family::Forever,
            "PvP target comparison requires Forever"
        );
        let previous = before.sections.get("player_pvp_progression");
        let current = after.sections.get("player_pvp_progression");
        let season = previous
            .and_then(|row| row["reported_season_id"].as_u64())
            .zip(current.and_then(|row| row["reported_season_id"].as_u64()))
            .map(|(before, after)| before == after);
        let old = previous.and_then(|row| pvp_rank_target_projection(row, target));
        let new = current.and_then(|row| pvp_rank_target_projection(row, target));
        let matching = old
            .as_ref()
            .zip(new.as_ref())
            .filter(|_| season == Some(true));
        let reached = matching.and_then(|(old, new)| {
            old["captured_rank_at_or_above_target"]
                .as_bool()
                .zip(new["captured_rank_at_or_above_target"].as_bool())
        });
        let delta = |field: &str| {
            let (old, new) = matching?;
            Some(i128::from(new[field].as_u64()?) - i128::from(old[field].as_u64()?))
        };
        let total_delta = previous.zip(current).and_then(|(old, new)| {
            let same_total_threshold = |rank: u64| {
                let key = rank.to_string();
                old["total_points_by_rank"][&key]
                    .as_u64()
                    .zip(new["total_points_by_rank"][&key].as_u64())
                    .is_some_and(|(old, new)| old == new)
            };
            (same_total_threshold(u64::from(target))
                && same_total_threshold(old["rank"].as_u64()?)
                && same_total_threshold(new["rank"].as_u64()?))
            .then_some(())?;
            delta("remaining_points_using_total_threshold")
        });
        let next_delta = previous.zip(current).and_then(|(old, new)| {
            (old["next_rank_threshold"] == new["next_rank_threshold"]).then_some(())?;
            delta("remaining_points_using_next_rank_threshold")
        });
        comparison["pvp_rank_target_comparison"] = json!({
            "target_rank":target,"before":old,"after":new,"reported_season_ids_match":season,
            "crossed_up":reached.map(|(before, after)| !before && after),
            "crossed_down":reached.map(|(before, after)| before && !after),
            "remaining_points_delta_using_total_threshold":total_delta,
            "remaining_points_delta_using_next_rank_threshold":next_delta,
            "same_season_verified":false,"live":false
        });
    }
    if selectors.player_pvp_progression_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "PvP progression comparison requires Forever"
        );
        let before = before
            .sections
            .get("player_pvp_progression")
            .map(player_pvp_progression_observation);
        let after = after
            .sections
            .get("player_pvp_progression")
            .map(player_pvp_progression_observation);
        let season = |row: &Option<Value>| row.as_ref()?.get("reported_season_id")?.as_u64();
        let season_match = season(&before).zip(season(&after)).map(|(a, b)| a == b);
        let matching_observations = before
            .as_ref()
            .zip(after.as_ref())
            .filter(|_| season_match == Some(true));
        let observed_change = matching_observations.and_then(|(a, b)| {
            let rank_delta = b["rank"].as_i64()? - a["rank"].as_i64()?;
            let rank_points_delta = if rank_delta == 0 {
                Some(b["rank_points"].as_i64()? - a["rank_points"].as_i64()?)
            } else {
                None
            };
            Some(json!({"rank_delta":rank_delta,"same_rank_reported_points_delta":rank_points_delta}))
        });
        let next_threshold_change = matching_observations
            .filter(|(a, b)| a["rank"] == b["rank"])
            .and_then(|(a, b)| {
                let old_threshold = a["next_rank_threshold"].as_i64()?;
                let new_threshold = b["next_rank_threshold"].as_i64()?;
                let remaining_delta =
                    (old_threshold == new_threshold)
                        .then_some(())
                        .and_then(|_| {
                            let old = a["remaining_points_to_reported_next_threshold"].as_i64()?;
                            let new = b["remaining_points_to_reported_next_threshold"].as_i64()?;
                            Some(new - old)
                        });
                Some(
                    json!({"reported_next_threshold_delta":new_threshold-old_threshold,
                    "same_threshold_remaining_points_delta":remaining_delta}),
                )
            });
        let cap_changes = matching_observations.map(|(a, b)| {
            PVP_CAP_FIELDS
                .into_iter()
                .map(|cap| {
                    let targets = a[cap].as_i64().zip(b[cap].as_i64());
                    let threshold = |row: &Value, rank: i64| {
                        row["total_points_by_rank"][rank.to_string()].as_i64()
                    };
                    let thresholds =
                        targets.and_then(|(old, new)| threshold(a, old).zip(threshold(b, new)));
                    let remaining_delta = targets
                        .filter(|(old, new)| old == new)
                        .and(thresholds.filter(|(old, new)| old == new))
                        .and_then(|_| {
                            let old = a["remaining_points_to_reported_caps"][cap].as_i64()?;
                            let new = b["remaining_points_to_reported_caps"][cap].as_i64()?;
                            Some(new - old)
                        });
                    (
                        cap,
                        json!({"reported_cap_rank_delta":targets.map(|(old,new)|new-old),
                        "reported_cap_threshold_delta":thresholds.map(|(old,new)|new-old),
                        "same_target_remaining_points_delta":remaining_delta}),
                    )
                })
                .collect::<BTreeMap<_, _>>()
        });
        comparison["player_pvp_progression_changes"] = json!({"before":before,"after":after,
            "observed_change_with_matching_season_id":observed_change,
            "observed_next_threshold_change_with_matching_season_id":next_threshold_change,
            "observed_cap_changes_with_matching_season_id":cap_changes,
            "reported_season_ids_match":season_match,"same_season_verified":false,"earned_points_delta":null,"live":false});
    }
    ensure!(
        selectors
            .instance_group_size_max_cache_age_seconds
            .is_none()
            || selectors.instance_group_size_reference,
        "A cache age limit requires the instance group size reference"
    );
    if selectors.instance_group_size_reference {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Instance group size comparison requires Forever"
        );
        comparison["instance_group_size_reference_comparison"] =
            compare_instance_group_size_references(
                before,
                after,
                selectors.instance_group_size_max_cache_age_seconds,
            );
    }
    if let Some(target) = selectors.group_member_target {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Group member target comparison requires Forever"
        );
        comparison["group_member_target_comparison"] =
            compare_group_member_target(before, after, target.get());
    }
    if selectors.player_life_state_changes {
        ensure!(before.client.family == crate::Family::Forever, "Player life-state comparison requires Forever");
        comparison["player_life_state_changes"] = crate::player_life_state::compare(
            before.sections.get("player_life_state"), after.sections.get("player_life_state"))?;
    }
    if selectors.battlefield_queue_changes {
        ensure!(before.client.family == crate::Family::Forever, "Battlefield queue comparison requires Forever");
        comparison["battlefield_queue_changes"] = crate::battlefield_queue_filter::compare(
            before.sections.get("battlefield_queue_slots"), after.sections.get("battlefield_queue_slots"))?;
    }
    if selectors.owned_auction_changes {
        ensure!(before.client.family == crate::Family::Forever, "Owned auction comparison requires Forever");
        comparison["owned_auction_changes"] = crate::owned_auction_filter::compare(
            before.sections.get("owned_auctions"), after.sections.get("owned_auctions"))?;
    }
    if selectors.group_context_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Group comparison requires Forever"
        );
        let categories = ["home", "instance"]
            .into_iter()
            .map(|category| {
                let observed = |snapshot: &Snapshot| {
                    snapshot
                        .sections
                        .get("group_context")
                        .and_then(|section| section["categories"].get(category))
                        .cloned()
                };
                let old = observed(before);
                let new = observed(after);
                let flag_changed = |field: &str| {
                    old.as_ref()?
                        .get(field)?
                        .as_bool()
                        .zip(new.as_ref()?.get(field)?.as_bool())
                        .map(|(a, b)| a != b)
                };
                let delta = old.as_ref().zip(new.as_ref()).and_then(|(a, b)| {
                    Some(
                        b["reported_member_count"].as_i64()?
                            - a["reported_member_count"].as_i64()?,
                    )
                });
                (
                    category.to_string(),
                    json!({"before":old,"after":new,"reported_member_count_delta":delta,
                        "reported_in_group_changed":flag_changed("in_group"),
                        "reported_in_raid_changed":flag_changed("in_raid"),
                        "reported_player_is_leader_changed":flag_changed("player_is_leader")}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        comparison["group_context_changes"] = json!({"categories":categories,
            "group_identity_verified":false,"membership_events_verified":false,"live":false});
    }
    if selectors.profession_spell_changes || selectors.player_resource_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Resource and profession spell comparison requires Forever"
        );
    }
    if selectors.profession_spell_changes {
        comparison["profession_spell_changes"] = before.sections.get("profession_spells")
            .zip(after.sections.get("profession_spells"))
            .filter(|(old,new)| old["profession_id"]==new["profession_id"])
            .map(|(old,new)| {
                let ids=|value:&Value| value["entries"].as_object().unwrap().keys()
                    .map(|id|id.parse::<u32>().unwrap()).collect::<BTreeSet<_>>();
                let old_ids=ids(old);let new_ids=ids(new);
                let added:Vec<_>=new_ids.difference(&old_ids).copied().collect();
                let removed:Vec<_>=old_ids.difference(&new_ids).copied().collect();
                let shared_count = new_ids.len() - added.len();
                json!({"profession_id":new["profession_id"],"before_reported_count":old_ids.len(),
                    "after_reported_count":new_ids.len(),"newly_reported_spell_count":added.len(),
                    "no_longer_reported_spell_count":removed.len(),
                    "newly_reported_spell_ids":added.iter().take(50).collect::<Vec<_>>(),
                    "no_longer_reported_spell_ids":removed.iter().take(50).collect::<Vec<_>>(),
                    "shared_reported_spell_count":shared_count,
                    "shared_reported_spell_ids":old_ids.intersection(&new_ids).take(50).collect::<Vec<_>>(),
                    "shared_reported_truncated":shared_count>50,
                    "newly_reported_truncated":added.len()>50,"no_longer_reported_truncated":removed.len()>50,
                    "learning_verified":false,"complete_recipe_knowledge_verified":false})
            }).unwrap_or(Value::Null);
    }
    if selectors.player_resource_changes {
        comparison["player_resource_changes"] = before.sections.get("player_resources")
            .zip(after.sections.get("player_resources"))
            .map(|(old,new)| {
                let counters = |a: &Value, b: &Value| {
                    let old_current = a["current"].as_i64().unwrap();
                    let old_maximum = a["maximum"].as_i64().unwrap();
                    let new_current = b["current"].as_i64().unwrap();
                    let new_maximum = b["maximum"].as_i64().unwrap();
                    let percentage_point_delta = (old_maximum > 0 && new_maximum > 0).then(|| {
                        let old_maximum = i128::from(old_maximum);
                        let new_maximum = i128::from(new_maximum);
                        let difference = i128::from(new_current) * old_maximum
                            - i128::from(old_current) * new_maximum;
                        difference as f64 * 100.0 / (old_maximum * new_maximum) as f64
                    });
                    let current_delta = new_current - old_current;
                    let maximum_delta = new_maximum - old_maximum;
                    json!({"before":a,"after":b,"current_delta":current_delta,
                        "maximum_delta":maximum_delta,"missing_delta":maximum_delta-current_delta,
                        "percentage_point_delta":percentage_point_delta})
                };
                let power=old.get("primary_power").zip(new.get("primary_power"));
                json!({"health":old.get("health").zip(new.get("health")).map(|(a,b)|counters(a,b)),
                    "primary_power":power.filter(|(a,b)| a["power_type"]==b["power_type"] && a["units"]==b["units"]).map(|(a,b)|counters(a,b)),
                    "primary_power_type_changed":power.map(|(a,b)|a["power_type"]!=b["power_type"]),
                    "resource_events_verified":false,"readiness_verified":false})
            }).unwrap_or(Value::Null);
    }
    ensure!(
        selectors.player_aura_spell_id.is_none() || selectors.player_aura_changes,
        "Player aura spell ID requires the aura comparison"
    );
    if selectors.player_aura_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Player aura comparison requires Forever"
        );
        comparison["player_aura_changes"] = compare_player_auras(
            before,
            after,
            selectors.player_aura_spell_id.map(|id| u64::from(id.get())),
        );
    }
    if let Some(item_id) = selectors.bag_cooldown_item_id {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Bag item cooldown comparison requires Forever"
        );
        comparison["bag_cooldown_item_observations"] =
            compare_bag_cooldown_item(before, after, item_id.get());
    }
    if selectors.pet_action_bar_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Pet action bar comparison requires Forever"
        );
        comparison["pet_action_bar_changes"] = compare_pet_action_bars(before, after);
    }
    if selectors.weapon_enchant_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Weapon enchant comparison requires Forever"
        );
        comparison["weapon_enchant_changes"] = compare_weapon_enchants(before, after);
    }
    if selectors.merchant_buyback_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Merchant buyback comparison requires Forever"
        );
        comparison["merchant_buyback_changes"] = compare_merchant_buyback(before, after);
    }
    if selectors.repair_quote_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Repair quote comparison requires Forever"
        );
        let old = repair_status(before);
        let new = repair_status(after);
        let delta = |field: &str| {
            old[field]
                .as_i64()
                .zip(new[field].as_i64())
                .map(|(a, b)| b - a)
        };
        comparison["repair_quote_changes"] = json!({
            "before":old,"after":new,
            "quote_copper_delta":delta("repair_all_quote_copper"),
            "wallet_shortfall_copper_delta":delta("wallet_shortfall_copper"),
            "wallet_remaining_copper_delta":delta("wallet_remaining_after_saved_repair_quote_copper"),
            "repair_event_verified":false,"live":false
        });
    }
    if selectors.selected_recipe_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Recipe comparison requires Forever"
        );
        comparison["selected_recipe_changes"] = compare_selected_recipe(
            before.sections.get("selected_recipe"),
            after.sections.get("selected_recipe"),
        );
    }
    if let Some(crafts) = selectors.selected_recipe_preparation_crafts {
        ensure!(
            before.client.family == crate::Family::Forever && (1..=1000).contains(&crafts),
            "Selected recipe preparation comparison requires Forever and 1 to 1000 crafts"
        );
        comparison["selected_recipe_preparation"] =
            compare_selected_recipe_preparation(before, after, crafts)?;
    }
    if selectors.mount_collection_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Mount comparison requires Forever"
        );
        comparison["mount_collection_changes"] = before
            .sections
            .get("mount_collection")
            .zip(after.sections.get("mount_collection"))
            .map(|(old, new)| compare_mount_collection(old, new))
            .unwrap_or(Value::Null);
    }
    if selectors.toy_view_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Toy comparison requires Forever"
        );
        comparison["toy_view_changes"] = before
            .sections
            .get("toy_collection_view")
            .zip(after.sections.get("toy_collection_view"))
            .map(|(old, new)| compare_toy_views(old, new))
            .unwrap_or(Value::Null);
    }
    if selectors.saved_world_boss_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Saved world boss comparison requires Forever"
        );
        comparison["saved_world_boss_changes"] = before
            .sections
            .get("saved_world_bosses")
            .zip(after.sections.get("saved_world_bosses"))
            .map(|(old, new)| crate::saved_resets::compare_world_bosses(old, new))
            .unwrap_or(Value::Null);
    }
    if selectors.instance_lock_warning_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Instance lock warning comparison requires Forever"
        );
        comparison["instance_lock_warning_changes"] = before
            .sections
            .get("instance_context")
            .zip(after.sections.get("instance_context"))
            .and_then(|(old, new)| crate::saved_resets::compare_instance_lock_warnings(old, new))
            .unwrap_or(Value::Null);
    }
    if selectors.saved_instance_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Saved-instance comparison requires Forever"
        );
        comparison["saved_instance_changes"] = before
            .sections
            .get("saved_instances")
            .zip(after.sections.get("saved_instances"))
            .map(|(old, new)| compare_saved_instances(old, new))
            .unwrap_or(Value::Null);
    }
    if selectors.gossip_option_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Gossip option comparison requires Forever"
        );
        comparison["gossip_option_changes"] = crate::gossip_option_filter::compare_options(
            before.sections.get("gossip_options"),
            after.sections.get("gossip_options"),
        );
    }
    if selectors.gossip_quest_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Gossip quest comparison requires Forever"
        );
        comparison["gossip_quest_changes"] =
            crate::gossip_option_filter::compare_quest_lists(before, after);
    }
    if selectors.loot_window_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Loot comparison requires Forever"
        );
        comparison["loot_window_changes"] = before
            .sections
            .get("loot_window")
            .zip(after.sections.get("loot_window"))
            .map(|(old, new)| crate::loot_observation::compare(old, new))
            .unwrap_or(Value::Null);
    }
    if selectors.recipe_cooldown_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Recipe cooldown comparison requires Forever"
        );
        comparison["recipe_cooldown_changes"] = before
            .sections
            .get("profession_recipe_materials")
            .zip(after.sections.get("profession_recipe_materials"))
            .map(|(old, new)| compare_recipe_cooldowns(old, new))
            .unwrap_or(Value::Null);
    }
    if selectors.profession_recipe_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Profession recipe comparison requires Forever"
        );
        comparison["profession_recipe_changes"] = before
            .sections
            .get("profession_recipe_view")
            .zip(after.sections.get("profession_recipe_view"))
            .map(|(old, new)| compare_profession_recipe_views(old, new))
            .unwrap_or(Value::Null);
    }
    if selectors.pet_view_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Pet journal view comparison requires Forever"
        );
        comparison["pet_view_changes"] = before
            .sections
            .get("pet_collection_view")
            .zip(after.sections.get("pet_collection_view"))
            .map(|(old, new)| compare_pet_views(old, new))
            .unwrap_or(Value::Null);
    }
    ensure!(
        !selectors.mail_attachment_compare_carried_item
            || selectors.mail_attachment_item_id.is_some(),
        "Carried item comparison requires an exact mail item"
    );
    ensure!(
        !selectors.mail_attachment_compare_currency_balance
            || selectors.mail_attachment_currency_id.is_some(),
        "Currency balance comparison requires an exact mail currency"
    );
    for (kind, id) in [
        ("item", selectors.mail_attachment_item_id),
        ("currency", selectors.mail_attachment_currency_id),
    ] {
        let Some(id) = id else { continue };
        ensure!(
            before.client.family == crate::Family::Forever,
            "Mail attachment comparison requires Forever"
        );
        let mut attachment = compare_mail_attachment_id(
            before.sections.get("mail_attachments"),
            after.sections.get("mail_attachments"),
            kind,
            id.get(),
        );
        if kind == "item" && selectors.mail_attachment_compare_carried_item {
            let mut carried = compare_carried_item_quantities(before, after, id.get())?;
            carried["mail_counts_toward_carried_stock"] = json!(false);
            attachment["carried_comparison"] = carried;
            let project = |snapshot: &Snapshot, side: &Value| {
                let quantity = side["known_quantity"].as_u64()?;
                Some(incoming_items_bag_space(
                    snapshot,
                    std::iter::once((id.get(), quantity)),
                    "matching_loaded_mail_items_only",
                    "known_pending_quantity",
                ))
            };
            attachment["hypothetical_bag_space"] = json!({
                "before":project(before, &attachment["before"]),
                "after":project(after, &attachment["after"]),
                "retrieval_eligibility_verified":false
            });
        }
        if kind == "currency" && selectors.mail_attachment_compare_currency_balance {
            attachment["balance_comparison"] =
                compare_mail_currency_balances(before, after, id.get());
            let key = id.to_string();
            let project = |snapshot: &Snapshot, side: &Value| {
                let quantity = side["known_quantity"].as_u64()?;
                let row = snapshot.sections.get("currency_balances")?["entries"].get(&key)?;
                let caps = currency_entry_cap_headroom(row)?;
                Some(currency_reward_projection(quantity, row, &caps))
            };
            attachment["hypothetical_cap_projection"] = json!({
                "before":project(before, &attachment["before"]),
                "after":project(after, &attachment["after"]),
                "mail_cap_application_verified":false
            });
        }
        comparison[format!("mail_attachment_{kind}_comparison")] = attachment;
    }
    if selectors.mail_attachment_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Mail attachment comparison requires Forever"
        );
        comparison["mail_attachment_changes"] = before
            .sections
            .get("mail_attachments")
            .zip(after.sections.get("mail_attachments"))
            .map(|(old, new)| compare_mail_attachments(old, new))
            .unwrap_or(Value::Null);
    }
    let skills_before = before
        .sections
        .get("skill_lines")
        .and_then(Value::as_object);
    let skills_after = after.sections.get("skill_lines").and_then(Value::as_object);
    if skills_before.is_some() || skills_after.is_some() {
        comparison["skill_line_progress"] = skills_before
            .zip(skills_after)
            .map(|(before, after)| compare_skill_lines(before, after))
            .unwrap_or(Value::Null);
    }
    let professions_before = before.sections.get("professions");
    let professions_after = after.sections.get("professions");
    if before.client.family == crate::Family::Forever
        && (professions_before.is_some() || professions_after.is_some())
    {
        comparison["profession_progress"] = professions_before
            .zip(professions_after)
            .and_then(|(before, after)| compare_professions(before, after))
            .unwrap_or(Value::Null);
    }
    let flight_before = before.sections.get("flight_map");
    let flight_after = after.sections.get("flight_map");
    if before.client.family == crate::Family::Forever
        && let Some(travel) = compare_travel_context(before, after)
    {
        comparison["travel_context_change"] = travel;
    }
    let merchant_before = before.sections.get("merchant_stock");
    let merchant_after = after.sections.get("merchant_stock");
    for (kind, id) in [
        ("item", selectors.merchant_offer_item_id),
        ("currency", selectors.merchant_offer_currency_id),
    ] {
        let Some(id) = id else { continue };
        ensure!(
            before.client.family == crate::Family::Forever,
            "Merchant offer comparisons require Forever snapshots"
        );
        comparison[format!("merchant_{kind}_offer_comparison")] =
            compare_merchant_product_offers(merchant_before, merchant_after, kind, id.get());
    }
    if merchant_before.is_some() || merchant_after.is_some() {
        let (changes, coverage) = merchant_before
            .zip(merchant_after)
            .and_then(|(old, new)| compare_merchant_offers(old, new))
            .unwrap_or((Value::Null, Value::Null));
        comparison["merchant_offer_changes"] = changes;
        comparison["merchant_offer_coverage"] = coverage;
    }
    let trainer_before = before.sections.get("trainer_services");
    let trainer_after = after.sections.get("trainer_services");
    if let Some(name) = selectors.trainer_lesson_name.map(str::trim) {
        ensure!(
            before.client.family == crate::Family::Forever && !name.is_empty() && name.len() <= 256,
            "Trainer lesson comparison requires Forever and a valid name"
        );
        comparison["trainer_lesson_comparison"] =
            compare_named_trainer_lesson(trainer_before, trainer_after, &name.to_lowercase());
    }
    if trainer_before.is_some() || trainer_after.is_some() {
        comparison["trainer_service_changes"] = trainer_before
            .zip(trainer_after)
            .and_then(|(old, new)| compare_trainer_services(old, new))
            .unwrap_or(Value::Null);
    }
    let wallet = |snapshot: &Snapshot| snapshot.sections.get("wallet")?["copper"].as_u64();
    ensure!(
        selectors
            .flight_wallet_reserve_copper
            .is_none_or(|reserve| {
                reserve <= 9_999_999_999_999 && selectors.flight_destination_node_id.is_some()
            }),
        "Flight wallet reserves require an exact destination and a valid copper amount"
    );
    if let Some(destination) = selectors.flight_destination_node_id {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Flight destination comparison requires Forever snapshots"
        );
        comparison["flight_destination_comparison"] = compare_flight_destination(
            flight_before,
            flight_after,
            wallet(before),
            wallet(after),
            destination.get(),
            selectors.flight_wallet_reserve_copper,
        );
    }
    if flight_before.is_some() || flight_after.is_some() {
        comparison["flight_map_changes"] = flight_before
            .zip(flight_after)
            .map(|(old_map, new_map)| {
                compare_flight_maps(old_map, new_map, wallet(before).zip(wallet(after)))
            })
            .unwrap_or(Value::Null);
    }
    let old_alerts = before
        .sections
        .get("durability_alerts")
        .and_then(Value::as_object);
    let new_alerts = after
        .sections
        .get("durability_alerts")
        .and_then(Value::as_object);
    let old_condition = before.sections.get("equipment_durability");
    let new_condition = after.sections.get("equipment_durability");
    if let Some(maximum) = selectors.equipment_durability_max_percent {
        ensure!(
            before.client.family == crate::Family::Forever && maximum <= 100,
            "Durability threshold comparison requires Forever and a percentage from 0 to 100"
        );
        let old_low =
            old_condition.and_then(|section| equipment_durability_matching_slots(section, maximum));
        let new_low =
            new_condition.and_then(|section| equipment_durability_matching_slots(section, maximum));
        let crossings = old_low.as_ref().zip(new_low.as_ref()).map(|(old, new)| {
            let shared = |low: &BTreeSet<u32>, other: &BTreeSet<u32>, section: &Value| {
                low.difference(other)
                    .copied()
                    .filter(|slot| section["entries"].get(slot.to_string()).is_some())
                    .collect::<Vec<_>>()
            };
            json!({
                "entered_slots":shared(new, old, old_condition.unwrap()),
                "exited_slots":shared(old, new, new_condition.unwrap()),
            })
        });
        comparison["equipment_durability_threshold"] = json!({
            "maximum_percent":maximum,"before_matching_slots":old_low,"after_matching_slots":new_low,
            "crossings":crossings,"same_item_verified":false,"repair_event_verified":false,"live":false,
        });
    }
    if old_condition.is_some() || new_condition.is_some() {
        comparison["equipment_durability_changes"] = json!(
            old_condition.zip(new_condition).map(|(old, new)| {
                let old = old["entries"].as_object().unwrap();
                let new = new["entries"].as_object().unwrap();
                let slots: BTreeSet<_> = old.keys().chain(new.keys())
                    .map(|slot| slot.parse::<u32>().unwrap()).collect();
                let mut only_before = Vec::new();
                let mut only_after = Vec::new();
                let mut changes = Vec::new();
                let mut item_context = BTreeMap::new();
                let item_at = |snapshot: &Snapshot, slot: &str| {
                    snapshot.sections.get("equipment")?[slot]["item_id"].as_u64()
                };
                for slot in slots {
                    let key = slot.to_string();
                    if old.contains_key(&key) && new.contains_key(&key) {
                        let before_id = item_at(before, &key);
                        let after_id = item_at(after, &key);
                        item_context.insert(slot, json!({"before_item_id":before_id,"after_item_id":after_id,
                            "base_item_ids_match":before_id.zip(after_id).map(|(a,b)| a == b)}));
                    }
                    match (old.get(&key), new.get(&key)) {
                        (Some(a), Some(b)) if a != b => changes.push(json!({
                            "slot":slot,"before":a,"after":b,
                            "current_delta":b["current"].as_i64().unwrap() - a["current"].as_i64().unwrap(),
                            "maximum_delta":b["maximum"].as_i64().unwrap() - a["maximum"].as_i64().unwrap(),
                        })),
                        (Some(_), None) => only_before.push(slot),
                        (None, Some(_)) => only_after.push(slot),
                        _ => {}
                    }
                }
                json!({"basis":"captured_equipment_slots","changes":changes,
                    "item_context_by_shared_slot":item_context,
                    "observed_only_before_slots":only_before,"observed_only_after_slots":only_after,
                    "same_item_verified":false,"repair_event_verified":false,"live":false})
            })
        );
    }
    if old_alerts.is_some() || new_alerts.is_some() {
        comparison["durability_alert_changes"] = json!(old_alerts.zip(new_alerts).map(|(old, new)| {
            old.keys().chain(new.keys()).collect::<BTreeSet<_>>().into_iter()
                .filter_map(|slot| {
                    let previous = old.get(slot).and_then(Value::as_str);
                    let current = new.get(slot).and_then(Value::as_str);
                    (previous != current).then(|| (slot, json!({
                        "before":previous.unwrap_or("no_alert"),"after":current.unwrap_or("no_alert")
                    })))
                }).collect::<BTreeMap<_, _>>()
        }));
    }
    let repair_quote = |snapshot: &Snapshot| {
        snapshot
            .sections
            .get("repair_quote")
            .and_then(|quote| quote["copper"].as_i64())
    };
    let (old_quote, new_quote) = (repair_quote(before), repair_quote(after));
    if old_quote.is_some() || new_quote.is_some() {
        comparison["repair_quote_change"] = json!({
            "before_copper":old_quote,"after_copper":new_quote,
            "delta_copper":old_quote.zip(new_quote).map(|(old,new)| new - old)
        });
    }
    let previous_space = carried_bag_space(before);
    let current_space = carried_bag_space(after);
    if previous_space.is_some() || current_space.is_some() {
        let delta = |field: &str| {
            Some(
                current_space.as_ref()?[field].as_i64()?
                    - previous_space.as_ref()?[field].as_i64()?,
            )
        };
        let family_changes = previous_space
            .as_ref()
            .zip(current_space.as_ref())
            .and_then(|(before, after)| {
                let previous = before["free_slots_by_family_mask"].as_object()?;
                let current = after["free_slots_by_family_mask"].as_object()?;
                Some(
                    previous
                        .keys()
                        .chain(current.keys())
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .filter_map(|mask| {
                            let old = previous.get(mask).and_then(Value::as_i64);
                            let new = current.get(mask).and_then(Value::as_i64);
                            (old != new).then(|| (mask, json!({
                            "before_free_slots":old,"after_free_slots":new,
                            "free_slots_delta":old.zip(new).map(|(before, after)| after - before),
                        })))
                        })
                        .collect::<BTreeMap<_, _>>(),
                )
            });
        comparison["carried_bag_space_change"] = json!({
            "total_slots_delta":delta("total_slots"),
            "free_slots_delta":delta("free_slots"),
            "unknown_family_free_slots_delta":delta("unknown_family_free_slots"),
            "family_space_changes":family_changes,
            "bag_slot_changes":compare_bag_capacity_slots(before, after),
            "before":previous_space,
            "after":current_space,
        });
    }
    if before.client.family == crate::Family::Forever
        && (before.sections.contains_key("pet_care") || after.sections.contains_key("pet_care"))
    {
        comparison["pet_care_change"] = compare_pet_care(
            before.sections.get("pet_care").unwrap_or(&Value::Null),
            after.sections.get("pet_care").unwrap_or(&Value::Null),
        );
    }
    let tracked_before = before.sections.get("tracked_recipes");
    let tracked_after = after.sections.get("tracked_recipes");
    if tracked_before.is_some() || tracked_after.is_some() {
        comparison["tracked_recipe_changes"] = tracked_before
            .zip(tracked_after)
            .map(|(before, after)| {
                json!({
                    "crafts":tracked_recipe_membership(&before["crafts"], &after["crafts"]),
                    "recrafts":tracked_recipe_membership(&before["recrafts"], &after["recrafts"]),
                })
            })
            .unwrap_or(Value::Null);
        comparison["tracked_recipe_learned_changes"] = tracked_before
            .zip(tracked_after)
            .map(|(before, after)| json!({
                "crafts":tracked_recipe_learned_changes(&before["crafts"], &after["crafts"]),
                "recrafts":tracked_recipe_learned_changes(&before["recrafts"], &after["recrafts"])
            })).unwrap_or(Value::Null);
    }
    if let Some(item_id) = selectors.item_id {
        comparison["item_quantity_change"] =
            compare_item(before, after, item_id, selectors.item_container_changes)?;
    }
    if let Some(currency_id) = resolve_comparison_id(
        before,
        after,
        "currency_balances",
        Some("entries"),
        "name",
        selectors.currency_id,
        selectors.currency_name,
    )? {
        ensure!(
            before.client.family == crate::Family::Forever && currency_id > 0,
            "Currency balance comparisons require a Forever currency ID"
        );
        let key = currency_id.to_string();
        let balance = |snapshot: &Snapshot| {
            snapshot
                .sections
                .get("currency_balances")
                .and_then(|section| section["entries"].get(&key))
                .cloned()
        };
        let previous = balance(before);
        let current = balance(after);
        let same_scope = previous
            .as_ref()
            .zip(current.as_ref())
            .filter(|(a, b)| a["account_wide"] == b["account_wide"]);
        let change = same_scope.and_then(|(a, b)| {
            let from = a["quantity"].as_u64()?;
            let to = b["quantity"].as_u64()?;
            let direction = match to.cmp(&from) {
                std::cmp::Ordering::Greater => "increased",
                std::cmp::Ordering::Less => "decreased",
                std::cmp::Ordering::Equal => "unchanged",
            };
            Some(json!({"direction":direction,"amount":to.abs_diff(from)}))
        });
        let weekly_earned_delta = same_scope.and_then(|(a, b)| {
            Some(b["earned_this_week"].as_i64()? - a["earned_this_week"].as_i64()?)
        });
        let weekly_headroom_change = same_scope.and_then(|(a, b)| {
            let from = weekly_cap_remaining(a)?;
            let to = weekly_cap_remaining(b)?;
            Some(json!({"before_remaining":from,"after_remaining":to,
                "reached_cap":from > 0 && to == 0,"left_cap":from == 0 && to > 0}))
        });
        comparison["currency_balance_change"] = json!({"currency_id":currency_id,"before":previous,"after":current,"change":change,
                "weekly_earned_delta":weekly_earned_delta,"weekly_headroom_change":weekly_headroom_change});
    }
    if let Some(faction_id) = resolve_comparison_id(
        before,
        after,
        "reputation",
        Some("entries"),
        "name",
        selectors.faction_id,
        selectors.faction_name,
    )? {
        ensure!(
            before.client.family == crate::Family::Forever && faction_id > 0,
            "Reputation comparisons require a Forever faction ID"
        );
        let key = faction_id.to_string();
        let faction = |snapshot: &Snapshot| {
            snapshot
                .sections
                .get("reputation")
                .and_then(|section| section["entries"].get(&key))
                .cloned()
        };
        let previous = faction(before);
        let current = faction(after);
        let standing_delta = previous.as_ref().zip(current.as_ref()).and_then(|(a, b)| {
            if a["kind"] != "standard"
                || b["kind"] != "standard"
                || a["account_wide"] != b["account_wide"]
            {
                return None;
            }
            Some(b["standing"].as_i64()? - a["standing"].as_i64()?)
        });
        let special_progress_change = previous
            .as_ref()
            .zip(current.as_ref())
            .and_then(|(before, after)| compare_special_reputation(before, after));
        comparison["reputation_change"] = json!({
            "faction_id":faction_id,"before":previous,"after":current,"standing_delta":standing_delta,
            "special_progress_change":special_progress_change
        });
    }
    if let Some(quest_id) = resolve_comparison_id(
        before,
        after,
        "quests",
        None,
        "title",
        selectors.quest_id,
        selectors.quest_title,
    )? {
        ensure!(quest_id > 0, "Expected a positive quest ID");
        let key = quest_id.to_string();
        let quest = |snapshot: &Snapshot| {
            snapshot
                .sections
                .get("quests")
                .and_then(|section| section.get(&key))
                .cloned()
        };
        let previous = quest(before);
        let current = quest(after);
        comparison["quest_change"] = json!({
            "quest_id":quest_id,"before":previous,"after":current,
            "ready_for_turn_in_changed":observed_flag_changed(previous.as_ref(), current.as_ref(), "ready_for_turn_in"),
            "failed_changed":observed_flag_changed(previous.as_ref(), current.as_ref(), "failed")
        });
        let timer = |quest: Option<&Value>| {
            let timer = quest?.get("timer")?;
            Some((
                timer["remaining_seconds"].as_i64()?,
                timer["total_seconds"].as_i64()?,
            ))
        };
        let (old_timer, new_timer) = (timer(previous.as_ref()), timer(current.as_ref()));
        if before.client.family == crate::Family::Forever
            && (old_timer.is_some() || new_timer.is_some())
        {
            let pair = old_timer.zip(new_timer);
            let old_expiry = old_timer.and_then(|(seconds, _)| {
                crate::saved_resets::projected_reset_at(seconds, before.captured_at)
            });
            let new_expiry = new_timer.and_then(|(seconds, _)| {
                crate::saved_resets::projected_reset_at(seconds, after.captured_at)
            });
            comparison["quest_change"]["timer_change"] = json!({
                "before_observed":old_timer.is_some(),"after_observed":new_timer.is_some(),
                "remaining_seconds_delta":pair.map(|(old,new)| new.0 - old.0),
                "total_seconds_delta":pair.map(|(old,new)| new.1 - old.1),
                "before_projected_expiry_at":old_expiry,"after_projected_expiry_at":new_expiry,
                "projected_expiry_changed":old_expiry.zip(new_expiry).map(|(old,new)| old != new)
            });
        }
        if before.client.family == crate::Family::Forever {
            let old_money =
                quest_wallet_projection(previous.as_ref(), before.sections.get("wallet"));
            let new_money = quest_wallet_projection(current.as_ref(), after.sections.get("wallet"));
            let mut rewards = serde_json::Map::new();
            for (field, old, new) in [
                (
                    "reward_xp",
                    previous
                        .as_ref()
                        .and_then(|row| row["reward_amounts"]["xp"].as_i64()),
                    current
                        .as_ref()
                        .and_then(|row| row["reward_amounts"]["xp"].as_i64()),
                ),
                (
                    "reward_honor",
                    previous
                        .as_ref()
                        .and_then(|row| row["reward_amounts"]["honor"].as_i64()),
                    current
                        .as_ref()
                        .and_then(|row| row["reward_amounts"]["honor"].as_i64()),
                ),
                (
                    "reward_copper",
                    old_money["reward_copper"].as_i64(),
                    new_money["reward_copper"].as_i64(),
                ),
                (
                    "required_copper",
                    old_money["required_copper"].as_i64(),
                    new_money["required_copper"].as_i64(),
                ),
                (
                    "net_copper",
                    old_money["net_copper"].as_i64(),
                    new_money["net_copper"].as_i64(),
                ),
                (
                    "upfront_shortfall_copper",
                    old_money["upfront_shortfall_copper"].as_i64(),
                    new_money["upfront_shortfall_copper"].as_i64(),
                ),
            ] {
                if old.is_some() || new.is_some() {
                    rewards.insert(
                        field.into(),
                        json!({"before":old,"after":new,
                        "delta":old.zip(new).map(|(a,b)| b-a)}),
                    );
                }
            }
            if !rewards.is_empty() {
                comparison["quest_change"]["reward_and_cost_changes"] = json!(rewards);
            }
            let mut reward_entries = serde_json::Map::new();
            for (field, id_field) in [
                ("item_rewards", "item_id"),
                ("item_choices", "item_id"),
                ("currency_rewards", "currency_id"),
                ("currency_choices", "currency_id"),
                ("renown_rewards", "faction_id"),
            ] {
                if let Some(change) = compare_quest_reward_entries(
                    previous.as_ref().and_then(|quest| quest.get(field)),
                    current.as_ref().and_then(|quest| quest.get(field)),
                    id_field,
                ) {
                    reward_entries.insert(field.into(), change);
                }
            }
            if !reward_entries.is_empty() {
                comparison["quest_change"]["reward_entry_changes"] = json!(reward_entries);
            }
            let old_objectives = previous
                .as_ref()
                .and_then(|quest| quest["objectives"].as_object());
            let new_objectives = current
                .as_ref()
                .and_then(|quest| quest["objectives"].as_object());
            if old_objectives.is_some() || new_objectives.is_some() {
                comparison["quest_change"]["objective_counts_by_kind"] = json!({"before":old_objectives.map(quest_objective_counts),
                           "after":new_objectives.map(quest_objective_counts)});
                comparison["quest_change"]["objective_quantities_by_kind"] = json!({
                    "before":old_objectives.map(quest_objective_quantities),
                    "after":new_objectives.map(quest_objective_quantities),
                    "objective_identity_verified":false
                });
                comparison["quest_change"]["objective_index_changes"] = json!(
                    old_objectives
                        .zip(new_objectives)
                        .map(|(before, after)| compare_quest_objective_indices(before, after))
                );
            }
        }
    }
    if selectors.quest_objective_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Quest objective comparison requires Forever"
        );
        let logs = before
            .sections
            .get("quests")
            .and_then(Value::as_object)
            .zip(after.sections.get("quests").and_then(Value::as_object));
        comparison["quest_objective_changes"] =
            json!(logs.map(|(before, after)| { compare_quest_log_objectives(before, after) }));
    }
    if selectors.quest_readiness_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Quest readiness comparison requires Forever"
        );
        let logs = before
            .sections
            .get("quests")
            .and_then(Value::as_object)
            .zip(after.sections.get("quests").and_then(Value::as_object));
        comparison["quest_readiness_changes"] = json!(logs.map(|(before, after)| {
            let mut changes = BTreeMap::new();
            let mut only_before = BTreeSet::new();
            let mut only_after = BTreeSet::new();
            let mut became_ready = BTreeSet::new();
            let mut became_not_ready = BTreeSet::new();
            let mut became_failed = BTreeSet::new();
            for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
                let Ok(id) = key.parse::<u32>() else { continue };
                match (before.get(key), after.get(key)) {
                    (Some(old), Some(new)) => {
                        let previous = quest_readiness(old);
                        let current = quest_readiness(new);
                        match (previous, current) {
                            (QuestReadiness::NotReady, QuestReadiness::Ready) => { became_ready.insert(id); }
                            (QuestReadiness::Ready, QuestReadiness::NotReady) => { became_not_ready.insert(id); }
                            (QuestReadiness::Ready | QuestReadiness::NotReady, QuestReadiness::Failed) => { became_failed.insert(id); }
                            _ => {}
                        }
                        if previous != current {
                            changes.insert(id, json!({"quest_id":id,"before":previous,"after":current}));
                        }
                    }
                    (Some(_), None) => { only_before.insert(id); }
                    (None, Some(_)) => { only_after.insert(id); }
                    (None, None) => {}
                }
            }
            json!({"changes":changes.into_values().collect::<Vec<_>>(),
                "quest_ids_not_ready_to_ready":became_ready,
                "quest_ids_ready_to_not_ready":became_not_ready,
                "quest_ids_ready_or_not_ready_to_failed":became_failed,
                "quest_ids_observed_only_before":only_before,"quest_ids_observed_only_after":only_after})
        }));
    }
    if selectors.quest_waypoints {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Quest waypoint comparison requires Forever"
        );
        let quest_rows = before
            .sections
            .get("quests")
            .and_then(Value::as_object)
            .zip(after.sections.get("quests").and_then(Value::as_object));
        comparison["quest_waypoint_changes"] = json!(quest_rows.map(|(old, new)| {
            let mut changes = BTreeMap::new();
            for (key, previous) in old {
                let Some(current) = new.get(key) else {
                    continue;
                };
                let Ok(id) = key.parse::<u32>() else { continue };
                let from = previous["next_waypoint_map_id"].as_u64();
                let to = current["next_waypoint_map_id"].as_u64();
                let status = match (from, to) {
                    (Some(a), Some(b)) if a != b => "map_changed",
                    (None, Some(_)) => "map_became_observed",
                    (Some(_), None) => "map_became_unknown",
                    _ => continue,
                };
                changes.insert(
                    id,
                    json!({"quest_id":id,"before_map_id":from,"after_map_id":to,"status":status}),
                );
            }
            changes.into_values().collect::<Vec<_>>()
        }));
    }
    if selectors.achievement_status_changes {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Achievement status comparison requires Forever"
        );
        let previous = before
            .sections
            .get("tracked_achievements")
            .and_then(|section| section["entries"].as_object());
        let current = after
            .sections
            .get("tracked_achievements")
            .and_then(|section| section["entries"].as_object());
        comparison["achievement_status_changes"] = json!(previous.zip(current).map(|(before, after)| {
            let mut changes = BTreeMap::new();
            let mut only_before = BTreeSet::new();
            let mut only_after = BTreeSet::new();
            let mut scope_changed = BTreeSet::new();
            let mut fewer_remaining = BTreeSet::new();
            let mut more_remaining = BTreeSet::new();
            let mut newly_complete = BTreeMap::<u32, Vec<u64>>::new();
            let mut no_longer_complete = BTreeMap::<u32, Vec<u64>>::new();
            for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
                let Ok(id) = key.parse::<u32>() else { continue };
                match (before.get(key), after.get(key)) {
                    (Some(old), Some(new)) if old["account_wide"] != new["account_wide"] => {
                        scope_changed.insert(id);
                    }
                    (Some(old), Some(new)) => {
                        for (criterion_id, previous, current) in matched_achievement_criteria(&old["criteria"], &new["criteria"]) {
                            if previous["required"] != current["required"] {
                                continue;
                            }
                            match (previous["completed"].as_bool(), current["completed"].as_bool()) {
                                (Some(false), Some(true)) => { newly_complete.entry(id).or_default().push(criterion_id); }
                                (Some(true), Some(false)) => { no_longer_complete.entry(id).or_default().push(criterion_id); }
                                _ => {}
                            }
                            if let Some(required) = previous["required"].as_u64()
                                && current["required"].as_u64() == Some(required)
                                && let Some((old, new)) = previous["quantity"].as_u64().zip(current["quantity"].as_u64())
                            {
                                match required.saturating_sub(new).cmp(&required.saturating_sub(old)) {
                                    std::cmp::Ordering::Less => { fewer_remaining.insert(id); }
                                    std::cmp::Ordering::Greater => { more_remaining.insert(id); }
                                    std::cmp::Ordering::Equal => {}
                                }
                            }
                        }
                        if old["completed"] != new["completed"] || old["earned_by_character"] != new["earned_by_character"] {
                            let status = |entry: &Value| json!({"completed":entry["completed"],
                                "earned_by_character":entry["earned_by_character"],"account_wide":entry["account_wide"]});
                            changes.insert(id, json!({"achievement_id":id,"before":status(old),"after":status(new)}));
                        }
                    }
                    (Some(_), None) => { only_before.insert(id); }
                    (None, Some(_)) => { only_after.insert(id); }
                    (None, None) => {}
                }
            }
            json!({"changes":changes.into_values().collect::<Vec<_>>(),
                "achievement_ids_observed_only_before":only_before,"achievement_ids_observed_only_after":only_after,
                "achievement_ids_with_decreased_remaining_criterion_quantities":fewer_remaining,
                "achievement_ids_with_increased_remaining_criterion_quantities":more_remaining,
                "newly_completed_criterion_ids_by_achievement":newly_complete,
                "no_longer_completed_criterion_ids_by_achievement":no_longer_complete,
                "achievement_ids_with_scope_changes":scope_changed})
        }));
    }
    if let Some(achievement_id) = resolve_comparison_id(
        before,
        after,
        "tracked_achievements",
        Some("entries"),
        "name",
        selectors.achievement_id,
        selectors.achievement_name,
    )? {
        ensure!(
            before.client.family == crate::Family::Forever && achievement_id > 0,
            "Tracked achievement comparisons require a Forever achievement ID"
        );
        let key = achievement_id.to_string();
        let achievement = |snapshot: &Snapshot| {
            snapshot
                .sections
                .get("tracked_achievements")
                .and_then(|section| section["entries"].get(&key))
                .cloned()
        };
        let previous = achievement(before);
        let current = achievement(after);
        let same_scope = previous
            .as_ref()
            .zip(current.as_ref())
            .is_some_and(|(a, b)| a["account_wide"] == b["account_wide"]);
        comparison["achievement_change"] = json!({
            "achievement_id":achievement_id,"before":previous,"after":current,
            "completed_changed":same_scope.then(|| observed_flag_changed(previous.as_ref(), current.as_ref(), "completed")).flatten(),
            "earned_by_character_changed":same_scope.then(|| observed_flag_changed(previous.as_ref(), current.as_ref(), "earned_by_character")).flatten(),
            "criterion_comparison":previous.as_ref().zip(current.as_ref()).filter(|_| same_scope)
                .map(|(a, b)| compare_criteria(&a["criteria"], &b["criteria"]))
        });
    }
    ensure!(
        selectors.profession_target_rank.is_none() || selectors.profession_skill_line.is_some(),
        "Profession rank targets require a profession skill line"
    );
    if let Some(skill_line) = selectors.profession_skill_line {
        ensure!(skill_line > 0, "Expected a positive profession skill line");
        let (before_status, previous) =
            profession_by_skill_line(before.sections.get("professions"), skill_line);
        let (after_status, current) =
            profession_by_skill_line(after.sections.get("professions"), skill_line);
        let rank_delta = previous
            .zip(current)
            .and_then(|(a, b)| Some(b["rank"].as_i64()? - a["rank"].as_i64()?));
        comparison["profession_change"] = json!({
            "skill_line":skill_line,"before_status":before_status,"after_status":after_status,
            "before":previous,"after":current,"rank_delta":rank_delta
        });
        if let Some(target) = selectors.profession_target_rank {
            let before = profession_target_progress(previous, target.get());
            let after = profession_target_progress(current, target.get());
            let reached = before["target_reached_at_capture"]
                .as_bool()
                .zip(after["target_reached_at_capture"].as_bool());
            comparison["profession_target_change"] = json!({
                "before":before,"after":after,
                "crossed_up":reached.map(|(old, new)| !old && new),
                "crossed_down":reached.map(|(old, new)| old && !new)
            });
        }
    }
    if selectors.action_bar_slots {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Action bar slot comparisons require Forever snapshots"
        );
        comparison["action_bar_slot_changes"] = compare_action_bar_slots(before, after);
    }
    for (kind, id) in [
        ("spell", selectors.action_bar_spell_id),
        ("item", selectors.action_bar_item_id),
    ] {
        let Some(id) = id else { continue };
        ensure!(
            before.client.family == crate::Family::Forever,
            "Action bar placement comparisons require Forever snapshots"
        );
        comparison[format!("action_bar_{kind}_placement_change")] =
            compare_action_bar_placement(before, after, kind, id.get());
    }
    if selectors.spellbook {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Spellbook comparisons require Forever snapshots"
        );
        comparison["spellbook_change"] = compare_spellbook(before, after);
    }
    if selectors.experience {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Experience comparisons require Forever snapshots"
        );
        let previous = before.sections.get("experience");
        let current = after.sections.get("experience");
        let before_level = before
            .sections
            .get("stats")
            .and_then(|stats| stats["level"].as_i64());
        let after_level = after
            .sections
            .get("stats")
            .and_then(|stats| stats["level"].as_i64());
        let levels = before_level.zip(after_level);
        let level_delta = levels.and_then(|(before, after)| after.checked_sub(before));
        let within_level = previous.zip(current).and_then(|(a, b)| {
            let (before_level, after_level) = levels?;
            let threshold = a["level_threshold_xp"].as_u64()?;
            if before_level != after_level || b["level_threshold_xp"].as_u64() != Some(threshold) {
                return None;
            }
            Some(b["current_xp"].as_i64()? - a["current_xp"].as_i64()?)
        });
        let observed = previous.zip(current);
        let single_level_up = observed.and_then(|(before, after)| {
            if level_delta != Some(1) || before_level.is_none_or(|level| level <= 0) {
                return None;
            }
            let remaining = before["level_threshold_xp"]
                .as_u64()?
                .checked_sub(before["current_xp"].as_u64()?)?;
            Some(remaining + after["current_xp"].as_u64()?)
        });
        comparison["experience_change"] = json!({
            "before":previous,"after":current,"before_level":before_level,"after_level":after_level,
            "level_delta":level_delta,"within_level_xp_delta":within_level,
            "single_level_up_xp_delta":single_level_up,
            "rested_xp_delta":observed.and_then(|(a,b)| Some(b["rested_xp"].as_i64()? - a["rested_xp"].as_i64()?)),
            "resting_changed":observed_flag_changed(previous, current, "resting"),
            "xp_disabled_changed":observed_flag_changed(previous, current, "xp_disabled"),
            "status_changed":observed.map(|(a,b)| a["status"] != b["status"]),
        });
    }
    if selectors.wallet {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Wallet comparisons require Forever snapshots"
        );
        let copper = |snapshot: &Snapshot| {
            snapshot
                .sections
                .get("wallet")
                .and_then(|section| section["copper"].as_i64())
        };
        let previous = copper(before);
        let current = copper(after);
        comparison["wallet_change"] = json!({
            "before_copper":previous,"after_copper":current,
            "delta_copper":previous.zip(current).map(|(a,b)| b-a)
        });
    }
    Ok(comparison)
}

fn observed_action_bar_slots(snapshot: &Snapshot) -> Option<BTreeMap<u32, &Value>> {
    snapshot.sections.get("action_bar_observation")?["slots"]
        .as_object()?
        .iter()
        .map(|(slot, row)| Some((slot.parse().ok()?, row)))
        .collect()
}

fn compare_action_bar_placement(before: &Snapshot, after: &Snapshot, kind: &str, id: u32) -> Value {
    let previous = observed_action_bar_slots(before);
    let current = observed_action_bar_slots(after);
    let matches = |row: &Value| {
        if row["kind"] == kind {
            row["id"].as_u64().map(|value| value == u64::from(id))
        } else {
            Some(false)
        }
    };
    let matched_slots = |slots: &BTreeMap<u32, &Value>| -> Vec<u32> {
        slots
            .iter()
            .filter_map(|(&slot, row)| (matches(row) == Some(true)).then_some(slot))
            .collect()
    };
    let before_matches = previous.as_ref().map(matched_slots);
    let after_matches = current.as_ref().map(matched_slots);
    let placement_change = previous
        .as_ref()
        .zip(current.as_ref())
        .map(|(before, after)| {
            let mut gained = Vec::new();
            let mut lost = Vec::new();
            let mut unknown_before = Vec::new();
            let mut unknown_after = Vec::new();
            for &slot in after_matches.as_ref().unwrap() {
                match before.get(&slot).and_then(|row| matches(row)) {
                    Some(false) => gained.push(slot),
                    None => unknown_before.push(slot),
                    Some(true) => {}
                }
            }
            for &slot in before_matches.as_ref().unwrap() {
                match after.get(&slot).and_then(|row| matches(row)) {
                    Some(false) => lost.push(slot),
                    None => unknown_after.push(slot),
                    Some(true) => {}
                }
            }
            json!({"gained_slots":gained,"lost_slots":lost,
            "matching_after_with_unknown_before_slots":unknown_before,
            "matching_before_with_unknown_after_slots":unknown_after})
        });
    json!({(format!("{kind}_id")):id,"before_matching_slots":before_matches,
        "after_matching_slots":after_matches,"changes":placement_change,"live":false})
}

fn compare_action_bar_slots(before: &Snapshot, after: &Snapshot) -> Value {
    let previous = observed_action_bar_slots(before);
    let current = observed_action_bar_slots(after);
    let changes = previous.as_ref().zip(current.as_ref()).map(|(a, b)| {
        let shared_changes = a
            .iter()
            .filter_map(|(slot, from)| {
                let to = b.get(slot)?;
                if ["empty", "kind", "id", "sub_type"]
                    .iter()
                    .all(|field| from[*field] == to[*field])
                {
                    return None;
                }
                let status = match (from["empty"].as_bool(), to["empty"].as_bool()) {
                    (Some(true), Some(false)) => "filled",
                    (Some(false), Some(true)) => "cleared",
                    _ => "observation_changed",
                };
                Some(json!({"slot":slot,"status":status,"before":from,"after":to}))
            })
            .collect::<Vec<_>>();
        json!({"changed_shared_slots":shared_changes,
            "before_only_slots":a.keys().filter(|slot| !b.contains_key(slot)).collect::<Vec<_>>(),
            "after_only_slots":b.keys().filter(|slot| !a.contains_key(slot)).collect::<Vec<_>>()})
    });
    json!({"before_available":previous.is_some(),"after_available":current.is_some(),"changes":changes,
        "button_hotkey_changes":compare_action_bar_hotkeys(before, after)})
}

fn compare_action_bar_hotkeys(before: &Snapshot, after: &Snapshot) -> Value {
    fn buttons(snapshot: &Snapshot) -> Option<BTreeMap<&str, &Value>> {
        snapshot.sections.get("action_bar_observation")?["button_hotkeys"]
            .as_object()?
            .values()
            .map(|row| Some((row["button"].as_str()?, row)))
            .collect()
    }
    let previous = buttons(before);
    let current = buttons(after);
    let changes = previous.as_ref().zip(current.as_ref()).map(|(a,b)| {
        let shared = a.iter().filter_map(|(button, from)| {
            let to = b.get(button)?;
            let keys_changed = from["key1"] != to["key1"] || from["key2"] != to["key2"];
            let slot_changed = from["slot"] != to["slot"];
            (keys_changed || slot_changed).then(|| json!({"button":button,"keys_changed":keys_changed,
                "slot_changed":slot_changed,"before":from,"after":to}))
        }).collect::<Vec<_>>();
        json!({"changed_shared_buttons":shared,
            "before_only_buttons":a.keys().filter(|button| !b.contains_key(*button)).collect::<Vec<_>>(),
            "after_only_buttons":b.keys().filter(|button| !a.contains_key(*button)).collect::<Vec<_>>()})
    });
    json!({"before_available":previous.is_some(),"after_available":current.is_some(),"changes":changes})
}

pub fn action_bar_item_stock_audit(
    actions: &Value,
    snapshot: &Snapshot,
    target: Option<std::num::NonZeroU64>,
) -> Option<Value> {
    let slots = actions["slots"].as_object()?;
    let mut items = BTreeMap::<u64, BTreeSet<u32>>::new();
    for (slot, row) in slots.iter().filter(|(_, row)| row["kind"] == "item") {
        if let (Ok(slot), Some(id)) = (slot.parse(), row["id"].as_u64()) {
            items.entry(id).or_default().insert(slot);
        }
    }
    let equipment = snapshot
        .sections
        .get("equipment")
        .and_then(Value::as_object);
    let bags = snapshot.sections.get("bags").and_then(Value::as_object);
    let count = |observations: &mut dyn Iterator<Item = (&Value, u64)>| {
        let mut counts = BTreeMap::<u64, u64>::new();
        for (row, quantity) in observations {
            if let Some(id) = row["item_id"].as_u64().filter(|id| items.contains_key(id)) {
                *counts.entry(id).or_default() += quantity;
            }
        }
        counts
    };
    let equipped = equipment.map(|rows| count(&mut rows.values().map(|row| (row, 1))));
    let carried = bags.map(|bags| {
        count(
            &mut bags
                .values()
                .filter_map(Value::as_object)
                .flat_map(|rows| rows.values())
                .filter_map(|row| Some((row, row["count"].as_u64()?))),
        )
    });
    let hotkeys = target.and_then(|_| observed_action_hotkeys_by_slot(actions));
    Some(json!(
        items
            .into_iter()
            .map(|(id, slots)| {
                let equipment = equipped
                    .as_ref()
                    .map(|counts| counts.get(&id).copied().unwrap_or(0));
                let bags = carried
                    .as_ref()
                    .map(|counts| counts.get(&id).copied().unwrap_or(0));
                let mut stock =
                    json!({"slots":slots,"equipment_quantity":equipment,"bag_quantity":bags});
                if let Some(target) = target {
                    stock["bag_target_quantity"] = json!(target.get());
                    stock["bag_quantity_shortfall"] =
                        json!(bags.map(|quantity| target.get().saturating_sub(quantity)));
                    stock["observed_hotkeys_by_slot"] =
                        selected_action_hotkeys(&slots, hotkeys.as_ref());
                }
                (id, stock)
            })
            .collect::<BTreeMap<_, _>>()
    ))
}

fn action_slots_by_id(actions: &Value, kind: &str) -> Option<BTreeMap<u64, BTreeSet<u32>>> {
    let slots = actions["slots"].as_object()?;
    let mut slots_by_id = BTreeMap::<u64, BTreeSet<u32>>::new();
    for (slot, row) in slots.iter().filter(|(_, row)| row["kind"] == kind) {
        if let (Ok(slot), Some(id)) = (slot.parse(), row["id"].as_u64()) {
            slots_by_id.entry(id).or_default().insert(slot);
        }
    }
    Some(slots_by_id)
}

pub fn action_bar_mount_audit(actions: &Value, mounts: Option<&Value>) -> Option<Value> {
    let mounts = mounts?["entries"].as_object()?;
    let slots_by_spell = action_slots_by_id(actions, "spell")?;
    let hotkeys = observed_action_hotkeys_by_slot(actions);
    let matches: BTreeMap<_, _> = mounts.iter().filter_map(|(id, row)| {
        let spell = row["spell_id"].as_u64()?;
        let slots = slots_by_spell.get(&spell)?;
        Some((id.parse::<u32>().ok()?, json!({"spell_id":spell,"name":row["name"],
            "observed_action_slots":slots,"captured_usable":row["usable"],"captured_favorite":row["favorite"],
            "observed_hotkeys_by_slot":selected_action_hotkeys(slots, hotkeys.as_ref())})))
    }).collect();
    Some(
        json!({"basis":"exact_saved_mount_spell_id","matches_by_mount_id":matches,
        "current_mount_usability_verified":false,"complete_action_bar_coverage_verified":false,"live":false}),
    )
}

pub fn action_bar_toy_audit(actions: &Value, toys: Option<&Value>) -> Option<Value> {
    let toys = toys?;
    let entries = toys["entries"].as_object()?;
    let slots_by_item = action_slots_by_id(actions, "item")?;
    let hotkeys = observed_action_hotkeys_by_slot(actions);
    let matches: BTreeMap<_, _> = slots_by_item
        .iter()
        .filter_map(|(id, slots)| {
            let row = entries.get(&id.to_string())?;
            Some((
                id,
                json!({"name":row["name"],"observed_action_slots":slots,
            "captured_collected":row["collected"],"captured_favorite":row["favorite"],
            "observed_hotkeys_by_slot":selected_action_hotkeys(slots, hotkeys.as_ref())}),
            ))
        })
        .collect();
    Some(
        json!({"basis":"exact_saved_toy_item_id","journal_coverage":toys["coverage"],
        "matches_by_item_id":matches,"current_toy_usability_verified":false,
        "complete_action_bar_coverage_verified":false,"live":false}),
    )
}

fn observed_action_hotkeys_by_slot(actions: &Value) -> Option<BTreeMap<u32, BTreeSet<&str>>> {
    let buttons = actions["button_hotkeys"].as_object()?;
    let mut hotkeys = BTreeMap::<u32, BTreeSet<&str>>::new();
    for button in buttons.values() {
        let slot = u32::try_from(button["slot"].as_u64()?).ok()?;
        hotkeys.entry(slot).or_default().extend(
            [button["key1"].as_str(), button["key2"].as_str()]
                .into_iter()
                .flatten(),
        );
    }
    Some(hotkeys)
}

fn selected_action_hotkeys(
    slots: &BTreeSet<u32>,
    hotkeys: Option<&BTreeMap<u32, BTreeSet<&str>>>,
) -> Value {
    json!(hotkeys.map(|hotkeys| {
        slots
            .iter()
            .map(|slot| (*slot, hotkeys.get(slot)))
            .collect::<BTreeMap<_, _>>()
    }))
}

pub fn shared_action_bar_hotkeys(actions: &Value) -> Option<BTreeMap<String, BTreeSet<u64>>> {
    let buttons = actions["button_hotkeys"].as_object()?;
    let mut slots_by_key = BTreeMap::<String, BTreeSet<u64>>::new();
    for button in buttons.values() {
        let Some(slot) = button["slot"].as_u64() else {
            continue;
        };
        for key in [button["key1"].as_str(), button["key2"].as_str()]
            .into_iter()
            .flatten()
        {
            slots_by_key
                .entry(key.to_ascii_uppercase())
                .or_default()
                .insert(slot);
        }
    }
    slots_by_key.retain(|_, slots| slots.len() > 1);
    Some(slots_by_key)
}

fn spellbook_observation_conflicts<'a>(
    observations: Option<&BTreeMap<u64, Vec<Value>>>,
    fields: &[&'a str],
) -> Option<BTreeMap<u64, Vec<&'a str>>> {
    observations.map(|by_id| {
        by_id
            .iter()
            .filter_map(|(&id, rows)| {
                let conflicts: Vec<_> = fields
                    .iter()
                    .copied()
                    .filter(|field| {
                        let mut known =
                            rows.iter().map(|row| &row[*field]).filter(|v| !v.is_null());
                        known
                            .next()
                            .is_some_and(|first| known.any(|value| value != first))
                    })
                    .collect();
                (!conflicts.is_empty()).then_some((id, conflicts))
            })
            .collect()
    })
}

pub fn action_bar_spell_resource_matches(
    actions: &Value,
    spellbook: Option<&Value>,
    filter: &crate::SpellResourceFilter,
    exact_spell_id: Option<u64>,
) -> Option<Value> {
    let action_slots = action_slots_by_id(actions, "spell")?;
    let entries = spellbook?["entries"].as_object()?;
    let hotkeys = exact_spell_id
        .is_none()
        .then(|| observed_action_hotkeys_by_slot(actions))
        .flatten();
    let mut spellbook_slots = BTreeMap::<u64, (BTreeSet<u32>, BTreeSet<u32>, BTreeSet<u32>)>::new();
    for (slot, row) in entries {
        if let (Ok(slot), Some(id)) = (slot.parse(), row["spell_id"].as_u64())
            && exact_spell_id.is_none_or(|wanted| wanted == id)
            && action_slots.contains_key(&id)
        {
            let (matched, nonmatching, unknown) = spellbook_slots.entry(id).or_default();
            match filter.matches(row) {
                Some(true) => matched,
                Some(false) => nonmatching,
                None => unknown,
            }
            .insert(slot);
        }
    }
    let matches: BTreeMap<_, _> = spellbook_slots
        .into_iter()
        .filter_map(|(id, (matched, nonmatching, unknown))| {
            if matched.is_empty() {
                return None;
            }
            let actions = &action_slots[&id];
            Some((
                id,
                json!({"action_slots":actions,"matching_spellbook_slots":matched,
                    "nonmatching_spellbook_slots":nonmatching,"unknown_spellbook_slots":unknown,
                    "observed_hotkeys_by_slot":selected_action_hotkeys(actions, hotkeys.as_ref())}),
            ))
        })
        .collect();
    Some(
        json!({"matches":matches,"basis":"exact_reported_spell_id_same_capture",
        "cast_eligibility_verified":false,"live":false}),
    )
}

pub fn action_bar_spellbook(
    actions: &Value,
    spellbook: Option<&Value>,
    include_reverse: bool,
) -> Value {
    let book = spellbook.and_then(|book| book["entries"].as_object());
    let mut base_aliases = BTreeMap::<u64, BTreeSet<u64>>::new();
    let mut reported_ids = BTreeSet::new();
    for row in book.into_iter().flat_map(|rows| rows.values()) {
        if let Some((base, reported)) = row["base_spell_id"].as_u64().zip(row["spell_id"].as_u64())
        {
            base_aliases.entry(base).or_default().insert(reported);
        }
        reported_ids.extend(row["spell_id"].as_u64());
    }
    let action_slots = actions["slots"].as_object();
    let slots = action_slots.map(|slots| {
        slots
            .iter()
            .filter(|(_, row)| row["kind"] == "spell")
            .map(|(slot, row)| {
                let id = row["id"].as_u64();
                let observed = id.filter(|_| book.is_some());
                (
                    slot,
                    json!({"action_spell_id":id,
                "matches_base_spell_id":observed.map(|id| base_aliases.contains_key(&id)),
                "matches_reported_spell_id":observed.map(|id| reported_ids.contains(&id))}),
                )
            })
            .collect::<BTreeMap<_, _>>()
    });
    let unmatched_slots = slots.as_ref().filter(|_| book.is_some()).map(|slots| {
        slots
            .iter()
            .filter_map(|(slot, row)| {
                (row["matches_base_spell_id"] == false && row["matches_reported_spell_id"] == false)
                    .then(|| slot.parse::<u32>().unwrap())
            })
            .collect::<BTreeSet<_>>()
    });
    let ambiguous_aliases = slots.as_ref().filter(|_| book.is_some()).map(|slots| {
        slots
            .values()
            .filter_map(|row| {
                let id = row["action_spell_id"].as_u64()?;
                if reported_ids.contains(&id) {
                    return None;
                }
                base_aliases
                    .get(&id)
                    .filter(|aliases| aliases.len() > 1)
                    .map(|aliases| (id, aliases))
            })
            .collect::<BTreeMap<_, _>>()
    });
    let mut review = json!({"spellbook_available":book.is_some(),"spell_slots":slots,
        "ambiguous_base_aliases_by_action_id":ambiguous_aliases,
        "observed_action_spell_slots_without_spellbook_match":unmatched_slots});
    let matches = book.zip(action_slots).map(|(book, actions)| {
        let mut matches: BTreeMap<u64, BTreeSet<_>> = actions.values()
            .filter(|row| row["kind"] == "spell")
            .filter_map(|row| Some((row["id"].as_u64()?, BTreeSet::new())))
            .collect();
        for row in book.values() {
            let (Some(base), Some(reported)) = (row["base_spell_id"].as_u64(), row["spell_id"].as_u64()) else {
                continue;
            };
            let flags = (base, reported, row["passive"].as_bool(), row["off_spec"].as_bool());
            for id in [base, reported] {
                if let Some(observations) = matches.get_mut(&id) {
                    observations.insert(flags);
                }
            }
        }
        matches.into_iter().map(|(id, observations)| {
            let rows: Vec<_> = observations.into_iter().map(|(base, reported, passive, off_spec)| {
                json!({"base_spell_id":base,"spell_id":reported,"passive":passive,"off_spec":off_spec})
            }).collect();
            (id, rows)
        }).collect::<BTreeMap<_, _>>()
    });
    review["matched_spellbook_flags_by_action_id"] = json!(matches);
    let usability = book.zip(action_slots).map(|(book, actions)| {
        let action_ids: BTreeSet<_> = actions
            .values()
            .filter(|row| row["kind"] == "spell")
            .filter_map(|row| row["id"].as_u64())
            .collect();
        let mut by_id = BTreeMap::<u64, Vec<Value>>::new();
        for (slot, row) in book {
            if let Some((id, observed)) = row["spell_id"].as_u64().zip(row.get("usability"))
                && action_ids.contains(&id)
            {
                by_id.entry(id).or_default().push(json!({
                    "spellbook_slot":slot.parse::<u32>().unwrap(),
                    "usable":observed["usable"],
                    "insufficient_power":observed["insufficient_power"]
                }));
            }
        }
        for rows in by_id.values_mut() {
            rows.sort_by_key(|row| row["spellbook_slot"].as_u64().unwrap());
        }
        by_id
    });
    review["spellbook_usability_conflicts_by_action_id"] = json!(spellbook_observation_conflicts(
        usability.as_ref(),
        &["usable", "insufficient_power"],
    ));
    review["matched_spellbook_usability_by_action_id"] = json!(usability);
    if book.is_some_and(|book| {
        book.values().any(|row| {
            ["charge_count", "reported_cast_count", "cooldown_state"]
                .iter()
                .any(|field| row.get(*field).is_some())
        })
    }) {
        // Resource getters query the reported spell ID. A base/override alias
        // alone is insufficient to attach those observations to an action.
        let resources = book.zip(action_slots).map(|(book, actions)| {
            let mut by_id: BTreeMap<u64, Vec<Value>> = actions.values()
                .filter(|row| row["kind"] == "spell")
                .filter_map(|row| Some((row["id"].as_u64()?, Vec::new())))
                .collect();
            for (slot, row) in book {
                if let Some(entries) = row["spell_id"].as_u64().and_then(|id| by_id.get_mut(&id)) {
                    entries.push(json!({"spellbook_slot":slot.parse::<u32>().unwrap(),
                        "base_spell_id":row["base_spell_id"],"queried_spell_id":row["spell_id"],
                        "charge_count":row.get("charge_count"),"reported_cast_count":row.get("reported_cast_count"),
                        "cooldown_state":row.get("cooldown_state"),"cast_eligibility_verified":false}));
                }
            }
            for entries in by_id.values_mut() {
                entries.sort_by_key(|row| row["spellbook_slot"].as_u64().unwrap());
            }
            by_id
        });
        let conflicts = spellbook_observation_conflicts(
            resources.as_ref(),
            &["charge_count", "reported_cast_count", "cooldown_state"],
        );
        review["spellbook_resource_conflicts_by_action_id"] = json!(conflicts);
        review["matched_spellbook_resources_by_action_id"] = json!(resources);
    }
    if !include_reverse {
        return review;
    }
    let repeated = action_slots.map(|slots| {
        let mut by_spell = BTreeMap::<u64, Vec<u32>>::new();
        for (slot, row) in slots.iter().filter(|(_, row)| row["kind"] == "spell") {
            if let (Ok(slot), Some(id)) = (slot.parse(), row["id"].as_u64()) {
                by_spell.entry(id).or_default().push(slot);
            }
        }
        by_spell.retain(|_, slots| slots.len() > 1);
        for slots in by_spell.values_mut() {
            slots.sort_unstable();
        }
        by_spell
    });
    review["repeated_observed_action_spell_slots"] = json!(repeated);
    let unslotted = book.zip(action_slots).map(|(book, actions)| {
        let slotted = actions
            .values()
            .filter(|row| row["kind"] == "spell")
            .filter_map(|row| row["id"].as_u64())
            .collect::<BTreeSet<_>>();
        book.iter()
            .filter(|(_, row)| {
                row["passive"].as_bool() == Some(false)
                    && row["off_spec"].as_bool() == Some(false)
                    && !["base_spell_id", "spell_id"]
                        .into_iter()
                        .any(|field| row[field].as_u64().is_some_and(|id| slotted.contains(&id)))
            })
            .collect::<BTreeMap<_, _>>()
    });
    review["active_spellbook_slots_without_observed_action"] = json!(unslotted);
    review
}

fn compare_spellbook(before: &Snapshot, after: &Snapshot) -> Value {
    fn by_base(snapshot: &Snapshot) -> Option<BTreeMap<u64, BTreeSet<u64>>> {
        let rows = snapshot.sections.get("spellbook")?["entries"].as_object()?;
        let mut spells = BTreeMap::<u64, BTreeSet<u64>>::new();
        for row in rows.values() {
            spells
                .entry(row["base_spell_id"].as_u64()?)
                .or_default()
                .insert(row["spell_id"].as_u64()?);
        }
        Some(spells)
    }
    let previous = by_base(before);
    let current = by_base(after);
    let changes = previous.as_ref().zip(current.as_ref()).map(|(a,b)| {
        let observed_ids = |book: &BTreeMap<u64, BTreeSet<u64>>| -> BTreeSet<u64> {
            book.iter()
                .flat_map(|(&base, reported)| std::iter::once(base).chain(reported.iter().copied()))
                .collect()
        };
        let (before_ids, after_ids) = (observed_ids(a), observed_ids(b));
        let added = b.keys().copied().filter(|id| !a.contains_key(id)).collect::<Vec<_>>();
        let removed = a.keys().copied().filter(|id| !b.contains_key(id)).collect::<Vec<_>>();
        let changed = a.iter().filter_map(|(id, old)| {
            let new = b.get(id)?;
            (old != new).then(|| json!({"base_spell_id":id,"before_reported_spell_ids":old,"after_reported_spell_ids":new}))
        }).collect::<Vec<_>>();
        json!({"added_base_spell_ids":added,"removed_base_spell_ids":removed,"reported_spell_id_changes":changed,
            "added_observed_names_by_base_id":spellbook_names_for(after, &added),
            "removed_observed_names_by_base_id":spellbook_names_for(before, &removed),
            "after_action_slots_with_lost_spellbook_match":changed_spellbook_action_matches(&before_ids, &after_ids, after),
            "after_action_slots_with_gained_spellbook_match":changed_spellbook_action_matches(&after_ids, &before_ids, after),
            "observed_flag_changes":compare_spellbook_flags(before, after),
            "observed_usability_changes":compare_spellbook_usability(before, after),
            "observed_resource_changes":compare_spellbook_resources(before, after),
            "observed_level_learned_changes":compare_spellbook_levels(before, after)})
    });
    json!({"before_available":previous.is_some(),"after_available":current.is_some(),"changes":changes})
}

fn changed_spellbook_action_matches<'a>(
    included: &BTreeSet<u64>,
    excluded: &BTreeSet<u64>,
    snapshot: &'a Snapshot,
) -> Option<BTreeMap<&'a str, u64>> {
    let slots = snapshot.sections.get("action_bar_observation")?["slots"].as_object()?;
    Some(
        slots
            .iter()
            .filter_map(|(slot, action)| {
                let id = action["id"].as_u64()?;
                (action["kind"] == "spell" && included.contains(&id) && !excluded.contains(&id))
                    .then_some((slot.as_str(), id))
            })
            .collect(),
    )
}

fn spellbook_names_for<'a>(
    snapshot: &'a Snapshot,
    ids: &[u64],
) -> BTreeMap<u64, BTreeSet<&'a str>> {
    let mut names = BTreeMap::<u64, BTreeSet<&str>>::new();
    for row in snapshot.sections["spellbook"]["entries"]
        .as_object()
        .into_iter()
        .flat_map(|entries| entries.values())
    {
        if let Some((id, name)) = row["base_spell_id"].as_u64().zip(row["name"].as_str())
            && ids.binary_search(&id).is_ok()
        {
            names.entry(id).or_default().insert(name);
        }
    }
    names
}

fn compare_spellbook_levels(before: &Snapshot, after: &Snapshot) -> Option<Vec<Value>> {
    fn levels(snapshot: &Snapshot) -> Option<BTreeMap<(u64, u64), BTreeSet<Option<u64>>>> {
        let mut observed = BTreeMap::<_, BTreeSet<_>>::new();
        for row in snapshot.sections.get("spellbook")?["entries"]
            .as_object()?
            .values()
        {
            observed
                .entry((row["base_spell_id"].as_u64()?, row["spell_id"].as_u64()?))
                .or_default()
                .insert(row["level_learned"].as_u64());
        }
        Some(observed)
    }
    let after = levels(after)?;
    Some(
        levels(before)?
            .into_iter()
            .filter_map(|((base, id), old)| {
                let new = after.get(&(base, id))?;
                (old != *new).then(|| {
                    json!({"base_spell_id":base,"spell_id":id,
            "before_levels":old,"after_levels":new})
                })
            })
            .collect(),
    )
}

fn compare_spellbook_flags(before: &Snapshot, after: &Snapshot) -> Option<Vec<Value>> {
    type Flags = BTreeSet<(Option<bool>, Option<bool>)>;
    fn flags(snapshot: &Snapshot) -> Option<BTreeMap<(u64, u64), Flags>> {
        let rows = snapshot.sections.get("spellbook")?["entries"].as_object()?;
        let mut by_spell = BTreeMap::<_, Flags>::new();
        for row in rows.values() {
            by_spell
                .entry((row["base_spell_id"].as_u64()?, row["spell_id"].as_u64()?))
                .or_default()
                .insert((row["passive"].as_bool(), row["off_spec"].as_bool()));
        }
        Some(by_spell)
    }
    let after = flags(after)?;
    Some(
        flags(before)?
            .into_iter()
            .filter_map(|(id, old)| {
                let new = after.get(&id)?;
                let describe = |flags: &Flags| {
                    flags
                        .iter()
                        .map(|(passive, off_spec)| json!({"passive":passive,"off_spec":off_spec}))
                        .collect::<Vec<_>>()
                };
                (old != *new).then(|| {
                    json!({
                        "base_spell_id":id.0,"spell_id":id.1,
                        "before_flags":describe(&old),"after_flags":describe(new)
                    })
                })
            })
            .collect(),
    )
}

fn compare_spellbook_usability(before: &Snapshot, after: &Snapshot) -> Option<Vec<Value>> {
    compare_reported_spell_readings(
        before,
        after,
        |row| {
            row["usability"]["usable"]
                .as_bool()
                .zip(row["usability"]["insufficient_power"].as_bool())
        },
        |reading| {
            json!(
                reading.map(|(usable, power)| json!({"usable":usable,"insufficient_power":power}))
            )
        },
    )
}

fn compare_spellbook_resources(before: &Snapshot, after: &Snapshot) -> Option<Vec<Value>> {
    compare_reported_spell_readings(
        before,
        after,
        |row| {
            (
                row["charge_count"]["current"]
                    .as_u64()
                    .zip(row["charge_count"]["maximum"].as_u64()),
                row["reported_cast_count"].as_u64(),
                row["cooldown_state"]["enabled"]
                    .as_bool()
                    .zip(row["cooldown_state"]["active"].as_bool()),
            )
        },
        |(charges, casts, cooldown)| {
            json!({
                "charge_count":charges.map(|(current, maximum)| json!({"current":current,"maximum":maximum})),
                "reported_cast_count":casts,
                "cooldown_state":cooldown.map(|(enabled, active)| json!({"enabled":enabled,"active":active}))
            })
        },
    )
}

fn compare_reported_spell_readings<T: Ord>(
    before: &Snapshot,
    after: &Snapshot,
    read: impl Fn(&Value) -> T,
    describe: impl Fn(&T) -> Value,
) -> Option<Vec<Value>> {
    let by_reported_id = |snapshot: &Snapshot| -> Option<BTreeMap<u64, BTreeSet<T>>> {
        let rows = snapshot.sections.get("spellbook")?["entries"].as_object()?;
        let mut readings = BTreeMap::<_, BTreeSet<_>>::new();
        for row in rows.values() {
            readings
                .entry(row["spell_id"].as_u64()?)
                .or_default()
                .insert(read(row));
        }
        Some(readings)
    };
    let after = by_reported_id(after)?;
    Some(
        by_reported_id(before)?
            .into_iter()
            .filter_map(|(id, old)| {
                let new = after.get(&id)?;
                (old != *new).then(|| {
                    json!({"spell_id":id,
                        "before_readings":old.iter().map(&describe).collect::<Vec<_>>(),
                        "after_readings":new.iter().map(&describe).collect::<Vec<_>>()})
                })
            })
            .collect(),
    )
}

fn compare_bag_capacity_slots(before: &Snapshot, after: &Snapshot) -> Option<Vec<Value>> {
    let previous = before.sections.get("bag_capacity")?.as_object()?;
    let current = after.sections.get("bag_capacity")?.as_object()?;
    let bags: BTreeSet<u32> = previous
        .keys()
        .chain(current.keys())
        .map(|key| key.parse())
        .collect::<Result<_, _>>()
        .ok()?;
    Some(
        bags.into_iter()
            .filter_map(|bag| {
                let key = bag.to_string();
                let old = previous.get(&key);
                let new = current.get(&key);
                let delta = |field: &str| Some(new?[field].as_i64()? - old?[field].as_i64()?);
                (old != new).then(|| {
                    json!({"bag":bag,"before":old,"after":new,
            "total_slots_delta":delta("total_slots"),"free_slots_delta":delta("free_slots"),
            "same_bag_item_verified":false})
                })
            })
            .collect(),
    )
}

fn compare_bag_cooldown_item(before: &Snapshot, after: &Snapshot, item_id: u32) -> Value {
    let observe = |snapshot: &Snapshot| {
        let entries = snapshot.sections.get("bag_item_cooldowns")?["entries"].as_object()?;
        let mut reported = 0;
        let mut unavailable = 0;
        let mut enabled = 0;
        let mut zero_enabled = 0;
        let mut minimum_enabled_seconds = None::<u64>;
        for row in entries
            .values()
            .filter_map(Value::as_object)
            .flat_map(|slots| slots.values())
        {
            if row["item_id"].as_u64() != Some(u64::from(item_id)) {
                continue;
            }
            if row["status"] == "reported" {
                reported += 1;
                if row["enabled"] == true {
                    let remaining = row["remaining_seconds"].as_u64()?;
                    enabled += 1;
                    zero_enabled += usize::from(remaining == 0);
                    minimum_enabled_seconds =
                        Some(minimum_enabled_seconds.map_or(remaining, |min| min.min(remaining)));
                }
            } else {
                unavailable += 1;
            }
        }
        Some(json!({
            "matching_item_locations":reported + unavailable,
            "reported_item_locations":reported,"unavailable_item_locations":unavailable,
            "enabled_item_locations":enabled,"zero_timer_enabled_item_locations":zero_enabled,
            "minimum_enabled_remaining_seconds":minimum_enabled_seconds,
        }))
    };
    json!({"item_id":item_id,"before":observe(before),"after":observe(after),
        "same_physical_item_verified":false,"current_usability_verified":false,"live":false})
}

fn compare_pet_action_bars(before: &Snapshot, after: &Snapshot) -> Value {
    let old = before.sections.get("pet_action_bar");
    let new = after.sections.get("pet_action_bar");
    let context = |bar: &Value| {
        json!({
            "pet_present":bar["pet_present"],"has_action_bar":bar["has_action_bar"],
            "observed_slots":bar["entries"].as_object().map(|rows| rows.len())
        })
    };
    let changes = old
        .zip(new)
        .filter(|(a, b)| a["has_action_bar"] == true && b["has_action_bar"] == true)
        .map(|(a, b)| {
            (1..=10)
                .filter_map(|slot| {
                    let key = slot.to_string();
                    let old = a["entries"].get(&key);
                    let new = b["entries"].get(&key);
                    (old != new).then(|| json!({"slot":slot,"before":old,"after":new}))
                })
                .collect::<Vec<_>>()
        });
    json!({"before_context":old.map(context),"after_context":new.map(context),
        "slot_changes":changes,"same_pet_verified":false,"cast_eligibility_verified":false,"live":false})
}

fn compare_merchant_buyback(before: &Snapshot, after: &Snapshot) -> Value {
    let rows = |snapshot: &Snapshot| {
        let entries = snapshot.sections.get("merchant_buyback")?["entries"].as_object()?;
        let mut rows: Vec<_> = entries.values().cloned().collect();
        rows.sort_unstable_by_key(|row| {
            (
                row["item_id"].as_u64(),
                row["count"].as_u64(),
                row["buyback_price_copper"].as_u64(),
            )
        });
        Some(rows)
    };
    let old = rows(before);
    let new = rows(after);
    let changed = old.as_ref().zip(new.as_ref()).map(|(a, b)| a != b);
    json!({"before_entries":old,"after_entries":new,"observed_entries_changed":changed,
        "transactions_verified":false,"included_in_owned_inventory":false,"live":false})
}

fn compare_weapon_enchants(before: &Snapshot, after: &Snapshot) -> Value {
    fn rows<'a>(snapshot: &'a Snapshot, hand: &str) -> Option<Vec<&'a Value>> {
        let entries =
            snapshot.sections.get("weapon_enchants")?["hands"][hand]["entries"].as_object()?;
        let mut rows: Vec<_> = entries.values().collect();
        rows.sort_unstable_by_key(|&row| {
            (
                row["has_enchant"].as_bool(),
                row["enchant_type"].as_str(),
                row["enchant_id"].as_u64(),
                row["charges"].as_u64(),
                row["remaining_milliseconds"].as_u64(),
            )
        });
        Some(rows)
    }
    fn expiry_groups(
        rows: &[&Value],
        at: chrono::DateTime<chrono::Utc>,
    ) -> BTreeMap<u64, Vec<Option<chrono::DateTime<chrono::Utc>>>> {
        let mut groups = BTreeMap::<_, Vec<_>>::new();
        for row in rows
            .iter()
            .filter(|row| row["has_enchant"] == true && row["enchant_type"] == "temporary")
        {
            if let Some(id) = row["enchant_id"].as_u64() {
                groups
                    .entry(id)
                    .or_default()
                    .push(temporary_weapon_enchant_expiry(row, at));
            }
        }
        for expiries in groups.values_mut() {
            expiries.sort_unstable();
        }
        groups
    }
    let hands: BTreeMap<_, _> = ["main_hand", "off_hand", "ranged"]
        .into_iter()
        .map(|hand| {
            let old = rows(before, hand);
            let new = rows(after, hand);
            let changed = old.as_ref().zip(new.as_ref()).map(|(a, b)| a != b);
            let expiry_comparison = old.as_ref().zip(new.as_ref()).map(|(old, new)| {
                let old = expiry_groups(old, before.captured_at);
                let new = expiry_groups(new, after.captured_at);
                old.iter().filter_map(|(id, previous)| {
                    let current = new.get(id)?;
                    let comparable = previous.len() == current.len()
                        && previous.iter().chain(current).all(Option::is_some);
                    Some(json!({"enchant_id":id,"before_projected_expiries":previous,"after_projected_expiries":current,
                        "projected_expiries_match":comparable.then(|| previous == current)}))
                }).collect::<Vec<_>>()
            });
            (
                hand,
                json!({"before_rows":old,"after_rows":new,"observed_rows_changed":changed,
                    "shared_temporary_enchant_expiries":expiry_comparison}),
            )
        })
        .collect();
    json!({"hands":hands,"enchant_events_verified":false,"live":false})
}

fn compare_player_auras(before: &Snapshot, after: &Snapshot, spell_id: Option<u64>) -> Value {
    let applications =
        |snapshot: &Snapshot, group: &str| -> Option<BTreeMap<u64, Vec<Option<u64>>>> {
            let rows =
                snapshot.sections.get("player_auras")?["groups"][group]["entries"].as_object()?;
            let mut by_id = BTreeMap::<_, Vec<_>>::new();
            for row in rows.values() {
                let id = row["spell_id"].as_u64()?;
                if spell_id.is_some_and(|requested| requested != id) {
                    continue;
                }
                by_id
                    .entry(id)
                    .or_default()
                    .push(row["applications"].as_u64());
            }
            for counts in by_id.values_mut() {
                counts.sort_unstable();
            }
            Some(by_id)
        };
    let compare_group = |old: &BTreeMap<u64, Vec<Option<u64>>>,
                         new: &BTreeMap<u64, Vec<Option<u64>>>| {
        let ids: BTreeSet<_> = old.keys().chain(new.keys()).copied().collect();
        let mut changes = Vec::new();
        let mut changed_spell_count = 0;
        for id in ids {
            let before_counts = old.get(&id).map(Vec::as_slice).unwrap_or_default();
            let after_counts = new.get(&id).map(Vec::as_slice).unwrap_or_default();
            if before_counts != after_counts {
                changed_spell_count += 1;
                if changes.len() < 50 {
                    changes.push(json!({"spell_id":id,"before_applications":before_counts,
                        "after_applications":after_counts}));
                }
            }
        }
        json!({"changed_spell_count":changed_spell_count,"truncated":changed_spell_count>changes.len(),
            "changes":changes})
    };
    let mut result =
        json!({"aura_events_verified":false,"effectiveness_verified":false,"live":false});
    for group in ["helpful", "harmful"] {
        let old = applications(before, group);
        let new = applications(after, group);
        result[group] = json!(
            old.as_ref()
                .zip(new.as_ref())
                .map(|(a, b)| compare_group(a, b))
        );
        if let Some(id) = spell_id {
            let counts = |rows: Option<BTreeMap<u64, Vec<Option<u64>>>>| {
                rows.map(|mut rows| rows.remove(&id).unwrap_or_default())
            };
            result["selected_aura"][group] = json!({
                "before_applications":counts(old),"after_applications":counts(new)
            });
        }
    }
    if let Some(id) = spell_id {
        result["selected_aura"]["spell_id"] = json!(id);
    }
    result
}

fn tracked_recipe_membership(before: &Value, after: &Value) -> Option<Value> {
    let ids = |group: &Value| -> Option<BTreeSet<u32>> {
        group
            .as_object()?
            .keys()
            .map(|id| id.parse().ok())
            .collect()
    };
    let before = ids(before)?;
    let after = ids(after)?;
    Some(json!({
        "added_recipe_ids":after.difference(&before).collect::<Vec<_>>(),
        "removed_recipe_ids":before.difference(&after).collect::<Vec<_>>()
    }))
}

fn tracked_recipe_learned_changes(before: &Value, after: &Value) -> Option<Value> {
    let after = after.as_object()?;
    let mut changes = before
        .as_object()?
        .iter()
        .filter_map(|(key, row)| {
            let previous = row["learned"].as_bool()?;
            let current = after.get(key)?["learned"].as_bool()?;
            let id = key.parse::<u32>().ok()?;
            (previous != current).then_some((id, previous, current))
        })
        .collect::<Vec<_>>();
    changes.sort_unstable_by_key(|(id, _, _)| *id);
    Some(json!(changes.into_iter().map(|(id, previous, current)|
        json!({"recipe_id":id,"before_learned":previous,"after_learned":current})
    ).collect::<Vec<_>>()))
}

fn remaining_renown_points(detail: &serde_json::Map<String, Value>) -> Option<u64> {
    if detail.get("capped")?.as_bool()? {
        return None;
    }
    let threshold = detail.get("level_threshold")?.as_u64()?;
    let earned = detail.get("earned_in_level")?.as_u64()?;
    threshold.checked_sub(earned).filter(|_| threshold > 0)
}

fn compare_special_reputation(before: &Value, after: &Value) -> Option<Value> {
    let kind = before["kind"].as_str()?;
    if after["kind"] != kind || before["account_wide"] != after["account_wide"] {
        return None;
    }
    let a = before.get(kind)?.as_object()?;
    let b = after.get(kind)?.as_object()?;
    let delta = |field| b.get(field)?.as_i64()?.checked_sub(a.get(field)?.as_i64()?);
    let flag_changed = |field| Some(a.get(field)?.as_bool()? != b.get(field)?.as_bool()?);
    match kind {
        "renown" => {
            let level_delta = delta("level")?;
            let earned_in_level_delta = if level_delta == 0
                && a.get("level_threshold") == b.get("level_threshold")
                && remaining_renown_points(a).is_some()
                && remaining_renown_points(b).is_some()
            {
                delta("earned_in_level")
            } else {
                None
            };
            Some(
                json!({"kind":kind,"level_delta":level_delta,"earned_in_level_delta":earned_in_level_delta,
                "capped_changed":flag_changed("capped")?}),
            )
        }
        "friendship" => Some(json!({"kind":kind,"standing_delta":delta("standing")?})),
        "paragon" => Some(
            json!({"kind":kind,"reward_pending_changed":flag_changed("reward_pending")?,
            "level_restriction_changed":flag_changed("level_too_low")?}),
        ),
        _ => None,
    }
}

fn criteria_by_id(criteria: &Value) -> BTreeMap<u64, Option<&Value>> {
    let mut indexed = BTreeMap::new();
    for entry in criteria
        .as_object()
        .into_iter()
        .flat_map(|rows| rows.values())
    {
        if let Some(id) = entry["criterion_id"].as_u64() {
            indexed
                .entry(id)
                .and_modify(|value| *value = None)
                .or_insert(Some(entry));
        }
    }
    indexed
}

fn matched_achievement_criteria<'a>(
    before: &'a Value,
    after: &'a Value,
) -> impl Iterator<Item = (u64, &'a Value, &'a Value)> {
    let before = criteria_by_id(before);
    let after = criteria_by_id(after);
    before
        .into_iter()
        .filter_map(move |(id, previous)| Some((id, previous?, after.get(&id).copied().flatten()?)))
}

fn compare_criteria(before: &Value, after: &Value) -> Value {
    let mut matched = 0;
    let mut changes = Vec::new();
    for (id, previous, current) in matched_achievement_criteria(before, after) {
        matched += 1;
        if previous != current {
            let quantity_delta = (previous["required"] == current["required"])
                .then(|| Some(current["quantity"].as_i64()? - previous["quantity"].as_i64()?))
                .flatten();
            changes.push(json!({
                "criterion_id":id,"before":previous,"after":current,"quantity_delta":quantity_delta
            }));
        }
    }
    json!({"matched_criterion_count":matched,"changes":changes})
}

pub fn profession_by_skill_line(
    section: Option<&Value>,
    skill_line: u32,
) -> (&'static str, Option<&Value>) {
    let Some(professions) = section.and_then(Value::as_object) else {
        return ("unavailable", None);
    };
    let mut matching = professions
        .values()
        .filter(|entry| entry["skill_line"].as_u64() == Some(u64::from(skill_line)));
    let Some(entry) = matching.next() else {
        return ("not_observed", None);
    };
    if matching.next().is_some() {
        ("ambiguous", None)
    } else {
        ("observed", Some(entry))
    }
}

type KnownMailTotals = BTreeMap<(&'static str, u64), (u64, u32)>;

pub fn restock_pending_mail(snapshot: &Snapshot, residual: &Value) -> Option<Value> {
    let mail = snapshot.sections.get("mail_attachments")?;
    let (totals, unknown_slots) = mail_attachment_totals(mail);
    let mut items = Vec::new();
    for row in residual["items"].as_array()? {
        let item_id = row["item_id"].as_u64()?;
        let shortfall = row["shortfall_vs_observed"].as_u64()?;
        if shortfall == 0 {
            continue;
        }
        let known = totals.get(&("item", item_id));
        items.push(json!({
            "item_id":item_id,"shortfall_after_observed_bank_withdrawals":shortfall,
            "known_pending_quantity":known.map(|(quantity, _)| quantity),
            "known_attachment_slots":known.map(|(_, slots)| slots),
        }));
    }
    Some(json!({
        "coverage":"loaded_inbox_attachments","captured_at":snapshot.captured_at,
        "loaded_messages":mail["loaded_messages"],"total_messages":mail["total_messages"],
        "unloaded_messages":unloaded_mail_messages(mail),"unknown_attachment_slots":unknown_slots,
        "items":items,"counts_toward_restock_stock":false,"retrieval_eligibility_verified":false,
        "live":false,
    }))
}

fn mail_attachment_scope_totals(
    mail: &Value,
    message_slot: Option<u32>,
) -> Option<(KnownMailTotals, u32)> {
    Some(if let Some(slot) = message_slot {
        let message = mail["messages"].get(slot.to_string())?.as_object()?;
        mail_attachment_rows_totals(message.values())
    } else {
        mail_attachment_totals(mail)
    })
}

fn mail_attachment_ids(
    totals: &KnownMailTotals,
    kind: &str,
    ids: Option<&[std::num::NonZeroU32]>,
) -> Vec<u32> {
    ids.map_or_else(
        || {
            totals
                .keys()
                .filter_map(|&(observed_kind, id)| (observed_kind == kind).then_some(id as u32))
                .collect()
        },
        |ids| ids.iter().map(|id| id.get()).collect(),
    )
}

pub fn mail_attachment_bag_space(
    snapshot: &Snapshot,
    ids: Option<&[std::num::NonZeroU32]>,
    message_slot: Option<u32>,
) -> Option<Value> {
    let mail = snapshot.sections.get("mail_attachments")?;
    let (totals, unknown_slots) = mail_attachment_scope_totals(mail, message_slot)?;
    let quantities = mail_attachment_ids(&totals, "item", ids)
        .into_iter()
        .map(|id| {
            let quantity = totals
                .get(&("item", u64::from(id)))
                .map_or(0, |(quantity, _)| *quantity);
            (id, quantity)
        });
    let mut report = incoming_items_bag_space(
        snapshot,
        quantities,
        if message_slot.is_some() {
            "selected_loaded_mail_message"
        } else {
            "matching_loaded_mail_items_only"
        },
        "known_pending_quantity",
    );
    report["unloaded_messages"] = json!(unloaded_mail_messages(mail));
    report["unknown_attachment_slots"] = json!(unknown_slots);
    if let Some(slot) = message_slot {
        report["message_slot"] = json!(slot);
    }
    report["retrieval_eligibility_verified"] = json!(false);
    Some(report)
}

pub fn mail_attachment_currency_headroom(
    snapshot: &Snapshot,
    ids: Option<&[std::num::NonZeroU32]>,
    message_slot: Option<u32>,
) -> Option<Value> {
    let mail = snapshot.sections.get("mail_attachments")?;
    let (totals, unknown_slots) = mail_attachment_scope_totals(mail, message_slot)?;
    let balances = snapshot.sections.get("currency_balances");
    let currencies: Vec<_> = mail_attachment_ids(&totals, "currency", ids)
        .into_iter()
        .map(|id| {
            let pending = totals.get(&("currency", u64::from(id)));
            let balance = balances.and_then(|rows| rows["entries"].get(id.to_string()));
            let caps = balance.and_then(currency_entry_cap_headroom);
            let projection =
                pending
                    .zip(balance)
                    .zip(caps.as_ref())
                    .map(|(((quantity, _), balance), caps)| {
                        currency_reward_projection(*quantity, balance, caps)
                    });
            json!({"currency_id":id,"known_pending_quantity":pending.map(|(quantity,_)| quantity),
            "known_attachment_slots":pending.map(|(_,slots)| slots),
            "captured_balance":balance.and_then(|row| row["quantity"].as_u64()),
            "account_wide":balance.and_then(|row| row["account_wide"].as_bool()),
            "recorded_headroom":caps,"hypothetical_cap_projection":projection})
        })
        .collect();
    let mut report = json!({"currencies":currencies,"unloaded_messages":unloaded_mail_messages(mail),
        "unknown_attachment_slots":unknown_slots,"mail_cap_application_verified":false,
        "retrieval_eligibility_verified":false,"live":false});
    if let Some(slot) = message_slot {
        report["message_slot"] = json!(slot);
    }
    Some(report)
}

fn mail_attachment_totals(mail: &Value) -> (KnownMailTotals, u32) {
    mail_attachment_rows_totals(
        mail["messages"]
            .as_object()
            .unwrap()
            .values()
            .flat_map(|message| message.as_object().unwrap().values()),
    )
}

fn mail_attachment_rows_totals<'a>(
    rows: impl Iterator<Item = &'a Value>,
) -> (KnownMailTotals, u32) {
    let mut totals = KnownMailTotals::new();
    let mut unknown = 0u32;
    for row in rows {
        let (kind, field) = match row["kind"].as_str().unwrap() {
            "item" => ("item", "item_id"),
            "currency" => ("currency", "currency_id"),
            _ => {
                unknown += 1;
                continue;
            }
        };
        let entry = totals
            .entry((kind, row[field].as_u64().unwrap()))
            .or_default();
        // Validated inbox coverage bounds totals to 100 * 16 * u32::MAX.
        entry.0 += row["count"].as_u64().unwrap();
        entry.1 += 1;
    }
    (totals, unknown)
}

pub(crate) fn unloaded_mail_messages(mail: &Value) -> u64 {
    mail["total_messages"].as_u64().unwrap() - mail["loaded_messages"].as_u64().unwrap()
}

fn compare_carried_item_quantities(before: &Snapshot, after: &Snapshot, id: u32) -> Result<Value> {
    let old = find_item(before, id, 0, 1)?["quantity_by_storage"]["bags"].as_u64();
    let new = find_item(after, id, 0, 1)?["quantity_by_storage"]["bags"].as_u64();
    Ok(json!({"before_quantity":old,"after_quantity":new,
        "quantity_delta":old.zip(new).map(|(old, new)| i128::from(new) - i128::from(old)),
        "inventory_transfers_verified":false}))
}

fn compare_mail_currency_balances(before: &Snapshot, after: &Snapshot, id: u32) -> Value {
    let key = id.to_string();
    let observe = |snapshot: &Snapshot| {
        let row = snapshot.sections.get("currency_balances")?["entries"].get(&key)?;
        Some((row["quantity"].as_u64()?, row["account_wide"].as_bool()))
    };
    let before = observe(before);
    let after = observe(after);
    let same_scope = before
        .and_then(|(_, scope)| scope)
        .zip(after.and_then(|(_, scope)| scope))
        .map(|(before, after)| before == after);
    let project =
        |(quantity, account_wide)| json!({"quantity":quantity,"account_wide":account_wide});
    json!({"before":before.map(project),"after":after.map(project),"same_scope":same_scope,
        "quantity_delta":before.zip(after).filter(|_| same_scope == Some(true))
            .map(|((before, _), (after, _))| i128::from(after) - i128::from(before)),
        "mail_counts_toward_currency_balance":false,"currency_transfers_verified":false})
}

fn compare_mail_attachment_id(
    before: Option<&Value>,
    after: Option<&Value>,
    kind: &str,
    id: u32,
) -> Value {
    let observe = |mail: &Value| {
        let (totals, unknown_slots) = mail_attachment_totals(mail);
        let found = totals.get(&(kind, u64::from(id)));
        json!({"known_quantity":found.map(|(quantity, _)| quantity),
            "known_attachment_slots":found.map(|(_, slots)| slots),
            "unloaded_messages":unloaded_mail_messages(mail),"unknown_attachment_slots":unknown_slots})
    };
    let before = before.map(observe);
    let after = after.map(observe);
    let delta = before
        .as_ref()
        .zip(after.as_ref())
        .and_then(|(before, after)| {
            Some(
                i128::from(after["known_quantity"].as_u64()?)
                    - i128::from(before["known_quantity"].as_u64()?),
            )
        });
    let mut result = json!({"coverage":"loaded_inbox_attachments",
        "before":before,"after":after,"known_quantity_delta":delta,
        "deliveries_or_retrievals_verified":false,"live":false});
    result[format!("{kind}_id")] = json!(id);
    result
}

fn compare_mail_attachments(before: &Value, after: &Value) -> Value {
    let (old, old_unknown) = mail_attachment_totals(before);
    let (new, new_unknown) = mail_attachment_totals(after);
    let ids: BTreeSet<_> = old.keys().chain(new.keys()).copied().collect();
    let mut only_before = Vec::new();
    let mut only_after = Vec::new();
    let mut changes = Vec::new();
    for (kind, id) in ids {
        match (old.get(&(kind, id)), new.get(&(kind, id))) {
            (Some(a), Some(b)) if a != b => changes.push(json!({"kind":kind,"id":id,
                "before_known_quantity":a.0,"after_known_quantity":b.0,
                "known_quantity_delta":(b.0 as i64)-(a.0 as i64),
                "before_attachment_slots":a.1,"after_attachment_slots":b.1})),
            (Some(a), None) => only_before
                .push(json!({"kind":kind,"id":id,"known_quantity":a.0,"attachment_slots":a.1})),
            (None, Some(b)) => only_after
                .push(json!({"kind":kind,"id":id,"known_quantity":b.0,"attachment_slots":b.1})),
            _ => {}
        }
    }
    json!({"coverage":"loaded_inbox_attachments","message_identity":"unverified","inventory_transfers":"unverified",
        "deliveries_or_retrievals":"unverified",
        "before_loaded_messages":before["loaded_messages"],"after_loaded_messages":after["loaded_messages"],
        "before_total_messages":before["total_messages"],"after_total_messages":after["total_messages"],
        "before_unloaded_messages":unloaded_mail_messages(before),
        "after_unloaded_messages":unloaded_mail_messages(after),
        "before_unknown_attachment_slots":old_unknown,"after_unknown_attachment_slots":new_unknown,
        "observed_only_before":only_before,"observed_only_after":only_after,"shared_observation_changes":changes})
}

fn compare_pet_views(before: &Value, after: &Value) -> Value {
    let old = before["entries"].as_object().unwrap();
    let new = after["entries"].as_object().unwrap();
    let ids: BTreeSet<u32> = old
        .keys()
        .chain(new.keys())
        .map(|key| key.parse().unwrap())
        .collect();
    let mut only_before = Vec::new();
    let mut only_after = Vec::new();
    let mut changes = Vec::new();
    for id in ids {
        match (old.get(&id.to_string()), new.get(&id.to_string())) {
            (Some(a), Some(b)) => {
                let owned_before = a["owned_rows"].as_i64().unwrap();
                let owned_after = b["owned_rows"].as_i64().unwrap();
                let unowned_before = a["unowned_rows"].as_i64().unwrap();
                let unowned_after = b["unowned_rows"].as_i64().unwrap();
                if owned_before != owned_after || unowned_before != unowned_after {
                    changes.push(json!({"species_id":id,
                        "before_owned_rows":owned_before,"after_owned_rows":owned_after,
                        "owned_rows_delta":owned_after-owned_before,
                        "before_unowned_rows":unowned_before,"after_unowned_rows":unowned_after,
                        "unowned_rows_delta":unowned_after-unowned_before}));
                }
            }
            (Some(_), None) => only_before.push(id),
            (None, Some(_)) => only_after.push(id),
            _ => {}
        }
    }
    json!({"coverage":"filtered_journal_species","equal_filters":"unverified",
        "collection_gains_or_losses":"unverified",
        "before_displayed_rows":before["displayed_rows"],"after_displayed_rows":after["displayed_rows"],
        "shared_species_count":old.len()-only_before.len(),
        "shared_species_with_unchanged_row_counts":old.len()-only_before.len()-changes.len(),
        "observed_only_before":only_before,"observed_only_after":only_after,
        "observed_count_changes":changes})
}

fn compare_recipe_cooldowns(before: &Value, after: &Value) -> Value {
    let same_profession = before["profession_id"] == after["profession_id"];
    let mut result = json!({
        "same_profession":same_profession,"before_profession_id":before["profession_id"],
        "after_profession_id":after["profession_id"],"matched_recipe_count":null,
        "changed_recipe_count":null,"changes":null,"changes_truncated":null,
        "live":false,"craftability_verified":false
    });
    if !same_profession {
        return result;
    }
    let old = before["entries"].as_object().unwrap();
    let new = after["entries"].as_object().unwrap();
    let mut matched = 0;
    let mut changes = BTreeMap::new();
    for (id, previous) in old {
        let Some(current) = new.get(id) else { continue };
        matched += 1;
        if previous["cooldown"] != current["cooldown"] {
            changes.insert(id.parse::<u32>().unwrap(), (previous, current));
        }
    }
    result["matched_recipe_count"] = json!(matched);
    result["changed_recipe_count"] = json!(changes.len());
    result["changes_truncated"] = json!(changes.len() > 50);
    result["changes"] =
        json!(changes.into_iter().take(50).map(|(id, (old, new))| json!({
        "recipe_id":id,"before":old["cooldown"],"after":new["cooldown"],
        "before_state":recipe_cooldown_state(old),"after_state":recipe_cooldown_state(new)
    })).collect::<Vec<_>>());
    result
}

fn compare_profession_recipe_views(before: &Value, after: &Value) -> Value {
    let old = before["entries"].as_object().unwrap();
    let new = after["entries"].as_object().unwrap();
    let same_profession = before["profession_id"] == after["profession_id"];
    let mut result = json!({"coverage":"filtered_local_recipe_list","same_profession":same_profession,
        "before_profession_id":before["profession_id"],"after_profession_id":after["profession_id"],
        "before_observed_rows":old.len(),"after_observed_rows":new.len(),
        "observed_only_before":null,"observed_only_after":null,"learning_observation_changes":null,
        "skill_up_observation_changes":null,"rank_observation_change":null});
    if !same_profession {
        return result;
    }
    let rank_observation = |view: &Value| {
        let rank = view["rank_observation"]["rank"].as_i64()?;
        let cap = view["rank_observation"]["max_rank"].as_i64()?;
        Some((rank, cap, cap.checked_sub(rank)?))
    };
    if let Some(((old_rank, old_cap, old_remaining), (new_rank, new_cap, new_remaining))) =
        rank_observation(before).zip(rank_observation(after))
    {
        result["rank_observation_change"] = json!({
            "before":{"rank":old_rank,"max_rank":old_cap,"rank_points_to_observed_cap":old_remaining},
            "after":{"rank":new_rank,"max_rank":new_cap,"rank_points_to_observed_cap":new_remaining},
            "rank_delta":new_rank-old_rank,"max_rank_delta":new_cap-old_cap,
            "rank_points_to_observed_cap_delta":new_remaining-old_remaining,
        });
        if let Some((old_modifier, new_modifier)) = before["rank_observation"]["skill_modifier"]
            .as_i64()
            .zip(after["rank_observation"]["skill_modifier"].as_i64())
        {
            result["rank_observation_change"]["skill_modifier_change"] = json!({
                "before":old_modifier,"after":new_modifier,"delta":new_modifier-old_modifier,
            });
        }
    }
    let ids: BTreeSet<u32> = old
        .keys()
        .chain(new.keys())
        .map(|key| key.parse().unwrap())
        .collect();
    let mut only_before = Vec::new();
    let mut only_after = Vec::new();
    let mut changes = Vec::new();
    let mut skill_up_changes = Vec::new();
    for id in ids {
        match (old.get(&id.to_string()), new.get(&id.to_string())) {
            (Some(a), Some(b)) => {
                let first = a["learned"].as_bool();
                let second = b["learned"].as_bool();
                if first != second {
                    changes.push(
                        json!({"recipe_id":id,"before_learned":first,"after_learned":second}),
                    );
                }
                let fields: serde_json::Map<_, _> =
                    ["can_skill_up", "skill_ups_reported", "relative_difficulty"]
                        .into_iter()
                        .filter(|field| a[*field] != b[*field])
                        .map(|field| {
                            (
                                field.to_owned(),
                                json!({"before":a[field],"after":b[field]}),
                            )
                        })
                        .collect();
                if !fields.is_empty() {
                    skill_up_changes.push(json!({"recipe_id":id,"fields":fields}));
                }
            }
            (Some(_), None) => only_before.push(id),
            (None, Some(_)) => only_after.push(id),
            _ => {}
        }
    }
    result["observed_only_before"] = json!(only_before);
    result["observed_only_after"] = json!(only_after);
    result["learning_observation_changes"] = json!(changes);
    result["skill_up_observation_changes"] = json!(skill_up_changes);
    result
}

fn compare_saved_instances(before: &Value, after: &Value) -> Value {
    type Key = (String, u64, bool, u64);
    fn index(section: &Value) -> BTreeMap<Key, Vec<&Value>> {
        let mut rows: BTreeMap<Key, Vec<&Value>> = BTreeMap::new();
        for row in section["entries"].as_object().unwrap().values() {
            let key = (
                row["name"].as_str().unwrap().to_lowercase(),
                row["difficulty_id"].as_u64().unwrap(),
                row["is_raid"].as_bool().unwrap(),
                row["max_players"].as_u64().unwrap(),
            );
            rows.entry(key).or_default().push(row);
        }
        rows
    }
    let old = index(before);
    let new = index(after);
    let keys: BTreeSet<_> = old.keys().chain(new.keys()).collect();
    let mut changes = Vec::new();
    let mut only_before = Vec::new();
    let mut only_after = Vec::new();
    let mut ambiguous = Vec::new();
    for key in keys {
        let a = old.get(key).map(Vec::as_slice).unwrap_or_default();
        let b = new.get(key).map(Vec::as_slice).unwrap_or_default();
        let description = json!({"name_normalized":key.0,"difficulty_id":key.1,
            "is_raid":key.2,"max_players":key.3});
        if a.len() > 1 || b.len() > 1 {
            ambiguous.push(
                json!({"description":description,"before_count":a.len(),"after_count":b.len()}),
            );
        } else if a.is_empty() {
            only_after.push(description);
        } else if b.is_empty() {
            only_before.push(description);
        } else {
            let fields: serde_json::Map<_, _> = [
                "reset_seconds",
                "locked",
                "extended",
                "encounter_total",
                "reported_encounter_progress",
                "reported_extension_disabled",
            ]
            .into_iter()
            .filter_map(|field| {
                let before = a[0].get(field).filter(|value| !value.is_null())?;
                let after = b[0].get(field).filter(|value| !value.is_null())?;
                (before != after)
                    .then(|| (field.to_owned(), json!({"before":before,"after":after})))
            })
            .collect();
            if !fields.is_empty() {
                changes.push(json!({"description":description,"fields":fields}));
            }
        }
    }
    json!({"identity_basis":"public_description_not_lockout_id","changes":changes,
        "observed_only_before":only_before,"observed_only_after":only_after,"ambiguous":ambiguous,
        "before_update_age_seconds":before["update_age_seconds"],
        "after_update_age_seconds":after["update_age_seconds"]})
}

fn compare_toy_views(before: &Value, after: &Value) -> Value {
    let old = before["entries"].as_object().unwrap();
    let new = after["entries"].as_object().unwrap();
    let ids: BTreeSet<u32> = old
        .keys()
        .chain(new.keys())
        .filter_map(|key| key.parse().ok())
        .collect();
    let mut only_before = Vec::new();
    let mut only_after = Vec::new();
    let mut changes = Vec::new();
    for id in ids {
        match (old.get(&id.to_string()), new.get(&id.to_string())) {
            (Some(a), Some(b)) => {
                let fields: serde_json::Map<_, _> = ["collected", "favorite"]
                    .into_iter()
                    .filter(|field| a[*field] != b[*field])
                    .map(|field| {
                        (
                            field.to_owned(),
                            json!({"before":a[field],"after":b[field]}),
                        )
                    })
                    .collect();
                if !fields.is_empty() {
                    changes.push(json!({"item_id":id,"fields":fields}));
                }
            }
            (Some(_), None) => only_before.push(id),
            (None, Some(_)) => only_after.push(id),
            (None, None) => {}
        }
    }
    json!({"coverage":"filtered_journal","observed_only_before":only_before,
        "observed_only_after":only_after,"changes":changes,
        "shared_toy_count":old.len()-only_before.len(),
        "shared_toys_with_unchanged_flags":old.len()-only_before.len()-changes.len(),
        "before_displayed_count":before["displayed_count"],"after_displayed_count":after["displayed_count"]})
}

fn mount_flag_counts(entries: &serde_json::Map<String, Value>) -> Value {
    let usable = entries.values().filter(|row| row["usable"] == true).count();
    let favorite = entries
        .values()
        .filter(|row| row["favorite"] == true)
        .count();
    let usable_favorite = entries
        .values()
        .filter(|row| row["usable"] == true && row["favorite"] == true)
        .count();
    json!({"usable":usable,"favorite":favorite,"usable_favorite":usable_favorite})
}

fn compare_mount_collection(before: &Value, after: &Value) -> Value {
    let old = before["entries"].as_object().unwrap();
    let new = after["entries"].as_object().unwrap();
    let ids: BTreeSet<u32> = old
        .keys()
        .chain(new.keys())
        .filter_map(|key| key.parse().ok())
        .collect();
    let mut only_before = Vec::new();
    let mut only_after = Vec::new();
    let mut changes = Vec::new();
    for id in ids {
        match (old.get(&id.to_string()), new.get(&id.to_string())) {
            (Some(a), Some(b)) => {
                let fields: serde_json::Map<_, _> = ["spell_id", "usable", "favorite"]
                    .into_iter()
                    .filter(|field| a[*field] != b[*field])
                    .map(|field| {
                        (
                            field.to_owned(),
                            json!({"before":a[field],"after":b[field]}),
                        )
                    })
                    .collect();
                if !fields.is_empty() {
                    changes.push(json!({"mount_id":id,"fields":fields}));
                }
            }
            (Some(_), None) => only_before.push(id),
            (None, Some(_)) => only_after.push(id),
            (None, None) => {}
        }
    }
    json!({"observed_only_before":only_before,"observed_only_after":only_after,"changes":changes,
        "before_collected_count":old.len(),"after_collected_count":new.len(),
        "collected_count_delta":new.len() as i64 - old.len() as i64,
        "before_observed_flag_counts":mount_flag_counts(old),
        "after_observed_flag_counts":mount_flag_counts(new),
        "before_catalogue_count":before["catalogue_count"],"after_catalogue_count":after["catalogue_count"]})
}

fn compare_selected_recipe_preparation(
    before: &Snapshot,
    after: &Snapshot,
    crafts: u32,
) -> Result<Value> {
    let observe = |snapshot: &Snapshot| -> Result<Option<Value>> {
        let Some(recipe) = snapshot.sections.get("selected_recipe") else {
            return Ok(None);
        };
        let review = review_recipe_observation(snapshot, recipe, crafts, None)?;
        Ok(Some(json!({
            "recipe_id":recipe["recipe_id"],
            "checks":review["captured_preparation"],
            "cooldown_state":review["captured_cooldown_state"],
            "material_capacity":review["material_capacity"],
            "material_capacity_basis":review["material_capacity_basis"],
            "unobserved_storage":review["unobserved_storage"],
            "shortfall_vs_observed_recipe_charges":review["shortfall_vs_observed_recipe_charges"],
        })))
    };
    let old = observe(before)?;
    let new = observe(after)?;
    let same_recipe = old
        .as_ref()
        .zip(new.as_ref())
        .map(|(old, new)| old["recipe_id"] == new["recipe_id"]);
    Ok(json!({
        "requested_crafts":crafts,"before":old,"after":new,"same_recipe":same_recipe,
        "craftability_verified":false,"game_execution":false,"live":false,
    }))
}

fn compare_selected_recipe(before: Option<&Value>, after: Option<&Value>) -> Value {
    let observation = |recipe: &Value| {
        json!({
            "recipe_id":recipe["recipe_id"], "learned":recipe["learned"],
            "cooldown":recipe["cooldown"], "cooldown_state":recipe_cooldown_state(recipe),
            "requirements":recipe["requirements"], "reagents":recipe["reagents"],
            "output_item_id":recipe["output_item_id"], "base_output_quantity":recipe["base_output_quantity"]
        })
    };
    let same_recipe = before
        .zip(after)
        .map(|(old, new)| old["recipe_id"] == new["recipe_id"]);
    let old = before.map(observation);
    let new = after.map(observation);
    let changes = (same_recipe == Some(true)).then(|| {
        old.as_ref()
            .unwrap()
            .as_object()
            .unwrap()
            .iter()
            .filter_map(|(field, value)| {
                let current = &new.as_ref().unwrap()[field];
                (value != current).then(|| (field.clone(), json!({"before":value,"after":current})))
            })
            .collect::<serde_json::Map<_, _>>()
    });
    let charge_delta = (same_recipe == Some(true))
        .then(|| {
            after?["cooldown"]["charges"]
                .as_i64()?
                .checked_sub(before?["cooldown"]["charges"].as_i64()?)
        })
        .flatten();
    json!({"before":old,"after":new,"same_recipe":same_recipe,
           "changed_observations":changes,"observed_charge_delta":charge_delta})
}

fn compare_professions(before: &Value, after: &Value) -> Option<Value> {
    fn index(section: &Value) -> Option<BTreeMap<u64, Option<&Value>>> {
        let mut by_skill = BTreeMap::new();
        for row in section.as_object()?.values() {
            let id = row["skill_line"].as_u64()?;
            by_skill
                .entry(id)
                .and_modify(|entry| *entry = None)
                .or_insert(Some(row));
        }
        Some(by_skill)
    }

    let before = index(before)?;
    let after = index(after)?;
    let mut changes = Vec::new();
    let mut only_before = Vec::new();
    let mut only_after = Vec::new();
    let mut ambiguous = Vec::new();
    for id in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        match (before.get(id), after.get(id)) {
            (Some(None), _) | (_, Some(None)) => ambiguous.push(id),
            (Some(Some(old)), Some(Some(new))) => {
                let (old_rank, new_rank) = (old["rank"].as_i64()?, new["rank"].as_i64()?);
                let (old_cap, new_cap) = (old["max_rank"].as_i64()?, new["max_rank"].as_i64()?);
                if (old_rank, old_cap) != (new_rank, new_cap) {
                    changes.push(json!({
                        "skill_line":id,
                        "before_rank":old_rank,"after_rank":new_rank,"rank_delta":new_rank-old_rank,
                        "before_max_rank":old_cap,"after_max_rank":new_cap,
                    }));
                }
            }
            (Some(Some(_)), None) => only_before.push(id),
            (None, Some(Some(_))) => only_after.push(id),
            (None, None) => {}
        }
    }
    Some(json!({"changes":changes,"observed_only_before":only_before,
                "observed_only_after":only_after,"ambiguous_skill_lines":ambiguous}))
}

pub fn pet_training_points_balance(pet: &Value) -> Option<i64> {
    let points = pet.get("training_points")?;
    points["total"]
        .as_i64()?
        .checked_sub(points["used"].as_i64()?)
}

fn compare_pet_care(before: &Value, after: &Value) -> Value {
    let food_types = |pet: &Value| {
        pet["food_types"].as_object().map(|types| {
            types
                .values()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<BTreeSet<_>>()
        })
    };
    let food_type_changes = food_types(before).zip(food_types(after)).map(|(old, new)| {
        json!({
            "observed_only_before":old.difference(&new).collect::<Vec<_>>(),
            "observed_only_after":new.difference(&old).collect::<Vec<_>>(),
        })
    });
    let observation = |pet: &Value| {
        pet.as_object().map(|_| {
            let mut observed = json!({
                "present":pet["present"],"happiness":pet["happiness"],"loyalty":pet["loyalty"],
            });
            for field in ["family_id", "level", "training_points"] {
                if let Some(value) = pet.get(field) {
                    observed[field] = value.clone();
                }
            }
            if let Some(balance) = pet_training_points_balance(pet) {
                observed["training_points_balance"] = json!(balance);
            }
            observed
        })
    };
    let old_food = before["carried_food"].as_object();
    let new_food = after["carried_food"].as_object();
    let total_food = |foods: &serde_json::Map<String, Value>| {
        foods.values().filter_map(Value::as_u64).sum::<u64>()
    };
    let food_changes =
        old_food
            .zip(new_food)
            .map(|(before, after)| {
                before
                    .keys()
                    .chain(after.keys())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .filter_map(|id| {
                        let old = before.get(id).and_then(Value::as_i64).unwrap_or(0);
                        let new = after.get(id).and_then(Value::as_i64).unwrap_or(0);
                        (old != new).then(|| (id, json!({
                        "before_quantity":old,"after_quantity":new,"quantity_delta":new-old,
                        "observed_before":before.contains_key(id),"observed_after":after.contains_key(id),
                    })))
                    })
                    .collect::<BTreeMap<_, _>>()
            });
    json!({
        "before":observation(before),"after":observation(after),
        "before_accepted_carried_food_item_count":old_food.map(|foods| foods.len()),
        "after_accepted_carried_food_item_count":new_food.map(|foods| foods.len()),
        "before_accepted_carried_food_quantity":old_food.map(total_food),
        "after_accepted_carried_food_quantity":new_food.map(total_food),
        "accepted_carried_food_changes":food_changes,
        "food_type_changes":food_type_changes,
    })
}

fn compare_travel_context(old: &Snapshot, new: &Snapshot) -> Option<Value> {
    let before = old.sections.get("stats").unwrap_or(&Value::Null);
    let after = new.sections.get("stats").unwrap_or(&Value::Null);
    let map_ids = (before["map_id"].as_u64(), after["map_id"].as_u64());
    let bind_names = (
        before["hearthstone_bind_name"].as_str(),
        after["hearthstone_bind_name"].as_str(),
    );
    let item_ids = (
        before["hearthstone_item_id"].as_u64(),
        after["hearthstone_item_id"].as_u64(),
    );
    if map_ids == (None, None) && bind_names == (None, None) && item_ids == (None, None) {
        return None;
    }
    let items = item_ids.0.zip(item_ids.1);
    let cooldown_delta = items.filter(|(old, new)| old == new).and_then(|_| {
        Some(
            after["hearthstone_cooldown_seconds"].as_i64()?
                - before["hearthstone_cooldown_seconds"].as_i64()?,
        )
    });
    let active_ready_at = |stats: &Value, captured_at| {
        let seconds = stats["hearthstone_cooldown_seconds"].as_i64()?;
        (seconds > 0).then_some(())?;
        crate::saved_resets::projected_reset_at(seconds, captured_at)
    };
    let active_ready_delta = items.filter(|(old, new)| old == new).and_then(|_| {
        let before = active_ready_at(before, old.captured_at)?;
        let after = active_ready_at(after, new.captured_at)?;
        Some((after - before).num_milliseconds())
    });
    let observed = |stats: &Value, captured_at| {
        let projected_ready = stats["hearthstone_cooldown_seconds"]
            .as_i64()
            .and_then(|seconds| crate::saved_resets::projected_reset_at(seconds, captured_at));
        let map_level = stats["map_min_level"]
            .as_i64()
            .zip(stats["map_max_level"].as_i64())
            .zip(stats["level"].as_i64())
            .map(|((minimum, maximum), level)| {
                let position = map_level_position(stats);
                json!({"minimum":minimum,"maximum":maximum,"character_level":level,
                    "position_in_reported_range":position})
            });
        json!({
            "map_id":stats["map_id"],"map_name":stats["map_name"],
            "map_parent_id":stats["map_parent_id"],
            "reported_map_level_range":map_level,
            "hearthstone_bind_name":stats["hearthstone_bind_name"],
            "hearthstone_item_id":stats["hearthstone_item_id"],
            "hearthstone_cooldown_seconds":stats["hearthstone_cooldown_seconds"],
            "hearthstone_projected_ready_at":projected_ready,
        })
    };
    Some(json!({
        "before":observed(before, old.captured_at),"after":observed(after, new.captured_at),
        "map_changed":map_ids.0.zip(map_ids.1).map(|(old,new)| old != new),
        "parent_map_changed":before["map_parent_id"].as_u64().zip(after["map_parent_id"].as_u64())
            .map(|(old,new)| old != new),
        "hearthstone_bind_name_changed":bind_names.0.zip(bind_names.1).map(|(old,new)| old != new),
        "hearthstone_item_changed":items.map(|(old,new)| old != new),
        "hearthstone_cooldown_seconds_delta":cooldown_delta,
        "hearthstone_active_ready_at_delta_milliseconds":active_ready_delta,
    }))
}

fn observed_flag_changed(before: Option<&Value>, after: Option<&Value>, key: &str) -> Option<bool> {
    Some(before?.get(key)?.as_bool()? != after?.get(key)?.as_bool()?)
}

pub fn ensure_comparable(before: &Snapshot, after: &Snapshot) -> Result<()> {
    ensure_same_character_and_client(before, after)?;
    ensure!(
        before.captured_at <= after.captured_at,
        "Snapshots are reversed"
    );
    Ok(())
}

pub fn ensure_same_character_and_client(before: &Snapshot, after: &Snapshot) -> Result<()> {
    ensure!(before.client == after.client, "Client/build mismatch");
    ensure!(
        before.character.name == after.character.name
            && before.character.realm == after.character.realm,
        "Character mismatch"
    );
    Ok(())
}

fn compare_skill_lines(
    before: &serde_json::Map<String, Value>,
    after: &serde_json::Map<String, Value>,
) -> Value {
    let mut matched = 0;
    let mut changes = BTreeMap::new();
    let mut modifier_changes = BTreeMap::new();
    for (key, old) in before {
        let Some(new) = after.get(key) else { continue };
        matched += 1;
        let (Some(id), Some(rank_before), Some(rank_after), Some(cap_before), Some(cap_after)) = (
            key.parse::<u32>().ok(),
            old["rank"].as_i64(),
            new["rank"].as_i64(),
            old["max_rank"].as_i64(),
            new["max_rank"].as_i64(),
        ) else {
            continue;
        };
        if rank_before != rank_after || cap_before != cap_after {
            changes.insert(id, json!({
                "skill_line_id":id,"name":new["name"],
                "rank_before":rank_before,"rank_after":rank_after,"rank_delta":rank_after-rank_before,
                "max_rank_before":cap_before,"max_rank_after":cap_after,"max_rank_delta":cap_after-cap_before,
            }));
        }
        let mut fields = serde_json::Map::new();
        for field in ["modifier", "temporary_points"] {
            let previous = old[field].as_i64();
            let current = new[field].as_i64();
            if previous != current {
                fields.insert(
                    field.to_owned(),
                    json!({
                        "before":previous,"after":current,
                        "delta":previous.zip(current).map(|(before, after)| after - before)
                    }),
                );
            }
        }
        if !fields.is_empty() {
            modifier_changes.insert(id, json!({"skill_line_id":id,"fields":fields}));
        }
    }
    let mut result = json!({
        "matched_skill_count":matched,
        "before_only_count":before.len()-matched,
        "after_only_count":after.len()-matched,
        "changes":changes.into_values().collect::<Vec<_>>()
    });
    if !modifier_changes.is_empty() {
        result["modifier_observation_changes"] =
            json!(modifier_changes.into_values().collect::<Vec<_>>());
    }
    result
}

fn diff(a: &Value, b: &Value, path: &str, paths: &mut Vec<String>, count: &mut usize) {
    if a == b {
        return;
    }
    if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
        let keys: BTreeSet<_> = a.keys().chain(b.keys()).collect();
        for key in keys {
            let next = format!("{}/{}", path, key.replace('~', "~0").replace('/', "~1"));
            match (a.get(key), b.get(key)) {
                (Some(a), Some(b)) => diff(a, b, &next, paths, count),
                _ => record(next, paths, count),
            }
        }
    } else {
        record(path.to_owned(), paths, count);
    }
}
fn record(path: String, paths: &mut Vec<String>, count: &mut usize) {
    *count += 1;
    if paths.len() < 100 {
        paths.push(path);
    }
}

fn container_slots(s: &Snapshot, section: &str) -> Option<usize> {
    s.sections
        .get(section)
        .and_then(Value::as_object)
        .and_then(|b| {
            b.values()
                .map(|v| v.as_object().map(|v| v.len()))
                .sum::<Option<usize>>()
        })
}

#[derive(
    Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ReputationKind {
    Standard,
    Renown,
    Friendship,
    Paragon,
    Unknown,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParagonRewardStatus {
    Pending,
    PendingLevelRestricted,
    NoPendingReward,
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReputationFilter {
    pub kind: Option<ReputationKind>,
    pub account_wide: Option<bool>,
    pub capped: Option<bool>,
    pub maximum_remaining_to_next_rank: Option<u64>,
    pub paragon_reward_status: Option<ParagonRewardStatus>,
}

impl ReputationFilter {
    pub fn valid(&self) -> bool {
        let rank_kind = matches!(
            self.kind,
            Some(ReputationKind::Standard | ReputationKind::Renown | ReputationKind::Friendship)
        );
        (self.kind.is_some()
            || self.account_wide.is_some()
            || self.capped.is_some()
            || self.maximum_remaining_to_next_rank.is_some()
            || self.paragon_reward_status.is_some())
            && ((self.capped.is_none() && self.maximum_remaining_to_next_rank.is_none())
                || rank_kind)
            && (self.paragon_reward_status.is_none() || self.kind == Some(ReputationKind::Paragon))
    }

    pub fn matches(&self, entry: &Value, progress: &Value) -> Option<bool> {
        if self.kind.is_some_and(|kind| entry["kind"] != json!(kind))
            || self
                .account_wide
                .is_some_and(|expected| entry["account_wide"].as_bool() != Some(expected))
        {
            return Some(false);
        }
        let mut unknown = false;
        if let Some(expected) = self.capped {
            match progress["capped"].as_bool() {
                Some(actual) if actual != expected => return Some(false),
                Some(_) => {}
                None => unknown = true,
            }
        }
        if let Some(maximum) = self.maximum_remaining_to_next_rank {
            if progress["capped"] == true {
                return Some(false);
            }
            match progress["remaining_to_next_rank"].as_u64() {
                Some(remaining) if remaining > maximum => return Some(false),
                Some(_) => {}
                None => unknown = true,
            }
        }
        if let Some(expected) = self.paragon_reward_status {
            let expected = json!(expected);
            match progress["reward_status"].as_str() {
                Some(actual) if Some(actual) != expected.as_str() => return Some(false),
                Some(_) => {}
                None => unknown = true,
            }
        }
        if unknown { None } else { Some(true) }
    }
}

pub fn standard_reputation_progress(section: &Value) -> Value {
    let Some(entries) = section["entries"].as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries
            .iter()
            .filter_map(|(id, entry)| {
                if entry["kind"] != "standard" {
                    return None;
                }
                // Forever treats reaction 8 as capped for standard reputation.
                let reaction = entry["reaction"].as_u64()?;
                let progress = match reaction {
                    8 => json!({"capped":true}),
                    1..=7 => {
                        let standing = entry["standing"].as_i64()?;
                        let threshold = entry["threshold"].as_i64()?;
                        let next = entry["next_threshold"].as_i64()?;
                        if next <= threshold || !(threshold..=next).contains(&standing) {
                            return None;
                        }
                        json!({"capped":false,"earned_in_rank":standing.checked_sub(threshold)?,
                            "rank_width":next.checked_sub(threshold)?,
                            "remaining_to_next_rank":next.checked_sub(standing)?})
                    }
                    _ => return None,
                };
                Some((id.clone(), progress))
            })
            .collect(),
    )
}

fn standard_reputation_workload(section: &Value) -> Option<Value> {
    let entries = section["entries"].as_object()?;
    let progress = standard_reputation_progress(section);
    let mut capped = 0usize;
    let mut rows = Vec::new();
    for (key, row) in progress.as_object()? {
        if row["capped"] == true {
            capped += 1;
        } else if let Some((remaining, id)) = row["remaining_to_next_rank"]
            .as_u64()
            .zip(key.parse::<u32>().ok())
        {
            rows.push((remaining, id, &entries[key]));
        }
    }
    rows.sort_unstable_by_key(|&(remaining, id, _)| (remaining, id));
    let closest: Vec<_> = rows
        .iter()
        .take(20)
        .map(|(remaining, id, entry)| {
            json!({"faction_id":id,"name":entry["name"],"account_wide":entry["account_wide"],
            "remaining_to_next_rank":remaining})
        })
        .collect();
    Some(
        json!({"capped_factions":capped,"factions_with_observed_rank_gap":rows.len(),
        "closest_to_next_rank":closest}),
    )
}

type MerchantOfferKey = (&'static str, u64, u64);

fn merchant_offers_by_product(panel: &Value) -> Option<BTreeMap<MerchantOfferKey, Option<&Value>>> {
    let mut offers = BTreeMap::new();
    for row in panel.as_object()?.values() {
        let (kind, id) = row["item_id"]
            .as_u64()
            .map(|id| ("item", id))
            .or_else(|| row["currency_id"].as_u64().map(|id| ("currency", id)))?;
        let key = (kind, id, row["bundle_size"].as_u64()?);
        offers
            .entry(key)
            .and_modify(|row| *row = None)
            .or_insert(Some(row));
    }
    Some(offers)
}

fn compare_merchant_product_offers(
    before: Option<&Value>,
    after: Option<&Value>,
    kind: &'static str,
    id: u32,
) -> Value {
    let before = before.and_then(merchant_offers_by_product);
    let after = after.and_then(merchant_offers_by_product);
    let sizes: BTreeSet<_> = before
        .iter()
        .chain(&after)
        .flat_map(|offers| offers.keys())
        .filter_map(|&(product_kind, product_id, size)| {
            (product_kind == kind && product_id == u64::from(id)).then_some(size)
        })
        .collect();
    let observation =
        |offers: Option<&BTreeMap<MerchantOfferKey, Option<&Value>>>, size| match offers
            .map(|offers| offers.get(&(kind, u64::from(id), size)))
        {
            None => json!({"status":"not_captured"}),
            Some(None) => json!({"status":"not_observed"}),
            Some(Some(None)) => json!({"status":"ambiguous"}),
            Some(Some(Some(row))) => json!({"status":"observed","price_copper":row["price_copper"],
                "available":row["available"],"extended_cost":row["extended_cost"]}),
        };
    let bundles: Vec<_> = sizes
        .iter()
        .take(20)
        .map(|&size| {
            let previous = observation(before.as_ref(), size);
            let current = observation(after.as_ref(), size);
            let delta = previous["price_copper"]
                .as_i64()
                .zip(current["price_copper"].as_i64())
                .map(|(a, b)| b - a);
            let mut bundle = json!({"bundle_size":size,"before":previous,"after":current,"price_copper_delta":delta});
            if bundle["before"]["extended_cost"] == true || bundle["after"]["extended_cost"] == true {
                let key = (kind, u64::from(id), size);
                let before_costs = before.as_ref().and_then(|offers| offers.get(&key))
                    .copied().flatten().and_then(merchant_offer_costs);
                let after_costs = after.as_ref().and_then(|offers| offers.get(&key))
                    .copied().flatten().and_then(merchant_offer_costs);
                let changed = before_costs.as_ref().zip(after_costs.as_ref())
                    .map(|(a,b)| a != b);
                bundle["extended_cost_comparison"] =
                    json!({"before":before_costs,"after":after_costs,"recorded_components_changed":changed});
            }
            bundle
        })
        .collect();
    json!({(format!("{kind}_id")):id,"before_observed":before.is_some(),"after_observed":after.is_some(),
        "matched_bundle_count":sizes.len(),"truncated":sizes.len()>bundles.len(),"bundles":bundles,
        "merchant_identity_verified":false,"current_price_verified":false})
}

fn merchant_offer_costs(offer: &Value) -> Option<Vec<&Value>> {
    if offer["extended_cost"] == false {
        return Some(Vec::new());
    }
    let mut rows: Vec<_> = offer["cost_components"].as_object()?.values().collect();
    rows.sort_unstable_by(|a, b| {
        (
            a["item_id"].as_u64(),
            a["currency_name"].as_str(),
            a["quantity"].as_u64(),
        )
            .cmp(&(
                b["item_id"].as_u64(),
                b["currency_name"].as_str(),
                b["quantity"].as_u64(),
            ))
    });
    Some(rows)
}

fn compare_merchant_offers(before: &Value, after: &Value) -> Option<(Value, Value)> {
    let previous = merchant_offers_by_product(before)?;
    let current = merchant_offers_by_product(after)?;
    let observed_only =
        |offers: &BTreeMap<MerchantOfferKey, Option<&Value>>,
         other: &BTreeMap<MerchantOfferKey, Option<&Value>>| {
            offers.iter().filter(|(key, _)| !other.contains_key(*key))
            .map(|((kind, id, size), row)| json!({
                "product_kind":kind,"product_id":id,"bundle_size":size,"ambiguous":row.is_none()
            })).collect::<Vec<_>>()
        };
    let coverage = json!({"observed_only_before":observed_only(&previous, &current),
        "observed_only_after":observed_only(&current, &previous)});
    let changes: Vec<_> = previous.into_iter().filter_map(|((kind, id, size), old)| {
        let (old, new) = old.zip(current.get(&(kind, id, size)).copied().flatten())?;
        let old_price = old["price_copper"].as_i64()?;
        let new_price = new["price_copper"].as_i64()?;
        let (old_costs, new_costs) = (merchant_offer_costs(old), merchant_offer_costs(new));
        let components_changed = old_costs != new_costs;
        (old_price != new_price || old["available"] != new["available"]
            || old["extended_cost"] != new["extended_cost"] || components_changed).then(|| {
            let mut change = json!({
                "product_kind":kind,"product_id":id,"bundle_size":size,
                "before_price_copper":old_price,"after_price_copper":new_price,
                "price_copper_delta":new_price-old_price,
                "before_available":old["available"],"after_available":new["available"],
                "before_extended_cost":old["extended_cost"],"after_extended_cost":new["extended_cost"]
            });
            if components_changed {
                change["cost_component_change"] = json!({"before":old_costs,"after":new_costs});
            }
            change
        })
    }).collect();
    Some((json!(changes), coverage))
}

fn trainer_requirements(row: &Value) -> Value {
    let abilities = row["ability_requirements"].as_object().map(|rows| {
        let mut observed: Vec<_> = rows
            .values()
            .filter_map(|row| Some((row["name"].as_str()?, row["met"].as_bool()?)))
            .collect();
        observed.sort_unstable();
        observed
            .into_iter()
            .map(|(name, met)| json!({"name":name,"met":met}))
            .collect::<Vec<_>>()
    });
    json!({"required_level":row["required_level"],"skill_requirement":row["skill_requirement"],
        "ability_requirements":abilities})
}

fn compare_named_trainer_lesson(
    before: Option<&Value>,
    after: Option<&Value>,
    name: &str,
) -> Value {
    fn by_rank<'a>(panel: &'a Value, name: &str) -> BTreeMap<Option<&'a str>, Option<&'a Value>> {
        let mut lessons = BTreeMap::new();
        for row in panel["entries"]
            .as_object()
            .into_iter()
            .flat_map(|rows| rows.values())
        {
            if row["name"]
                .as_str()
                .is_some_and(|label| label.trim().to_lowercase() == name)
            {
                lessons
                    .entry(row["rank_label"].as_str())
                    .and_modify(|row| *row = None)
                    .or_insert(Some(row));
            }
        }
        lessons
    }
    let previous = before.map(|panel| by_rank(panel, name));
    let current = after.map(|panel| by_rank(panel, name));
    let ranks: BTreeSet<_> = previous
        .iter()
        .chain(&current)
        .flat_map(|rows| rows.keys().copied())
        .collect();
    let observation =
        |rows: Option<&BTreeMap<Option<&str>, Option<&Value>>>, rank, panel: Option<&Value>| {
            match rows.map(|rows| rows.get(&rank)) {
                None => json!({"status":"not_captured"}),
                Some(None) => json!({"status":"not_observed"}),
                Some(Some(None)) => json!({"status":"ambiguous"}),
                Some(Some(Some(row))) => {
                    let pet = panel.is_some_and(|panel| panel["kind"] == "pet");
                    let (field, unit) = if pet {
                        ("price_training_points", "training_points")
                    } else {
                        ("price_copper", "copper")
                    };
                    let mut observation = json!({"status":"observed","lesson_status":row["status"],
                    "price":row[field],"price_unit":unit});
                    let requirements = trainer_requirements(row);
                    if requirements
                        .as_object()
                        .is_some_and(|fields| fields.values().any(|value| !value.is_null()))
                    {
                        observation["requirements"] = requirements;
                    }
                    observation
                }
            }
        };
    let before_kind = before.and_then(|panel| panel["kind"].as_str());
    let after_kind = after.and_then(|panel| panel["kind"].as_str());
    let same_kind = before_kind.zip(after_kind).map(|(a, b)| a == b);
    let lessons: Vec<_> = ranks
        .iter()
        .take(20)
        .map(|&rank| {
            let a = observation(previous.as_ref(), rank, before);
            let b = observation(current.as_ref(), rank, after);
            let delta = a["price"]
                .as_i64()
                .zip(b["price"].as_i64())
                .filter(|_| same_kind == Some(true))
                .map(|(a, b)| b - a);
            let mut lesson = json!({"rank_label":rank,"before":a,"after":b,"price_delta":delta});
            let previous = &lesson["before"]["requirements"];
            let current = &lesson["after"]["requirements"];
            if !previous.is_null() || !current.is_null() {
                let changed = |field: &str| {
                    let (a, b) = (&previous[field], &current[field]);
                    (same_kind == Some(true) && !a.is_null() && !b.is_null()).then(|| a != b)
                };
                lesson["requirement_changes"] = json!({
                    "required_level":changed("required_level"),
                    "skill_requirement":changed("skill_requirement"),
                    "ability_requirements":changed("ability_requirements"),
                });
            }
            lesson
        })
        .collect();
    json!({"before_kind":before_kind,"after_kind":after_kind,"same_kind":same_kind,
        "matched_rank_count":ranks.len(),"truncated":ranks.len()>lessons.len(),"lessons":lessons,
        "trainer_identity_verified":false,"current_price_verified":false})
}

fn compare_trainer_services(before: &Value, after: &Value) -> Option<Value> {
    if before["kind"] != after["kind"] {
        return None;
    }
    fn by_name(panel: &Value) -> Option<BTreeMap<(&str, Option<&str>), Option<&Value>>> {
        let mut lessons = BTreeMap::new();
        for row in panel["entries"].as_object()?.values() {
            let key = (row["name"].as_str()?, row["rank_label"].as_str());
            lessons
                .entry(key)
                .and_modify(|row| *row = None)
                .or_insert(Some(row));
        }
        Some(lessons)
    }
    let previous = by_name(before)?;
    let current = by_name(after)?;
    let observed_only =
        |entries: &BTreeMap<(&str, Option<&str>), Option<&Value>>,
         other: &BTreeMap<(&str, Option<&str>), Option<&Value>>| {
            entries
                .iter()
                .filter(|(key, _)| !other.contains_key(*key))
                .map(|((name, rank), row)| {
                    json!({
                        "name":name,"rank_label":rank,"ambiguous":row.is_none()
                    })
                })
                .collect::<Vec<_>>()
        };
    let observed_only_before = observed_only(&previous, &current);
    let observed_only_after = observed_only(&current, &previous);
    let (price_field, price_unit) = if after["kind"] == "pet" {
        ("price_training_points", "training_points")
    } else {
        ("price_copper", "copper")
    };
    let mut status_changes = Vec::new();
    let mut price_changes = Vec::new();
    let mut requirement_changes = Vec::new();
    for ((name, rank), old) in previous {
        let Some((old, new)) = old.zip(current.get(&(name, rank)).copied().flatten()) else {
            continue;
        };
        if old["status"] != new["status"] {
            status_changes.push(json!({"name":name,"rank_label":rank,
                "before_status":old["status"],"after_status":new["status"]}));
        }
        let (old_price, new_price) = (old[price_field].as_i64(), new[price_field].as_i64());
        if old_price != new_price {
            price_changes.push(
                json!({"name":name,"rank_label":rank,"before_price":old_price,
                "after_price":new_price,"price_delta":old_price.zip(new_price).map(|(a,b)| b-a)}),
            );
        }
        let (before_requirements, after_requirements) =
            (trainer_requirements(old), trainer_requirements(new));
        if before_requirements != after_requirements {
            requirement_changes.push(json!({"name":name,"rank_label":rank,
                "before":before_requirements,"after":after_requirements}));
        }
    }
    Some(json!({"kind":after["kind"],"status_changes":status_changes,
        "price_unit":price_unit,"price_changes":price_changes,
        "requirement_changes":requirement_changes,
        "lessons_observed_only_before":observed_only_before,
        "lessons_observed_only_after":observed_only_after}))
}

fn paragon_reward_status(detail: &serde_json::Map<String, Value>) -> Option<&'static str> {
    let pending = detail.get("reward_pending")?.as_bool()?;
    let restricted = detail.get("level_too_low")?.as_bool()?;
    Some(match (pending, restricted) {
        (true, false) => "pending",
        (true, true) => "pending_level_restricted",
        (false, _) => "no_pending_reward",
    })
}

fn paragon_reward_summary(
    section: &Value,
    quests: Option<&serde_json::Map<String, Value>>,
) -> Option<Value> {
    let entries = section["entries"].as_object()?;
    let mut pending = BTreeMap::new();
    let mut restricted = BTreeMap::new();
    let mut unknown = Vec::new();
    for (key, entry) in entries.iter().filter(|(_, row)| row["kind"] == "paragon") {
        let Ok(id) = key.parse::<u32>() else { continue };
        let detail = &entry["paragon"];
        let group = match detail.as_object().and_then(paragon_reward_status) {
            Some("pending") => &mut pending,
            Some("pending_level_restricted") => &mut restricted,
            Some(_) => continue,
            None => {
                unknown.push(id);
                continue;
            }
        };
        let quest_key = detail["reward_quest_id"].as_u64().map(|id| id.to_string());
        let quest_lookup = quests
            .zip(quest_key.as_deref())
            .map(|(quests, key)| quests.get(key));
        group.insert(
            id,
            json!({"name":entry["name"],"account_wide":entry["account_wide"],
            "reward_quest_id":detail["reward_quest_id"],
            "reward_quest_observed":quest_lookup.map(|quest| quest.is_some()),
            "reward_quest_readiness":quest_lookup.flatten().map(quest_readiness)}),
        );
    }
    unknown.sort_unstable();
    Some(
        json!({"pending_by_faction_id":pending,"level_restricted_by_faction_id":restricted,
        "unknown_faction_ids":unknown}),
    )
}

pub fn special_reputation_progress(section: &Value) -> Value {
    let Some(entries) = section["entries"].as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries
            .iter()
            .filter_map(|(id, entry)| {
                let kind = entry["kind"].as_str()?;
                let detail = entry.get(kind)?.as_object()?;
                let progress = match kind {
                    "renown" => {
                        let capped = detail.get("capped")?.as_bool()?;
                        let remaining = remaining_renown_points(detail);
                        json!({"kind":kind,"capped":capped,"remaining_to_next_rank":remaining})
                    }
                    "friendship" => {
                        let next = detail.get("next_threshold").and_then(Value::as_i64);
                        let remaining = next.and_then(|next| {
                            let threshold = detail.get("threshold")?.as_i64()?;
                            let standing = detail.get("standing")?.as_i64()?;
                            if next <= threshold || !(threshold..=next).contains(&standing) {
                                return None;
                            }
                            next.checked_sub(standing)
                        });
                        json!({"kind":kind,"capped":next.is_none(),"remaining_to_next_rank":remaining})
                    }
                    "paragon" => {
                        let status = paragon_reward_status(detail)?;
                        json!({"kind":kind,"reward_status":status})
                    }
                    _ => return None,
                };
                Some((id.clone(), progress))
            })
            .collect(),
    )
}

fn weekly_cap_remaining(entry: &Value) -> Option<u64> {
    let cap = entry["max_weekly_quantity"]
        .as_u64()
        .filter(|cap| *cap > 0)?;
    Some(cap.saturating_sub(entry["earned_this_week"].as_u64()?))
}

pub(crate) fn currency_entry_cap_headroom(entry: &Value) -> Option<Value> {
    let weekly = weekly_cap_remaining(entry);
    let basis = if entry["cap_uses_total_earned"].as_bool()? {
        "total_earned"
    } else {
        "quantity"
    };
    let maximum = entry["max_quantity"]
        .as_u64()
        .filter(|limit| *limit > 0)
        .zip(entry[basis].as_u64())
        .map(|(limit, used)| limit.saturating_sub(used));
    (weekly.is_some() || maximum.is_some()).then(|| {
        json!({
            "weekly_remaining":weekly,"maximum_remaining":maximum,
            "maximum_basis":maximum.map(|_| basis),
        })
    })
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CurrencyFilter {
    pub account_wide: Option<bool>,
    pub inactive: Option<bool>,
    pub can_earn_per_week: Option<bool>,
    pub has_weekly_cap: Option<bool>,
    pub has_total_cap: Option<bool>,
    pub maximum_weekly_remaining: Option<u64>,
    pub maximum_total_remaining: Option<u64>,
}

impl CurrencyFilter {
    pub fn is_empty(&self) -> bool {
        self.account_wide.is_none()
            && self.inactive.is_none()
            && self.can_earn_per_week.is_none()
            && self.has_weekly_cap.is_none()
            && self.has_total_cap.is_none()
            && self.maximum_weekly_remaining.is_none()
            && self.maximum_total_remaining.is_none()
    }

    pub fn matches(&self, entry: &Value, headroom: &Value) -> Option<bool> {
        let mut unknown = false;
        for (actual, expected) in [
            (entry["account_wide"].as_bool(), self.account_wide),
            (entry["inactive"].as_bool(), self.inactive),
            (entry["can_earn_per_week"].as_bool(), self.can_earn_per_week),
            (
                entry["max_weekly_quantity"].as_u64().map(|cap| cap > 0),
                self.has_weekly_cap,
            ),
            (
                entry["max_quantity"].as_u64().map(|cap| cap > 0),
                self.has_total_cap,
            ),
        ] {
            if let Some(expected) = expected {
                match actual {
                    Some(actual) if actual != expected => return Some(false),
                    Some(_) => {}
                    None => unknown = true,
                }
            }
        }
        for (maximum, cap_field, remaining_field) in [
            (
                self.maximum_weekly_remaining,
                "max_weekly_quantity",
                "weekly_remaining",
            ),
            (
                self.maximum_total_remaining,
                "max_quantity",
                "maximum_remaining",
            ),
        ] {
            if let Some(maximum) = maximum {
                if entry[cap_field].as_u64() == Some(0) {
                    return Some(false);
                }
                match headroom[remaining_field].as_u64() {
                    Some(remaining) if remaining > maximum => return Some(false),
                    Some(_) => {}
                    None => unknown = true,
                }
            }
        }
        if unknown { None } else { Some(true) }
    }
}

pub fn currency_cap_headroom(section: &Value) -> Value {
    let Some(entries) = section["entries"].as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries
            .iter()
            .filter_map(|(id, entry)| {
                currency_entry_cap_headroom(entry).map(|headroom| (id.clone(), headroom))
            })
            .collect(),
    )
}

fn currency_weekly_cap_summary(section: &Value) -> Option<Value> {
    let entries = section["entries"].as_object()?;
    let mut rows = Vec::new();
    for (id, entry) in entries {
        let (Ok(id), Some(cap), Some(earned)) = (
            id.parse::<u32>(),
            entry["max_weekly_quantity"].as_u64(),
            entry["earned_this_week"].as_u64(),
        ) else {
            continue;
        };
        if cap > 0 {
            rows.push((id, cap, earned, cap.saturating_sub(earned), entry));
        }
    }
    rows.sort_unstable_by(|a, b| {
        (u128::from(a.3) * u128::from(b.1))
            .cmp(&(u128::from(b.3) * u128::from(a.1)))
            .then_with(|| a.0.cmp(&b.0))
    });
    let closest: Vec<_> = rows
        .iter()
        .take(20)
        .map(|(id, cap, earned, remaining, entry)| {
            json!({"currency_id":id,"name":entry["name"],"weekly_cap":cap,
            "earned_this_week":earned,"weekly_remaining":remaining,
            "inactive":entry["inactive"],"can_earn_per_week":entry["can_earn_per_week"]})
        })
        .collect();
    Some(json!({"weekly_caps_observed":rows.len(),
        "weekly_caps_reached":rows.iter().filter(|row| row.3 == 0).count(),
        "closest_weekly_caps":closest}))
}

pub fn currency_reward_projection(proposed: u64, entry: &Value, caps: &Value) -> Value {
    let maximum = caps["maximum_remaining"].as_u64();
    let weekly = caps["weekly_remaining"]
        .as_u64()
        .filter(|_| entry["can_earn_per_week"].as_bool() != Some(false));
    let limit = maximum.into_iter().chain(weekly).min();
    json!({
        "proposed_quantity":proposed,
        "quantity_within_recorded_headroom":limit.map(|cap| proposed.min(cap)),
        "quantity_above_recorded_headroom":limit.map(|cap| proposed.saturating_sub(cap))
    })
}

fn compare_quest_reward_entries(
    before: Option<&Value>,
    after: Option<&Value>,
    id_field: &str,
) -> Option<Value> {
    let quantities_by_id = |entries: &Value| {
        let mut quantities = BTreeMap::<u64, Vec<u64>>::new();
        for row in entries.as_object()?.values() {
            quantities
                .entry(row[id_field].as_u64()?)
                .or_default()
                .push(row["quantity"].as_u64()?);
        }
        for values in quantities.values_mut() {
            values.sort_unstable();
        }
        Some(quantities)
    };
    let previous = before.and_then(quantities_by_id);
    let current = after.and_then(quantities_by_id);
    if previous.is_none() && current.is_none() {
        return None;
    }
    let ids: BTreeSet<_> = previous
        .iter()
        .chain(&current)
        .flat_map(|rows| rows.keys())
        .collect();
    let changes: Vec<_> = ids
        .into_iter()
        .filter_map(|id| {
            let a = previous
                .as_ref()
                .map(|rows| rows.get(id).map(Vec::as_slice).unwrap_or_default());
            let b = current
                .as_ref()
                .map(|rows| rows.get(id).map(Vec::as_slice).unwrap_or_default());
            let changed = a.zip(b).map(|(a, b)| a != b);
            (changed != Some(false)).then(|| {
                json!({
                    (id_field):id,"before_quantities":a,"after_quantities":b,"changed":changed
                })
            })
        })
        .collect();
    Some(
        json!({"before_observed":previous.is_some(),"after_observed":current.is_some(),"changes":changes}),
    )
}

pub fn quest_list_item_rewards(quests: &serde_json::Map<String, Value>) -> Value {
    let mut totals = BTreeMap::<u64, u64>::new();
    let mut unknown_fixed = 0;
    let mut unknown_choices = 0;
    let mut quests_with_choices = Vec::new();
    for (id, quest) in quests {
        match quest["item_choices"].as_object() {
            Some(choices) if !choices.is_empty() => quests_with_choices.push(id),
            Some(_) => (),
            None => unknown_choices += 1,
        }
        let Some(rows) = quest["item_rewards"].as_object() else {
            unknown_fixed += 1;
            continue;
        };
        for row in rows.values() {
            if let (Some(id), Some(quantity)) = (row["item_id"].as_u64(), row["quantity"].as_u64())
            {
                *totals.entry(id).or_default() += quantity;
            }
        }
    }
    json!({"known_fixed_quantity_by_item_id":totals,
        "unknown_fixed_reward_list_count":unknown_fixed,
        "quests_with_observed_item_choices":quests_with_choices,
        "unknown_choice_list_count":unknown_choices})
}

pub fn quest_list_currency_projection(
    quests: &serde_json::Map<String, Value>,
    balances: &Value,
) -> Value {
    let mut totals = BTreeMap::<String, u128>::new();
    let mut unknown = 0;
    for quest in quests.values() {
        let Some(rows) = quest["currency_rewards"].as_object() else {
            unknown += 1;
            continue;
        };
        for row in rows.values() {
            if let (Some(id), Some(quantity)) =
                (row["currency_id"].as_u64(), row["quantity"].as_u64())
            {
                *totals.entry(id.to_string()).or_default() += u128::from(quantity);
            }
        }
    }
    let headroom = currency_cap_headroom(balances);
    let projections: BTreeMap<_, _> = totals
        .into_iter()
        .map(|(id, total)| {
            let projection = u64::try_from(total).ok().map(|quantity| {
                currency_reward_projection(quantity, &balances["entries"][&id], &headroom[&id])
            });
            (id, projection)
        })
        .collect();
    json!({"known_fixed_rewards":projections,"unknown_fixed_reward_list_count":unknown})
}

pub enum QuestRewardTarget<'a> {
    Item(u32, Option<&'a Value>),
    Currency(u32, &'a Value, Option<&'a Value>),
    Renown(u32, &'a Value, Option<&'a Value>),
}

pub fn quest_ready_reward_projection(
    quests: &serde_json::Map<String, Value>,
    reward: QuestRewardTarget<'_>,
    target: u64,
    include_choices: bool,
) -> Value {
    let (product_id, id_field, fixed_field, choice_field, coverage_field) = match reward {
        QuestRewardTarget::Item(id, _) => (
            id,
            "item_id",
            "item_rewards",
            Some("item_choices"),
            "covers_target",
        ),
        QuestRewardTarget::Currency(id, _, _) => (
            id,
            "currency_id",
            "currency_rewards",
            Some("currency_choices"),
            "covers_target_before_caps",
        ),
        QuestRewardTarget::Renown(id, _, _) => {
            (id, "faction_id", "renown_rewards", None, "covers_target")
        }
    };
    let choice_field = choice_field.filter(|_| include_choices);
    let mut unknown_fixed = 0;
    let mut unknown_choices = 0;
    let mut fixed_total = 0;
    let mut rewards = Vec::new();
    for (id, quest) in quests {
        if quest_readiness(quest) != QuestReadiness::Ready {
            continue;
        }
        let fixed = quest[fixed_field].as_object();
        unknown_fixed += usize::from(fixed.is_none());
        let fixed_quantity: u64 = fixed
            .into_iter()
            .flat_map(|rows| rows.values())
            .filter(|row| row[id_field].as_u64() == Some(u64::from(product_id)))
            .filter_map(|row| row["quantity"].as_u64())
            .sum();
        fixed_total += fixed_quantity;
        let choice = if let Some(choice_field) = choice_field {
            let choices = quest[choice_field].as_object();
            unknown_choices += usize::from(choices.is_none());
            choices
                .into_iter()
                .flat_map(|rows| rows.iter())
                .filter(|(_, row)| row[id_field].as_u64() == Some(u64::from(product_id)))
                .filter_map(|(slot, row)| {
                    Some((row["quantity"].as_u64()?, slot.parse::<u32>().ok()?))
                })
                .max_by_key(|&(quantity, slot)| (quantity, std::cmp::Reverse(slot)))
        } else {
            None
        };
        let quantity = fixed_quantity + choice.map_or(0, |(quantity, _)| quantity);
        if quantity > 0
            && let Ok(id) = id.parse::<u32>()
        {
            rewards.push((std::cmp::Reverse(quantity), id, fixed_quantity, choice));
        }
    }
    rewards.sort_unstable_by_key(|(quantity, id, _, _)| (*quantity, *id));
    let total: u64 = rewards.iter().map(|(quantity, _, _, _)| quantity.0).sum();
    let shortfall = target.saturating_sub(total);
    let covers_target = if shortfall == 0 {
        Some(true)
    } else if unknown_fixed == 0 && unknown_choices == 0 {
        Some(false)
    } else {
        None
    };
    let plan = (shortfall == 0).then(|| {
        let mut ids = Vec::new();
        let mut selected_quantity = 0;
        let mut selected_fixed = 0;
        let mut choices = Vec::new();
        for (quantity, id, fixed, choice) in rewards {
            if selected_quantity >= target {
                break;
            }
            selected_quantity += quantity.0;
            selected_fixed += fixed;
            if ids.len() < 20 && let Some((quantity, position)) = choice {
                choices.push(json!({"quest_id":id,"position":position,"quantity":quantity}));
            }
            ids.push((id, quantity.0));
        }
        let mut plan = json!({"quest_count":ids.len(),"quest_ids":ids.iter().take(20).map(|(id, _)| id).collect::<Vec<_>>(),
            "truncated":ids.len()>20,"recorded_fixed_quantity":selected_fixed});
        let currency_caps = if let QuestRewardTarget::Currency(id, balances, _) = reward {
            let entry = &balances["entries"][id.to_string()];
            let caps = currency_entry_cap_headroom(entry).unwrap_or(Value::Null);
            plan["cap_projection"] = currency_reward_projection(selected_quantity, entry, &caps);
            Some((entry, caps))
        } else {
            None
        };
        if let QuestRewardTarget::Renown(_, faction, _) = reward {
            plan["renown_projection"] = renown_reward_projection(selected_quantity, faction);
        }
        let (wallet, quantity_field, shortfall_field) = match reward {
            QuestRewardTarget::Currency(_, _, wallet) => (wallet,
                "currency_from_affordable_prefix_before_caps", "currency_shortfall_after_affordable_prefix_before_caps"),
            QuestRewardTarget::Renown(_, _, wallet) => (wallet,
                "renown_from_affordable_prefix", "renown_shortfall_after_affordable_prefix"),
            QuestRewardTarget::Item(_, wallet) => (wallet,
                "items_from_affordable_prefix", "item_shortfall_after_affordable_prefix"),
        };
            let selected: BTreeMap<_, _> = ids.iter().map(|(id, points)| (id.to_string(), points)).collect();
            let balance = wallet.and_then(|wallet| wallet["copper"].as_u64());
            plan["selected_quests_wallet_projection"] = json!(quest_wallet_turn_in_order(
                selected.keys().filter_map(|id| quests.get_key_value(id)), balance,
            ).map(|mut order| {
                let prefix_points = order["captured_wallet_progress"]["affordable_prefix_count"]
                    .as_u64().and_then(|count| {
                        order["quest_ids"].as_array()?.iter().take(usize::try_from(count).ok()?)
                            .try_fold(0u64, |total, id| total.checked_add(**selected.get(id.as_str()?)?))
                    });
                if let Some(ordered_ids) = order["quest_ids"].as_array_mut() {
                    let truncated = ordered_ids.len() > 20;
                    ordered_ids.truncate(20);
                    order["quest_ids_truncated"] = json!(truncated);
                }
                order[quantity_field] = json!(prefix_points);
                order[shortfall_field] =
                    json!(prefix_points.map(|points| target.saturating_sub(points)));
                if let Some((entry, caps)) = currency_caps {
                    let projection = prefix_points.map(|points| currency_reward_projection(points, entry, &caps));
                    let within_headroom = projection.as_ref().and_then(|value| {
                        value["quantity_within_recorded_headroom"].as_u64()
                    });
                    order["affordable_prefix_cap_projection"] = json!(projection);
                    order["currency_shortfall_after_affordable_prefix_within_recorded_headroom"] =
                        json!(within_headroom.map(|quantity| target.saturating_sub(quantity)));
                }
                order
            }));
        if let Some(choice_field) = choice_field {
            plan["recorded_choice_quantity"] = json!(selected_quantity - selected_fixed);
            plan["recorded_total_quantity"] = json!(selected_quantity);
            plan[choice_field] = json!(choices);
        }
        plan
    });
    let mut report = json!({(id_field):product_id,"target_quantity":target,
        "known_ready_fixed_quantity":fixed_total,"ready_quests_with_unknown_fixed_rewards":unknown_fixed,
        (coverage_field):covers_target,
        "shortfall_after_known_ready_fixed_rewards":target.saturating_sub(fixed_total),
        "ready_turn_in_plan":plan,"reward_eligibility_verified":false,"live":false});
    if choice_field.is_some() {
        report["known_ready_quantity_with_choices"] = json!(total);
        report["ready_quests_with_unknown_choices"] = json!(unknown_choices);
        report["shortfall_after_known_ready_rewards"] = json!(shortfall);
    }
    report
}

pub fn quest_item_reward_bag_space(
    snapshot: &Snapshot,
    item_id: u32,
    plan: &Value,
) -> Option<Value> {
    let quantity = plan["recorded_total_quantity"]
        .as_u64()
        .or_else(|| plan["recorded_fixed_quantity"].as_u64())?;
    Some(quest_item_reward_quantity_bag_space(
        snapshot, item_id, quantity,
    ))
}

pub fn quest_item_reward_quantity_bag_space(
    snapshot: &Snapshot,
    item_id: u32,
    quantity: u64,
) -> Value {
    let mut report = carried_item_quantity_bag_space(snapshot, item_id, quantity);
    report["scope"] = json!("selected_target_item_rewards_only");
    report["recorded_reward_quantity"] = json!(quantity);
    report
}

pub fn recipe_output_bag_space(snapshot: &Snapshot, output: &Value) -> Option<Value> {
    let item_id = u32::try_from(output["item_id"].as_u64()?).ok()?;
    let minimum = output["minimum"].as_u64()?;
    let maximum = output["maximum"].as_u64()?;
    Some(json!({
        "scope":"base_output_against_captured_bags_before_crafting",
        "item_id":item_id,
        "minimum_output":carried_item_quantity_bag_space(snapshot, item_id, minimum),
        "maximum_output":carried_item_quantity_bag_space(snapshot, item_id, maximum),
        "reagent_slots_freed_included":false,
        "bonus_output_included":false,
    }))
}

pub fn combined_recipe_output_bag_space(snapshot: &Snapshot, outputs: &Value) -> Value {
    let items = outputs["items"].as_array().into_iter().flatten();
    let projections: Vec<_> = items
        .map(|output| recipe_output_bag_space(snapshot, output))
        .collect();
    let missing_outputs = outputs["recipes_without_observed_output_item"].as_u64();
    let free_slots =
        carried_bag_space(snapshot).and_then(|space| space["known_general_free_slots"].as_u64());
    let summarize = |bound: &str| {
        let total = |field: &str| {
            if missing_outputs != Some(0) || projections.is_empty() {
                return None;
            }
            projections.iter().try_fold(0u64, |sum, projection| {
                sum.checked_add(projection.as_ref()?[bound][field].as_u64()?)
            })
        };
        let slots = total("estimated_new_stack_slots");
        let with_merges = total("estimated_new_stack_slots_if_same_item_stacks_merge");
        json!({"estimated_new_stack_slots":slots,
            "fits_recorded_general_slots":slots.zip(free_slots).map(|(needed, free)| needed <= free),
            "estimated_new_stack_slots_if_same_item_stacks_merge":with_merges,
            "fits_recorded_general_slots_if_same_item_stacks_merge":with_merges.zip(free_slots)
                .map(|(needed, free)| needed <= free)})
    };
    json!({"scope":"combined_base_output_against_captured_bags_before_crafting",
        "recorded_general_free_slots":free_slots,
        "recipes_without_observed_output_item":missing_outputs,
        "output_items_without_observed_range":projections.iter().filter(|row| row.is_none()).count(),
        "minimum_output":summarize("minimum_output"),"maximum_output":summarize("maximum_output"),
        "reagent_slots_freed_included":false,"bonus_output_included":false,
        "same_item_stack_merge_verified":false,"placement_verified":false,"live":false})
}

fn carried_item_quantity_bag_space(snapshot: &Snapshot, item_id: u32, quantity: u64) -> Value {
    let limit = reported_inventory_stack_limits(snapshot, false)
        .get(&item_id)
        .copied();
    let slots = if quantity == 0 {
        Some(0)
    } else {
        limit.map(|limit| quantity.div_ceil(u64::from(limit.get())))
    };
    let general_free =
        carried_bag_space(snapshot).and_then(|space| space["known_general_free_slots"].as_u64());
    let headroom = limit.map(|limit| {
        snapshot
            .sections
            .get("bags")
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(|bags| bags.values())
            .filter_map(Value::as_object)
            .flat_map(|slots| slots.values())
            .filter(|item| item["item_id"].as_u64() == Some(u64::from(item_id)))
            .filter_map(|item| item["count"].as_u64())
            .map(|count| u64::from(limit.get()).saturating_sub(count))
            .sum::<u64>()
    });
    let slots_with_merges = if quantity == 0 {
        Some(0)
    } else {
        limit.zip(headroom).map(|(limit, headroom)| {
            quantity
                .saturating_sub(headroom)
                .div_ceil(u64::from(limit.get()))
        })
    };
    json!({"reported_carried_stack_limit":limit,"estimated_new_stack_slots":slots,
        "recorded_general_free_slots":general_free,
        "fits_recorded_general_slots":slots.zip(general_free).map(|(needed, free)| needed <= free),
        "hypothetical_carried_stack_headroom":headroom,
        "estimated_new_stack_slots_if_same_item_stacks_merge":slots_with_merges,
        "fits_recorded_general_slots_if_same_item_stacks_merge":slots_with_merges.zip(general_free)
            .map(|(needed, free)| needed <= free),
        "same_item_stack_merge_verified":false,
        "placement_verified":false,"live":false})
}

pub fn quest_currency_projection(quest: Option<&Value>, balances: &Value) -> Value {
    let Some(quest) = quest else {
        return Value::Null;
    };
    let headroom = currency_cap_headroom(balances);
    let project = |id: u64, quantity| {
        let key = id.to_string();
        currency_reward_projection(quantity, &balances["entries"][&key], &headroom[&key])
    };
    let receive = quest["currency_rewards"].as_object().map(|rows| {
        let mut totals = BTreeMap::<u64, u64>::new();
        for row in rows.values() {
            if let (Some(id), Some(quantity)) =
                (row["currency_id"].as_u64(), row["quantity"].as_u64())
            {
                *totals.entry(id).or_default() += quantity;
            }
        }
        totals
    });
    let fixed = receive.as_ref().map(|totals| {
        totals
            .iter()
            .map(|(&id, &quantity)| (id, project(id, quantity)))
            .collect::<BTreeMap<_, _>>()
    });
    let choices = quest["currency_choices"].as_object().map(|rows| {
        rows.iter().filter_map(|(index, row)| {
            let id = row["currency_id"].as_u64()?;
            let quantity = row["quantity"].as_u64()?;
            let combined = receive.as_ref().map(|totals| project(id, quantity + totals.get(&id).copied().unwrap_or(0)));
            Some((index, json!({"currency_id":id,"choice_quantity":quantity,"with_receive_rewards":combined})))
        }).collect::<BTreeMap<_, _>>()
    });
    json!({"receive_by_currency_id":fixed,"choices_by_position":choices})
}

pub fn quest_list_renown_projection(
    quests: &serde_json::Map<String, Value>,
    reputation: &Value,
) -> Value {
    let mut totals = BTreeMap::<String, u64>::new();
    let mut unknown = 0;
    for quest in quests.values() {
        let Some(rows) = quest["renown_rewards"].as_object() else {
            unknown += 1;
            continue;
        };
        for row in rows.values() {
            if let (Some(id), Some(points)) = (row["faction_id"].as_u64(), row["quantity"].as_u64())
            {
                *totals.entry(id.to_string()).or_default() += points;
            }
        }
    }
    let projections: BTreeMap<_, _> = totals
        .into_iter()
        .map(|(id, points)| {
            let mut projection = renown_reward_projection(points, &reputation["entries"][&id]);
            if unknown > 0 && projection["reaches_next_rank"] == false {
                projection["reaches_next_rank"] = Value::Null;
            }
            (id, projection)
        })
        .collect();
    json!({"known_rewards_by_faction":projections,"unknown_reward_list_count":unknown})
}

pub fn quest_ready_renown_projection(
    quests: &serde_json::Map<String, Value>,
    reputation: &Value,
    wallet: Option<&Value>,
    faction_id: u32,
    quantity: Option<u64>,
) -> Value {
    let faction = &reputation["entries"][faction_id.to_string()];
    let observed = renown_reward_projection(0, faction);
    let target = quantity.or_else(|| observed["points_to_next_rank"].as_u64());
    let Some(target) = target else {
        return json!({"faction_id":faction_id,"status":"next_rank_target_unknown",
            "capped":observed["capped"],"live":false});
    };
    let mut report = quest_ready_reward_projection(
        quests,
        QuestRewardTarget::Renown(faction_id, faction, wallet),
        target,
        false,
    );
    report["target_basis"] = json!(if quantity.is_some() {
        "requested_quantity"
    } else {
        "recorded_next_rank"
    });
    report
}

fn renown_reward_projection(quantity: u64, faction: &Value) -> Value {
    let detail = (faction["kind"] == "renown")
        .then(|| faction["renown"].as_object())
        .flatten();
    let capped = detail
        .and_then(|row| row.get("capped"))
        .and_then(Value::as_bool);
    let remaining = detail.and_then(remaining_renown_points);
    json!({"reward_points":quantity,"capped":capped,
        "points_to_next_rank":remaining,"reaches_next_rank":remaining.map(|points| quantity >= points),
        "points_short_of_next_rank":remaining.map(|points| points.saturating_sub(quantity))})
}

pub fn quest_renown_projection(quest: Option<&Value>, reputation: &Value) -> Value {
    let Some(rows) = quest.and_then(|q| q["renown_rewards"].as_object()) else {
        return Value::Null;
    };
    let mut totals = BTreeMap::<u64, u64>::new();
    for row in rows.values() {
        if let (Some(id), Some(quantity)) = (row["faction_id"].as_u64(), row["quantity"].as_u64()) {
            *totals.entry(id).or_default() += quantity;
        }
    }
    json!(
        totals
            .into_iter()
            .map(|(id, quantity)| {
                let key = id.to_string();
                let faction = &reputation["entries"][&key];
                let mut projection = renown_reward_projection(quantity, faction);
                projection["name"] = faction["name"].clone();
                (id, projection)
            })
            .collect::<BTreeMap<_, _>>()
    )
}

pub(crate) fn xp_to_next_level(xp: &Value) -> Option<u64> {
    xp["level_threshold_xp"]
        .as_u64()?
        .checked_sub(xp["current_xp"].as_u64()?)
}

pub fn quest_list_xp_projection(
    quests: &serde_json::Map<String, Value>,
    experience: Option<&Value>,
) -> Value {
    let mut unknown = 0;
    let mut known_xp = 0u128;
    for quest in quests.values() {
        match quest["reward_amounts"]["xp"].as_u64() {
            Some(xp) => known_xp += u128::from(xp),
            None => unknown += 1,
        }
    }
    let total = u64::try_from(known_xp).ok();
    let disabled = experience.and_then(|xp| xp["xp_disabled"].as_bool());
    let remaining = experience.and_then(xp_to_next_level);
    let estimate = remaining.zip(total).filter(|_| disabled != Some(true));
    let ready_turn_in_plan = remaining
        .filter(|_| disabled != Some(true))
        .map(|remaining| {
            let mut rewards: Vec<_> = quests
                .iter()
                .filter_map(|(id, quest)| {
                    (quest_readiness(quest) == QuestReadiness::Ready).then_some(())?;
                    Some((
                        std::cmp::Reverse(quest["reward_amounts"]["xp"].as_u64()?),
                        id.parse::<u32>().ok()?,
                    ))
                })
                .collect();
            rewards.sort_unstable();
            let mut shortfall = remaining;
            let mut selected = Vec::new();
            for (std::cmp::Reverse(xp), id) in rewards {
                if shortfall == 0 {
                    break;
                }
                shortfall = shortfall.saturating_sub(xp);
                selected.push(id);
            }
            json!({"quest_ids_to_threshold":(shortfall == 0).then_some(selected),
            "xp_shortfall_after_known_ready_rewards":shortfall})
        });
    json!({
        "known_reward_xp":total,"unknown_reward_count":unknown,
        "xp_to_next_level":remaining,"xp_disabled":disabled,
        "ready_turn_in_plan":ready_turn_in_plan,
        "covers_recorded_xp_threshold":estimate.and_then(|(remaining,total)| {
            if total >= remaining { Some(true) } else if unknown == 0 { Some(false) } else { None }
        }),
        "xp_shortfall_after_known_rewards":estimate.map(|(remaining,total)| remaining.saturating_sub(total))
    })
}

pub fn quest_ready_honor_projection(
    quests: &serde_json::Map<String, Value>,
    target: u64,
    wallet: Option<&Value>,
) -> Value {
    let mut unknown = 0;
    let mut rewards = Vec::new();
    for (id, quest) in quests {
        if quest_readiness(quest) != QuestReadiness::Ready {
            continue;
        }
        let Some(honor) = quest["reward_amounts"]["honor"].as_u64() else {
            unknown += 1;
            continue;
        };
        if honor > 0
            && let Ok(numeric_id) = id.parse::<u32>()
        {
            rewards.push((std::cmp::Reverse(honor), numeric_id, id, quest));
        }
    }
    rewards.sort_unstable_by_key(|(honor, id, _, _)| (*honor, *id));
    let known_total: u64 = rewards.iter().map(|(honor, _, _, _)| honor.0).sum();
    let shortfall = target.saturating_sub(known_total);
    let covers_target = if shortfall == 0 {
        Some(true)
    } else if unknown == 0 {
        Some(false)
    } else {
        None
    };
    let plan = (shortfall == 0).then(|| {
        let mut selected = Vec::new();
        let mut selected_ids = Vec::new();
        let mut total = 0;
        for (honor, numeric_id, id, quest) in rewards {
            total += honor.0;
            selected.push((id, quest));
            selected_ids.push(numeric_id);
            if total >= target { break; }
        }
        let balance = wallet.and_then(|wallet| wallet["copper"].as_u64());
        let wallet_order = quest_wallet_turn_in_order(selected.into_iter(), balance).map(|mut order| {
            let prefix_honor = order["captured_wallet_progress"]["affordable_prefix_count"]
                .as_u64().and_then(|count| {
                    order["quest_ids"].as_array()?.iter().take(usize::try_from(count).ok()?)
                        .try_fold(0u64, |total, id| {
                            total.checked_add(quests.get(id.as_str()?)?["reward_amounts"]["honor"].as_u64()?)
                        })
                });
            let ids = order["quest_ids"].as_array_mut().unwrap();
            let truncated = ids.len() > 20;
            ids.truncate(20);
            order["quest_ids_truncated"] = json!(truncated);
            order["honor_from_affordable_prefix"] = json!(prefix_honor);
            order["honor_shortfall_after_affordable_prefix"] =
                json!(prefix_honor.map(|honor| target.saturating_sub(honor)));
            order
        });
        json!({"quest_count":selected_ids.len(),"quest_ids":selected_ids.iter().take(20).collect::<Vec<_>>(),
            "truncated":selected_ids.len()>20,"recorded_honor_total":total,
            "selected_quests_wallet_projection":wallet_order})
    });
    json!({"target_honor":target,"known_ready_reward_honor":known_total,
        "ready_quests_with_unknown_honor":unknown,"covers_target":covers_target,
        "shortfall_after_known_ready_rewards":shortfall,"ready_turn_in_plan":plan,
        "rank_progress_verified":false,"live":false})
}

pub fn quest_xp_projection(quest: Option<&Value>, experience: Option<&Value>) -> Value {
    let Some(quest) = quest else {
        return Value::Null;
    };
    let reward = quest["reward_amounts"]["xp"].as_u64();
    let remaining = experience.and_then(xp_to_next_level);
    let disabled = experience.and_then(|xp| xp["xp_disabled"].as_bool());
    let estimate = remaining.zip(reward).filter(|_| disabled != Some(true));
    json!({
        "reward_xp":reward,"experience_status":experience.and_then(|xp| xp["status"].as_str()),
        "xp_disabled":disabled,"xp_to_next_level":remaining,
        "covers_recorded_xp_threshold":estimate.map(|(remaining,reward)| reward >= remaining),
        "xp_shortfall_after_reward":estimate.map(|(remaining,reward)| remaining.saturating_sub(reward))
    })
}

fn flight_destinations(map: &Value) -> Option<(u64, BTreeMap<u64, &Value>)> {
    let mut origin = None;
    let mut nodes = BTreeMap::new();
    for row in map["entries"].as_object()?.values() {
        if row["map_transition"].as_bool()? {
            continue;
        }
        let id = row["node_id"].as_u64()?;
        if nodes.insert(id, row).is_some() {
            return None;
        }
        if row["state"].as_str()? == "current" && origin.replace(id).is_some() {
            return None;
        }
    }
    Some((origin?, nodes))
}

pub(crate) fn captured_flight_route(map: &Value, destination_id: u32) -> Option<Value> {
    let (origin_id, nodes) = flight_destinations(map)?;
    let origin = nodes.get(&origin_id)?;
    let destination = nodes.get(&u64::from(destination_id))?;
    if destination["state"] != "reachable" {
        return None;
    }
    let mut route = json!({"map_id":map["map_id"],"origin_node_id":origin_id,"origin_name":origin["name"],
        "destination_node_id":destination_id,"destination_name":destination["name"],
        "fare_copper":destination["fare_copper"]});
    if let Some(position) = origin.get("map_position") {
        route["origin_map_position"] = position.clone();
    }
    if let Some(position) = destination.get("map_position") {
        route["destination_map_position"] = position.clone();
    }
    if let Some(path) = destination.get("observed_route_node_ids") {
        route["observed_route_node_ids"] = path.clone();
    }
    Some(route)
}

pub(crate) fn named_flight_destination(map: &Value, name: &str) -> Option<u32> {
    let (_, nodes) = flight_destinations(map)?;
    let mut matches = nodes.into_iter().filter(|(_, row)| {
        row["state"] == "reachable"
            && row["name"]
                .as_str()
                .is_some_and(|label| label.trim().to_lowercase() == name)
    });
    let (id, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    u32::try_from(id).ok()
}

pub fn review_flight_itinerary(
    legs: &[(&str, Snapshot, u32)],
    reserve: Option<u64>,
    include_repair_quote: bool,
) -> Result<Value> {
    let (_, first, _) = legs
        .first()
        .ok_or_else(|| anyhow::anyhow!("Missing first leg"))?;
    ensure!(
        first.client.family == crate::Family::Forever && (2..=8).contains(&legs.len()),
        "Invalid flight itinerary"
    );
    let mut routes = Vec::new();
    let mut previous_destination = None;
    let mut total = Some(0u64);
    for (snapshot_id, snapshot, destination) in legs {
        ensure_same_character_and_client(first, snapshot)?;
        let mut route = snapshot
            .sections
            .get("flight_map")
            .and_then(|map| captured_flight_route(map, *destination))
            .ok_or_else(|| anyhow::anyhow!("Route unavailable"))?;
        ensure!(
            previous_destination.is_none_or(|id| route["origin_node_id"].as_u64() == Some(id)),
            "Flight legs are disconnected"
        );
        previous_destination = Some(u64::from(*destination));
        total = total
            .zip(route["fare_copper"].as_u64())
            .and_then(|(sum, fare)| sum.checked_add(fare));
        route["snapshot_id"] = json!(snapshot_id);
        route["captured_at"] = json!(snapshot.captured_at);
        routes.push(route);
    }
    let starting_wallet = first
        .sections
        .get("wallet")
        .and_then(|wallet| wallet["copper"].as_u64());
    let budget = total.zip(starting_wallet);
    let count_affordable_legs = |balance: u64| {
        routes
            .iter()
            .scan(balance, |remaining, route| {
                *remaining = remaining.checked_sub(route["fare_copper"].as_u64()?)?;
                Some(())
            })
            .count()
    };
    let affordable_prefix = starting_wallet.map(count_affordable_legs);
    let first_uncovered_leg = affordable_prefix.and_then(|index| {
        let route = routes.get(index)?;
        let reason = if route["fare_copper"].is_null() {
            "fare_unknown"
        } else {
            "insufficient_copper"
        };
        Some(json!({"leg_number":index + 1,"reason":reason}))
    });
    let mut itinerary = json!({"legs":routes,"total_fare_copper":total,
        "affordable_from_starting_wallet":budget.map(|(fare, balance)| fare <= balance),
        "starting_wallet_shortfall_copper":budget.map(|(fare, balance)| fare.saturating_sub(balance)),
        "starting_wallet_affordable_prefix_legs":affordable_prefix,
        "starting_wallet_first_uncovered_leg":first_uncovered_leg});
    let project_budget = |extra_cost: Option<u64>, affordability_field: &str| {
        let budget = total
            .zip(extra_cost)
            .and_then(|(fare, extra)| fare.checked_add(extra))
            .zip(starting_wallet);
        json!({"reserve_copper":reserve.unwrap_or_default(),
        (affordability_field):budget.map(|(cost, wallet)| cost <= wallet),
        "shortfall_copper":budget.map(|(cost, wallet)| cost.saturating_sub(wallet)),
        "affordable_prefix_legs":starting_wallet.zip(extra_cost).map(|(balance, extra)| {
            balance.checked_sub(extra).map_or(0, count_affordable_legs)
        })})
    };
    if reserve.is_some() {
        itinerary["wallet_reserve_projection"] = project_budget(reserve, "affordable_with_reserve");
    }
    if include_repair_quote {
        let repair = first
            .sections
            .get("repair_quote")
            .and_then(|row| row["copper"].as_u64());
        let extra_cost = repair.and_then(|quote| quote.checked_add(reserve.unwrap_or_default()));
        let mut projection = project_budget(extra_cost, "affordable_with_repairs_and_reserve");
        projection["repair_quote_copper"] = json!(repair);
        itinerary["repair_budget_projection"] = projection;
    }
    Ok(itinerary)
}

#[derive(Clone, Copy, Default, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FlightRouteObjective {
    #[default]
    LowestFare,
    FewestLegs,
}

pub fn plan_saved_flights(
    captures: &[(&str, Snapshot)],
    destination: Option<u32>,
    destination_name: Option<&str>,
    reserve: Option<u64>,
    max_legs: u8,
    objective: FlightRouteObjective,
    include_repair_quote: bool,
    require_affordable: bool,
    maximum_total_fare_copper: Option<u64>,
    excluded_landing_node_ids: &[std::num::NonZeroU32],
    excluded_flyover_node_ids: &[std::num::NonZeroU32],
) -> Result<Value> {
    ensure!((1..=8).contains(&max_legs), "Invalid flight leg limit");
    let exclusions = |ids: &[std::num::NonZeroU32]| -> Result<BTreeSet<u64>> {
        ensure!(ids.len() <= 32, "Too many excluded flight nodes");
        let set: BTreeSet<_> = ids.iter().map(|id| u64::from(id.get())).collect();
        ensure!(set.len() == ids.len(), "Duplicate excluded flight node");
        Ok(set)
    };
    let excluded = exclusions(excluded_landing_node_ids)?;
    let excluded_flyovers = exclusions(excluded_flyover_node_ids)?;
    let destination_name = destination_name.map(str::trim);
    ensure!(
        (destination.is_some() || destination_name.is_some())
            && destination_name.is_none_or(|name| !name.is_empty() && name.len() <= 256),
        "Expected a destination ID or valid destination name"
    );
    let destination_name = destination_name.map(str::to_lowercase);
    let (_, first) = captures
        .first()
        .ok_or_else(|| anyhow::anyhow!("Missing departure"))?;
    ensure!(
        first.client.family == crate::Family::Forever && captures.len() <= 8,
        "Expected up to eight Forever captures"
    );
    let mut departures = BTreeMap::<u64, Vec<_>>::new();
    let mut named_destinations = BTreeSet::new();
    let mut start = None;
    for (index, (_, capture)) in captures.iter().enumerate() {
        ensure_same_character_and_client(first, capture)?;
        let (origin, nodes) = capture
            .sections
            .get("flight_map")
            .and_then(flight_destinations)
            .ok_or_else(|| anyhow::anyhow!("Missing or ambiguous flight panel"))?;
        if index == 0 {
            start = Some(origin);
        }
        let edges = departures.entry(origin).or_default();
        edges.extend(nodes.into_iter().filter_map(|(node, row)| {
            (row["state"] == "reachable").then_some(())?;
            if destination_name.as_ref().is_some_and(|name| {
                destination.is_none_or(|id| u64::from(id) == node)
                    && row["name"]
                        .as_str()
                        .is_some_and(|label| label.trim().to_lowercase() == *name)
            }) {
                named_destinations.insert(node);
            }
            if excluded.contains(&node) {
                return None;
            }
            if !excluded_flyovers.is_empty() {
                let preview = row["observed_route_node_ids"].as_object()?;
                if preview
                    .values()
                    .filter_map(Value::as_u64)
                    .any(|id| id != origin && id != node && excluded_flyovers.contains(&id))
                {
                    return None;
                }
            }
            Some((index, node, row["fare_copper"].as_u64()))
        }));
    }
    let start = start.ok_or_else(|| anyhow::anyhow!("Missing departure"))?;
    let destination = if destination_name.is_some() {
        ensure!(
            named_destinations.len() == 1,
            "Destination name is absent or ambiguous"
        );
        *named_destinations.first().unwrap()
    } else {
        u64::from(destination.unwrap())
    };
    ensure!(
        start != destination,
        "Already at the requested departure node"
    );
    ensure!(
        !excluded.contains(&start) && !excluded.contains(&destination),
        "Cannot exclude a route endpoint"
    );
    let wallet = first
        .sections
        .get("wallet")
        .and_then(|row| row["copper"].as_u64());
    let repair = first
        .sections
        .get("repair_quote")
        .and_then(|row| row["copper"].as_u64());
    let status = |status: &str| {
        let mut report = json!({
            "status":status,"origin_node_id":start,"destination_node_id":destination,
        });
        if !excluded.is_empty() {
            report["excluded_landing_node_ids"] = json!(excluded);
        }
        if !excluded_flyovers.is_empty() {
            report["excluded_flyover_node_ids"] = json!(excluded_flyovers);
        }
        report
    };
    let render_legs = |path: Vec<(usize, u64)>| {
        path.into_iter()
            .map(|(index, target)| {
                let (id, capture) = &captures[index];
                let mut route = capture
                    .sections
                    .get("flight_map")
                    .and_then(|map| captured_flight_route(map, target as u32))
                    .ok_or_else(|| anyhow::anyhow!("Missing selected leg"))?;
                route["snapshot_id"] = json!(id);
                route["captured_at"] = json!(capture.captured_at);
                Ok(route)
            })
            .collect::<Result<Vec<_>>>()
    };
    let fare_budget = if require_affordable {
        let required_repair = if include_repair_quote {
            repair
        } else {
            Some(0)
        };
        match (wallet, required_repair) {
            (None, _) => Err(("starting_budget_unknown", "wallet_unobserved")),
            (_, None) => Err(("starting_budget_unknown", "repair_quote_unobserved")),
            (Some(balance), Some(repair_cost)) => balance
                .checked_sub(repair_cost)
                .and_then(|remaining| remaining.checked_sub(reserve.unwrap_or_default()))
                .map(|spendable| {
                    Some(
                        maximum_total_fare_copper
                            .map_or(spendable, |maximum| maximum.min(spendable)),
                    )
                })
                .ok_or(("no_affordable_route", "upfront_cost_exceeds_wallet")),
        }
    } else {
        Ok(maximum_total_fare_copper)
    };
    let max_fare = fare_budget.ok().flatten();
    let search_legs = if fare_budget.is_ok() { max_legs } else { 0 };
    let mut frontier = BTreeMap::from([(start, (0u64, Vec::new()))]);
    let mut best = None;
    for depth in 0..search_legs {
        let mut next = BTreeMap::new();
        for (node, (cost, path)) in frontier {
            for &(index, target, fare) in departures.get(&node).into_iter().flatten() {
                let Some(fare) = fare else { continue };
                let total = cost + fare;
                if target == start
                    || (depth == 0 && index != 0)
                    || max_fare.is_some_and(|maximum| total > maximum)
                {
                    continue;
                }
                let mut path = path.clone();
                path.push((index, target));
                let candidate = (total, path);
                if target == destination {
                    let candidate = (candidate.0, depth + 1, candidate.1);
                    if best.as_ref().is_none_or(|previous| &candidate < previous) {
                        best = Some(candidate);
                    }
                } else {
                    let current = next.entry(target).or_insert_with(|| candidate.clone());
                    if candidate < *current {
                        *current = candidate;
                    }
                }
            }
        }
        frontier = next;
        if frontier.is_empty()
            || (matches!(objective, FlightRouteObjective::FewestLegs) && best.is_some())
        {
            break;
        }
    }
    let Some((fare, _, path)) = best else {
        let mut report = status(if let Err((status, _)) = fare_budget {
            status
        } else if require_affordable {
            "no_affordable_route"
        } else if maximum_total_fare_copper.is_some() {
            "no_route_within_fare_ceiling"
        } else {
            "no_fully_priced_route"
        });
        if let Err((_, issue)) = fare_budget {
            report["starting_budget_issue"] = json!(issue);
        }
        // Prefer quoted paths, then cheaper fares. For unquoted paths, prefer fewer legs.
        let mut witness = None;
        let mut frontier = BTreeMap::from([(start, (false, 0u64, Vec::new()))]);
        for depth in 0..max_legs {
            let mut next = BTreeMap::new();
            for (node, (unquoted, cost, path)) in frontier {
                for &(index, target, fare) in departures.get(&node).into_iter().flatten() {
                    if target == start || (depth == 0 && index != 0) {
                        continue;
                    }
                    let unquoted = unquoted || fare.is_none();
                    let cost = if unquoted {
                        0
                    } else {
                        cost + fare.unwrap_or_default()
                    };
                    let mut path = path.clone();
                    path.push((index, target));
                    if target == destination {
                        let candidate = (unquoted, cost, depth + 1, path);
                        if witness
                            .as_ref()
                            .is_none_or(|previous| &candidate < previous)
                        {
                            witness = Some(candidate);
                        }
                    } else {
                        let candidate = (unquoted, cost, path);
                        let previous = next.entry(target).or_insert_with(|| candidate.clone());
                        if candidate < *previous {
                            *previous = candidate;
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        let minimum_fare = witness
            .as_ref()
            .and_then(|(unquoted, cost, _, _)| (!unquoted).then_some(*cost));
        report["saved_path_evidence"] = json!({
            "reachable_path_observed":witness.is_some(),
            "fully_priced_path_observed_without_budget":minimum_fare.is_some(),
            "maximum_legs":max_legs,
            "minimum_fully_priced_fare_copper":minimum_fare,
            "effective_fare_ceiling_copper":max_fare,
            "shortfall_vs_effective_ceiling_copper":minimum_fare.zip(max_fare)
                .map(|(fare,ceiling)| fare.saturating_sub(ceiling))
        });
        if let Some((_, _, _, path)) = witness {
            report["saved_path_witness"] =
                json!({"legs":render_legs(path)?,"total_fare_copper":minimum_fare});
        }
        return Ok(report);
    };
    let legs = render_legs(path)?;
    let mut plan = status("found");
    plan["legs"] = json!(legs);
    plan["total_fare_copper"] = json!(fare);
    plan["affordable_from_starting_wallet"] = json!(wallet.map(|balance| fare <= balance));
    if let Some(reserve) = reserve {
        plan["affordable_with_reserve"] = json!(wallet.map(|balance| {
            balance
                .checked_sub(reserve)
                .is_some_and(|spendable| fare <= spendable)
        }));
    }
    if include_repair_quote {
        plan["starting_repair_quote_copper"] = json!(repair);
        plan["affordable_with_repairs_and_reserve"] =
            json!(wallet.zip(repair).map(|(balance, repair)| {
                balance
                    .checked_sub(repair)
                    .and_then(|remaining| remaining.checked_sub(reserve.unwrap_or_default()))
                    .is_some_and(|spendable| fare <= spendable)
            }));
    }
    Ok(plan)
}

fn captured_flight_fare_summary(map: &Value) -> Option<Value> {
    let (origin_id, nodes) = flight_destinations(map)?;
    let mut unknown_fares = 0usize;
    let mut known = Vec::new();
    for (id, node) in nodes
        .iter()
        .filter(|(_, node)| node["state"] == "reachable")
    {
        if let Some(fare) = node["fare_copper"].as_u64() {
            known.push((fare, id, &node["name"]));
        } else {
            unknown_fares += 1;
        }
    }
    known.sort_unstable_by_key(|&(fare, id, _)| (fare, id));
    let cheapest: Vec<_> = known
        .iter()
        .take(20)
        .map(|(fare, id, name)| json!({"node_id":id,"name":name,"fare_copper":fare}))
        .collect();
    Some(json!({"map_id":map["map_id"],"origin_node_id":origin_id,
        "known_fare_count":known.len(),"unknown_fare_count":unknown_fares,
        "cheapest_destinations":cheapest}))
}

fn compare_flight_destination(
    before: Option<&Value>,
    after: Option<&Value>,
    before_wallet: Option<u64>,
    after_wallet: Option<u64>,
    destination: u32,
    reserve: Option<u64>,
) -> Value {
    let observation = |map: Option<&Value>, wallet| {
        let Some(map) = map else {
            return json!({"status":"not_captured"});
        };
        let Some((origin, nodes)) = flight_destinations(map) else {
            return json!({"status":"origin_or_destinations_ambiguous","map_id":map["map_id"]});
        };
        let Some(row) = nodes
            .get(&u64::from(destination))
            .filter(|_| origin != u64::from(destination))
        else {
            return json!({"status":"not_observed","map_id":map["map_id"],"origin_node_id":origin});
        };
        let fare = row["fare_copper"].as_u64();
        let quote = fare.zip(wallet).filter(|_| row["state"] == "reachable");
        let affordable = quote.map(|(fare, balance)| fare <= balance);
        let mut observed = json!({"status":"observed","map_id":map["map_id"],"origin_node_id":origin,
            "state":row["state"],"fare_copper":fare,"affordable":affordable,
            "wallet_shortfall_copper":quote.map(|(fare, balance)| fare.saturating_sub(balance)),
            "remaining_wallet_after_fare_copper":quote.and_then(|(fare, balance)| balance.checked_sub(fare))});
        if let Some(path) = row.get("observed_route_node_ids") {
            observed["observed_route_node_ids"] = path.clone();
        }
        if let Some(reserve) = reserve {
            let budget =
                quote.and_then(|(fare, wallet)| Some((fare.checked_add(reserve)?, wallet)));
            observed["wallet_reserve_projection"] = json!({
                "affordable":budget.map(|(cost, wallet)| cost <= wallet),
                "shortfall_copper":budget.map(|(cost, wallet)| cost.saturating_sub(wallet)),
                "spendable_copper":budget.and_then(|(cost, wallet)| wallet.checked_sub(cost))
            });
        }
        observed
    };
    let a = observation(before, before_wallet);
    let b = observation(after, after_wallet);
    let same_map_and_origin = a["map_id"]
        .as_u64()
        .zip(a["origin_node_id"].as_u64())
        .zip(b["map_id"].as_u64().zip(b["origin_node_id"].as_u64()))
        .filter(|_| a["status"] == "observed" && b["status"] == "observed")
        .map(|(a, b)| a == b);
    let fare_delta = a["fare_copper"]
        .as_i64()
        .zip(b["fare_copper"].as_i64())
        .filter(|_| same_map_and_origin == Some(true))
        .map(|(a, b)| b - a);
    let affordability_changed = a["affordable"]
        .as_bool()
        .zip(b["affordable"].as_bool())
        .filter(|_| same_map_and_origin == Some(true))
        .map(|(a, b)| a != b);
    let route_before = a.get("observed_route_node_ids");
    let route_after = b.get("observed_route_node_ids");
    let route_changed = route_before
        .zip(route_after)
        .filter(|_| same_map_and_origin == Some(true))
        .map(|(a, b)| a != b);
    let has_route_evidence = route_before.is_some() || route_after.is_some();
    let wallet_delta = |before: &Value, after: &Value, field: &str| {
        before[field]
            .as_u64()
            .zip(after[field].as_u64())
            .filter(|_| same_map_and_origin == Some(true))
            .map(|(before, after)| i128::from(after) - i128::from(before))
    };
    let mut comparison = json!({"destination_node_id":destination,"before":a,"after":b,
        "same_map_and_origin":same_map_and_origin,"fare_copper_delta":fare_delta,
        "affordability_changed":affordability_changed,
        "wallet_shortfall_copper_delta":wallet_delta(&a, &b, "wallet_shortfall_copper"),
        "remaining_wallet_after_fare_copper_delta":wallet_delta(&a, &b, "remaining_wallet_after_fare_copper"),
        "live":false});
    if has_route_evidence {
        comparison["observed_route_changed"] = json!(route_changed);
    }
    if let Some(reserve) = reserve {
        let before = &a["wallet_reserve_projection"];
        let after = &b["wallet_reserve_projection"];
        comparison["required_wallet_reserve_copper"] = json!(reserve);
        comparison["wallet_reserve_changes"] = json!({
            "affordability_changed":before["affordable"].as_bool()
                .zip(after["affordable"].as_bool())
                .filter(|_| same_map_and_origin == Some(true))
                .map(|(before, after)| before != after),
            "shortfall_copper_delta":wallet_delta(before, after, "shortfall_copper"),
            "spendable_copper_delta":wallet_delta(before, after, "spendable_copper")
        });
    }
    comparison
}

fn compare_flight_maps(before: &Value, after: &Value, wallets: Option<(u64, u64)>) -> Value {
    if before["map_id"] != after["map_id"] {
        return json!({"status":"different_maps"});
    }
    let Some(((from, previous), (to, current))) =
        flight_destinations(before).zip(flight_destinations(after))
    else {
        return json!({"status":"origin_or_destinations_ambiguous"});
    };
    if from != to {
        return json!({"status":"different_origins"});
    }
    let changes: Vec<_> = previous
        .iter()
        .filter_map(|(&id, old)| {
            let new = current.get(&id)?;
            let state_changed = old["state"] != new["state"];
            let fare_delta = old["fare_copper"]
                .as_i64()
                .zip(new["fare_copper"].as_i64())
                .map(|(old, new)| new - old)
                .filter(|delta| *delta != 0);
            (state_changed || fare_delta.is_some()).then(|| {
                json!({
                    "node_id":id,"name":new["name"],
                    "before_state":old["state"],"after_state":new["state"],
                    "before_fare_copper":old["fare_copper"],"after_fare_copper":new["fare_copper"],
                    "fare_copper_delta":fare_delta,
                })
            })
        })
        .collect();
    let fare_observation_changes: Vec<_> =
        previous
            .iter()
            .filter_map(|(&id, old)| {
                let new = current.get(&id)?;
                let before_fare = old["fare_copper"].as_u64();
                let after_fare = new["fare_copper"].as_u64();
                (before_fare.is_some() != after_fare.is_some()).then(|| json!({
            "node_id":id,"name":new["name"],"before_fare_copper":before_fare,"after_fare_copper":after_fare,
            "status":if after_fare.is_some() {"became_observed"} else {"became_unknown"}
        }))
            })
            .collect();
    let observed_only_before: Vec<_> = previous
        .keys()
        .filter(|id| !current.contains_key(id))
        .collect();
    let observed_only_after: Vec<_> = current
        .keys()
        .filter(|id| !previous.contains_key(id))
        .collect();
    let affordability_changes = wallets.map(|(before_wallet, after_wallet)| {
        previous.iter().filter_map(|(&id, old)| {
            let new = current.get(&id)?;
            if old["state"] != "reachable" || new["state"] != "reachable" {
                return None;
            }
            let before_affordable = old["fare_copper"].as_u64()? <= before_wallet;
            let after_affordable = new["fare_copper"].as_u64()? <= after_wallet;
            (before_affordable != after_affordable).then(|| json!({
                "node_id":id,"before_affordable":before_affordable,"after_affordable":after_affordable,
            }))
        }).collect::<Vec<_>>()
    });
    json!({"status":"compared","map_id":before["map_id"],"origin_node_id":from,"changes":changes,
        "fare_observation_changes":fare_observation_changes,
        "affordability_changes":affordability_changes,
        "node_ids_observed_only_before":observed_only_before,"node_ids_observed_only_after":observed_only_after})
}

pub fn trainer_requirements_review(
    trainer: &Value,
    captured_trainer: Option<&Value>,
    level: Option<u64>,
    skills: Option<&Value>,
    spellbook: Option<&Value>,
    active_pet: Option<&Value>,
) -> Value {
    let Some(entries) = trainer["entries"].as_object() else {
        return Value::Null;
    };
    let pet_level = trainer["pet_level"].as_u64();
    let active_pet_level = active_pet.and_then(|pet| pet["level"].as_u64());
    let requirement_level = if trainer["kind"] == "pet" {
        pet_level
    } else {
        level
    };
    let mut ranks_by_name = BTreeMap::new();
    let mut available_slots_by_name = BTreeMap::<&str, BTreeSet<u32>>::new();
    let candidate_entries = captured_trainer
        .and_then(|section| section["entries"].as_object())
        .unwrap_or(entries);
    for (slot, lesson) in candidate_entries
        .iter()
        .filter(|(_, lesson)| lesson["status"] == "available")
    {
        if let Some((name, slot)) = lesson["name"].as_str().zip(slot.parse::<u32>().ok()) {
            available_slots_by_name
                .entry(name)
                .or_default()
                .insert(slot);
        }
    }
    let spell_names = spellbook
        .filter(|_| trainer["kind"] != "pet")
        .and_then(|book| book["entries"].as_object())
        .map(|rows| {
            rows.values()
                .filter_map(|row| row["name"].as_str())
                .collect::<BTreeSet<_>>()
        });
    if trainer["kind"] != "pet" {
        for (id, row) in skills.and_then(Value::as_object).into_iter().flatten() {
            if let Some((name, rank)) = row["name"].as_str().zip(row["rank"].as_u64()) {
                ranks_by_name
                    .entry(name)
                    .and_modify(|rank| *rank = None)
                    .or_insert(Some((id, rank)));
            }
        }
    }
    let lessons: BTreeMap<_, _> = entries
        .iter()
        .filter(|(_, lesson)| lesson["status"] == "unavailable")
        .map(|(slot, lesson)| {
            let level_shortfall = lesson["required_level"]
                .as_u64()
                .zip(requirement_level)
                .map(|(required, observed)| required.saturating_sub(observed));
            let unmet_skill = lesson["skill_requirement"]
                .as_object()
                .map(|skill| skill["met"] == false);
            let skill = &lesson["skill_requirement"];
            let skill_comparison = skill["name"]
                .as_str()
                .zip(skill["required_rank"].as_u64())
                .and_then(|(name, required)| {
                    let (id, rank) = ranks_by_name.get(name)?.as_ref()?;
                    Some(
                        json!({"skill_line_id":id.parse::<u32>().ok()?,"captured_rank":rank,
                        "required_rank":required,"rank_shortfall":required.saturating_sub(*rank)}),
                    )
                });
            let unmet_abilities = lesson["ability_requirements"].as_object().map(|abilities| {
                abilities
                    .values()
                    .filter(|ability| ability["met"] == false)
                    .map(|ability| &ability["name"])
                    .collect::<Vec<_>>()
            });
            let observed_unmet_abilities =
                unmet_abilities
                    .as_ref()
                    .zip(spell_names.as_ref())
                    .map(|(abilities, names)| {
                        abilities
                            .iter()
                            .filter_map(|name| name.as_str().filter(|name| names.contains(name)))
                            .collect::<BTreeSet<_>>()
                    });
            let candidate_lessons = unmet_abilities.as_ref().map(|abilities| {
                abilities
                    .iter()
                    .filter_map(|name| {
                        let name = name.as_str()?;
                        Some((name, available_slots_by_name.get(name)?))
                    })
                    .collect::<BTreeMap<_, _>>()
            });
            let mut requirements = json!({
                "level_shortfall":level_shortfall,
                "skill_requirement_unmet":unmet_skill,
                "captured_skill_rank_comparison":skill_comparison,
                "unmet_ability_names":unmet_abilities,
                "unmet_ability_names_observed_in_spellbook":observed_unmet_abilities,
                "available_lesson_slots_by_unmet_ability_name":candidate_lessons
            });
            if trainer["kind"] == "pet" {
                requirements["level_shortfall_vs_active_pet_observation"] = json!(
                    lesson["required_level"]
                        .as_u64()
                        .zip(active_pet_level)
                        .map(|(required, observed)| required.saturating_sub(observed))
                );
            }
            (slot, requirements)
        })
        .collect();
    let mut review =
        json!({"captured_character_level":level,"unavailable_lessons_by_slot":lessons});
    if trainer["kind"] == "pet" {
        review["captured_pet_level"] = json!(pet_level);
        review["active_pet_capture_context"] = pet_trainer_capture_context(trainer, active_pet);
    }
    review["profession_slot_checks"] = if trainer["kind"] == "pet" {
        Value::Null
    } else {
        let full = trainer["primary_profession_slots_full"].as_bool();
        let checks:BTreeMap<_,_>=entries.iter().filter(|(_,lesson)|lesson["status"]!="used")
            .map(|(slot,lesson)| {
                let is_profession=lesson["is_profession"].as_bool();
                let blocked=match is_profession {Some(true)=>full,Some(false)=>Some(false),None=>None};
                (slot,json!({"status":lesson["status"],"is_profession":is_profession,"blocked_by_primary_profession_slots":blocked}))
            }).collect();
        json!({"primary_profession_slots_full":full,"lessons_by_slot":checks,"eligibility_verified":false})
    };
    review
}

fn available_trainer_prices(trainer: &Value, price_field: &str) -> Option<(usize, usize, u64)> {
    let entries = trainer["entries"].as_object()?;
    let mut available = 0;
    let mut unknown_prices = 0;
    let mut subtotal = 0u64;
    for lesson in entries
        .values()
        .filter(|lesson| lesson["status"] == "available")
    {
        available += 1;
        if let Some(price) = lesson[price_field].as_u64() {
            subtotal += price;
        } else {
            unknown_prices += 1;
        }
    }
    Some((available, unknown_prices, subtotal))
}

pub fn reserved_balance_shortfall(cost: u64, balance: u64, reserve: u64) -> Option<u64> {
    match balance.checked_sub(reserve) {
        Some(spendable) => Some(cost.saturating_sub(spendable)),
        None => cost.checked_add(reserve - balance),
    }
}

pub fn reputation_turn_in_wallet_plan(
    turn_ins: Option<u64>,
    per_turn_in: std::num::NonZeroU64,
    balance: Option<u64>,
    reserve: u64,
) -> Value {
    let cost = per_turn_in.get();
    let required = turn_ins.and_then(|count| count.checked_mul(cost));
    let capacity = balance.map(|balance| balance.saturating_sub(reserve) / cost);
    json!({
        "assumed_copper_per_turn_in":cost,
        "required_wallet_reserve_copper":reserve,
        "captured_wallet_copper":balance,
        "required_copper":required,
        "required_copper_overflow":turn_ins.is_some() && required.is_none(),
        "wallet_shortfall_copper":required.zip(balance)
            .and_then(|(required, balance)| reserved_balance_shortfall(required, balance, reserve)),
        "supported_turn_ins":capacity,
        "turn_ins_shortfall_to_next_rank":turn_ins.zip(capacity)
            .map(|(needed, capacity)| needed.saturating_sub(capacity))
    })
}

pub fn trainer_lesson_affordability(
    trainer: &Value,
    price_field: &str,
    balance: Option<u64>,
    reserve: u64,
) -> Value {
    json!(
        trainer["entries"]
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(_, lesson)| lesson["status"] == "available")
            .map(|(id, lesson)| {
                let price = lesson[price_field].as_u64();
                let affordable = balance.and_then(|balance| match balance.checked_sub(reserve) {
                    Some(spendable) => price.map(|price| price <= spendable),
                    None => Some(false),
                });
                let shortfall = price.zip(balance).and_then(|(price, balance)| {
                    reserved_balance_shortfall(price, balance, reserve)
                });
                (
                    id,
                    json!({
                        "affordable": affordable,
                        "shortfall": shortfall
                    }),
                )
            })
            .collect::<BTreeMap<_, _>>()
    )
}

pub fn trainer_cheapest_affordable(
    trainer: &Value,
    price_field: &str,
    balance: Option<u64>,
) -> Option<Value> {
    let balance = balance?;
    let priced: Vec<_> = trainer["entries"]
        .as_object()?
        .iter()
        .filter(|(_, lesson)| lesson["status"] == "available")
        .filter_map(|(slot, lesson)| Some((lesson[price_field].as_u64()?, slot)))
        .collect();
    let (slots, subtotal) = cheapest_affordable_rows(priced, balance);
    Some(json!({"lesson_slots":slots,"lesson_count":slots.len(),"total_price":subtotal}))
}

fn cheapest_affordable_rows<T: Ord>(mut priced: Vec<(u64, T)>, balance: u64) -> (Vec<T>, u64) {
    priced.sort_unstable();
    let mut subtotal = 0;
    let mut slots = Vec::new();
    for (price, slot) in priced {
        if price > balance - subtotal {
            break;
        }
        subtotal += price;
        slots.push(slot);
    }
    (slots, subtotal)
}

pub fn trainer_order_budget(
    trainer: &Value,
    slots: &[std::num::NonZeroU32],
    price_field: &str,
    balance: Option<u64>,
) -> Value {
    let Some(mut remaining) = balance else {
        return Value::Null;
    };
    for (index, slot) in slots.iter().enumerate() {
        let row = &trainer["entries"][slot.to_string()];
        let (reason, shortfall) = match (row["status"].as_str(), row[price_field].as_u64()) {
            (None, _) => ("lesson_not_retained", None),
            (Some("available"), None) => ("price_unknown", None),
            (Some("available"), Some(price)) if price <= remaining => {
                remaining -= price;
                continue;
            }
            (Some("available"), Some(price)) => ("insufficient_balance", Some(price - remaining)),
            _ => ("lesson_not_available", None),
        };
        let mut uncovered = json!({"slot":slot,"reason":reason});
        if let Some(shortfall) = shortfall {
            uncovered["additional_balance_needed"] = json!(shortfall);
        }
        return json!({"covered_prefix_lessons":index,
            "first_uncovered_lesson":uncovered});
    }
    json!({"covered_prefix_lessons":slots.len(),"first_uncovered_lesson":null})
}

pub fn trainer_wallet_projection(
    trainer: &Value,
    wallet: Option<&Value>,
    slots: Option<&[std::num::NonZeroU32]>,
) -> Value {
    if trainer["kind"] == "pet" {
        return Value::Null;
    }
    let Some((available, unknown_prices, subtotal)) =
        available_trainer_prices(trainer, "price_copper")
    else {
        return Value::Null;
    };
    let total = (unknown_prices == 0).then_some(subtotal);
    let balance = wallet.and_then(|wallet| wallet["copper"].as_u64());
    let mut projection = json!({
        "available_lessons":available,
        "lessons_with_unknown_price":unknown_prices,
        "known_price_subtotal_copper":subtotal,
        "total_price_copper":total,
        "wallet_shortfall_copper":total.zip(balance).map(|(cost, balance)| cost.saturating_sub(balance)),
        "individual_lesson_affordability":trainer_lesson_affordability(trainer, "price_copper", balance, 0),
        "cheapest_affordable_known_lessons":trainer_cheapest_affordable(trainer, "price_copper", balance),
    });
    if let Some(slots) = slots {
        projection["ordered_lesson_budget"] =
            trainer_order_budget(trainer, slots, "price_copper", balance);
    }
    projection
}

pub fn trainer_points_projection(trainer: &Value, slots: Option<&[std::num::NonZeroU32]>) -> Value {
    if trainer["kind"] != "pet" {
        return Value::Null;
    }
    let Some((available, unknown_prices, subtotal)) =
        available_trainer_prices(trainer, "price_training_points")
    else {
        return Value::Null;
    };
    let total = (unknown_prices == 0).then_some(subtotal);
    let balance = trainer["pet_training_points_available"].as_u64();
    let mut projection = json!({
        "available_lessons":available,
        "lessons_with_unknown_price":unknown_prices,
        "known_price_subtotal_points":subtotal,
        "total_price_points":total,
        "training_point_shortfall":total.zip(balance).map(|(cost, balance)| cost.saturating_sub(balance)),
        "individual_lesson_affordability":trainer_lesson_affordability(trainer, "price_training_points", balance, 0),
        "cheapest_affordable_known_lessons":trainer_cheapest_affordable(trainer, "price_training_points", balance),
    });
    if let Some(slots) = slots {
        projection["ordered_lesson_budget"] =
            trainer_order_budget(trainer, slots, "price_training_points", balance);
    }
    projection
}

pub fn pet_trainer_capture_context(trainer: &Value, pet: Option<&Value>) -> Value {
    let trainer_level = trainer["pet_level"].as_u64();
    let trainer_points = trainer["pet_training_points_available"].as_u64();
    let pet_level = pet.and_then(|row| row["level"].as_u64());
    let pet_balance = pet.and_then(pet_training_points_balance);
    let comparable_balance = pet.and_then(|row| {
        let points = &row["training_points"];
        Some(
            points["total"]
                .as_u64()?
                .saturating_sub(points["used"].as_u64()?),
        )
    });
    json!({"trainer_pet_level":trainer_level,"active_pet_level":pet_level,
        "levels_agree":trainer_level.zip(pet_level).map(|(a,b)| a == b),
        "trainer_training_points_available":trainer_points,"active_pet_signed_training_point_balance":pet_balance,
        "spendable_points_observations_agree":trainer_points.zip(comparable_balance).map(|(a,b)| a == b),
        "same_pet_verified":false,"eligibility_verified":false,"live":false})
}

pub fn merchant_quantity_plan(offers: &Value, quantity: u32, exact_product: bool) -> Value {
    let Some(offers) = offers.as_object() else {
        return Value::Null;
    };
    let rows: BTreeMap<_, _> = offers
        .iter()
        .map(|(slot, offer)| (slot, merchant_quantity_row(offer, u64::from(quantity))))
        .collect();
    let lowest_cost = rows
        .iter()
        .filter_map(|(slot, plan)| {
            let plan = plan.as_ref()?;
            if plan["captured_stock_covers_quantity"] != true || plan["extended_cost"] != false {
                return None;
            }
            Some((
                plan["required_copper"].as_u64()?,
                slot.parse::<u32>().ok()?,
                plan,
            ))
        })
        .min_by_key(|(cost, slot, _)| (*cost, *slot));
    let mut report = json!({"target_quantity":quantity,"offers_by_slot":rows,
        "lowest_cost_copper_only_offer":lowest_cost.map(|(_, slot, plan)| json!({"slot":slot,"plan":plan}))});
    if exact_product {
        report["lowest_cost_copper_only_bundle_mix"] =
            cheapest_merchant_bundle_mix(offers.iter(), quantity).unwrap_or(Value::Null);
    }
    report
}

pub(crate) fn cheapest_merchant_bundle_mix<'a>(
    offers: impl Iterator<Item = (&'a String, &'a Value)>,
    quantity: u32,
) -> Option<Value> {
    if !(1..=1000).contains(&quantity) {
        return None;
    }
    let mut bundles: Vec<_> = offers
        .filter_map(|(slot, offer)| {
            if offer["available"] != -1 || offer["extended_cost"] != false {
                return None;
            }
            Some((
                slot.parse::<u32>().ok()?,
                offer["bundle_size"].as_u64()?.max(1),
                offer["price_copper"].as_u64()?,
            ))
        })
        .collect();
    bundles.sort_unstable_by_key(|(slot, _, _)| *slot);
    if bundles.is_empty() {
        return None;
    }

    // Each bundle strictly reduces remaining demand. Reusing a slot is valid for unlimited stock.
    let mut best = vec![None; quantity as usize + 1];
    best[0] = Some((0u64, 0u64, 0, 0));
    for remaining in 1..=quantity {
        best[remaining as usize] = bundles
            .iter()
            .filter_map(|&(slot, size, price)| {
                let next = u64::from(remaining).saturating_sub(size) as usize;
                let (cost, units, _, _) = best[next]?;
                Some((
                    cost.checked_add(price)?,
                    units.checked_add(size)?,
                    slot,
                    next,
                ))
            })
            .min();
    }
    let mut remaining = quantity as usize;
    let (cost, units, _, _) = best[remaining]?;
    let mut counts = BTreeMap::<u32, u32>::new();
    while remaining > 0 {
        let (_, _, slot, next) = best[remaining]?;
        *counts.entry(slot).or_default() += 1;
        remaining = next;
    }
    Some(
        json!({"required_copper":cost,"units":units,"extra_units":units-u64::from(quantity),
        "bundles_by_slot":counts}),
    )
}

pub(crate) fn merchant_quantity_row(offer: &Value, quantity: u64) -> Option<Value> {
    let size = offer["bundle_size"].as_u64().filter(|size| *size > 0)?;
    let bundles = quantity.div_ceil(size);
    let units = bundles.checked_mul(size)?;
    let copper = offer["price_copper"].as_u64()?.checked_mul(bundles);
    let captured_stock_covers_quantity = match offer["available"].as_i64() {
        Some(-1) => Some(true),
        Some(0) => Some(false),
        _ => None,
    };
    Some(json!({
        "bundles":bundles,"units":units,"extra_units":units - quantity,
        "required_copper":copper,"extended_cost":offer["extended_cost"],
        "captured_available":offer["available"],
        "captured_stock_covers_quantity":captured_stock_covers_quantity
    }))
}

pub fn recipe_merchant_plan(review: &Value, merchant: &Snapshot, inventory: &Snapshot) -> Value {
    let materials = if let Some(ordered) = review.get("ordered_starting_materials") {
        ordered.as_array()
    } else {
        review["materials"]
            .as_array()
            .filter(|_| review["recipe"]["requirements"] == "fixed_items")
    };
    let Some((materials, offers)) = materials.zip(
        merchant
            .sections
            .get("merchant_stock")
            .and_then(Value::as_object),
    ) else {
        return Value::Null;
    };
    let wallet = inventory
        .sections
        .get("wallet")
        .and_then(|row| row["copper"].as_u64());
    let mut plan = material_merchant_plan(materials, merchant, inventory);
    let checkpoint = &review["ordered_material_checkpoint"]["first_uncovered_recipe"];
    if let Some(blockers) = checkpoint["blocking_reagents_by_item_id"].as_object() {
        let demands = blockers.iter().filter_map(|(id, row)| {
            Some((id.parse().ok()?, row["shortfall_vs_observed"].as_u64()))
        });
        let mut budget = merchant_reagent_budget(demands, offers, wallet);
        budget["recipe_number"] = checkpoint["recipe_number"].clone();
        budget["recipe_id"] = checkpoint["recipe_id"].clone();
        if let Some(step) = checkpoint.get("suggested_step") {
            budget["suggested_step"] = step.clone();
        }
        plan["first_uncovered_recipe_budget"] = budget;
    }
    plan
}

/// Price already combined material shortfalls without inventing a recipe observation.
pub fn material_merchant_plan(
    materials: &[Value],
    merchant: &Snapshot,
    inventory: &Snapshot,
) -> Value {
    let Some(offers) = merchant
        .sections
        .get("merchant_stock")
        .and_then(Value::as_object)
    else {
        return Value::Null;
    };
    let demands = materials.iter().filter_map(|material| {
        let item = material["item_id"].as_u64()?;
        let shortfall = material["shortfall_vs_observed"].as_u64();
        (shortfall != Some(0) && material["required_before_batch"].as_u64() != Some(0))
            .then_some((item, shortfall))
    });
    let wallet = inventory
        .sections
        .get("wallet")
        .and_then(|row| row["copper"].as_u64());
    let mut plan = merchant_reagent_budget(demands, offers, wallet);
    plan["merchant_captured_at"] = json!(merchant.captured_at);
    plan
}

pub fn restock_travel_adjusted_plan(
    demands: &[Value],
    visit_ids: &[String],
    quotes: &[BTreeMap<u64, (u64, Value)>],
    visit_costs: &[u64],
    max_visits: u8,
    wallet: Option<u64>,
) -> Option<Value> {
    if visit_ids.len() != quotes.len() || visit_costs.len() != quotes.len() || quotes.len() > 4 {
        return None;
    }
    let items: Vec<_> = demands
        .iter()
        .filter(|row| row["shortfall_vs_observed"] != 0)
        .map(|row| row["item_id"].as_u64())
        .collect::<Option<_>>()?;
    let mut best = None;
    for subset in 0..(1u8 << quotes.len()) {
        let count = subset.count_ones();
        if count > u32::from(max_visits) {
            continue;
        }
        let assignments = items
            .iter()
            .map(|item| {
                quotes
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| subset & (1 << index) != 0)
                    .filter_map(|(index, quote)| {
                        quote.get(item).map(|(cost, plan)| (*cost, index, plan))
                    })
                    .min_by_key(|(cost, index, _)| (*cost, *index))
                    .map(|assignment| (*item, assignment))
            })
            .collect::<Option<Vec<_>>>();
        let Some(assignments) = assignments else {
            continue;
        };
        let Some(purchases) = assignments
            .iter()
            .try_fold(0u64, |sum, (_, (cost, _, _))| sum.checked_add(*cost))
        else {
            continue;
        };
        let Some(total) = visit_costs
            .iter()
            .enumerate()
            .filter(|(index, _)| subset & (1 << index) != 0)
            .try_fold(purchases, |sum, (_, cost)| sum.checked_add(*cost))
        else {
            continue;
        };
        let key = (total, count, subset);
        if best.as_ref().is_none_or(|(best_key, _, _)| key < *best_key) {
            best = Some((key, purchases, assignments));
        }
    }
    let ((total, count, _), purchases, assignments) = best?;
    let mut by_visit = BTreeMap::<usize, (u64, Vec<Value>)>::new();
    for &(item, (cost, index, plan)) in &assignments {
        let (subtotal, lines) = by_visit.entry(index).or_default();
        *subtotal = subtotal.checked_add(cost)?;
        lines.push(
            json!({"item_id":item,"bundles_by_slot":plan["bundles_by_slot"],
            "quoted_units":plan["units"],"purchase_copper":cost}),
        );
    }
    // The selected complete plan already bounded the sum of purchases and visit costs.
    let mut cumulative_copper = 0;
    let checklist: Vec<_> = by_visit
        .into_iter()
        .map(|(index, (subtotal, items))| {
            cumulative_copper += subtotal + visit_costs[index];
            json!({"merchant_snapshot_id":visit_ids[index],"purchase_copper":subtotal,
                "assumed_visit_cost_copper":visit_costs[index],"items":items,
                "cumulative_cost_copper":cumulative_copper,
                "inventory_wallet_remaining_copper":wallet.and_then(|balance| balance.checked_sub(cumulative_copper)),
                "inventory_wallet_shortfall_copper":wallet.map(|balance| cumulative_copper.saturating_sub(balance))})
        })
        .collect();
    Some(json!({
        "items":assignments.into_iter().map(|(item, (_, index, plan))| json!({
            "item_id":item,"merchant_snapshot_id":visit_ids[index],"bundle_plan":plan,
        })).collect::<Vec<_>>(),
        "visit_count":count,"merchant_visit_checklist":checklist,
        "purchase_copper":purchases,"assumed_visit_cost_copper":total-purchases,
        "complete_quote_copper":total,"affordable_from_inventory_wallet":wallet.map(|balance| total <= balance),
        "inventory_wallet_copper":wallet,
        "remaining_inventory_wallet_copper":wallet.and_then(|balance| balance.checked_sub(total)),
        "inventory_wallet_shortfall_copper":wallet.map(|balance| total.saturating_sub(balance)),
        "travel_cost_verified":false,"purchase_eligibility_verified":false,"live":false,
    }))
}

/// Explain why captured merchant rows may be absent from a copper-only restocking quote.
pub fn restock_merchant_offer_coverage(quote: &mut Value, merchant: &Snapshot) {
    let Some(offers) = merchant
        .sections
        .get("merchant_stock")
        .and_then(Value::as_object)
    else {
        return;
    };
    let mut by_item = BTreeMap::<u64, [usize; 4]>::new();
    for row in offers.values() {
        let Some(item_id) = row["item_id"].as_u64() else {
            continue;
        };
        let counts = by_item.entry(item_id).or_default();
        match row["available"].as_i64() {
            Some(0) => counts[0] += 1,
            _ if row["extended_cost"] == true => counts[1] += 1,
            Some(-1) => counts[2] += 1,
            Some(1..) => counts[3] += 1,
            _ => {}
        }
    }
    if let Some(reagents) = quote["reagents"].as_array_mut() {
        for reagent in reagents {
            let [sold_out, extended, unlimited, finite] = reagent["item_id"]
                .as_u64()
                .and_then(|id| by_item.get(&id).copied())
                .unwrap_or_default();
            reagent["captured_offer_coverage"] = json!({
                "sold_out_slots":sold_out,"in_stock_extended_cost_slots":extended,
                "unlimited_copper_only_slots":unlimited,"finite_copper_only_slots":finite,
                "finite_stock_quantity_verified":false,
            });
        }
    }
}

/// Join historical owner-local provider references to already-net chain leaves.
pub fn reagent_chain_captured_sources(
    chain: &Value,
    recipe: &Snapshot,
    source: &Snapshot,
) -> Result<Value> {
    ensure!(
        recipe.client.family == crate::Family::Forever,
        "Expected Forever"
    );
    ensure_same_character_and_client(recipe, source)?;
    if chain["draft_complete"] != true {
        return Ok(Value::Null);
    }
    let leaves = chain["external_material_requirements"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing chain external materials"))?;
    ensure!(leaves.len() <= 512, "Too many chain external materials");
    let mut needs = BTreeMap::new();
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
            needs.insert(id, quantity).is_none(),
            "Duplicate chain material"
        );
    }
    Ok(captured_material_sources(
        needs,
        source,
        "chain_external_material_shortfalls",
    ))
}

pub fn quest_inventory_captured_sources(
    review: &Value,
    quest: &Snapshot,
    source: &Snapshot,
) -> Result<Value> {
    let mut report = inventory_shortfall_captured_sources(
        review,
        quest,
        source,
        "user_supplied_quest_item_shortfalls",
    )?;
    if report.is_object() {
        report["quest_item_binding_verified"] = json!(false);
    }
    Ok(report)
}

pub fn inventory_shortfall_captured_sources(
    review: &Value,
    inventory: &Snapshot,
    source: &Snapshot,
    basis: &str,
) -> Result<Value> {
    ensure!(
        inventory.client.family == crate::Family::Forever,
        "Expected Forever"
    );
    ensure_same_character_and_client(inventory, source)?;
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
    Ok(captured_material_sources(needs, source, basis))
}

fn captured_material_sources(needs: BTreeMap<u64, u64>, source: &Snapshot, basis: &str) -> Value {
    let section = source.sections.get("recipe_item_sources");
    let fields = [
        "npc_drops",
        "object_drops",
        "container_drops",
        "quest_rewards",
        "vendors",
    ];
    let mut captured = 0;
    let mut reported = 0;
    let mut with_sources = 0;
    let records = needs
        .into_iter()
        .map(|(id, quantity)| {
            let row = section.and_then(|section| section["entries"].get(id.to_string()));
            if row.is_some() {
                captured += 1;
            }
            let status =
                row.and_then(|row| row["status"].as_str())
                    .unwrap_or(if section.is_some() {
                        "item_not_captured"
                    } else {
                        "capture_unavailable"
                    });
            let references = (status == "reported").then(|| {
                reported += 1;
                fields
                    .iter()
                    .filter_map(|field| {
                        row.unwrap().get(field).map(|value| (*field, value.clone()))
                    })
                    .collect::<BTreeMap<_, _>>()
            });
            let unknown = fields
                .iter()
                .filter(|field| references.as_ref().is_none_or(|r| !r.contains_key(**field)))
                .copied()
                .collect::<Vec<_>>();
            let truncated = fields
                .iter()
                .filter(|field| {
                    references.as_ref().is_some_and(|r| {
                        r.get(**field)
                            .is_some_and(|value| value["truncated"] == true)
                    })
                })
                .copied()
                .collect::<Vec<_>>();
            if references.as_ref().is_some_and(|refs| {
                refs.values().any(|value| {
                    value["entries"]
                        .as_object()
                        .is_some_and(|entries| !entries.is_empty())
                })
            }) {
                with_sources += 1;
            }
            json!({"item_id":id,"quantity":quantity,"status":status,"references":references,
            "unknown_reference_categories":unknown,"truncated_reference_categories":truncated})
        })
        .collect::<Vec<_>>();
    let coverage = section.map(|section| json!({"coverage":section["coverage"],"provider":section["provider"],
        "provider_mode":section["provider_mode"],"provider_contract_version":section["provider_contract_version"],
        "profession_id":section["profession_id"],"reported_item_count":section["reported_item_count"],
        "retained_item_count":section["entries"].as_object().map(|rows| rows.len())}));
    let locations = section
        .and_then(|section| section.get("entity_locations"))
        .map(|cache| {
            let mut projected = json!({"coordinate_system":cache["coordinate_system"],
            "coordinate_frame_verified":false,"phase_visibility_verified":false,
            "native_spawn_verified":false,"routing_verified":false});
            for (kind, count_field, categories) in [
                ("npcs", "reported_npc_count", &["npc_drops", "vendors"][..]),
                ("objects", "reported_object_count", &["object_drops"][..]),
            ] {
                let mut ids = BTreeSet::<u32>::new();
                for record in &records {
                    for category in categories {
                        if let Some(entries) =
                            record["references"][*category]["entries"].as_object()
                        {
                            ids.extend(entries.keys().map(|id| id.parse::<u32>().unwrap()));
                        }
                    }
                }
                projected[count_field] = cache[count_field].clone();
                projected[format!("retained_{kind}_count")] =
                    json!(cache[kind].as_object().unwrap().len());
                projected[format!("referenced_{kind}_count")] = json!(ids.len());
                projected[kind] = json!(
                    ids.into_iter()
                        .map(|id| {
                            let key = id.to_string();
                            let row = cache[kind]
                                .get(&key)
                                .cloned()
                                .unwrap_or_else(|| json!({"status":"not_captured"}));
                            (key, row)
                        })
                        .collect::<BTreeMap<_, _>>()
                );
            }
            projected
        });
    let mut result = json!({"basis":basis,"captured_at":source.captured_at,
        "section_available":section.is_some(),"source_capture_coverage":coverage,
        "external_item_count":records.len(),"captured_item_count":captured,
        "uncaptured_item_count":records.len()-captured,"items_reported_by_provider":reported,
        "items_with_retained_sources":with_sources,"records":records,
        "dataset_revision_verified":false,"complete_world_sources_verified":false,
        "current_availability_verified":false,"acquisition_eligibility_verified":false,
        "ownership_verified":false,"live":false,"game_execution":false});
    if let Some(locations) = locations {
        result["entity_locations"] = locations;
    }
    result
}

/// Chain leaves are already net shortages: do not subtract the observed stock again.
pub fn reagent_chain_merchant_plan(
    chain: &Value,
    merchant: &Snapshot,
    inventory: &Snapshot,
) -> Value {
    if chain["draft_complete"] != true {
        return Value::Null;
    }
    let Some(external) = chain["external_material_requirements"].as_array() else {
        return Value::Null;
    };
    let mut seen = BTreeSet::new();
    let materials: Option<Vec<_>> = external.iter().map(|row| {
        let item = row["item_id"].as_u64().filter(|n|*n > 0 && *n <= u64::from(u32::MAX))?;
        let quantity = row["quantity"].as_u64().filter(|n|*n > 0)?;
        if !seen.insert(item) { return None; }
        Some(json!({"item_id":item,"shortfall_vs_observed":quantity,"required_before_batch":quantity}))
    }).collect();
    let Some(materials) = materials else {
        return Value::Null;
    };
    let mut plan = material_merchant_plan(&materials, merchant, inventory);
    if plan.is_object() {
        plan["basis"] = json!("chain_external_material_shortfalls");
        plan["inventory_captured_at"] = json!(inventory.captured_at);
        plan["purchase_eligibility_verified"] = json!(false);
        plan["live"] = json!(false);
        plan["game_execution"] = json!(false);
    }
    plan
}

fn merchant_reagent_budget(
    demands: impl Iterator<Item = (u64, Option<u64>)>,
    offers: &serde_json::Map<String, Value>,
    wallet: Option<u64>,
) -> Value {
    let reagents: Vec<_> = demands
        .map(|(item, shortfall)| {
            let matching = offers
                .iter()
                .filter(|(_, offer)| offer["item_id"].as_u64() == Some(item));
            let bundle_mix = shortfall
                .and_then(|quantity| u32::try_from(quantity).ok())
                .and_then(|quantity| cheapest_merchant_bundle_mix(matching.clone(), quantity));
            let mix_status = match shortfall {
                None => "unknown_shortfall",
                Some(0 | 1001..) => "outside_optimiser_range",
                Some(_) if bundle_mix.is_some() => "quoted",
                Some(_) => "no_eligible_offer",
            };
            let options: BTreeMap<_, _> = matching
                .map(|(slot, offer)| {
                    (
                        slot,
                        shortfall.and_then(|quantity| merchant_quantity_row(offer, quantity)),
                    )
                })
                .collect();
            let lowest_copper = options
                .iter()
                .filter_map(|(slot, offer)| {
                    let offer = offer.as_ref()?;
                    if offer["captured_available"] != -1 || offer["extended_cost"] != false {
                        return None;
                    }
                    Some((
                        offer["required_copper"].as_u64()?,
                        slot.parse::<u32>().ok()?,
                    ))
                })
                .min()
                .map(|(copper, slot)| json!({"slot":slot,"required_copper":copper}));
            json!({
                "item_id":item,"shortfall_vs_observed":shortfall,"offers_by_slot":options,
                "lowest_unlimited_copper_only_offer":lowest_copper,
                "lowest_cost_copper_only_bundle_mix":bundle_mix,
                "bundle_mix_quote_status":mix_status
            })
        })
        .collect();
    let total = reagents.iter().try_fold(0u64, |total, reagent| {
        total
            .checked_add(reagent["lowest_unlimited_copper_only_offer"]["required_copper"].as_u64()?)
    });
    let budget = total.zip(wallet);
    let unquoted: BTreeSet<_> = reagents
        .iter()
        .filter(|reagent| reagent["bundle_mix_quote_status"] != "quoted")
        .filter_map(|reagent| reagent["item_id"].as_u64())
        .collect();
    let mix_subtotal = reagents
        .iter()
        .filter_map(|reagent| {
            reagent["lowest_cost_copper_only_bundle_mix"]["required_copper"].as_u64()
        })
        .try_fold(0u64, u64::checked_add);
    let mix_total = mix_subtotal.filter(|_| unquoted.is_empty());
    let mix_budget = mix_total.zip(wallet);
    json!({"reagents":reagents,
        "unlimited_copper_only_total":total,
        "unlimited_copper_only_affordable":budget.map(|(cost, balance)| cost <= balance),
        "unlimited_copper_only_shortfall":budget.map(|(cost, balance)| cost.saturating_sub(balance)),
        "copper_only_bundle_mix_total":mix_total,
        "copper_only_bundle_mix_quoted_subtotal":mix_subtotal,
        "copper_only_bundle_mix_quoted_subtotal_wallet_shortfall":
            mix_subtotal.zip(wallet).map(|(cost, balance)| cost.saturating_sub(balance)),
        "copper_only_bundle_mix_unquoted_item_ids":unquoted,
        "copper_only_bundle_mix_affordable":mix_budget.map(|(cost, balance)| cost <= balance),
        "copper_only_bundle_mix_shortfall":mix_budget.map(|(cost, balance)| cost.saturating_sub(balance))})
}

pub fn buyback_cost_review(
    snapshot: &Snapshot,
    slots: &[u32],
    reserve: u64,
    estimate_bag_space: bool,
) -> Result<Value> {
    ensure!(
        snapshot.client.family == crate::Family::Forever
            && (1..=12).contains(&slots.len())
            && slots.iter().all(|slot| (1..=12).contains(slot))
            && slots.iter().collect::<BTreeSet<_>>().len() == slots.len()
            && reserve <= 9_999_999_999_999,
        "Invalid buyback review"
    );
    let Some(entries) = snapshot
        .sections
        .get("merchant_buyback")
        .and_then(|section| section["entries"].as_object())
    else {
        return Ok(Value::Null);
    };
    let mut lines = Vec::new();
    let mut missing = Vec::new();
    let mut unknown_prices = Vec::new();
    let mut subtotal = 0u64;
    for slot in slots {
        let row = entries.get(&slot.to_string());
        if let Some(row) = row {
            if let Some(price) = row["buyback_price_copper"].as_u64() {
                subtotal = subtotal
                    .checked_add(price)
                    .ok_or_else(|| anyhow::anyhow!("Buyback quote total too large"))?;
            } else {
                unknown_prices.push(slot);
            }
        } else {
            missing.push(slot);
        }
        lines.push(json!({"captured_slot":slot,"observed":row.is_some(),"item":row}));
    }
    let total = (missing.is_empty() && unknown_prices.is_empty()).then_some(subtotal);
    let wallet = snapshot
        .sections
        .get("wallet")
        .and_then(|row| row["copper"].as_u64());
    let spendable = wallet.map(|balance| balance.saturating_sub(reserve));
    let reserve_shortfall = wallet.map(|balance| reserve.saturating_sub(balance));
    let known_shortfall = spendable.map(|balance| subtotal.saturating_sub(balance));
    let affordable = if reserve_shortfall.is_some_and(|gap| gap > 0)
        || known_shortfall.is_some_and(|gap| gap > 0)
    {
        Some(false)
    } else {
        total.zip(spendable).map(|(cost, balance)| cost <= balance)
    };
    let mut report = json!({"lines":lines,"missing_captured_slots":missing,"slots_with_unknown_price":unknown_prices,
        "known_price_subtotal_copper":subtotal,"total_selected_quote_copper":total,
        "captured_wallet_copper":wallet,"wallet_reserve_copper":reserve,"spendable_after_reserve_copper":spendable,
        "reserve_shortfall_copper":reserve_shortfall,"known_shortfall_vs_spendable_copper":known_shortfall,
        "total_shortfall_vs_spendable_copper":total.zip(spendable).map(|(cost,balance)|cost.saturating_sub(balance)),
        "affordable_from_captured_wallet":affordable,"buyback_eligibility_verified":false,"game_action_performed":false});
    if estimate_bag_space {
        let mut quantities = BTreeMap::<u32, u64>::new();
        for row in slots
            .iter()
            .filter_map(|slot| entries.get(&slot.to_string()))
        {
            let id = u32::try_from(row["item_id"].as_u64().unwrap())?;
            // Validated captures contain at most 12 entries with u32 quantities.
            *quantities.entry(id).or_default() += row["count"].as_u64().unwrap();
        }
        let mut space = incoming_items_bag_space(
            snapshot,
            quantities.into_iter(),
            "observed_selected_buyback_items_only",
            "selected_quantity",
        );
        space["all_selected_slots_observed"] = json!(missing.is_empty());
        if !missing.is_empty() {
            space["fits_recorded_general_slots"] = Value::Null;
            space["fits_recorded_general_slots_if_same_item_stacks_merge"] = Value::Null;
        }
        report["bag_space_projection"] = space;
    }
    Ok(report)
}

pub fn merchant_budget(snapshot: &Snapshot, offer: &Value, bundles: u32) -> Result<Value> {
    ensure!((1..=1000).contains(&bundles), "Invalid bundle count");
    let bundles = u64::from(bundles);
    let price = offer["price_copper"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid merchant price"))?;
    let size = offer["bundle_size"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid merchant bundle"))?;
    let copper = price * bundles;
    let wallet = snapshot
        .sections
        .get("wallet")
        .and_then(|wallet| wallet["copper"].as_u64());
    let mut token_costs = BTreeMap::<u32, u64>::new();
    let mut named_currencies = Vec::new();
    let components = offer["cost_components"].as_object();
    for cost in components.into_iter().flat_map(|rows| rows.values()) {
        let quantity = cost["quantity"].as_u64().unwrap() * bundles;
        if let Some(id) = cost["item_id"].as_u64() {
            *token_costs.entry(u32::try_from(id)?).or_default() += quantity;
        } else {
            named_currencies
                .push(json!({"currency_name":cost["currency_name"],"required_quantity":quantity}));
        }
    }
    let mut tokens = Vec::new();
    let mut resource_limits = Vec::new();
    let mut balances_known = price == 0 || wallet.is_some();
    if price > 0 {
        resource_limits.extend(wallet.map(|balance| balance / price));
    }
    for (id, required) in token_costs {
        let found = find_item(snapshot, id, 0, 1)?;
        let carried = found["quantity_by_storage"]["bags"].as_u64();
        balances_known &= carried.is_some();
        resource_limits.extend(carried.map(|count| count / (required / bundles)));
        tokens.push(json!({"item_id":id,"required_quantity":required,"shortfall_vs_bags":carried.map(|count| required.saturating_sub(count))}));
    }
    let costs_observed = offer["extended_cost"].as_bool() == Some(false) || components.is_some();
    let resource_upper_bound = resource_limits.into_iter().min();
    let resource_capacity = if resource_upper_bound == Some(0)
        || (costs_observed && named_currencies.is_empty() && balances_known)
    {
        json!({
            "status":if resource_upper_bound.is_some() {"bounded"} else {"no_recorded_cost"},
            "max_bundles":resource_upper_bound
        })
    } else {
        json!({"status":"unknown","max_bundles":null})
    };
    let stock = match offer["available"].as_i64() {
        Some(-1) => "unlimited",
        Some(0) => "sold_out",
        Some(1..) => "limited_quantity_unverified",
        _ => "unknown",
    };
    Ok(json!({
        "requested_bundles":bundles,"requested_quantity":size * bundles,"required_copper":copper,
        "captured_stock_status":stock,"captured_available":offer["available"],
        "captured_resource_capacity":resource_capacity,
        "wallet_shortfall_copper":wallet.map(|balance| copper.saturating_sub(balance)),
        "item_token_costs":costs_observed.then_some(tokens),
        "named_currency_costs":costs_observed.then_some(named_currencies)
    }))
}

pub fn merchant_cart_budget(
    snapshot: &Snapshot,
    cart: impl Iterator<Item = (u32, u32)>,
    include_repair: bool,
    estimate_bag_space: bool,
) -> Result<Value> {
    let Some(offers) = snapshot
        .sections
        .get("merchant_stock")
        .and_then(Value::as_object)
    else {
        return Ok(Value::Null);
    };
    let mut lines = BTreeMap::new();
    let mut missing = Vec::new();
    let mut copper = 0u64;
    let mut token_costs = BTreeMap::<u32, u64>::new();
    let mut unknown_token_cost_slots = Vec::new();
    let mut named_currency_cost_slots = Vec::new();
    let mut sold_out_slots = Vec::new();
    for (slot, bundles) in cart {
        let Some(offer) = offers.get(&slot.to_string()) else {
            missing.push(slot);
            continue;
        };
        let mut line = merchant_budget(snapshot, offer, bundles)?;
        if line["captured_stock_status"] == "sold_out" {
            sold_out_slots.push(slot);
        }
        copper = copper
            .checked_add(line["required_copper"].as_u64().unwrap())
            .ok_or_else(|| anyhow::anyhow!("Merchant cart cost is too large"))?;
        if let Some(tokens) = line["item_token_costs"].as_array() {
            for token in tokens {
                let id = u32::try_from(token["item_id"].as_u64().unwrap())?;
                *token_costs.entry(id).or_default() += token["required_quantity"].as_u64().unwrap();
            }
        } else {
            unknown_token_cost_slots.push(slot);
        }
        if line["named_currency_costs"]
            .as_array()
            .is_some_and(|costs| !costs.is_empty())
        {
            named_currency_cost_slots.push(slot);
        }
        line["item_id"] = offer["item_id"].clone();
        line["currency_id"] = offer["currency_id"].clone();
        lines.insert(slot, line);
    }
    let wallet = snapshot
        .sections
        .get("wallet")
        .and_then(|row| row["copper"].as_u64());
    let mut tokens = Vec::new();
    for (id, required) in token_costs {
        let found = find_item(snapshot, id, 0, 1)?;
        let carried = found["quantity_by_storage"]["bags"].as_u64();
        tokens.push(json!({"item_id":id,"required_quantity":required,
            "shortfall_vs_bags":carried.map(|count| required.saturating_sub(count))}));
    }
    let copper_shortfall = wallet.map(|balance| copper.saturating_sub(balance));
    let repair = snapshot
        .sections
        .get("repair_quote")
        .and_then(|quote| quote["copper"].as_u64());
    let budget_copper = if include_repair {
        copper
            .checked_add(repair.unwrap_or_default())
            .ok_or_else(|| anyhow::anyhow!("Merchant and repair cost is too large"))?
    } else {
        copper
    };
    let budget_shortfall = wallet.map(|balance| budget_copper.saturating_sub(balance));
    let repair_reservation = include_repair.then(|| json!({
        "captured_quote_copper":repair,
        "known_combined_copper":budget_copper,
        "total_combined_copper":(missing.is_empty() && repair.is_some()).then_some(budget_copper),
        "known_combined_shortfall_copper":budget_shortfall
    }));
    let item_costs_complete = missing.is_empty() && unknown_token_cost_slots.is_empty();
    let token_shortfalls = tokens.iter().map(|row| row["shortfall_vs_bags"].as_u64());
    let budget_status = if budget_shortfall.is_some_and(|gap| gap > 0)
        || token_shortfalls.clone().flatten().any(|gap| gap > 0)
    {
        "insufficient"
    } else if item_costs_complete
        && named_currency_cost_slots.is_empty()
        && (!include_repair || repair.is_some())
        && (budget_copper == 0 || budget_shortfall == Some(0))
        && token_shortfalls.into_iter().all(|gap| gap == Some(0))
    {
        "fits_observed_budget"
    } else {
        "unknown"
    };
    let mut result = json!({"lines_by_slot":lines,"missing_slots":missing,"known_required_copper":copper,
        "total_required_copper":missing.is_empty().then_some(copper),
        "known_item_token_costs":tokens,
        "item_token_costs_complete":item_costs_complete,
        "unknown_item_token_cost_slots":unknown_token_cost_slots,
        "named_currency_cost_slots":named_currency_cost_slots,
        "sold_out_slots":sold_out_slots,
        "known_copper_shortfall":copper_shortfall,"resource_budget_status":budget_status});
    if let Some(reservation) = repair_reservation {
        result["repair_reservation"] = reservation;
    }
    if estimate_bag_space {
        let mut quantities = BTreeMap::<u32, u64>::new();
        for line in lines.values() {
            if let Some(id) = line["item_id"].as_u64() {
                let total = quantities.entry(u32::try_from(id)?).or_default();
                *total = total
                    .checked_add(line["requested_quantity"].as_u64().unwrap())
                    .ok_or_else(|| anyhow::anyhow!("Merchant cart quantity is too large"))?;
            }
        }
        let mut space = incoming_items_bag_space(
            snapshot,
            quantities.into_iter(),
            "observed_cart_item_bundles_before_purchase",
            "requested_quantity",
        );
        space["all_cart_slots_observed"] = json!(missing.is_empty());
        space["item_token_slots_freed_included"] = json!(false);
        if !missing.is_empty() {
            space["fits_recorded_general_slots"] = Value::Null;
            space["fits_recorded_general_slots_if_same_item_stacks_merge"] = Value::Null;
        }
        result["bag_space_projection"] = space;
    }
    Ok(result)
}

pub fn flight_wallet_projection(map: &Value, balance: Option<u64>, reserve: u64) -> Value {
    let Some(entries) = map["entries"].as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries.iter().map(|(slot, node)| {
            let fare = node["fare_copper"].as_u64();
            let cost_and_balance = fare.and_then(|fare| fare.checked_add(reserve)).zip(balance);
            (slot.clone(), json!({
                "affordable":cost_and_balance.map(|(cost, balance)| cost <= balance),
                "shortfall_copper":cost_and_balance.map(|(cost, balance)| cost.saturating_sub(balance)),
                "remaining_wallet_after_fare_copper":fare.zip(balance).and_then(|(fare, balance)| balance.checked_sub(fare)),
                "spendable_after_fare_and_reserve_copper":cost_and_balance.and_then(|(cost, balance)| balance.checked_sub(cost)),
            }))
        }).collect()
    )
}

pub fn quest_list_wallet_projection(
    quests: &serde_json::Map<String, Value>,
    wallet: Option<&Value>,
) -> Value {
    quest_rows_wallet_projection(quests.iter(), wallet)
}

fn quest_rows_wallet_projection<'a>(
    quests: impl Iterator<Item = (&'a String, &'a Value)> + Clone,
    wallet: Option<&Value>,
) -> Value {
    let mut known_cost = 0u128;
    let mut known_reward = 0u128;
    let mut unknown_cost = 0;
    let mut unknown_reward = 0;
    for (_, quest) in quests.clone() {
        match quest["required_copper"].as_u64() {
            Some(copper) => known_cost += u128::from(copper),
            None => unknown_cost += 1,
        }
        match quest["reward_amounts"]["copper"].as_u64() {
            Some(copper) => known_reward += u128::from(copper),
            None => unknown_reward += 1,
        }
    }
    let known_cost = u64::try_from(known_cost).ok();
    let balance = wallet.and_then(|row| row["copper"].as_u64());
    let estimate = known_cost.zip(balance);
    json!({
        "known_required_copper":known_cost,"unknown_cost_count":unknown_cost,
        "known_reward_copper":u64::try_from(known_reward).ok(),"unknown_reward_count":unknown_reward,
        "ready_turn_in_order":quest_wallet_turn_in_order(quests, balance),
        "wallet_observed":balance.is_some(),
        "known_cost_shortfall_without_rewards_copper":estimate.map(|(cost,balance)| cost.saturating_sub(balance)),
        "all_costs_fit_wallet_without_rewards":estimate.and_then(|(cost,balance)| {
            if cost > balance { Some(false) } else if unknown_cost == 0 { Some(true) } else { None }
        })
    })
}

pub fn quest_xp_wallet_projection(
    quests: &serde_json::Map<String, Value>,
    wallet: Option<&Value>,
    xp_to_threshold: u64,
) -> Value {
    let mut projection = quest_list_wallet_projection(quests, wallet);
    let order = &projection["ready_turn_in_order"];
    let prefix_xp = order["quest_ids"].as_array().and_then(|ids| {
        let count =
            usize::try_from(order["captured_wallet_progress"]["affordable_prefix_count"].as_u64()?)
                .ok()?;
        ids.iter().take(count).try_fold(0u64, |total, id| {
            total.checked_add(quests.get(id.as_str()?)?["reward_amounts"]["xp"].as_u64()?)
        })
    });
    projection["xp_from_affordable_prefix"] = json!(prefix_xp);
    projection["xp_shortfall_after_affordable_prefix"] =
        json!(prefix_xp.map(|xp| xp_to_threshold.saturating_sub(xp)));
    projection
}

pub(crate) fn quest_wallet_turn_in_order<'a>(
    quests: impl Iterator<Item = (&'a String, &'a Value)>,
    balance: Option<u64>,
) -> Option<Value> {
    let mut steps = quests
        .map(|(id, quest)| {
            if quest_readiness(quest) != QuestReadiness::Ready {
                return None;
            }
            Some((
                id,
                quest["required_copper"].as_u64()?,
                quest["reward_amounts"]["copper"].as_u64()?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    if steps.is_empty() {
        return None;
    }
    // Do net gains first (cheapest first), then net losses (largest reward first).
    steps.sort_by_key(|(_, cost, reward)| {
        if reward >= cost {
            (false, *cost)
        } else {
            (true, u64::MAX - reward)
        }
    });
    let mut net = 0i128;
    let mut minimum = 0i128;
    let mut first_blocked = None;
    for (index, (id, cost, reward)) in steps.iter().enumerate() {
        let required = i128::from(*cost) - net;
        minimum = minimum.max(required);
        if first_blocked.is_none()
            && let Some(balance) = balance
            && required > i128::from(balance)
        {
            first_blocked = Some((index, *id, required - i128::from(balance)));
        }
        net += i128::from(*reward) - i128::from(*cost);
    }
    let minimum = u64::try_from(minimum).ok()?;
    let progress = balance.map(|_| {
        let (affordable, blocked_id, shortfall) = first_blocked
            .map_or((steps.len(), None, None), |(index, id, gap)| {
                (index, Some(id), u64::try_from(gap).ok())
            });
        json!({"affordable_prefix_count":affordable,"first_blocked_quest_id":blocked_id,
            "first_blocked_shortfall_copper":shortfall})
    });
    Some(json!({
        "quest_ids":steps.into_iter().map(|(id, _, _)| id).collect::<Vec<_>>(),
        "minimum_starting_copper":minimum,
        "shortfall_copper":balance.map(|balance| minimum.saturating_sub(balance)),
        "captured_wallet_progress":progress
    }))
}

pub fn quest_wallet_projection(quest: Option<&Value>, wallet: Option<&Value>) -> Value {
    let Some(quest) = quest else {
        return Value::Null;
    };
    let cost = quest["required_copper"].as_u64();
    let reward = quest["reward_amounts"]["copper"].as_u64();
    let balance = wallet.and_then(|wallet| wallet["copper"].as_u64());
    let upfront = balance.zip(cost);
    json!({
        "required_copper":cost,
        "reward_copper":reward,
        "net_copper":reward.zip(cost).map(|(reward, cost)| i128::from(reward) - i128::from(cost)),
        "upfront_shortfall_copper":upfront.map(|(balance, cost)| cost.saturating_sub(balance)),
        "projected_wallet_copper":upfront.and_then(|(balance, cost)| balance.checked_sub(cost)).zip(reward)
            .and_then(|(remaining, reward)| remaining.checked_add(reward))
    })
}

pub fn profession_training_progress(section: &Value) -> Value {
    let Some(entries) = section.as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries
            .iter()
            .map(|(slot, profession)| (slot.clone(), profession_rank_progress(profession)))
            .collect(),
    )
}

pub fn profession_rank_progress(profession: &Value) -> Value {
    let remaining = profession["max_rank"]
        .as_u64()
        .filter(|maximum| *maximum > 0)
        .zip(profession["rank"].as_u64())
        .and_then(|(maximum, rank)| maximum.checked_sub(rank));
    json!({"skill_line":profession["skill_line"],"remaining_to_observed_cap":remaining})
}

fn observed_profession_cap(profession: &Value, rank: Option<u64>) -> Option<u64> {
    profession["max_rank"]
        .as_u64()
        .filter(|cap| *cap > 0 && rank.is_none_or(|rank| rank <= *cap))
}

pub fn profession_target_progress(profession: Option<&Value>, target: u32) -> Value {
    let target = u64::from(target);
    let rank = profession.and_then(|row| row["rank"].as_u64());
    let cap = profession.and_then(|row| observed_profession_cap(row, rank));
    json!({
        "target_rank": target,
        "points_to_target": rank.map(|rank| target.saturating_sub(rank)),
        "target_reached_at_capture": rank.map(|rank| rank >= target),
        "target_above_observed_cap": cap.map(|cap| target > cap),
        "points_above_observed_cap": cap.map(|cap| target.saturating_sub(cap))
    })
}

pub fn profession_target_trainer_lessons(
    professions: &Value,
    profession: Option<&Value>,
    target: u32,
    trainer: Option<&Value>,
    wallet_copper: Option<u64>,
    reserve_copper: Option<u64>,
) -> Option<Vec<Value>> {
    let profession = profession?;
    let name = profession["name"]
        .as_str()
        .filter(|name| !name.trim().is_empty())?;
    let rank = profession["rank"].as_u64()?;
    let cap = observed_profession_cap(profession, Some(rank));
    if professions
        .as_object()?
        .values()
        .filter(|row| row["name"].as_str() == Some(name))
        .count()
        != 1
    {
        return None;
    }
    let trainer = trainer.filter(|row| row["kind"] != "pet")?;
    let mut lessons: Vec<_> = trainer["entries"].as_object()?.iter().filter_map(|(slot, lesson)| {
        let skill = &lesson["skill_requirement"];
        let required = skill["required_rank"].as_u64()?;
        let slot = slot.parse::<u32>().ok()?;
        let quote = lesson["price_copper"].as_u64().zip(wallet_copper);
        (skill["name"].as_str() == Some(name) && rank < required && required <= u64::from(target))
            .then(|| {
                let mut result = json!({"slot":slot,"name":lesson["name"],
                "required_rank":required,"points_to_required_rank":required-rank,
                "required_rank_above_observed_cap":cap.map(|cap| required > cap),
                "captured_status":lesson["status"],
                "price_within_captured_wallet":quote.map(|(price, wallet)| price <= wallet),
                "price_shortfall_copper":quote.map(|(price, wallet)| price.saturating_sub(wallet)),
                "captured_skill_requirement_met":skill["met"],"price_copper":lesson["price_copper"]});
                if let Some(reserve) = reserve_copper {
                    let shortfall = quote.and_then(|(price, wallet)| reserved_balance_shortfall(price, wallet, reserve));
                    let reserve_covered = wallet_copper.map(|wallet| wallet >= reserve);
                    result["after_reserve"] = json!({
                        "reserve_copper":reserve,
                        "reserve_covered":reserve_covered,
                        "price_affordable":if reserve_covered == Some(false) { Some(false) }
                            else { shortfall.map(|amount| amount == 0) },
                        "price_and_reserve_shortfall_copper":shortfall
                    });
                }
                (required, slot, result)
            })
    }).collect();
    lessons.sort_unstable_by_key(|(required, slot, _)| (*required, *slot));
    Some(lessons.into_iter().map(|(_, _, lesson)| lesson).collect())
}

pub fn profession_trainer_quote_budget(
    lessons: &[Value],
    wallet: Option<u64>,
    reserve: u64,
) -> Value {
    let priced: Vec<_> = lessons
        .iter()
        .filter_map(|lesson| Some((lesson["price_copper"].as_u64()?, lesson["slot"].as_u64()?)))
        .collect();
    let subtotal = priced.iter().map(|(price, _)| price).sum();
    let unknown_count = lessons.len() - priced.len();
    let spendable = wallet.and_then(|wallet| wallet.checked_sub(reserve));
    let cheapest = spendable.map(|spendable| {
        let (slots, cost) = cheapest_affordable_rows(priced, spendable);
        json!({"lesson_slots":slots,"lesson_count":slots.len(),"quote_subtotal_copper":cost})
    });
    let known_shortfall =
        wallet.and_then(|wallet| reserved_balance_shortfall(subtotal, wallet, reserve));
    let affordable = known_shortfall.and_then(|shortfall| {
        if shortfall > 0 {
            Some(false)
        } else if unknown_count == 0 {
            Some(true)
        } else {
            None
        }
    });
    json!({
        "scope":"projected_trainer_rows",
        "lesson_count":lessons.len(),
        "unknown_price_count":unknown_count,
        "known_quote_subtotal_copper":subtotal,
        "reserve_copper":reserve,
        "reserve_covered":wallet.map(|wallet| wallet >= reserve),
        "known_quotes_and_reserve_shortfall_copper":known_shortfall,
        "all_quotes_and_reserve_shortfall_copper":(unknown_count == 0).then_some(known_shortfall).flatten(),
        "all_quotes_fit_wallet_with_reserve":affordable,
        "cheapest_affordable_known_quotes":cheapest,
        "rank_order_quote_budget":spendable.map(|balance| profession_rank_order_quote_budget(lessons, balance)),
        "training_eligibility_verified":false
    })
}

fn profession_rank_order_quote_budget(lessons: &[Value], mut remaining: u64) -> Value {
    let mut covered = 0;
    let mut first_uncovered = None;
    for tier in lessons.chunk_by(|left, right| left["required_rank"] == right["required_rank"]) {
        let subtotal: u64 = tier
            .iter()
            .filter_map(|lesson| lesson["price_copper"].as_u64())
            .sum();
        let unknown = tier
            .iter()
            .filter(|lesson| lesson["price_copper"].is_null())
            .count();
        if subtotal > remaining || unknown > 0 {
            first_uncovered = Some(json!({
                "required_rank":tier[0]["required_rank"],
                "lesson_count":tier.len(),
                "unknown_price_count":unknown,
                "known_quote_subtotal_copper":subtotal,
                "known_shortfall_copper":subtotal.saturating_sub(remaining)
            }));
            break;
        }
        remaining -= subtotal;
        covered += tier.len();
    }
    json!({
        "covered_prefix_lessons":covered,
        "remaining_spendable_copper":remaining,
        "first_uncovered_rank":first_uncovered
    })
}

fn tracked_achievement_workload(section: &Value) -> Option<Value> {
    let entries = section["entries"].as_object()?;
    let mut unfinished = Vec::new();
    let mut completed_without_credit = Vec::new();
    for (id, achievement) in entries {
        let Ok(id) = id.parse::<u32>() else { continue };
        if achievement["completed"] == true && achievement["earned_by_character"] == false {
            completed_without_credit.push(id);
        }
        let Some(count) = unfinished_achievement_criteria_count(achievement) else {
            continue;
        };
        if count > 0 {
            unfinished.push((count, id, achievement));
        }
    }
    completed_without_credit.sort_unstable();
    unfinished.sort_unstable_by_key(|&(count, id, _)| (std::cmp::Reverse(count), id));
    let rows: Vec<_> = unfinished
        .iter()
        .take(20)
        .map(|(count, id, achievement)| {
            json!({
                "achievement_id":id,"name":achievement["name"],"completed":achievement["completed"],
                "earned_by_character":achievement["earned_by_character"],"account_wide":achievement["account_wide"],
                "unfinished_criteria":count
            })
        })
        .collect();
    Some(
        json!({"achievements_with_unfinished_criteria":unfinished.len(),
        "achievements_with_most_unfinished_criteria":rows,
        "completed_without_character_credit_ids":completed_without_credit}),
    )
}

pub(crate) fn unfinished_achievement_criteria_count(achievement: &Value) -> Option<usize> {
    Some(
        achievement["criteria"]
            .as_object()?
            .values()
            .filter(|row| row["completed"] == false)
            .count(),
    )
}

#[derive(Clone, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AchievementFilter {
    pub completed: Option<bool>,
    pub earned_by_character: Option<bool>,
    pub account_wide: Option<bool>,
    pub criterion_id: Option<std::num::NonZeroU32>,
    pub criterion_completed: Option<bool>,
    pub criterion_max_remaining: Option<u32>,
    /// Maximum number of explicitly unfinished captured criteria on the achievement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_unfinished_criteria: Option<u32>,
}

impl AchievementFilter {
    pub fn is_empty(&self) -> bool {
        self.completed.is_none()
            && self.earned_by_character.is_none()
            && self.account_wide.is_none()
            && self.criterion_id.is_none()
            && self.criterion_completed.is_none()
            && self.criterion_max_remaining.is_none()
            && self.maximum_unfinished_criteria.is_none()
    }

    pub fn matches(&self, achievement: &Value) -> Option<bool> {
        for (field, expected) in [
            ("completed", self.completed),
            ("earned_by_character", self.earned_by_character),
            ("account_wide", self.account_wide),
        ] {
            if expected.is_some_and(|expected| achievement[field].as_bool() != Some(expected)) {
                return Some(false);
            }
        }
        let has_criterion_filter = self.criterion_id.is_some()
            || self.criterion_completed.is_some()
            || self.criterion_max_remaining.is_some();
        if !has_criterion_filter && self.maximum_unfinished_criteria.is_none() {
            return Some(true);
        }
        let criteria = achievement["criteria"].as_object()?;
        if let Some(maximum) = self.maximum_unfinished_criteria {
            if unfinished_achievement_criteria_count(achievement)? > maximum as usize {
                return Some(false);
            }
        }
        if !has_criterion_filter {
            return Some(true);
        }
        let mut unknown = false;
        for criterion in criteria.values() {
            if self
                .criterion_completed
                .is_some_and(|expected| criterion["completed"].as_bool() != Some(expected))
            {
                continue;
            }
            let mut row_unknown = false;
            if let Some(id) = self.criterion_id {
                match criterion["criterion_id"].as_u64() {
                    Some(actual) if actual != u64::from(id.get()) => continue,
                    Some(_) => {}
                    None => row_unknown = true,
                }
            }
            if let Some(maximum) = self.criterion_max_remaining {
                match criterion["required"]
                    .as_u64()
                    .zip(criterion["quantity"].as_u64())
                    .map(|(required, quantity)| required.saturating_sub(quantity))
                {
                    Some(remaining) if remaining > u64::from(maximum) => continue,
                    Some(_) => {}
                    None => row_unknown = true,
                }
            }
            if !row_unknown {
                return Some(true);
            }
            unknown = true;
        }
        if unknown { None } else { Some(false) }
    }
}

pub fn tracked_achievement_work(section: &Value) -> Value {
    let Some(entries) = section["entries"].as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries
            .iter()
            .filter_map(|(id, achievement)| {
                let criteria = achievement["criteria"].as_object()?;
                let unfinished = criteria
                    .iter()
                    .filter(|(_, criterion)| criterion["completed"].as_bool() == Some(false))
                    .map(|(index, criterion)| {
                        let remaining = criterion["required"]
                            .as_u64()
                            .zip(criterion["quantity"].as_u64())
                            .map(|(required, quantity)| required.saturating_sub(quantity));
                        (
                            index.clone(),
                            json!({"criterion_id":criterion["criterion_id"],"remaining_count":remaining}),
                        )
                    })
                    .collect();
                Some((id.clone(), Value::Object(unfinished)))
            })
            .collect(),
    )
}

fn quest_objective_counts(objectives: &serde_json::Map<String, Value>) -> Value {
    let mut counts = BTreeMap::<&str, (usize, usize)>::new();
    for row in objectives.values() {
        if let Some((kind, finished)) = row["kind"].as_str().zip(row["finished"].as_bool()) {
            let (done, pending) = counts.entry(kind).or_default();
            if finished {
                *done += 1;
            } else {
                *pending += 1;
            }
        }
    }
    json!(
        counts
            .into_iter()
            .map(|(kind, (finished, unfinished))| {
                (kind, json!({"finished":finished,"unfinished":unfinished}))
            })
            .collect::<BTreeMap<_, _>>()
    )
}

pub fn quest_objective_quantities(objectives: &serde_json::Map<String, Value>) -> Value {
    let mut counts = BTreeMap::<&str, (u64, u64, u64, usize, usize)>::new();
    for row in objectives.values() {
        let Some(kind) = row["kind"].as_str() else {
            continue;
        };
        let (fulfilled, required, remaining, counted, unknown) = counts.entry(kind).or_default();
        if let Some((done, target)) = row["fulfilled"].as_u64().zip(row["required"].as_u64()) {
            *fulfilled += done;
            *required += target;
            *remaining += target.saturating_sub(done);
            *counted += 1;
        } else {
            *unknown += 1;
        }
    }
    json!(
        counts
            .into_iter()
            .map(
                |(kind, (fulfilled, required, remaining, counted, unknown))| {
                    (
                        kind,
                        json!({"fulfilled":(counted > 0).then_some(fulfilled),
            "required":(counted > 0).then_some(required),
            "remaining":(counted > 0).then_some(remaining),
            "counted_objectives":counted,"unknown_objectives":unknown}),
                    )
                }
            )
            .collect::<BTreeMap<_, _>>()
    )
}

fn quest_objective_observations_equal(before: &Value, after: &Value) -> bool {
    ["kind", "finished", "required", "fulfilled"]
        .into_iter()
        .all(|field| before[field] == after[field])
}

fn compare_quest_log_objectives(
    before: &serde_json::Map<String, Value>,
    after: &serde_json::Map<String, Value>,
) -> Value {
    let mut changed = BTreeSet::new();
    let mut unavailable = BTreeSet::new();
    let mut newly_finished = BTreeSet::new();
    let mut no_longer_finished = BTreeSet::new();
    let mut fewer_remaining = BTreeSet::new();
    let mut more_remaining = BTreeSet::new();
    let mut compared = 0;
    for (key, old) in before {
        let Some(new) = after.get(key) else { continue };
        let Ok(id) = key.parse::<u32>() else { continue };
        let Some((old, new)) = old["objectives"]
            .as_object()
            .zip(new["objectives"].as_object())
        else {
            unavailable.insert(id);
            continue;
        };
        compared += 1;
        if old.len() != new.len() {
            changed.insert(id);
        }
        for (index, row) in old {
            let Some(other) = new.get(index) else {
                changed.insert(id);
                continue;
            };
            if quest_objective_observations_equal(row, other) {
                continue;
            }
            changed.insert(id);
            if let Some((_, remaining)) = quest_objective_count_deltas(row, other) {
                if remaining < 0 {
                    fewer_remaining.insert(id);
                } else if remaining > 0 {
                    more_remaining.insert(id);
                }
            }
            if row["kind"] == other["kind"] && row["required"] == other["required"] {
                match (row["finished"].as_bool(), other["finished"].as_bool()) {
                    (Some(false), Some(true)) => {
                        newly_finished.insert(id);
                    }
                    (Some(true), Some(false)) => {
                        no_longer_finished.insert(id);
                    }
                    _ => {}
                }
            }
        }
    }
    json!({"compared_quest_count":compared,"changed_quest_ids":changed,
        "quest_ids_with_newly_finished_objectives":newly_finished,
        "quest_ids_with_no_longer_finished_objectives":no_longer_finished,
        "quest_ids_with_decreased_remaining_counts":fewer_remaining,
        "quest_ids_with_increased_remaining_counts":more_remaining,
        "quest_ids_with_unavailable_objectives":unavailable,"objective_identity_verified":false})
}

fn quest_objective_count_deltas(old: &Value, new: &Value) -> Option<(i128, i128)> {
    let required = old["required"].as_u64()?;
    (old["kind"] == new["kind"] && new["required"].as_u64() == Some(required)).then_some(())?;
    let old = old["fulfilled"].as_u64()?;
    let new = new["fulfilled"].as_u64()?;
    Some((
        i128::from(new) - i128::from(old),
        i128::from(required.saturating_sub(new)) - i128::from(required.saturating_sub(old)),
    ))
}

fn compare_quest_objective_indices(
    before: &serde_json::Map<String, Value>,
    after: &serde_json::Map<String, Value>,
) -> Value {
    let indices: BTreeSet<_> = before
        .keys()
        .chain(after.keys())
        .filter_map(|key| key.parse::<u32>().ok())
        .collect();
    let changes: Vec<_> = indices
        .into_iter()
        .filter_map(|index| {
            let key = index.to_string();
            let old = before.get(&key);
            let new = after.get(&key);
            if old
                .zip(new)
                .is_some_and(|(old, new)| quest_objective_observations_equal(old, new))
            {
                return None;
            }
            let deltas = old
                .zip(new)
                .and_then(|(old, new)| quest_objective_count_deltas(old, new));
            let project = |row: &Value| {
                json!({"kind":row["kind"],"finished":row["finished"],
            "required":row["required"],"fulfilled":row["fulfilled"]})
            };
            Some(
                json!({"index":index,"before":old.map(project),"after":new.map(project),
            "fulfilled_delta":deltas.map(|(fulfilled, _)| fulfilled),
            "remaining_count_delta":deltas.map(|(_, remaining)| remaining)}),
            )
        })
        .collect();
    json!({"changes":changes,"objective_identity_verified":false})
}

fn quest_objective_workload(quests: &serde_json::Map<String, Value>) -> Value {
    let mut by_kind = BTreeMap::<&str, usize>::new();
    let mut quests_without_objectives = 0usize;
    let mut quests_with_work = Vec::new();
    for (id, quest) in quests.iter().filter(|(_, quest)| quest["failed"] != true) {
        let Some(objectives) = quest["objectives"].as_object() else {
            quests_without_objectives += 1;
            continue;
        };
        let mut unfinished = 0usize;
        for objective in objectives.values().filter(|row| row["finished"] == false) {
            if let Some(kind) = objective["kind"].as_str() {
                *by_kind.entry(kind).or_default() += 1;
                unfinished += 1;
            }
        }
        if unfinished > 0
            && let Ok(id) = id.parse::<u32>()
        {
            quests_with_work.push((unfinished, id, &quest["title"]));
        }
    }
    quests_with_work.sort_unstable_by_key(|&(count, id, _)| (std::cmp::Reverse(count), id));
    let most_unfinished: Vec<_> = quests_with_work
        .iter()
        .take(20)
        .map(
            |(count, id, title)| json!({"quest_id":id,"title":title,"unfinished_objectives":count}),
        )
        .collect();
    json!({"unfinished_objectives_by_kind":by_kind,
        "quests_without_objective_observations":quests_without_objectives,
        "quests_with_unfinished_objectives":quests_with_work.len(),
        "quests_with_most_unfinished_objectives":most_unfinished})
}

pub fn unfinished_quest_objectives(quest: Option<&Value>) -> Value {
    let Some(objectives) = quest.and_then(|quest| quest["objectives"].as_object()) else {
        return Value::Null;
    };
    Value::Object(
        objectives
            .iter()
            .filter(|(_, objective)| objective["finished"].as_bool() == Some(false))
            .map(|(index, objective)| {
                let remaining = objective["required"]
                    .as_u64()
                    .zip(objective["fulfilled"].as_u64())
                    .map(|(required, fulfilled)| required.saturating_sub(fulfilled));
                (
                    index.clone(),
                    json!({"kind":objective["kind"],"remaining_count":remaining}),
                )
            })
            .collect(),
    )
}

pub fn quest_objective_checklist(quests: &serde_json::Map<String, Value>) -> Value {
    let mut missing = Vec::new();
    let mut entries = BTreeMap::new();
    let mut finished_without_ready = BTreeMap::new();
    let mut unfinished_order = Vec::new();
    for (key, quest) in quests {
        let Ok(id) = key.parse::<u32>() else { continue };
        let readiness = quest_readiness(quest);
        if readiness == QuestReadiness::Failed {
            continue;
        }
        let unfinished = unfinished_quest_objectives(Some(quest));
        match unfinished.as_object() {
            None => missing.push(id),
            Some(rows) if !rows.is_empty() => {
                unfinished_order.push((rows.len(), id));
                entries.insert(
                    id,
                    json!({"readiness":readiness,"unfinished_objectives":unfinished}),
                );
            }
            Some(_) if readiness != QuestReadiness::Ready => {
                if let Some(objectives) = quest["objectives"].as_object().filter(|objectives| {
                    !objectives.is_empty() && objectives.values().all(|row| row["finished"] == true)
                }) {
                    finished_without_ready.insert(
                        id,
                        json!({"readiness":readiness,"finished_objective_count":objectives.len()}),
                    );
                }
            }
            _ => {}
        }
    }
    missing.sort_unstable();
    unfinished_order.sort_unstable();
    let by_unfinished_count: Vec<_> = unfinished_order
        .into_iter()
        .map(|(count, id)| json!({"quest_id":id,"unfinished_objective_count":count}))
        .collect();
    json!({"quests":entries,"quest_ids_without_objective_observations":missing,
        "all_observed_objectives_finished_without_ready_status":finished_without_ready,
        "by_unfinished_objective_count":by_unfinished_count})
}

#[derive(
    Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum QuestReadiness {
    Ready,
    NotReady,
    Failed,
    Unknown,
}

pub fn quest_readiness(quest: &Value) -> QuestReadiness {
    if quest["failed"].as_bool() == Some(true) {
        QuestReadiness::Failed
    } else {
        match quest["ready_for_turn_in"].as_bool() {
            Some(true) => QuestReadiness::Ready,
            Some(false) => QuestReadiness::NotReady,
            None => QuestReadiness::Unknown,
        }
    }
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuestDifficulty {
    Elite,
    SuggestedGroup,
}

impl QuestDifficulty {
    pub fn matches(self, quest: &serde_json::Value) -> Option<bool> {
        match self {
            Self::Elite => quest["elite"].as_bool(),
            Self::SuggestedGroup => quest["suggested_group_size"].as_u64().map(|size| size > 1),
        }
    }
}

pub fn quest_objective_matches(
    quest: &Value,
    kind: Option<&str>,
    finished: Option<bool>,
    minimum_remaining: Option<u32>,
    maximum_remaining: Option<u32>,
) -> Option<bool> {
    let objectives = quest["objectives"].as_object()?;
    let mut unknown = false;
    for objective in objectives.values() {
        if kind.is_some_and(|kind| objective["kind"].as_str() != Some(kind))
            || finished.is_some_and(|finished| objective["finished"].as_bool() != Some(finished))
        {
            continue;
        }
        if minimum_remaining.is_some() || maximum_remaining.is_some() {
            let remaining = objective["required"]
                .as_u64()
                .zip(objective["fulfilled"].as_u64())
                .map(|(required, fulfilled)| required.saturating_sub(fulfilled));
            match remaining {
                Some(remaining)
                    if minimum_remaining.is_none_or(|min| remaining >= u64::from(min))
                        && maximum_remaining.is_none_or(|max| remaining <= u64::from(max)) =>
                {
                    return Some(true);
                }
                Some(_) => {}
                None => unknown = true,
            }
        } else {
            return Some(true);
        }
    }
    if unknown { None } else { Some(false) }
}

pub fn quest_ready_waypoint_groups(
    quests: &serde_json::Map<String, Value>,
    include_wallet_projection: bool,
    wallet: Option<&Value>,
    timed_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Value {
    let mut by_map = BTreeMap::<u64, BTreeSet<u32>>::new();
    let mut without_map = BTreeSet::new();
    let mut grouped_rows = BTreeMap::<Option<u64>, Vec<_>>::new();
    for (key, quest) in quests {
        if quest_readiness(quest) != QuestReadiness::Ready {
            continue;
        }
        let Ok(id) = key.parse::<u32>() else { continue };
        let map = quest["next_waypoint_map_id"].as_u64();
        if include_wallet_projection || timed_at.is_some() {
            grouped_rows.entry(map).or_default().push((key, quest));
        }
        if let Some(map) = map {
            by_map.entry(map).or_default().insert(id);
        } else {
            without_map.insert(id);
        }
    }
    let mut result = json!({"quest_ids_by_waypoint_map":by_map,"quest_ids_without_waypoint_map":without_map,
        "turn_in_location_verified":false});
    let without_map = grouped_rows.remove(&None);
    if include_wallet_projection {
        let by_map: BTreeMap<_, _> = grouped_rows
            .iter()
            .filter_map(|(map, rows)| {
                Some((
                    (*map)?,
                    quest_rows_wallet_projection(rows.iter().copied(), wallet),
                ))
            })
            .collect();
        result["wallet_projection"] = json!({
            "by_waypoint_map":by_map,
            "without_waypoint_map":without_map.as_ref().map(|rows| quest_rows_wallet_projection(rows.iter().copied(), wallet)),
            "groups_evaluated_independently":true
        });
    }
    if let Some(captured_at) = timed_at {
        let by_map: BTreeMap<_, _> = grouped_rows
            .into_iter()
            .filter_map(|(map, rows)| {
                Some((
                    map?,
                    timed_quest_rows_checklist(rows.into_iter(), captured_at),
                ))
            })
            .collect();
        result["timed_checklist"] = json!({
            "by_waypoint_map":by_map,
            "without_waypoint_map":without_map.map(|rows| timed_quest_rows_checklist(rows.into_iter(), captured_at))
        });
    }
    result
}

pub fn quest_checklist(quests: &serde_json::Map<String, Value>) -> Value {
    let (mut ready, mut not_ready, mut failed, mut unknown) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (key, quest) in quests {
        let Ok(id) = key.parse::<u32>() else {
            continue;
        };
        match quest_readiness(quest) {
            QuestReadiness::Ready => ready.push(id),
            QuestReadiness::NotReady => not_ready.push(id),
            QuestReadiness::Failed => failed.push(id),
            QuestReadiness::Unknown => unknown.push(id),
        }
    }
    for ids in [&mut ready, &mut not_ready, &mut failed, &mut unknown] {
        ids.sort_unstable();
    }
    json!({"ready_for_turn_in":ready,"turn_in_not_ready":not_ready,"failed":failed,"turn_in_unknown":unknown})
}

pub fn timed_quest_checklist(
    quests: &serde_json::Map<String, Value>,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> Value {
    timed_quest_rows_checklist(quests.iter(), captured_at)
}

fn timed_quest_rows_checklist<'a>(
    quests: impl ExactSizeIterator<Item = (&'a String, &'a Value)>,
    captured_at: chrono::DateTime<chrono::Utc>,
) -> Value {
    let quest_count = quests.len();
    let mut timed: Vec<_> = quests
        .filter_map(|(id, quest)| {
            Some((
                quest["timer"]["remaining_seconds"].as_i64()?,
                id.parse::<u32>().ok()?,
                quest,
            ))
        })
        .collect();
    timed.sort_unstable_by_key(|(seconds, id, _)| (*seconds, *id));
    let entry = |(seconds, id, quest): &(i64, u32, &Value)| {
        let expiry = crate::saved_resets::projected_reset_at(*seconds, captured_at);
        json!({"quest_id":id,"remaining_seconds_at_capture":seconds,"projected_expiry_at":expiry,
            "readiness_at_capture":quest_readiness(quest)})
    };
    let entries: Vec<_> = timed.iter().take(20).map(entry).collect();
    let mut ready = timed
        .iter()
        .filter(|(_, _, quest)| quest_readiness(quest) == QuestReadiness::Ready);
    let first_ready = ready.next().map(entry);
    let ready_count = usize::from(first_ready.is_some()) + ready.count();
    json!({"entries":entries,"recorded_timer_count":timed.len(),
        "ready_for_turn_in_at_capture_count":ready_count,"first_ready_for_turn_in_at_capture":first_ready,
        "quest_count_without_recorded_timer":quest_count-timed.len(),
        "truncated":timed.len()>entries.len(),"current_status_verified":false})
}

fn quest_waypoint_maps(quests: &serde_json::Map<String, Value>) -> Value {
    let mut maps = BTreeMap::<u64, Vec<u32>>::new();
    let mut unknown = Vec::new();
    for (key, quest) in quests {
        let Ok(id) = key.parse::<u32>() else { continue };
        if let Some(map) = quest["next_waypoint_map_id"].as_u64() {
            maps.entry(map).or_default().push(id);
        } else {
            unknown.push(id);
        }
    }
    for ids in maps.values_mut() {
        ids.sort_unstable();
    }
    unknown.sort_unstable();
    json!({"by_map_id":maps,"waypoint_map_unknown":unknown})
}

pub(crate) fn repair_status(s: &Snapshot) -> Value {
    let alerts = s
        .sections
        .get("durability_alerts")
        .and_then(Value::as_object);
    let quote = s
        .sections
        .get("repair_quote")
        .and_then(|quote| quote["copper"].as_u64());
    let wallet = s
        .sections
        .get("wallet")
        .and_then(|wallet| wallet["copper"].as_u64());
    let budget = quote.zip(wallet);
    json!({
        "low_slots":alerts.map(|slots| slots.values().filter(|status| status.as_str()==Some("low")).count()),
        "broken_slots":alerts.map(|slots| slots.values().filter(|status| status.as_str()==Some("broken")).count()),
        "repair_all_quote_copper":quote,
        "wallet_shortfall_copper":budget.map(|(cost, balance)| cost.saturating_sub(balance)),
        "wallet_covers_saved_repair_quote":budget.map(|(cost, balance)| cost <= balance),
        "wallet_remaining_after_saved_repair_quote_copper":budget
            .and_then(|(cost, balance)| balance.checked_sub(cost)),
    })
}

pub fn repair_reserve_projection(snapshot: &Snapshot, reserve: u64) -> Value {
    let quote = snapshot
        .sections
        .get("repair_quote")
        .and_then(|row| row["copper"].as_u64());
    let wallet = snapshot
        .sections
        .get("wallet")
        .and_then(|row| row["copper"].as_u64());
    let reserve_covered = wallet.map(|balance| balance >= reserve);
    let budget = quote.zip(wallet);
    let remaining =
        budget.and_then(|(quote, balance)| balance.checked_sub(quote)?.checked_sub(reserve));
    json!({"reserve_copper":reserve,"reserve_covered":reserve_covered,
        "affordable_with_reserve":if reserve_covered == Some(false) { Some(false) }
            else { budget.map(|_| remaining.is_some()) },
        "shortfall_copper":budget.and_then(|(cost, balance)| reserved_balance_shortfall(cost, balance, reserve)),
        "reserve_shortfall_copper":wallet.map(|balance| reserve.saturating_sub(balance)),
        "wallet_remaining_after_quote_and_reserve_copper":remaining,
        "repair_eligibility_verified":false,"live":false})
}

pub fn repair_equipment_condition(snapshot: &Snapshot) -> Option<Value> {
    let entries = snapshot.sections.get("equipment_durability")?["entries"].as_object()?;
    let equipment = snapshot
        .sections
        .get("equipment")
        .and_then(Value::as_object);
    let equipped_slots_without_durability_reading = equipment.map(|slots| {
        slots
            .keys()
            .filter(|slot| !entries.contains_key(*slot))
            .filter_map(|slot| slot.parse::<u32>().ok())
            .collect::<BTreeSet<_>>()
    });
    let mut damaged = BTreeMap::new();
    let mut missing_points = 0;
    for (slot, row) in entries {
        let current = row["current"].as_u64()?;
        let maximum = row["maximum"].as_u64()?;
        let missing = maximum.checked_sub(current)?;
        missing_points += missing;
        if missing > 0 {
            let item_id = equipment
                .and_then(|slots| slots.get(slot))
                .and_then(|item| item["item_id"].as_u64());
            let slot = slot.parse::<u32>().ok()?;
            damaged.insert(
                slot,
                json!({"slot":slot,"current":current,"maximum":maximum,
                "missing_durability_points":missing,"broken":current == 0,
                "observed_equipped_item_id":item_id}),
            );
        }
    }
    Some(
        json!({"scope":"readable_equipped_slots_only","observed_slot_count":entries.len(),
        "damaged_slot_count":damaged.len(),"damaged_slots":damaged.into_values().collect::<Vec<_>>(),
        "total_missing_observed_durability_points":missing_points,
        "equipped_slots_without_durability_reading":equipped_slots_without_durability_reading,
        "quote_item_scope_verified":false,"per_slot_repair_price_verified":false,"live":false}),
    )
}

pub fn summary(s: &Snapshot) -> Value {
    let occupied = s
        .sections
        .get("equipment")
        .and_then(Value::as_object)
        .map(|v| v.len());
    let unspent = s
        .sections
        .get("talents")
        .and_then(|v| v.get("has_unspent"))
        .and_then(Value::as_bool);
    let quests = s.sections.get("quests").and_then(Value::as_object);
    let timed_quests = quests.filter(|_| s.client.family == crate::Family::Forever).map(|quests| {
        let mut rows = quests.iter().filter_map(|(id, quest)| {
            let timer = quest["timer"].as_object()?;
            Some((timer.get("remaining_seconds")?.as_u64()?, id.parse::<u32>().ok()?,
                timer.get("total_seconds")?.as_u64()?))
        }).collect::<Vec<_>>();
        rows.sort_unstable();
        rows.into_iter().map(|(remaining_seconds, quest_id, total_seconds)| {
            let estimated_end = i64::try_from(remaining_seconds).ok()
                .and_then(chrono::TimeDelta::try_seconds)
                .and_then(|remaining| s.captured_at.checked_add_signed(remaining));
            json!({"quest_id":quest_id,"remaining_seconds":remaining_seconds,"total_seconds":total_seconds,
                "estimated_end_at":estimated_end})
        }).collect::<Vec<_>>()
    });
    let group_quests = quests.filter(|_| s.client.family == crate::Family::Forever).map(|quests| {
        let mut groups = Vec::new();
        let mut elite = Vec::new();
        for (key, quest) in quests {
            let Ok(id) = key.parse::<u32>() else { continue };
            if let Some(size) = quest["suggested_group_size"].as_u64().filter(|size| *size > 1) {
                groups.push((id, size));
            }
            if quest["elite"].as_bool() == Some(true) { elite.push(id); }
        }
        groups.sort_unstable();
        elite.sort_unstable();
        json!({"suggested_groups":groups.into_iter().map(|(quest_id,size)| json!({"quest_id":quest_id,"suggested_group_size":size})).collect::<Vec<_>>(),"elite_quest_ids":elite})
    });
    let is_forever = s.client.family == crate::Family::Forever;
    let wallet = s
        .sections
        .get("wallet")
        .and_then(|wallet| wallet["copper"].as_u64());
    let repair_status = is_forever.then(|| repair_status(s));
    let waypoint_maps = quests.filter(|_| is_forever).map(quest_waypoint_maps);
    let captured_map = s
        .sections
        .get("stats")
        .filter(|_| is_forever)
        .and_then(|stats| stats["map_id"].as_u64());
    let quests_on_captured_map =
        captured_map
            .zip(waypoint_maps.as_ref())
            .map(|(map, waypoints)| {
                waypoints["by_map_id"]
                    .get(map.to_string())
                    .cloned()
                    .unwrap_or_else(|| json!([]))
            });
    let quest_money = quests
        .filter(|_| s.client.family == crate::Family::Forever)
        .map(|quests| {
            let mut unknown = 0;
            let mut costs = BTreeMap::new();
            for (id, quest) in quests {
                match quest["required_copper"].as_u64() {
                    Some(0) => (),
                    Some(cost) => {
                        costs.insert(
                            id,
                            json!({"required_copper":cost,
                        "shortfall_copper":wallet.map(|balance| cost.saturating_sub(balance))}),
                        );
                    }
                    None => unknown += 1,
                }
            }
            json!({"quests_with_cost":costs,"cost_unknown_count":unknown})
        });
    let quest_turn_in = quests.map(|quests| {
        let (mut ready, mut known) = (0, 0);
        for quest in quests.values() {
            if let Some(value) = quest["ready_for_turn_in"].as_bool() {
                known += 1;
                ready += usize::from(value);
            }
        }
        json!({"ready_observed":ready,"status_observed":known,"status_unknown":quests.len()-known})
    });
    let mut summary = json!({"client":s.client,"character":s.character,"captured_at":s.captured_at,"live":false,
      "experience_progress":s.sections.get("experience").map(|xp|
          json!({"observed":xp,"xp_to_next_level":xp_to_next_level(xp)})),
      "spellbook_slots_observed":s.sections.get("spellbook").and_then(|book|book["entries"].as_object()).map(|slots|slots.len()),
      "equipped_slots_observed":occupied,"carried_item_slots_observed":container_slots(s,"bags"),
      "carried_bag_space":carried_bag_space(s),
      "instance_context":s.sections.get("instance_context"),
      "character_bank_item_slots_observed":container_slots(s,"character_bank"),
      "account_bank_item_slots_observed":container_slots(s,"account_bank"),
      "reputation_factions_observed":s.sections.get("reputation").and_then(|v|v["entries"].as_object()).map(|v|v.len()),
      "currency_balances_observed":s.sections.get("currency_balances").and_then(|v|v["entries"].as_object()).map(|v|v.len()),
      "tracked_achievements_observed":s.sections.get("tracked_achievements").and_then(|v|v["entries"].as_object()).map(|v|v.len()),
      "quest_turn_in":quest_turn_in,
      "quest_checklist":quests.map(quest_checklist),
      "quest_objective_workload":quests.filter(|_| is_forever).map(quest_objective_workload),
      "quest_waypoint_maps":waypoint_maps,
      "captured_map_id":captured_map,
      "captured_map_name":captured_map.and_then(|_| s.sections["stats"]["map_name"].as_str()),
      "captured_map_parent_id":captured_map.and_then(|_| s.sections["stats"]["map_parent_id"].as_u64()),
      "captured_map_min_level":captured_map.and_then(|_| s.sections["stats"]["map_min_level"].as_u64()),
      "captured_map_max_level":captured_map.and_then(|_| s.sections["stats"]["map_max_level"].as_u64()),
      "new_mail_indicator":s.sections.get("stats").filter(|_| is_forever).and_then(|stats| stats["new_mail_indicator"].as_bool()),
      "hearthstone_bind_name":s.sections.get("stats").filter(|_| is_forever).and_then(|stats| stats["hearthstone_bind_name"].as_str()),
      "hearthstone_item_id":s.sections.get("stats").filter(|_| is_forever).and_then(|stats| stats["hearthstone_item_id"].as_u64()),
      "hearthstone_cooldown_seconds":s.sections.get("stats").filter(|_| is_forever).and_then(|stats| stats["hearthstone_cooldown_seconds"].as_u64()),
      "quests_with_waypoint_on_captured_map":quests_on_captured_map,
      "quest_money":quest_money,
      "repair_status":repair_status,
      "pet_food_locations":is_forever.then(|| pet_food_locations(s)).flatten(),
      "observed_quest_timers":timed_quests,
      "observed_quest_difficulty":group_quests,
      "carried_item_review":carried_items(s),
      "unspent_talent_points_reported":unspent,"available_sections":s.sections.keys().collect::<Vec<_>>(),
      "unavailable":s.unavailable,"limitations":["Counts describe exported entries, not item quality or optimal gear","No bank/mail inventory inferred","No enchant, socket or talent legality inferred from missing fields"]});
    if is_forever {
        let mut counts = BTreeMap::from([
            ("observed", 0usize),
            ("reported_unavailable", 0),
            ("not_recorded", 0),
        ]);
        let sections: BTreeMap<_, _> = crate::SNAPSHOT_SECTIONS
            .iter()
            .map(|name| {
                let status = s.section_capture_status(name);
                *counts.get_mut(status).unwrap() += 1;
                let scope = s
                    .sections
                    .get(*name)
                    .and_then(|section| section["coverage"].as_str());
                (*name, json!({"status":status,"reported_scope":scope}))
            })
            .collect();
        summary["capture_coverage"] = json!({
            "known_section_count":crate::SNAPSHOT_SECTIONS.len(),
            "section_status_counts":counts,"sections":sections,
            "complete_game_coverage_verified":false
        });
    }
    let mounts = s
        .sections
        .get("mount_collection")
        .and_then(|collection| collection["entries"].as_object());
    summary["collected_mounts_observed"] = json!(mounts.map(|entries| entries.len()));
    if is_forever {
        summary["mount_observed_flag_counts"] = json!(mounts.map(mount_flag_counts));
    }
    summary["active_pet_observation"] = s
        .sections
        .get("pet_care")
        .map(|pet| {
            let mut observed =
                json!({"present":pet["present"],"family_id":pet["family_id"],"level":pet["level"]});
            if let Some(points) = pet.get("training_points") {
                observed["training_points"] = points.clone();
                observed["training_points_balance"] = json!(pet_training_points_balance(pet));
            }
            observed
        })
        .unwrap_or(Value::Null);
    summary["pet_collection_view_counts"] = s
        .sections
        .get("pet_collection_view")
        .map(|view| {
            let rows = view["entries"].as_object().unwrap();
            let (mut owned_rows, mut unowned_rows) = (0u64, 0u64);
            let (mut owned_only, mut unowned_only, mut mixed) = (0, 0, 0);
            for row in rows.values() {
                let owned = row["owned_rows"].as_u64().unwrap();
                let unowned = row["unowned_rows"].as_u64().unwrap();
                owned_rows += owned;
                unowned_rows += unowned;
                match (owned > 0, unowned > 0) {
                    (true, true) => mixed += 1,
                    (true, false) => owned_only += 1,
                    (false, true) => unowned_only += 1,
                    (false, false) => {}
                }
            }
            json!({"displayed_rows":view["displayed_rows"],"species_observed":rows.len(),
            "owned_rows_observed":owned_rows,"unowned_rows_observed":unowned_rows,
            "species_with_owned_rows_only":owned_only,"species_with_unowned_rows_only":unowned_only,
            "species_with_both_row_types":mixed})
        })
        .unwrap_or(Value::Null);
    summary["mail_attachment_counts"] = s.sections.get("mail_attachments").map(|mail| {
        let rows: Vec<_> = mail["messages"].as_object().unwrap().values()
            .flat_map(|attachments| attachments.as_object().unwrap().values()).collect();
        json!({"loaded_messages":mail["loaded_messages"],"total_messages":mail["total_messages"],
            "unloaded_messages":unloaded_mail_messages(mail),
            "observed_slots":rows.len(),"item_slots":rows.iter().filter(|row| row["kind"]=="item").count(),
            "currency_slots":rows.iter().filter(|row| row["kind"]=="currency").count(),
            "unknown_slots":rows.iter().filter(|row| row["kind"]=="unknown").count()})
    }).unwrap_or(Value::Null);
    if let Some(talents) = s
        .sections
        .get("talents")
        .filter(|talents| talents.get("observed_tree_ids").is_some())
    {
        let nodes = talents["nodes"].as_object().unwrap();
        let defined = nodes
            .values()
            .filter(|node| node.get("definition_id").is_some())
            .count();
        summary["talent_definition_observation"] = json!({"basis":"reported_active_config_trait_observations",
            "observed_tree_count":talents["observed_tree_ids"].as_object().unwrap().len(),
            "observed_node_count":talents["observed_node_count"],"active_entry_nodes":nodes.len(),
            "nodes_with_definition_observation":defined,"nodes_without_definition_observation":nodes.len()-defined,
            "staged_changes_observed":talents.get("staged_changes_observed"),
            "allocation_match_verified":false,"simulation_mapping_verified":false,"game_execution":false});
    }
    summary["bag_item_cooldown_observation"] = s.sections.get("bag_item_cooldowns").map(|section| {
        let rows: Vec<_> = section["entries"].as_object().unwrap().values()
            .flat_map(|bag| bag.as_object().unwrap().values()).collect();
        let reported = rows.iter().filter(|row| row["status"] == "reported").count();
        json!({"coverage":section["coverage"],"captured_item_locations":rows.len(),
            "reported_item_locations":reported,"unavailable_item_locations":rows.len()-reported,
            "disabled_item_locations":rows.iter().filter(|row| row["enabled"] == false).count(),
            "positive_timer_item_locations":rows.iter().filter(|row| row["remaining_seconds"].as_u64().is_some_and(|n| n>0)).count(),
            "current_usability_verified":false,"game_execution":false})
    }).unwrap_or(Value::Null);
    summary["player_life_state_observation"] = s.sections.get("player_life_state")
        .map(crate::player_life_state::observation).unwrap_or(Value::Null);
    summary["quest_giver_reference_observation"] = s.sections.get("quest_giver_references")
        .map(crate::quest_references::observation).unwrap_or(Value::Null);
    summary["battlefield_queue_observation"] = s.sections.get("battlefield_queue_slots").map(|section| {
        let mut slots = BTreeMap::from([("none",Vec::<u32>::new()),("queued",Vec::new()),
            ("confirm",Vec::new()),("active",Vec::new()),("locked",Vec::new())]);
        for (slot,row) in section["entries"].as_object().unwrap() {
            slots.get_mut(row["reported_status"].as_str().unwrap()).unwrap().push(slot.parse().unwrap());
        }
        for rows in slots.values_mut() { rows.sort_unstable(); }
        let counts: BTreeMap<_,_> = slots.iter().map(|(status,rows)|(*status,rows.len())).collect();
        json!({"coverage":section["coverage"],"reported_slot_count":section["reported_slot_count"],
            "reported_status_counts":counts,"reported_status_slots":slots,
            "reported_non_none_slot_count":section["reported_slot_count"].as_u64().unwrap()-slots["none"].len() as u64,
            "current_queue_membership_verified":false,"complete_queue_coverage_verified":false,
            "acceptance_eligibility_verified":false,"queue_identity_verified":false,
            "game_execution":false,"live":false})
    }).unwrap_or(Value::Null);
    summary["group_context_observation"] = s.sections.get("group_context").map(|section| {
        json!({"coverage":section["coverage"],"categories":section["categories"],
            "current_group_verified":false,"group_identity_verified":false,
            "complete_roster_verified":false,"social_action_eligibility_verified":false,"game_execution":false})
    }).unwrap_or(Value::Null);
    summary["player_pvp_progression_observation"] = s
        .sections
        .get("player_pvp_progression")
        .map(player_pvp_progression_observation)
        .unwrap_or(Value::Null);
    summary["player_trade_offer_observation"] = s
        .sections
        .get("player_trade_offer")
        .map(|section| {
            let (items, quantities) = observed_trade_offer_quantities(section);
            json!({"coverage":section["coverage"],"normal_slots":6,"item_slots":items,
            "unreadable_or_empty_slots":6-items,"observed_offered_item_quantities":quantities,
            "counts_toward_owned_inventory":false,"trade_completion_verified":false,
            "current_offer_verified":false,"game_execution":false})
        })
        .unwrap_or(Value::Null);
    if is_forever {
        summary["timed_quest_checklist"] =
            json!(quests.map(|quests| timed_quest_checklist(quests, s.captured_at)));
    }
    summary["loot_window_observation"] = s
        .sections
        .get("loot_window")
        .map(|section| {
            let rows = section["entries"].as_object().unwrap();
            let mut quantities = BTreeMap::<u32, u64>::new();
            let mut item_slots = 0;
            for row in rows.values().filter(|row| row["status"] == "item") {
                item_slots += 1;
                *quantities
                    .entry(row["item_id"].as_u64().unwrap() as u32)
                    .or_default() += row["quantity"].as_u64().unwrap();
            }
            let mut report=json!({"coverage":section["coverage"],"slot_count":section["slot_count"],
            "item_slots":item_slots,"unreadable_or_non_item_slots":rows.len()-item_slots,
            "observed_item_quantities":quantities,"counts_toward_owned_inventory":false,
            "ownership_verified":false,"lootability_verified":false,"game_execution":false});
            let currency_slots=rows.values().filter(|row| row["status"]=="reported_currency").count();
            let money_slots=rows.values().filter(|row| row["status"]=="money").count();
            if currency_slots+money_slots>0 {
                let mut currencies=BTreeMap::<u32,u64>::new();
                for row in rows.values().filter(|row| row["status"]=="reported_currency") {
                    *currencies.entry(row["reported_currency_id"].as_u64().unwrap() as u32).or_default()
                        +=row["reported_quantity"].as_u64().unwrap();
                }
                report["typed_non_item_observation"]=json!({"reported_currency_slots":currency_slots,
                    "money_slots":money_slots,"unreadable_slots":rows.len()-item_slots-currency_slots-money_slots,
                    "reported_currency_quantities":currencies,"currency_basis":"raw_slot_info_before_container_conversion",
                    "currency_rewards_verified":false,"money_amounts_verified":false,
                    "counts_toward_owned_balances":false});
            }
            report
        })
        .unwrap_or(Value::Null);
    summary["profession_recipe_view_counts"] = s.sections.get("profession_recipe_view").map(|view| {
        let rows = view["entries"].as_object().unwrap();
        json!({"observed":rows.len(),
            "learned":rows.values().filter(|row| row["learned"] == true).count(),
            "unlearned":rows.values().filter(|row| row["learned"] == false).count(),
            "learning_unknown":rows.values().filter(|row| row["learned"].as_bool().is_none()).count()})
    }).unwrap_or(Value::Null);
    summary["profession_skill_up_candidates"] = s.sections.get("profession_recipe_view").map(|view| {
        let rows=view["entries"].as_object().unwrap();
        let mut candidates:Vec<_>=rows.iter().filter(|(_,row)|row["learned"]==true && row["can_skill_up"]==true).collect();
        candidates.sort_by_key(|(id,_)|id.parse::<u32>().unwrap());
        let mut report = json!({"coverage":view["coverage"],"profession_id":view["profession_id"],
            "candidate_count":candidates.len(),"truncated":candidates.len()>50,
            "learned_rows_with_unknown_skill_up_flag":rows.values().filter(|row|row["learned"]==true && row["can_skill_up"].as_bool().is_none()).count(),
            "eligibility_verified":false,
            "recipes":candidates.into_iter().take(50).map(|(id,row)|json!({"recipe_id":id.parse::<u32>().unwrap(),
                "relative_difficulty":row["relative_difficulty"],"skill_ups_reported":row["skill_ups_reported"]})).collect::<Vec<_>>()});
        if let Some(rank) = view.get("rank_observation") {
            report["rank_observation"] = rank.clone();
        }
        report
    }).unwrap_or(Value::Null);
    summary["merchant_buyback_observations"] = s.sections.get("merchant_buyback").map(|section| {
        let rows=section["entries"].as_object().unwrap();
        let prices:Vec<_>=rows.values().filter_map(|row|row["buyback_price_copper"].as_u64()).collect();
        let subtotal:u64=prices.iter().sum();
        let ids:BTreeSet<_>=rows.values().map(|row|row["item_id"].as_u64().unwrap()).collect();
        json!({"coverage":section["coverage"],"listed_item_locations":rows.len(),
            "listed_units":rows.values().map(|row|row["count"].as_u64().unwrap()).sum::<u64>(),
            "item_ids":ids,"priced_entries":prices.len(),"unknown_price_entries":rows.len()-prices.len(),
            "known_price_subtotal_copper":subtotal,"total_listed_quote_copper":if prices.len()==rows.len() {Some(subtotal)} else {None},
            "included_in_owned_inventory":false,"buyback_eligibility_verified":false})
    }).unwrap_or(Value::Null);
    summary["equipped_bag_item_observations"] = s
        .sections
        .get("bag_capacity")
        .map(|section| {
            let mut rows: Vec<_> = section
                .as_object()
                .unwrap()
                .iter()
                .filter_map(|(bag, row)| {
                    row.get("equipped_item").map(|item| {
                        json!({"bag":bag.parse::<u32>().unwrap(),
                "item_id":item["item_id"],"inventory_slot":item["inventory_slot"]})
                    })
                })
                .collect();
            rows.sort_unstable_by_key(|row| row["bag"].as_u64().unwrap());
            json!({"observed_identity_count":rows.len(),"entries":rows,
            "complete_equipped_bag_identity_verified":false,"quiver_bonus_verified":false,
            "credited_as_material_stock":false,"game_execution":false})
        })
        .unwrap_or(Value::Null);
    summary["gossip_option_observations"] = s.sections.get("gossip_options").map(|section| {
        let rows=section["entries"].as_object().unwrap();
        json!({"coverage":section["coverage"],"observed_option_count":rows.len(),
            "options_with_reported_id":rows.values().filter(|row|row.get("option_id").is_some()).count(),
            "npc_identity_verified":false,"selection_eligibility_verified":false,
            "complete_dialogue_coverage_verified":false,"game_execution":false})
    }).unwrap_or(Value::Null);
    summary["gossip_quest_observations"] = s.sections.get("gossip_quests").map(|section| {
        let quest_log = s.sections.get("quests").and_then(Value::as_object);
        let describe=|rows:Option<&Value>|rows.map(|rows| {
            let rows=rows.as_object().unwrap();
            let mut ids:Vec<_>=rows.keys().map(|id|id.parse::<u32>().unwrap()).collect();ids.sort_unstable();
            let complete:Vec<_>=ids.iter().filter(|id|rows[&id.to_string()]["is_complete"]==true).copied().collect();
            let mut report = json!({"observed_quest_count":ids.len(),"quest_ids":ids.iter().take(50).collect::<Vec<_>>(),"quest_ids_truncated":ids.len()>50,
                "reported_complete_quest_count":complete.len(),"reported_complete_quest_ids":complete.iter().take(50).collect::<Vec<_>>(),
                "reported_complete_truncated":complete.len()>50,
                "quests_with_unknown_completion":rows.values().filter(|row|row.get("is_complete").is_none()).count()});
            if let Some(log) = quest_log {
                let matched: Vec<_> = ids.iter().filter_map(|id| {
                    let key = id.to_string();
                    let readiness = quest_readiness(log.get(&key)?);
                    Some(json!({"quest_id":id,"quest_log_readiness":readiness,
                        "panel_is_complete":rows[&key]["is_complete"]}))
                }).collect();
                report["quest_log_matches"] = json!({
                    "matched_quest_count":matched.len(),
                    "rows":matched.iter().take(50).collect::<Vec<_>>(),
                    "truncated":matched.len()>50,
                });
            }
            report
        });
        json!({"coverage":section["coverage"],"available":describe(section.get("available")),"active":describe(section.get("active")),
            "acceptance_eligibility_verified":false,"turn_in_eligibility_verified":false})
    }).unwrap_or(Value::Null);
    summary["player_aura_observations"] = s.sections.get("player_auras").map(|section| {
        let describe=|group:Option<&Value>|group.map(|group| {
            let rows=group["entries"].as_object().unwrap();
            let ids:BTreeSet<_>=rows.values().map(|row|row["spell_id"].as_u64().unwrap()).collect();
            json!({"observed_rows":rows.len(),"distinct_spell_ids":ids.len(),
                "spell_ids":ids.iter().take(50).collect::<Vec<_>>(),"spell_ids_truncated":ids.len()>50,
                "rows_with_unknown_applications":rows.values().filter(|row|row.get("applications").is_none()).count()})
        });
        json!({"coverage":section["coverage"],"helpful":describe(section["groups"].get("helpful")),
            "harmful":describe(section["groups"].get("harmful")),"effectiveness_verified":false,"cast_eligibility_verified":false})
    }).unwrap_or(Value::Null);
    summary["profession_recipe_material_observations"] = s.sections.get("profession_recipe_materials").map(|section| {
        let rows=section["entries"].as_object().unwrap();
        let count=|status: &str| rows.values().filter(|row|row["status"]==status).count();
        json!({"coverage":section["coverage"],"profession_id":section["profession_id"],
            "reported_spell_count":rows.len(),"fixed_item_recipe_count":count("fixed_items"),
            "unavailable_recipe_count":count("unavailable"),"unsupported_recipe_count":count("unsupported_required_slots"),
            "budget_exhausted_recipe_count":count("budget_exhausted"),
            "complete_recipe_knowledge_verified":false,"craftability_verified":false})
    }).unwrap_or(Value::Null);
    summary["profession_spell_observations"] = s.sections.get("profession_spells").map(|section| {
        let mut ids: Vec<_>=section["entries"].as_object().unwrap().keys().map(|key|key.parse::<u32>().unwrap()).collect();
        ids.sort_unstable();
        json!({"coverage":section["coverage"],"profession_id":section["profession_id"],
            "reported_spell_count":ids.len(),"spell_ids":ids.iter().take(50).collect::<Vec<_>>(),
            "truncated":ids.len()>50,"complete_recipe_knowledge_verified":false,"craftability_verified":false})
    }).unwrap_or(Value::Null);
    summary["player_resource_observations"] = s.sections.get("player_resources").map(|resources| {
        let describe = |row: Option<&Value>| row.map(|row| {
            let current=row["current"].as_u64().unwrap();
            let maximum=row["maximum"].as_u64().unwrap();
            let mut value=json!({"current":current,"maximum":maximum,"missing":maximum-current,
                "percent":if maximum>0 {Some(current as f64*100.0/maximum as f64)} else {None}});
            if let Some(kind)=row.get("power_type") {value["power_type"]=kind.clone();value["units"]=row["units"].clone();}
            value
        });
        json!({"coverage":resources["coverage"],"health":describe(resources.get("health")),
            "primary_power":describe(resources.get("primary_power")),"readiness_verified":false})
    }).unwrap_or(Value::Null);
    summary["pet_action_bar_observations"] = s.sections.get("pet_action_bar").map(|bar| {
        let rows=bar["entries"].as_object().unwrap();
        let mut slots:Vec<_>=rows.iter().map(|(slot,row)|(slot.parse::<u32>().unwrap(),row)).collect();slots.sort_by_key(|(slot,_)|*slot);
        json!({"coverage":bar["coverage"],"pet_present":bar["pet_present"],"has_action_bar":bar["has_action_bar"],
            "observed_slots":rows.len(),"unobserved_slots":if bar["has_action_bar"]==true {Some(10-rows.len())} else {None},
            "command_slots":slots.iter().filter(|(_,row)|row["kind"]=="command").map(|(slot,_)|slot).collect::<Vec<_>>(),
            "spell_slots":slots.iter().filter(|(_,row)|row["kind"]=="spell").map(|(slot,_)|slot).collect::<Vec<_>>(),
            "reported_active_slots":slots.iter().filter(|(_,row)|row["is_active"]==true).map(|(slot,_)|slot).collect::<Vec<_>>(),
            "autocast_enabled_spell_slots":slots.iter().filter(|(_,row)|row["kind"]=="spell" && row["autocast_enabled"]==true).map(|(slot,_)|slot).collect::<Vec<_>>(),
            "unknown_autocast_spell_slots":slots.iter().filter(|(_,row)|row["kind"]=="spell" && row["autocast_enabled"].as_bool().is_none()).map(|(slot,_)|slot).collect::<Vec<_>>(),
            "cast_eligibility_verified":false})
    }).unwrap_or(Value::Null);
    summary["spellbook_usability_observations"] = s.sections.get("spellbook").map(|book| {
        let rows=book["entries"].as_object().unwrap();
        let known=rows.values().filter(|row|row.get("usability").is_some()).count();
        json!({"coverage":book["coverage"],"observed_rows":rows.len(),"known_usability_rows":known,
            "unknown_usability_rows":rows.len()-known,
            "usable_rows":rows.values().filter(|row|row["usability"]["usable"]==true).count(),
            "unusable_rows":rows.values().filter(|row|row["usability"]["usable"]==false).count(),
            "insufficient_power_rows":rows.values().filter(|row|row["usability"]["insufficient_power"]==true).count(),
            "cast_eligibility_verified":false})
    }).unwrap_or(Value::Null);
    summary["spellbook_resource_observations"] = s.sections.get("spellbook").map(|book| {
        let rows = book["entries"].as_object().unwrap();
        let charges = rows.values().filter(|row| row.get("charge_count").is_some()).count();
        let casts = rows.values().filter(|row| row.get("reported_cast_count").is_some()).count();
        let cooldowns = rows.values().filter(|row| row.get("cooldown_state").is_some()).count();
        json!({"coverage":book["coverage"],"observed_rows":rows.len(),
            "known_charge_rows":charges,"unknown_charge_rows":rows.len()-charges,
            "positive_charge_rows":rows.values().filter(|row|row["charge_count"]["current"].as_u64().is_some_and(|n|n>0)).count(),
            "zero_charge_rows":rows.values().filter(|row|row["charge_count"]["current"].as_u64()==Some(0)).count(),
            "known_reported_cast_count_rows":casts,"unknown_reported_cast_count_rows":rows.len()-casts,
            "positive_reported_cast_count_rows":rows.values().filter(|row|row["reported_cast_count"].as_u64().is_some_and(|n|n>0)).count(),
            "zero_reported_cast_count_rows":rows.values().filter(|row|row["reported_cast_count"].as_u64()==Some(0)).count(),
            "known_cooldown_state_rows":cooldowns,"unknown_cooldown_state_rows":rows.len()-cooldowns,
            "active_cooldown_rows":rows.values().filter(|row|row["cooldown_state"]["active"]==true).count(),
            "enabled_cooldown_rows":rows.values().filter(|row|row["cooldown_state"]["enabled"]==true).count(),
            "cast_eligibility_verified":false,"live":false})
    }).unwrap_or(Value::Null);
    summary["captured_vendor_price_values"] = json!(["equipment","bags","character_bank","account_bank"].into_iter().map(|storage| {
        let value=s.sections.get(storage).map(|section| {
            let rows:Vec<_>=if storage=="equipment" {section.as_object().unwrap().values().collect()}
                else {section.as_object().unwrap().values().flat_map(|container|container.as_object().unwrap().values()).collect()};
            let mut priced=0usize;let mut observed_units=0u64;let mut priced_units=0u64;let mut subtotal=Some(0u64);
            for row in &rows {
                let count=if storage=="equipment" {1} else {row["count"].as_u64().unwrap()};
                observed_units+=count;
                if let Some(price)=row["sell_price_copper"].as_u64() {
                    priced+=1;priced_units+=count;
                    subtotal=subtotal.and_then(|sum|price.checked_mul(count).and_then(|value|sum.checked_add(value)));
                }
            }
            json!({"observed_locations":rows.len(),"priced_locations":priced,"unpriced_locations":rows.len()-priced,
                "observed_units":observed_units,"priced_units":priced_units,"unpriced_units":observed_units-priced_units,
                "known_price_subtotal_copper":subtotal,"observed_total_copper":if priced==rows.len() {subtotal} else {None},
                "arithmetic_overflow":subtotal.is_none(),"sale_eligibility_verified":false})
        });
        (storage,value)
    }).collect::<std::collections::BTreeMap<_,_>>());
    summary["weapon_enchant_observations"] = s.sections.get("weapon_enchants").map(|section| {
        let hands=section["hands"].as_object().unwrap();
        let rows:Vec<_>=hands.iter().map(|(hand,value)| {
            let entries=value["entries"].as_object().unwrap();
            let mut active:Vec<_>=entries.iter().filter(|(_,row)|row["has_enchant"]==true).collect();
            active.sort_by_key(|(index,_)|index.parse::<u32>().unwrap());
            (hand,json!({"observed_rows":entries.len(),"active_enchants":active.into_iter().map(|(index,row)|json!({"row":index.parse::<u32>().unwrap(),
                "enchant_type":row["enchant_type"],"enchant_id":row["enchant_id"],"charges":row["charges"],"remaining_milliseconds":row["remaining_milliseconds"]})).collect::<Vec<_>>()}))
        }).collect();
        json!({"coverage":section["coverage"],"unobserved_hands":(["main_hand","off_hand","ranged"].into_iter().filter(|hand|!hands.contains_key(*hand)).collect::<Vec<_>>()),
            "hands":rows.into_iter().collect::<std::collections::BTreeMap<_,_>>(),"effect_on_performance":"unverified"})
    }).unwrap_or(Value::Null);
    summary["equipment_condition"] = s.sections.get("equipment_durability").map(|condition| {
        let rows=condition["entries"].as_object().unwrap();
        let mut slots:Vec<_>=rows.iter().map(|(slot,row)| {
            let current=row["current"].as_u64().unwrap();
            let maximum=row["maximum"].as_u64().unwrap();
            (slot.parse::<u32>().unwrap(),current,maximum)
        }).collect();
        slots.sort_by_key(|(slot,_,_)|*slot);
        let minimum=slots.iter().map(|(_,current,maximum)|100.0*(*current as f64)/(*maximum as f64)).reduce(f64::min);
        json!({"coverage":condition["coverage"],"observed_slots":slots.len(),"unobserved_slots":19-slots.len(),
            "minimum_observed_percent":minimum,
            "damaged_slots":slots.iter().filter(|(_,current,maximum)|current<maximum).map(|(slot,_,_)|slot).collect::<Vec<_>>(),
            "broken_slots":slots.iter().filter(|(_,current,_)|*current==0).map(|(slot,_,_)|slot).collect::<Vec<_>>(),
            "slots":slots.into_iter().map(|(slot,current,maximum)|json!({"slot":slot,"current":current,"maximum":maximum,
                "percent":100.0*(current as f64)/(maximum as f64)})).collect::<Vec<_>>()})
    }).unwrap_or(Value::Null);
    summary["saved_world_bosses_observed"] = json!(
        s.sections
            .get("saved_world_bosses")
            .map(|section| section["entries"].as_object().unwrap().len())
    );
    let saved_instances = s
        .sections
        .get("saved_instances")
        .and_then(|saved| saved["entries"].as_object());
    summary["saved_instances_observed"] = json!(saved_instances.map(|rows| rows.len()));
    summary["locked_instances_observed"] = json!(
        saved_instances.map(|rows| rows.values().filter(|row| row["locked"] == true).count())
    );
    let toys = s
        .sections
        .get("toy_collection_view")
        .and_then(|view| view["entries"].as_object());
    summary["toy_rows_observed"] = json!(toys.map(|entries| entries.len()));
    summary["collected_toys_in_view"] = json!(toys.map(|entries| {
        entries
            .values()
            .filter(|row| row["collected"] == true)
            .count()
    }));
    if is_forever {
        summary["favorite_toys_in_view"] = json!(toys.map(|entries| {
            entries
                .values()
                .filter(|row| row["favorite"] == true)
                .count()
        }));
        summary["collected_favorite_toys_in_view"] = json!(toys.map(|entries| {
            entries
                .values()
                .filter(|row| row["collected"] == true && row["favorite"] == true)
                .count()
        }));
    }
    summary["tracked_achievement_workload"] = json!(
        s.sections
            .get("tracked_achievements")
            .and_then(tracked_achievement_workload)
    );
    summary["currency_weekly_cap_summary"] = json!(
        s.sections
            .get("currency_balances")
            .and_then(currency_weekly_cap_summary)
    );
    summary["standard_reputation_workload"] = json!(
        s.sections
            .get("reputation")
            .and_then(standard_reputation_workload)
    );
    summary["paragon_reward_summary"] = json!(
        s.sections
            .get("reputation")
            .and_then(|section| paragon_reward_summary(section, quests))
    );
    summary["flight_fare_summary"] = json!(
        s.sections
            .get("flight_map")
            .and_then(captured_flight_fare_summary)
    );
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Snapshot {
        serde_json::from_value(json!({"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z","client":{"family":"retail","variant":"live","build":"123","interface":120105},"character":{"name":"Example","realm":"Test"},"sections":{"equipment":{"16":{"item_id":1}}}})).unwrap()
    }
    #[test]
    fn compare_identity_missing_and_changed() {
        let a = fixture();
        let mut b = a.clone();
        b.sections.get_mut("equipment").unwrap()["16"]["item_id"] = json!(2);
        assert_eq!(
            compare(&a, &b).unwrap()["sections"]["equipment"]["changed_paths"],
            json!(["/16/item_id"])
        );
        b.sections.remove("equipment");
        assert_eq!(
            compare(&a, &b).unwrap()["sections"]["equipment"]["status"],
            "not_comparable"
        );
        b.client.build = "124".into();
        assert!(compare(&a, &b).is_err());
        b = a.clone();
        b.character.name = "Other".into();
        assert!(compare(&a, &b).is_err());
    }
    #[test]
    fn output_bounded_and_missing_not_zero() {
        let a = fixture();
        assert!(summary(&a)["carried_item_slots_observed"].is_null());
        let mut b = a.clone();
        for n in 0..500 {
            b.sections.get_mut("equipment").unwrap()[n.to_string()] = json!({"item_id":n});
        }
        let result = compare(&a, &b).unwrap();
        assert_eq!(
            result["sections"]["equipment"]["changed_paths"]
                .as_array()
                .unwrap()
                .len(),
            100
        );
        assert_eq!(result["sections"]["equipment"]["truncated"], true);
    }

    #[test]
    fn unavailable_sections_and_malformed_bags_remain_unknown() {
        let mut a = fixture();
        a.unavailable
            .insert("talents".into(), "API unavailable".into());
        let result = compare(&a, &a).unwrap();
        assert_eq!(result["sections"]["talents"]["status"], "not_comparable");
        a.sections.insert("bags".into(), json!({"0": {}, "1": []}));
        assert!(summary(&a)["carried_item_slots_observed"].is_null());
        a.sections.insert("bags".into(), json!({"0": {}}));
        assert_eq!(summary(&a)["carried_item_slots_observed"], 0);
    }

    #[test]
    fn carried_review_counts_locations_without_claiming_stack_compatibility() {
        let mut s = fixture();
        assert!(summary(&s)["carried_item_review"].is_null());
        s.sections.insert(
            "bags".into(),
            json!({"0":{"1":{"item_id":5,"count":3,"link":"variant-a"}},
            "1":{"2":{"item_id":5,"count":7,"link":"variant-b"},"3":{"item_id":6,"count":1}}}),
        );
        s.validate().unwrap();
        let review = summary(&s)["carried_item_review"].clone();
        assert_eq!(review["quantity_observed"], 11);
        assert_eq!(review["distinct_item_ids_observed"], 2);
        assert_eq!(review["repeated_item_ids"][0]["quantity_observed"], 10);
        assert_eq!(
            review["repeated_item_ids"][0]["locations"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(review["stack_compatibility"], "unknown");
        assert_eq!(review["game_execution"], false);
        s.sections.insert("bags".into(), json!({"0":{}}));
        assert_eq!(summary(&s)["carried_item_review"]["quantity_observed"], 0);
        s.sections
            .insert("bags".into(), json!({"0":{"1":{"item_id":1}}}));
        assert!(summary(&s)["carried_item_review"].is_null());
        let mut slots = serde_json::Map::new();
        for slot in 1..=462 {
            slots.insert(slot.to_string(), json!({"item_id":(slot-1)/22+1,"count":1}));
        }
        s.sections.insert("bags".into(), json!({"0":slots}));
        s.validate().unwrap();
        let bounded = summary(&s)["carried_item_review"].clone();
        assert_eq!(bounded["quantity_observed"], 462);
        assert_eq!(bounded["repeated_item_id_count"], 21);
        assert_eq!(bounded["repeated_item_ids"].as_array().unwrap().len(), 20);
        assert_eq!(bounded["groups_truncated"], true);
        assert_eq!(
            bounded["repeated_item_ids"][0]["locations"]
                .as_array()
                .unwrap()
                .len(),
            20
        );
        assert_eq!(bounded["repeated_item_ids"][0]["locations_truncated"], true);
    }
}
