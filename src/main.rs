use azeroth_operator::{Family, Store};
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

// New routes are unavailable until their inputs and effects pass policy review.
const OFFLINE_TOOLS: &[&str] = &[
    "capabilities",
    "inspect_snapshot",
    "list_snapshots",
    "compare_snapshots",
    "character_summary",
    "review_equipment_selection",
    "captured_talent_loadout",
    "lookup_forever_catalogue",
    "review_forever_owned_gear",
    "review_forever_recipe",
    "find_snapshot_item",
];

#[derive(Clone)]
struct Operator {
    store: Store,
    forever_catalogue: Option<std::path::PathBuf>,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Inspect {
    snapshot_id: String,
    section: Option<String>,
    /// Select one saved quest by ID. Requires section "quests".
    quest_id: Option<std::num::NonZeroU32>,
    /// Compare an explicit basket of costs against one saved Forever inventory.
    spending_budget: Option<SpendingBudget>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SpendingBudget {
    /// Total gold cost expressed in copper.
    wallet_copper: Option<std::num::NonZeroU64>,
    /// Up to 20 distinct currency costs.
    currencies: Option<Vec<CurrencyCost>>,
    /// Up to 20 distinct item costs, checked against carried bags.
    items: Option<Vec<ItemCost>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CurrencyCost {
    currency_id: std::num::NonZeroU32,
    quantity: std::num::NonZeroU64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ItemCost {
    item_id: std::num::NonZeroU32,
    quantity: std::num::NonZeroU64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct List {
    family: String,
    character: Option<azeroth_operator::Character>,
    /// Filter Forever snapshots by the exact output item ID of the captured selected recipe.
    recipe_output_item_id: Option<std::num::NonZeroU32>,
    offset: Option<usize>,
    limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Compare {
    before_id: String,
    after_id: String,
    /// Optionally compare captured quantities for one base item ID.
    item_id: Option<std::num::NonZeroU32>,
    /// Optionally compare one captured Forever currency balance by ID.
    currency_id: Option<std::num::NonZeroU32>,
    /// Optionally compare one captured Forever reputation faction by ID.
    faction_id: Option<std::num::NonZeroU32>,
    /// Optionally compare one captured quest by ID.
    quest_id: Option<std::num::NonZeroU32>,
    /// Optionally compare one tracked Forever achievement by ID.
    achievement_id: Option<std::num::NonZeroU32>,
    /// Optionally compare one captured profession by its skill-line ID.
    profession_skill_line: Option<std::num::NonZeroU32>,
    /// Include a net change in captured Forever wallet copper.
    #[serde(default)]
    wallet: bool,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Summary {
    snapshot_id: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OwnedGearReview {
    snapshot_id: String,
    offset: Option<usize>,
    limit: Option<usize>,
    /// Filter by the pinned catalogue category before paging. Unknown types are excluded when filtering.
    item_type: Option<azeroth_operator::forever_catalogue::GearType>,
    /// Filter by the pinned catalogue's setId before paging.
    set_id: Option<std::num::NonZeroU32>,
    /// Find every observed copy of this base item ID, including items absent from the catalogue.
    item_id: Option<std::num::NonZeroU32>,
    /// Find observed copies of up to sixteen distinct base item IDs. Exclusive with item_id.
    item_ids: Option<Vec<std::num::NonZeroU32>>,
    /// Filter candidates by a pinned simulator slot before paging.
    simulator_slot: Option<azeroth_operator::forever_catalogue::SimulatorSlot>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RecipeReview {
    snapshot_id: String,
    /// Number of crafts. Supply exactly one of crafts or target_output_quantity.
    crafts: Option<u32>,
    /// Desired base item quantity, using the captured minimum per craft.
    target_output_quantity: Option<std::num::NonZeroU32>,
    /// Optional same-character and same-build capture, at least as recent as the recipe.
    inventory_snapshot_id: Option<String>,
    /// Up to seven other saved recipes checked against the same inventory capture.
    additional_recipes: Option<Vec<RecipeBatchEntry>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RecipeBatchEntry {
    snapshot_id: String,
    /// Number of crafts. Supply exactly one of crafts or target_output_quantity.
    crafts: Option<u32>,
    /// Desired base item quantity, using the captured minimum per craft.
    target_output_quantity: Option<std::num::NonZeroU32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FindItem {
    snapshot_id: String,
    item_id: std::num::NonZeroU32,
    offset: Option<usize>,
    limit: Option<usize>,
    /// Up to seven additional snapshot IDs with the exact same client/build.
    additional_snapshot_ids: Option<Vec<String>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EquipmentReview {
    snapshot_id: String,
    /// Exact saved selections. An empty list reviews the captured Forever baseline with a local catalogue.
    selections: Vec<azeroth_operator::planning::EquipmentSelection>,
    /// Allow a selected two-handed main-hand item to clear the captured off-hand in the simulator draft.
    #[serde(default)]
    allow_two_hand_off_hand_clear: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MacroOperation {
    snapshot_id: String,
    operation: azeroth_operator::macro_operation::MacroRequest,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BindingOperation {
    snapshot_id: String,
    operation: azeroth_operator::binding_operation::BindingRequest,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EquipmentOperation {
    snapshot_id: String,
    operation: azeroth_operator::equipment_operation::EquipmentRequest,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BagOperation {
    snapshot_id: String,
    operation: azeroth_operator::bag_operation::BagRequest,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TalentOperation {
    snapshot_id: String,
    operation: azeroth_operator::talent_operation::TalentRequest,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ActionOperation {
    snapshot_id: String,
    operation: azeroth_operator::action_operation::ActionRequest,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EquipmentSetOperation {
    snapshot_id: String,
    operation: azeroth_operator::equipment_set_operation::EquipmentSetRequest,
}

#[tool_router]
impl Operator {
    #[tool(
        description = "Prepare Retail equipment-set operations. Create/save require addon 0.13.0-dev with player review/Apply, explicit ignored_slots and expect_empty_slots. Existing metadata, deletion and use requests are supported. Assign_spec/unassign_spec return association data with displaced associations requiring addon 0.12.0-dev and player review/Apply. Data only; never changes the game. Metadata requires addon 0.10.0-dev, use 0.11.0-dev and player review/Apply. Uncaptured equipped slots require explicit expect_empty_slots. Live execution remains unqualified. Does not repair external references.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_equipment_set_operation(
        &self,
        Parameters(args): Parameters<EquipmentSetOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::equipment_set_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Equipment set operation rejected: captured set or requested metadata unavailable or invalid",
            )]),
        }
    }
    #[tool(
        description = "Prepare a Retail action-slot move/swap or copy into an empty slot from two explicitly observed slots, including exact macro contents. Data only; copy player review and Apply requires addon 0.18.0-dev or later. Move/swap player review and Apply requires addon 0.9.0-dev or later. Live execution remains unqualified. Does not change action bars.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_action_operation(
        &self,
        Parameters(args): Parameters<ActionOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::action_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Action operation rejected: explicit slot or macro preimages unavailable or invalid",
            )]),
        }
    }

    #[tool(
        description = "Prepare Retail talent operations from captured contents. Load (default), rename and delete target a saved config; rename/delete require addon 0.14.0-dev. Create saves the current build under a new name, requires expected_new_shared_action_bars, forbids target_config_id and requires addon 0.15.0-dev. Creation selects the new build. Import accepts import_string plus explicit allow_empty_hash and expected_new_shared_action_bars, creates/selects a new build and changes active talents; requires addon 0.17.0-dev. Selected-loadout deletion requires addon 0.16.0-dev and clears saved selection while preserving current talent contents. No game changes; native qualification remains pending.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_talent_operation(
        &self,
        Parameters(args): Parameters<TalentOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::talent_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Talent operation rejected: committed or saved loadout observations unavailable",
            )]),
        }
    }
    #[tool(
        description = "Prepare a Retail carried-bag move/split to an explicitly empty slot, or operation=swap between two captured distinct item IDs. Bag compatibility preflight and player Apply require addon 0.20.0-dev or later; native execution unqualified. Omit amount for full-stack movement; provide a positive smaller amount to split. Binds captured GUID and count; player review and Apply required. Does not move items.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_bag_operation(&self, Parameters(args): Parameters<BagOperation>) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::bag_operation::compile(&snapshot, &args.snapshot_id, &args.operation)
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Bag operation rejected: source identity or empty destination precondition unavailable",
            )]),
        }
    }
    #[tool(
        description = "Prepare one exact-GUID Retail bag-to-equipment swap for player review. Requires item instance observations. Empty targets require explicit precondition. Does not equip anything; multi-slot handling remains unsupported.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_equipment_operation(
        &self,
        Parameters(args): Parameters<EquipmentOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::equipment_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Equipment operation rejected: source/target identities or preconditions unavailable",
            )]),
        }
    }
    #[tool(
        description = "Compile a Retail keybinding change for explicit player review/application. Requires context and current binding set observations. For uncaptured keys, expect_unbound=true supplies a precondition checked live, not an observed fact. Empty desired action means unbind. Does not press keys or invoke actions.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_binding_operation(
        &self,
        Parameters(args): Parameters<BindingOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::binding_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Binding operation rejected: missing, ambiguous or invalid captured key/context/set",
            )]),
        }
    }
    #[tool(
        description = "Report implemented capabilities, not Blizzard approval",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn capabilities(&self) -> String {
        let mut implemented: Vec<_> = self
            .tool_router
            .list_all()
            .into_iter()
            .map(|tool| tool.name)
            .collect();
        implemented.sort_unstable();
        serde_json::json!({"families":["retail","classic","forever"],"implemented":implemented,"game_execution":false,"live_client_qualified":[],"policy":{"mode":"offline_analysis","generated_game_imports":false,"default":"deny_unreviewed_tools","snapshot_scope":"configured_owner_store","reviewed_on":"2026-11-07","sources":["https://www.blizzard.com/en-gb/legal/2c72a35c-bf1b-4ae6-99ec-80624e1b429c/blizzard-end-user-license-agreement","https://us.forums.blizzard.com/en/wow/t/ui-add-on-development-policy/24534","https://us.forums.blizzard.com/en/wow/t/policy-update-for-input-broadcasting-may-2021/956613"]}}).to_string()
    }
    #[tool(
        description = "Look up gear, consumables, world reference records and encounter presets in the local pinned WoWSims Forever catalogue. A saved snapshot can supply stock counts. Omit all filters for the section count. No network, live acquisition validation or simulation execution.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn lookup_forever_catalogue(
        &self,
        Parameters(query): Parameters<azeroth_operator::forever_catalogue::Query>,
    ) -> CallToolResult {
        let result = (|| {
            let path = self
                .forever_catalogue
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Catalogue unavailable"))?;
            let snapshot = query
                .acquisition_snapshot_id
                .as_deref()
                .map(|id| self.store.read(id))
                .transpose()?;
            let mut data =
                azeroth_operator::forever_catalogue::lookup(path, &query, snapshot.as_ref())?;
            if let Some((id, snapshot)) = query.acquisition_snapshot_id.zip(snapshot) {
                let mut progress = if matches!(
                    query.section,
                    azeroth_operator::forever_catalogue::Section::Consumables
                ) {
                    azeroth_operator::forever_catalogue::consumable_inventory(
                        &snapshot,
                        &data["records"],
                        query.consumable_targets.as_deref(),
                    )?
                } else {
                    azeroth_operator::forever_catalogue::acquisition_progress(
                        &snapshot,
                        &data["records"],
                    )?
                };
                if let Some(recipe_ids) = query.restock_recipe_snapshot_ids {
                    self.add_restock_crafting(&snapshot, &mut progress, recipe_ids)?;
                }
                progress["snapshot_id"] = serde_json::json!(id);
                data["acquisition_progress"] = progress;
            }
            Ok::<_, anyhow::Error>(data)
        })();
        match result {
            Ok(data) => CallToolResult::success(vec![ContentBlock::text(data.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Forever catalogue lookup failed: check filters, simulation settings, the local cache and compatible saved captures. Restock recipes require learned fixed-material recipes with a recorded yield for distinct confirmed shortages.",
            )]),
        }
    }
    #[tool(
        description = "Review one saved Forever character's observed equipment, bags and banks with local catalogue matches by base item ID. Reports missing storage and preserves observed variants. No ranking, game commands or network.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn review_forever_owned_gear(
        &self,
        Parameters(args): Parameters<OwnedGearReview>,
    ) -> CallToolResult {
        let result = (|| {
            let catalogue = self
                .forever_catalogue
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Catalogue unavailable"))?;
            let snapshot = self.store.read(&args.snapshot_id)?;
            anyhow::ensure!(
                args.item_id.is_none() || args.item_ids.is_none(),
                "Choose item_id or item_ids"
            );
            let item_ids = args
                .item_ids
                .as_deref()
                .unwrap_or_else(|| args.item_id.as_slice());
            anyhow::ensure!(
                args.item_ids.is_none() || !item_ids.is_empty(),
                "Expected at least one item ID"
            );
            let mut review = azeroth_operator::forever_catalogue::review_owned(
                catalogue,
                &snapshot,
                args.offset.unwrap_or(0),
                args.limit.unwrap_or(20),
                args.item_type,
                args.set_id,
                item_ids,
                args.simulator_slot,
            )?;
            review["snapshot_id"] = serde_json::json!(args.snapshot_id);
            Ok::<_, anyhow::Error>(review)
        })();
        match result {
            Ok(review) => CallToolResult::success(vec![ContentBlock::text(review.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Owned gear review requires a valid Forever snapshot, a verified local catalogue and a limit from 1 to 50.",
            )]),
        }
    }
    #[tool(
        description = "Find a base item ID in captured equipment, bags and supported banks. Optionally search several explicitly selected snapshots separately, without a simulator catalogue.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn find_snapshot_item(&self, Parameters(args): Parameters<FindItem>) -> CallToolResult {
        let result = (|| {
            let additional = args.additional_snapshot_ids.unwrap_or_default();
            anyhow::ensure!(additional.len() <= 7, "Too many snapshots");
            let query = |id: &str, snapshot: &azeroth_operator::Snapshot| {
                let mut found = azeroth_operator::analysis::find_item(
                    snapshot,
                    args.item_id.get(),
                    args.offset.unwrap_or(0),
                    args.limit.unwrap_or(20),
                )?;
                found["snapshot_id"] = serde_json::json!(id);
                Ok::<_, anyhow::Error>(found)
            };
            let primary = self.store.read(&args.snapshot_id)?;
            let mut found = query(&args.snapshot_id, &primary)?;
            if !additional.is_empty() {
                let mut seen = std::collections::BTreeSet::from([args.snapshot_id.as_str()]);
                let mut extra = Vec::new();
                for id in &additional {
                    anyhow::ensure!(seen.insert(id), "Duplicate snapshot ID");
                    let snapshot = self.store.read(id)?;
                    anyhow::ensure!(snapshot.client == primary.client, "Client/build mismatch");
                    extra.push(query(id, &snapshot)?);
                }
                found["additional_snapshot_results"] = serde_json::json!(extra);
            }
            Ok::<_, anyhow::Error>(found)
        })();
        match result {
            Ok(found) => CallToolResult::success(vec![ContentBlock::text(found.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Item search requires up to eight distinct compatible snapshots and a limit from 1 to 50.",
            )]),
        }
    }
    #[tool(
        description = "Compare one to eight saved Forever recipes with observed bags and banks for 1 to 1000 crafts each. Multiple recipes share one compatible inventory snapshot. Offline estimate only.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn review_forever_recipe(&self, Parameters(args): Parameters<RecipeReview>) -> CallToolResult {
        let result = (|| {
            let additional = args.additional_recipes.unwrap_or_default();
            anyhow::ensure!(additional.len() <= 7, "Too many recipes");
            let snapshot = self.store.read(&args.snapshot_id)?;
            let inventory = args
                .inventory_snapshot_id
                .as_deref()
                .map(|id| self.store.read(id))
                .transpose()?;
            let mut review = azeroth_operator::analysis::plan_recipe(
                &snapshot,
                args.crafts,
                args.target_output_quantity,
                inventory.as_ref(),
            )?;
            if let Some(id) = args.inventory_snapshot_id {
                review["inventory_snapshot_id"] = serde_json::json!(id);
            }
            if !additional.is_empty() {
                let stock = inventory.as_ref().unwrap_or(&snapshot);
                let mut extra_reviews = Vec::new();
                let mut recipe_ids =
                    std::collections::BTreeSet::from([review["recipe"]["recipe_id"].as_u64()]);
                for entry in additional {
                    let saved = self.store.read(&entry.snapshot_id)?;
                    let mut extra = azeroth_operator::analysis::plan_recipe(
                        &saved,
                        entry.crafts,
                        entry.target_output_quantity,
                        Some(stock),
                    )?;
                    anyhow::ensure!(
                        recipe_ids.insert(extra["recipe"]["recipe_id"].as_u64()),
                        "Duplicate recipe ID"
                    );
                    extra["snapshot_id"] = serde_json::json!(entry.snapshot_id);
                    extra_reviews.push(extra);
                }
                review["combined_materials"] =
                    azeroth_operator::analysis::combine_recipe_materials(
                        std::iter::once(&review).chain(&extra_reviews),
                    )?;
                review["ordered_starting_materials"] =
                    azeroth_operator::analysis::ordered_recipe_materials(
                        std::iter::once(&review).chain(&extra_reviews),
                        &review["combined_materials"],
                    )?;
                review["combined_base_output_if_completed"] =
                    azeroth_operator::analysis::combine_recipe_outputs(
                        std::iter::once(&review).chain(&extra_reviews),
                    );
                review["additional_recipe_reviews"] = serde_json::json!(extra_reviews);
                if inventory.is_none() {
                    review["inventory_snapshot_id"] = serde_json::json!(args.snapshot_id);
                }
            }
            Ok::<_, anyhow::Error>(review)
        })();
        match result {
            Ok(review) => CallToolResult::success(vec![ContentBlock::text(review.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Recipe review requires up to eight distinct saved Forever recipes and a compatible inventory capture. Supply crafts or a supported target output quantity per recipe, within 1 to 1000 crafts.",
            )]),
        }
    }
    #[tool(
        description = "Compile an explicit Retail macro create/edit/delete/scope move into data for player review and application in the addon. Existing macros are bound to captured name, icon and body. Does not execute in game or relocate action-bar references.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn prepare_macro_operation(
        &self,
        Parameters(args): Parameters<MacroOperation>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            azeroth_operator::macro_operation::compile(
                &snapshot,
                &args.snapshot_id,
                &args.operation,
            )
        })();
        match result {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Macro operation rejected: snapshot, client, source or requested fields invalid",
            )]),
        }
    }
    #[tool(
        description = "Return the original captured Retail talent import string for player review, if available. Does not apply talents, create optimal builds or verify game acceptance.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn captured_talent_loadout(&self, Parameters(args): Parameters<Summary>) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            let mut review = azeroth_operator::planning::captured_loadout(&snapshot)?;
            review["snapshot_id"] = serde_json::json!(args.snapshot_id);
            Ok::<_, anyhow::Error>(review)
        })();
        match result {
            Ok(review) => CallToolResult::success(vec![ContentBlock::text(review.to_string())]),
            Err(_) => {
                CallToolResult::error(vec![ContentBlock::text("Snapshot unavailable or invalid")])
            }
        }
    }
    #[tool(
        description = "Review player-selected equipment from exact exported locations. Returns a JSON draft; does not equip, optimise, validate game legality or generate a game import.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn review_equipment_selection(
        &self,
        Parameters(args): Parameters<EquipmentReview>,
    ) -> CallToolResult {
        let result = (|| {
            let snapshot = self.store.read(&args.snapshot_id)?;
            anyhow::ensure!(
                !args.selections.is_empty() || self.forever_catalogue.is_some(),
                "An empty baseline requires a local Forever catalogue"
            );
            anyhow::ensure!(
                !args.allow_two_hand_off_hand_clear
                    || (snapshot.client.family == Family::Forever
                        && self.forever_catalogue.is_some()
                        && args
                            .selections
                            .iter()
                            .any(|selection| selection.target_slot == 16)
                        && args
                            .selections
                            .iter()
                            .all(|selection| selection.target_slot != 17)),
                "Clearing the off-hand requires a Forever catalogue and a main-hand selection without an off-hand selection"
            );
            let mut review =
                azeroth_operator::planning::equipment_review(&snapshot, &args.selections)?;
            if snapshot.client.family == Family::Forever
                && let Some(path) = &self.forever_catalogue
            {
                let checks = azeroth_operator::forever_catalogue::review_equipment_catalogue(
                    path,
                    &snapshot,
                    &args.selections,
                    args.allow_two_hand_off_hand_clear,
                )
                .or_else(|error| {
                    if args.selections.is_empty() || args.allow_two_hand_off_hand_clear {
                        return Err(error);
                    }
                    Ok(serde_json::json!({
                        "catalogue_two_hand_pairing":"unknown",
                        "catalogue_slot_fit":null,
                        "catalogue_requirement_conflicts":null,
                        "catalogue_unique_item_duplicates":null,
                        "catalogue_shared_limit_categories":null,
                        "simulator_equipment_spec":null,
                        "simulator_equipment_unresolved_slots":null
                    }))
                })?;
                review["checks"]
                    .as_object_mut()
                    .unwrap()
                    .extend(checks.as_object().unwrap().clone());
            }
            review["snapshot_id"] = serde_json::json!(args.snapshot_id);
            Ok::<_, anyhow::Error>(review)
        })();
        match result {
            Ok(review) => CallToolResult::success(vec![ContentBlock::text(review.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Equipment review rejected: check the snapshot, selected locations, projection options and Forever catalogue",
            )]),
        }
    }
    #[tool(
        description = "Compare same-character same-client snapshots, with optional item, quest, profession, Forever wallet, currency, reputation and tracked achievement comparisons.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn compare_snapshots(&self, Parameters(args): Parameters<Compare>) -> CallToolResult {
        let result = (|| {
            let a = self.store.read(&args.before_id)?;
            let b = self.store.read(&args.after_id)?;
            azeroth_operator::analysis::compare_observations(
                &a,
                &b,
                azeroth_operator::analysis::ComparisonSelectors {
                    item_id: args.item_id.map(|id| id.get()),
                    currency_id: args.currency_id.map(|id| id.get()),
                    faction_id: args.faction_id.map(|id| id.get()),
                    quest_id: args.quest_id.map(|id| id.get()),
                    achievement_id: args.achievement_id.map(|id| id.get()),
                    profession_skill_line: args.profession_skill_line.map(|id| id.get()),
                    wallet: args.wallet,
                },
            )
        })();
        match result {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Snapshots unavailable, reversed or incompatible",
            )]),
        }
    }
    #[tool(
        description = "Summarise observed character data; missing values remain unknown",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn character_summary(&self, Parameters(args): Parameters<Summary>) -> CallToolResult {
        match self.store.read(&args.snapshot_id) {
            Ok(s) => CallToolResult::success(vec![ContentBlock::text(
                azeroth_operator::analysis::summary(&s).to_string(),
            )]),
            Err(_) => {
                CallToolResult::error(vec![ContentBlock::text("Snapshot unavailable or invalid")])
            }
        }
    }
    #[tool(
        description = "List saved snapshots for an explicit client family and optional character name plus realm, newest capture first. Forever listings can find captures of selected recipes by output item ID. Does not identify the character currently in play.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn list_snapshots(&self, Parameters(args): Parameters<List>) -> CallToolResult {
        let family = match args.family.as_str() {
            "retail" => Family::Retail,
            "classic" => Family::Classic,
            "forever" => Family::Forever,
            _ => return CallToolResult::error(vec![ContentBlock::text("Unknown family")]),
        };
        match self.store.list_matching_recipe_output(
            family,
            args.character.as_ref(),
            args.recipe_output_item_id,
            args.offset.unwrap_or(0),
            args.limit.unwrap_or(20),
        ) {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Snapshot listing failed: check the character, recipe output filter and limit",
            )]),
        }
    }
    #[tool(
        description = "Read a stored snapshot by hash, with capture age. Select one quest by ID for remaining objectives, or compare explicit Forever costs against saved gold, currencies and carried items. Never live state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn inspect_snapshot(&self, Parameters(args): Parameters<Inspect>) -> CallToolResult {
        if let Some(budget) = args.spending_budget {
            if args.section.is_some() || args.quest_id.is_some() {
                return CallToolResult::error(vec![ContentBlock::text(
                    "Spending budgets cannot be combined with section or quest selection",
                )]);
            }
            return match self.review_spending_budget(&args.snapshot_id, budget) {
                Ok(value) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
                Err(_) => CallToolResult::error(vec![ContentBlock::text(
                    "Spending budget requires a saved Forever snapshot and bounded distinct costs",
                )]),
            };
        }
        if args.quest_id.is_some() && args.section.as_deref() != Some("quests") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest selection requires the quests section",
            )]);
        }
        match self
            .store
            .inspect(&args.snapshot_id, args.section.as_deref())
        {
            Ok(mut value) => {
                if let Some(id) = args.quest_id {
                    let quests = value["data"].take();
                    let quest = quests.get(id.to_string());
                    value["quest_id"] = serde_json::json!(id.get());
                    value["quest_observed"] =
                        serde_json::json!(quests.as_object().map(|_| quest.is_some()));
                    value["unfinished_objectives"] =
                        azeroth_operator::analysis::unfinished_quest_objectives(quest);
                    value["data"] = quest.cloned().unwrap_or(serde_json::Value::Null);
                }
                if args.section.as_deref() == Some("currency_balances") {
                    value["currency_cap_headroom"] =
                        azeroth_operator::analysis::currency_cap_headroom(&value["data"]);
                } else if args.section.as_deref() == Some("reputation") {
                    value["standard_reputation_progress"] =
                        azeroth_operator::analysis::standard_reputation_progress(&value["data"]);
                    value["special_reputation_progress"] =
                        azeroth_operator::analysis::special_reputation_progress(&value["data"]);
                } else if args.section.as_deref() == Some("tracked_achievements") {
                    value["tracked_achievement_work"] =
                        azeroth_operator::analysis::tracked_achievement_work(&value["data"]);
                } else if args.section.as_deref() == Some("professions") {
                    value["profession_training_progress"] =
                        azeroth_operator::analysis::profession_training_progress(&value["data"]);
                }
                CallToolResult::success(vec![ContentBlock::text(value.to_string())])
            }
            Err(_) => {
                CallToolResult::error(vec![ContentBlock::text("Snapshot unavailable or invalid")])
            }
        }
    }
}

