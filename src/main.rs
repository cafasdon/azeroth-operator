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
    /// Compare recorded fares with the wallet from the same Forever flight_map capture.
    #[serde(default)]
    flight_wallet_projection: bool,
    /// Budget all lessons marked available in the saved Forever trainer list.
    #[serde(default)]
    trainer_wallet_projection: bool,
    /// Budget available pet trainer lessons against saved training points.
    #[serde(default)]
    trainer_points_projection: bool,
    /// Case-insensitive substring of a saved trainer lesson name.
    trainer_service_name: Option<String>,
    /// Saved trainer lesson status: available, unavailable, or used.
    trainer_service_status: Option<String>,
    /// Review a saved merchant offer by its 1-based goods-tab slot.
    merchant_offer_slot: Option<std::num::NonZeroU32>,
    /// Requested bundles for the saved merchant offer. Defaults to one.
    merchant_bundles: Option<std::num::NonZeroU32>,
    /// Find saved merchant offers that sell this exact item ID.
    merchant_item_id: Option<std::num::NonZeroU32>,
    /// Find saved merchant offers that sell this exact currency ID.
    merchant_currency_id: Option<std::num::NonZeroU32>,
    /// Select one saved quest by ID. Requires section "quests".
    quest_id: Option<std::num::NonZeroU32>,
    /// Compare captured quest currency rewards against caps from the same snapshot.
    #[serde(default)]
    quest_currency_projection: bool,
    /// Resolve captured quest reward items and recipe spell references in the pinned local Forever catalogue.
    #[serde(default)]
    quest_item_catalogue: bool,
    /// Compare a saved quest's coin cost and reward with the optional captured wallet.
    #[serde(default)]
    quest_wallet_projection: bool,
    /// Compare captured quest renown rewards with saved faction progress.
    #[serde(default)]
    quest_renown_projection: bool,
    /// Compare the saved quest XP reward with the character's captured XP threshold.
    #[serde(default)]
    quest_xp_projection: bool,
    /// Filter saved Forever quests by their next waypoint map. Requires section "quests".
    quest_waypoint_map_id: Option<std::num::NonZeroU32>,
    /// Filter quest waypoints to the player map recorded in the same Forever snapshot.
    #[serde(default)]
    quest_on_captured_map: bool,
    /// Filter saved Forever quests to observed elite or suggested-group quests.
    quest_difficulty: Option<QuestDifficulty>,
    /// Filter saved Forever quests by captured turn-in readiness.
    quest_readiness: Option<azeroth_operator::analysis::QuestReadiness>,
    /// Keep saved Forever quests with at most this many seconds remaining at capture.
    quest_max_remaining_seconds: Option<u32>,
    /// Keep saved Forever quests that offer this base item as a fixed reward or choice.
    quest_reward_item_id: Option<std::num::NonZeroU32>,
    /// Keep saved Forever quests that offer this currency as a fixed reward or choice.
    quest_reward_currency_id: Option<std::num::NonZeroU32>,
    /// Keep saved Forever quests with a recorded major-faction reward for this faction.
    quest_reward_faction_id: Option<std::num::NonZeroU32>,
    /// Keep saved Forever quests with this recorded reward spell.
    quest_reward_spell_id: Option<std::num::NonZeroU32>,
    /// Select one saved Forever currency by ID. Requires section "currency_balances".
    currency_id: Option<std::num::NonZeroU32>,
    /// Compare a proposed reward against this currency's recorded cap headroom.
    currency_reward_quantity: Option<std::num::NonZeroU64>,
    /// Select one saved Forever faction by ID. Requires section "reputation".
    faction_id: Option<std::num::NonZeroU32>,
    /// Select one saved Forever tracked achievement by ID. Requires section "tracked_achievements".
    achievement_id: Option<std::num::NonZeroU32>,
    /// Select one saved profession by skill line. Requires section "professions".
    profession_skill_line: Option<std::num::NonZeroU32>,
    /// Select one saved Forever recipe ID in the tracked_recipes section.
    tracked_recipe_id: Option<std::num::NonZeroU32>,
    /// Find this base or reported spell ID in the saved Forever spellbook section.
    spellbook_spell_id: Option<std::num::NonZeroU32>,
    /// Keep saved spellbook slots with this observed passive flag.
    spellbook_passive: Option<bool>,
    /// Keep saved spellbook slots with this observed off-spec flag.
    spellbook_off_spec: Option<bool>,
    /// Match a case-insensitive substring of the captured spell name.
    spellbook_name: Option<String>,
    /// Match observed built-in action bar spells to the same saved Forever spellbook.
    #[serde(default)]
    action_bar_spellbook: bool,
    /// Select one captured Forever built-in action slot, numbered 1 to 4096.
    action_bar_slot: Option<std::num::NonZeroU32>,
    /// Match an exact key on captured Forever built-in action buttons.
    action_bar_hotkey: Option<String>,
    /// Filter saved tracked crafts and recrafts by observed learned status.
    tracked_recipe_learning: Option<RecipeLearning>,
    /// Compare an explicit basket of costs against one saved Forever inventory.
    spending_budget: Option<SpendingBudget>,
}

#[derive(Clone, Copy, Deserialize, serde::Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum QuestDifficulty {
    Elite,
    SuggestedGroup,
}

