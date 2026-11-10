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
    "resolve_character",
    "compare_snapshots",
    "character_summary",
    "review_equipment_selection",
    "captured_talent_loadout",
    "lookup_forever_catalogue",
    "review_forever_owned_gear",
    "review_forever_recipe",
    "search_forever_recipes",
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
struct ResolveCharacter {
    family: Family,
    /// Explicit saved character; no target leaves active identity unknown.
    character: Option<azeroth_operator::Character>,
    /// Inclusive earliest capture timestamp for the explicit character.
    captured_from: Option<String>,
    /// Inclusive latest capture timestamp for the explicit character.
    captured_through: Option<String>,
    /// Require this exact client family, variant, build and interface for the explicit character.
    exact_client: Option<azeroth_operator::ClientIdentity>,
    /// Require every named section in the same saved capture of the explicit character.
    required_sections: Option<Vec<String>>,
    /// Return ambiguity if multiple matching captures share the latest timestamp.
    #[serde(default)]
    require_unique_latest: bool,
    /// Require the selected newest capture itself to contain all required sections.
    #[serde(default)]
    require_latest_coverage: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RestockStackLimit {
    item_id: std::num::NonZeroU32,
    maximum_stack: std::num::NonZeroU32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct QuestCurrencyTarget {
    currency_id: std::num::NonZeroU32,
    quantity: std::num::NonZeroU64,
    /// Consider one recorded optional reward for this currency per quest.
    #[serde(default)]
    include_choices: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct QuestItemTarget {
    item_id: std::num::NonZeroU32,
    quantity: std::num::NonZeroU64,
    /// Consider one recorded optional reward for this item per quest.
    #[serde(default)]
    include_choices: bool,
    /// Count copies in the same capture's carried bags toward the requested total.
    #[serde(default)]
    count_carried: bool,
    /// Estimate new general bag slots for the selected rewards of this item.
    #[serde(default)]
    estimate_bag_space: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct QuestRenownTarget {
    faction_id: std::num::NonZeroU32,
    /// Points to target. Omit to use the captured distance to the next rank.
    quantity: Option<std::num::NonZeroU64>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Inspect {
    snapshot_id: String,
    section: Option<String>,
    /// Compare each saved group category's reported member count against this target.
    group_member_target: Option<std::num::NonZeroU32>,
    /// Compare the saved instance group count with a uniquely matching saved lockout size reference.
    #[serde(default)]
    instance_group_size_reference: bool,
    /// Maximum lockout cache age at capture for the instance group size reference.
    instance_group_size_max_cache_age_seconds: Option<u32>,
    /// Return only capture statuses for these distinct section names instead of section data.
    coverage_sections: Option<Vec<String>>,
    /// Compare a target PvP rank with saved rank thresholds and caps.
    pvp_target_rank: Option<u32>,
    /// Review historical encounter progress and expiry for the saved instance lock warning.
    #[serde(default)]
    instance_lock_encounter_review: bool,
    /// Compare the projected saved lock warning expiry with this timestamp. Requires encounter review.
    instance_lock_warning_reference_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Compare projected temporary enchant expiries with this explicit reference timestamp.
    weapon_enchant_reference_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Shortlist equipped slots at or below this whole durability percentage (0 to 100).
    #[schemars(range(min = 0, max = 100))]
    equipment_durability_max_percent: Option<u8>,
    /// Copper to preserve after paying the captured repair-all quote.
    repair_wallet_reserve_copper: Option<u64>,
    /// Show damaged equipped slots observed alongside the saved repair quote.
    #[serde(default)]
    repair_include_equipment_condition: bool,
    /// Filter saved pet species by presence of an observed owned row.
    pet_has_owned_rows: Option<bool>,
    /// Filter saved pet species by presence of an observed unowned row.
    pet_has_unowned_rows: Option<bool>,
    /// Look up an exact toy ID and recorded flags in the saved toy_collection_view section.
    toy_in_view: Option<azeroth_operator::ToyViewFilter>,
    /// Look up one mount and its recorded flags in the saved mount_collection section.
    collected_mount: Option<azeroth_operator::CollectedMountFilter>,
    /// Look up one species and observed row bounds in the saved pet_collection_view section.
    pet_species: Option<azeroth_operator::PetSpeciesFilter>,
    /// Project historical reset timestamps for saved_instances or saved_world_bosses; no current eligibility.
    #[serde(default, deserialize_with = "saved_reset_flag")]
    saved_reset_projection: bool,
    /// Classify projected resets against this explicit timestamp. Requires saved_reset_projection.
    saved_reset_reference_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Compare 1..64 distinct base-item requirements with readable saved loot; no acquisition or stock credit.
    loot_requirements: Option<Vec<azeroth_operator::analysis::LootRequirement>>,
    /// Estimate carried space for the requested quantities visible in the saved loot window.
    #[serde(default)]
    loot_estimate_bag_space: bool,
    /// Estimate bag space for all observed loaded mail attachments matching these distinct item IDs.
    mail_attachment_bag_space_item_ids: Option<Vec<std::num::NonZeroU32>>,
    /// Estimate bag space for one loaded saved message, optionally limited to the requested item IDs.
    mail_attachment_bag_space_message_slot: Option<std::num::NonZeroU32>,
    /// Compare known loaded mail quantities for these distinct currency IDs with recorded cap headroom.
    mail_attachment_headroom_currency_ids: Option<Vec<std::num::NonZeroU32>>,
    /// Compare known currency attachments in one loaded saved message with recorded cap headroom.
    mail_attachment_headroom_message_slot: Option<std::num::NonZeroU32>,
    /// Compare the expected outgoing item quantities with a saved Forever trade offer.
    trade_offer_requirements: Option<Vec<azeroth_operator::analysis::LootRequirement>>,
    /// Compare the expected outgoing items with carried stock from the same saved capture.
    #[serde(default)]
    trade_offer_compare_carried: bool,
    /// Check one exact spell ID against the saved Forever profession_spells observation.
    profession_spell_id: Option<std::num::NonZeroU32>,
    /// Filter and page historical carried-item cooldown rows; requires section bag_item_cooldowns.
    bag_cooldown_filter: Option<azeroth_operator::analysis::BagCooldownFilter>,
    /// Project saved carried cooldowns at this timestamp for a cooldown filter or exact action-bar item stock lookup.
    bag_cooldown_reference_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Plan this carried quantity for exact action-bar item stock. Requires a cooldown reference time.
    bag_cooldown_target_quantity: Option<std::num::NonZeroU64>,
    /// Filter recorded base skill ranks/caps using {filter: {...}, offset: 0, limit: 20}; skill_lines only.
    #[serde(default, deserialize_with = "skill_line_argument")]
    skill_line_filter: Option<azeroth_operator::skill_line_filter::SkillLinePage>,
    /// Compare recorded fares with the wallet from the same Forever flight_map capture.
    #[serde(default)]
    flight_wallet_projection: bool,
    /// Select a captured flight-map node by its exact ID.
    flight_node_id: Option<std::num::NonZeroU32>,
    /// Keep reachable non-transition destinations at or below this saved fare.
    flight_max_fare_copper: Option<u64>,
    /// Check up to seven onward flight legs using explicit saved captures.
    flight_onward_legs: Option<Vec<FlightLeg>>,
    /// Include the starting capture's repair-all quote in the manual itinerary budget.
    #[serde(default)]
    flight_itinerary_include_repair_quote: bool,
    /// Find a route across this saved flight panel and optional onward panels.
    flight_route_search: Option<FlightRouteSearch>,
    /// Keep captured reachable flight destinations and exclude map-transition nodes.
    #[serde(default)]
    flight_reachable_only: bool,
    /// Keep reachable flights with a recorded fare covered by the wallet in this capture.
    #[serde(default)]
    flight_affordable_only: bool,
    /// Copper to reserve before evaluating captured flight fares.
    flight_wallet_reserve_copper: Option<u64>,
    /// Budget all lessons marked available in the saved Forever trainer list.
    #[serde(default)]
    trainer_wallet_projection: bool,
    /// Budget available pet trainer lessons against saved training points.
    #[serde(default)]
    trainer_points_projection: bool,
    /// Explain observed requirements for unavailable Forever trainer lessons.
    #[serde(default)]
    trainer_requirements_review: bool,
    /// Case-insensitive substring of a saved trainer lesson name.
    trainer_service_name: Option<String>,
    /// Saved trainer lesson status: available, unavailable, or used.
    trainer_service_status: Option<String>,
    /// Select up to 32 distinct slots from the captured trainer list.
    trainer_service_slots: Option<Vec<std::num::NonZeroU32>>,
    /// Reserve this many copper or pet training points before shortlisting affordable lessons.
    trainer_balance_reserve: Option<u64>,
    /// Keep lessons whose individual recorded price fits the captured wallet or pet points.
    #[serde(default)]
    trainer_affordable_only: bool,
    /// Keep lessons with a recorded copper price at or below this amount.
    trainer_max_price_copper: Option<u64>,
    /// Keep lessons with a recorded pet training-point price at or below this amount.
    trainer_max_price_points: Option<u32>,
    /// Review a saved merchant offer by its 1-based goods-tab slot.
    merchant_offer_slot: Option<std::num::NonZeroU32>,
    /// Requested bundles for the saved merchant offer. Defaults to one.
    merchant_bundles: Option<std::num::NonZeroU32>,
    /// Budget one to sixteen distinct captured merchant slots together.
    merchant_cart: Option<Vec<MerchantCartLine>>,
    /// Include the repair quote from the same saved merchant capture in the cart budget.
    #[serde(default)]
    merchant_cart_include_repair: bool,
    /// Estimate carried bag space for item bundles in the saved cart.
    #[serde(default)]
    merchant_cart_estimate_bag_space: bool,
    /// Filter saved merchant search results by captured stock state.
    merchant_stock_state: Option<azeroth_operator::MerchantStockState>,
    /// Review whole-entry buyback quotes for 1 to 12 distinct captured slots; requires merchant_buyback.
    buyback_slots: Option<Vec<std::num::NonZeroU32>>,
    /// Estimate carried bag space for the whole selected buyback entries.
    #[serde(default)]
    buyback_estimate_bag_space: bool,
    /// Reserve this much of the same captured wallet before reviewing buyback quotes.
    buyback_wallet_reserve_copper: Option<u64>,
    /// Find saved merchant offers that sell this exact item ID.
    merchant_item_id: Option<std::num::NonZeroU32>,
    /// Find saved merchant offers that sell this exact currency ID.
    merchant_currency_id: Option<std::num::NonZeroU32>,
    /// Match merchant items classified as accepted pet food in this compatible Forever capture.
    merchant_pet_food_snapshot_id: Option<String>,
    /// Calculate whole bundles needed for this quantity in each matching merchant offer.
    merchant_target_quantity: Option<std::num::NonZeroU32>,
    /// Select one saved quest by ID. Requires section "quests".
    quest_id: Option<std::num::NonZeroU32>,
    /// Compare captured quest currency rewards against caps from the same snapshot.
    #[serde(default)]
    quest_currency_projection: bool,
    /// Resolve captured quest reward items and recipe spell references in the pinned local Forever catalogue.
    #[serde(default)]
    quest_item_catalogue: bool,
    /// Compare user-supplied quest item requirements with one saved storage section.
    quest_inventory_requirements: Option<azeroth_operator::InventoryRequirementsFilter>,
    /// Review required bag quantities and observed bank restocking for one saved Forever capture.
    bag_restock_requirements: Option<Vec<azeroth_operator::InventoryRequirement>>,
    /// Page observed bank source locations for bag_restock_requirements.
    bag_restock_location_offset: Option<usize>,
    /// Estimate general bag slots for requested shortfalls using supplied stack limits.
    bag_restock_stack_limits: Option<Vec<RestockStackLimit>>,
    /// Estimate bag space from agreeing stack limits on recorded carried copies.
    #[serde(default)]
    bag_restock_use_reported_stack_limits: bool,
    /// Include recorded bank copies when inferring restock stack limits.
    #[serde(default)]
    bag_restock_include_bank_stack_limits: bool,
    /// Price the remaining shortfall using an explicitly selected compatible saved merchant visit.
    bag_restock_merchant_snapshot_id: Option<String>,
    /// Compare independent residual quotes from one to four compatible saved merchant visits.
    bag_restock_compare_merchant_snapshot_ids: Option<Vec<String>>,
    /// Choose the cheapest complete quote per item across the selected saved visits.
    #[serde(default)]
    bag_restock_split_merchant_plan: bool,
    /// Assumed copper cost per used visit for the optional travel-adjusted split plan.
    #[schemars(range(max = 9999999999999_u64))]
    bag_restock_cost_per_merchant_visit_copper: Option<u64>,
    /// Assumed costs in the same order as the selected merchant snapshot IDs.
    bag_restock_merchant_visit_costs_copper: Option<Vec<u64>>,
    /// Cap the number of distinct visits used by the travel-adjusted split plan.
    #[schemars(range(min = 0, max = 4))]
    bag_restock_max_merchant_visits: Option<u8>,
    /// Look up captured item source references for the residual bag shortfall.
    bag_restock_source_snapshot_id: Option<String>,
    /// Look up pinned local catalogue sources for the residual bag shortfall.
    #[serde(default)]
    bag_restock_catalogue_sources: bool,
    /// Annotate residual bag shortfalls with pending item attachment observations.
    #[serde(default)]
    bag_restock_pending_mail: bool,
    /// Use an explicitly selected compatible inbox capture for bag_restock_pending_mail.
    bag_restock_mail_snapshot_id: Option<String>,
    /// Compare residual bag shortfalls with readable loot from a compatible saved capture.
    bag_restock_loot_snapshot_id: Option<String>,
    /// Reserve copper from the saved inventory wallet for a merchant quote or split plan.
    #[schemars(range(max = 9999999999999_u64))]
    bag_restock_wallet_reserve_copper: Option<u64>,
    /// Use this compatible same-time or newer capture for quest inventory and wallet observations.
    quest_inventory_snapshot_id: Option<String>,
    /// Quote quest inventory shortfalls using this saved same-character merchant visit.
    quest_inventory_merchant_snapshot_id: Option<String>,
    /// Show verified local catalogue sources for known quest inventory shortfalls.
    #[serde(default)]
    quest_inventory_catalogue_sources: bool,
    /// Use this same-character saved source capture for known quest inventory shortfalls.
    quest_inventory_source_snapshot_id: Option<String>,
    /// Suggest saved bank withdrawals for quest item shortfalls in bags.
    #[serde(default)]
    quest_inventory_bank_withdrawals: bool,
    /// Zero-based page offset for the quest bank withdrawal locations.
    quest_inventory_bank_withdrawal_offset: Option<usize>,
    /// Compare a saved quest's coin cost and reward with the optional captured wallet.
    #[serde(default)]
    quest_wallet_projection: bool,
    /// Compare captured quest renown rewards with saved faction progress.
    #[serde(default)]
    quest_renown_projection: bool,
    /// Compare the saved quest XP reward with the character's captured XP threshold.
    #[serde(default)]
    quest_xp_projection: bool,
    /// Project combined observed XP for retained quests against the saved level threshold.
    #[serde(default)]
    quest_list_xp_projection: bool,
    /// Review whether saved ready quests cover this explicit honor reward target.
    quest_ready_honor_target: Option<std::num::NonZeroU64>,
    /// Project total costs and rewards for the retained quests using the same captured wallet.
    #[serde(default)]
    quest_list_wallet_projection: bool,
    /// Combine fixed currency rewards from retained quests against recorded currency cap headroom.
    #[serde(default)]
    quest_list_currency_projection: bool,
    /// Review fixed rewards from recorded ready quests against a currency target.
    quest_ready_currency_target: Option<QuestCurrencyTarget>,
    /// Review fixed rewards from recorded ready quests against an item target.
    quest_ready_item_target: Option<QuestItemTarget>,
    /// Combine renown rewards from retained quests against the same capture's faction progress.
    #[serde(default)]
    quest_list_renown_projection: bool,
    /// Plan ready quest turn-ins toward a renown target for one faction.
    quest_ready_renown_target: Option<QuestRenownTarget>,
    /// Total fixed item rewards and identify item choices across retained Forever quests.
    #[serde(default)]
    quest_list_item_rewards: bool,
    /// Show the shortest saved quest timers after applying all quest filters.
    #[serde(default)]
    quest_list_timed_checklist: bool,
    /// List unfinished captured objectives by quest, after applying quest filters.
    #[serde(default)]
    quest_list_objective_checklist: bool,
    /// Group quests marked ready in the saved log by their next waypoint map.
    #[serde(default)]
    quest_list_ready_waypoint_groups: bool,
    /// Filter saved Forever quests by their next waypoint map. Requires section "quests".
    quest_waypoint_map_id: Option<std::num::NonZeroU32>,
    /// Filter quest waypoints to the player map recorded in the same Forever snapshot.
    #[serde(default)]
    quest_on_captured_map: bool,
    /// Filter saved Forever quests to observed elite or suggested-group quests.
    quest_difficulty: Option<azeroth_operator::analysis::QuestDifficulty>,
    /// Filter saved Forever quests by captured turn-in readiness.
    quest_readiness: Option<azeroth_operator::analysis::QuestReadiness>,
    /// Keep quests with an objective of this exact captured kind (1 to 64 bytes).
    quest_objective_kind: Option<String>,
    /// Keep quests with an objective whose captured finished flag matches.
    quest_objective_finished: Option<bool>,
    /// Keep quests with an objective needing at most this many recorded units.
    /// All objective filters must match the same objective row.
    quest_objective_max_remaining: Option<u32>,
    /// Keep quests with an objective needing at least this many recorded units.
    quest_objective_min_remaining: Option<u32>,
    /// Keep saved Forever quests with at most this many seconds remaining at capture.
    quest_max_remaining_seconds: Option<u32>,
    /// Keep quests whose saved timer projects an expiry at or before this RFC 3339 timestamp.
    quest_projected_expiry_by: Option<chrono::DateTime<chrono::Utc>>,
    /// Keep quests whose saved timer projects an expiry at or after this RFC 3339 timestamp.
    quest_projected_expiry_from: Option<chrono::DateTime<chrono::Utc>>,
    /// Keep saved Forever quests with a recorded required copper cost at or below this amount.
    quest_max_required_copper: Option<u64>,
    /// Keep saved Forever quests with at least this much recorded XP reward.
    quest_min_reward_xp: Option<u64>,
    /// Keep saved Forever quests with at least this much recorded honor reward.
    quest_min_reward_honor: Option<u64>,
    /// Keep quests whose recorded copper cost fits the wallet in the same saved Forever snapshot.
    #[serde(default)]
    quest_within_captured_wallet: bool,
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
    /// Filter saved Forever currencies by captured flags and cap headroom; no exact ID selection.
    currency_filter: Option<azeroth_operator::analysis::CurrencyFilter>,
    /// Compare a proposed reward against this currency's recorded cap headroom.
    currency_reward_quantity: Option<std::num::NonZeroU64>,
    /// Select one saved Forever faction by ID. Requires section "reputation".
    faction_id: Option<std::num::NonZeroU32>,
    /// Estimate repetitions to the observed next standard reputation threshold at this many points each.
    reputation_points_per_turn_in: Option<std::num::NonZeroU32>,
    /// One assumed item cost per reputation turn-in, checked against the same saved bags.
    reputation_turn_in_item_cost: Option<ItemCost>,
    /// Distinct assumed item costs per turn-in. Supply 1 to 20, without the singular cost.
    reputation_turn_in_item_costs: Option<Vec<ItemCost>>,
    /// Include saved bank withdrawal suggestions, starting at this location offset.
    reputation_turn_in_bank_withdrawal_offset: Option<usize>,
    /// Show pending saved mail quantities for residuals in a reputation bank plan.
    #[serde(default)]
    reputation_turn_in_pending_mail: bool,
    /// Check an assumed copper cost per reputation turn-in against the captured wallet.
    reputation_turn_in_copper_cost: Option<ReputationTurnInCopperCost>,
    /// Filter visible Forever reputation rows; rank predicates require an explicit rank kind.
    reputation_filter: Option<azeroth_operator::analysis::ReputationFilter>,
    /// Select one saved Forever tracked achievement by ID. Requires section "tracked_achievements".
    achievement_id: Option<std::num::NonZeroU32>,
    /// Filter tracked Forever achievements by flags and same-row criterion predicates.
    /// Requires tracked_achievements without achievement_id; empty filters reject.
    achievement_filter: Option<azeroth_operator::analysis::AchievementFilter>,
    /// Select a collected mount by ID. Absence does not establish non-ownership.
    mount_id: Option<std::num::NonZeroU32>,
    /// Keep saved mount rows with this reported usability flag.
    mount_usable: Option<bool>,
    /// Keep saved mount rows with this reported favourite flag.
    mount_favorite: Option<bool>,
    /// Keep collected mounts with this recorded mount spell ID.
    mount_spell_id: Option<std::num::NonZeroU32>,
    /// Select one item from a captured filtered toy view; absent items have unknown collection state.
    toy_item_id: Option<std::num::NonZeroU32>,
    /// Keep only saved toy rows with this recorded collection flag.
    toy_collected: Option<bool>,
    /// Keep only saved toy rows with this recorded favourite flag.
    toy_favorite: Option<bool>,
    /// Keep saved toy rows with this exact case-insensitive public name.
    toy_name: Option<String>,
    /// Select one saved profession by skill line. Requires section "professions".
    profession_skill_line: Option<std::num::NonZeroU32>,
    /// Check this target rank against the selected saved profession's rank and cap.
    profession_target_rank: Option<std::num::NonZeroU32>,
    /// Include same-capture trainer thresholds between the saved profession rank and target.
    #[serde(default)]
    profession_target_trainer_lessons: bool,
    /// Preserve this much saved wallet copper when pricing each projected profession lesson.
    profession_target_trainer_reserve_copper: Option<u64>,
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
    /// Same-row captured charges/cast counts/cooldown flags; never current castability.
    spellbook_resource_filter: Option<azeroth_operator::SpellResourceFilter>,
    /// Match observed built-in action bar spells to the same saved Forever spellbook.
    #[serde(default)]
    action_bar_spellbook: bool,
    /// Find retained saved spell shortcuts with an exact reported spellbook row matching these resources.
    action_bar_spell_resource_filter: Option<azeroth_operator::SpellResourceFilter>,
    /// Select one captured Forever built-in action slot, numbered 1 to 4096.
    action_bar_slot: Option<std::num::NonZeroU32>,
    /// Match an exact key on captured Forever built-in action buttons.
    action_bar_hotkey: Option<String>,
    /// Report recorded hotkeys shared across multiple retained built-in slots.
    #[serde(default)]
    action_bar_shared_hotkeys: bool,
    /// Find captured action bar slots with this exact observed spell action ID.
    action_bar_spell_id: Option<std::num::NonZeroU32>,
    /// Find spell shortcuts by exact saved name through the same capture's reported spell IDs.
    action_bar_spell_name: Option<String>,
    /// Find captured action bar slots with this exact observed item action ID.
    action_bar_item_id: Option<std::num::NonZeroU32>,
    /// Show equipment and bag quantities for the matched item shortcut in the same capture.
    #[serde(default)]
    action_bar_item_stock: bool,
    /// Show carried stock for every retained item shortcut in the same capture.
    #[serde(default)]
    action_bar_item_stock_audit: bool,
    /// Report the captured carried-stock shortfall per item against this quantity. Requires the stock audit.
    action_bar_item_stock_target_quantity: Option<std::num::NonZeroU64>,
    /// Cross-reference observed action-bar spell slots with the saved Forever mount collection.
    #[serde(default)]
    action_bar_mount_audit: bool,
    /// Cross-reference observed action-bar item slots with the saved filtered toy journal.
    #[serde(default)]
    action_bar_toy_audit: bool,
    /// Filter saved tracked crafts and recrafts by observed learned status.
    tracked_recipe_learning: Option<azeroth_operator::RecipeLearning>,
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
struct ReputationTurnInCopperCost {
    copper_per_turn_in: std::num::NonZeroU64,
    #[serde(default)]
    wallet_reserve_copper: u64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct List {
    family: String,
    character: Option<azeroth_operator::Character>,
    /// Require this exact client family, variant, build and interface.
    exact_client: Option<azeroth_operator::ClientIdentity>,
    /// Inclusive earliest capture timestamp in RFC 3339 format.
    captured_from: Option<String>,
    /// Inclusive latest capture timestamp in RFC 3339 format.
    captured_through: Option<String>,
    /// Evaluate feature filters only against the newest capture per character, realm and client variant in the time window.
    #[serde(default)]
    latest_per_character: bool,
    /// Require every named snapshot section to be present in the same capture.
    required_sections: Option<Vec<String>>,
    /// Find captures missing at least one named section and report the coverage status.
    any_missing_sections: Option<Vec<String>>,
    /// Count capture statuses for these sections across all matching snapshots before paging.
    coverage_summary_sections: Option<Vec<String>>,
    /// Filter Forever snapshots by the exact output item ID of the captured selected recipe.
    recipe_output_item_id: Option<std::num::NonZeroU32>,
    /// Filter Forever snapshots by the captured selected recipe's profession ID.
    recipe_profession_id: Option<std::num::NonZeroU32>,
    /// Find saved Forever captures that track this exact recipe as a craft or recraft.
    tracked_recipe_id: Option<std::num::NonZeroU32>,
    /// Restrict the match to tracked crafts or recrafts. Without an ID, report matching counts.
    tracked_recipe_kind: Option<azeroth_operator::TrackedRecipeKind>,
    /// Restrict tracking rows by learned status. Without an ID, report matching counts.
    tracked_recipe_learning: Option<azeroth_operator::RecipeLearning>,
    /// Find saved Forever captures with a uniquely observed profession skill line.
    profession_skill_line: Option<std::num::NonZeroU32>,
    /// Minimum observed rank for the selected profession skill line.
    profession_min_rank: Option<u32>,
    /// Maximum observed rank for the selected profession skill line, inclusive.
    profession_max_rank: Option<u32>,
    /// Report progress toward this rank for each matching saved profession.
    profession_target_rank: Option<std::num::NonZeroU32>,
    /// Maximum rank points remaining to the saved profession target, inclusive.
    profession_target_max_remaining: Option<u32>,
    /// Require a known recorded cap at or above the saved profession target.
    #[serde(default)]
    profession_target_within_cap_only: bool,
    /// Filter Forever snapshots by the exact captured selected recipe ID.
    recipe_id: Option<std::num::NonZeroU32>,
    /// Match the selected recipe's recorded cooldown state, not overall craftability.
    recipe_cooldown_state: Option<azeroth_operator::analysis::RecipeCooldownState>,
    /// Filter saved Forever instance context by kind, including "none" for outside an instance.
    instance_kind: Option<String>,
    /// Filter saved Forever instance context by exact, case-insensitive name.
    instance_name: Option<String>,
    /// Filter saved Forever instance context by its observed difficulty ID.
    instance_difficulty_id: Option<u32>,
    /// Filter by the saved Forever instance map ID, independent of the localized name.
    instance_map_id: Option<std::num::NonZeroU32>,
    /// Match the player's exact map ID recorded in a saved Forever capture.
    player_map_id: Option<std::num::NonZeroU32>,
    /// Match an exact case-insensitive captured player map name.
    player_map_name: Option<String>,
    /// Match the recorded immediate parent of the player's map in a saved Forever capture.
    player_parent_map_id: Option<std::num::NonZeroU32>,
    /// Match an exact case-insensitive saved Forever Hearthstone bind name.
    hearthstone_bind_name: Option<String>,
    /// Match a recorded Hearthstone cooldown at or below this many seconds at capture.
    hearthstone_max_cooldown_seconds: Option<u32>,
    /// Inclusive RFC 3339 deadline for capture time plus recorded Hearthstone cooldown.
    hearthstone_projected_ready_by: Option<chrono::DateTime<chrono::Utc>>,
    /// Find saved Forever visits with a repair-all quote at or below this amount.
    repair_max_quote_copper: Option<u64>,
    /// Require at least this much saved wallet copper to remain after paying the repair quote.
    #[schemars(range(max = 9999999999999_u64))]
    repair_min_remaining_copper: Option<u64>,
    /// Find saved Forever captures with a slot at or below this whole durability percentage.
    #[schemars(range(min = 0, max = 100))]
    equipment_durability_max_percent: Option<u8>,
    /// Require the repair-all quote to fit the wallet recorded on the same visit.
    #[serde(default)]
    repair_affordable_only: bool,
    /// Minimum observed free slots across all carried bags, including specialty bags.
    minimum_free_bag_slots: Option<u32>,
    /// Minimum observed free slots in known general-purpose carried bags.
    minimum_general_free_bag_slots: Option<u32>,
    /// Require an exact observed bag family and a minimum number of free slots in it.
    bag_family_space: Option<azeroth_operator::BagFamilySpaceFilter>,
    /// Find saved captures with enough carried food classified as accepted by the active pet.
    pet_food: Option<azeroth_operator::PetFoodCaptureFilter>,
    /// Find saved characters by one base item's observed quantity in one explicit storage; never aggregate characters or shared banks.
    inventory_item: Option<azeroth_operator::InventoryItemFilter>,
    /// Find one capture whose explicit storage covers every item requirement; never pool characters or imply craftability.
    inventory_requirements: Option<azeroth_operator::InventoryRequirementsFilter>,
    /// Find observed shortfalls instead of fully covered requirements. Missing storage does not match.
    #[serde(default)]
    inventory_requirements_shortfall_only: bool,
    /// Include a quantity-only bank restocking plan for saved Forever bag shortfalls.
    #[serde(default)]
    inventory_requirements_bank_plan: bool,
    /// Find historical displayed loot; raw currency/money observations never imply ownership, rewards or balances.
    loot_observation: Option<azeroth_operator::loot_observation::LootObservationFilter>,
    /// Find a matching recorded base skill row; no bonuses, training or full skill coverage inferred.
    #[serde(default, deserialize_with = "skill_line_argument")]
    skill_line: Option<azeroth_operator::skill_line_filter::SkillLineFilter>,
    /// Find an exact recorded dialogue label or raw ID/status/flags on the same saved option; no selection eligibility.
    #[serde(default, deserialize_with = "gossip_option_argument")]
    gossip_option: Option<azeroth_operator::gossip_option_filter::GossipOptionFilter>,
    /// Find saved Forever captures whose outgoing trade offer visibly contained this base item ID.
    outgoing_trade_item_id: Option<std::num::NonZeroU32>,
    /// Inclusive minimum reported personal PvP rank in a saved Forever capture.
    minimum_pvp_rank: Option<u32>,
    /// Inclusive maximum reported personal PvP rank in a saved Forever capture.
    maximum_pvp_rank: Option<u32>,
    /// Match the exact reported PvP season ID in a saved Forever capture.
    pvp_season_id: Option<std::num::NonZeroU32>,
    /// Maximum saved points remaining to a known positive next PvP rank threshold.
    maximum_pvp_points_to_next_threshold: Option<u32>,
    /// Match a saved Forever PvP cap and its maximum remaining points.
    pvp_cap_distance: Option<azeroth_operator::PvpCapDistanceFilter>,
    /// Match saved PvP captures within a points distance of an exact rank.
    pvp_target_distance: Option<azeroth_operator::PvpTargetDistanceFilter>,
    /// Match saved Forever health or primary power by percentage range or primary power type.
    player_resource: Option<azeroth_operator::PlayerResourceFilter>,
    /// Match an exact spell ID in a saved helpful or harmful Forever player aura list.
    player_aura: Option<azeroth_operator::PlayerAuraFilter>,
    /// Search saved Forever weapon enchant rows for one hand and optional exact readings.
    weapon_enchant: Option<azeroth_operator::WeaponEnchantFilter>,
    /// Find an exact item in a saved Forever buyback list, optionally by quantity and whole-entry price.
    merchant_buyback: Option<azeroth_operator::MerchantBuybackFilter>,
    /// Find a saved Forever pet action spell by exact ID and optional observed flags.
    pet_action_spell: Option<azeroth_operator::PetActionSpellFilter>,
    /// Find saved Forever pet command slots by exact PET_ token and optional recorded active flag.
    pet_action_command: Option<azeroth_operator::PetActionCommandFilter>,
    /// Find saved Forever captures with matching observations in one explicit group category.
    group_context: Option<azeroth_operator::GroupContextFilter>,
    /// Find saved Forever captures that report this exact profession spell ID.
    profession_spell_id: Option<std::num::NonZeroU32>,
    /// Find a recorded currency balance at or above the requested quantity.
    currency_balance: Option<azeroth_operator::CurrencyBalanceFilter>,
    /// Find an observed standard faction rank at or above the requested reaction.
    reputation_rank: Option<azeroth_operator::ReputationRankFilter>,
    /// Find a captured renown faction at or above a level, optionally by capped status.
    renown: Option<azeroth_operator::RenownFilter>,
    /// Find recorded paragon rewards and level eligibility for an exact faction.
    paragon: Option<azeroth_operator::ParagonFilter>,
    /// Match a captured tracked achievement by ID and optional completion or character-credit flags.
    tracked_achievement: Option<azeroth_operator::TrackedAchievementFilter>,
    /// Find an observed collected mount by ID and optional recorded usability/favourite flags.
    collected_mount: Option<azeroth_operator::CollectedMountFilter>,
    /// Find collected mounts by exact case-insensitive saved public name.
    collected_mount_name: Option<String>,
    /// Find a collected mount by exact saved spell ID.
    collected_mount_spell_id: Option<std::num::NonZeroU32>,
    /// Find an exact toy ID in the saved filtered journal with optional recorded flags.
    toy_in_view: Option<azeroth_operator::ToyViewFilter>,
    /// Find an exact case-insensitive public name in saved filtered toy journals.
    toy_name: Option<String>,
    /// Match a named saved instance and its recorded lock/reset state.
    saved_instance: Option<azeroth_operator::SavedInstanceFilter>,
    /// Find a named world boss in the saved cache; does not establish loot eligibility.
    saved_world_boss: Option<azeroth_operator::SavedWorldBossFilter>,
    /// Find saved Forever instance lock warnings matching aggregate encounter progress.
    instance_lock_warning: Option<azeroth_operator::saved_resets::InstanceLockWarningFilter>,
    /// Match a recipe observed in the saved local filtered profession list, optionally by learning state.
    profession_recipe: Option<azeroth_operator::ProfessionRecipeFilter>,
    /// Match a known pending attachment in the saved loaded inbox, never spendable stock.
    mail_attachment: Option<azeroth_operator::MailAttachmentFilter>,
    /// Match a species observed in the saved filtered pet journal, not full account ownership.
    pet_species: Option<azeroth_operator::PetSpeciesFilter>,
    /// Find saved pet journal species by exact case-insensitive public species name.
    pet_species_name: Option<String>,
    /// Match saved active-pet presence, family, level and signed training-point balance.
    active_pet: Option<azeroth_operator::ActivePetCaptureFilter>,
    /// Match a saved visible player spellbook row by ID, exact name or recorded resources.
    spellbook_spell: Option<azeroth_operator::SpellbookSpellFilter>,
    /// Find saved Forever captures with this exact shortcut and matching recorded resources.
    action_bar_spell_resources: Option<azeroth_operator::ActionBarSpellResourceFilter>,
    /// Minimum rested XP recorded in a saved Forever capture.
    minimum_rested_xp: Option<u32>,
    /// Require an uncapped XP observation with XP gain explicitly enabled.
    #[serde(default)]
    xp_gain_enabled_only: bool,
    /// Match an exact quest ID in a saved Forever quest log.
    quest_id: Option<std::num::NonZeroU32>,
    /// Match an exact case-insensitive title in the saved Forever quest log.
    quest_title: Option<String>,
    /// Match an item ID in the same quest's recorded fixed or choice rewards.
    quest_reward_item_id: Option<std::num::NonZeroU32>,
    /// Match a currency ID in the same quest's recorded fixed or choice rewards.
    quest_reward_currency_id: Option<std::num::NonZeroU32>,
    /// Match a faction ID in the same quest's recorded renown rewards.
    quest_reward_renown_faction_id: Option<std::num::NonZeroU32>,
    /// Match recorded elite or suggested-group quests.
    quest_difficulty: Option<azeroth_operator::analysis::QuestDifficulty>,
    /// Match a recorded suggested group size at or below this value.
    quest_max_suggested_group_size: Option<std::num::NonZeroU32>,
    /// Match a known quest XP reward at or above this value.
    quest_min_reward_xp: Option<u64>,
    /// Match a known quest honor reward at or above this value.
    quest_min_reward_honor: Option<u64>,
    /// Match a known quest copper requirement at or below this value.
    quest_max_required_copper: Option<u64>,
    /// Match a known quest copper requirement covered by the same captured wallet.
    #[serde(default)]
    quest_within_captured_wallet: bool,
    /// Match an exact recorded objective kind on the same quest.
    quest_objective_kind: Option<String>,
    /// Match the reported finished state on the same objective.
    quest_objective_finished: Option<bool>,
    /// Inclusive minimum known remaining objective quantity.
    quest_objective_min_remaining: Option<u32>,
    /// Inclusive maximum known remaining objective quantity.
    quest_objective_max_remaining: Option<u32>,
    /// Match the selected quest's readiness, or count matching quests when no ID is supplied.
    quest_readiness: Option<azeroth_operator::analysis::QuestReadiness>,
    /// Match the next waypoint map recorded for the same quest.
    quest_waypoint_map_id: Option<std::num::NonZeroU32>,
    /// Match quests whose next waypoint is on the player's map in the same capture.
    #[serde(default)]
    quest_on_captured_map: bool,
    /// Skip this many matched quests in each saved local-map shortlist.
    local_quest_offset: Option<usize>,
    /// Put known shortest timers first within each saved local-map shortlist.
    #[serde(default)]
    local_quest_soonest_first: bool,
    /// Put known highest XP rewards first within each saved local-map shortlist.
    #[serde(default)]
    local_quest_highest_xp_first: bool,
    /// Match quests with a recorded remaining duration at or below this many seconds.
    quest_max_remaining_seconds: Option<u32>,
    /// Match quests whose historical expiry projects at or before this timestamp.
    quest_projected_expiry_by: Option<chrono::DateTime<chrono::Utc>>,
    /// Find Forever snapshots whose captured merchant sold this exact item ID.
    merchant_item_id: Option<std::num::NonZeroU32>,
    /// Find Forever snapshots whose captured merchant offered this exact currency ID.
    merchant_currency_id: Option<std::num::NonZeroU32>,
    /// Find merchant visits for the character and build of this saved pet food classification.
    merchant_pet_food_snapshot_id: Option<String>,
    /// Require one unlimited copper-only offer to cover this quantity, up to 1000 units.
    merchant_target_quantity: Option<std::num::NonZeroU32>,
    /// Sort matching merchant visits by the full bundle cost. Requires merchant_target_quantity.
    #[serde(default)]
    merchant_cheapest_first: bool,
    /// Combine eligible bundles for an exact item or currency and target quantity in each saved visit.
    #[serde(default)]
    merchant_combine_bundles: bool,
    /// Maximum total copper for the requested quantity, including whole-bundle rounding.
    merchant_max_total_copper: Option<u64>,
    /// Limit the captured copper component per bundle of a matching merchant offer.
    merchant_max_price_copper: Option<u64>,
    /// Require an offer with unlimited or positive captured stock.
    #[serde(default)]
    merchant_in_stock_only: bool,
    /// Match unlimited, limited in-stock, or sold-out stock as recorded in the capture.
    merchant_stock_state: Option<azeroth_operator::MerchantStockState>,
    /// Require a captured offer without extended costs.
    #[serde(default)]
    merchant_copper_only: bool,
    /// Require positive or unlimited recorded stock and a copper-only bundle within the captured wallet.
    #[serde(default)]
    merchant_affordable_only: bool,
    /// Find saved Forever trainer visits by a case-insensitive lesson name substring.
    trainer_service_name: Option<String>,
    /// Find saved Forever trainer visits with a lesson recorded as available, unavailable or used.
    trainer_service_status: Option<String>,
    /// Match trainer lessons with an observed copper price at or below this amount.
    trainer_max_price_copper: Option<u64>,
    /// Match pet trainer lessons with an observed point price at or below this amount.
    trainer_max_price_points: Option<u32>,
    /// Match trainer lessons priced within the wallet or pet points from that capture.
    #[serde(default)]
    trainer_affordable_only: bool,
    /// Reserve captured wallet copper or pet training points. Requires trainer_affordable_only.
    trainer_balance_reserve: Option<u64>,
    /// Find Forever captures with this reachable flight destination and an unambiguous origin.
    flight_destination_node_id: Option<std::num::NonZeroU32>,
    /// Find an unambiguous reachable flight destination by exact saved name.
    flight_destination_name: Option<String>,
    /// Narrow the flight destination search to this captured departure node.
    flight_origin_node_id: Option<std::num::NonZeroU32>,
    /// Keep saved routes with a recorded fare at or below this copper amount.
    flight_max_fare_copper: Option<u64>,
    /// Match the player's captured map while the flight panel was open.
    flight_departure_map_id: Option<std::num::NonZeroU32>,
    /// Keep routes whose recorded fare fits the wallet in the same saved capture.
    #[serde(default)]
    flight_affordable_only: bool,
    /// Keep saved fares that leave at least this much copper in the same captured wallet.
    flight_wallet_reserve_copper: Option<u64>,
    offset: Option<usize>,
    limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Compare {
    before_id: String,
    after_id: String,
    /// Compare reported profession spell ID sets only for matching native profession IDs.
    #[serde(default)]
    profession_spell_changes: bool,
    /// Compare historical player resource counters; power deltas require the same type and units.
    #[serde(default)]
    player_resource_changes: bool,
    /// Compare saved Forever aura observations without inferring application or removal events.
    #[serde(default)]
    player_aura_changes: bool,
    /// Compare saved Forever weapon enchant readings by hand.
    #[serde(default)]
    weapon_enchant_changes: bool,
    /// Compare saved Forever pet action bar contexts and slot readings.
    #[serde(default)]
    pet_action_bar_changes: bool,
    /// Compare saved bag cooldown readings for an exact Forever item ID.
    bag_cooldown_item_id: Option<std::num::NonZeroU32>,
    /// Compare saved Forever buyback entries without attributing transactions.
    #[serde(default)]
    merchant_buyback_changes: bool,
    /// Compare recorded merchant bundle prices for this exact item ID.
    merchant_offer_item_id: Option<std::num::NonZeroU32>,
    /// Compare recorded merchant bundle prices for this exact currency ID.
    merchant_offer_currency_id: Option<std::num::NonZeroU32>,
    /// Compare saved trainer prices and statuses for this exact lesson name.
    trainer_lesson_name: Option<String>,
    /// Compare saved flight fares and affordability for this exact destination node.
    flight_destination_node_id: Option<std::num::NonZeroU32>,
    /// Set aside this much copper in each capture's flight destination wallet check.
    flight_wallet_reserve_copper: Option<u64>,
    /// Compare saved Forever repair quotes and captured affordability.
    #[serde(default)]
    repair_quote_changes: bool,
    /// Compare saved equipped slots crossing this inclusive durability percentage (0 to 100).
    #[schemars(range(min = 0, max = 100))]
    equipment_durability_max_percent: Option<u8>,
    /// Restrict the saved aura comparison to this exact spell ID.
    player_aura_spell_id: Option<std::num::NonZeroU32>,
    /// Compare reported group categories across two saved Forever captures.
    #[serde(default)]
    group_context_changes: bool,
    /// Compare saved instance group size references across both captures.
    #[serde(default)]
    instance_group_size_reference: bool,
    /// Maximum saved lockout cache age at capture for both size references.
    instance_group_size_max_cache_age_seconds: Option<u32>,
    /// Compare reported group member counts against the same target in both captures.
    group_member_target: Option<std::num::NonZeroU32>,
    /// Compare the reported personal PvP progression and cap distances in saved Forever captures.
    #[serde(default)]
    player_pvp_progression_changes: bool,
    /// Compare the same saved PvP target rank across both captures.
    pvp_target_rank: Option<u32>,
    /// Compare captured outgoing trade item quantities by base item ID.
    #[serde(default)]
    player_trade_offer_changes: bool,
    /// Compare outgoing trade observations for this exact base item ID only.
    player_trade_offer_item_id: Option<std::num::NonZeroU32>,
    /// Compare the selected outgoing trade item's recorded bag quantities separately.
    #[serde(default)]
    player_trade_offer_compare_carried: bool,
    /// Optionally compare captured quantities for one base item ID.
    item_id: Option<std::num::NonZeroU32>,
    /// Compare recorded bag and bank container quantities for item_id.
    #[serde(default)]
    item_container_changes: bool,
    /// Optionally compare one captured Forever currency balance by ID.
    currency_id: Option<std::num::NonZeroU32>,
    /// Resolve a currency by exact saved name before comparing balances.
    currency_name: Option<String>,
    /// Optionally compare one captured Forever reputation faction by ID.
    faction_id: Option<std::num::NonZeroU32>,
    /// Resolve a faction by exact saved name before comparing reputation.
    faction_name: Option<String>,
    /// Optionally compare one captured quest by ID.
    quest_id: Option<std::num::NonZeroU32>,
    /// Resolve one Forever quest by exact saved title before comparing progress.
    quest_title: Option<String>,
    /// Compare next waypoint maps for quests present in both Forever captures.
    #[serde(default)]
    quest_waypoints: bool,
    /// Compare recorded readiness across both Forever quest logs.
    #[serde(default)]
    quest_readiness_changes: bool,
    /// Identify quests with objective changes across two saved Forever quest logs.
    #[serde(default)]
    quest_objective_changes: bool,
    /// Compare completion and character credit across both saved Forever tracked lists.
    #[serde(default)]
    achievement_status_changes: bool,
    /// Compare observations for the selected recipe, only deriving changes when recipe IDs agree.
    #[serde(default)]
    selected_recipe_changes: bool,
    /// Compare saved preparation for this many crafts of the selected Forever recipe (1 to 1000).
    selected_recipe_preparation_crafts: Option<u32>,
    /// Compare recorded collected mount IDs and flags, without inferring gains or losses.
    #[serde(default)]
    mount_collection_changes: bool,
    /// Compare overlapping toy-view rows without assuming equal filters or full collection coverage.
    #[serde(default)]
    toy_view_changes: bool,
    /// Compare public saved-instance descriptions; ambiguous rows never produce deltas.
    #[serde(default)]
    saved_instance_changes: bool,
    /// Compare encounter progress from saved warnings with matching instance descriptions.
    #[serde(default)]
    instance_lock_warning_changes: bool,
    /// Compare saved world boss reset durations by unique case-insensitive reported names.
    #[serde(default)]
    saved_world_boss_changes: bool,
    /// Compare learning, reported skill-ups and difficulty for shared recipes when captured profession IDs agree.
    #[serde(default)]
    profession_recipe_changes: bool,
    /// Compare historical bulk cooldown observations for shared recipe IDs within one profession.
    #[serde(default)]
    recipe_cooldown_changes: bool,
    /// Compare saved visible dialogue rows as a multiset, without assigning NPC identity.
    #[serde(default)]
    gossip_option_changes: bool,
    /// Compare displayed loot quantities and slot counts across two historical Forever captures.
    #[serde(default)]
    loot_window_changes: bool,
    /// Compare shared pet-journal species row counts without inferring collection gains or losses.
    #[serde(default)]
    pet_view_changes: bool,
    /// Compare known loaded mail attachment totals; never infer deliveries or inventory transfers.
    #[serde(default)]
    mail_attachment_changes: bool,
    /// Compare loaded saved mail quantities for this exact item without listing other attachments.
    mail_attachment_item_id: Option<std::num::NonZeroU32>,
    /// Compare the selected mail item's recorded carried quantity separately.
    #[serde(default)]
    mail_attachment_compare_carried_item: bool,
    /// Compare loaded saved mail quantities for this exact currency.
    mail_attachment_currency_id: Option<std::num::NonZeroU32>,
    /// Compare the selected mail currency's recorded balance separately.
    #[serde(default)]
    mail_attachment_compare_currency_balance: bool,
    /// Optionally compare one tracked Forever achievement by ID.
    achievement_id: Option<std::num::NonZeroU32>,
    /// Resolve one tracked achievement by exact saved name before comparing progress.
    achievement_name: Option<String>,
    /// Optionally compare one captured profession by its skill-line ID.
    profession_skill_line: Option<std::num::NonZeroU32>,
    /// Compare progress toward this rank for the selected profession.
    profession_target_rank: Option<std::num::NonZeroU32>,
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
    /// Compare placement of this exact reported spell ID across saved action bars.
    action_bar_spell_id: Option<std::num::NonZeroU32>,
    /// Compare placement of this exact reported item ID across saved action bars.
    action_bar_item_id: Option<std::num::NonZeroU32>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Summary {
    snapshot_id: String,
    /// Merge saved Forever dungeon, raid and world boss reset projections after this timestamp.
    saved_reset_reference_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Inclusive end of the saved reset schedule window. Requires saved_reset_reference_at.
    saved_reset_through: Option<chrono::DateTime<chrono::Utc>>,
    /// Limit the saved reset schedule to one activity kind. Requires saved_reset_reference_at.
    saved_reset_kind: Option<azeroth_operator::saved_resets::SavedResetKind>,
    /// Limit the reset schedule to saved instances matching these recorded details.
    saved_reset_instance_filter: Option<azeroth_operator::saved_resets::SavedResetInstanceFilter>,
    /// Maximum cache update age in seconds at capture. Requires saved_reset_reference_at.
    saved_reset_maximum_update_age_seconds: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TalentSummary {
    snapshot_id: String,
    /// Optional Forever simulator digit string; never a game import or action.
    simulator_talent_string: Option<String>,
    /// Prepare a simulator-only digit string from explicit ranks for every pinned definition; never infer missing zeros.
    #[serde(default, deserialize_with = "talent_preparation_flag")]
    prepare_simulator_talents: bool,
}

fn bank_restock_residual(review: &serde_json::Value) -> Option<serde_json::Value> {
    let rows = review.pointer("/bank_withdrawal_plan/rows")?.as_array()?;
    let items: Vec<_> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "item_id":row["item_id"],
                "shortfall_vs_observed":row["remaining_after_observed_withdrawals"],
            })
        })
        .collect();
    Some(serde_json::json!({"storage_observed":true,"items":items}))
}

fn talent_preparation_flag<'de, D: serde::Deserializer<'de>>(input: D) -> Result<bool, D::Error> {
    serde_json::Value::deserialize(input)?
        .as_bool()
        .ok_or_else(|| serde::de::Error::custom("Invalid simulator talent preparation flag"))
}

fn saved_reset_flag<'de, D: serde::Deserializer<'de>>(input: D) -> Result<bool, D::Error> {
    serde_json::Value::deserialize(input)?
        .as_bool()
        .ok_or_else(|| serde::de::Error::custom("Invalid saved reset projection flag"))
}

fn skill_line_argument<'de, D: serde::Deserializer<'de>, T: serde::de::DeserializeOwned>(
    input: D,
) -> Result<Option<T>, D::Error> {
    let value = serde_json::Value::deserialize(input)?;
    if value.is_null() {
        return Ok(None);
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(|_| serde::de::Error::custom("Invalid skill-line filter"))
}

fn gossip_option_argument<'de, D: serde::Deserializer<'de>>(
    input: D,
) -> Result<Option<azeroth_operator::gossip_option_filter::GossipOptionFilter>, D::Error> {
    let value = serde_json::Value::deserialize(input)?;
    if value.is_null() {
        return Ok(None);
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(|_| serde::de::Error::custom("Invalid gossip option filter"))
}

fn recipe_search_order<'de, D: serde::Deserializer<'de>>(
    input: D,
) -> Result<Option<azeroth_operator::analysis::RecipeSearchOrder>, D::Error> {
    let value = serde_json::Value::deserialize(input)?;
    if value.is_null() {
        return Ok(None);
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(|_| serde::de::Error::custom("Invalid recipe search ordering"))
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MerchantCartLine {
    slot: std::num::NonZeroU32,
    bundles: std::num::NonZeroU32,
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
    /// Filter by the exact signed random suffix ID encoded in a captured item link.
    observed_random_suffix_id: Option<i32>,
    /// Select locations with or without a resolved simulator item input before paging.
    simulator_input_resolved: Option<bool>,
    /// Filter by base-item presence in the pinned catalogue before paging.
    catalogue_matched: Option<bool>,
    /// Filter owned locations by the exact simulator-input resolution status before paging.
    simulator_item_resolution: Option<azeroth_operator::forever_catalogue::SimulatorItemResolution>,
    /// Count all matching locations by simulator-input status and storage before paging.
    #[serde(default)]
    include_simulator_item_resolution_counts: bool,
    /// Exclude known catalogue conflicts against the saved character. Unknown eligibility can still match.
    #[serde(default)]
    exclude_observed_requirement_conflicts: bool,
    /// Restrict matching gear locations to one captured storage section before paging.
    storage: Option<azeroth_operator::forever_catalogue::GearStorage>,
    /// Restrict results to a recorded bag or bank container ID before paging.
    #[schemars(range(min = 0, max = 64))]
    container_id: Option<u8>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FlightLeg {
    snapshot_id: String,
    destination_node_id: std::num::NonZeroU32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FlightRouteSearch {
    /// Exact destination ID. Can disambiguate a supplied destination name.
    destination_node_id: Option<std::num::NonZeroU32>,
    /// Exact case-insensitive saved destination name.
    destination_name: Option<String>,
    onward_snapshot_ids: Option<Vec<String>>,
    /// Exclude up to 32 distinct landing node IDs. Flyover preview nodes are outside this filter.
    excluded_landing_node_ids: Option<Vec<std::num::NonZeroU32>>,
    /// Exclude up to 32 distinct interior preview node IDs. A nonempty list requires a saved preview for each leg.
    excluded_flyover_node_ids: Option<Vec<std::num::NonZeroU32>>,
    /// Maximum flight legs, including the first leg. Defaults to eight.
    #[schemars(range(min = 1, max = 8))]
    max_legs: Option<u8>,
    /// Inclusive total fare ceiling across all legs, before any repair quote or wallet reserve.
    maximum_total_fare_copper: Option<u64>,
    /// Maximum elapsed seconds between the earliest and latest selected saved panels, inclusive.
    maximum_capture_skew_seconds: Option<u32>,
    /// Rank by lowest fare (default) or fewest flight legs.
    objective: Option<azeroth_operator::analysis::FlightRouteObjective>,
    /// Check the route budget including the repair-all quote in the starting capture.
    #[serde(default)]
    include_repair_quote: bool,
    /// Only consider routes covered by the starting wallet after any requested reserve and repair quote.
    #[serde(default)]
    require_affordable: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RecipeReview {
    snapshot_id: String,
    /// Select an exact recipe from bulk material observations. Omit for the selected schematic.
    recipe_id: Option<std::num::NonZeroU32>,
    /// Number of crafts. Supply exactly one of crafts or target_output_quantity.
    crafts: Option<u32>,
    /// Desired base item quantity, using the captured minimum per craft.
    target_output_quantity: Option<std::num::NonZeroU32>,
    /// Optional same-character and same-build capture, at least as recent as the recipe.
    inventory_snapshot_id: Option<String>,
    /// Up to seven other saved recipes checked against the same inventory capture.
    additional_recipes: Option<Vec<RecipeBatchEntry>>,
    /// Suggest a producer-before-consumer order for a batch with known fixed inputs and outputs.
    #[serde(default)]
    suggest_dependency_order: bool,
    /// Draft up to eight captured producer steps for one exact bulk recipe; no batch or dependency-order mode.
    #[serde(default)]
    suggest_reagent_chain: bool,
    /// Up to eight explicit item/recipe producer choices within the same bulk capture; requires reagent-chain mode.
    reagent_producer_choices: Option<Vec<azeroth_operator::analysis::RecipeProducerChoice>>,
    /// Preview up to four alternatives at the first unresolved chain producer choice.
    #[serde(default)]
    preview_reagent_producer_choices: bool,
    /// Zero-based offset into the alternatives for the first unresolved reagent.
    reagent_producer_preview_offset: Option<usize>,
    /// Estimate initial carried material needs and bank withdrawals for the printed reagent-chain order.
    #[serde(default)]
    reagent_chain_stock_plan: bool,
    /// Zero-based offset into the saved chain withdrawal slots. Requires a chain stock plan.
    reagent_chain_withdrawal_offset: Option<usize>,
    /// Annotate chain external shortages with bounded public sources from the verified local catalogue.
    #[serde(default)]
    reagent_chain_catalogue_sources: bool,
    /// Annotate chain external shortages using owner-captured provider source IDs; no local catalogue required.
    #[serde(default)]
    reagent_chain_captured_sources: bool,
    /// Optional same-character, exact-client/build reference capture; defaults to the recipe capture, not inventory.
    reagent_source_snapshot_id: Option<String>,
    /// Price saved merchant offers for material shortfalls, or net external leaves in reagent-chain mode.
    #[serde(default)]
    merchant_reagent_plan: bool,
    /// Optional merchant capture for the same character and exact client/build.
    merchant_snapshot_id: Option<String>,
    /// Estimate space for each requested recipe's base output against the saved carried bags.
    #[serde(default)]
    estimate_output_bag_space: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RecipeBatchEntry {
    snapshot_id: String,
    /// Select an exact bulk-observed recipe; omit for this capture's selected schematic.
    recipe_id: Option<std::num::NonZeroU32>,
    /// Number of crafts. Supply exactly one of crafts or target_output_quantity.
    crafts: Option<u32>,
    /// Desired base item quantity, using the captured minimum per craft.
    target_output_quantity: Option<std::num::NonZeroU32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RecipeSearch {
    snapshot_id: String,
    /// Optional same-character exact-client/build capture, no older than the recipe; requires material_filter.
    inventory_snapshot_id: Option<String>,
    recipe_id: Option<std::num::NonZeroU32>,
    output_item_id: Option<std::num::NonZeroU32>,
    /// Require at least this many units at the low end of the captured base output per craft. Missing ranges are excluded.
    minimum_base_output: Option<std::num::NonZeroU32>,
    reagent_item_id: Option<std::num::NonZeroU32>,
    status: Option<azeroth_operator::analysis::RecipeMaterialStatus>,
    /// Filter fixed reagent support in observed items, not actual craftability.
    material_filter: Option<azeroth_operator::analysis::RecipeMaterialFilter>,
    /// Same-ID/profession captured recipe-view flags, not verified learning or gains.
    recipe_view_filter: Option<azeroth_operator::analysis::RecipeViewFilter>,
    /// Filter original bulk-row cooldown state, observed charges or recorded timer; no elapsed recharge/current eligibility.
    cooldown_filter: Option<azeroth_operator::analysis::RecipeCooldownFilter>,
    /// Order by recipe ID, observed capacity, sole shortfall quantity, skill-ups, cooldown, or base output. Output per reagent ordering requires output_item_id and reagent_item_id.
    #[serde(default, deserialize_with = "recipe_search_order")]
    order: Option<azeroth_operator::analysis::RecipeSearchOrder>,
    offset: Option<usize>,
    /// Page size, 1 to 50; defaults to 20. Default ordering is numeric recipe ID.
    limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FindItem {
    snapshot_id: String,
    item_id: std::num::NonZeroU32,
    offset: Option<usize>,
    limit: Option<usize>,
    /// Compare matching carried cooldowns with this reference time. Requires Forever.
    bag_cooldown_reference_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Target carried quantity for projected cooldown planning. Requires bag_cooldown_reference_at.
    bag_cooldown_target_quantity: Option<std::num::NonZeroU64>,
    /// Up to seven additional snapshot IDs with the exact same client/build.
    additional_snapshot_ids: Option<Vec<String>>,
    /// Include exact item quantities by recorded bag or bank container, before paging.
    #[serde(default)]
    include_container_quantities: bool,
    /// Review carried stacks with identical saved item links, without generating operations.
    #[serde(default)]
    bag_stack_groups: bool,
    /// User-supplied stack capacity for a hypothetical consolidation estimate.
    assumed_max_stack_size: Option<std::num::NonZeroU32>,
    /// Estimate each group from unanimous captured stack limits instead of a supplied limit.
    #[serde(default)]
    use_reported_stack_limits: bool,
    /// Scope the stack review to bags, character_bank, or account_bank. Banks require Forever.
    stack_group_storage: Option<azeroth_operator::InventoryStorage>,
    /// Scope stack groups to one recorded container in the selected storage.
    #[schemars(range(min = 0, max = 64))]
    stack_group_container_id: Option<u8>,
    /// Skip this many stack groups, independently of item location paging.
    stack_group_offset: Option<usize>,
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
    /// Produce exact positional Forever simulator input; requires the qualified positional engine contract at execution.
    #[serde(default)]
    positional_simulator_equipment: bool,
    /// Desired empty simulator slots, requiring positional Forever review and captured occupied slots. No game action.
    #[serde(default)]
    clear_equipment_slots: Vec<u32>,
    /// Review up to seven more Forever drafts against this same snapshot.
    additional_drafts: Option<Vec<EquipmentDraft>>,
    /// Compare the primary draft's exact source locations with a newer capture of the same Forever character and build.
    compare_source_locations_snapshot_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EquipmentDraft {
    selections: Vec<azeroth_operator::planning::EquipmentSelection>,
    #[serde(default)]
    allow_two_hand_off_hand_clear: bool,
    #[serde(default)]
    clear_equipment_slots: Vec<u32>,
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
        let mut section_names = azeroth_operator::SNAPSHOT_SECTIONS.to_vec();
        section_names.sort_unstable();
        let mut forever_only = azeroth_operator::FOREVER_ONLY_OBSERVATIONS.to_vec();
        forever_only.extend(["character_bank", "account_bank"]);
        forever_only.sort_unstable();
        serde_json::json!({"families":["retail","classic","forever"],"implemented":implemented,"game_execution":false,"live_client_qualified":[],
            "snapshot_section_schema":{"known_names":section_names,"forever_only_names":forever_only},
            "policy":{"mode":"offline_analysis","generated_game_imports":false,"default":"deny_unreviewed_tools","snapshot_scope":"configured_owner_store","reviewed_on":"2026-11-09",
            "review_scope":"published_source_checks_and_project_controls_only",
            "blizzard_authorization_verified":false,"complete_applicable_terms_review_verified":false,
            "absolute_compliance_guaranteed":false,
            "source_evidence":[
                {"url":"https://us.forums.blizzard.com/en/wow/t/ui-add-on-development-policy/24534/1","checked_on":"2026-11-09","retrieval":"official_opening_post_read"},
                {"url":"https://us.forums.blizzard.com/en/wow/t/policy-update-for-input-broadcasting-may-2021/956613/1","checked_on":"2026-11-09","retrieval":"official_opening_post_read"},
                {"url":"https://us.forums.blizzard.com/en/wow/t/prohibitions-on-third-party-software/2142972/1","checked_on":"2026-11-09","retrieval":"official_opening_post_read"},
                {"url":"https://www.blizzard.com/en-gb/legal/2c72a35c-bf1b-4ae6-99ec-80624e1b429c/blizzard-end-user-license-agreement","checked_on":"2026-11-09","retrieval":"direct_access_refused"},
                {"url":"https://www.blizzard.com/en-us/legal/bfbbb648-bcc6-4b78-a5c7-e2fe20c135df/blizzard-end-user-license-agreement","checked_on":"2026-11-09","retrieval":"direct_access_refused"}
            ],"sources":["https://www.blizzard.com/en-gb/legal/2c72a35c-bf1b-4ae6-99ec-80624e1b429c/blizzard-end-user-license-agreement","https://us.forums.blizzard.com/en/wow/t/ui-add-on-development-policy/24534","https://us.forums.blizzard.com/en/wow/t/policy-update-for-input-broadcasting-may-2021/956613"]}}).to_string()
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
                let restock_recipes = query.restock_recipes.or_else(|| {
                    query.restock_recipe_snapshot_ids.map(|ids| {
                        ids.into_iter()
                            .map(
                                |snapshot_id| azeroth_operator::forever_catalogue::RestockRecipe {
                                    snapshot_id,
                                    recipe_id: None,
                                },
                            )
                            .collect()
                    })
                });
                if let Some(recipes) = restock_recipes {
                    self.add_restock_crafting(&snapshot, &mut progress, recipes)?;
                }
                if query.restock_merchant_reagent_plan {
                    let merchant = query
                        .restock_merchant_snapshot_id
                        .as_deref()
                        .map(|id| self.store.read(id))
                        .transpose()?;
                    if let Some(merchant) = &merchant {
                        azeroth_operator::analysis::ensure_same_character_and_client(
                            &snapshot, merchant,
                        )?;
                    }
                    let materials = progress["crafting_plan"]["combined_materials"]
                        .as_array()
                        .ok_or_else(|| anyhow::anyhow!("Restock materials unavailable"))?;
                    let mut quote = azeroth_operator::analysis::material_merchant_plan(
                        materials,
                        merchant.as_ref().unwrap_or(&snapshot),
                        &snapshot,
                    );
                    if !quote.is_null() {
                        quote["game_execution"] = serde_json::json!(false);
                        quote["purchase_eligibility_verified"] = serde_json::json!(false);
                    }
                    progress["crafting_plan"]["merchant_reagent_plan"] = quote;
                    if let Some(id) = query.restock_merchant_snapshot_id {
                        progress["crafting_plan"]["merchant_snapshot_id"] = serde_json::json!(id);
                    }
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
                args.observed_random_suffix_id,
                args.simulator_input_resolved,
                args.catalogue_matched,
                args.simulator_item_resolution,
                args.include_simulator_item_resolution_counts,
                args.exclude_observed_requirement_conflicts,
                args.storage,
                args.container_id,
            )?;
            review["snapshot_id"] = serde_json::json!(args.snapshot_id);
            Ok::<_, anyhow::Error>(review)
        })();
        match result {
            Ok(review) => CallToolResult::success(vec![ContentBlock::text(review.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Owned gear review requires a valid Forever snapshot, a verified local catalogue, a limit from 1 to 50 and ordered level bounds. A container ID must be from 0 to 64 and cannot be used with equipment storage.",
            )]),
        }
    }
    #[tool(
        description = "Find a base item ID in captured equipment, bags and supported banks. Matching bag locations retain available cooldown observations from that exact capture, without current usability claims. Optionally search several explicitly selected snapshots separately, without a simulator catalogue.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn find_snapshot_item(&self, Parameters(args): Parameters<FindItem>) -> CallToolResult {
        if args.bag_cooldown_target_quantity.is_some() && args.bag_cooldown_reference_at.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Item cooldown quantity targets require a reference time",
            )]);
        }
        let result = (|| {
            let additional = args.additional_snapshot_ids.unwrap_or_default();
            anyhow::ensure!(additional.len() <= 7, "Too many snapshots");
            anyhow::ensure!(
                (args.assumed_max_stack_size.is_none()
                    && !args.use_reported_stack_limits
                    && args.stack_group_storage.is_none()
                    && args.stack_group_container_id.is_none()
                    && args.stack_group_offset.is_none())
                    || args.bag_stack_groups,
                "Stack review options require bag stack groups"
            );
            anyhow::ensure!(
                !args.use_reported_stack_limits || args.assumed_max_stack_size.is_none(),
                "Choose reported stack limits or an assumed maximum"
            );
            anyhow::ensure!(
                args.stack_group_container_id.is_none_or(|id| id <= 64),
                "Stack group container ID must be from 0 to 64"
            );
            let query = |id: &str, snapshot: &azeroth_operator::Snapshot| {
                let mut found = azeroth_operator::analysis::find_item_with_container_quantities(
                    snapshot,
                    args.item_id.get(),
                    args.offset.unwrap_or(0),
                    args.limit.unwrap_or(20),
                    args.include_container_quantities,
                    args.bag_cooldown_reference_at,
                    args.bag_cooldown_target_quantity,
                )?;
                found["snapshot_id"] = serde_json::json!(id);
                if args.bag_stack_groups {
                    match args.stack_group_storage {
                        Some(azeroth_operator::InventoryStorage::Equipment) => {
                            anyhow::bail!("Equipment has no stack groups");
                        }
                        Some(
                            azeroth_operator::InventoryStorage::CharacterBank
                            | azeroth_operator::InventoryStorage::AccountBank,
                        ) => {
                            anyhow::ensure!(
                                snapshot.client.family == azeroth_operator::Family::Forever,
                                "Bank stack reviews require Forever"
                            );
                        }
                        _ => {}
                    }
                    found["bag_stack_groups"] =
                        serde_json::json!(azeroth_operator::analysis::observed_bag_stack_groups(
                            snapshot,
                            args.item_id.get(),
                            args.assumed_max_stack_size,
                            args.use_reported_stack_limits,
                            args.stack_group_storage,
                            args.stack_group_container_id,
                            args.stack_group_offset
                        ));
                }
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
                "Item search requires up to eight distinct compatible snapshots and a limit from 1 to 50. Cooldown projections require Forever. Stack review options require bag stack groups. Choose bags or a saved Forever bank, with a container ID from 0 to 64 if specified.",
            )]),
        }
    }
    #[tool(
        description = "Search owner-local captured Forever recipes by exact IDs/status, fixed material support, matching captured profession recipe-view flags and original bulk-row cooldown state/charges/timer. Material filtering can use an explicit no-older compatible inventory snapshot; recipe and cooldown evidence stay with the recipe capture. Observations do not verify learning, skill gains, elapsed recharge or craftability. Numeric paging up to 50 rows; unknowns stay unknown. No external lookup or game action.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn search_forever_recipes(&self, Parameters(args): Parameters<RecipeSearch>) -> CallToolResult {
        let result = (|| {
            anyhow::ensure!(
                args.inventory_snapshot_id.is_none() || args.material_filter.is_some(),
                "Inventory capture requires material filtering"
            );
            let snapshot = self.store.read(&args.snapshot_id)?;
            let inventory = args
                .inventory_snapshot_id
                .as_deref()
                .map(|id| self.store.read(id))
                .transpose()?;
            let mut found = azeroth_operator::analysis::search_recipe_materials_ordered(
                &snapshot,
                args.recipe_id,
                args.output_item_id,
                args.reagent_item_id,
                args.status,
                args.offset.unwrap_or(0),
                args.limit.unwrap_or(20),
                args.material_filter.as_ref(),
                args.recipe_view_filter.as_ref(),
                args.cooldown_filter.as_ref(),
                inventory.as_ref(),
                args.order.unwrap_or_default(),
                args.minimum_base_output,
            )?;
            found["snapshot_id"] = serde_json::json!(args.snapshot_id);
            if let Some(id) = args.inventory_snapshot_id {
                found["inventory_snapshot_id"] = serde_json::json!(id);
            }
            Ok::<_, anyhow::Error>(found)
        })();
        match result {
            Ok(found) => CallToolResult::success(vec![ContentBlock::text(found.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Recipe search requires owned valid Forever snapshots, exact positive IDs and a page size from 1 to 50. Observed capacity ordering requires material_filter. Sole shortfall ordering requires material_filter.sole_shortfall_item_id and insufficient_only. Output per reagent ordering requires output_item_id and reagent_item_id. An inventory snapshot requires material filtering and the same character/client/build with no older capture time.",
            )]),
        }
    }
    #[tool(
        description = "Compare saved Forever recipes with observed bags/banks for 1 to 1000 crafts each. An exact standalone bulk recipe can suggest a bounded reagent chain with explicit producer choices and optional external-material references from the local catalogue or owner-captured provider data. Modeled stock is shared; unsupported evidence still blocks. Offline drafts only, no crafting actions.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn review_forever_recipe(&self, Parameters(args): Parameters<RecipeReview>) -> CallToolResult {
        let result = (|| {
            anyhow::ensure!(
                args.merchant_snapshot_id.is_none() || args.merchant_reagent_plan,
                "Merchant snapshot requires a merchant reagent plan"
            );
            let additional = args.additional_recipes.unwrap_or_default();
            anyhow::ensure!(
                !args.reagent_chain_captured_sources || args.suggest_reagent_chain,
                "Captured chain sources require reagent-chain mode"
            );
            anyhow::ensure!(
                args.reagent_source_snapshot_id.is_none() || args.reagent_chain_captured_sources,
                "Reference capture requires captured chain sources"
            );
            anyhow::ensure!(
                !args.reagent_chain_catalogue_sources || args.suggest_reagent_chain,
                "Chain catalogue sources require reagent-chain mode"
            );
            anyhow::ensure!(
                !args.reagent_chain_stock_plan || args.suggest_reagent_chain,
                "Chain stock planning requires reagent-chain mode"
            );
            anyhow::ensure!(
                args.reagent_chain_withdrawal_offset.is_none() || args.reagent_chain_stock_plan,
                "Withdrawal location paging requires a chain stock plan"
            );
            anyhow::ensure!(
                args.reagent_producer_choices.is_none() || args.suggest_reagent_chain,
                "Producer choices require reagent-chain mode"
            );
            anyhow::ensure!(
                args.reagent_producer_preview_offset.is_none()
                    || args.preview_reagent_producer_choices,
                "Producer preview paging requires producer previews"
            );
            anyhow::ensure!(
                !args.preview_reagent_producer_choices
                    || (args.suggest_reagent_chain
                        && args
                            .reagent_producer_choices
                            .as_ref()
                            .is_none_or(|choices| choices.len() < 8)),
                "Producer previews require reagent-chain mode and room for one additional choice"
            );
            anyhow::ensure!(
                args.reagent_producer_choices
                    .as_ref()
                    .is_none_or(|choices| choices.len() <= 8),
                "Too many reagent producer choices"
            );
            anyhow::ensure!(
                !args.suggest_reagent_chain
                    || (args.recipe_id.is_some()
                        && additional.is_empty()
                        && !args.suggest_dependency_order),
                "Reagent chains require one exact bulk recipe without batch or dependency-order mode"
            );
            anyhow::ensure!(additional.len() <= 7, "Too many recipes");
            anyhow::ensure!(
                !args.suggest_dependency_order || !additional.is_empty(),
                "Dependency ordering requires multiple recipes"
            );
            let snapshot = self.store.read(&args.snapshot_id)?;
            let inventory = args
                .inventory_snapshot_id
                .as_deref()
                .map(|id| self.store.read(id))
                .transpose()?;
            let mut review = azeroth_operator::analysis::plan_recipe_by_id(
                &snapshot,
                args.recipe_id,
                args.crafts,
                args.target_output_quantity,
                inventory.as_ref(),
            )?;
            if args.suggest_reagent_chain {
                review["reagent_chain"] =
                    azeroth_operator::analysis::recipe_reagent_chain_with_choices(
                        &snapshot,
                        args.recipe_id.unwrap(),
                        review["requested_crafts"].as_u64().unwrap() as u32,
                        inventory.as_ref(),
                        args.reagent_producer_choices.as_deref().unwrap_or(&[]),
                    )?;
                if args.preview_reagent_producer_choices {
                    let previews = azeroth_operator::analysis::preview_reagent_producers(
                        &review["reagent_chain"],
                        &snapshot,
                        args.recipe_id.unwrap(),
                        review["requested_crafts"].as_u64().unwrap() as u32,
                        inventory.as_ref(),
                        args.reagent_producer_choices.as_deref().unwrap_or(&[]),
                        args.reagent_chain_stock_plan,
                        args.reagent_producer_preview_offset.unwrap_or_default(),
                    )?;
                    review["reagent_chain"]["producer_previews"] = previews;
                }
                if args.reagent_chain_stock_plan {
                    let plan = azeroth_operator::analysis::reagent_chain_stock_plan(
                        &review["reagent_chain"],
                        &snapshot,
                        inventory.as_ref(),
                        args.reagent_chain_withdrawal_offset.unwrap_or_default(),
                    )?;
                    review["reagent_chain"]["stock_plan"] = plan;
                }
                if args.reagent_chain_catalogue_sources {
                    let path = self.forever_catalogue.as_deref().ok_or_else(|| {
                        anyhow::anyhow!("Chain source references require a local catalogue")
                    })?;
                    let catalogue =
                        azeroth_operator::forever_catalogue::EquipmentCatalogue::load(path)?;
                    let sources = catalogue.reagent_chain_sources(&review["reagent_chain"])?;
                    review["reagent_chain"]["catalogue_sources"] = sources;
                    if let Some(previews) = review["reagent_chain"]
                        .pointer_mut("/producer_previews/candidate_previews")
                        .and_then(serde_json::Value::as_array_mut)
                    {
                        for preview in previews {
                            let sources = catalogue.reagent_chain_sources(preview)?;
                            preview["catalogue_source_summary"] = if sources.is_null() {
                                serde_json::Value::Null
                            } else {
                                let records = sources["records"].as_array().unwrap();
                                serde_json::json!({
                                    "catalogue_source":sources["catalogue_source"],
                                    "external_item_count":sources["external_item_count"],
                                    "items_with_recorded_sources":sources["items_with_recorded_sources"],
                                    "records":records.iter().take(20).collect::<Vec<_>>(),
                                    "records_truncated":records.len() > 20,
                                    "current_availability_verified":false,
                                })
                            };
                        }
                    }
                }
                if args.reagent_chain_captured_sources {
                    let source_capture = args
                        .reagent_source_snapshot_id
                        .as_deref()
                        .map(|id| self.store.read(id))
                        .transpose()?;
                    let source = source_capture.as_ref().unwrap_or(&snapshot);
                    let source_id = args
                        .reagent_source_snapshot_id
                        .as_ref()
                        .unwrap_or(&args.snapshot_id);
                    let mut sources = azeroth_operator::analysis::reagent_chain_captured_sources(
                        &review["reagent_chain"],
                        &snapshot,
                        source,
                    )?;
                    if sources.is_object() {
                        sources["snapshot_id"] = serde_json::json!(source_id);
                    }
                    review["reagent_chain"]["captured_sources"] = sources;
                    if let Some(previews) = review["reagent_chain"]
                        .pointer_mut("/producer_previews/candidate_previews")
                        .and_then(serde_json::Value::as_array_mut)
                    {
                        for preview in previews {
                            let sources =
                                azeroth_operator::analysis::reagent_chain_captured_sources(
                                    preview, &snapshot, source,
                                )?;
                            preview["captured_source_summary"] = if sources.is_null() {
                                serde_json::Value::Null
                            } else {
                                let records = sources["records"].as_array().unwrap();
                                serde_json::json!({
                                    "snapshot_id":source_id,"captured_at":sources["captured_at"],
                                    "section_available":sources["section_available"],
                                    "external_item_count":sources["external_item_count"],
                                    "items_with_retained_sources":sources["items_with_retained_sources"],
                                    "records":records.iter().take(20).collect::<Vec<_>>(),
                                    "records_truncated":records.len() > 20,
                                    "current_availability_verified":false,
                                })
                            };
                        }
                    }
                }
            }
            if let Some(id) = args.inventory_snapshot_id {
                review["inventory_snapshot_id"] = serde_json::json!(id);
            }
            let stock = inventory.as_ref().unwrap_or(&snapshot);
            if args.estimate_output_bag_space {
                review["output_bag_space_projection"] =
                    serde_json::json!(azeroth_operator::analysis::recipe_output_bag_space(
                        stock,
                        &review["base_output_if_completed"]
                    ));
            }
            if !additional.is_empty() {
                let mut extra_reviews = Vec::new();
                let mut recipe_ids =
                    std::collections::BTreeSet::from([review["recipe"]["recipe_id"].as_u64()]);
                for entry in additional {
                    let saved = self.store.read(&entry.snapshot_id)?;
                    let mut extra = azeroth_operator::analysis::plan_recipe_by_id(
                        &saved,
                        entry.recipe_id,
                        entry.crafts,
                        entry.target_output_quantity,
                        Some(stock),
                    )?;
                    anyhow::ensure!(
                        recipe_ids.insert(extra["recipe"]["recipe_id"].as_u64()),
                        "Duplicate recipe ID"
                    );
                    extra["snapshot_id"] = serde_json::json!(entry.snapshot_id);
                    if args.estimate_output_bag_space {
                        extra["output_bag_space_projection"] =
                            serde_json::json!(azeroth_operator::analysis::recipe_output_bag_space(
                                stock,
                                &extra["base_output_if_completed"]
                            ));
                    }
                    extra_reviews.push(extra);
                }
                review["combined_materials"] =
                    azeroth_operator::analysis::combine_recipe_materials(
                        std::iter::once(&review).chain(&extra_reviews),
                    )?;
                let (materials, checkpoint) = azeroth_operator::analysis::ordered_recipe_materials(
                    std::iter::once(&review).chain(&extra_reviews),
                    &review["combined_materials"],
                )?;
                review["ordered_starting_materials"] = materials;
                review["ordered_material_checkpoint"] = checkpoint;
                review["ordered_nonmaterial_requirements"] =
                    azeroth_operator::analysis::ordered_recipe_requirements(
                        std::iter::once(&review).chain(&extra_reviews),
                    );
                review["ordered_starting_material_coverage"] =
                    azeroth_operator::analysis::ordered_recipe_material_coverage(
                        &review["ordered_starting_materials"],
                    );
                review["combined_base_output_if_completed"] =
                    azeroth_operator::analysis::combine_recipe_outputs(
                        std::iter::once(&review).chain(&extra_reviews),
                    );
                if args.estimate_output_bag_space {
                    review["combined_output_bag_space_projection"] =
                        azeroth_operator::analysis::combined_recipe_output_bag_space(
                            stock,
                            &review["combined_base_output_if_completed"],
                        );
                }
                if args.suggest_dependency_order {
                    review["dependency_order"] =
                        azeroth_operator::analysis::recipe_dependency_order(
                            std::iter::once(&review).chain(&extra_reviews),
                            &review["combined_materials"],
                        )?;
                }
                review["additional_recipe_reviews"] = serde_json::json!(extra_reviews);
                if inventory.is_none() {
                    review["inventory_snapshot_id"] = serde_json::json!(args.snapshot_id);
                }
            }
            if args.merchant_reagent_plan {
                let merchant = args
                    .merchant_snapshot_id
                    .as_deref()
                    .map(|id| self.store.read(id))
                    .transpose()?;
                if let Some(merchant) = &merchant {
                    azeroth_operator::analysis::ensure_same_character_and_client(
                        &snapshot, merchant,
                    )?;
                }
                let stock = inventory.as_ref().unwrap_or(&snapshot);
                let offers = merchant.as_ref().unwrap_or(stock);
                if args.suggest_reagent_chain {
                    let quote = azeroth_operator::analysis::reagent_chain_merchant_plan(
                        &review["reagent_chain"],
                        offers,
                        stock,
                    );
                    review["reagent_chain"]["merchant_reagent_plan"] = quote;
                    if let Some(previews) = review["reagent_chain"]
                        .pointer_mut("/producer_previews/candidate_previews")
                        .and_then(serde_json::Value::as_array_mut)
                    {
                        for preview in previews {
                            let quote = azeroth_operator::analysis::reagent_chain_merchant_plan(
                                preview, offers, stock,
                            );
                            preview["merchant_quote_summary"] = if quote.is_null() {
                                serde_json::Value::Null
                            } else {
                                serde_json::json!({
                                    "merchant_captured_at":quote["merchant_captured_at"],
                                    "inventory_captured_at":quote["inventory_captured_at"],
                                    "copper_only_bundle_mix_total":quote["copper_only_bundle_mix_total"],
                                    "copper_only_bundle_mix_quoted_subtotal":quote["copper_only_bundle_mix_quoted_subtotal"],
                                    "copper_only_bundle_mix_unquoted_item_ids":quote["copper_only_bundle_mix_unquoted_item_ids"],
                                    "copper_only_bundle_mix_affordable":quote["copper_only_bundle_mix_affordable"],
                                    "purchase_eligibility_verified":false,
                                })
                            };
                        }
                    }
                } else {
                    review["merchant_reagent_plan"] =
                        azeroth_operator::analysis::recipe_merchant_plan(&review, offers, stock);
                }
                if args.suggest_dependency_order {
                    let ordered = &review["dependency_order"];
                    let dependency_review = serde_json::json!({
                        "ordered_starting_materials":ordered["starting_materials"],
                        "ordered_material_checkpoint":ordered["material_checkpoint"]
                    });
                    review["dependency_order"]["merchant_reagent_plan"] =
                        azeroth_operator::analysis::recipe_merchant_plan(
                            &dependency_review,
                            offers,
                            stock,
                        );
                }
                if let Some(id) = args.merchant_snapshot_id {
                    review["merchant_snapshot_id"] = serde_json::json!(id);
                }
            }
            Ok::<_, anyhow::Error>(review)
        })();
        match result {
            Ok(review) => CallToolResult::success(vec![ContentBlock::text(review.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Recipe review requires up to eight distinct saved Forever recipes and compatible captures. Supply crafts or a supported target output quantity per recipe, within 1 to 1000 crafts. A merchant snapshot requires a merchant reagent plan and the same character and client/build. Chain source options require chain mode; catalogue sources require a verified local catalogue, and an explicit reference capture requires captured sources and the same character/client/build.",
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
        description = "Return the captured Retail talent import string if available. Forever simulator_talent_string compares purchased ranks; prepare_simulator_talents prepares simulator digits only from an exact captured definition set with no staged changes. These modes are exclusive. Native import availability stays separate. Does not simulate, apply talents, optimise or verify build legality, native completeness or game acceptance.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn captured_talent_loadout(
        &self,
        Parameters(args): Parameters<TalentSummary>,
    ) -> CallToolResult {
        let result = (|| {
            anyhow::ensure!(
                !args.prepare_simulator_talents || args.simulator_talent_string.is_none(),
                "Exclusive talent review modes"
            );
            let snapshot = self.store.read(&args.snapshot_id)?;
            let mut review = azeroth_operator::planning::captured_loadout(&snapshot)?;
            if let Some(text) = args.simulator_talent_string {
                review["simulator_talent_observation"] =
                    azeroth_operator::forever_talent_identity::review(&snapshot, &text)?;
            }
            if args.prepare_simulator_talents {
                review["simulator_talent_preparation"] =
                    azeroth_operator::forever_talent_identity::prepare(&snapshot)?;
            }
            review["snapshot_id"] = serde_json::json!(args.snapshot_id);
            Ok::<_, anyhow::Error>(review)
        })();
        match result {
            Ok(review) => CallToolResult::success(vec![ContentBlock::text(review.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Snapshot or talent review input unavailable or invalid",
            )]),
        }
    }
    #[tool(
        description = "Review player-selected equipment from exact exported locations. Forever reviews can batch up to eight drafts against the same snapshot. Explicit clear_equipment_slots requires captured occupied simulator slots, a Forever catalogue and positional_simulator_equipment. Does not equip, optimise, validate game legality or generate a game import.",
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
                args.positional_simulator_equipment,
                &args.clear_equipment_slots,
                catalogue.as_ref(),
            )?;
            if let Some(id) = args.compare_source_locations_snapshot_id {
                let newer = self.store.read(&id)?;
                azeroth_operator::analysis::ensure_same_character_and_client(&snapshot, &newer)?;
                anyhow::ensure!(
                    snapshot.client.family == Family::Forever
                        && newer.captured_at > snapshot.captured_at,
                    "Source location comparison requires a newer Forever capture"
                );
                review["source_location_comparison"] = serde_json::json!({
                    "snapshot_id": id, "captured_at": newer.captured_at,
                    "current_locations_verified": false,
                    "selections": azeroth_operator::planning::compare_equipment_sources(
                        &snapshot, &newer, &args.selections)?
                });
            }
            if !additional.is_empty() {
                let extra = additional
                    .into_iter()
                    .map(|draft| {
                        self.review_equipment_draft(
                            &snapshot,
                            &args.snapshot_id,
                            &draft.selections,
                            draft.allow_two_hand_off_hand_clear,
                            args.positional_simulator_equipment,
                            &draft.clear_equipment_slots,
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
        description = "Compare same-character same-client snapshots, with optional item, quest, profession, filtered pet-journal, Forever wallet, currency, reputation and tracked achievement comparisons. Forever quest comparisons can include waypoint map changes.",
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
                    item_container_changes: args.item_container_changes,
                    currency_id: args.currency_id.map(|id| id.get()),
                    currency_name: args.currency_name.as_deref(),
                    faction_id: args.faction_id.map(|id| id.get()),
                    faction_name: args.faction_name.as_deref(),
                    quest_id: args.quest_id.map(|id| id.get()),
                    quest_title: args.quest_title.as_deref(),
                    quest_waypoints: args.quest_waypoints,
                    quest_readiness_changes: args.quest_readiness_changes,
                    quest_objective_changes: args.quest_objective_changes,
                    achievement_status_changes: args.achievement_status_changes,
                    selected_recipe_changes: args.selected_recipe_changes,
                    selected_recipe_preparation_crafts: args.selected_recipe_preparation_crafts,
                    mount_collection_changes: args.mount_collection_changes,
                    toy_view_changes: args.toy_view_changes,
                    saved_instance_changes: args.saved_instance_changes,
                    instance_lock_warning_changes: args.instance_lock_warning_changes,
                    saved_world_boss_changes: args.saved_world_boss_changes,
                    profession_recipe_changes: args.profession_recipe_changes,
                    recipe_cooldown_changes: args.recipe_cooldown_changes,
                    loot_window_changes: args.loot_window_changes,
                    gossip_option_changes: args.gossip_option_changes,
                    profession_spell_changes: args.profession_spell_changes,
                    player_resource_changes: args.player_resource_changes,
                    player_aura_changes: args.player_aura_changes,
                    weapon_enchant_changes: args.weapon_enchant_changes,
                    pet_action_bar_changes: args.pet_action_bar_changes,
                    bag_cooldown_item_id: args.bag_cooldown_item_id,
                    merchant_buyback_changes: args.merchant_buyback_changes,
                    merchant_offer_item_id: args.merchant_offer_item_id,
                    merchant_offer_currency_id: args.merchant_offer_currency_id,
                    trainer_lesson_name: args.trainer_lesson_name.as_deref(),
                    flight_destination_node_id: args.flight_destination_node_id,
                    flight_wallet_reserve_copper: args.flight_wallet_reserve_copper,
                    repair_quote_changes: args.repair_quote_changes,
                    equipment_durability_max_percent: args.equipment_durability_max_percent,
                    player_aura_spell_id: args.player_aura_spell_id,
                    group_context_changes: args.group_context_changes,
                    instance_group_size_reference: args.instance_group_size_reference,
                    instance_group_size_max_cache_age_seconds: args
                        .instance_group_size_max_cache_age_seconds,
                    group_member_target: args.group_member_target,
                    player_pvp_progression_changes: args.player_pvp_progression_changes,
                    pvp_target_rank: args.pvp_target_rank,
                    player_trade_offer_changes: args.player_trade_offer_changes,
                    player_trade_offer_item_id: args.player_trade_offer_item_id,
                    player_trade_offer_compare_carried: args.player_trade_offer_compare_carried,
                    pet_view_changes: args.pet_view_changes,
                    mail_attachment_changes: args.mail_attachment_changes,
                    mail_attachment_item_id: args.mail_attachment_item_id,
                    mail_attachment_compare_carried_item: args.mail_attachment_compare_carried_item,
                    mail_attachment_currency_id: args.mail_attachment_currency_id,
                    mail_attachment_compare_currency_balance: args
                        .mail_attachment_compare_currency_balance,
                    achievement_id: args.achievement_id.map(|id| id.get()),
                    achievement_name: args.achievement_name.as_deref(),
                    profession_skill_line: args.profession_skill_line.map(|id| id.get()),
                    profession_target_rank: args.profession_target_rank,
                    wallet: args.wallet,
                    experience: args.experience,
                    spellbook: args.spellbook,
                    action_bar_slots: args.action_bar_slots,
                    action_bar_spell_id: args.action_bar_spell_id,
                    action_bar_item_id: args.action_bar_item_id,
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
        if args.saved_reset_maximum_update_age_seconds.is_some()
            && args.saved_reset_reference_at.is_none()
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "A saved reset cache age limit requires a reference time",
            )]);
        }
        if args.saved_reset_instance_filter.is_some()
            && (args.saved_reset_reference_at.is_none()
                || matches!(
                    args.saved_reset_kind,
                    Some(azeroth_operator::saved_resets::SavedResetKind::WorldBoss)
                ))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "A saved reset instance filter requires a reference time and excludes world boss schedules",
            )]);
        }
        if args.saved_reset_kind.is_some() && args.saved_reset_reference_at.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "A saved reset kind requires a reference time",
            )]);
        }
        if let Some(end) = args.saved_reset_through
            && args
                .saved_reset_reference_at
                .is_none_or(|start| end < start)
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "A saved reset schedule end requires a reference time at or before it",
            )]);
        }
        match self.store.read(&args.snapshot_id) {
            Ok(s) => {
                let mut summary = azeroth_operator::analysis::summary(&s);
                if let Some(reference) = args.saved_reset_reference_at {
                    let Ok(schedule) = azeroth_operator::saved_resets::combined_schedule(
                        &s,
                        reference,
                        args.saved_reset_through,
                        args.saved_reset_kind,
                        args.saved_reset_instance_filter,
                        args.saved_reset_maximum_update_age_seconds,
                    ) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Saved reset schedules require a valid Forever snapshot",
                        )]);
                    };
                    summary["saved_reset_schedule"] = schedule;
                }
                CallToolResult::success(vec![ContentBlock::text(summary.to_string())])
            }
            Err(_) => {
                CallToolResult::error(vec![ContentBlock::text("Snapshot unavailable or invalid")])
            }
        }
    }
    #[tool(
        description = "Resolve an explicitly requested saved character in the owner's store. Without an explicit character, return unknown: active-character detection is not yet available. Never infer current play from the newest saved snapshot. Selection is historical and does not authorise game actions.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    fn resolve_character(&self, Parameters(args): Parameters<ResolveCharacter>) -> CallToolResult {
        let Some(character) = args.character else {
            if args.captured_from.is_some()
                || args.captured_through.is_some()
                || args.exact_client.is_some()
                || args.required_sections.is_some()
                || args.require_unique_latest
                || args.require_latest_coverage
            {
                return CallToolResult::error(vec![ContentBlock::text(
                    "Capture time, exact client, section and latest-capture options require an explicit character",
                )]);
            }
            return CallToolResult::success(vec![ContentBlock::text(
                serde_json::json!({"status":"active_identity_unavailable","selection":null,
                    "live":false,"active_character_verified":false})
                .to_string(),
            )]);
        };
        let latest_coverage_sections = if args.require_latest_coverage {
            match args.required_sections.as_deref() {
                Some(sections) if azeroth_operator::validate_section_names(sections).is_ok() => {
                    Some(sections)
                }
                _ => {
                    return CallToolResult::error(vec![ContentBlock::text(
                        "Latest coverage requires valid required_sections",
                    )]);
                }
            }
        } else {
            None
        };
        match self.store.list_filtered(
            args.family,
            Some(&character),
            azeroth_operator::CaptureFilters {
                exact_client: args.exact_client.as_ref(),
                captured_from: args.captured_from.as_deref(),
                captured_through: args.captured_through.as_deref(),
                required_sections: if args.require_latest_coverage {
                    None
                } else {
                    args.required_sections.as_deref()
                },
                ..Default::default()
            },
            0,
            if args.require_unique_latest { 2 } else { 1 },
        ) {
            Ok(result) => {
                let rows = result["snapshots"]
                    .as_array()
                    .map_or(&[][..], Vec::as_slice);
                let tied = args.require_unique_latest
                    && matches!(rows, [first, second, ..] if first["captured_at"] == second["captured_at"]);
                let (status, selection) = match (tied, rows.first()) {
                    (true, _) => ("ambiguous_latest_snapshot", None),
                    (false, Some(row)) => ("explicit_saved_character", Some(row)),
                    (false, None) => ("no_matching_snapshot", None),
                };
                let mut result = serde_json::json!({
                    "status":status,"selection":selection,"live":false,"active_character_verified":false,
                });
                if let Some((row, sections)) = selection.zip(latest_coverage_sections) {
                    let Some(coverage) = row["snapshot_id"].as_str().and_then(|id| {
                        self.store
                            .inspect_with_coverage(id, None, Some(sections))
                            .ok()
                    }) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Latest capture coverage unavailable",
                        )]);
                    };
                    let statuses = &coverage["section_coverage"];
                    if sections.iter().any(|name| statuses[name] != "observed") {
                        result["status"] =
                            serde_json::json!("latest_snapshot_missing_required_sections");
                        result["selection"] = serde_json::Value::Null;
                    }
                    result["latest_section_coverage"] = statuses.clone();
                }
                CallToolResult::success(vec![ContentBlock::text(result.to_string())])
            }
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Character selection unavailable or invalid",
            )]),
        }
    }
    #[tool(
        description = "List saved snapshots for an explicit client family and optional character name plus realm, newest capture first. Supports capture time bounds, newest-per-character selection and exact item quantity filters in one explicit observed storage without cross-character aggregation. Forever listings can filter captured quests, currency, XP, maps, professions, recipes, bag space, pet food, repair quotes, merchant offers, trainer lessons and flights. Does not identify the character currently in play.",
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
                exact_client: args.exact_client.as_ref(),
                captured_from: args.captured_from.as_deref(),
                captured_through: args.captured_through.as_deref(),
                latest_per_character: args.latest_per_character,
                recipe: azeroth_operator::RecipeCaptureFilters {
                    recipe_id: args.recipe_id,
                    output_item_id: args.recipe_output_item_id,
                    profession_id: args.recipe_profession_id,
                    cooldown_state: args.recipe_cooldown_state,
                },
                tracked_recipe_id: args.tracked_recipe_id,
                tracked_recipe_kind: args.tracked_recipe_kind,
                tracked_recipe_learning: args.tracked_recipe_learning,
                profession_skill_line: args.profession_skill_line,
                profession_min_rank: args.profession_min_rank,
                profession_max_rank: args.profession_max_rank,
                profession_target_rank: args.profession_target_rank,
                profession_target_max_remaining: args.profession_target_max_remaining,
                profession_target_within_cap_only: args.profession_target_within_cap_only,
                instance: azeroth_operator::InstanceCaptureFilters {
                    kind: args.instance_kind.as_deref(),
                    name: args.instance_name.as_deref(),
                    difficulty_id: args.instance_difficulty_id,
                    map_id: args.instance_map_id,
                },
                player_map_id: args.player_map_id,
                player_map_name: args.player_map_name.as_deref(),
                player_parent_map_id: args.player_parent_map_id,
                hearthstone_bind_name: args.hearthstone_bind_name.as_deref(),
                hearthstone_max_cooldown_seconds: args.hearthstone_max_cooldown_seconds,
                hearthstone_projected_ready_by: args.hearthstone_projected_ready_by,
                repair_max_quote_copper: args.repair_max_quote_copper,
                repair_min_remaining_copper: args.repair_min_remaining_copper,
                equipment_durability_max_percent: args.equipment_durability_max_percent,
                repair_affordable_only: args.repair_affordable_only,
                minimum_free_bag_slots: args.minimum_free_bag_slots,
                minimum_general_free_bag_slots: args.minimum_general_free_bag_slots,
                bag_family_space: args.bag_family_space,
                pet_food: args.pet_food,
                inventory_item: args.inventory_item,
                inventory_requirements: args.inventory_requirements,
                inventory_requirements_shortfall_only: args.inventory_requirements_shortfall_only,
                inventory_requirements_bank_plan: args.inventory_requirements_bank_plan,
                required_sections: args.required_sections.as_deref(),
                any_missing_sections: args.any_missing_sections.as_deref(),
                coverage_summary_sections: args.coverage_summary_sections.as_deref(),
                loot_observation: args.loot_observation,
                skill_line: args.skill_line,
                gossip_option: args.gossip_option,
                outgoing_trade_item_id: args.outgoing_trade_item_id,
                pvp_rank_range: (args.minimum_pvp_rank, args.maximum_pvp_rank),
                pvp_season_id: args.pvp_season_id,
                maximum_pvp_points_to_next_threshold: args.maximum_pvp_points_to_next_threshold,
                pvp_cap_distance: args.pvp_cap_distance,
                pvp_target_distance: args.pvp_target_distance,
                player_resource: args.player_resource,
                player_aura: args.player_aura,
                weapon_enchant: args.weapon_enchant,
                merchant_buyback: args.merchant_buyback,
                pet_action_spell: args.pet_action_spell,
                pet_action_command: args.pet_action_command,
                group_context: args.group_context,
                profession_spell_id: args.profession_spell_id,
                currency_balance: args.currency_balance,
                reputation_rank: args.reputation_rank,
                renown: args.renown,
                paragon: args.paragon,
                tracked_achievement: args.tracked_achievement,
                collected_mount: args.collected_mount,
                collected_mount_name: args.collected_mount_name.as_deref(),
                collected_mount_spell_id: args.collected_mount_spell_id,
                toy_in_view: args.toy_in_view,
                toy_name: args.toy_name.as_deref(),
                saved_instance: args.saved_instance,
                saved_world_boss: args.saved_world_boss,
                instance_lock_warning: args.instance_lock_warning,
                profession_recipe: args.profession_recipe,
                mail_attachment: args.mail_attachment,
                pet_species: args.pet_species,
                pet_species_name: args.pet_species_name.as_deref(),
                active_pet: args.active_pet,
                spellbook_spell: args.spellbook_spell,
                action_bar_spell_resources: args.action_bar_spell_resources,
                minimum_rested_xp: args.minimum_rested_xp,
                xp_gain_enabled_only: args.xp_gain_enabled_only,
                quest_id: args.quest_id,
                quest_title: args.quest_title.as_deref(),
                quest_reward_item_id: args.quest_reward_item_id,
                quest_reward_currency_id: args.quest_reward_currency_id,
                quest_reward_renown_faction_id: args.quest_reward_renown_faction_id,
                quest_difficulty: args.quest_difficulty,
                quest_max_suggested_group_size: args.quest_max_suggested_group_size,
                quest_min_reward_xp: args.quest_min_reward_xp,
                quest_min_reward_honor: args.quest_min_reward_honor,
                quest_max_required_copper: args.quest_max_required_copper,
                quest_within_captured_wallet: args.quest_within_captured_wallet,
                quest_objective_kind: args.quest_objective_kind.as_deref(),
                quest_objective_finished: args.quest_objective_finished,
                quest_objective_remaining: (
                    args.quest_objective_min_remaining,
                    args.quest_objective_max_remaining,
                ),
                quest_readiness: args.quest_readiness,
                quest_waypoint_map_id: args.quest_waypoint_map_id,
                quest_on_captured_map: args.quest_on_captured_map,
                local_quest_offset: args.local_quest_offset,
                local_quest_soonest_first: args.local_quest_soonest_first,
                local_quest_highest_xp_first: args.local_quest_highest_xp_first,
                quest_max_remaining_seconds: args.quest_max_remaining_seconds,
                quest_projected_expiry_by: args.quest_projected_expiry_by,
                merchant_item_id: args.merchant_item_id,
                merchant_currency_id: args.merchant_currency_id,
                merchant_pet_food_snapshot_id: args.merchant_pet_food_snapshot_id.as_deref(),
                merchant_target_quantity: args.merchant_target_quantity,
                merchant_cheapest_first: args.merchant_cheapest_first,
                merchant_combine_bundles: args.merchant_combine_bundles,
                merchant_max_total_copper: args.merchant_max_total_copper,
                merchant_max_price_copper: args.merchant_max_price_copper,
                merchant_in_stock_only: args.merchant_in_stock_only,
                merchant_stock_state: args.merchant_stock_state,
                merchant_copper_only: args.merchant_copper_only,
                merchant_affordable_only: args.merchant_affordable_only,
                trainer_service_name: args.trainer_service_name.as_deref(),
                trainer_service_status: args.trainer_service_status.as_deref(),
                trainer_max_price_copper: args.trainer_max_price_copper,
                trainer_max_price_points: args.trainer_max_price_points,
                trainer_affordable_only: args.trainer_affordable_only,
                trainer_balance_reserve: args.trainer_balance_reserve,
                flight_destination_node_id: args.flight_destination_node_id,
                flight_origin_node_id: args.flight_origin_node_id,
                flight_destination_name: args.flight_destination_name.as_deref(),
                flight_max_fare_copper: args.flight_max_fare_copper,
                flight_departure_map_id: args.flight_departure_map_id,
                flight_affordable_only: args.flight_affordable_only,
                flight_wallet_reserve_copper: args.flight_wallet_reserve_copper,
            },
            args.offset.unwrap_or(0),
            args.limit.unwrap_or(20),
        ) {
            Ok(v) => CallToolResult::success(vec![ContentBlock::text(v.to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(
                "Snapshot listing failed: check capture time bounds, character, feature filters and pagination limits",
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
        if args.instance_group_size_max_cache_age_seconds.is_some()
            && !args.instance_group_size_reference
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "A cache age limit requires the instance group size reference",
            )]);
        }
        if args.instance_group_size_reference && args.section.as_deref() != Some("instance_context")
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Instance group size review requires the instance_context section",
            )]);
        }
        if args.group_member_target.is_some() && args.section.as_deref() != Some("group_context") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Group member targets require the group_context section",
            )]);
        }
        if args.repair_include_equipment_condition
            && args.section.as_deref() != Some("repair_quote")
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Repair equipment condition requires repair_quote",
            )]);
        }
        if args.repair_wallet_reserve_copper.is_some_and(|reserve| {
            reserve > 9_999_999_999_999 || args.section.as_deref() != Some("repair_quote")
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Repair reserve requires repair_quote and a valid copper amount",
            )]);
        }
        if let Some(maximum) = args.equipment_durability_max_percent
            && (maximum > 100 || args.section.as_deref() != Some("equipment_durability"))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "A durability threshold needs equipment_durability and a percentage from 0 to 100",
            )]);
        }
        let filter_pet_rows =
            args.pet_has_owned_rows.is_some() || args.pet_has_unowned_rows.is_some();
        if filter_pet_rows
            && (args.section.as_deref() != Some("pet_collection_view")
                || args.pet_species.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Pet row filtering requires the pet_collection_view section without an exact species selector",
            )]);
        }
        let filter_mounts = args.mount_usable.is_some()
            || args.mount_favorite.is_some()
            || args.mount_spell_id.is_some();
        if filter_mounts
            && (args.section.as_deref() != Some("mount_collection")
                || args.mount_id.is_some()
                || args.collected_mount.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Mount filtering requires the mount_collection section without an exact mount selector",
            )]);
        }
        let toy_name = args.toy_name.as_deref().map(str::trim);
        if toy_name.is_some_and(|name| name.is_empty() || name.len() > 256) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Toy name requires one to 256 bytes after trimming",
            )]);
        }
        let toy_name = toy_name.map(str::to_lowercase);
        let filter_toys =
            args.toy_collected.is_some() || args.toy_favorite.is_some() || toy_name.is_some();
        if filter_toys
            && (args.section.as_deref() != Some("toy_collection_view")
                || args.toy_item_id.is_some()
                || args.toy_in_view.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Toy filtering requires the toy_collection_view section without an exact toy selector",
            )]);
        }
        if args.toy_in_view.is_some()
            && (args.section.as_deref() != Some("toy_collection_view")
                || args.toy_item_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Toy lookup requires the toy_collection_view section without a separate toy ID",
            )]);
        }
        if args.collected_mount.is_some() && args.section.as_deref() != Some("mount_collection") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Collected mount lookup requires the mount_collection section",
            )]);
        }
        if args.pet_species.as_ref().is_some_and(|filter| {
            args.section.as_deref() != Some("pet_collection_view") || filter.validate().is_err()
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Pet species lookup requires pet_collection_view and ordered row bounds",
            )]);
        }
        if args.saved_reset_reference_at.is_some() && !args.saved_reset_projection {
            return CallToolResult::error(vec![ContentBlock::text(
                "A reset reference time requires saved reset projection",
            )]);
        }
        if args.saved_reset_projection
            && !matches!(
                args.section.as_deref(),
                Some("saved_instances" | "saved_world_bosses")
            )
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Reset projection requires a saved reset section",
            )]);
        }
        if args.bag_cooldown_target_quantity.is_some()
            && (!args.action_bar_item_stock || args.bag_cooldown_reference_at.is_none())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Cooldown quantity targets require exact action-bar item stock and a reference time",
            )]);
        }
        if args.bag_cooldown_reference_at.is_some()
            && args.bag_cooldown_filter.is_none()
            && !args.action_bar_item_stock
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Bag cooldown reference requires a cooldown filter or exact action-bar item stock lookup",
            )]);
        }
        if args.bag_cooldown_filter.as_ref().is_some_and(|filter| {
            args.section.as_deref() != Some("bag_item_cooldowns") || filter.validate().is_err()
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Bag cooldown filters require bag_item_cooldowns, a nonempty filter, distinct item IDs, ordered timer bounds and a limit from 1 to 50",
            )]);
        }
        if args.skill_line_filter.as_ref().is_some_and(|filter| {
            args.section.as_deref() != Some("skill_lines") || filter.validate().is_err()
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Skill-line inspection requires skill_lines, a nonempty ordered filter and limit from 1 to 50",
            )]);
        }
        if [
            args.mail_attachment_bag_space_item_ids.as_ref(),
            args.mail_attachment_headroom_currency_ids.as_ref(),
        ]
        .into_iter()
        .flatten()
        .any(|ids| {
            args.section.as_deref() != Some("mail_attachments")
                || !(1..=64).contains(&ids.len())
                || ids.iter().collect::<std::collections::BTreeSet<_>>().len() != ids.len()
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Mail projections require mail_attachments and one to 64 distinct positive IDs per list",
            )]);
        }
        if [
            args.mail_attachment_bag_space_message_slot,
            args.mail_attachment_headroom_message_slot,
        ]
        .into_iter()
        .flatten()
        .any(|slot| args.section.as_deref() != Some("mail_attachments") || slot.get() > 100)
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Mail message projections require mail_attachments and a message slot from 1 to 100",
            )]);
        }
        if args.weapon_enchant_reference_at.is_some()
            && args.section.as_deref() != Some("weapon_enchants")
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Weapon enchant expiry review requires weapon_enchants",
            )]);
        }
        if args.instance_lock_encounter_review
            && args.section.as_deref() != Some("instance_context")
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Instance lock review requires instance_context",
            )]);
        }
        if args.instance_lock_warning_reference_at.is_some() && !args.instance_lock_encounter_review
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Instance lock warning reference requires encounter review",
            )]);
        }
        if args.pvp_target_rank.is_some()
            && args.section.as_deref() != Some("player_pvp_progression")
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "PvP rank targets require player_pvp_progression",
            )]);
        }
        if args.loot_estimate_bag_space && args.loot_requirements.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Loot bag space requires loot requirements",
            )]);
        }
        if let Some(requirements) = args.loot_requirements.as_ref() {
            if args.section.as_deref() != Some("loot_window")
                || azeroth_operator::analysis::validate_loot_requirements(requirements).is_err()
            {
                return CallToolResult::error(vec![ContentBlock::text(
                    "Loot requirements need the loot_window section and one to 64 distinct positive item IDs and quantities",
                )]);
            }
        }
        if args.trade_offer_compare_carried && args.trade_offer_requirements.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Trade carried comparison requires trade offer requirements",
            )]);
        }
        if let Some(requirements) = args.trade_offer_requirements.as_ref()
            && (args.section.as_deref() != Some("player_trade_offer")
                || azeroth_operator::analysis::validate_loot_requirements(requirements).is_err())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Trade offer review needs the player_trade_offer section and one to 64 distinct positive item IDs and quantities",
            )]);
        }
        if args.profession_spell_id.is_some()
            && args.section.as_deref() != Some("profession_spells")
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Profession spell lookup requires the profession_spells section",
            )]);
        }
        if args.buyback_estimate_bag_space && args.buyback_slots.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Buyback bag space requires selected slots",
            )]);
        }
        if args.buyback_wallet_reserve_copper.is_some() && args.buyback_slots.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Buyback reserve requires selected buyback slots",
            )]);
        }
        if let Some(slots) = &args.buyback_slots {
            let unique: std::collections::BTreeSet<_> =
                slots.iter().map(|slot| slot.get()).collect();
            if args.section.as_deref() != Some("merchant_buyback")
                || !(1..=12).contains(&slots.len())
                || unique.len() != slots.len()
                || slots.iter().any(|slot| slot.get() > 12)
                || args
                    .buyback_wallet_reserve_copper
                    .is_some_and(|reserve| reserve > 9_999_999_999_999)
            {
                return CallToolResult::error(vec![ContentBlock::text(
                    "Buyback review requires merchant_buyback, 1 to 12 distinct slots and a valid reserve",
                )]);
            }
        }
        let trainer_name = args.trainer_service_name.as_deref().map(str::trim);
        let trainer_status = args.trainer_service_status.as_deref();
        let trainer_slots = args.trainer_service_slots.as_ref().map(|slots| {
            slots
                .iter()
                .map(|slot| slot.to_string())
                .collect::<std::collections::BTreeSet<_>>()
        });
        if let Some(slots) = &args.trainer_service_slots
            && (!(1..=32).contains(&slots.len())
                || slots.iter().any(|slot| slot.get() > 512)
                || trainer_slots
                    .as_ref()
                    .is_some_and(|keys| keys.len() != slots.len()))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Expected 1 to 32 distinct trainer slots from 1 to 512",
            )]);
        }
        if args.trainer_max_price_copper.is_some_and(|copper| {
            copper > 9_999_999_999_999 || args.trainer_max_price_points.is_some()
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Specify one valid trainer price ceiling",
            )]);
        }
        let trainer_price = args
            .trainer_max_price_copper
            .map(|amount| ("price_copper", amount))
            .or_else(|| {
                args.trainer_max_price_points
                    .map(|amount| ("price_training_points", u64::from(amount)))
            });
        if args.trainer_balance_reserve.is_some()
            && !(args.trainer_affordable_only
                || args.trainer_wallet_projection
                || args.trainer_points_projection)
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Trainer balance reserve requires the affordability filter or a budget projection",
            )]);
        }
        let filter_trainer = args.trainer_affordable_only
            || trainer_name.is_some()
            || trainer_status.is_some()
            || trainer_slots.is_some()
            || trainer_price.is_some();
        let review_trainer = args.trainer_wallet_projection
            || args.trainer_points_projection
            || args.trainer_requirements_review
            || filter_trainer;
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
        if args.merchant_stock_state.is_some()
            && merchant_search.is_none()
            && args.merchant_pet_food_snapshot_id.is_none()
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Merchant stock state requires an item, currency or pet food search",
            )]);
        }
        if args.merchant_target_quantity.is_some_and(|quantity| {
            (merchant_search.is_none() && args.merchant_pet_food_snapshot_id.is_none())
                || quantity.get() > 1000
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Merchant quantity planning requires an item, currency or pet food search and 1 to 1000 units",
            )]);
        }
        if args.merchant_cart_include_repair && args.merchant_cart.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Repair reservation requires a merchant cart",
            )]);
        }
        if args.merchant_cart_estimate_bag_space && args.merchant_cart.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Merchant bag space requires a merchant cart",
            )]);
        }
        if let Some(cart) = &args.merchant_cart
            && (args.section.as_deref() != Some("merchant_stock")
                || args.merchant_offer_slot.is_some()
                || args.merchant_bundles.is_some()
                || merchant_search.is_some()
                || !(1..=16).contains(&cart.len())
                || cart
                    .iter()
                    .any(|line| line.slot.get() > 256 || line.bundles.get() > 1000)
                || cart
                    .iter()
                    .map(|line| line.slot)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != cart.len())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Merchant cart requires merchant_stock and up to sixteen distinct slots with 1 to 1000 bundles each",
            )]);
        }
        if merchant_search.is_some()
            && (args.section.as_deref() != Some("merchant_stock")
                || args.merchant_offer_slot.is_some()
                || (args.merchant_item_id.is_some() && args.merchant_currency_id.is_some()))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Merchant search requires merchant_stock and exactly one item or currency ID without a slot",
            )]);
        }
        if args.merchant_pet_food_snapshot_id.is_some()
            && (args.section.as_deref() != Some("merchant_stock")
                || merchant_search.is_some()
                || args.merchant_offer_slot.is_some()
                || args.merchant_cart.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Merchant pet food search requires merchant_stock without another offer selector or cart",
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
        let filter_flight = args.flight_node_id.is_some()
            || args.flight_reachable_only
            || args.flight_affordable_only
            || args.flight_max_fare_copper.is_some();
        if args
            .flight_max_fare_copper
            .is_some_and(|fare| fare > 9_999_999_999_999)
        {
            return CallToolResult::error(vec![ContentBlock::text("Invalid flight fare ceiling")]);
        }
        if args.flight_onward_legs.as_ref().is_some_and(|legs| {
            args.flight_node_id.is_none()
                || !(1..=7).contains(&legs.len())
                || args.flight_affordable_only
                || args.flight_max_fare_copper.is_some()
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Flight itinerary requires a destination and one to seven onward legs, without affordability or fare filters",
            )]);
        }
        if args.flight_itinerary_include_repair_quote && args.flight_onward_legs.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Flight itinerary repair budgeting requires explicit onward legs",
            )]);
        }
        if args.flight_wallet_reserve_copper.is_some_and(|reserve| {
            reserve > 9_999_999_999_999
                || !(args.flight_wallet_projection
                    || args.flight_affordable_only
                    || args.flight_route_search.is_some()
                    || args.flight_onward_legs.is_some())
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Flight reserve requires an affordability filter, wallet projection, route search or itinerary and a valid copper amount",
            )]);
        }
        if args.flight_route_search.as_ref().is_some_and(|search| {
            search
                .onward_snapshot_ids
                .as_ref()
                .is_some_and(|ids| ids.len() > 7)
                || args.flight_onward_legs.is_some()
                || filter_flight
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Flight route search accepts up to seven onward captures and no flight node filters or manual itinerary",
            )]);
        }
        if (args.flight_wallet_projection || filter_flight || args.flight_route_search.is_some())
            && args.section.as_deref() != Some("flight_map")
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Flight review requires the flight_map section",
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
        let expiry_from = args.quest_projected_expiry_from;
        let expiry_by = args.quest_projected_expiry_by;
        let filter_quest_expiry = expiry_from.is_some() || expiry_by.is_some();
        let filter_quest_timer = args.quest_max_remaining_seconds.is_some() || filter_quest_expiry;
        if expiry_from
            .zip(expiry_by)
            .is_some_and(|(from, by)| from > by)
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Projected quest expiry window is reversed",
            )]);
        }
        let filter_quest_objectives = args.quest_objective_kind.is_some()
            || args.quest_objective_finished.is_some()
            || args.quest_objective_max_remaining.is_some()
            || args.quest_objective_min_remaining.is_some();
        if args
            .quest_objective_min_remaining
            .zip(args.quest_objective_max_remaining)
            .is_some_and(|(min, max)| min > max)
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest objective remaining bounds are reversed",
            )]);
        }
        if args
            .quest_objective_kind
            .as_ref()
            .is_some_and(|kind| kind.is_empty() || kind.len() > 64)
            || (filter_quest_objectives
                && (args.section.as_deref() != Some("quests") || args.quest_id.is_some()))
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Objective filters require the quests section without a quest ID and an exact kind of 1 to 64 bytes",
            )]);
        }
        if args.quest_waypoint_map_id.is_some() && args.quest_on_captured_map {
            return CallToolResult::error(vec![ContentBlock::text(
                "Choose either an explicit waypoint map or the captured map",
            )]);
        }
        let filter_spellbook = args.spellbook_spell_id.is_some()
            || args.spellbook_passive.is_some()
            || args.spellbook_off_spec.is_some()
            || args.spellbook_name.is_some()
            || args.spellbook_resource_filter.is_some();
        let action_bar_hotkey = args.action_bar_hotkey.as_deref().map(str::trim);
        if action_bar_hotkey.is_some_and(|key| key.is_empty() || key.len() > 128) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Action bar hotkey requires 1 to 128 UTF-8 bytes",
            )]);
        }
        let action_spell_name = args.action_bar_spell_name.as_deref().map(str::trim);
        if action_spell_name.is_some_and(|name| name.is_empty() || name.len() > 256) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Action bar spell name requires 1 to 256 UTF-8 bytes",
            )]);
        }
        let action_spell_name = action_spell_name.map(str::to_lowercase);
        if (args.action_bar_spell_id.is_some() || action_spell_name.is_some())
            && args.action_bar_item_id.is_some()
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Choose either a spell ID or an item ID for an action bar search",
            )]);
        }
        if args.action_bar_item_stock && args.action_bar_item_id.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Action bar item stock requires an item ID",
            )]);
        }
        if args.action_bar_item_stock_target_quantity.is_some() && !args.action_bar_item_stock_audit
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Action bar stock targets require the item stock audit",
            )]);
        }
        let action_id_filter = args
            .action_bar_spell_id
            .map(|id| ("spell", id))
            .or_else(|| args.action_bar_item_id.map(|id| ("item", id)));
        let review_action_bar = args.action_bar_spellbook
            || args.action_bar_spell_resource_filter.is_some()
            || args.action_bar_item_stock_audit
            || args.action_bar_mount_audit
            || args.action_bar_toy_audit
            || args.action_bar_shared_hotkeys
            || args.action_bar_slot.is_some()
            || action_id_filter.is_some()
            || action_spell_name.is_some()
            || action_bar_hotkey.is_some();
        if review_action_bar && args.section.as_deref() != Some("action_bar_observation") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Action bar slot and spellbook review require the action_bar_observation section",
            )]);
        }
        if args
            .action_bar_spell_resource_filter
            .as_ref()
            .is_some_and(|filter| filter.validate().is_err())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Action bar spell resource filters require nonempty, ordered predicates",
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
        if args.quest_inventory_bank_withdrawal_offset.is_some()
            && !args.quest_inventory_bank_withdrawals
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest withdrawal location paging requires bank withdrawals",
            )]);
        }
        if (args.quest_inventory_catalogue_sources
            || args.quest_inventory_bank_withdrawals
            || args.quest_inventory_source_snapshot_id.is_some()
            || args.quest_inventory_merchant_snapshot_id.is_some()
            || args.quest_inventory_snapshot_id.is_some())
            && args.quest_inventory_requirements.is_none()
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest inventory, merchant and source options require quest inventory requirements",
            )]);
        }
        if args.quest_inventory_bank_withdrawals
            && args
                .quest_inventory_requirements
                .as_ref()
                .is_some_and(|requirements| {
                    requirements.container_id.is_some()
                        || !matches!(
                            requirements.storage,
                            azeroth_operator::InventoryStorage::Bags
                        )
                })
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest bank withdrawal planning requires a storage-wide bag review",
            )]);
        }
        if let Some(requirements) = &args.quest_inventory_requirements
            && (args.quest_id.is_none()
                || args.section.as_deref() != Some("quests")
                || requirements.validate(&Family::Forever).is_err())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest inventory review requires a quest ID, the quests section and valid item requirements",
            )]);
        }
        if args.currency_reward_quantity.is_some() && args.currency_id.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Currency reward comparison requires a currency ID",
            )]);
        }
        if args
            .bag_restock_wallet_reserve_copper
            .is_some_and(|reserve| {
                reserve > 9_999_999_999_999
                    || (args.bag_restock_merchant_snapshot_id.is_none()
                        && !args.bag_restock_split_merchant_plan)
            })
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "A restocking wallet reserve requires a saved merchant quote or split plan and a valid copper amount",
            )]);
        }
        if args.bag_restock_mail_snapshot_id.is_some() && !args.bag_restock_pending_mail {
            return CallToolResult::error(vec![ContentBlock::text(
                "A saved inbox selection requires bag_restock_pending_mail",
            )]);
        }
        if args
            .bag_restock_compare_merchant_snapshot_ids
            .as_ref()
            .is_some_and(|ids| {
                !(1..=4).contains(&ids.len())
                    || ids.iter().collect::<std::collections::BTreeSet<_>>().len() != ids.len()
            })
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Restocking comparison requires one to four distinct saved merchant snapshots",
            )]);
        }
        if args.bag_restock_split_merchant_plan
            && args.bag_restock_compare_merchant_snapshot_ids.is_none()
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "A split restocking plan requires saved merchant comparison snapshots",
            )]);
        }
        if args
            .bag_restock_cost_per_merchant_visit_copper
            .is_some_and(|cost| cost > 9_999_999_999_999 || !args.bag_restock_split_merchant_plan)
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Merchant visit cost requires a split restocking plan and a valid copper amount",
            )]);
        }
        if args
            .bag_restock_merchant_visit_costs_copper
            .as_ref()
            .is_some_and(|costs| {
                !args.bag_restock_split_merchant_plan
                    || args.bag_restock_cost_per_merchant_visit_copper.is_some()
                    || args
                        .bag_restock_compare_merchant_snapshot_ids
                        .as_ref()
                        .is_none_or(|ids| costs.len() != ids.len())
                    || costs.iter().any(|cost| *cost > 9_999_999_999_999)
            })
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Supply one valid cost per compared visit with split planning, without a uniform cost",
            )]);
        }
        if args
            .bag_restock_max_merchant_visits
            .is_some_and(|cap| cap > 4 || !args.bag_restock_split_merchant_plan)
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "A merchant visit cap requires split restocking and a limit from zero to four",
            )]);
        }
        let bag_restock = args.bag_restock_requirements.map(|items| {
            azeroth_operator::InventoryRequirementsFilter {
                storage: azeroth_operator::InventoryStorage::Bags,
                container_id: None,
                items,
            }
        });
        if ((args.bag_restock_location_offset.is_some()
            || args.bag_restock_merchant_snapshot_id.is_some()
            || args.bag_restock_compare_merchant_snapshot_ids.is_some()
            || args.bag_restock_source_snapshot_id.is_some()
            || args.bag_restock_catalogue_sources
            || args.bag_restock_pending_mail
            || args.bag_restock_use_reported_stack_limits
            || args.bag_restock_loot_snapshot_id.is_some())
            && bag_restock.is_none())
            || bag_restock.as_ref().is_some_and(|requirements| {
                args.section.as_deref() != Some("bags")
                    || args.spending_budget.is_some()
                    || requirements.validate(&Family::Forever).is_err()
            })
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Bag restocking requires the bags section and one to 64 distinct positive item requirements",
            )]);
        }
        let stack_limits = if let Some(limits) = &args.bag_restock_stack_limits {
            let index: std::collections::BTreeMap<_, _> = limits
                .iter()
                .map(|row| (row.item_id.get(), row.maximum_stack))
                .collect();
            if args.bag_restock_use_reported_stack_limits
                || limits.is_empty()
                || index.len() != limits.len()
                || !bag_restock.as_ref().is_some_and(|requirements| {
                    index.keys().all(|id| {
                        requirements
                            .items
                            .iter()
                            .any(|row| row.item_id.get() == *id)
                    })
                })
            {
                return CallToolResult::error(vec![ContentBlock::text(
                    "Supply distinct requested item stack limits or use reported limits",
                )]);
            }
            Some(index)
        } else {
            None
        };
        if args.bag_restock_include_bank_stack_limits && !args.bag_restock_use_reported_stack_limits
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Bank stack limits require reported stack limit estimates",
            )]);
        }
        if args.profession_target_rank.is_some() && args.profession_skill_line.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Profession rank targets require a profession skill line",
            )]);
        }
        if args.profession_target_trainer_lessons && args.profession_target_rank.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Profession trainer lesson review requires a profession rank target",
            )]);
        }
        if args
            .profession_target_trainer_reserve_copper
            .is_some_and(|reserve| {
                !args.profession_target_trainer_lessons || reserve > 9_999_999_999_999
            })
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "A profession trainer reserve requires lesson review and a valid copper amount",
            )]);
        }
        if args.reputation_points_per_turn_in.is_some() && args.faction_id.is_none() {
            return CallToolResult::error(vec![ContentBlock::text(
                "Reputation turn-in estimates require an exact faction",
            )]);
        }
        if args
            .reputation_turn_in_copper_cost
            .as_ref()
            .is_some_and(|cost| {
                args.reputation_points_per_turn_in.is_none()
                    || cost.wallet_reserve_copper > 9_999_999_999_999
            })
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Reputation copper costs require a points-per-turn-in estimate and a valid reserve",
            )]);
        }
        let reputation_item_costs = args
            .reputation_turn_in_item_costs
            .as_deref()
            .unwrap_or_else(|| args.reputation_turn_in_item_cost.as_slice());
        if args.reputation_turn_in_pending_mail
            && args.reputation_turn_in_bank_withdrawal_offset.is_none()
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Reputation pending mail requires a bank withdrawal plan",
            )]);
        }
        if args.reputation_turn_in_bank_withdrawal_offset.is_some()
            && reputation_item_costs.is_empty()
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Reputation bank withdrawal review requires item costs",
            )]);
        }
        if (args.reputation_turn_in_item_cost.is_some()
            || args.reputation_turn_in_item_costs.is_some())
            && (args.reputation_points_per_turn_in.is_none()
                || reputation_item_costs.is_empty()
                || reputation_item_costs.len() > 20
                || (args.reputation_turn_in_item_cost.is_some()
                    && args.reputation_turn_in_item_costs.is_some())
                || reputation_item_costs
                    .iter()
                    .map(|cost| cost.item_id)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != reputation_item_costs.len())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Reputation item costs require a points-per-turn-in estimate and one to twenty distinct items",
            )]);
        }
        if let Some(budget) = args.spending_budget {
            if args.coverage_sections.is_some() {
                return CallToolResult::error(vec![ContentBlock::text(
                    "Spending budgets cannot be combined with section coverage inspection",
                )]);
            }
            if args.section.is_some()
                || args.quest_id.is_some()
                || filter_quest_map
                || filter_quest_objectives
                || args.quest_difficulty.is_some()
                || args.quest_readiness.is_some()
                || filter_quest_timer
                || args.quest_max_required_copper.is_some()
                || args.quest_min_reward_xp.is_some()
                || args.quest_min_reward_honor.is_some()
                || args.quest_list_xp_projection
                || args.quest_ready_honor_target.is_some()
                || args.quest_list_wallet_projection
                || args.quest_list_currency_projection
                || args.quest_ready_currency_target.is_some()
                || args.quest_ready_item_target.is_some()
                || args.quest_list_renown_projection
                || args.quest_ready_renown_target.is_some()
                || args.quest_list_item_rewards
                || args.quest_list_timed_checklist
                || args.quest_list_objective_checklist
                || args.quest_list_ready_waypoint_groups
                || args.quest_within_captured_wallet
                || filter_rewards
                || args.currency_id.is_some()
                || args.currency_filter.is_some()
                || args.faction_id.is_some()
                || args.reputation_filter.is_some()
                || args.achievement_id.is_some()
                || args.achievement_filter.is_some()
                || args.mount_id.is_some()
                || args.toy_item_id.is_some()
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
        if filter_quest_timer
            && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest timer selection requires the quests section without a quest ID",
            )]);
        }
        if args.quest_list_timed_checklist
            && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Timed quest checklist requires the quests section without a quest ID",
            )]);
        }
        if args.quest_list_objective_checklist
            && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Objective quest checklist requires the quests section without a quest ID",
            )]);
        }
        if args.quest_list_ready_waypoint_groups
            && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Ready quest waypoint groups require the quests section without a quest ID",
            )]);
        }
        if (args.quest_max_required_copper.is_some() || args.quest_within_captured_wallet)
            && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest cost filtering requires the quests section without a quest ID",
            )]);
        }
        if (filter_rewards
            || args.quest_min_reward_xp.is_some()
            || args.quest_min_reward_honor.is_some()
            || args.quest_list_xp_projection
            || args.quest_ready_honor_target.is_some()
            || args.quest_list_wallet_projection
            || args.quest_list_currency_projection
            || args.quest_ready_currency_target.is_some()
            || args.quest_ready_item_target.is_some()
            || args.quest_list_renown_projection
            || args.quest_ready_renown_target.is_some()
            || args.quest_list_item_rewards)
            && (args.section.as_deref() != Some("quests") || args.quest_id.is_some())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Quest reward selection requires the quests section without a quest ID",
            )]);
        }
        if args.currency_filter.as_ref().is_some_and(|filter| {
            filter.is_empty()
                || args.section.as_deref() != Some("currency_balances")
                || args.currency_id.is_some()
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Currency filters require nonempty predicates and currency_balances without currency_id",
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
        if args.reputation_filter.as_ref().is_some_and(|filter| {
            !filter.valid()
                || args.section.as_deref() != Some("reputation")
                || args.faction_id.is_some()
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Reputation filters require reputation without faction_id, nonempty compatible predicates and explicit progress kind",
            )]);
        }
        if args.achievement_id.is_some() && args.section.as_deref() != Some("tracked_achievements")
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Achievement selection requires the tracked_achievements section",
            )]);
        }
        if args.achievement_filter.as_ref().is_some_and(|filter| {
            filter.is_empty()
                || args.section.as_deref() != Some("tracked_achievements")
                || args.achievement_id.is_some()
        }) {
            return CallToolResult::error(vec![ContentBlock::text(
                "Achievement filters require nonempty predicates and tracked_achievements without an achievement ID",
            )]);
        }
        if args.mount_id.is_some() && args.section.as_deref() != Some("mount_collection") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Mount selection requires the mount_collection section",
            )]);
        }
        if args.toy_item_id.is_some() && args.section.as_deref() != Some("toy_collection_view") {
            return CallToolResult::error(vec![ContentBlock::text(
                "Toy selection requires the toy_collection_view section",
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
        if args
            .spellbook_resource_filter
            .as_ref()
            .is_some_and(|filter| filter.validate().is_err())
        {
            return CallToolResult::error(vec![ContentBlock::text(
                "Spell resource filtering requires nonempty predicates and ordered count bounds",
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
        match self.store.inspect_with_coverage(
            &args.snapshot_id,
            args.section.as_deref(),
            args.coverage_sections.as_deref(),
        ) {
            Ok(mut value) => {
                if let Some(requirements) = bag_restock {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Saved capture unavailable",
                        )]);
                    };
                    if snapshot.client.family != Family::Forever {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Bag restocking requires a saved Forever snapshot",
                        )]);
                    }
                    use azeroth_operator::analysis::RestockStackLimitSource;
                    let (stack_limits, limit_source) = if args.bag_restock_use_reported_stack_limits
                    {
                        let include_banks = args.bag_restock_include_bank_stack_limits;
                        let source = if include_banks {
                            RestockStackLimitSource::ReportedBagsAndBanks
                        } else {
                            RestockStackLimitSource::ReportedBags
                        };
                        (
                            Some(azeroth_operator::analysis::reported_inventory_stack_limits(
                                &snapshot,
                                include_banks,
                            )),
                            source,
                        )
                    } else {
                        (stack_limits, RestockStackLimitSource::Supplied)
                    };
                    value["bag_restock_review"] = requirements.review_observed_stock(
                        &snapshot,
                        Some(args.bag_restock_location_offset.unwrap_or_default()),
                    );
                    if let Some(limits) = &stack_limits {
                        value["bag_restock_new_stack_space"] =
                            serde_json::json!(azeroth_operator::analysis::restock_new_stack_space(
                                &snapshot,
                                &value["bag_restock_review"],
                                limits,
                                None,
                                limit_source,
                            ));
                    }
                    if let Some(id) = args.bag_restock_loot_snapshot_id.as_deref() {
                        let result = self.store.read(id).and_then(|loot| {
                            azeroth_operator::analysis::ensure_same_character_and_client(
                                &snapshot, &loot,
                            )?;
                            let residual = bank_restock_residual(&value["bag_restock_review"]);
                            let needs: Vec<_> = residual
                                .as_ref()
                                .and_then(|review| review["items"].as_array())
                                .into_iter()
                                .flatten()
                                .filter_map(|row| {
                                    Some(azeroth_operator::analysis::LootRequirement {
                                        item_id: std::num::NonZeroU32::new(
                                            u32::try_from(row["item_id"].as_u64()?).ok()?,
                                        )?,
                                        quantity: std::num::NonZeroU64::new(
                                            row["shortfall_vs_observed"].as_u64()?,
                                        )?,
                                    })
                                })
                                .collect();
                            if needs.is_empty() {
                                Ok(serde_json::Value::Null)
                            } else {
                                azeroth_operator::analysis::review_loot_requirements(&loot, &needs)
                            }
                        });
                        let Ok(mut loot) = result else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Bag restocking requires a compatible saved loot snapshot",
                            )]);
                        };
                        if loot.is_object() {
                            loot["snapshot_id"] = serde_json::json!(id);
                        }
                        value["bag_restock_loot_review"] = loot;
                    }
                    if args.bag_restock_pending_mail {
                        let mail_id = args.bag_restock_mail_snapshot_id.as_deref();
                        let mail_snapshot = if let Some(id) = mail_id {
                            let Ok(mail) = self.store.read(id).and_then(|mail| {
                                azeroth_operator::analysis::ensure_same_character_and_client(
                                    &snapshot, &mail,
                                )?;
                                Ok(mail)
                            }) else {
                                return CallToolResult::error(vec![ContentBlock::text(
                                    "Bag restocking requires a compatible saved inbox snapshot",
                                )]);
                            };
                            Some(mail)
                        } else {
                            None
                        };
                        let mut observations = bank_restock_residual(&value["bag_restock_review"])
                            .and_then(|residual| {
                                azeroth_operator::analysis::restock_pending_mail(
                                    mail_snapshot.as_ref().unwrap_or(&snapshot),
                                    &residual,
                                )
                            });
                        if let Some((report, id)) = observations.as_mut().zip(mail_id) {
                            report["snapshot_id"] = serde_json::json!(id);
                        }
                        value["bag_restock_pending_mail_observations"] =
                            serde_json::json!(observations);
                    }
                    if args.bag_restock_catalogue_sources {
                        let result = self
                            .forever_catalogue
                            .as_deref()
                            .ok_or_else(|| anyhow::anyhow!("Local catalogue unavailable"))
                            .and_then(azeroth_operator::forever_catalogue::EquipmentCatalogue::load)
                            .and_then(|catalogue| {
                                catalogue.inventory_shortfall_sources(
                                    &bank_restock_residual(&value["bag_restock_review"])
                                        .unwrap_or_default(),
                                    "user_supplied_bag_shortfalls_after_observed_bank_withdrawals",
                                )
                            });
                        let Ok(sources) = result else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Bag restocking catalogue sources require a verified local catalogue",
                            )]);
                        };
                        value["bag_restock_catalogue_source_review"] = sources;
                    }
                    if let Some(id) = args.bag_restock_source_snapshot_id.as_deref() {
                        let result = self.store.read(id).and_then(|source| {
                            azeroth_operator::analysis::inventory_shortfall_captured_sources(
                                &bank_restock_residual(&value["bag_restock_review"])
                                    .unwrap_or_default(),
                                &snapshot,
                                &source,
                                "user_supplied_bag_shortfalls_after_observed_bank_withdrawals",
                            )
                        });
                        let Ok(mut sources) = result else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Bag restocking requires a compatible saved source snapshot",
                            )]);
                        };
                        if sources.is_object() {
                            sources["snapshot_id"] = serde_json::json!(id);
                        }
                        value["bag_restock_captured_sources"] = sources;
                    }
                    if let Some(ids) = &args.bag_restock_compare_merchant_snapshot_ids {
                        let residual = bank_restock_residual(&value["bag_restock_review"]);
                        let demands = residual
                            .as_ref()
                            .and_then(|review| review["items"].as_array());
                        let uniform_cost = args.bag_restock_cost_per_merchant_visit_copper;
                        let visit_costs = args
                            .bag_restock_merchant_visit_costs_copper
                            .clone()
                            .or_else(|| {
                                uniform_cost
                                    .or(args.bag_restock_max_merchant_visits.map(|_| 0))
                                    .or(args
                                        .bag_restock_wallet_reserve_copper
                                        .filter(|_| args.bag_restock_split_merchant_plan)
                                        .map(|_| 0))
                                    .map(|cost| vec![cost; ids.len()])
                            });
                        let mut visits = Vec::new();
                        let mut best_by_item = std::collections::BTreeMap::new();
                        let mut travel_quotes = Vec::new();
                        for id in ids {
                            let Ok(merchant) = self.store.read(id).and_then(|merchant| {
                                azeroth_operator::analysis::ensure_same_character_and_client(
                                    &snapshot, &merchant,
                                )?;
                                Ok(merchant)
                            }) else {
                                return CallToolResult::error(vec![ContentBlock::text(
                                    "Restocking comparison requires compatible saved merchant snapshots",
                                )]);
                            };
                            let quote = demands.map(|rows| {
                                azeroth_operator::analysis::material_merchant_plan(
                                    rows, &merchant, &snapshot,
                                )
                            });
                            if args.bag_restock_split_merchant_plan
                                && let Some(rows) = quote
                                    .as_ref()
                                    .and_then(|quote| quote["reagents"].as_array())
                            {
                                for row in rows {
                                    let plan = &row["lowest_cost_copper_only_bundle_mix"];
                                    if let Some((item, cost)) = row["item_id"]
                                        .as_u64()
                                        .zip(plan["required_copper"].as_u64())
                                        && best_by_item
                                            .get(&item)
                                            .is_none_or(|(best, _, _)| cost < *best)
                                    {
                                        best_by_item.insert(item, (cost, id, plan.clone()));
                                    }
                                }
                            }
                            if visit_costs.is_some() {
                                travel_quotes.push(
                                    quote
                                        .as_ref()
                                        .and_then(|quote| quote["reagents"].as_array())
                                        .into_iter()
                                        .flatten()
                                        .filter_map(|row| {
                                            let plan = &row["lowest_cost_copper_only_bundle_mix"];
                                            Some((
                                                row["item_id"].as_u64()?,
                                                (plan["required_copper"].as_u64()?, plan.clone()),
                                            ))
                                        })
                                        .collect(),
                                );
                            }
                            visits.push(serde_json::json!({
                                "snapshot_id":id,"captured_at":merchant.captured_at,
                                "captured_player_map_id":merchant.sections.get("stats").and_then(|stats| stats["map_id"].as_u64()),
                                "quote_available":quote.as_ref().is_some_and(serde_json::Value::is_object),
                                "complete_quote_copper":quote.as_ref().and_then(|quote| quote["copper_only_bundle_mix_total"].as_u64()),
                                "unquoted_item_ids":quote.as_ref().map(|quote| &quote["copper_only_bundle_mix_unquoted_item_ids"]),
                                "affordable_from_inventory_wallet":quote.as_ref().and_then(|quote| quote["copper_only_bundle_mix_affordable"].as_bool()),
                            }));
                        }
                        let cheapest = visits
                            .iter()
                            .filter_map(|row| row["complete_quote_copper"].as_u64())
                            .min();
                        let cheapest_ids: Vec<_> = cheapest
                            .into_iter()
                            .flat_map(|cost| {
                                visits
                                    .iter()
                                    .filter(move |row| {
                                        row["complete_quote_copper"].as_u64() == Some(cost)
                                    })
                                    .map(|row| &row["snapshot_id"])
                            })
                            .collect();
                        if let Some(costs) = visit_costs.as_deref() {
                            value["bag_restock_travel_adjusted_merchant_plan"] =
                                serde_json::json!(demands.and_then(|rows| {
                                    let wallet = snapshot
                                        .sections
                                        .get("wallet")
                                        .and_then(|row| row["copper"].as_u64());
                                    let mut plan =
                                        azeroth_operator::analysis::restock_travel_adjusted_plan(
                                            rows,
                                            ids,
                                            &travel_quotes,
                                            costs,
                                            args.bag_restock_max_merchant_visits.unwrap_or(4),
                                            wallet,
                                        )?;
                                    if let Some(cost) = uniform_cost {
                                        plan["assumed_copper_per_visit"] = serde_json::json!(cost);
                                    } else {
                                        plan["assumed_visit_costs_copper"] =
                                            serde_json::json!(costs);
                                    }
                                    if let Some(cap) = args.bag_restock_max_merchant_visits {
                                        plan["maximum_merchant_visits"] = serde_json::json!(cap);
                                    }
                                    if let Some(reserve) = args.bag_restock_wallet_reserve_copper {
                                        let total = plan["complete_quote_copper"].as_u64()?;
                                        let spendable = wallet.map(|balance| balance.saturating_sub(reserve));
                                        plan["after_wallet_reserve"] = serde_json::json!({
                                            "wallet_reserve_copper":reserve,
                                            "spendable_copper":spendable,
                                            "reserve_shortfall_copper":wallet.map(|balance| reserve.saturating_sub(balance)),
                                            "plan_shortfall_vs_spendable_copper":spendable.map(|balance| total.saturating_sub(balance)),
                                            "affordable":wallet.map(|balance| balance.checked_sub(reserve)
                                                .is_some_and(|spendable| total <= spendable)),
                                        });
                                    }
                                    if let Some(checklist) = plan["merchant_visit_checklist"].as_array_mut() {
                                        for stop in checklist {
                                            if let Some(visit) = visits.iter().find(|visit| {
                                                visit["snapshot_id"] == stop["merchant_snapshot_id"]
                                            }) {
                                                stop["captured_player_map_id"] = visit["captured_player_map_id"].clone();
                                                stop["merchant_captured_at"] = visit["captured_at"].clone();
                                            }
                                        }
                                    }
                                    Some(plan)
                                }));
                        }
                        value["bag_restock_merchant_comparison"] = serde_json::json!({
                            "visits":visits,"lowest_complete_quote_copper":cheapest,
                            "lowest_complete_quote_snapshot_ids":cheapest_ids,
                            "inventory_captured_at":snapshot.captured_at,
                            "offers_combined_across_visits":false,"purchase_eligibility_verified":false,
                            "live":false,
                        });
                        if args.bag_restock_split_merchant_plan {
                            let plan = demands.map(|rows| {
                                let mut missing = Vec::new();
                                let mut items = Vec::new();
                                let mut subtotal = Some(0u64);
                                for row in rows {
                                    let item = row["item_id"].as_u64().unwrap();
                                    if row["shortfall_vs_observed"] == 0 {
                                        continue;
                                    }
                                    if let Some((cost, id, bundles)) = best_by_item.get(&item) {
                                        subtotal = subtotal.and_then(|sum| sum.checked_add(*cost));
                                        items.push(serde_json::json!({"item_id":item,
                                            "merchant_snapshot_id":id,"bundle_plan":bundles}));
                                    } else {
                                        missing.push(item);
                                    }
                                }
                                let total = subtotal.filter(|_| missing.is_empty());
                                let wallet = snapshot.sections.get("wallet").and_then(|row| row["copper"].as_u64());
                                serde_json::json!({"items":items,"unquoted_item_ids":missing,
                                    "quoted_subtotal_copper":subtotal,"complete_quote_copper":total,
                                    "affordable_from_inventory_wallet":total.zip(wallet).map(|(cost,balance)| cost <= balance),
                                    "offers_combined_within_item_across_visits":false,
                                    "purchase_eligibility_verified":false,"live":false})
                            });
                            value["bag_restock_split_merchant_plan"] = serde_json::json!(plan);
                        }
                    }
                    if let Some(id) = args.bag_restock_merchant_snapshot_id.as_deref() {
                        let Ok(merchant) = self.store.read(id).and_then(|merchant| {
                            azeroth_operator::analysis::ensure_same_character_and_client(
                                &snapshot, &merchant,
                            )?;
                            Ok(merchant)
                        }) else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Bag restocking requires a compatible saved merchant snapshot",
                            )]);
                        };
                        let mut quote = bank_restock_residual(&value["bag_restock_review"])
                            .and_then(|review| {
                                Some(azeroth_operator::analysis::material_merchant_plan(
                                    review["items"].as_array()?,
                                    &merchant,
                                    &snapshot,
                                ))
                            })
                            .unwrap_or(serde_json::Value::Null);
                        if quote.is_object() {
                            azeroth_operator::analysis::restock_merchant_offer_coverage(
                                &mut quote, &merchant,
                            );
                            quote["merchant_snapshot_id"] = serde_json::json!(id);
                            quote["inventory_captured_at"] =
                                serde_json::json!(snapshot.captured_at);
                            quote["purchase_eligibility_verified"] = serde_json::json!(false);
                        }
                        if let Some(reserve) = args.bag_restock_wallet_reserve_copper
                            && quote.is_object()
                        {
                            let wallet = snapshot
                                .sections
                                .get("wallet")
                                .and_then(|row| row["copper"].as_u64());
                            let spendable = wallet.map(|balance| balance.saturating_sub(reserve));
                            let reserve_shortfall =
                                wallet.map(|balance| reserve.saturating_sub(balance));
                            let known_shortfall = quote["copper_only_bundle_mix_quoted_subtotal"]
                                .as_u64()
                                .zip(spendable)
                                .map(|(cost, balance)| cost.saturating_sub(balance));
                            let affordable = if reserve_shortfall.is_some_and(|gap| gap > 0)
                                || known_shortfall.is_some_and(|gap| gap > 0)
                            {
                                Some(false)
                            } else {
                                quote["copper_only_bundle_mix_total"]
                                    .as_u64()
                                    .zip(spendable)
                                    .map(|(cost, balance)| cost <= balance)
                            };
                            quote["copper_only_bundle_mix_after_reserve"] = serde_json::json!({
                                "wallet_reserve_copper":reserve,
                                "spendable_copper":spendable,
                                "reserve_shortfall_copper":reserve_shortfall,
                                "known_shortfall_copper":known_shortfall,
                                "affordable":affordable,
                            });
                        }
                        if let Some(limits) = &stack_limits {
                            value["bag_restock_quoted_bundle_stack_space"] = serde_json::json!(
                                azeroth_operator::analysis::restock_new_stack_space(
                                    &snapshot,
                                    &value["bag_restock_review"],
                                    limits,
                                    Some(&quote),
                                    limit_source,
                                )
                            );
                        }
                        value["bag_restock_merchant_quote"] = quote;
                    }
                }
                if args.repair_wallet_reserve_copper.is_some()
                    || args.repair_include_equipment_condition
                {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read saved repair budget",
                        )]);
                    };
                    if snapshot.client.family != Family::Forever {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Repair review requires a saved Forever snapshot",
                        )]);
                    }
                    if let Some(reserve) = args.repair_wallet_reserve_copper {
                        value["repair_reserve_projection"] =
                            azeroth_operator::analysis::repair_reserve_projection(
                                &snapshot, reserve,
                            );
                    }
                    if args.repair_include_equipment_condition {
                        value["repair_equipment_condition"] = serde_json::json!(
                            azeroth_operator::analysis::repair_equipment_condition(&snapshot)
                        );
                    }
                }
                if let Some(maximum) = args.equipment_durability_max_percent {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Durability shortlisting requires a saved Forever snapshot",
                        )]);
                    }
                    let matches = azeroth_operator::analysis::equipment_durability_matching_slots(
                        &value["data"],
                        maximum,
                    );
                    value["equipment_durability_matching_slots"] = serde_json::json!(matches);
                }
                if let Some(spell_id) = args.profession_spell_id {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Profession spell lookup requires a saved Forever snapshot",
                        )]);
                    }
                    let data = &value["data"];
                    let observed = data["entries"]
                        .as_object()
                        .map(|entries| entries.contains_key(&spell_id.to_string()));
                    value["profession_spell_lookup"] = serde_json::json!({
                        "spell_id": spell_id, "profession_id": data["profession_id"],
                        "reported_in_capture": observed, "current_knowledge_verified": false,
                    });
                }
                if let Some(requirements) = args.trade_offer_requirements.as_ref() {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Trade offer review requires a saved Forever snapshot",
                        )]);
                    }
                    value["trade_offer_requirement_review"] =
                        azeroth_operator::analysis::review_trade_offer(
                            &value["data"],
                            requirements,
                        );
                    if args.trade_offer_compare_carried
                        && value["trade_offer_requirement_review"].is_object()
                    {
                        let result = self.store.read(&args.snapshot_id).and_then(|snapshot| {
                            azeroth_operator::analysis::trade_offer_carried_comparison(
                                &snapshot,
                                requirements,
                            )
                        });
                        let Ok(comparison) = result else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Could not read saved trade carried stock",
                            )]);
                        };
                        value["trade_offer_requirement_review"]["carried_comparison"] = comparison;
                    }
                }
                if filter_pet_rows {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Pet row filtering requires a saved Forever snapshot",
                        )]);
                    }
                    let count = value["data"]
                        .get_mut("entries")
                        .and_then(|entries| entries.as_object_mut())
                        .map(|entries| {
                            entries.retain(|_, row| {
                                args.pet_has_owned_rows.is_none_or(|flag| {
                                    row["owned_rows"]
                                        .as_u64()
                                        .is_some_and(|count| (count > 0) == flag)
                                }) && args.pet_has_unowned_rows.is_none_or(|flag| {
                                    row["unowned_rows"]
                                        .as_u64()
                                        .is_some_and(|count| (count > 0) == flag)
                                })
                            });
                            entries.len()
                        });
                    value["pet_species_matched_count"] = serde_json::json!(count);
                }
                if filter_mounts {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Mount filtering requires a saved Forever snapshot",
                        )]);
                    }
                    let count = value["data"]
                        .get_mut("entries")
                        .and_then(|entries| entries.as_object_mut())
                        .map(|entries| {
                            entries.retain(|_, row| {
                                args.mount_usable
                                    .is_none_or(|flag| row["usable"].as_bool() == Some(flag))
                                    && args
                                        .mount_favorite
                                        .is_none_or(|flag| row["favorite"].as_bool() == Some(flag))
                                    && args.mount_spell_id.is_none_or(|id| {
                                        row["spell_id"].as_u64() == Some(u64::from(id.get()))
                                    })
                            });
                            entries.len()
                        });
                    value["mount_matched_count"] = serde_json::json!(count);
                }
                if filter_toys {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Toy filtering requires a saved Forever snapshot",
                        )]);
                    }
                    let count = value["data"]
                        .get_mut("entries")
                        .and_then(|entries| entries.as_object_mut())
                        .map(|entries| {
                            entries.retain(|_, row| {
                                args.toy_collected
                                    .is_none_or(|flag| row["collected"].as_bool() == Some(flag))
                                    && args
                                        .toy_favorite
                                        .is_none_or(|flag| row["favorite"].as_bool() == Some(flag))
                                    && toy_name.as_ref().is_none_or(|name| {
                                        row["name"].as_str().is_some_and(|label| {
                                            label.trim().to_lowercase() == *name
                                        })
                                    })
                            });
                            entries.len()
                        });
                    value["toy_matched_count"] = serde_json::json!(count);
                }
                if let Some(filter) = &args.toy_in_view {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Toy lookup requires a saved Forever snapshot",
                        )]);
                    }
                    value["toy_in_view_match"] =
                        serde_json::json!(filter.match_view(&value["data"]));
                }
                if let Some(filter) = &args.collected_mount {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Collected mount lookup requires a saved Forever snapshot",
                        )]);
                    }
                    value["collected_mount_match"] =
                        serde_json::json!(filter.match_view(&value["data"]));
                }
                if let Some(filter) = &args.pet_species {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Pet species lookup requires a saved Forever snapshot",
                        )]);
                    }
                    value["pet_species_match"] =
                        serde_json::json!(filter.match_view(&value["data"]));
                }
                if args.saved_reset_projection {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Reset projection requires Forever",
                        )]);
                    }
                    let projected = value["captured_at"]
                        .as_str()
                        .and_then(|text| text.parse::<chrono::DateTime<chrono::Utc>>().ok())
                        .and_then(|date| {
                            azeroth_operator::saved_resets::project(
                                args.section.as_deref().unwrap(),
                                (!value["data"].is_null()).then_some(&value["data"]),
                                date,
                            )
                            .ok()
                        });
                    let Some(mut projected) = projected else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Invalid saved reset projection",
                        )]);
                    };
                    if let Some(reference) = args.saved_reset_reference_at {
                        azeroth_operator::saved_resets::classify_at(&mut projected, reference);
                    }
                    value["saved_reset_projection"] = projected;
                }
                if let Some(filter) = args.skill_line_filter.as_ref() {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Skill-line review requires Forever",
                        )]);
                    }
                    let section = value["data"].take();
                    match azeroth_operator::skill_line_filter::inspect(
                        (!section.is_null()).then_some(&section),
                        filter,
                    ) {
                        Ok(filtered) => value["data"] = filtered,
                        Err(_) => {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Invalid captured skill-line review",
                            )]);
                        }
                    }
                }
                if let Some(filter) = args.bag_cooldown_filter.as_ref() {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Bag cooldown review requires Forever",
                        )]);
                    }
                    let section = value["data"].take();
                    let Some(captured_at) = value["captured_at"]
                        .as_str()
                        .and_then(|text| text.parse::<chrono::DateTime<chrono::Utc>>().ok())
                    else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Invalid saved cooldown capture timestamp",
                        )]);
                    };
                    match azeroth_operator::analysis::filter_bag_cooldowns(
                        (!section.is_null()).then_some(&section),
                        filter,
                        captured_at,
                    ) {
                        Ok(mut review) => {
                            if let Some(reference) = args.bag_cooldown_reference_at {
                                azeroth_operator::analysis::project_bag_cooldown_reference(
                                    &mut review,
                                    captured_at,
                                    reference,
                                );
                            }
                            value["data"] = review;
                        }
                        Err(_) => {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Invalid captured bag cooldown review",
                            )]);
                        }
                    }
                }
                if let Some(reference) = args.weapon_enchant_reference_at {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Weapon enchant expiry review requires a saved Forever snapshot",
                        )]);
                    }
                    value["weapon_enchant_expiry_review"] = serde_json::json!(
                        value["captured_at"]
                            .as_str()
                            .and_then(|text| text.parse::<chrono::DateTime<chrono::Utc>>().ok())
                            .and_then(|captured_at| {
                                azeroth_operator::analysis::weapon_enchant_expiry_review(
                                    &value["data"],
                                    captured_at,
                                    reference,
                                )
                            })
                    );
                }
                if args.instance_lock_encounter_review {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Instance lock review requires a saved Forever snapshot",
                        )]);
                    }
                    value["instance_lock_encounter_review"] = serde_json::json!(
                        value["captured_at"]
                            .as_str()
                            .and_then(|text| text.parse::<chrono::DateTime<chrono::Utc>>().ok())
                            .and_then(|captured_at| {
                                azeroth_operator::saved_resets::instance_lock_encounter_review(
                                    &value["data"],
                                    captured_at,
                                    args.instance_lock_warning_reference_at,
                                )
                            })
                    );
                }
                if args.instance_group_size_reference {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read the captured instance group context",
                        )]);
                    };
                    if snapshot.client.family != Family::Forever {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Instance group size review requires Forever",
                        )]);
                    }
                    value["instance_group_size_reference"] =
                        azeroth_operator::analysis::instance_group_size_reference(
                            &snapshot,
                            args.instance_group_size_max_cache_age_seconds,
                        );
                }
                if let Some(target) = args.group_member_target {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Group member targets require Forever",
                        )]);
                    }
                    value["group_member_target_projection"] =
                        azeroth_operator::analysis::group_member_target_projection(
                            &value["data"],
                            target.get(),
                        );
                }
                if let Some(rank) = args.pvp_target_rank {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "PvP rank targets require a saved Forever snapshot",
                        )]);
                    }
                    value["pvp_rank_target_projection"] =
                        serde_json::json!(azeroth_operator::analysis::pvp_rank_target_projection(
                            &value["data"],
                            rank
                        ));
                }
                if args.mail_attachment_bag_space_item_ids.is_some()
                    || args.mail_attachment_bag_space_message_slot.is_some()
                    || args.mail_attachment_headroom_currency_ids.is_some()
                    || args.mail_attachment_headroom_message_slot.is_some()
                {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read the saved inbox capture",
                        )]);
                    };
                    if snapshot.client.family != azeroth_operator::Family::Forever {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Mail projections require a saved Forever snapshot",
                        )]);
                    }
                    if args.mail_attachment_bag_space_item_ids.is_some()
                        || args.mail_attachment_bag_space_message_slot.is_some()
                    {
                        value["mail_attachment_bag_space_projection"] = serde_json::json!(
                            azeroth_operator::analysis::mail_attachment_bag_space(
                                &snapshot,
                                args.mail_attachment_bag_space_item_ids.as_deref(),
                                args.mail_attachment_bag_space_message_slot
                                    .map(|slot| slot.get()),
                            )
                        );
                    }
                    if args.mail_attachment_headroom_currency_ids.is_some()
                        || args.mail_attachment_headroom_message_slot.is_some()
                    {
                        value["mail_attachment_currency_headroom"] = serde_json::json!(
                            azeroth_operator::analysis::mail_attachment_currency_headroom(
                                &snapshot,
                                args.mail_attachment_headroom_currency_ids.as_deref(),
                                args.mail_attachment_headroom_message_slot
                                    .map(|slot| slot.get()),
                            )
                        );
                    }
                }
                if let Some(requirements) = args.loot_requirements.as_ref() {
                    let reviewed = self.store.read(&args.snapshot_id).and_then(|snapshot| {
                        let mut review = azeroth_operator::analysis::review_loot_requirements(
                            &snapshot,
                            requirements,
                        )?;
                        if args.loot_estimate_bag_space && review.is_object() {
                            review["bag_space_projection"] =
                                azeroth_operator::analysis::loot_requirement_bag_space(
                                    &snapshot, &review,
                                );
                        }
                        Ok(review)
                    });
                    match reviewed {
                        Ok(review) => value["loot_requirement_review"] = review,
                        Err(_) => {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Loot requirement review needs a valid owned Forever capture",
                            )]);
                        }
                    }
                }
                if review_trainer && value["client"]["family"] != "forever" {
                    return CallToolResult::error(vec![ContentBlock::text(
                        "Trainer review requires a saved Forever snapshot",
                    )]);
                }
                let trainer_wallet_context = if args.trainer_wallet_projection
                    || (args.trainer_affordable_only
                        && value["data"]["kind"]
                            .as_str()
                            .is_some_and(|kind| kind != "pet"))
                {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read the captured trainer wallet context",
                        )]);
                    };
                    Some(snapshot)
                } else {
                    None
                };
                let trainer_wallet = trainer_wallet_context
                    .as_ref()
                    .and_then(|snapshot| snapshot.sections.get("wallet"));
                let (trainer_balance_field, trainer_balance) = if value["data"]["kind"] == "pet" {
                    (
                        "price_training_points",
                        value["data"]["pet_training_points_available"].as_u64(),
                    )
                } else {
                    (
                        "price_copper",
                        trainer_wallet.and_then(|wallet| wallet["copper"].as_u64()),
                    )
                };
                if args.trainer_affordable_only {
                    value["trainer_balance_observed"] =
                        serde_json::json!(trainer_balance.is_some());
                }
                let trainer_spendable = trainer_balance.and_then(|balance| {
                    balance.checked_sub(args.trainer_balance_reserve.unwrap_or_default())
                });
                if let Some(reserve) = args.trainer_balance_reserve {
                    value["trainer_balance_reserve"] = serde_json::json!(reserve);
                }
                if filter_trainer {
                    let missing_slots = args.trainer_service_slots.as_ref().and_then(|slots| {
                        value["data"]["entries"].as_object().map(|entries| {
                            slots
                                .iter()
                                .filter(|slot| !entries.contains_key(&slot.to_string()))
                                .collect::<Vec<_>>()
                        })
                    });
                    if args.trainer_service_slots.is_some() {
                        value["trainer_missing_requested_slots"] = serde_json::json!(missing_slots);
                    }
                    if args.trainer_affordable_only && trainer_balance.is_none() {
                        value["data"] = serde_json::Value::Null;
                    }
                    let count = value["data"]
                        .get_mut("entries")
                        .and_then(serde_json::Value::as_object_mut)
                        .map(|entries| {
                            entries.retain(|slot, lesson| {
                                trainer_slots
                                    .as_ref()
                                    .is_none_or(|slots| slots.contains(slot))
                                    && trainer_status.is_none_or(|status| {
                                        lesson["status"].as_str() == Some(status)
                                    })
                                    && trainer_name.as_ref().is_none_or(|search| {
                                        lesson["name"].as_str().is_some_and(|name| {
                                            name.to_lowercase().contains(search)
                                        })
                                    })
                                    && trainer_price.is_none_or(|(field, max)| {
                                        lesson[field].as_u64().is_some_and(|price| price <= max)
                                    })
                                    && (!args.trainer_affordable_only
                                        || lesson[trainer_balance_field]
                                            .as_u64()
                                            .zip(trainer_spendable)
                                            .is_some_and(|(price, balance)| price <= balance))
                            });
                            entries.len()
                        });
                    value["trainer_match_count"] = serde_json::json!(count);
                    if let Some(slots) = &args.trainer_service_slots {
                        value["trainer_filtered_requested_slots"] = serde_json::json!(
                            missing_slots.zip(value["data"]["entries"].as_object()).map(
                                |(missing, retained)| {
                                    slots
                                        .iter()
                                        .filter(|slot| {
                                            !missing.contains(slot)
                                                && !retained.contains_key(&slot.to_string())
                                        })
                                        .collect::<Vec<_>>()
                                }
                            )
                        );
                    }
                }
                if args.trainer_wallet_projection {
                    value["trainer_wallet_projection"] =
                        azeroth_operator::analysis::trainer_wallet_projection(
                            &value["data"],
                            trainer_wallet,
                            args.trainer_service_slots.as_deref(),
                        );
                }
                if args.trainer_points_projection {
                    value["trainer_points_projection"] =
                        azeroth_operator::analysis::trainer_points_projection(
                            &value["data"],
                            args.trainer_service_slots.as_deref(),
                        );
                    if value["trainer_points_projection"].is_object() {
                        let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Could not read the saved pet trainer context",
                            )]);
                        };
                        value["trainer_points_projection"]["active_pet_capture_context"] =
                            azeroth_operator::analysis::pet_trainer_capture_context(
                                &value["data"],
                                snapshot.sections.get("pet_care"),
                            );
                    }
                }
                if let Some(reserve) = args.trainer_balance_reserve {
                    let ordered_budget = args.trainer_service_slots.as_deref().map(|slots| {
                        azeroth_operator::analysis::trainer_order_budget(
                            &value["data"],
                            slots,
                            trainer_balance_field,
                            trainer_spendable,
                        )
                    });
                    let cheapest = azeroth_operator::analysis::trainer_cheapest_affordable(
                        &value["data"],
                        trainer_balance_field,
                        trainer_spendable,
                    );
                    let individual = azeroth_operator::analysis::trainer_lesson_affordability(
                        &value["data"],
                        trainer_balance_field,
                        trainer_balance,
                        reserve,
                    );
                    let (projection_key, total_key, subtotal_key) =
                        if trainer_balance_field == "price_training_points" {
                            (
                                "trainer_points_projection",
                                "total_price_points",
                                "known_price_subtotal_points",
                            )
                        } else {
                            (
                                "trainer_wallet_projection",
                                "total_price_copper",
                                "known_price_subtotal_copper",
                            )
                        };
                    if let Some(projection) = value
                        .get_mut(projection_key)
                        .and_then(|report| report.as_object_mut())
                    {
                        let total = projection
                            .get(total_key)
                            .and_then(serde_json::Value::as_u64);
                        let budget = total.zip(trainer_spendable);
                        let subtotal = projection
                            .get(subtotal_key)
                            .and_then(serde_json::Value::as_u64);
                        let reserve_covered = trainer_balance.map(|balance| reserve <= balance);
                        let known_budget_insufficient = subtotal
                            .zip(trainer_spendable)
                            .is_some_and(|(cost, spendable)| cost > spendable);
                        let shortfall = |cost: Option<u64>| {
                            cost.zip(trainer_balance).and_then(|(cost, balance)| {
                                azeroth_operator::analysis::reserved_balance_shortfall(
                                    cost, balance, reserve,
                                )
                            })
                        };
                        projection.insert("after_reserve".into(), serde_json::json!({
                            "reserve_covered":reserve_covered,
                            "spendable_balance":trainer_spendable,
                            "retained_lessons_affordable":if reserve_covered == Some(false) || known_budget_insufficient { Some(false) }
                                else { budget.map(|(cost, balance)| cost <= balance) },
                            "retained_lessons_shortfall":shortfall(total),
                            "known_lessons_and_reserve_shortfall":shortfall(subtotal),
                            "cheapest_affordable_known_lessons":cheapest,
                            "individual_lesson_affordability":individual
                        }));
                        if let Some(ordered_budget) = ordered_budget {
                            projection["after_reserve"]["ordered_lesson_budget"] = ordered_budget;
                        }
                    }
                }
                if args.trainer_requirements_review {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read the captured trainer requirements context",
                        )]);
                    };
                    let level = snapshot
                        .sections
                        .get("stats")
                        .and_then(|stats| stats["level"].as_u64());
                    let requirements = azeroth_operator::analysis::trainer_requirements_review(
                        &value["data"],
                        snapshot.sections.get("trainer_services"),
                        level,
                        snapshot.sections.get("skill_lines"),
                        snapshot.sections.get("spellbook"),
                        snapshot.sections.get("pet_care"),
                    );
                    for pointer in [
                        "/trainer_wallet_projection/ordered_lesson_budget/first_uncovered_lesson",
                        "/trainer_points_projection/ordered_lesson_budget/first_uncovered_lesson",
                        "/trainer_wallet_projection/after_reserve/ordered_lesson_budget/first_uncovered_lesson",
                        "/trainer_points_projection/after_reserve/ordered_lesson_budget/first_uncovered_lesson",
                    ] {
                        if let Some(blocked) = value.pointer_mut(pointer)
                            && let Some(slot) = blocked["slot"].as_u64()
                            && let Some(requirements) =
                                requirements["unavailable_lessons_by_slot"].get(slot.to_string())
                        {
                            blocked["observed_requirements"] = requirements.clone();
                        }
                    }
                    value["trainer_requirements_review"] = requirements;
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
                if let Some(food_id) = &args.merchant_pet_food_snapshot_id {
                    let result = (|| {
                        let merchant = self.store.read(&args.snapshot_id)?;
                        let food = self.store.read(food_id)?;
                        anyhow::ensure!(merchant.client.family == Family::Forever);
                        azeroth_operator::analysis::ensure_same_character_and_client(
                            &merchant, &food,
                        )?;
                        Ok::<_, anyhow::Error>(food)
                    })();
                    let Ok(food) = result else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Merchant pet food search requires compatible saved Forever captures",
                        )]);
                    };
                    let accepted = food
                        .sections
                        .get("pet_care")
                        .and_then(|pet| pet["carried_food"].as_object());
                    let count = if let Some(accepted) = accepted {
                        value["data"].as_object_mut().map(|offers| {
                            offers.retain(|_, offer| {
                                offer["item_id"]
                                    .as_u64()
                                    .is_some_and(|id| accepted.contains_key(&id.to_string()))
                            });
                            offers.len()
                        })
                    } else {
                        value["data"] = serde_json::Value::Null;
                        None
                    };
                    value["merchant_pet_food_snapshot_id"] = serde_json::json!(food_id);
                    value["merchant_match_count"] = serde_json::json!(count);
                }
                if let Some(state) = args.merchant_stock_state {
                    let count = value["data"].as_object_mut().map(|offers| {
                        offers.retain(|_, offer| state.matches(offer["available"].as_i64()));
                        offers.len()
                    });
                    value["merchant_match_count"] = serde_json::json!(count);
                }
                if let Some(quantity) = args.merchant_target_quantity {
                    value["merchant_quantity_plan"] =
                        azeroth_operator::analysis::merchant_quantity_plan(
                            &value["data"],
                            quantity.get(),
                            merchant_search.is_some(),
                        );
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
                if let Some(slots) = &args.buyback_slots {
                    let result = self.store.read(&args.snapshot_id).and_then(|snapshot| {
                        azeroth_operator::analysis::buyback_cost_review(
                            &snapshot,
                            &slots.iter().map(|slot| slot.get()).collect::<Vec<_>>(),
                            args.buyback_wallet_reserve_copper.unwrap_or_default(),
                            args.buyback_estimate_bag_space,
                        )
                    });
                    let Ok(review) = result else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not review saved buyback quotes",
                        )]);
                    };
                    value["buyback_cost_review"] = review;
                }
                if let Some(cart) = &args.merchant_cart {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Merchant cart requires a saved Forever snapshot",
                        )]);
                    }
                    let result = self.store.read(&args.snapshot_id).and_then(|snapshot| {
                        azeroth_operator::analysis::merchant_cart_budget(
                            &snapshot,
                            cart.iter()
                                .map(|line| (line.slot.get(), line.bundles.get())),
                            args.merchant_cart_include_repair,
                            args.merchant_cart_estimate_bag_space,
                        )
                    });
                    let Ok(result) = result else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not review the captured merchant cart",
                        )]);
                    };
                    value["data"] = serde_json::Value::Null;
                    value["merchant_cart_budget"] = result;
                }
                if (args.flight_wallet_projection
                    || filter_flight
                    || args.flight_route_search.is_some())
                    && value["client"]["family"] != "forever"
                {
                    return CallToolResult::error(vec![ContentBlock::text(
                        "Flight review requires a saved Forever snapshot",
                    )]);
                }
                let flight_context = if args.flight_wallet_projection || args.flight_affordable_only
                {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read the captured flight wallet context",
                        )]);
                    };
                    Some(snapshot)
                } else {
                    None
                };
                let flight_balance = flight_context
                    .as_ref()
                    .and_then(|snapshot| snapshot.sections.get("wallet"))
                    .and_then(|wallet| wallet["copper"].as_u64());
                let flight_reserve = args.flight_wallet_reserve_copper.unwrap_or_default();
                if args.flight_wallet_reserve_copper.is_some() {
                    value["flight_wallet_reserve_copper"] = serde_json::json!(flight_reserve);
                }
                if filter_flight {
                    let spendable =
                        flight_balance.and_then(|balance| balance.checked_sub(flight_reserve));
                    if args.flight_affordable_only {
                        value["flight_wallet_observed"] =
                            serde_json::json!(flight_balance.is_some());
                        if flight_balance.is_none() {
                            value["data"] = serde_json::Value::Null;
                        }
                    }
                    let count = value["data"]
                        .get_mut("entries")
                        .and_then(serde_json::Value::as_object_mut)
                        .map(|entries| {
                            entries.retain(|_, node| {
                                args.flight_node_id.is_none_or(|id| {
                                    node["node_id"].as_u64() == Some(u64::from(id.get()))
                                }) && (!(args.flight_reachable_only
                                    || args.flight_affordable_only
                                    || args.flight_max_fare_copper.is_some())
                                    || (node["state"] == "reachable"
                                        && node["map_transition"] == false))
                                    && args.flight_max_fare_copper.is_none_or(|max| {
                                        node["fare_copper"].as_u64().is_some_and(|fare| fare <= max)
                                    })
                                    && (!args.flight_affordable_only
                                        || node["fare_copper"]
                                            .as_u64()
                                            .zip(spendable)
                                            .is_some_and(|(fare, balance)| fare <= balance))
                            });
                            entries.len()
                        });
                    value["flight_match_count"] = serde_json::json!(count);
                    if let Some(max) = args.flight_max_fare_copper {
                        value["flight_max_fare_copper"] = serde_json::json!(max);
                    }
                }
                if args.flight_wallet_projection {
                    value["flight_wallet_projection"] =
                        azeroth_operator::analysis::flight_wallet_projection(
                            &value["data"],
                            flight_balance,
                            flight_reserve,
                        );
                }
                if let Some(onward) = &args.flight_onward_legs {
                    let result = (|| {
                        let mut legs = Vec::new();
                        let destination = args
                            .flight_node_id
                            .ok_or_else(|| anyhow::anyhow!("Missing destination"))?;
                        legs.push((
                            args.snapshot_id.as_str(),
                            self.store.read(&args.snapshot_id)?,
                            destination.get(),
                        ));
                        for leg in onward {
                            legs.push((
                                leg.snapshot_id.as_str(),
                                self.store.read(&leg.snapshot_id)?,
                                leg.destination_node_id.get(),
                            ));
                        }
                        azeroth_operator::analysis::review_flight_itinerary(
                            &legs,
                            args.flight_wallet_reserve_copper,
                            args.flight_itinerary_include_repair_quote,
                        )
                    })();
                    match result {
                        Ok(itinerary) => value["flight_itinerary"] = itinerary,
                        Err(_) => {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Flight itinerary requires connected reachable routes in compatible Forever captures",
                            )]);
                        }
                    }
                }
                if let Some(search) = &args.flight_route_search {
                    let result = (|| {
                        let ids = std::iter::once(args.snapshot_id.as_str()).chain(
                            search
                                .onward_snapshot_ids
                                .iter()
                                .flatten()
                                .map(String::as_str),
                        );
                        let captures = ids
                            .map(|id| Ok((id, self.store.read(id)?)))
                            .collect::<anyhow::Result<Vec<_>>>()?;
                        let capture_window = search
                            .maximum_capture_skew_seconds
                            .map(|maximum| {
                                let times = captures.iter().map(|(_, capture)| capture.captured_at);
                                let earliest =
                                    times.clone().min().expect("starting capture is present");
                                let latest = times.max().expect("starting capture is present");
                                anyhow::ensure!(
                                    latest - earliest
                                        <= chrono::TimeDelta::seconds(i64::from(maximum)),
                                    "Selected flight panels exceed the capture time span"
                                );
                                Ok(serde_json::json!({
                                    "earliest_captured_at":earliest,"latest_captured_at":latest,
                                    "maximum_skew_seconds":maximum
                                }))
                            })
                            .transpose()?;
                        let mut plan = azeroth_operator::analysis::plan_saved_flights(
                            &captures,
                            search.destination_node_id.map(|id| id.get()),
                            search.destination_name.as_deref(),
                            args.flight_wallet_reserve_copper,
                            search.max_legs.unwrap_or(8),
                            search.objective.unwrap_or_default(),
                            search.include_repair_quote,
                            search.require_affordable,
                            search.maximum_total_fare_copper,
                            search
                                .excluded_landing_node_ids
                                .as_deref()
                                .unwrap_or_default(),
                            search
                                .excluded_flyover_node_ids
                                .as_deref()
                                .unwrap_or_default(),
                        )?;
                        if let Some(window) = capture_window {
                            plan["selected_capture_window"] = window;
                        }
                        Ok::<_, anyhow::Error>(plan)
                    })();
                    match result {
                        Ok(plan) => value["flight_route_search"] = plan,
                        Err(_) => {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Flight route search requires compatible Forever captures within any requested capture time span, unambiguous flight panels, a destination ID or an unambiguous name, one to eight legs, and up to 32 distinct nodes per exclusion list. Landing exclusions cannot include the endpoints.",
                            )]);
                        }
                    }
                }
                if let Some(id) = args.quest_id {
                    let quests = value["data"].take();
                    let quest = quests.get(id.to_string());
                    value["quest_id"] = serde_json::json!(id.get());
                    value["quest_observed"] =
                        serde_json::json!(quests.as_object().map(|_| quest.is_some()));
                    value["unfinished_objectives"] =
                        azeroth_operator::analysis::unfinished_quest_objectives(quest);
                    if value["client"]["family"] == "forever" {
                        value["objective_quantities_by_kind"] = serde_json::json!(
                            quest
                                .and_then(|row| row["objectives"].as_object())
                                .map(azeroth_operator::analysis::quest_objective_quantities)
                        );
                    }
                    let review_snapshot = if args.quest_item_catalogue
                        || args.quest_inventory_requirements.is_some()
                        || args.quest_currency_projection
                        || args.quest_wallet_projection
                        || args.quest_renown_projection
                        || args.quest_xp_projection
                    {
                        if value["client"]["family"] != "forever" {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Quest review requires a saved Forever snapshot",
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
                    let inventory_override = if let Some(id) = &args.quest_inventory_snapshot_id {
                        let inventory = self.store.read(id).and_then(|inventory| {
                            azeroth_operator::analysis::ensure_comparable(
                                review_snapshot.as_ref().unwrap(),
                                &inventory,
                            )?;
                            Ok(inventory)
                        });
                        let Ok(inventory) = inventory else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Quest inventory requires a compatible same-time or newer capture",
                            )]);
                        };
                        Some(inventory)
                    } else {
                        None
                    };
                    let quest_stock = inventory_override.as_ref().or(review_snapshot.as_ref());
                    if let Some(requirements) = &args.quest_inventory_requirements {
                        value["quest_inventory_review"] =
                            serde_json::json!(quest.zip(quest_stock).map(|(_, snapshot)| {
                                requirements.review_observed_stock(
                                    snapshot,
                                    args.quest_inventory_bank_withdrawals.then_some(
                                        args.quest_inventory_bank_withdrawal_offset
                                            .unwrap_or_default(),
                                    ),
                                )
                            }));
                        if let Some(id) = &args.quest_inventory_snapshot_id
                            && let Some(report) = value["quest_inventory_review"].as_object_mut()
                        {
                            report.insert("inventory_snapshot_id".into(), serde_json::json!(id));
                            report.insert(
                                "inventory_captured_at".into(),
                                serde_json::json!(quest_stock.unwrap().captured_at),
                            );
                        }
                    }
                    let residual_review = bank_restock_residual(&value["quest_inventory_review"]);
                    if let Some(id) = &args.quest_inventory_source_snapshot_id {
                        let sources =
                            self.store.read(id).and_then(|source| {
                                let quest_snapshot = review_snapshot.as_ref().unwrap();
                                let mut report =
                                    azeroth_operator::analysis::quest_inventory_captured_sources(
                                        &value["quest_inventory_review"],
                                        quest_snapshot,
                                        &source,
                                    )?;
                                if let Some(residual) = &residual_review {
                                    report["after_observed_bank_withdrawals"] =
                                    azeroth_operator::analysis::quest_inventory_captured_sources(
                                        residual, quest_snapshot, &source,
                                    )?;
                                }
                                Ok(report)
                            });
                        let Ok(mut sources) = sources else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Quest inventory sources require a compatible saved source capture",
                            )]);
                        };
                        if sources.is_object() {
                            sources["snapshot_id"] = serde_json::json!(id);
                        }
                        value["quest_inventory_captured_source_review"] = sources;
                    }
                    if args.quest_inventory_catalogue_sources {
                        let sources = self
                            .forever_catalogue
                            .as_deref()
                            .ok_or_else(|| anyhow::anyhow!("Local catalogue unavailable"))
                            .and_then(azeroth_operator::forever_catalogue::EquipmentCatalogue::load)
                            .and_then(|catalogue| {
                                let mut report = catalogue
                                    .quest_inventory_sources(&value["quest_inventory_review"])?;
                                if let Some(residual) = &residual_review {
                                    report["after_observed_bank_withdrawals"] =
                                        catalogue.quest_inventory_sources(residual)?;
                                }
                                Ok(report)
                            });
                        let Ok(sources) = sources else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Quest inventory sources require a verified local catalogue",
                            )]);
                        };
                        value["quest_inventory_catalogue_source_review"] = sources;
                    }
                    if let Some(merchant_id) = &args.quest_inventory_merchant_snapshot_id {
                        let snapshot = quest_stock.unwrap();
                        let merchant = self.store.read(merchant_id).and_then(|merchant| {
                            azeroth_operator::analysis::ensure_same_character_and_client(
                                snapshot, &merchant,
                            )?;
                            Ok(merchant)
                        });
                        let Ok(merchant) = merchant else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Quest merchant quotes require a compatible saved merchant snapshot",
                            )]);
                        };
                        let mut quote = value["quest_inventory_review"]["items"]
                            .as_array()
                            .map(|rows| {
                                azeroth_operator::analysis::material_merchant_plan(
                                    rows, &merchant, snapshot,
                                )
                            })
                            .unwrap_or(serde_json::Value::Null);
                        if quote.is_object() {
                            quote["merchant_snapshot_id"] = serde_json::json!(merchant_id);
                            quote["inventory_captured_at"] =
                                serde_json::json!(snapshot.captured_at);
                            quote["purchase_eligibility_verified"] = serde_json::json!(false);
                            quote["quest_item_binding_verified"] = serde_json::json!(false);
                            if args.quest_inventory_bank_withdrawals {
                                quote["after_observed_bank_withdrawals"] = residual_review
                                    .as_ref()
                                    .and_then(|review| review["items"].as_array())
                                    .map(|rows| {
                                        azeroth_operator::analysis::material_merchant_plan(
                                            rows, &merchant, snapshot,
                                        )
                                    })
                                    .unwrap_or(serde_json::Value::Null);
                            }
                        }
                        value["quest_inventory_merchant_quote"] = quote;
                    }
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
                    || filter_quest_objectives
                    || args.quest_difficulty.is_some()
                    || args.quest_readiness.is_some()
                    || filter_quest_timer
                    || args.quest_max_required_copper.is_some()
                    || args.quest_min_reward_xp.is_some()
                    || args.quest_min_reward_honor.is_some()
                    || args.quest_list_xp_projection
                    || args.quest_ready_honor_target.is_some()
                    || args.quest_list_wallet_projection
                    || args.quest_list_currency_projection
                    || args.quest_ready_currency_target.is_some()
                    || args.quest_ready_item_target.is_some()
                    || args.quest_list_renown_projection
                    || args.quest_ready_renown_target.is_some()
                    || args.quest_list_item_rewards
                    || args.quest_list_timed_checklist
                    || args.quest_list_objective_checklist
                    || args.quest_list_ready_waypoint_groups
                    || args.quest_within_captured_wallet
                    || filter_rewards
                {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Quest filters require a saved Forever snapshot",
                        )]);
                    }
                    let mut data = value["data"].take();
                    let context = if args.quest_on_captured_map
                        || args.quest_within_captured_wallet
                        || args.quest_list_xp_projection
                        || args.quest_ready_honor_target.is_some()
                        || args.quest_list_wallet_projection
                        || args.quest_list_currency_projection
                        || args.quest_ready_currency_target.is_some()
                        || args.quest_ready_item_target.is_some()
                        || args.quest_list_renown_projection
                        || args.quest_ready_renown_target.is_some()
                    {
                        let snapshot = match self.store.read(&args.snapshot_id) {
                            Ok(snapshot) => snapshot,
                            Err(_) => {
                                return CallToolResult::error(vec![ContentBlock::text(
                                    "Could not read the saved quest filter context",
                                )]);
                            }
                        };
                        Some(snapshot)
                    } else {
                        None
                    };
                    let map = if args.quest_on_captured_map {
                        let map = context.as_ref().and_then(|snapshot| {
                            snapshot
                                .sections
                                .get("stats")
                                .and_then(|stats| stats["map_id"].as_u64())
                        });
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
                    if filter_quest_objectives {
                        value["quest_objective_filter"] = serde_json::json!({
                            "kind": args.quest_objective_kind,
                            "finished": args.quest_objective_finished,
                            "maximum_remaining": args.quest_objective_max_remaining,
                        });
                        if let Some(minimum) = args.quest_objective_min_remaining {
                            value["quest_objective_filter"]["minimum_remaining"] =
                                serde_json::json!(minimum);
                        }
                        value["quest_objective_unknown_count"] =
                            serde_json::json!(data.as_object_mut().map(|quests| {
                                let mut unknown = 0;
                                quests.retain(|_, quest| {
                                    let matched =
                                        azeroth_operator::analysis::quest_objective_matches(
                                            quest,
                                            args.quest_objective_kind.as_deref(),
                                            args.quest_objective_finished,
                                            args.quest_objective_min_remaining,
                                            args.quest_objective_max_remaining,
                                        );
                                    unknown += usize::from(matched.is_none());
                                    matched == Some(true)
                                });
                                unknown
                            }));
                    }
                    if let Some(maximum) = args.quest_max_remaining_seconds {
                        value["quest_max_remaining_seconds"] = serde_json::json!(maximum);
                    }
                    let mut projected_expiries = serde_json::Map::new();
                    if let Some(start) = expiry_from {
                        value["quest_projected_expiry_from"] = serde_json::json!(start);
                    }
                    if let Some(deadline) = args.quest_projected_expiry_by {
                        value["quest_projected_expiry_by"] = serde_json::json!(deadline);
                    }
                    if filter_quest_timer {
                        let captured_at = value["captured_at"]
                            .as_str()
                            .and_then(|text| text.parse::<chrono::DateTime<chrono::Utc>>().ok());
                        value["quest_timer_unknown_count"] =
                            serde_json::json!(data.as_object_mut().map(|quests| {
                                let mut unknown = 0;
                                quests.retain(|id, quest| {
                                    let Some(seconds) =
                                        quest["timer"]["remaining_seconds"].as_u64()
                                    else {
                                        unknown += 1;
                                        return false;
                                    };
                                    if args
                                        .quest_max_remaining_seconds
                                        .is_some_and(|max| seconds > u64::from(max))
                                    {
                                        return false;
                                    }
                                    if !filter_quest_expiry {
                                        return true;
                                    }
                                    let expiry = i64::try_from(seconds)
                                        .ok()
                                        .and_then(chrono::TimeDelta::try_seconds)
                                        .and_then(|delta| captured_at?.checked_add_signed(delta));
                                    unknown += usize::from(expiry.is_none());
                                    expiry.is_some_and(|expiry| {
                                        if expiry_from.is_some_and(|start| expiry < start)
                                            || expiry_by.is_some_and(|deadline| expiry > deadline)
                                        {
                                            return false;
                                        }
                                        projected_expiries
                                            .insert(id.clone(), serde_json::json!(expiry));
                                        true
                                    })
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
                    for (reward, minimum) in [
                        ("xp", args.quest_min_reward_xp),
                        ("honor", args.quest_min_reward_honor),
                    ] {
                        let Some(minimum) = minimum else { continue };
                        value[format!("quest_min_reward_{reward}")] = serde_json::json!(minimum);
                        value[format!("quest_reward_{reward}_unknown_count")] =
                            serde_json::json!(data.as_object_mut().map(|quests| {
                                let mut unknown = 0;
                                quests.retain(|_, quest| {
                                    let amount = quest["reward_amounts"][reward].as_u64();
                                    unknown += usize::from(amount.is_none());
                                    amount.is_some_and(|amount| amount >= minimum)
                                });
                                unknown
                            }));
                    }
                    if let Some(maximum) = args.quest_max_required_copper {
                        value["quest_max_required_copper"] = serde_json::json!(maximum);
                    }
                    let maximum = if args.quest_within_captured_wallet {
                        let wallet = context.as_ref().and_then(|snapshot| {
                            snapshot
                                .sections
                                .get("wallet")
                                .and_then(|wallet| wallet["copper"].as_u64())
                        });
                        value["quest_within_captured_wallet"] = serde_json::json!(true);
                        value["quest_wallet_observed"] = serde_json::json!(wallet.is_some());
                        if wallet.is_none() {
                            data = serde_json::Value::Null;
                            value["quest_cost_unknown_count"] = serde_json::Value::Null;
                        }
                        wallet.map(|balance| {
                            args.quest_max_required_copper
                                .map_or(balance, |cap| balance.min(cap))
                        })
                    } else {
                        args.quest_max_required_copper
                    };
                    if let Some(maximum) = maximum {
                        value["quest_cost_unknown_count"] =
                            serde_json::json!(data.as_object_mut().map(|quests| {
                                let mut unknown = 0;
                                quests.retain(|_, quest| {
                                    let cost = quest["required_copper"].as_u64();
                                    unknown += usize::from(cost.is_none());
                                    cost.is_some_and(|cost| cost <= maximum)
                                });
                                unknown
                            }));
                    }
                    value["quest_checklist"] =
                        data.as_object().map_or(serde_json::Value::Null, |quests| {
                            azeroth_operator::analysis::quest_checklist(quests)
                        });
                    if args.quest_list_item_rewards {
                        value["quest_list_item_rewards"] = data.as_object().map_or(
                            serde_json::Value::Null,
                            azeroth_operator::analysis::quest_list_item_rewards,
                        );
                    }
                    if args.quest_list_xp_projection {
                        let experience = context
                            .as_ref()
                            .and_then(|snapshot| snapshot.sections.get("experience"));
                        value["quest_list_xp_projection"] =
                            data.as_object().map_or(serde_json::Value::Null, |quests| {
                                azeroth_operator::analysis::quest_list_xp_projection(
                                    quests, experience,
                                )
                            });
                    }
                    if let Some(target) = args.quest_ready_honor_target {
                        value["quest_ready_honor_projection"] =
                            data.as_object().map_or(serde_json::Value::Null, |quests| {
                                azeroth_operator::analysis::quest_ready_honor_projection(
                                    quests,
                                    target.get(),
                                    context
                                        .as_ref()
                                        .and_then(|snapshot| snapshot.sections.get("wallet")),
                                )
                            });
                    }
                    if args.quest_list_wallet_projection {
                        let wallet = context
                            .as_ref()
                            .and_then(|snapshot| snapshot.sections.get("wallet"));
                        if let Some(ids) = value["quest_list_xp_projection"]["ready_turn_in_plan"]
                            ["quest_ids_to_threshold"].as_array()
                            && let Some(quests) = data.as_object()
                            && let Some(xp_remaining) = value["quest_list_xp_projection"]["xp_to_next_level"].as_u64()
                        {
                            let selected = ids.iter().filter_map(|id| {
                                let key = id.as_u64()?.to_string();
                                Some((key.clone(), quests.get(&key)?.clone()))
                            }).collect();
                            value["quest_list_xp_projection"]["ready_turn_in_plan"]["wallet_projection"] =
                                azeroth_operator::analysis::quest_xp_wallet_projection(&selected, wallet, xp_remaining);
                        }
                        value["quest_list_wallet_projection"] =
                            data.as_object().map_or(serde_json::Value::Null, |quests| {
                                azeroth_operator::analysis::quest_list_wallet_projection(
                                    quests, wallet,
                                )
                            });
                    }
                    if args.quest_list_currency_projection {
                        let balances = context
                            .as_ref()
                            .and_then(|snapshot| snapshot.sections.get("currency_balances"))
                            .unwrap_or(&serde_json::Value::Null);
                        value["quest_list_currency_projection"] =
                            data.as_object().map_or(serde_json::Value::Null, |quests| {
                                azeroth_operator::analysis::quest_list_currency_projection(
                                    quests, balances,
                                )
                            });
                    }
                    if let Some(target) = &args.quest_ready_currency_target {
                        let balances = context
                            .as_ref()
                            .and_then(|snapshot| snapshot.sections.get("currency_balances"))
                            .unwrap_or(&serde_json::Value::Null);
                        value["quest_ready_currency_projection"] =
                            data.as_object().map_or(serde_json::Value::Null, |quests| {
                                azeroth_operator::analysis::quest_ready_reward_projection(
                                    quests,
                                    azeroth_operator::analysis::QuestRewardTarget::Currency(
                                        target.currency_id.get(),
                                        balances,
                                        context
                                            .as_ref()
                                            .and_then(|snapshot| snapshot.sections.get("wallet")),
                                    ),
                                    target.quantity.get(),
                                    target.include_choices,
                                )
                            });
                    }
                    if let Some(target) = &args.quest_ready_item_target {
                        value["quest_ready_item_projection"] =
                            data.as_object().map_or(serde_json::Value::Null, |quests| {
                                let item_id = target.item_id.get();
                                let quantity = target.quantity.get();
                                let carried = context
                                    .as_ref()
                                    .filter(|_| target.count_carried)
                                    .and_then(|snapshot| {
                                        azeroth_operator::analysis::find_item(
                                            snapshot, item_id, 0, 1,
                                        )
                                        .ok()
                                    })
                                    .and_then(|found| {
                                        found["quantity_by_storage"]["bags"].as_u64()
                                    });
                                let remaining = carried.map(|count| quantity.saturating_sub(count));
                                let reward_goal = remaining.unwrap_or(quantity);
                                let mut projection =
                                    azeroth_operator::analysis::quest_ready_reward_projection(
                                        quests,
                                        azeroth_operator::analysis::QuestRewardTarget::Item(
                                            item_id,
                                            context.as_ref().and_then(|snapshot| {
                                                snapshot.sections.get("wallet")
                                            }),
                                        ),
                                        reward_goal,
                                        target.include_choices,
                                    );
                                if target.count_carried {
                                    projection["target_quantity"] = serde_json::json!(quantity);
                                    projection["observed_carried_quantity"] =
                                        serde_json::json!(carried);
                                    projection["remaining_reward_quantity"] =
                                        serde_json::json!(remaining);
                                    projection["reward_quantity_used_for_plan"] =
                                        serde_json::json!(reward_goal);
                                    if carried.is_none() && projection["covers_target"] == false {
                                        projection["covers_target"] = serde_json::Value::Null;
                                    }
                                }
                                if target.estimate_bag_space {
                                    let plan = &projection["ready_turn_in_plan"];
                                    let affordable_quantity = plan["selected_quests_wallet_projection"]
                                        ["items_from_affordable_prefix"].as_u64();
                                    projection["bag_space_projection"] =
                                        serde_json::json!(context.as_ref().and_then(|snapshot| {
                                            azeroth_operator::analysis::quest_item_reward_bag_space(
                                                snapshot,
                                                item_id,
                                                plan,
                                            )
                                        }));
                                    projection["affordable_prefix_bag_space_projection"] = serde_json::json!(
                                        context.as_ref().zip(affordable_quantity).map(|(snapshot, quantity)| {
                                            azeroth_operator::analysis::quest_item_reward_quantity_bag_space(
                                                snapshot, item_id, quantity,
                                            )
                                        })
                                    );
                                }
                                projection
                            });
                    }
                    if args.quest_list_renown_projection {
                        let reputation = context
                            .as_ref()
                            .and_then(|snapshot| snapshot.sections.get("reputation"))
                            .unwrap_or(&serde_json::Value::Null);
                        value["quest_list_renown_projection"] =
                            data.as_object().map_or(serde_json::Value::Null, |quests| {
                                azeroth_operator::analysis::quest_list_renown_projection(
                                    quests, reputation,
                                )
                            });
                    }
                    if let Some(target) = &args.quest_ready_renown_target {
                        let reputation = context
                            .as_ref()
                            .and_then(|snapshot| snapshot.sections.get("reputation"))
                            .unwrap_or(&serde_json::Value::Null);
                        value["quest_ready_renown_projection"] =
                            data.as_object().map_or(serde_json::Value::Null, |quests| {
                                azeroth_operator::analysis::quest_ready_renown_projection(
                                    quests,
                                    reputation,
                                    context
                                        .as_ref()
                                        .and_then(|snapshot| snapshot.sections.get("wallet")),
                                    target.faction_id.get(),
                                    target.quantity.map(|quantity| quantity.get()),
                                )
                            });
                    }
                    if filter_quest_expiry {
                        value["quest_projected_expiry_by_quest_id"] =
                            serde_json::json!(data.as_object().map(|quests| {
                                projected_expiries.retain(|id, _| quests.contains_key(id));
                                projected_expiries
                            }));
                    }
                    let timed_at = args
                        .quest_list_timed_checklist
                        .then(|| {
                            value["captured_at"]
                                .as_str()?
                                .parse::<chrono::DateTime<chrono::Utc>>()
                                .ok()
                        })
                        .flatten();
                    if args.quest_list_timed_checklist {
                        value["timed_quest_checklist"] = serde_json::json!(
                            data.as_object().zip(timed_at).map(|(quests, captured_at)| {
                                azeroth_operator::analysis::timed_quest_checklist(
                                    quests,
                                    captured_at,
                                )
                            })
                        );
                    }
                    if args.quest_list_objective_checklist {
                        value["quest_objective_checklist"] = serde_json::json!(
                            data.as_object()
                                .map(azeroth_operator::analysis::quest_objective_checklist)
                        );
                    }
                    if args.quest_list_ready_waypoint_groups {
                        value["quest_ready_waypoint_groups"] =
                            serde_json::json!(data.as_object().map(|quests| {
                                let wallet = context
                                    .as_ref()
                                    .and_then(|snapshot| snapshot.sections.get("wallet"));
                                azeroth_operator::analysis::quest_ready_waypoint_groups(
                                    quests,
                                    args.quest_list_wallet_projection,
                                    wallet,
                                    timed_at,
                                )
                            }));
                    }
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
                        let expected = learning.observed();
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
                let mut name_evidence_available = false;
                let named_spell_ids = if let Some(name) = &action_spell_name {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read captured spellbook",
                        )]);
                    };
                    let book = snapshot
                        .sections
                        .get("spellbook")
                        .and_then(|book| book["entries"].as_object());
                    name_evidence_available = book.is_some();
                    let mut matches = std::collections::BTreeMap::new();
                    for row in book.into_iter().flat_map(|rows| rows.values()) {
                        if let Some(id) = row["spell_id"].as_u64() {
                            let matches_name = row["name"]
                                .as_str()
                                .is_some_and(|label| label.trim().to_lowercase() == *name);
                            matches
                                .entry(id)
                                .and_modify(|agrees| *agrees &= matches_name)
                                .or_insert(matches_name);
                        }
                    }
                    Some(matches)
                } else {
                    None
                };
                if action_id_filter.is_some() || named_spell_ids.is_some() {
                    let data = &mut value["data"];
                    let matched_slots = data
                        .get_mut("slots")
                        .and_then(serde_json::Value::as_object_mut)
                        .map(|slots| {
                            slots.retain(|_, row| {
                                action_id_filter.is_none_or(|(kind, id)| {
                                    row["kind"] == kind
                                        && row["id"].as_u64() == Some(u64::from(id.get()))
                                }) && named_spell_ids.as_ref().is_none_or(|ids| {
                                    row["kind"] == "spell"
                                        && row["id"]
                                            .as_u64()
                                            .is_some_and(|id| ids.get(&id) == Some(&true))
                                })
                            });
                            slots
                                .keys()
                                .filter_map(|slot| slot.parse::<u64>().ok())
                                .collect::<std::collections::BTreeSet<_>>()
                        });
                    if let Some(buttons) = data
                        .get_mut("button_hotkeys")
                        .and_then(serde_json::Value::as_object_mut)
                    {
                        buttons.retain(|_, row| {
                            row["slot"].as_u64().is_some_and(|slot| {
                                matched_slots
                                    .as_ref()
                                    .is_some_and(|matches| matches.contains(&slot))
                            })
                        });
                    }
                    if let Some((kind, id)) = action_id_filter {
                        value[format!("action_bar_{kind}_id")] = serde_json::json!(id);
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
                if let Some((kind, _)) = action_id_filter {
                    value[format!("action_bar_{kind}_match_count")] = serde_json::json!(
                        value["data"]["slots"].as_object().map(|slots| slots.len())
                    );
                }
                if named_spell_ids.is_some() {
                    value["action_bar_spell_name_match_count"] = serde_json::json!(
                        value["data"]["slots"]
                            .as_object()
                            .filter(|_| name_evidence_available)
                            .map(|slots| slots.len())
                    );
                    value["action_bar_spell_name_spellbook_observed"] =
                        serde_json::json!(name_evidence_available);
                }
                if args.action_bar_shared_hotkeys {
                    value["action_bar_shared_hotkeys"] = serde_json::json!(
                        azeroth_operator::analysis::shared_action_bar_hotkeys(&value["data"])
                    );
                }
                if args.action_bar_mount_audit || args.action_bar_toy_audit {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read captured action bar collection context",
                        )]);
                    };
                    if snapshot.client.family != Family::Forever {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Action bar collection audits require a saved Forever snapshot",
                        )]);
                    }
                    if args.action_bar_mount_audit {
                        value["action_bar_mount_audit"] =
                            serde_json::json!(azeroth_operator::analysis::action_bar_mount_audit(
                                &value["data"],
                                snapshot.sections.get("mount_collection")
                            ));
                    }
                    if args.action_bar_toy_audit {
                        value["action_bar_toy_audit"] =
                            serde_json::json!(azeroth_operator::analysis::action_bar_toy_audit(
                                &value["data"],
                                snapshot.sections.get("toy_collection_view")
                            ));
                    }
                }
                if args.action_bar_item_stock_audit {
                    let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Could not read captured action bar item stock",
                        )]);
                    };
                    value["action_bar_item_stock_audit"] =
                        serde_json::json!(azeroth_operator::analysis::action_bar_item_stock_audit(
                            &value["data"],
                            &snapshot,
                            args.action_bar_item_stock_target_quantity
                        ));
                }
                if args.action_bar_item_stock {
                    if args.bag_cooldown_reference_at.is_some()
                        && value["client"]["family"] != "forever"
                    {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Action-bar item cooldown projections require a saved Forever snapshot",
                        )]);
                    }
                    let mut stock = serde_json::Value::Null;
                    if let Some(id) = args.action_bar_item_id
                        && value["action_bar_item_match_count"]
                            .as_u64()
                            .is_some_and(|count| count > 0)
                    {
                        let result = self.store.read(&args.snapshot_id).and_then(|snapshot| {
                            azeroth_operator::analysis::find_item_with_container_quantities(
                                &snapshot,
                                id.get(),
                                0,
                                1,
                                false,
                                args.bag_cooldown_reference_at,
                                args.bag_cooldown_target_quantity,
                            )
                        });
                        let Ok(found) = result else {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Could not read captured action bar item stock",
                            )]);
                        };
                        let quantities = &found["quantity_by_storage"];
                        stock = serde_json::json!({"equipment_quantity":quantities["equipment"],
                            "bag_quantity":quantities["bags"]});
                        if let Some(reference) = args.bag_cooldown_reference_at {
                            stock["bag_cooldown_reference_at"] = serde_json::json!(reference);
                            stock["bag_cooldown_quantity_projection"] =
                                found["bag_cooldown_quantity_projection"].clone();
                        }
                        if args.bag_cooldown_target_quantity.is_some() {
                            stock["bag_cooldown_target_projection"] =
                                found["bag_cooldown_target_projection"].clone();
                        }
                    }
                    value["action_bar_item_stock"] = stock;
                }
                if args.action_bar_spellbook || args.action_bar_spell_resource_filter.is_some() {
                    let snapshot = match self.store.read(&args.snapshot_id) {
                        Ok(snapshot) => snapshot,
                        Err(_) => {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Could not read captured spellbook",
                            )]);
                        }
                    };
                    if args.action_bar_spellbook {
                        value["action_bar_spellbook"] =
                            azeroth_operator::analysis::action_bar_spellbook(
                                &value["data"],
                                snapshot.sections.get("spellbook"),
                                args.action_bar_slot.is_none()
                                    && action_bar_hotkey.is_none()
                                    && action_id_filter.is_none()
                                    && named_spell_ids.is_none(),
                            );
                    }
                    if let Some(filter) = args.action_bar_spell_resource_filter.as_ref() {
                        value["action_bar_spell_resource_matches"] = serde_json::json!(
                            azeroth_operator::analysis::action_bar_spell_resource_matches(
                                &value["data"],
                                snapshot.sections.get("spellbook"),
                                filter,
                                None
                            )
                        );
                    }
                }
                if filter_spellbook {
                    if value["client"]["family"] != "forever" {
                        return CallToolResult::error(vec![ContentBlock::text(
                            "Spellbook selection requires a saved Forever snapshot",
                        )]);
                    }
                    let mut resource_unknown = 0;
                    let mut resource_nonmatching = 0;
                    let observed = value["data"]
                        .get_mut("entries")
                        .and_then(serde_json::Value::as_object_mut)
                        .map(|rows| {
                            rows.retain(|_, row| {
                                let selected = args.spellbook_spell_id.is_none_or(|id| {
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
                                    });
                                selected
                                    && match args
                                        .spellbook_resource_filter
                                        .as_ref()
                                        .map(|filter| filter.matches(row))
                                    {
                                        Some(Some(false)) => {
                                            resource_nonmatching += 1;
                                            false
                                        }
                                        Some(None) => {
                                            resource_unknown += 1;
                                            false
                                        }
                                        _ => true,
                                    }
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
                    if let Some(resources) = args.spellbook_resource_filter.as_ref() {
                        value["spellbook_resource_filter"] = serde_json::json!(resources);
                        value["spellbook_resource_unknown_slots"] =
                            serde_json::json!(observed.map(|_| resource_unknown));
                        value["spellbook_resource_nonmatching_slots"] =
                            serde_json::json!(observed.map(|_| resource_nonmatching));
                        value["cast_eligibility_verified"] = serde_json::json!(false);
                    }
                }
                if args.section.as_deref() == Some("currency_balances") {
                    let mut headroom =
                        azeroth_operator::analysis::currency_cap_headroom(&value["data"]);
                    if let Some(filter) = args.currency_filter.as_ref() {
                        if value["client"]["family"] != "forever" {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Currency filters require a saved Forever snapshot",
                            )]);
                        }
                        value["currency_filter"] = serde_json::json!(filter);
                        value["currency_filter_unknown_count"] = serde_json::json!(
                            value["data"]
                                .get_mut("entries")
                                .and_then(serde_json::Value::as_object_mut)
                                .map(|entries| {
                                    let mut unknown = 0;
                                    entries.retain(|id, entry| {
                                        let matched = filter.matches(entry, &headroom[id]);
                                        unknown += usize::from(matched.is_none());
                                        matched == Some(true)
                                    });
                                    unknown
                                })
                        );
                        if let Some(rows) = headroom.as_object_mut() {
                            rows.retain(|id, _| value["data"]["entries"].get(id).is_some());
                        }
                    }
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
                    let mut standard =
                        azeroth_operator::analysis::standard_reputation_progress(&value["data"]);
                    let mut special =
                        azeroth_operator::analysis::special_reputation_progress(&value["data"]);
                    if let Some(filter) = args.reputation_filter.as_ref() {
                        if value["client"]["family"] != "forever" {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Reputation filters require a saved Forever snapshot",
                            )]);
                        }
                        value["reputation_filter"] = serde_json::json!(filter);
                        value["reputation_filter_unknown_count"] = serde_json::json!(
                            value["data"]
                                .get_mut("entries")
                                .and_then(serde_json::Value::as_object_mut)
                                .map(|entries| {
                                    let mut unknown = 0;
                                    entries.retain(|id, entry| {
                                        let progress = if entry["kind"] == "standard" {
                                            &standard[id]
                                        } else {
                                            &special[id]
                                        };
                                        let matched = filter.matches(entry, progress);
                                        unknown += usize::from(matched.is_none());
                                        matched == Some(true)
                                    });
                                    unknown
                                })
                        );
                        for progress in [&mut standard, &mut special] {
                            if let Some(rows) = progress.as_object_mut() {
                                rows.retain(|id, _| value["data"]["entries"].get(id).is_some());
                            }
                        }
                    }
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
                        if let Some(points) = args.reputation_points_per_turn_in {
                            let points = u64::from(points.get());
                            let remaining = standard[&key]["remaining_to_next_rank"].as_u64();
                            let turn_ins = remaining.map(|remaining| remaining.div_ceil(points));
                            value["reputation_turn_in_estimate"] = serde_json::json!({
                                "assumed_points_per_turn_in":points,
                                "remaining_to_next_rank":remaining,
                                "turn_ins_to_next_rank":turn_ins,
                                "capped":standard[&key]["capped"].as_bool()
                            });
                            let context = if !reputation_item_costs.is_empty()
                                || args.reputation_turn_in_copper_cost.is_some()
                            {
                                let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                                    return CallToolResult::error(vec![ContentBlock::text(
                                        "Could not read the captured reputation turn-in resources",
                                    )]);
                                };
                                Some(snapshot)
                            } else {
                                None
                            };
                            if let Some((cost, snapshot)) = args
                                .reputation_turn_in_copper_cost
                                .as_ref()
                                .zip(context.as_ref())
                            {
                                let balance = snapshot
                                    .sections
                                    .get("wallet")
                                    .and_then(|wallet| wallet["copper"].as_u64());
                                value["reputation_turn_in_wallet_plan"] =
                                    azeroth_operator::analysis::reputation_turn_in_wallet_plan(
                                        turn_ins,
                                        cost.copper_per_turn_in,
                                        balance,
                                        cost.wallet_reserve_copper,
                                    );
                            }
                            if let Some(snapshot) = context
                                .as_ref()
                                .filter(|_| !reputation_item_costs.is_empty())
                            {
                                let stock = azeroth_operator::InventoryRequirementsFilter {
                                    storage: azeroth_operator::InventoryStorage::Bags,
                                    container_id: None,
                                    items: reputation_item_costs
                                        .iter()
                                        .map(|cost| azeroth_operator::InventoryRequirement {
                                            item_id: cost.item_id,
                                            quantity: cost.quantity,
                                        })
                                        .collect(),
                                }
                                .review_observed_stock(snapshot, None);
                                let observed = stock["items"]
                                    .as_array()
                                    .expect("inventory review always returns item rows");
                                let plans: Vec<_> = reputation_item_costs.iter().zip(observed).map(|(cost, row)| {
                                    let carried = row["observed_quantity"].as_u64();
                                    let required = turn_ins.and_then(|count| count.checked_mul(cost.quantity.get()));
                                    serde_json::json!({
                                    "item_id":cost.item_id,
                                    "assumed_quantity_per_turn_in":cost.quantity,
                                    "required_quantity":required,
                                    "required_quantity_overflow":turn_ins.is_some() && required.is_none(),
                                    "observed_carried_quantity":carried,
                                    "shortfall_quantity":required.zip(carried).map(|(required, carried)| required.saturating_sub(carried))
                                    })
                                }).collect();
                                if let Some(offset) = args.reputation_turn_in_bank_withdrawal_offset
                                {
                                    let totals: Option<Vec<_>> = reputation_item_costs
                                        .iter()
                                        .map(|cost| {
                                            turn_ins?
                                                .checked_mul(cost.quantity.get())
                                                .map(|quantity| (cost.item_id, quantity))
                                        })
                                        .collect();
                                    value["reputation_turn_in_bank_withdrawals"] = serde_json::json!(totals.map(|totals| {
                                        let requirements = azeroth_operator::InventoryRequirementsFilter {
                                            storage: azeroth_operator::InventoryStorage::Bags,
                                            container_id: None,
                                            items: totals.into_iter().filter_map(|(item_id, quantity)| {
                                                Some(azeroth_operator::InventoryRequirement {
                                                    item_id, quantity: std::num::NonZeroU64::new(quantity)?,
                                                })
                                            }).collect(),
                                        };
                                        requirements.review_observed_stock(snapshot, Some(offset))["bank_withdrawal_plan"].clone()
                                    }));
                                    if args.reputation_turn_in_pending_mail {
                                        let rows =
                                            value["reputation_turn_in_bank_withdrawals"]["rows"]
                                                .as_array();
                                        value["reputation_turn_in_pending_mail_observations"] = serde_json::json!(rows.and_then(|rows| {
                                            let residual = serde_json::json!({"items":rows.iter().map(|row| serde_json::json!({
                                                "item_id":row["item_id"],
                                                "shortfall_vs_observed":row["remaining_after_observed_withdrawals"]
                                            })).collect::<Vec<_>>()});
                                            azeroth_operator::analysis::restock_pending_mail(snapshot, &residual)
                                        }));
                                    }
                                }
                                let capacities: Vec<_> = reputation_item_costs
                                    .iter()
                                    .zip(observed)
                                    .filter_map(|(cost, row)| {
                                        let carried = row["observed_quantity"].as_u64()?;
                                        Some((cost.item_id, carried / cost.quantity.get()))
                                    })
                                    .collect();
                                let supported = (capacities.len() == reputation_item_costs.len())
                                    .then(|| capacities.iter().map(|(_, count)| *count).min())
                                    .flatten();
                                if args.reputation_turn_in_copper_cost.is_some() {
                                    let wallet = value["reputation_turn_in_wallet_plan"]["supported_turn_ins"].as_u64();
                                    let combined = supported
                                        .zip(wallet)
                                        .map(|(bags, wallet)| bags.min(wallet));
                                    value["reputation_turn_in_combined_capacity"] = serde_json::json!({
                                        "supported_turn_ins":combined,
                                        "limiting_resources":combined.map(|capacity| {
                                            [("bags", supported), ("wallet", wallet)].into_iter()
                                                .filter_map(|(name, count)| (count == Some(capacity)).then_some(name))
                                                .collect::<Vec<_>>()
                                        }),
                                        "turn_ins_shortfall_to_next_rank":turn_ins.zip(combined)
                                            .map(|(needed, capacity)| needed.saturating_sub(capacity))
                                    });
                                }
                                if args.reputation_turn_in_item_cost.is_some() {
                                    value["reputation_turn_in_item_plan"] =
                                        serde_json::json!(plans.into_iter().next());
                                } else {
                                    value["reputation_turn_in_bag_capacity"] = serde_json::json!({
                                        "supported_turn_ins":supported,
                                        "limiting_item_ids":supported.map(|capacity| capacities.iter()
                                            .filter_map(|(id, count)| (*count == capacity).then_some(*id))
                                            .collect::<Vec<_>>()),
                                        "turn_ins_shortfall_to_next_rank":turn_ins.zip(supported)
                                            .map(|(needed, capacity)| needed.saturating_sub(capacity))
                                    });
                                    value["reputation_turn_in_item_plans"] =
                                        serde_json::json!(plans);
                                }
                            }
                        }
                    } else {
                        value["standard_reputation_progress"] = standard;
                        value["special_reputation_progress"] = special;
                    }
                } else if args.section.as_deref() == Some("tracked_achievements") {
                    if let Some(filter) = args.achievement_filter.as_ref() {
                        if value["client"]["family"] != "forever" {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Achievement filters require a saved Forever snapshot",
                            )]);
                        }
                        value["achievement_filter"] = serde_json::json!(filter);
                        value["achievement_filter_unknown_count"] = serde_json::json!(
                            value["data"]
                                .get_mut("entries")
                                .and_then(serde_json::Value::as_object_mut)
                                .map(|entries| {
                                    let mut unknown = 0;
                                    entries.retain(|_, achievement| {
                                        let matched = filter.matches(achievement);
                                        unknown += usize::from(matched.is_none());
                                        matched == Some(true)
                                    });
                                    unknown
                                })
                        );
                    }
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
                } else if args.section.as_deref() == Some("mount_collection") {
                    if let Some(id) = args.mount_id {
                        if value["client"]["family"] != "forever" {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Mount selection requires Forever",
                            )]);
                        }
                        let section = value["data"].take();
                        let row = section["entries"].get(id.to_string());
                        value["mount_id"] = serde_json::json!(id.get());
                        value["catalogue_count"] = section["catalogue_count"].clone();
                        value["observed_collected"] = serde_json::json!(row.map(|_| true));
                        value["data"] = row.cloned().unwrap_or_default();
                    }
                } else if args.section.as_deref() == Some("toy_collection_view") {
                    if let Some(id) = args.toy_item_id {
                        if value["client"]["family"] != "forever" {
                            return CallToolResult::error(vec![ContentBlock::text(
                                "Toy selection requires Forever",
                            )]);
                        }
                        let section = value["data"].take();
                        let row = section["entries"].get(id.to_string());
                        value["toy_item_id"] = serde_json::json!(id.get());
                        value["coverage"] = section["coverage"].clone();
                        value["displayed_count"] = section["displayed_count"].clone();
                        value["observed_in_view"] =
                            serde_json::json!(section.as_object().map(|_| row.is_some()));
                        value["collected_in_capture"] = row
                            .map(|entry| entry["collected"].clone())
                            .unwrap_or_default();
                        value["data"] = row.cloned().unwrap_or_default();
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
                        if let Some(target) = args.profession_target_rank {
                            value["profession_target_progress"] =
                                azeroth_operator::analysis::profession_target_progress(
                                    row,
                                    target.get(),
                                );
                            if args.profession_target_trainer_lessons {
                                let Ok(snapshot) = self.store.read(&args.snapshot_id) else {
                                    return CallToolResult::error(vec![ContentBlock::text(
                                        "Could not read the captured profession trainer context",
                                    )]);
                                };
                                let wallet = snapshot
                                    .sections
                                    .get("wallet")
                                    .and_then(|wallet| wallet["copper"].as_u64());
                                let lessons =
                                    azeroth_operator::analysis::profession_target_trainer_lessons(
                                        &section,
                                        row,
                                        target.get(),
                                        snapshot.sections.get("trainer_services"),
                                        wallet,
                                        args.profession_target_trainer_reserve_copper,
                                    );
                                value["profession_target_trainer_quote_budget"] =
                                    serde_json::json!(lessons.as_deref().map(|lessons| {
                                        azeroth_operator::analysis::profession_trainer_quote_budget(
                                            lessons,
                                            wallet,
                                            args.profession_target_trainer_reserve_copper
                                                .unwrap_or_default(),
                                        )
                                    }));
                                value["profession_target_trainer_lessons"] =
                                    serde_json::json!(lessons);
                            }
                        }
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
        positional_simulator_equipment: bool,
        clear_equipment_slots: &[u32],
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
        anyhow::ensure!(
            !positional_simulator_equipment
                || (snapshot.client.family == Family::Forever && catalogue.is_some()),
            "Positional equipment requires a Forever catalogue"
        );
        let mut review = azeroth_operator::planning::equipment_review(snapshot, selections)?;
        anyhow::ensure!(
            clear_equipment_slots.is_empty()
                || (positional_simulator_equipment && catalogue.is_some()),
            "Explicit clears require a positional Forever catalogue review"
        );
        if let Some(catalogue) = catalogue {
            let checks = catalogue
                .as_ref()
                .map_err(|_| anyhow::anyhow!("Catalogue verification failed"))
                .and_then(|catalogue| {
                    azeroth_operator::forever_catalogue::review_equipment_catalogue_with_clears(
                        catalogue,
                        snapshot,
                        selections,
                        allow_two_hand_off_hand_clear,
                        positional_simulator_equipment,
                        clear_equipment_slots,
                    )
                })
                .or_else(|error| {
                    if selections.is_empty()
                        || allow_two_hand_off_hand_clear
                        || positional_simulator_equipment
                    {
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
        if !clear_equipment_slots.is_empty() {
            let mut slots = clear_equipment_slots.to_vec();
            slots.sort_unstable();
            review["explicitly_cleared_equipment"] = serde_json::json!(slots.iter().map(|slot|
                serde_json::json!({"slot":slot,"previous_item_observed":snapshot.sections["equipment"][slot.to_string()]}))
                .collect::<Vec<_>>());
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
        recipes: Vec<azeroth_operator::forever_catalogue::RestockRecipe>,
    ) -> anyhow::Result<()> {
        use azeroth_operator::analysis::{combine_recipe_materials, plan_recipe_by_id};
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
        for entry in recipes {
            let id = entry.snapshot_id;
            let saved = self.store.read(&id)?;
            let recipe = if let Some(recipe_id) = entry.recipe_id {
                let row = saved
                    .sections
                    .get("profession_recipe_materials")
                    .and_then(|section| section["entries"].get(recipe_id.to_string()))
                    .ok_or_else(|| anyhow::anyhow!("Bulk restock recipe unavailable"))?;
                anyhow::ensure!(
                    row["status"] == "fixed_items",
                    "Expected fixed bulk restock materials"
                );
                row
            } else {
                let row = saved
                    .sections
                    .get("selected_recipe")
                    .ok_or_else(|| anyhow::anyhow!("Missing selected recipe"))?;
                anyhow::ensure!(
                    row["learned"] == true && row["requirements"] == "fixed_items",
                    "Expected a learned fixed-material recipe"
                );
                row
            };
            let item = recipe["output_item_id"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing recipe output"))?;
            let quantity = shortages
                .remove(&item)
                .and_then(|quantity| u32::try_from(quantity).ok())
                .and_then(NonZeroU32::new)
                .ok_or_else(|| anyhow::anyhow!("Expected a distinct confirmed shortage"))?;
            let review = plan_recipe_by_id(
                &saved,
                entry.recipe_id,
                None,
                Some(quantity),
                Some(inventory),
            )?;
            let mut plan = json!({
                "recipe_snapshot_id":id,"recipe_id":review["recipe"]["recipe_id"],"item_id":item,
                "target_output_quantity":quantity,"requested_crafts":review["requested_crafts"],
                "base_output_if_completed":review["base_output_if_completed"],
                "observed_cooldown":recipe["cooldown"],
                "estimated_cooldown_end_at":review["estimated_cooldown_end_at"],
                "shortfall_vs_observed_recipe_charges":review["shortfall_vs_observed_recipe_charges"],
                "unmet_crafting_conditions":review["unmet_crafting_conditions"]
            });
            if entry.recipe_id.is_some() {
                plan["recipe_observation_source"] = review["recipe_observation_source"].clone();
                plan["recipe_profession_id"] = review["recipe_profession_id"].clone();
            }
            plans.push(plan);
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