impl Operator {
    fn review_spending_budget(
        &self,
        snapshot_id: &str,
        budget: SpendingBudget,
    ) -> anyhow::Result<serde_json::Value> {
        use serde_json::json;
        use std::collections::BTreeSet;

        let currencies = budget.currencies.unwrap_or_default();
        let items = budget.items.unwrap_or_default();
        anyhow::ensure!(
            currencies.len() <= 20
                && items.len() <= 20
                && (budget.wallet_copper.is_some() || !currencies.is_empty() || !items.is_empty()),
            "Expected at least one cost and at most 20 of each cost type"
        );
        let mut seen = BTreeSet::new();
        anyhow::ensure!(
            currencies.iter().all(|cost| seen.insert(cost.currency_id)),
            "Currency IDs must be distinct"
        );
        seen.clear();
        anyhow::ensure!(
            items.iter().all(|cost| seen.insert(cost.item_id)),
            "Item IDs must be distinct"
        );
        let snapshot = self.store.read(snapshot_id)?;
        anyhow::ensure!(
            snapshot.client.family == Family::Forever,
            "Expected Forever"
        );
        let mut known_shortage = false;
        let mut unknown = false;
        let mut compare = |required: u64, observed: Option<u64>| {
            let shortfall = observed.map(|quantity| required.saturating_sub(quantity));
            known_shortage |= shortfall.is_some_and(|quantity| quantity > 0);
            unknown |= shortfall.is_none();
            json!({"required":required,"observed":observed,"shortfall":shortfall})
        };
        let wallet = budget.wallet_copper.map(|cost| {
            let observed = snapshot
                .sections
                .get("wallet")
                .and_then(|row| row["copper"].as_u64());
            compare(cost.get(), observed)
        });
        let currencies: Vec<_> = currencies
            .into_iter()
            .map(|cost| {
                let row = snapshot
                    .sections
                    .get("currency_balances")
                    .and_then(|section| section["entries"].get(cost.currency_id.to_string()));
                let mut result = compare(
                    cost.quantity.get(),
                    row.and_then(|entry| entry["quantity"].as_u64()),
                );
                result["currency_id"] = json!(cost.currency_id);
                result["account_wide"] =
                    json!(row.and_then(|entry| entry["account_wide"].as_bool()));
                result["inactive"] = json!(row.and_then(|entry| entry["inactive"].as_bool()));
                result
            })
            .collect();
        let mut item_balances = Vec::new();
        for cost in items {
            let found = azeroth_operator::analysis::find_item(&snapshot, cost.item_id.get(), 0, 1)?;
            let stock = &found["quantity_by_storage"];
            let mut result = compare(cost.quantity.get(), stock["bags"].as_u64());
            result["item_id"] = json!(cost.item_id);
            result["observed_bank_quantities"] = json!({
                "character_bank":stock["character_bank"],"account_bank":stock["account_bank"]
            });
            item_balances.push(result);
        }
        let covered_by_observed_balances = if known_shortage {
            Some(false)
        } else if unknown {
            None
        } else {
            Some(true)
        };
        let mut result = json!({
            "snapshot_id":snapshot_id,"captured_at":snapshot.captured_at,"live":false,
            "age_seconds":(chrono::Utc::now()-snapshot.captured_at).num_seconds(),
            "spending_budget":{"wallet_copper":wallet,"currencies":currencies,
                "covered_by_observed_balances":covered_by_observed_balances}
        });
        if !item_balances.is_empty() {
            result["spending_budget"]["items"] = json!(item_balances);
        }
        Ok(result)
    }