impl QuestDifficulty {
    fn matches(self, quest: &serde_json::Value) -> Option<bool> {
        match self {
            Self::Elite => quest["elite"].as_bool(),
            Self::SuggestedGroup => quest["suggested_group_size"].as_u64().map(|size| size > 1),
        }
    }
}

#[derive(Clone, Copy, Deserialize, serde::Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum RecipeLearning {
    Learned,
    Unlearned,
    Unknown,
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
    /// Filter Forever snapshots by the captured selected recipe's profession ID.
    recipe_profession_id: Option<std::num::NonZeroU32>,
    /// Filter Forever snapshots by the exact captured selected recipe ID.
    recipe_id: Option<std::num::NonZeroU32>,
    /// Filter saved Forever instance context by kind, including "none" for outside an instance.
    instance_kind: Option<String>,
    /// Filter saved Forever instance context by exact, case-insensitive name.
    instance_name: Option<String>,
    /// Filter saved Forever instance context by its observed difficulty ID.
    instance_difficulty_id: Option<u32>,
    /// Filter by the saved Forever instance map ID, independent of the localized name.
    instance_map_id: Option<std::num::NonZeroU32>,
    /// Find Forever snapshots whose captured merchant sold this exact item ID.
    merchant_item_id: Option<std::num::NonZeroU32>,
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
    /// Compare next waypoint maps for quests present in both Forever captures.
    #[serde(default)]
    quest_waypoints: bool,
    /// Optionally compare one tracked Forever achievement by ID.
    achievement_id: Option<std::num::NonZeroU32>,
    /// Optionally compare one captured profession by its skill-line ID.
    profession_skill_line: Option<std::num::NonZeroU32>,
    /// Include a net change in captured Forever wallet copper.
    #[serde(default)]
    wallet: bool,
    /// Compare captured Forever experience and level observations.
    #[serde(default)]
    experience: bool,
    /// Compare saved Forever spellbook observations by base spell ID.
    #[serde(default)]
    spellbook: bool,
    /// Compare loaded built-in action bar observations in two Forever captures.
    #[serde(default)]
    action_bar_slots: bool,
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
    /// Minimum recorded character level required to equip the item, inclusive.
    minimum_required_level: Option<u32>,
    /// Maximum recorded character level required to equip the item, inclusive.
    maximum_required_level: Option<u32>,
    /// Filter by the exact nonzero enchant effect ID encoded in a captured item link.
    observed_enchant_effect_id: Option<std::num::NonZeroU32>,
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
    /// Review up to seven more Forever drafts against this same snapshot.
    additional_drafts: Option<Vec<EquipmentDraft>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EquipmentDraft {
    selections: Vec<azeroth_operator::planning::EquipmentSelection>,
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
                if matches!(
                    query.section,
                    azeroth_operator::forever_catalogue::Section::Enchants
                ) {
                    data["recipe_observation"] = serde_json::json!({"snapshot_id": id});
                    return Ok(data);
                }
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
                (args.minimum_required_level, args.maximum_required_level),
                args.observed_enchant_effect_id,
            )?;
            review["snapshot_id"] = serde_json::json!(args.snapshot_id);
            Ok::<_, anyhow::Error>(review)
        })();
        match result {
            Ok(review) => CallToolResult::success(vec![ContentBlock::text(review.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Owned gear review requires a valid Forever snapshot, a verified local catalogue, a limit from 1 to 50 and ordered level bounds.",
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
        description = "Review player-selected equipment from exact exported locations. Forever reviews can batch up to eight drafts against the same snapshot. Does not equip, optimise, validate game legality or generate a game import.",
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
            let additional = args.additional_drafts.unwrap_or_default();
            anyhow::ensure!(
                additional.len() <= 7
                    && (additional.is_empty()
                        || (snapshot.client.family == Family::Forever
                            && self.forever_catalogue.is_some())),
                "Additional drafts require a Forever catalogue and a bounded batch"
            );
            let catalogue = self
                .forever_catalogue
                .as_deref()
                .filter(|_| snapshot.client.family == Family::Forever)
                .map(azeroth_operator::forever_catalogue::EquipmentCatalogue::load);
            let mut review = self.review_equipment_draft(
                &snapshot,
                &args.snapshot_id,
                &args.selections,
                args.allow_two_hand_off_hand_clear,
                catalogue.as_ref(),
            )?;
            if !additional.is_empty() {
                let extra = additional
                    .into_iter()
                    .map(|draft| {
                        self.review_equipment_draft(
                            &snapshot,
                            &args.snapshot_id,
                            &draft.selections,
                            draft.allow_two_hand_off_hand_clear,
                            catalogue.as_ref(),
                        )
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                review["additional_draft_reviews"] = serde_json::json!(extra);
            }
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
        description = "Compare same-character same-client snapshots, with optional item, quest, profession, Forever wallet, currency, reputation and tracked achievement comparisons. Forever quest comparisons can include waypoint map changes.",
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
                    quest_waypoints: args.quest_waypoints,
                    achievement_id: args.achievement_id.map(|id| id.get()),
                    profession_skill_line: args.profession_skill_line.map(|id| id.get()),
                    wallet: args.wallet,
                    experience: args.experience,
                    spellbook: args.spellbook,
                    action_bar_slots: args.action_bar_slots,
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
        description = "List saved snapshots for an explicit client family and optional character name plus realm, newest capture first. Forever listings can filter by captured recipe, instance context or merchant item. Does not identify the character currently in play.",
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
        match self.store.list_filtered(
            family,
            args.character.as_ref(),
            azeroth_operator::CaptureFilters {
                recipe: azeroth_operator::RecipeCaptureFilters {
                    recipe_id: args.recipe_id,
                    output_item_id: args.recipe_output_item_id,
                    profession_id: args.recipe_profession_id,
                },
                instance: azeroth_operator::InstanceCaptureFilters {
                    kind: args.instance_kind.as_deref(),
                    name: args.instance_name.as_deref(),
                    difficulty_id: args.instance_difficulty_id,
                    map_id: args.instance_map_id,
                },
                merchant_item_id: args.merchant_item_id,
            },
            args.offset.unwrap_or(0),
            args.limit.unwrap_or(20),
        ) {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Snapshot listing failed: check the character, recipe, instance or merchant filters and limit",
            )]),
        }
    }
    #[tool(
        description = "Read a stored snapshot by hash, with capture age. Select a quest, currency, faction, tracked achievement or profession by ID, compare a currency reward against recorded caps, or check costs against saved balances. Never live state.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn inspect_snapshot(&self, Parameters(args): Parameters<Inspect>) -> CallToolResult {
        let trainer_name = args.trainer_service_name.as_deref().map(str::trim);
        let trainer_status = args.trainer_service_status.as_deref();
        let filter_trainer = trainer_name.is_some() || trainer_status.is_some();
        let review_trainer =
            args.trainer_wallet_projection || args.trainer_points_projection || filter_trainer;
        if review_trainer && args.section.as_deref() != Some("trainer_services") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Trainer review requires the trainer_services section",
            )]);
        }
        if trainer_name.is_some_and(|name| name.is_empty() || name.len() > 256)
            || trainer_status
                .is_some_and(|status| !matches!(status, "available" | "unavailable" | "used"))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Invalid trainer name or status filter",
            )]);
        }
        let trainer_name = trainer_name.map(str::to_lowercase);
        let merchant_search = args
            .merchant_item_id
            .map(|id| ("item_id", id))
            .or_else(|| args.merchant_currency_id.map(|id| ("currency_id", id)));
        if merchant_search.is_some()
            && (args.section.as_deref() != Some("merchant_stock")
                || args.merchant_offer_slot.is_some()
                || (args.merchant_item_id.is_some() && args.merchant_currency_id.is_some()))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Merchant search requires merchant_stock and exactly one item or currency ID without a slot",
            )]);
        }
        if (args.merchant_offer_slot.is_some() && args.section.as_deref() != Some("merchant_stock"))
            || args
                .merchant_offer_slot
                .is_some_and(|slot| slot.get() > 256)
            || args
                .merchant_bundles
                .is_some_and(|bundles| args.merchant_offer_slot.is_none() || bundles.get() > 1000)
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Merchant review requires merchant_stock, a slot from 1 to 256 and up to 1000 bundles",
            )]);
        }
        if args.flight_wallet_projection && args.section.as_deref() != Some("flight_map") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Flight wallet projection requires the flight_map section",
            )]);
        }
        let reward_filters: [(_, _, _, &[&str]); 4] = [
            (
                "item",
                args.quest_reward_item_id,
                "item_id",
                &["item_rewards", "item_choices"],
            ),
            (
                "currency",
                args.quest_reward_currency_id,
                "currency_id",
                &["currency_rewards", "currency_choices"],
            ),
            (
                "faction",
                args.quest_reward_faction_id,
                "faction_id",
                &["renown_rewards"],
            ),
            (
                "spell",
                args.quest_reward_spell_id,
                "spell_id",
                &["spell_rewards"],
            ),
        ];
        let filter_rewards = reward_filters.iter().any(|(_, id, _, _)| id.is_some());
        let filter_quest_map = args.quest_waypoint_map_id.is_some() || args.quest_on_captured_map;
        if args.quest_waypoint_map_id.is_some() && args.quest_on_captured_map {
            return CallToolResult::error(vec![ContentBlock::text(
                "Choose either an explicit waypoint map or the captured map",
            )]);
        }
        let filter_spellbook = args.spellbook_spell_id.is_some()
            || args.spellbook_passive.is_some()
            || args.spellbook_off_spec.is_some()
            || args.spellbook_name.is_some();
        let action_bar_hotkey = args.action_bar_hotkey.as_deref().map(str::trim);
        if action_bar_hotkey.is_some_and(|key| key.is_empty() || key.len() > 128) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Action bar hotkey requires 1 to 128 UTF-8 bytes",
            )]);
        }
        let review_action_bar = args.action_bar_spellbook
            || args.action_bar_slot.is_some()
            || action_bar_hotkey.is_some();
        if review_action_bar && args.section.as_deref() != Some("action_bar_observation") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Action bar slot and spellbook review require the action_bar_observation section",
            )]);
        }
        if args.action_bar_slot.is_some_and(|slot| slot.get() > 4096) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Action bar slot must be 1 to 4096",
            )]);
        }
        let spellbook_name = args.spellbook_name.as_deref().map(str::trim);
        if spellbook_name.is_some_and(|name| name.is_empty() || name.len() > 256) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Spellbook name search requires 1 to 256 UTF-8 bytes",
            )]);
        }
        let spellbook_name = spellbook_name.map(str::to_lowercase);
        if args.quest_xp_projection
            && (args.quest_id.is_none() || args.section.as_deref() != Some("quests"))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest XP projection requires a quest ID and the quests section",
            )]);
        }
        if args.quest_renown_projection
            && (args.quest_id.is_none() || args.section.as_deref() != Some("quests"))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest renown projection requires a quest ID and the quests section",
            )]);
        }
        if args.quest_wallet_projection
            && (args.quest_id.is_none() || args.section.as_deref() != Some("quests"))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest wallet projection requires a quest ID and the quests section",
            )]);
        }
        if args.quest_currency_projection
            && (args.quest_id.is_none() || args.section.as_deref() != Some("quests"))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest currency projection requires a quest ID and the quests section",
            )]);
        }
        if args.quest_item_catalogue
            && (args.quest_id.is_none() || args.section.as_deref() != Some("quests"))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest item catalogue requires a quest ID and the quests section",
            )]);
        }
        if args.currency_reward_quantity.is_some() && args.currency_id.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Currency reward comparison requires a currency ID",
            )]);
        }
        if let Some(budget) = args.spending_budget {
            if args.section.is_some()
                || args.quest_id.is_some()
                || filter_quest_map
                || args.quest_difficulty.is_some()
                || args.quest_readiness.is_some()
                || args.quest_max_remaining_seconds.is_some()
                || filter_rewards
                || args.currency_id.is_some()
                || args.faction_id.is_some()
                || args.achievement_id.is_some()
                || args.profession_skill_line.is_some()
                || args.tracked_recipe_id.is_some()
                || filter_spellbook
                || args.tracked_recipe_learning.is_some()
            {
                return CallToolResult::error(vec![ContentBlock::text(
                    "Spending budgets cannot be combined with a section or record selection",
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
        if filter_quest_map
            && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest waypoint map selection requires the quests section without a quest ID",
            )]);
        }
        if args.quest_difficulty.is_some()
            && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest difficulty selection requires the quests section without a quest ID",
            )]);
        }
        if args.quest_readiness.is_some()
            && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest readiness selection requires the quests section without a quest ID",
            )]);
        }
        if args.quest_max_remaining_seconds.is_some()
            && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest timer selection requires the quests section without a quest ID",
            )]);
        }
        if filter_rewards && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest reward selection requires the quests section without a quest ID",
            )]);
        }
        if args.currency_id.is_some() && args.section.as_deref() != Some("currency_balances") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Currency selection requires the currency_balances section",
            )]);
        }
        if args.faction_id.is_some() && args.section.as_deref() != Some("reputation") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Faction selection requires the reputation section",
            )]);
        }
        if args.achievement_id.is_some() && args.section.as_deref() != Some("tracked_achievements")
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Achievement selection requires the tracked_achievements section",
            )]);
        }
        if args.profession_skill_line.is_some() && args.section.as_deref() != Some("professions") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Profession selection requires the professions section",
            )]);
        }
        if filter_spellbook && args.section.as_deref() != Some("spellbook") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Spellbook selection requires the spellbook section",
            )]);
        }
        if args.tracked_recipe_id.is_some() && args.section.as_deref() != Some("tracked_recipes") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Tracked recipe selection requires the tracked_recipes section",
            )]);
        }
        if args.tracked_recipe_learning.is_some()
            && (args.section.as_deref() != Some("tracked_recipes")
                || args.tracked_recipe_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Tracked recipe learning filter requires tracked_recipes without a recipe ID",
            )]);
        }
        match self
            .store
            .inspect(&args.snapshot_id, args.section.as_deref())
        {
            Ok(mut value) => {
                if review_trainer && value["client"]["family"] != "forever" {
                    return CallToolResult::error(vec![ContentBlock::text(
                        "Trainer review requires a saved Forever snapshot",
                    )]);
                }
                if filter_trainer {
                    let count = value["data"]
                        .get_mut("entries")
                        .and_then(serde_json::Value::as_object_mut)
                        .map(|entries| {
                            entries.retain(|_, lesson| {
                                trainer_status
                                    .is_none_or(|status| lesson["status"].as_str() == Some(status))
                                    && trainer_name.as_ref().is_none_or(|search| {
                                        lesson["name"].as_str().is_some_and(|name| {
                                            name.to_lowercase().contains(search)
                                        })
                                    })
                            });
                            entries.len()
                        });
                    value["trainer_match_count"] = serde_json::json!(count);
                }
                if args.trainer_wallet_projection {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read the captured trainer wallet context",
                        )]);
                    };
                    value["trainer_wallet_projection"] =
                        azeroth_operator::analysis::trainer_wallet_projection(
                            &value["data"],
                            snapshot.sections.get("wallet"),
                        );
                }
                if args.trainer_points_projection {
                    value["trainer_points_projection"] =
                        azeroth_operator::analysis::trainer_points_projection(&value["data"]);
                }
                if let Some((field, id)) = merchant_search {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Merchant search requires a saved Forever snapshot",
                        )]);
                    }
                    let count = value["data"].as_object_mut().map(|offers| {
                        offers
                            .retain(|_, offer| offer[field].as_u64() == Some(u64::from(id.get())));
                        offers.len()
                    });
                    value["merchant_match_count"] = serde_json::json!(count);
                }
                if let Some(slot) = args.merchant_offer_slot {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Merchant review requires a saved Forever snapshot",
                        )]);
                    }
                    let offers = value["data"].take();
                    let offer = offers.get(slot.to_string());
                    value["merchant_offer_slot"] = serde_json::json!(slot.get());
                    value["merchant_offer_observed"] =
                        serde_json::json!(offers.as_object().map(|_| offer.is_some()));
                    if let Some(offer) = offer {
                        let projection = self.store.read(&args.snapshot_id).and_then(|snapshot| {
                            azeroth_operator::analysis::merchant_budget(
                                &snapshot,
                                offer,
                                args.merchant_bundles.map_or(1, std::num::NonZeroU32::get),
                            )
                        });
                        let Ok(projection) = projection else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Could not review captured merchant costs",
                            )]);
                        };
                        value["merchant_budget"] = projection;
                        value["data"] = offer.clone();
                    }
                }
                if args.flight_wallet_projection {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Flight wallet projection requires a saved Forever snapshot",
                        )]);
                    }
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read the captured flight wallet context",
                        )]);
                    };
                    value["flight_wallet_projection"] =
                        azeroth_operator::analysis::flight_wallet_projection(
                            &value["data"],
                            snapshot.sections.get("wallet"),
                        );
                }
                if let Some(id) = args.quest_id {
                    let quests = value["data"].take();
                    let quest = quests.get(id.to_string());
                    value["quest_id"] = serde_json::json!(id.get());
                    value["quest_observed"] =
                        serde_json::json!(quests.as_object().map(|_| quest.is_some()));
                    value["unfinished_objectives"] =
                        azeroth_operator::analysis::unfinished_quest_objectives(quest);
                    let review_snapshot = if args.quest_item_catalogue
                        || args.quest_currency_projection
                        || args.quest_wallet_projection
                        || args.quest_renown_projection
                        || args.quest_xp_projection
                    {
                        if value["client"]["family"] != "forever" {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Quest reward review requires a saved Forever snapshot",
                            )]);
                        }
                        match self.store.read(&args.snapshot_id) {
                            Ok(snapshot) => Some(snapshot),
                            Err(_) => {
                                return CallToolResult::error(vec![ContentBlock::text(
                                    "Could not read the captured quest reward context",
                                )]);
                            }
                        }
                    } else {
                        None
                    };
                    if args.quest_item_catalogue {
                        let catalogue = self.forever_catalogue.as_deref().and_then(|path| {
                            azeroth_operator::forever_catalogue::EquipmentCatalogue::load(path).ok()
                        });
                        value["quest_item_catalogue_available"] =
                            serde_json::json!(catalogue.is_some());
                        value["quest_item_catalogue"] = quest
                            .zip(catalogue.as_ref())
                            .zip(review_snapshot.as_ref())
                            .map_or(serde_json::Value::Null, |((quest, catalogue), snapshot)| {
                                catalogue.quest_item_references(quest, snapshot)
                            });
                    }
                    if let Some(snapshot) = review_snapshot
                        .as_ref()
                        .filter(|_| args.quest_currency_projection)
                    {
                        value["quest_currency_projection"] =
                            azeroth_operator::analysis::quest_currency_projection(
                                quest,
                                snapshot
                                    .sections
                                    .get("currency_balances")
                                    .unwrap_or(&serde_json::Value::Null),
                            );
                    }
                    if let Some(snapshot) = review_snapshot
                        .as_ref()
                        .filter(|_| args.quest_wallet_projection)
                    {
                        value["quest_wallet_projection"] =
                            azeroth_operator::analysis::quest_wallet_projection(
                                quest,
                                snapshot.sections.get("wallet"),
                            );
                    }
                    if let Some(snapshot) = review_snapshot
                        .as_ref()
                        .filter(|_| args.quest_renown_projection)
                    {
                        value["quest_renown_projection"] =
                            azeroth_operator::analysis::quest_renown_projection(
                                quest,
                                snapshot
                                    .sections
                                    .get("reputation")
                                    .unwrap_or(&serde_json::Value::Null),
                            );
                    }
                    if let Some(snapshot) = review_snapshot
                        .as_ref()
                        .filter(|_| args.quest_xp_projection)
                    {
                        value["quest_xp_projection"] =
                            azeroth_operator::analysis::quest_xp_projection(
                                quest,
                                snapshot.sections.get("experience"),
                            );
                    }
                    value["data"] = quest.cloned().unwrap_or(serde_json::Value::Null);
                }
                if filter_quest_map
                    || args.quest_difficulty.is_some()
                    || args.quest_readiness.is_some()
                    || args.quest_max_remaining_seconds.is_some()
                    || filter_rewards
                {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Quest filters require a saved Forever snapshot",
                        )]);
                    }
                    let mut data = value["data"].take();
                    let map = if args.quest_on_captured_map {
                        let snapshot = match self.store.read(&args.snapshot_id) {
                            Ok(snapshot) => snapshot,
                            Err(_) => {
                                return CallToolResult::error(vec![ContentBlock::text(
                                    "Could not read the captured player map",
                                )]);
                            }
                        };
                        let map = snapshot
                            .sections
                            .get("stats")
                            .and_then(|stats| stats["map_id"].as_u64());
                        value["quest_on_captured_map"] = serde_json::json!(true);
                        if map.is_none() {
                            data = serde_json::Value::Null;
                        }
                        map
                    } else {
                        args.quest_waypoint_map_id.map(|map| u64::from(map.get()))
                    };
                    if filter_quest_map {
                        value["quest_waypoint_map_id"] = serde_json::json!(map);
                        value["quest_waypoint_map_unknown_count"] =
                            serde_json::json!(data.as_object_mut().map(|quests| {
                                let unknown = quests
                                    .values()
                                    .filter(|quest| quest["next_waypoint_map_id"].is_null())
                                    .count();
                                quests.retain(|_, quest| {
                                    quest["next_waypoint_map_id"].as_u64() == map
                                });
                                unknown
                            }));
                    }
                    if let Some(difficulty) = args.quest_difficulty {
                        value["quest_difficulty"] = serde_json::json!(difficulty);
                        value["quest_difficulty_unknown_count"] =
                            serde_json::json!(data.as_object_mut().map(|quests| {
                                let mut unknown = 0;
                                quests.retain(|_, quest| {
                                    let matched = difficulty.matches(quest);
                                    unknown += usize::from(matched.is_none());
                                    matched == Some(true)
                                });
                                unknown
                            }));
                    }
                    if let Some(readiness) = args.quest_readiness {
                        value["quest_readiness"] = serde_json::json!(readiness);
                        if let Some(quests) = data.as_object_mut() {
                            quests.retain(|_, quest| {
                                azeroth_operator::analysis::quest_readiness(quest) == readiness
                            });
                        }
                    }
                    if let Some(maximum) = args.quest_max_remaining_seconds {
                        value["quest_max_remaining_seconds"] = serde_json::json!(maximum);
                        value["quest_timer_unknown_count"] =
                            serde_json::json!(data.as_object_mut().map(|quests| {
                                let mut unknown = 0;
                                quests.retain(|_, quest| {
                                    let remaining = quest["timer"]["remaining_seconds"].as_u64();
                                    unknown += usize::from(remaining.is_none());
                                    remaining.is_some_and(|seconds| seconds <= u64::from(maximum))
                                });
                                unknown
                            }));
                    }
                    for (kind, id, id_field, fields) in reward_filters {
                        let Some(id) = id else { continue };
                        value[format!("quest_reward_{kind}_id")] = serde_json::json!(id);
                        value[format!("quest_reward_{kind}_unknown_count")] =
                            serde_json::json!(data.as_object_mut().map(|quests| {
                                let mut unknown = 0;
                                quests.retain(|_, quest| {
                                    let lists =
                                        fields.iter().map(|field| quest[*field].as_object());
                                    let matched = lists.clone().flatten().any(|rows| {
                                        rows.values().any(|row| {
                                            row[id_field].as_u64() == Some(u64::from(id.get()))
                                        })
                                    });
                                    unknown += usize::from(
                                        !matched && lists.into_iter().any(|list| list.is_none()),
                                    );
                                    matched
                                });
                                unknown
                            }));
                    }
                    value["quest_checklist"] =
                        data.as_object().map_or(serde_json::Value::Null, |quests| {
                            azeroth_operator::analysis::quest_checklist(quests)
                        });
                    value["data"] = data;
                }
                if let Some(id) = args.tracked_recipe_id {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Tracked recipe selection requires a saved Forever snapshot",
                        )]);
                    }
                    let section = value["data"].take();
                    let key = id.to_string();
                    let crafts = section["crafts"].as_object();
                    let recrafts = section["recrafts"].as_object();
                    let craft = crafts.and_then(|rows| rows.get(&key));
                    let recraft = recrafts.and_then(|rows| rows.get(&key));
                    value["tracked_recipe_id"] = serde_json::json!(id);
                    value["coverage"] = section["coverage"].clone();
                    value["tracked_as_craft"] = serde_json::json!(crafts.map(|_| craft.is_some()));
                    value["tracked_as_recraft"] =
                        serde_json::json!(recrafts.map(|_| recraft.is_some()));
                    if section.is_object() {
                        value["data"] = serde_json::json!({"craft":craft,"recraft":recraft});
                    }
                }
                if let Some(learning) = args.tracked_recipe_learning {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Tracked recipe learning filter requires a saved Forever snapshot",
                        )]);
                    }
                    value["tracked_recipe_learning"] = serde_json::json!(learning);
                    if let Some(data) = value["data"].as_object_mut() {
                        let expected = match learning {
                            RecipeLearning::Learned => Some(true),
                            RecipeLearning::Unlearned => Some(false),
                            RecipeLearning::Unknown => None,
                        };
                        for group in ["crafts", "recrafts"] {
                            if let Some(rows) = data
                                .get_mut(group)
                                .and_then(serde_json::Value::as_object_mut)
                            {
                                rows.retain(|_, row| row["learned"].as_bool() == expected);
                            }
                        }
                    }
                }
                if review_action_bar && value["client"]["family"] != "forever" {
                    return CallToolResult::error(vec![ContentBlock::text(
                        "Action bar review requires a saved Forever snapshot",
                    )]);
                }
                if let Some(slot) = args.action_bar_slot {
                    let key = slot.to_string();
                    let observed = value["data"]
                        .get_mut("slots")
                        .and_then(serde_json::Value::as_object_mut)
                        .map(|slots| {
                            slots.retain(|id, _| id == &key);
                            !slots.is_empty()
                        });
                    value["action_bar_slot"] = serde_json::json!(slot);
                    value["action_bar_slot_observed"] = serde_json::json!(observed);
                    if let Some(rows) = value["data"]
                        .get_mut("button_hotkeys")
                        .and_then(serde_json::Value::as_object_mut)
                    {
                        rows.retain(|_, row| row["slot"].as_u64() == Some(u64::from(slot.get())));
                    }
                }
                if let Some(key) = action_bar_hotkey {
                    let data = &mut value["data"];
                    let matches = data
                        .get_mut("button_hotkeys")
                        .and_then(serde_json::Value::as_object_mut)
                        .map(|buttons| {
                            buttons.retain(|_, row| {
                                ["key1", "key2"].iter().any(|field| {
                                    row[*field]
                                        .as_str()
                                        .is_some_and(|observed| observed.eq_ignore_ascii_case(key))
                                })
                            });
                            let slots = buttons
                                .values()
                                .filter_map(|row| row["slot"].as_u64())
                                .collect::<std::collections::BTreeSet<_>>();
                            (buttons.len(), slots)
                        });
                    if let Some(slots) = data
                        .get_mut("slots")
                        .and_then(serde_json::Value::as_object_mut)
                    {
                        slots.retain(|slot, _| {
                            matches.as_ref().is_some_and(|(_, ids)| {
                                slot.parse::<u64>().is_ok_and(|slot| ids.contains(&slot))
                            })
                        });
                    }
                    value["action_bar_hotkey_match_count"] =
                        serde_json::json!(matches.map(|(count, _)| count));
                }
                if args.action_bar_spellbook {
                    let snapshot = match self.store.read(&args.snapshot_id) {
                        Ok(snapshot) => snapshot,
                        Err(_) => {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Could not read captured spellbook",
                            )]);
                        }
                    };
                    value["action_bar_spellbook"] =
                        azeroth_operator::analysis::action_bar_spellbook(
                            &value["data"],
                            snapshot.sections.get("spellbook"),
                            args.action_bar_slot.is_none() && action_bar_hotkey.is_none(),
                        );
                }
                if filter_spellbook {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Spellbook selection requires a saved Forever snapshot",
                        )]);
                    }
                    let observed = value["data"]
                        .get_mut("entries")
                        .and_then(serde_json::Value::as_object_mut)
                        .map(|rows| {
                            rows.retain(|_, row| {
                                args.spellbook_spell_id.is_none_or(|id| {
                                    let requested = Some(u64::from(id.get()));
                                    row["base_spell_id"].as_u64() == requested
                                        || row["spell_id"].as_u64() == requested
                                }) && args
                                    .spellbook_passive
                                    .is_none_or(|flag| row["passive"].as_bool() == Some(flag))
                                    && args
                                        .spellbook_off_spec
                                        .is_none_or(|flag| row["off_spec"].as_bool() == Some(flag))
                                    && spellbook_name.as_ref().is_none_or(|search| {
                                        row["name"].as_str().is_some_and(|name| {
                                            name.to_lowercase().contains(search)
                                        })
                                    })
                            });
                            !rows.is_empty()
                        });
                    if let Some(id) = args.spellbook_spell_id {
                        value["spellbook_spell_id"] = serde_json::json!(id);
                        value["spellbook_spell_observed"] = serde_json::json!(observed);
                    }
                    if let Some(passive) = args.spellbook_passive {
                        value["spellbook_passive"] = serde_json::json!(passive);
                    }
                    if let Some(off_spec) = args.spellbook_off_spec {
                        value["spellbook_off_spec"] = serde_json::json!(off_spec);
                    }
                }
                if args.section.as_deref() == Some("currency_balances") {
                    let headroom =
                        azeroth_operator::analysis::currency_cap_headroom(&value["data"]);
                    if let Some(id) = args.currency_id {
                        if value["client"]["family"] != "forever" {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Currency selection requires a saved Forever snapshot",
                            )]);
                        }
                        let key = id.to_string();
                        let section = value["data"].take();
                        let entries = section["entries"].as_object();
                        let row = entries.and_then(|entries| entries.get(&key));
                        value["currency_id"] = serde_json::json!(id);
                        value["currency_observed"] =
                            serde_json::json!(entries.map(|_| row.is_some()));
                        value["coverage"] = section["coverage"].clone();
                        value["data"] = row.cloned().unwrap_or_default();
                        value["currency_cap_headroom"] =
                            headroom.get(&key).cloned().unwrap_or_default();
                        if let Some(quantity) = args.currency_reward_quantity {
                            value["currency_reward_projection"] =
                                azeroth_operator::analysis::currency_reward_projection(
                                    quantity.get(),
                                    &value["data"],
                                    &value["currency_cap_headroom"],
                                );
                        }
                    } else {
                        value["currency_cap_headroom"] = headroom;
                    }
                } else if args.section.as_deref() == Some("reputation") {
                    let standard =
                        azeroth_operator::analysis::standard_reputation_progress(&value["data"]);
                    let special =
                        azeroth_operator::analysis::special_reputation_progress(&value["data"]);
                    if let Some(id) = args.faction_id {
                        if value["client"]["family"] != "forever" {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Faction selection requires a saved Forever snapshot",
                            )]);
                        }
                        let key = id.to_string();
                        let section = value["data"].take();
                        let entries = section["entries"].as_object();
                        let row = entries.and_then(|entries| entries.get(&key));
                        value["faction_id"] = serde_json::json!(id);
                        value["faction_observed"] =
                            serde_json::json!(entries.map(|_| row.is_some()));
                        value["coverage"] = section["coverage"].clone();
                        value["data"] = row.cloned().unwrap_or_default();
                        value["standard_reputation_progress"] =
                            standard.get(&key).cloned().unwrap_or_default();
                        value["special_reputation_progress"] =
                            special.get(&key).cloned().unwrap_or_default();
                    } else {
                        value["standard_reputation_progress"] = standard;
                        value["special_reputation_progress"] = special;
                    }
                } else if args.section.as_deref() == Some("tracked_achievements") {
                    let work = azeroth_operator::analysis::tracked_achievement_work(&value["data"]);
                    if let Some(id) = args.achievement_id {
                        if value["client"]["family"] != "forever" {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Achievement selection requires a saved Forever snapshot",
                            )]);
                        }
                        let key = id.to_string();
                        let section = value["data"].take();
                        let entries = section["entries"].as_object();
                        let row = entries.and_then(|entries| entries.get(&key));
                        value["achievement_id"] = serde_json::json!(id);
                        value["achievement_observed"] =
                            serde_json::json!(entries.map(|_| row.is_some()));
                        value["coverage"] = section["coverage"].clone();
                        value["data"] = row.cloned().unwrap_or_default();
                        value["tracked_achievement_work"] =
                            work.get(&key).cloned().unwrap_or_default();
                    } else {
                        value["tracked_achievement_work"] = work;
                    }
                } else if args.section.as_deref() == Some("professions") {
                    if let Some(id) = args.profession_skill_line {
                        let section = value["data"].take();
                        let (status, row) = azeroth_operator::analysis::profession_by_skill_line(
                            Some(&section),
                            id.get(),
                        );
                        value["profession_skill_line"] = serde_json::json!(id);
                        value["profession_observation_status"] = serde_json::json!(status);
                        value["data"] = row.cloned().unwrap_or_default();
                        value["profession_training_progress"] = row
                            .map(azeroth_operator::analysis::profession_rank_progress)
                            .unwrap_or_default();
                    } else {
                        value["profession_training_progress"] =
                            azeroth_operator::analysis::profession_training_progress(
                                &value["data"],
                            );
                    }
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
    fn review_equipment_draft(
        &self,
        snapshot: &azeroth_operator::Snapshot,
        snapshot_id: &str,
        selections: &[azeroth_operator::planning::EquipmentSelection],
        allow_two_hand_off_hand_clear: bool,
        catalogue: Option<&anyhow::Result<azeroth_operator::forever_catalogue::EquipmentCatalogue>>,
    ) -> anyhow::Result<serde_json::Value> {
        anyhow::ensure!(
            !selections.is_empty() || self.forever_catalogue.is_some(),
            "An empty baseline requires a local Forever catalogue"
        );
        anyhow::ensure!(
            !allow_two_hand_off_hand_clear
                || (snapshot.client.family == Family::Forever
                    && self.forever_catalogue.is_some()
                    && selections
                        .iter()
                        .any(|selection| selection.target_slot == 16)
                    && selections
                        .iter()
                        .all(|selection| selection.target_slot != 17)),
            "Clearing the off-hand requires a Forever catalogue and a main-hand selection without an off-hand selection"
        );
        let mut review = azeroth_operator::planning::equipment_review(snapshot, selections)?;
        if let Some(catalogue) = catalogue {
            let checks = catalogue
                .as_ref()
                .map_err(|_| anyhow::anyhow!("Catalogue verification failed"))
                .and_then(|catalogue| {
                    azeroth_operator::forever_catalogue::review_equipment_catalogue(
                        catalogue,
                        snapshot,
                        selections,
                        allow_two_hand_off_hand_clear,
                    )
                })
                .or_else(|error| {
                    if selections.is_empty() || allow_two_hand_off_hand_clear {
                        return Err(error);
                    }
                    Ok(serde_json::json!({
                        "catalogue_two_hand_pairing":"unknown",
                        "catalogue_slot_fit":null,
                        "catalogue_projected_weapon_skills":null,
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
        review["snapshot_id"] = serde_json::json!(snapshot_id);
        Ok(review)
    }

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
            let (bag_shortfall, withdrawals) =
                azeroth_operator::analysis::bag_restock(stock, cost.quantity.get());
            let after_observed_withdrawals =
                bag_shortfall
                    .zip(withdrawals.as_ref())
                    .map(|(shortfall, banks)| {
                        shortfall
                            - banks["character_bank"].as_u64().unwrap_or(0)
                            - banks["account_bank"].as_u64().unwrap_or(0)
                    });
            result["item_id"] = json!(cost.item_id);
            result["observed_bank_quantities"] = json!({
                "character_bank":stock["character_bank"],"account_bank":stock["account_bank"]
            });
            result["suggested_bank_withdrawals"] = json!(withdrawals);
            result["shortfall_after_observed_withdrawals"] = json!(after_observed_withdrawals);
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
                "estimated_cooldown_end_at":review["estimated_cooldown_end_at"],
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
