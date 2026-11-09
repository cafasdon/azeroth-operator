//! Snapshot-only analysis. Missing data is not an empty inventory.
use crate::Snapshot;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct ObservedReagent {
    quantities: [u64; 3],
    stack_count: usize,
    locations: Vec<Value>,
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
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecipeViewFilter {
    pub learned: Option<bool>,
    pub can_skill_up: Option<bool>,
    pub minimum_skill_ups_reported: Option<u32>,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecipeCooldownFilter {
    pub state: Option<RecipeCooldownState>,
    pub minimum_charges: Option<u32>,
    pub maximum_remaining_seconds: Option<u32>,
}

fn recipe_cooldown_matches(row: &Value, filter: &RecipeCooldownFilter) -> Option<bool> {
    let checks: Vec<_> = [
        filter
            .state
            .map(|state| Some(recipe_cooldown_state(row) == state)),
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
                || filter.minimum_charges.is_some()
                || filter.maximum_remaining_seconds.is_some(),
            "Cooldown filter requires at least one predicate"
        );
    }
    let mut cooldown_unknown = 0;
    let mut cooldown_nonmatching = 0;
    if let Some(filter) = recipe_view_filter {
        ensure!(
            filter.learned.is_some()
                || filter.can_skill_up.is_some()
                || filter.minimum_skill_ups_reported.is_some(),
            "Recipe view filter requires at least one predicate"
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
    let status_value = status.map(|value| json!(value));
    let mut matches: Vec<_> = rows
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter(|(id, row)| {
            recipe_id.is_none_or(|value| id.as_str() == value.to_string())
                && output_item_id.is_none_or(|value| {
                    row["output_item_id"].as_u64() == Some(u64::from(value.get()))
                })
                && reagent_item_id
                    .is_none_or(|value| row["reagents"].get(value.to_string()).is_some())
                && status_value
                    .as_ref()
                    .is_none_or(|value| row["status"] == *value)
        })
        .filter_map(|(id, row)| {
            if let Some(filter) = cooldown_filter {
                match recipe_cooldown_matches(row, filter) {
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
                    Some(support) if support["covers_minimum_crafts"] == true => Some(support),
                    Some(_) => {
                        insufficient += 1;
                        return None;
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
    matches.sort_unstable_by_key(|(id, _, _, _)| id.parse::<u32>().unwrap());
    let records: Vec<_> = matches
        .iter()
        .skip(offset)
        .take(limit)
        .map(|(id, row, support, observation)| {
            let mut record = (*row).clone();
            record["recipe_id"] = json!(id.parse::<u32>().unwrap());
            if cooldown_filter.is_some() {
                record["captured_cooldown_state"] = json!(recipe_cooldown_state(row));
            }
            if let Some(support) = support {
                record["material_support"] = support.clone();
            }
            if recipe_view_filter.is_some() {
                let observation = observation.unwrap();
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
    if let Some(inventory) = inventory {
        result["recipe_captured_at"] = json!(snapshot.captured_at);
        result["inventory_captured_at"] = json!(inventory.captured_at);
    }
    if let Some(filter) = cooldown_filter {
        result["filters"]["cooldown_filter"] = json!(filter);
        result["cooldown_unknown_count"] = json!(rows.map(|_| cooldown_unknown));
        result["cooldown_nonmatching_count"] = json!(rows.map(|_| cooldown_nonmatching));
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

/// Explicit IDs select only the bulk observation, never a fallback schematic.
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
        if row["status"] == "fixed_items"
            && let Some(output) = row["output_item_id"].as_u64()
        {
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
        *requested += review["requested_crafts"].as_u64().unwrap();
        let charges = recipe["cooldown"]["charges"].as_u64();
        json!({"requested_crafts_through_step":*requested,
            "observed_charges":charges,
            "observed_charges_before_step":charges.map(|n| n.saturating_sub(before)),
            "observed_charges_remaining_if_completed":charges.map(|n| n.saturating_sub(*requested)),
            "shortfall_vs_cumulative_crafts":charges.map(|n| requested.saturating_sub(n)),
            "covers_cumulative_crafts":charges.map(|n| n >= *requested)})
    }
}

/// Reuse ordered-batch accounting for the exact printed chain steps, without external acquisitions.
pub fn reagent_chain_stock_plan(
    chain: &Value,
    snapshot: &Snapshot,
    inventory: Option<&Snapshot>,
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
    Ok(
        json!({"basis":"minimum_starting_items_for_printed_chain_order_and_reported_minimum_yields",
        "material_coverage":ordered_recipe_material_coverage(&materials),
        "starting_materials":materials,"material_checkpoint":checkpoint,
        "nonmaterial_requirements":ordered_recipe_requirements_with_charges(reviews.iter(), true),
        "recipe_captured_at":snapshot.captured_at,"inventory_captured_at":inventory.unwrap_or(snapshot).captured_at,
        "withdrawal_eligibility_verified":false,"current_stock_verified":false,"minimum_yields_verified":false,
        "craftability_verified":false,"live":false,"game_execution":false}),
    )
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
                    return Err(
                        json!({"reason":"ambiguous_captured_producer","item_id":item,
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
    let base_output_if_completed = recipe["base_output_quantity"]["minimum"]
        .as_u64()
        .zip(recipe["base_output_quantity"]["maximum"].as_u64())
        .map(|(minimum, maximum)| {
            json!({"item_id":recipe["output_item_id"],"minimum":minimum * u64::from(crafts),
                "maximum":maximum * u64::from(crafts)})
        });
    let recipe_id = recipe["recipe_id"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Invalid recipe ID"))?;
    let charges = recipe["cooldown"]["charges"].as_u64();
    let estimated_cooldown_end = recipe["cooldown"]["remaining_seconds"]
        .as_i64()
        .filter(|seconds| *seconds > 0)
        .and_then(chrono::TimeDelta::try_seconds)
        .and_then(|remaining| snapshot.captured_at.checked_add_signed(remaining));
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
        "base_output_if_completed":base_output_if_completed,
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
    for (index, review) in reviews.enumerate() {
        let mut preparation = review["captured_preparation"].clone();
        if cumulative {
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
    let unknown = entries
        .values()
        .filter(|row| row["status"] != "item")
        .count();
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
    Ok(
        json!({"basis":"captured_readable_loot_rows_only","grouping":"base_item_id_only",
        "captured_at":snapshot.captured_at,"slot_count":section["slot_count"],
        "unreadable_or_non_item_slots":unknown,"additional_matching_quantities_unknown":unknown > 0,
        "all_requests_covered_by_readable_slots":all_covered,"items":rows,
        "counts_toward_owned_inventory":false,"ownership_verified":false,"lootability_verified":false,
        "game_execution":false,"live":false}),
    )
}

pub fn find_item(snapshot: &Snapshot, item_id: u32, offset: usize, limit: usize) -> Result<Value> {
    ensure!(
        item_id > 0 && (1..=50).contains(&limit),
        "Invalid item query"
    );
    let mut locations = Vec::new();
    let mut matched_locations = 0usize;
    let mut quantities = BTreeMap::new();
    for storage in ["equipment", "bags", "character_bank", "account_bank"] {
        if storage.ends_with("_bank") && snapshot.client.family != crate::Family::Forever {
            continue;
        }
        let Some(section) = snapshot.sections.get(storage).and_then(Value::as_object) else {
            quantities.insert(storage, None);
            continue;
        };
        let mut quantity = 0;
        let mut record = |container: Option<&str>, slot: &str, item: &Value, count: u64| {
            if item["item_id"].as_u64() == Some(u64::from(item_id)) {
                quantity += count;
                if matched_locations >= offset && locations.len() < limit {
                    locations.push(json!({
                        "storage":storage,"container":container,"slot":slot,"count":count
                    }));
                }
                matched_locations += 1;
            }
        };
        for (key, item) in section {
            if storage == "equipment" {
                record(None, key, item, 1);
            } else if let Some(slots) = item.as_object() {
                for (slot, observed) in slots {
                    if let Some(count) = observed["count"].as_u64() {
                        record(Some(key), slot, observed, count);
                    }
                }
            }
        }
        quantities.insert(storage, Some(quantity));
    }
    Ok(json!({
        "captured_at":snapshot.captured_at,"item_id":item_id,"live":false,
        "locations":locations,"matched_locations":matched_locations,
        "quantity_by_storage":quantities,"has_more":matched_locations.saturating_sub(offset)>limit
    }))
}

fn compare_item(before: &Snapshot, after: &Snapshot, item_id: u32) -> Result<Value> {
    let before = find_item(before, item_id, 0, 1)?;
    let after = find_item(after, item_id, 0, 1)?;
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
    Ok(json!({"item_id":item_id,"by_storage":by_storage,"net_delta":net_delta}))
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

fn pet_food_locations(snapshot: &Snapshot) -> Option<Value> {
    let foods = snapshot.sections.get("pet_care")?["carried_food"].as_object()?;
    let bags = snapshot.sections.get("bags")?.as_object()?;
    let mut locations = Vec::new();
    let mut matching_stacks = 0usize;
    for (bag, slots) in bags {
        for (slot, item) in slots.as_object()? {
            let id = item["item_id"].as_u64()?;
            if foods.contains_key(&id.to_string()) {
                matching_stacks += 1;
                if locations.len() < 50 {
                    locations
                        .push(json!({"bag":bag,"slot":slot,"item_id":id,"count":item["count"]}));
                }
            }
        }
    }
    Some(json!({
        "matching_stacks":matching_stacks,
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
pub struct ComparisonSelectors {
    pub profession_spell_changes: bool,
    pub player_resource_changes: bool,
    pub item_id: Option<u32>,
    pub currency_id: Option<u32>,
    pub faction_id: Option<u32>,
    pub quest_id: Option<u32>,
    pub quest_waypoints: bool,
    pub quest_readiness_changes: bool,
    pub achievement_status_changes: bool,
    pub selected_recipe_changes: bool,
    pub mount_collection_changes: bool,
    pub toy_view_changes: bool,
    pub saved_instance_changes: bool,
    pub profession_recipe_changes: bool,
    pub pet_view_changes: bool,
    pub mail_attachment_changes: bool,
    pub achievement_id: Option<u32>,
    pub profession_skill_line: Option<u32>,
    pub wallet: bool,
    pub experience: bool,
    pub spellbook: bool,
    pub action_bar_slots: bool,
}

pub fn compare_observations(
    before: &Snapshot,
    after: &Snapshot,
    selectors: ComparisonSelectors,
) -> Result<Value> {
    ensure_comparable(before, after)?;
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
                json!({"status":"not_comparable","before_available":before.sections.contains_key(name),"after_available":after.sections.contains_key(name)})
            }
        };
        sections.insert(name.clone(), result);
    }
    let mut comparison = json!({"client":after.client,"character":after.character,
        "before_captured_at":before.captured_at,"after_captured_at":after.captured_at,
        "live":false,"sections":sections});
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
                json!({"profession_id":new["profession_id"],"before_reported_count":old_ids.len(),
                    "after_reported_count":new_ids.len(),"newly_reported_spell_count":added.len(),
                    "no_longer_reported_spell_count":removed.len(),
                    "newly_reported_spell_ids":added.iter().take(50).collect::<Vec<_>>(),
                    "no_longer_reported_spell_ids":removed.iter().take(50).collect::<Vec<_>>(),
                    "newly_reported_truncated":added.len()>50,"no_longer_reported_truncated":removed.len()>50,
                    "learning_verified":false,"complete_recipe_knowledge_verified":false})
            }).unwrap_or(Value::Null);
    }
    if selectors.player_resource_changes {
        comparison["player_resource_changes"] = before.sections.get("player_resources")
            .zip(after.sections.get("player_resources"))
            .map(|(old,new)| {
                let counters=|a:&Value,b:&Value| json!({"before":a,"after":b,
                    "current_delta":b["current"].as_i64().unwrap()-a["current"].as_i64().unwrap(),
                    "maximum_delta":b["maximum"].as_i64().unwrap()-a["maximum"].as_i64().unwrap()});
                let power=old.get("primary_power").zip(new.get("primary_power"));
                json!({"health":old.get("health").zip(new.get("health")).map(|(a,b)|counters(a,b)),
                    "primary_power":power.filter(|(a,b)| a["power_type"]==b["power_type"] && a["units"]==b["units"]).map(|(a,b)|counters(a,b)),
                    "primary_power_type_changed":power.map(|(a,b)|a["power_type"]!=b["power_type"]),
                    "resource_events_verified":false,"readiness_verified":false})
            }).unwrap_or(Value::Null);
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
        && let Some(travel) = compare_travel_context(
            before.sections.get("stats").unwrap_or(&Value::Null),
            after.sections.get("stats").unwrap_or(&Value::Null),
        )
    {
        comparison["travel_context_change"] = travel;
    }
    let merchant_before = before.sections.get("merchant_stock");
    let merchant_after = after.sections.get("merchant_stock");
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
    if trainer_before.is_some() || trainer_after.is_some() {
        comparison["trainer_service_changes"] = trainer_before
            .zip(trainer_after)
            .and_then(|(old, new)| compare_trainer_services(old, new))
            .unwrap_or(Value::Null);
    }
    if flight_before.is_some() || flight_after.is_some() {
        comparison["flight_map_changes"] = flight_before
            .zip(flight_after)
            .map(|(old_map, new_map)| {
                let wallet =
                    |snapshot: &Snapshot| snapshot.sections.get("wallet")?["copper"].as_u64();
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
        comparison["item_quantity_change"] = compare_item(before, after, item_id)?;
    }
    if let Some(currency_id) = selectors.currency_id {
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
    if let Some(faction_id) = selectors.faction_id {
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
    if let Some(quest_id) = selectors.quest_id {
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
            comparison["quest_change"]["timer_change"] = json!({
                "before_observed":old_timer.is_some(),"after_observed":new_timer.is_some(),
                "remaining_seconds_delta":pair.map(|(old,new)| new.0 - old.0),
                "total_seconds_delta":pair.map(|(old,new)| new.1 - old.1)
            });
        }
        if before.client.family == crate::Family::Forever {
            let old_objectives = previous
                .as_ref()
                .and_then(|quest| quest["objectives"].as_object());
            let new_objectives = current
                .as_ref()
                .and_then(|quest| quest["objectives"].as_object());
            if old_objectives.is_some() || new_objectives.is_some() {
                comparison["quest_change"]["objective_counts_by_kind"] = json!({"before":old_objectives.map(quest_objective_counts),
                           "after":new_objectives.map(quest_objective_counts)});
            }
        }
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
            for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
                let Ok(id) = key.parse::<u32>() else { continue };
                match (before.get(key), after.get(key)) {
                    (Some(old), Some(new)) => {
                        let previous = quest_readiness(old);
                        let current = quest_readiness(new);
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
            for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
                let Ok(id) = key.parse::<u32>() else { continue };
                match (before.get(key), after.get(key)) {
                    (Some(old), Some(new)) if old["account_wide"] != new["account_wide"] => {
                        scope_changed.insert(id);
                    }
                    (Some(old), Some(new)) => {
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
                "achievement_ids_with_scope_changes":scope_changed})
        }));
    }
    if let Some(achievement_id) = selectors.achievement_id {
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
    }
    if selectors.action_bar_slots {
        ensure!(
            before.client.family == crate::Family::Forever,
            "Action bar slot comparisons require Forever snapshots"
        );
        comparison["action_bar_slot_changes"] = compare_action_bar_slots(before, after);
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

fn compare_action_bar_slots(before: &Snapshot, after: &Snapshot) -> Value {
    fn slots(snapshot: &Snapshot) -> Option<BTreeMap<u32, &Value>> {
        snapshot.sections.get("action_bar_observation")?["slots"]
            .as_object()?
            .iter()
            .map(|(slot, row)| Some((slot.parse().ok()?, row)))
            .collect()
    }
    let previous = slots(before);
    let current = slots(after);
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

pub fn action_bar_item_stock_audit(actions: &Value, snapshot: &Snapshot) -> Option<Value> {
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
                (
                    id,
                    json!({"slots":slots,"equipment_quantity":equipment,"bag_quantity":bags}),
                )
            })
            .collect::<BTreeMap<_, _>>()
    ))
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

pub fn action_bar_spellbook(
    actions: &Value,
    spellbook: Option<&Value>,
    include_reverse: bool,
) -> Value {
    let book = spellbook.and_then(|book| book["entries"].as_object());
    let mut base_ids = BTreeSet::new();
    let mut reported_ids = BTreeSet::new();
    for row in book.into_iter().flat_map(|rows| rows.values()) {
        base_ids.extend(row["base_spell_id"].as_u64());
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
                "matches_base_spell_id":observed.map(|id| base_ids.contains(&id)),
                "matches_reported_spell_id":observed.map(|id| reported_ids.contains(&id))}),
                )
            })
            .collect::<BTreeMap<_, _>>()
    });
    let mut review = json!({"spellbook_available":book.is_some(),"spell_slots":slots});
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
        let added = b.keys().copied().filter(|id| !a.contains_key(id)).collect::<Vec<_>>();
        let removed = a.keys().copied().filter(|id| !b.contains_key(id)).collect::<Vec<_>>();
        let changed = a.iter().filter_map(|(id, old)| {
            let new = b.get(id)?;
            (old != new).then(|| json!({"base_spell_id":id,"before_reported_spell_ids":old,"after_reported_spell_ids":new}))
        }).collect::<Vec<_>>();
        json!({"added_base_spell_ids":added,"removed_base_spell_ids":removed,"reported_spell_id_changes":changed,
            "added_observed_names_by_base_id":spellbook_names_for(after, &added),
            "removed_observed_names_by_base_id":spellbook_names_for(before, &removed),
            "after_action_slots_with_lost_spellbook_match":lost_spellbook_action_matches(a, b, after),
            "observed_flag_changes":compare_spellbook_flags(before, after),
            "observed_level_learned_changes":compare_spellbook_levels(before, after)})
    });
    json!({"before_available":previous.is_some(),"after_available":current.is_some(),"changes":changes})
}

fn lost_spellbook_action_matches<'a>(
    before: &BTreeMap<u64, BTreeSet<u64>>,
    after: &BTreeMap<u64, BTreeSet<u64>>,
    snapshot: &'a Snapshot,
) -> Option<BTreeMap<&'a str, u64>> {
    let slots = snapshot.sections.get("action_bar_observation")?["slots"].as_object()?;
    fn observed_ids(book: &BTreeMap<u64, BTreeSet<u64>>) -> BTreeSet<u64> {
        book.iter()
            .flat_map(|(&base, reported)| std::iter::once(base).chain(reported.iter().copied()))
            .collect()
    }
    let (before, after) = (observed_ids(before), observed_ids(after));
    Some(
        slots
            .iter()
            .filter_map(|(slot, action)| {
                let id = action["id"].as_u64()?;
                (action["kind"] == "spell" && before.contains(&id) && !after.contains(&id))
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

fn compare_criteria(before: &Value, after: &Value) -> Value {
    let before = criteria_by_id(before);
    let after = criteria_by_id(after);
    let mut matched = 0;
    let mut changes = Vec::new();
    for (id, previous) in before {
        let Some((previous, current)) = previous.zip(after.get(&id).copied().flatten()) else {
            continue;
        };
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

fn mail_attachment_totals(mail: &Value) -> (KnownMailTotals, u32) {
    let mut totals = KnownMailTotals::new();
    let mut unknown = 0u32;
    for message in mail["messages"].as_object().unwrap().values() {
        for row in message.as_object().unwrap().values() {
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
    }
    (totals, unknown)
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
        "observed_only_before":only_before,"observed_only_after":only_after,
        "observed_count_changes":changes})
}

fn compare_profession_recipe_views(before: &Value, after: &Value) -> Value {
    let old = before["entries"].as_object().unwrap();
    let new = after["entries"].as_object().unwrap();
    let same_profession = before["profession_id"] == after["profession_id"];
    let mut result = json!({"coverage":"filtered_local_recipe_list","same_profession":same_profession,
        "before_profession_id":before["profession_id"],"after_profession_id":after["profession_id"],
        "before_observed_rows":old.len(),"after_observed_rows":new.len(),
        "observed_only_before":null,"observed_only_after":null,"learning_observation_changes":null});
    if !same_profession {
        return result;
    }
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
                let first = a["learned"].as_bool();
                let second = b["learned"].as_bool();
                if first != second {
                    changes.push(
                        json!({"recipe_id":id,"before_learned":first,"after_learned":second}),
                    );
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
            let fields: serde_json::Map<_, _> = ["reset_seconds", "locked", "extended"]
                .into_iter()
                .filter(|field| a[0][*field] != b[0][*field])
                .map(|field| {
                    (
                        field.to_owned(),
                        json!({"before":a[0][field],"after":b[0][field]}),
                    )
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
        "before_displayed_count":before["displayed_count"],"after_displayed_count":after["displayed_count"]})
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
        "before_catalogue_count":before["catalogue_count"],"after_catalogue_count":after["catalogue_count"]})
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
            observed
        })
    };
    let food_changes =
        before["carried_food"]
            .as_object()
            .zip(after["carried_food"].as_object())
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
                    })))
                    })
                    .collect::<BTreeMap<_, _>>()
            });
    json!({
        "before":observation(before),"after":observation(after),
        "accepted_carried_food_changes":food_changes,
    })
}

fn compare_travel_context(before: &Value, after: &Value) -> Option<Value> {
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
    let observed = |stats: &Value| {
        json!({
            "map_id":stats["map_id"],"map_name":stats["map_name"],
            "hearthstone_bind_name":stats["hearthstone_bind_name"],
            "hearthstone_item_id":stats["hearthstone_item_id"],
            "hearthstone_cooldown_seconds":stats["hearthstone_cooldown_seconds"],
        })
    };
    Some(json!({
        "before":observed(before),"after":observed(after),
        "map_changed":map_ids.0.zip(map_ids.1).map(|(old,new)| old != new),
        "hearthstone_bind_name_changed":bind_names.0.zip(bind_names.1).map(|(old,new)| old != new),
        "hearthstone_item_changed":items.map(|(old,new)| old != new),
        "hearthstone_cooldown_seconds_delta":cooldown_delta,
    }))
}

fn observed_flag_changed(before: Option<&Value>, after: Option<&Value>, key: &str) -> Option<bool> {
    Some(before?.get(key)?.as_bool()? != after?.get(key)?.as_bool()?)
}

fn ensure_comparable(before: &Snapshot, after: &Snapshot) -> Result<()> {
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
    }
    json!({
        "matched_skill_count":matched,
        "before_only_count":before.len()-matched,
        "after_only_count":after.len()-matched,
        "changes":changes.into_values().collect::<Vec<_>>()
    })
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

fn compare_merchant_offers(before: &Value, after: &Value) -> Option<(Value, Value)> {
    type OfferKey = (&'static str, u64, u64);
    fn by_product(panel: &Value) -> Option<BTreeMap<OfferKey, Option<&Value>>> {
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
    fn costs(offer: &Value) -> Option<Vec<&Value>> {
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
    let previous = by_product(before)?;
    let current = by_product(after)?;
    let observed_only =
        |offers: &BTreeMap<OfferKey, Option<&Value>>,
         other: &BTreeMap<OfferKey, Option<&Value>>| {
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
        let (old_costs, new_costs) = (costs(old), costs(new));
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
    let requirements = |row: &Value| {
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
    };
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
        let (before_requirements, after_requirements) = (requirements(old), requirements(new));
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

fn xp_to_next_level(xp: &Value) -> Option<u64> {
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

pub fn review_flight_itinerary(legs: &[(&str, Snapshot, u32)]) -> Result<Value> {
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
    let affordable_prefix = starting_wallet.map(|balance| {
        routes
            .iter()
            .scan(balance, |remaining, route| {
                *remaining = remaining.checked_sub(route["fare_copper"].as_u64()?)?;
                Some(())
            })
            .count()
    });
    let first_uncovered_leg = affordable_prefix.and_then(|index| {
        let route = routes.get(index)?;
        let reason = if route["fare_copper"].is_null() {
            "fare_unknown"
        } else {
            "insufficient_copper"
        };
        Some(json!({"leg_number":index + 1,"reason":reason}))
    });
    Ok(json!({"legs":routes,"total_fare_copper":total,
        "affordable_from_starting_wallet":budget.map(|(fare, balance)| fare <= balance),
        "starting_wallet_shortfall_copper":budget.map(|(fare, balance)| fare.saturating_sub(balance)),
        "starting_wallet_affordable_prefix_legs":affordable_prefix,
        "starting_wallet_first_uncovered_leg":first_uncovered_leg}))
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
    destination: u32,
    reserve: Option<u64>,
    max_legs: u8,
    objective: FlightRouteObjective,
    include_repair_quote: bool,
    require_affordable: bool,
) -> Result<Value> {
    ensure!((1..=8).contains(&max_legs), "Invalid flight leg limit");
    let (_, first) = captures
        .first()
        .ok_or_else(|| anyhow::anyhow!("Missing departure"))?;
    ensure!(
        first.client.family == crate::Family::Forever && captures.len() <= 8,
        "Expected up to eight Forever captures"
    );
    let mut departures = BTreeMap::<u64, Vec<_>>::new();
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
            Some((index, node, row["fare_copper"].as_u64()?))
        }));
    }
    let start = start.ok_or_else(|| anyhow::anyhow!("Missing departure"))?;
    let destination = u64::from(destination);
    ensure!(
        start != destination,
        "Already at the requested departure node"
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
        json!({
            "status":status,"origin_node_id":start,"destination_node_id":destination,
        })
    };
    let max_fare = if require_affordable {
        let required_repair = if include_repair_quote {
            repair
        } else {
            Some(0)
        };
        let Some((balance, repair_cost)) = wallet.zip(required_repair) else {
            return Ok(status("starting_budget_unknown"));
        };
        let Some(spendable) = balance
            .checked_sub(repair_cost)
            .and_then(|remaining| remaining.checked_sub(reserve.unwrap_or_default()))
        else {
            return Ok(status("no_affordable_route"));
        };
        Some(spendable)
    } else {
        None
    };
    let mut frontier = BTreeMap::from([(start, (0u64, Vec::new()))]);
    let mut best = None;
    for depth in 0..max_legs {
        let mut next = BTreeMap::new();
        for (node, (cost, path)) in frontier {
            for &(index, target, fare) in departures.get(&node).into_iter().flatten() {
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
        return Ok(status(if require_affordable {
            "no_affordable_route"
        } else {
            "no_fully_priced_route"
        }));
    };
    let legs = path
        .into_iter()
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
        .collect::<Result<Vec<_>>>()?;
    let mut plan = json!({"status":"found","origin_node_id":start,"destination_node_id":destination,
        "legs":legs,"total_fare_copper":fare,
        "affordable_from_starting_wallet":wallet.map(|balance| fare <= balance)});
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
    level: Option<u64>,
    skills: Option<&Value>,
    spellbook: Option<&Value>,
) -> Value {
    let Some(entries) = trainer["entries"].as_object() else {
        return Value::Null;
    };
    let pet_level = trainer["pet_level"].as_u64();
    let requirement_level = if trainer["kind"] == "pet" {
        pet_level
    } else {
        level
    };
    let mut ranks_by_name = BTreeMap::new();
    let mut available_slots_by_name = BTreeMap::<&str, BTreeSet<u32>>::new();
    for (slot, lesson) in entries
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
            (
                slot,
                json!({
                    "level_shortfall":level_shortfall,
                    "skill_requirement_unmet":unmet_skill,
                    "captured_skill_rank_comparison":skill_comparison,
                    "unmet_ability_names":unmet_abilities,
                    "unmet_ability_names_observed_in_spellbook":observed_unmet_abilities,
                    "available_lesson_slots_by_unmet_ability_name":candidate_lessons
                }),
            )
        })
        .collect();
    let mut review =
        json!({"captured_character_level":level,"unavailable_lessons_by_slot":lessons});
    if trainer["kind"] == "pet" {
        review["captured_pet_level"] = json!(pet_level);
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

fn trainer_lesson_affordability(trainer: &Value, price_field: &str, balance: Option<u64>) -> Value {
    json!(trainer["entries"].as_object().into_iter().flatten()
        .filter(|(_, lesson)| lesson["status"] == "available")
        .map(|(id, lesson)| {
            let comparison = lesson[price_field].as_u64().zip(balance);
            (id, json!({
                "affordable": comparison.map(|(price, balance)| price <= balance),
                "shortfall": comparison.map(|(price, balance)| price.saturating_sub(balance))
            }))
        }).collect::<BTreeMap<_, _>>())
}

fn trainer_cheapest_affordable(
    trainer: &Value,
    price_field: &str,
    balance: Option<u64>,
) -> Option<Value> {
    let balance = balance?;
    let mut priced: Vec<_> = trainer["entries"]
        .as_object()?
        .iter()
        .filter(|(_, lesson)| lesson["status"] == "available")
        .filter_map(|(slot, lesson)| Some((lesson[price_field].as_u64()?, slot)))
        .collect();
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
    Some(json!({"lesson_slots":slots,"lesson_count":slots.len(),"total_price":subtotal}))
}

fn trainer_order_budget(
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
        "individual_lesson_affordability":trainer_lesson_affordability(trainer, "price_copper", balance),
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
        "individual_lesson_affordability":trainer_lesson_affordability(trainer, "price_training_points", balance),
        "cheapest_affordable_known_lessons":trainer_cheapest_affordable(trainer, "price_training_points", balance),
    });
    if let Some(slots) = slots {
        projection["ordered_lesson_budget"] =
            trainer_order_budget(trainer, slots, "price_training_points", balance);
    }
    projection
}

pub fn merchant_quantity_plan(offers: &Value, quantity: u32) -> Value {
    let Some(offers) = offers.as_object() else {
        return Value::Null;
    };
    let rows: BTreeMap<_, _> = offers
        .iter()
        .map(|(slot, offer)| (slot, merchant_quantity_row(offer, u64::from(quantity))))
        .collect();
    json!({"target_quantity":quantity,"offers_by_slot":rows})
}

pub(crate) fn merchant_quantity_row(offer: &Value, quantity: u64) -> Option<Value> {
    let size = offer["bundle_size"].as_u64().filter(|size| *size > 0)?;
    let bundles = quantity.div_ceil(size);
    let units = bundles.checked_mul(size)?;
    let copper = offer["price_copper"].as_u64()?.checked_mul(bundles);
    Some(json!({
        "bundles":bundles,"units":units,"extra_units":units - quantity,
        "required_copper":copper,"extended_cost":offer["extended_cost"],
        "captured_available":offer["available"]
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
    let mut result = json!({"basis":"chain_external_material_shortfalls","captured_at":source.captured_at,
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
    Ok(result)
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
            let options: BTreeMap<_, _> = offers
                .iter()
                .filter(|(_, offer)| offer["item_id"].as_u64() == Some(item))
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
                "lowest_unlimited_copper_only_offer":lowest_copper
            })
        })
        .collect();
    let total = reagents.iter().try_fold(0u64, |total, reagent| {
        total
            .checked_add(reagent["lowest_unlimited_copper_only_offer"]["required_copper"].as_u64()?)
    });
    let budget = total.zip(wallet);
    json!({"reagents":reagents,
        "unlimited_copper_only_total":total,
        "unlimited_copper_only_affordable":budget.map(|(cost, balance)| cost <= balance),
        "unlimited_copper_only_shortfall":budget.map(|(cost, balance)| cost.saturating_sub(balance))})
}

pub fn buyback_cost_review(snapshot: &Snapshot, slots: &[u32], reserve: u64) -> Result<Value> {
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
    Ok(
        json!({"lines":lines,"missing_captured_slots":missing,"slots_with_unknown_price":unknown_prices,
        "known_price_subtotal_copper":subtotal,"total_selected_quote_copper":total,
        "captured_wallet_copper":wallet,"wallet_reserve_copper":reserve,"spendable_after_reserve_copper":spendable,
        "reserve_shortfall_copper":reserve_shortfall,"known_shortfall_vs_spendable_copper":known_shortfall,
        "total_shortfall_vs_spendable_copper":total.zip(spendable).map(|(cost,balance)|cost.saturating_sub(balance)),
        "affordable_from_captured_wallet":affordable,"buyback_eligibility_verified":false,"game_action_performed":false}),
    )
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
    Ok(result)
}

pub fn flight_wallet_projection(map: &Value, balance: Option<u64>, reserve: u64) -> Value {
    let Some(entries) = map["entries"].as_object() else {
        return Value::Null;
    };
    Value::Object(
        entries.iter().map(|(slot, node)| {
            let cost_and_balance = node["fare_copper"].as_u64()
                .and_then(|fare| fare.checked_add(reserve)).zip(balance);
            (slot.clone(), json!({
                "affordable":cost_and_balance.map(|(cost, balance)| cost <= balance),
                "shortfall_copper":cost_and_balance.map(|(cost, balance)| cost.saturating_sub(balance)),
            }))
        }).collect()
    )
}

pub fn quest_list_wallet_projection(
    quests: &serde_json::Map<String, Value>,
    wallet: Option<&Value>,
) -> Value {
    let mut known_cost = 0u128;
    let mut known_reward = 0u128;
    let mut unknown_cost = 0;
    let mut unknown_reward = 0;
    for quest in quests.values() {
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

fn quest_wallet_turn_in_order(
    quests: &serde_json::Map<String, Value>,
    balance: Option<u64>,
) -> Option<Value> {
    let mut steps = quests
        .iter()
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

fn tracked_achievement_workload(section: &Value) -> Option<Value> {
    let entries = section["entries"].as_object()?;
    let mut unfinished = Vec::new();
    let mut completed_without_credit = Vec::new();
    for (id, achievement) in entries {
        let Ok(id) = id.parse::<u32>() else { continue };
        if achievement["completed"] == true && achievement["earned_by_character"] == false {
            completed_without_credit.push(id);
        }
        let Some(criteria) = achievement["criteria"].as_object() else {
            continue;
        };
        let count = criteria
            .values()
            .filter(|row| row["completed"] == false)
            .count();
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

#[derive(Clone, Default, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AchievementFilter {
    pub completed: Option<bool>,
    pub earned_by_character: Option<bool>,
    pub account_wide: Option<bool>,
    pub criterion_id: Option<std::num::NonZeroU32>,
    pub criterion_completed: Option<bool>,
    pub criterion_max_remaining: Option<u32>,
}

impl AchievementFilter {
    pub fn is_empty(&self) -> bool {
        self.completed.is_none()
            && self.earned_by_character.is_none()
            && self.account_wide.is_none()
            && self.criterion_id.is_none()
            && self.criterion_completed.is_none()
            && self.criterion_max_remaining.is_none()
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
        if self.criterion_id.is_none()
            && self.criterion_completed.is_none()
            && self.criterion_max_remaining.is_none()
        {
            return Some(true);
        }
        let criteria = achievement["criteria"].as_object()?;
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

pub fn quest_objective_matches(
    quest: &Value,
    kind: Option<&str>,
    finished: Option<bool>,
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
        if let Some(maximum) = maximum_remaining {
            let remaining = objective["required"]
                .as_u64()
                .zip(objective["fulfilled"].as_u64())
                .map(|(required, fulfilled)| required.saturating_sub(fulfilled));
            match remaining {
                Some(remaining) if remaining <= u64::from(maximum) => return Some(true),
                Some(_) => {}
                None => unknown = true,
            }
        } else {
            return Some(true);
        }
    }
    if unknown { None } else { Some(false) }
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
    let repair_status = is_forever.then(|| {
        let alerts = s.sections.get("durability_alerts").and_then(Value::as_object);
        let quote = s.sections.get("repair_quote").and_then(|quote| quote["copper"].as_u64());
        let shortfall = quote.zip(wallet).map(|(cost, balance)| cost.saturating_sub(balance));
        json!({
            "low_slots":alerts.map(|slots| slots.values().filter(|status| status.as_str()==Some("low")).count()),
            "broken_slots":alerts.map(|slots| slots.values().filter(|status| status.as_str()==Some("broken")).count()),
            "repair_all_quote_copper":quote,
            "wallet_shortfall_copper":shortfall
        })
    });
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
    summary["collected_mounts_observed"] = json!(
        s.sections
            .get("mount_collection")
            .and_then(|collection| collection["entries"].as_object())
            .map(|entries| entries.len())
    );
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
    summary["pet_collection_view_counts"] = s.sections.get("pet_collection_view").map(|view| {
        let rows=view["entries"].as_object().unwrap();
        json!({"displayed_rows":view["displayed_rows"],"species_observed":rows.len(),
            "owned_rows_observed":rows.values().map(|row|row["owned_rows"].as_u64().unwrap()).sum::<u64>(),
            "unowned_rows_observed":rows.values().map(|row|row["unowned_rows"].as_u64().unwrap()).sum::<u64>()})
    }).unwrap_or(Value::Null);
    summary["mail_attachment_counts"] = s.sections.get("mail_attachments").map(|mail| {
        let rows: Vec<_> = mail["messages"].as_object().unwrap().values()
            .flat_map(|attachments| attachments.as_object().unwrap().values()).collect();
        json!({"loaded_messages":mail["loaded_messages"],"total_messages":mail["total_messages"],
            "observed_slots":rows.len(),"item_slots":rows.iter().filter(|row| row["kind"]=="item").count(),
            "currency_slots":rows.iter().filter(|row| row["kind"]=="currency").count(),
            "unknown_slots":rows.iter().filter(|row| row["kind"]=="unknown").count()})
    }).unwrap_or(Value::Null);
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
            json!({"coverage":section["coverage"],"slot_count":section["slot_count"],
            "item_slots":item_slots,"unreadable_or_non_item_slots":rows.len()-item_slots,
            "observed_item_quantities":quantities,"counts_toward_owned_inventory":false,
            "ownership_verified":false,"lootability_verified":false,"game_execution":false})
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
        json!({"coverage":view["coverage"],"profession_id":view["profession_id"],
            "candidate_count":candidates.len(),"truncated":candidates.len()>50,
            "learned_rows_with_unknown_skill_up_flag":rows.values().filter(|row|row["learned"]==true && row["can_skill_up"].as_bool().is_none()).count(),
            "eligibility_verified":false,
            "recipes":candidates.into_iter().take(50).map(|(id,row)|json!({"recipe_id":id.parse::<u32>().unwrap(),
                "relative_difficulty":row["relative_difficulty"],"skill_ups_reported":row["skill_ups_reported"]})).collect::<Vec<_>>()})
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
    summary["gossip_quest_observations"] = s.sections.get("gossip_quests").map(|section| {
        let describe=|rows:Option<&Value>|rows.map(|rows| {
            let rows=rows.as_object().unwrap();
            let mut ids:Vec<_>=rows.keys().map(|id|id.parse::<u32>().unwrap()).collect();ids.sort_unstable();
            let complete:Vec<_>=ids.iter().filter(|id|rows[&id.to_string()]["is_complete"]==true).copied().collect();
            json!({"observed_quest_count":ids.len(),"quest_ids":ids.iter().take(50).collect::<Vec<_>>(),"quest_ids_truncated":ids.len()>50,
                "reported_complete_quest_count":complete.len(),"reported_complete_quest_ids":complete.iter().take(50).collect::<Vec<_>>(),
                "reported_complete_truncated":complete.len()>50,
                "quests_with_unknown_completion":rows.values().filter(|row|row.get("is_complete").is_none()).count()})
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