    fn add_restock_crafting(
        &self,
        inventory: &azeroth_operator::Snapshot,
        progress: &mut serde_json::Value,
        recipe_ids: Vec<String>,
    ) -> anyhow::Result<()> {
        use azeroth_operator::analysis::{combine_recipe_materials, plan_recipe};
        use serde_json::json;
        use std::collections::BTreeMap;
        use std::num::NonZeroU32;

        let mut shortages: BTreeMap<_, _> = progress["restock_plan"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| {
                Some((
                    row["item_id"].as_u64()?,
                    row["confirmed_shortfall"].as_u64()?,
                ))
            })
            .collect();
        let mut reviews = Vec::new();
        let mut plans = Vec::new();
        for id in recipe_ids {
            let saved = self.store.read(&id)?;
            let recipe = saved
                .sections
                .get("selected_recipe")
                .ok_or_else(|| anyhow::anyhow!("Missing selected recipe"))?;
            anyhow::ensure!(
                recipe["learned"].as_bool() == Some(true)
                    && recipe["requirements"] == "fixed_items",
                "Expected a learned fixed-material recipe"
            );
            let item = recipe["output_item_id"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing recipe output"))?;
            let quantity = shortages
                .remove(&item)
                .and_then(|quantity| u32::try_from(quantity).ok())
                .and_then(NonZeroU32::new)
                .ok_or_else(|| anyhow::anyhow!("Expected a distinct confirmed shortage"))?;
            let review = plan_recipe(&saved, None, Some(quantity), Some(inventory))?;
            plans.push(json!({
                "recipe_snapshot_id":id,"recipe_id":recipe["recipe_id"],"item_id":item,
                "target_output_quantity":quantity,"requested_crafts":review["requested_crafts"],
                "base_output_if_completed":review["base_output_if_completed"],
                "observed_cooldown":recipe["cooldown"],
                "shortfall_vs_observed_recipe_charges":review["shortfall_vs_observed_recipe_charges"],
                "unmet_crafting_conditions":review["unmet_crafting_conditions"]
            }));
            reviews.push(review);
        }
        let materials = combine_recipe_materials(reviews.iter())?;
        anyhow::ensure!(
            materials.as_array().into_iter().flatten().all(|material| {
                progress["restock_plan"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .all(|target| target["item_id"] != material["item_id"])
            }),
            "A restock target also serves as a reagent"
        );
        progress["crafting_plan"] = json!({
            "recipes":plans,"combined_materials":materials,"craftability_verified":false
        });
        Ok(())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Operator {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = std::env::var_os("AZEROTH_DATA_DIR").map(std::path::PathBuf::from);
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !args.is_empty() {
        if args.len() == 4 && args[0] == "sync" {
            let family = match args[1].to_str() {
                Some("retail") => Family::Retail,
                Some("classic") => Family::Classic,
                Some("forever") => Family::Forever,
                _ => anyhow::bail!("Unknown sync family"),
            };
            println!(
                "{}",
                azeroth_operator::sync::sync_file(
                    std::path::Path::new(&args[2]),
                    family,
                    std::path::Path::new(&args[3])
                )
                .await?
            );
            return Ok(());
        }
        if args.len() == 2 && args[0] == "upload" {
            let snapshot = azeroth_operator::import::import_file(std::path::Path::new(&args[1]))?;
            println!("{}", azeroth_operator::upload::upload(&snapshot).await?);
            return Ok(());
        }
        let snapshot = if args.len() == 1 && args[0] == "import-stdin" {
            azeroth_operator::import::import_reader(std::io::stdin().lock())?
        } else if args.len() == 2 && args[0] == "import" {
            azeroth_operator::import::import_file(std::path::Path::new(&args[1]))?
        } else {
            anyhow::bail!(
                "Usage: azeroth-operator [import <file> | import-stdin | upload <file> | sync <family> <file> <receipt-dir>]"
            );
        };
        let root = root.ok_or_else(|| {
            anyhow::anyhow!("AZEROTH_DATA_DIR must name the owner's absolute snapshot directory")
        })?;
        anyhow::ensure!(root.is_absolute(), "AZEROTH_DATA_DIR must be absolute");
        println!("{}", Store { root }.ingest(&snapshot)?);
        return Ok(());
    }
    let root = root.ok_or_else(|| {
        anyhow::anyhow!("AZEROTH_DATA_DIR must name the owner's absolute snapshot directory")
    })?;
    anyhow::ensure!(root.is_absolute(), "AZEROTH_DATA_DIR must be absolute");
    let root = root
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("Owner snapshot directory unavailable"))?;
    anyhow::ensure!(root.is_dir(), "Owner snapshot directory unavailable");
    let forever_catalogue =
        std::env::var_os("AZEROTH_FOREVER_CATALOGUE").map(std::path::PathBuf::from);
    anyhow::ensure!(
        forever_catalogue
            .as_ref()
            .is_none_or(|path| path.is_absolute()),
        "AZEROTH_FOREVER_CATALOGUE must be an absolute path"
    );
    let mut tool_router = Operator::tool_router();
    for tool in tool_router.list_all() {
        if !OFFLINE_TOOLS.contains(&tool.name.as_ref())
            || (matches!(
                tool.name.as_ref(),
                "lookup_forever_catalogue" | "review_forever_owned_gear"
            ) && forever_catalogue.is_none())
        {
            tool_router.remove_route(&tool.name);
        }
    }
    let service = Operator {
        store: Store { root },
        forever_catalogue,
        tool_router,
    }
    .serve(rmcp::transport::stdio())
    .await?;
    service.waiting().await?;
    Ok(())
}
