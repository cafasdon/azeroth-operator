# Forever interaction map

Use `lookup_forever_catalogue` with `section: "items"` and `set_name`
to find pieces by a case-insensitive set-name substring. Combine it with
`set_id` or other item filters to narrow the same catalogue records.
The same `set_name` filter works on `review_forever_owned_gear` before
paging. Without an explicit `set_id`, it reports how many catalogue set
IDs match and lists up to 50 of them with their catalogue names. A unique match supplies the same
whole-capture set observation as an explicit ID. An ambiguous match
leaves the set observation absent.
Each name match also shows the number of distinct pieces observed anywhere
in the saved inventory and the number observed equipped. Catalogue totals
include all pieces sharing the set ID. Unobserved counts measure the gap
against that local catalogue, including pieces that may be in unseen storage. The equipped
count is null when equipment was unavailable. Item filters and paging do
not change these counts.

In `review_forever_owned_gear`, a `set_id` review reports distinct
catalogue-matched set pieces separately for equipment, bags, character
bank and account bank. Duplicate copies within one storage count once.
The counts use the whole saved capture before other filters or pagination.
Unavailable storage is null and observed storage with no matches is zero.
The set review lists `observed_stored_not_equipped_item_ids` when equipment
was captured. It also lists `unobserved_catalogue_item_ids` for catalogue
pieces absent from the capture. Unobserved pieces may be in unavailable storage.
Set `include_unobserved_set_sources: true` with an explicit set ID or unique
set name to see local catalogue sources and saved acquisition observations
for up to 50 unobserved pieces in item-ID order.
When that capture includes a merchant window, the acquisition review can
quote one unit per unobserved piece against those recorded offers and
compare supported copper totals with the saved wallet. A missing merchant
window or an empty piece list gives a null quote.

With the local catalogue, `review_equipment_selection` reports
`checks.catalogue_projected_set_changes` for sets whose distinct equipped
item IDs would change under the draft. Counts of unmatched equipment slots
expose gaps in the projection. Native set bonuses are unverified.
The offline slot ranker retains this evidence as `projected_set_changes`
in resolved gear plans, per-encounter results and weighted DPS rankings.
Saved-window rankings also report it under `saved_loadout` when the
retained plan includes the projection.
Request preparation checks retained projections against a fresh offline
review of the same saved snapshot and draft.

Use `list_snapshots.gossip_quest` with a `quest_id` and `list` of
`available` or `active`. Optional `is_complete`, `repeatable`,
`is_trivial` and `is_ignored` flags must match the same recorded row.
Missing flags do not match either true or false. Results describe saved
lists and do not establish current acceptance or turn-in eligibility.
When the same quest ID also appears in the saved quest log, matches show
`quest_log_readiness`. Use it as an optional filter independently of the
panel's `is_complete` flag. An unknown readiness requires a recorded log
row. A missing row does not match any readiness predicate.
On `inspect_snapshot`, set `section: "quests"`, a `quest_id` and
`quest_gossip_observation: true` to see that quest in the same capture's
dialogue lists. An observed list can show that the ID was absent. A missing
list stays unknown, and the dialogue observation can exist without a quest
log row.
In `character_summary`, observed dialogue lists also include
`quest_log_matches` when the quest log was captured. It reports the full
match count and the first 50 matching IDs in numeric order with their
separate panel completion and log readiness observations.

Set `gossip_quest_changes` on `compare_snapshots` to compare the saved
available and active quest lists separately by quest ID. Each list shows
IDs seen on only one side and changes to flags explicitly observed on both
sides. A missing list leaves that comparison unknown. The comparison does
not establish NPC identity, quest acceptance or turn-in eligibility.
When both quest logs are available, each compared list includes
`quest_log_readiness_changes` for IDs seen in that list and the log on both
captures. These transitions are separate from the panel's reported flags.

Saved dialogue search accepts `gossip_option.minimum_reported_order_index`
and `maximum_reported_order_index` as inclusive signed bounds. They match
the reported index on the same option row as other predicates, while
missing indices stay unknown. Label matching ignores surrounding whitespace
and preserves the original recorded label.

The saved recipe view and its skill-up candidate summary can include
`rank_observation` for the same open profession. The addon retains readable
rank and cap values when they agree across the capture. Older captures
omit the observation. Update strict readers before rolling out the addon.
An optional `skill_modifier` records the separate signed value when it
is readable and stable alongside the base pair. Rank and cap calculations
continue to use the base values.
Use `profession_recipe.minimum_skill_modifier` and
`maximum_skill_modifier` on `list_snapshots` for inclusive signed bounds.
The filter requires a recorded modifier from that recipe view. Missing
modifiers are excluded, including when the bound is zero.
Use `list_snapshots.profession_recipe.maximum_rank_points_to_cap` for an
inclusive distance from the same recipe view's recorded cap. Results show
the recorded pair and the remaining points. Recipe search shows the pair
when the view and saved material schematics have the same profession ID.
With `profession_recipe_changes` on `compare_snapshots`, rank, cap and
remaining points are compared only when both views report a rank pair for
the same profession. The differences describe saved observations.
When both views also report `skill_modifier`, the comparison shows its
signed change separately from the base rank and cap.
[Forever profession rank bar](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRankBar.lua)

Use `order: "reported_skill_ups_per_reagent_desc"` with a
`reagent_item_id` on `search_forever_recipes`. It orders the saved
skill-up count divided by the recipe's required units of that reagent.
Unknown ratios sort last, with ties ordered by recipe ID. The ratio uses
reported values from the same profession and does not predict skill gains.

Use `maximum_skill_ups_reported` on a saved recipe view filter or
`list_snapshots.profession_recipe` for an inclusive upper bound. Combine
it with the minimum for an exact count. Saved character discovery can
also match `relative_difficulty`. Missing values cannot satisfy those
predicates, and reported counts do not guarantee skill gains.

On `list_snapshots`, combine `equipment_durability_slot` (1–19) with
`equipment_durability_max_percent` to find saved captures where that
specific slot meets the threshold. Missing readings do not match.
Owned gear reviews include `observed_equipped_durability` when a readable
condition was recorded for that equipped slot in the same capture. The
reading describes the saved equipped location.
`find_snapshot_item` shows the same field on equipped matches and works
without a catalogue. Each selected snapshot uses its own slot readings.
Both tools also show `observed_weapon_hand_enchants` on equipped main-hand,
off-hand and ranged rows when that hand has a saved observation. An observed
empty list is retained. These lists describe the captured hand and do not
verify enchant identity on an item outside that location or current duration.
Equipment selection reviews show saved durability and enchant observations
for the equipped source and occupied target slots when available. Both
describe the capture before the proposed change.
When revalidating a draft with `compare_source_locations_snapshot_id`,
`captured_source_slot_conditions` reports the original and newer readings
for an equipped source slot. A reading is attached only when the slot was
occupied in that capture. The pair describes the slot, even if the item ID
changed.
Equipped rows in `same_base_item_candidates` also carry any observed
durability and weapon-hand enchant data from the newer capture. These
readings describe each candidate location.

Set `maximum_xp_to_next_level` on `list_snapshots` to match saved Forever
characters within an inclusive XP budget. Matching rows report their saved
XP remaining. Combine it with `xp_gain_enabled_only` or `minimum_rested_xp`.
Capped or unreadable progress does not match.
Set `ready_quest_xp_covers_next_level: true` to find captures with explicitly
enabled XP gain and enough known XP from all quests marked ready in that
capture. Matches show up to 20 quest IDs from the reward-based plan.
This uses the saved quest log independently of any other quest filters and
does not establish turn-in eligibility or affordability.
Add `ready_quest_xp_affordable_only: true` to require known saved costs
and rewards for the selected shortlist and enough recorded starting copper.
Matching plans include the minimum starting copper and the first 20 quest
IDs in wallet order. This checks the selected plan rather than searching
alternate quest combinations.
With affordability filtering enabled, set `ready_quest_xp_wallet_reserve_copper`
to keep that copper untouched throughout the selected turn-ins. Matching
plans report the reserve separately from the minimum starting copper for
the quest costs.

Saved pet searches also support `active_pet.happiness` for the exact
recorded value and `active_pet.loyalty` for the exact case-insensitive
localized label, ignoring surrounding whitespace. Missing observations do not match, and matching results
retain the recorded values.

Use `active_pet.food_type_name` on `list_snapshots` to match an exact
case-insensitive localized food category from a saved pet capture. The
result retains the recorded category spelling. Missing or empty diet
observations cannot match. This does not establish acceptance of an individual item.
In pet-care comparisons, `food_type_changes` lists the exact localized
labels observed on only one side. Order and duplicate labels are ignored.
Both captures must contain a diet list. A missing list leaves the comparison
unknown, and the captures can describe different pets or client locales.

Find saved pet-food restock candidates with `list_snapshots.pet_food`.
Set `minimum_quantity: 0` and an inclusive `maximum_quantity` to look
for low carried stock. Use `item_id` to count one accepted food item.
Captures without a readable carried-food observation do not match.

On `list_snapshots`, find saved travel options by exact, case-insensitive
`hearthstone_bind_name` and inclusive `hearthstone_max_cooldown_seconds`.
Use zero for a cooldown that was ready at capture. Missing observations
cannot match the corresponding filter. Both filters can be combined with
captured player-map filters. This does not project current item usability.
Use `hearthstone_projected_ready_by` with an explicit RFC 3339 timestamp
to match capture time plus recorded cooldown at or before that deadline.
Matching rows include `hearthstone_projected_ready_at`. Zero uses the capture
time itself. This historical projection does not establish current readiness.
In `compare_snapshots`, the two `travel_context_change` observations also
report their own projected timestamps. An unavailable cooldown or timestamp
appears as null.

Saved instance inspection includes `encounter_total` and
`reported_encounter_progress` when both cache values are readable and
consistent. Older captures and rows with missing or restricted values omit
the pair. The reported progress value does not establish which encounters
were defeated or whether loot is currently available.
Update strict snapshot readers before installing an addon that emits the
pair. Native Forever readback remains pending.

With `saved_instance_changes`, compare both reported values across uniquely
matched public instance descriptions when both captures contain the pair.
Missing progress remains unknown. The result describes differences between
saved rows and cannot establish that they belong to the same lockout.

On `list_snapshots`, use `saved_instance.minimum_reported_encounter_progress`
or `maximum_reported_encounter_progress` to find captures by the recorded
value. Bounds are inclusive and apply to the same row as other instance
filters. Rows with missing progress cannot satisfy either bound.
Add `encounter_total` to match the exact reported total on the same row.

Saved instance rows can include `reported_extension_disabled`. This is the
cached per-row flag the Forever UI uses when the client supports the extension
feature. Filter on it with `list_snapshots.saved_instance` or compare it with
`saved_instance_changes` when both matching rows report it. The flag does
not establish current control visibility, eligibility or an extension outcome.
Update strict snapshot readers before installing an addon that emits the flag.
[Forever Raid Info UI](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_RaidFrame/Mainline/RaidFrame.lua#L226)

Set `instance_lock_warning_reference_at` with
`instance_lock_encounter_review` to check whether the saved warning's
projected expiry falls at or before an explicit timestamp. The projection
uses the saved timer and capture time. Current warning state remains unverified.
On `list_snapshots`, use `instance_lock_warning.projected_expiry_from`
and/or `projected_expiry_by` to match an inclusive window. Matching
captures report the projected expiry and can also be filtered by encounter
count or name.

Use `instance_lock_warning.maximum_completed_encounters` or
`maximum_remaining_encounters` on `list_snapshots` to find saved
warnings within an inclusive encounter-count range. Set the minimum and
maximum to the same value for an exact count.

Use `achievement_filter.maximum_unfinished_criteria` when inspecting
`tracked_achievements` to find achievements within a limit on explicitly
unfinished captured criteria. A missing achievement section stays unknown. Combine it
with `criterion_completed: false` to require at least one unfinished
criterion, or with `completed: false` to restrict achievement status.
On `list_snapshots`, use `tracked_achievement.maximum_unfinished_criteria`
with an exact tracked achievement ID or name to find matching saved
characters. The result includes the captured unfinished count.
With `achievement_status_changes`, saved comparisons also list achievements
whose comparable criteria have increased or decreased remaining quantities.
The comparison requires the same account scope, a unique criterion ID on
both sides and the same known requirement. Different criteria can put
one achievement in both lists.
The same comparison groups newly completed and no-longer-completed
criterion IDs by achievement. The recorded completion flag is evaluated
separately from numeric progress. Criterion requirements must agree,
including when both observations omit a requirement.

Use `mail_attachment_bag_space_message_slot` when inspecting saved mail
to estimate space for known item attachments in one loaded message.
You can also provide `mail_attachment_bag_space_item_ids` to limit the
estimate to selected items. Unloaded slots return unknown. The message
slot belongs to that capture and does not identify a live message.

Use `mail_attachment_headroom_currency_ids` when inspecting a saved
`mail_attachments` section to compare loaded pending quantities with
the same capture's currency caps. Missing attachment or cap observations
stay unknown. The comparison is hypothetical and does not establish
whether mailbox retrieval applies the recorded caps.
Use `mail_attachment_headroom_message_slot` to check a single loaded
message, with optional currency IDs to narrow the result. Unknown slots
are counted for that message and unloaded message slots return unknown.
For saved inbox discovery, set
`mail_attachment.maximum_quantity_above_recorded_headroom` with
`kind: "currency"` on `list_snapshots`. Use zero to require the known
loaded quantity to fit. Missing cap evidence cannot satisfy this filter.
Set `mail_attachment.require_recorded_bag_space: true` with
`kind: "item"` to require a known fit in the same capture's recorded
general bag slots without stack merges. The estimate covers the selected
item's known loaded quantities. Retrieval eligibility remains unverified.
Set `mail_attachment.require_fully_readable_capture: true` for either
attachment kind to require zero unloaded messages and zero unknown
attachment slots in the saved inbox. It combines with cap or bag-space
filters and applies to the recorded capture.

Set `quest_list_ready_waypoint_groups` when inspecting the saved quest
list to group quests marked ready by their next waypoint map. Ready
quests without a recorded map are listed separately. Use
`quest_on_captured_map` to narrow the list to the saved player map.
The next waypoint is a map hint and does not verify the turn-in location.
Combine it with `quest_list_wallet_projection` for saved coin costs,
rewards and wallet projections per map and for the unmapped group.
Each group starts from the same saved wallet balance.
Combine the waypoint groups with `quest_list_timed_checklist` to see
the earliest saved timers within each ready group, including unmapped
quests. Each group shows at most 20 timers and counts ready quests
without a recorded timer.

Use `quest_readiness_changes` on `compare_snapshots` to see which shared
quests switched between explicitly not ready and ready, or switched from
either state to failed. Changes involving unknown observations remain in
the detailed list. These are saved status changes.

With `quest_objective_changes`, saved quest comparisons list quest IDs
whose remaining objective counts increased or decreased. Counts require
the same objective index, kind and recorded requirement in both captures.
A quest can appear in both lists if different objectives moved in
opposite directions.

The saved quest objective checklist includes
`by_unfinished_objective_count`, ordered by the number of explicitly
unfinished objectives and then by numeric quest ID. It follows the
quest filters. It counts objective rows, even when their remaining
quantities are unknown, and does not estimate completion time.

Set `flight_route_search.excluded_landing_node_ids` to avoid up to 32
distinct connections in a saved route. The planner and its fallback
evidence exclude those landing nodes. The start and destination cannot
be excluded. A preview may still show an excluded node as a flyover.
Use `flight_route_search.excluded_flyover_node_ids` to exclude up to
32 distinct interior preview nodes. A nonempty list requires a saved
preview for every leg, including fallback routes. The leg's origin and
landing are outside this filter.

Use `flight_route_search.maximum_capture_skew_seconds` to require the
earliest and latest of the selected saved flight panels to fall within
an inclusive time span. This includes panels that the chosen route does
not use. The result reports both capture times. The limit checks how far
apart the captures were taken and does not verify current routes or fares.

Set `material_filter.insufficient_only` on `search_forever_recipes` to
find fixed-item recipes below the requested craft count in the selected
observed storage. Each matching row lists the required, observed and
missing quantities by reagent ID. Unavailable recipe or inventory evidence
does not satisfy the filter. A gap in observed storage can understate the
items you have elsewhere.
Add `material_filter.sole_shortfall_item_id` with `insufficient_only: true`
to find recipes where that exact item is the only observed reagent gap
for the requested batch. The count of insufficient recipes still includes
all observed gaps before applying the sole-item filter.
Set `material_filter.maximum_sole_shortfall_quantity` with that item ID
to keep only gaps at or below a planned pickup quantity. Each recipe is
evaluated independently against the saved inventory.
Use `order: "observed_sole_shortfall_asc"` with the sole-item filter to
show the smallest pickups first, with ties ordered by numeric recipe ID.

With `profession_target_trainer_lessons` on a saved profession inspection,
set `profession_target_trainer_reserve_copper` to check each projected
lesson's saved quote against the saved wallet while preserving a reserve.
Each quote is evaluated separately. Missing prices or wallet observations
leave the combined shortfall unknown.
The accompanying `profession_target_trainer_quote_budget` totals the
projected rows and applies the reserve once. A known subtotal can show
a budget shortfall even if other quotes are missing. Affordability refers
to saved prices and does not establish training eligibility.
The budget also shows the largest count of individually quoted rows that
fits after the reserve, taking the cheapest prices first and breaking ties
by saved slot number. Missing prices are excluded from this shortlist.
`rank_order_quote_budget` checks the quoted rows in ascending required
rank and groups lessons at the same rank. It stops at the first group
with an unknown quote or a known shortfall. The projection requires a
saved wallet that covers the reserve and does not establish a training path.

Use `profession_target_max_remaining` on `list_snapshots` with a
`profession_skill_line` and `profession_target_rank` to find saved
professions within an inclusive rank-point budget. Add
`profession_target_within_cap_only` to require a known recorded cap
at or above the target. These filters use saved ranks and caps.

Use `action_bar_spell_resources` on `list_snapshots` with a positive
`spell_id` and a `resources` predicate to find saved captures with a
matching shortcut. The exact reported spell ID must appear in both
sections and all predicates must match one spellbook row. The result
shows action and spellbook slots. It leaves cast eligibility unverified.

Use `action_bar_spell_resource_filter` with saved action-bar inspection
to find retained spell shortcuts matching recorded charge, cast-count or
cooldown predicates. All predicates must match one spellbook row with the
exact reported action spell ID. Missing values cannot satisfy a predicate.
The result reports matching slots and leaves cast eligibility unverified.
Matching slots include their captured hotkeys where available. An
observed button with no keys has an empty list. Missing observations
remain unknown.
For each matched spell ID, the result also lists exact spellbook rows
that failed a predicate or had insufficient readings. A matching row
does not override other recorded observations for the same spell.

Set `group_member_target` with `inspect_snapshot` on `group_context`
to compare the saved home and instance counts separately. It reports
shortfall and excess against the target. Missing counts stay unknown.
The result uses raw captured counts and leaves roster and current group
membership unverified.
The same selector on `compare_snapshots` shows the count and target
projection on each side and flags crossings separately for home and
instance. A crossing requires both counts and does not establish a
membership event or group continuity.
Set `instance_group_size_reference` on saved `instance_context` inspection
to compare the instance-group count with a saved lockout row's reported
size. The reference requires exactly one cache row with matching name,
difficulty and party or raid kind, and a positive size. It does not
establish instance identity, current capacity or roster completeness.
Set `instance_group_size_max_cache_age_seconds` to require an inclusive
cache age limit at capture time. An older or unavailable cache leaves the
size and shortfall unknown. The report still shows any matching row count.
Use these same options with `compare_snapshots` to see both references.
Shortfall and excess deltas require the same recorded name, kind,
difficulty, map ID and positive size reference. This still leaves
instance and group identity unverified.

Set `pvp_target_rank` on `compare_snapshots` for the same rank target
in both saved PvP captures. Each side retains its own threshold and cap
observations. Crossings use captured ranks and require matching reported
season IDs. Season continuity and eligibility remain unverified.
Points-remaining deltas also require matching reported season IDs.
The total-points delta requires the same target threshold and current-rank
thresholds in both captures. The next-rank delta requires the same
reported next-rank threshold and a usable estimate on each side.
Use `pvp_target_distance` on `list_snapshots` with `target_rank`
and `maximum_remaining_points` to find saved captures near a rank.
The default `total_threshold` basis uses recorded total thresholds.
Select `next_rank_threshold` to use the recorded next-rank estimate
only. Missing estimates cannot satisfy the inclusive bound.

Set `player_trade_offer_item_id` on `compare_snapshots` to limit saved
outgoing trade comparisons to one base item, even if the general trade
comparison flag is also set. Missing or incomplete offer observations
stay unknown where appropriate. Deltas require fully readable offers
in both captures and do not establish trade continuity or completion.
An unfiltered `player_trade_offer_changes` comparison also lists
`readable_slot_changes` when a slot reports an item in both captures and
its item ID or quantity changes. Unreadable slots cannot establish a
change. A missing offer makes the slot comparison null. Exact-item
comparisons omit this list to keep other item IDs out of the response.
With the exact item selector, set `player_trade_offer_compare_carried`
to view each capture's bag quantity and its change independently of the
offer. Offer quantities stay separate from carried stock.

Set `mail_attachment_item_id` on `compare_snapshots` to compare one
item's saved loaded inbox quantities and attachment counts. A missing
inbox is unknown, and an item absent from the loaded rows has no known
quantity. The delta requires known quantities in both captures and
does not establish delivery or retrieval.
Use `mail_attachment_currency_id` for the corresponding currency
comparison. It can be combined with the item selector, and the two
identifier namespaces remain separate.
With the currency selector, set `mail_attachment_compare_currency_balance`
to view each capture's balance separately. A balance delta requires both
balances to have the same known account-wide flag. Pending attachments
stay separate, and balance changes do not prove a transfer.
The same option shows a hypothetical comparison of each loaded attachment
total with the recorded cap headroom from that capture. It requires a
known attachment quantity and recorded cap. Whether mailbox retrieval
applies those caps remains unverified.
With the item selector, set `mail_attachment_compare_carried_item` to
compare the same captures' bag quantities separately. Pending mail
does not count toward carried stock and changes do not prove transfers.
The same option estimates bag space for the known loaded attachment
quantity on each side, using that capture's reported stacks and free
slots. It leaves retrieval and placement unverified.

Set `quest_list_objective_checklist` in saved quest-list inspection to
see explicitly unfinished objectives and remaining counts for matching
quests. It skips failed quests, preserves the captured readiness flag,
and lists quests whose objective section was absent separately. Unknown
remaining counts stay unknown.
The checklist also lists quests with a nonempty set of objectives all
marked finished and captured readiness of `not_ready` or `unknown`.
Completing the observed objectives alone does not establish turn-in readiness.
Comparing an exact saved quest shows changed objective indices when both
captures contain objective observations. Fulfilled-count deltas require
the same captured kind and required count on both sides. The remaining-count
delta uses the same checks and treats overfulfilled objectives as zero
remaining. Index matching
does not establish objective identity.
Set `quest_objective_changes` on `compare_snapshots` to list changed
quest IDs across both logs. It compares only quests present in both logs
and reports shared quests with unavailable objectives separately.
It separately lists quests with newly finished or no-longer-finished
objectives when the saved index, kind and required count agree. A quest
can appear in both lists when different objectives changed.

In `inspect_snapshot`, select a Forever `faction_id` in the reputation
section and pass `reputation_points_per_turn_in` to estimate repetitions
to its observed next standard rank. The estimate uses a constant
caller-supplied reward and rounds up. It leaves capped, unobserved,
inconsistent and special reputation counts unknown.
With that estimate, `reputation_turn_in_item_cost` accepts an exact item ID
and assumed quantity per repetition. It shows the total needed and
shortfall against the same capture's bags. Missing bags or an unknown
total leave the shortfall unknown. An overflowing total is flagged.
For a turn-in requiring several items, use `reputation_turn_in_item_costs`
with one to twenty distinct item IDs and quantities. The response reports
each shortfall separately in request order. Use one cost form per request.
The plural plan also reports complete turn-ins supported by the saved
bags, all limiting item IDs and the remaining turn-ins needed for the
next rank. Missing bag observations leave capacity unknown.
With either item cost form, set `reputation_turn_in_bank_withdrawal_offset`
to zero for the first page of saved withdrawal locations. The plan uses
the same capture's bags and banks and leaves withdrawal eligibility
unverified. Missing bags or an unknown or overflowing required total
leave the plan unknown.
Set `reputation_turn_in_pending_mail` with the bank withdrawal plan to
check its remaining shortfalls against loaded attachments in the same
capture. Only item quantities and coverage are reported. Pending mail
does not count as owned stock, and retrieval eligibility stays unverified.
With the same points estimate, `reputation_turn_in_copper_cost` accepts
`copper_per_turn_in` and an optional `wallet_reserve_copper`. It reports
the total cost, wallet shortfall and wallet-supported repetitions from
the same capture. When both costs are provided, the combined capacity
uses the lower limit, lists both resources if tied, and stays unknown
unless both observations are available. Saved bank stock remains separate.

In a saved flight map, `flight_wallet_projection` shows the wallet balance
after each recorded fare and the spendable balance after also setting aside
`flight_wallet_reserve_copper`. Each calculation is independent. Missing
values or an insufficient balance leave the corresponding remainder unknown.
Comparing an exact flight destination reports each captured reachable fare's
wallet shortfall and remaining balance. Deltas require known values on
both sides and the same map and departure node.
With an exact destination, set `flight_wallet_reserve_copper` to also
check affordability, shortfall and spendable copper after the reserve in
each capture. The existing fare-only comparison remains available.
Reserve-adjusted changes require known values on both sides and the same
map and departure node, just like the fare-only comparison.

In `inspect_snapshot`, select a saved profession with `section: "professions"`
and `profession_skill_line`, then set `profession_target_rank` to check points
remaining and whether the target exceeds the captured cap. A zero or
inconsistent cap leaves cap comparisons unknown. Ambiguous or unavailable
observations leave progress unknown. This does not verify training eligibility.
Set `profession_target_trainer_lessons` to list lessons in the same saved
non-pet trainer panel whose exact skill requirement name matches the
unique profession name and whose rank threshold falls above the captured
rank and at or below the target. The list reflects observed rows and
recorded status and price. Each lesson shows points to its required rank
and whether that rank exceeds the observed profession cap. The cap check
is unknown for a zero or inconsistent cap. Missing or ambiguous context
returns null.
Each lesson's quoted price is compared separately with the wallet in that
capture. Missing quotes or wallet data leave affordability and shortfall
unknown. The comparison does not establish eligibility or combined cost.
For cross-character discovery, use `list_snapshots` with the same skill
line and optional target. Inclusive `profession_min_rank` and
`profession_max_rank` filter the observed rank. Results require a unique
skill-line observation in each matching Forever capture.
Use the skill line and target with `compare_snapshots` to compare progress
in both captures. As with other comparisons, the before capture cannot
be newer than the after capture. Crossing flags stay unknown unless both
captures report a unique rank.

Use `list_snapshots.pet_action_command` with an exact `command_token` such as
`PET_ACTION_ATTACK` to locate saved command slots. Optional `is_active` matches
only explicit captured flags. Results are historical readings in slot order.
They carry no current control or cast eligibility guarantee.

Use `list_snapshots.pet_action_spell` with an exact `spell_id` to find
saved pet bar slots. Optional `is_active`, `autocast_allowed`,
`autocast_enabled` and `in_range` flags must match on the same row.
A missing flag cannot satisfy a predicate, including false. Results
describe the saved action bar and leave current cast eligibility unverified.
Use `compare_snapshots.pet_action_bar_changes` to compare saved pet and
bar context and changed numeric slots. Slot comparison requires a bar
reported in both captures. One-sided rows are unobserved on the other
side. Pet identity and casting eligibility remain unverified.

Saved carried bag space comparisons list changed bag slots in numeric
order, with before and after readings and signed capacity changes.
One-sided slots retain their known reading and leave deltas unknown.
A missing bag-capacity section leaves the slot comparison unknown.
The comparison does not establish bag item continuity.

Use `compare_snapshots.repair_quote_changes` to compare saved repair-all
quotes and affordability. The report shows each capture's repair status
and signed changes in quote, wallet shortfall and copper remaining after
the quote. Each change requires two known values. A remaining balance
requires an affordable quote in that capture. Repair events stay unverified.
Use `repair_wallet_reserve_copper` with `inspect_snapshot` on `repair_quote`
to check the quote and reserve against the same saved wallet. The report
shows affordability, the combined shortfall and any copper left after
both. A missing quote leaves the combined shortfall unknown.
Set `repair_include_equipment_condition: true` to list damaged equipped
slots and missing durability points observed in the same capture. The
condition report remains available if the quote is missing. It covers
readable equipped slots and leaves the quote's item scope and per-slot
prices unverified.
Damaged rows include the item ID from the same captured equipment slot
when available. The report also lists observed equipped slots without a
durability reading. These slots may contain items without durability.

Use `list_snapshots.merchant_buyback` with an exact `item_id` to find
saved buyback entries. Optional `minimum_quantity` and
`maximum_price_copper` are inclusive and apply to the same entry.
The price is for the whole entry. A missing price cannot meet a ceiling.
Results leave ownership and current buyback eligibility unverified.
Set `minimum_remaining_wallet_copper` to require that an individual quote
fits the wallet captured in the same save and leaves that much copper.
Zero still requires a known wallet and price. Entries are checked separately.
In `inspect_snapshot`, use `buyback_slots` and `buyback_estimate_bag_space`
to check the carried space for the whole selected entries alongside their
saved quotes. Quantities for the same item share stack headroom. Missing
entries leave whole-selection fit unknown, while a missing price leaves
the space estimate independent of affordability.
Use `compare_snapshots.merchant_buyback_changes` to compare saved lists
by item ID, quantity and whole-entry price, ignoring slot order and
retaining duplicate entries. Missing lists remain unknown and observed
empty lists stay empty. Differences leave transactions unverified.

Use `list_snapshots.required_sections` to find captures containing all
named sections together. Observed empty sections count as present.
Missing sections, including those with a recorded unavailable reason,
do not match. With `latest_per_character`, only the newest eligible
capture is checked for section presence. Presence leaves completeness
within each section unverified.
Use `any_missing_sections` to find captures missing at least one named
section. Each matching section is marked `reported_unavailable` or
`not_recorded`. The list omits reason text. Use snapshot inspection to
read a recorded reason for a specific section.
When a section cannot be compared, `compare_snapshots` reports its
capture status on each side as `observed`, `reported_unavailable`
or `not_recorded`. The comparison omits reason text.
Use `inspect_snapshot.coverage_sections` with distinct recognised names
to check those statuses in one saved capture. This view returns snapshot
metadata and the requested statuses. It omits section payloads and
reason text and cannot be combined with a section data request.
Set `list_snapshots.coverage_summary_sections` to count those statuses
across all matching saves. The summary applies the same filters and
newest-per-character selection as the list, then counts before paging.
It omits section contents and unavailability reasons.
Set `list_snapshots.exact_client` to restrict the search to an exact
family, variant, build and interface. The family must match the list
request. Newest-per-character selection and coverage totals use only
captures within that client scope.
The Rust MCP `capabilities.snapshot_section_schema` lists recognised
section names and the names whose captured payloads require Forever.
These are reader rules. Capture availability still depends on the saved
observation and native verification remains separate.
For an explicitly named character, `resolve_character.required_sections`
selects the newest capture containing all requested sections within any
client or time bounds. An older matching capture can qualify. This is
historical selection and does not verify the character currently in play.

Use `list_snapshots.weapon_enchant` to search a saved `main_hand`,
`off_hand` or `ranged` enchant list. Set an optional exact `enchant_id`,
`minimum_charges` or `maximum_remaining_milliseconds`. All predicates
must match the same row that reported an enchant. Missing readings cannot
satisfy a bound, and zero remains a known value. Results give the numeric
row indices. Timers describe the capture.
Use `enchant_type` for `permanent`, `temporary` or `imbue`, and
`maximum_charges` to find known low-charge readings. Combine it with
`minimum_charges` for an inclusive range on one captured row.
Use `compare_snapshots.weapon_enchant_changes` to compare saved readings
per hand. The comparison ignores row order and preserves duplicate
readings. Missing hand observations give null, while observed empty
lists remain empty. Differences leave enchant application, expiration
and weapon continuity unverified.

Use `list_snapshots.player_aura` with a `group` of `helpful` or
`harmful` and an exact `spell_id`. Matches count observed rows for that
ID in the selected group, including duplicates. An absent match leaves
the aura's presence outside the saved observation unknown.
Optional `minimum_applications` and `maximum_applications` bounds are
inclusive and must match the same captured row as the spell ID. A missing
count cannot satisfy either bound. A recorded zero remains searchable.
Use `compare_snapshots.player_aura_changes` to compare saved rows by
group and exact spell ID. Each observed group reports up to 50 changed
IDs in numeric order and counts all changed IDs. Application lists
retain duplicates and unknown counts and ignore row ordering. A group
missing from either capture has no comparison. The result describes
saved observations and leaves aura application or removal events unverified.
Set `player_aura_spell_id` with the comparison to restrict both groups
to an exact spell ID before the display limit is applied.
The `selected_aura` report lists that ID's readings independently on
each side, including unchanged readings. A missing group gives null and
an observed group with no matching rows gives an empty list.

Use `list_snapshots.player_resource` with `resource` set to `health` or
`primary_power` and at least one predicate. Whole `minimum_percent` and
`maximum_percent` bounds from 0 to 100 are inclusive and use the exact
captured ratio. Missing readings and zero maximums cannot match percentage
bounds. For primary power, `power_type` from 0 to 63 matches the exact
captured type, with or without percentage bounds. Each match includes the
saved counter and leaves current readiness unverified.
Saved `player_resource_changes` also reports changes in missing units
and percentage points alongside the current and maximum counter changes.
Percentage-point differences require two positive maximums. Primary
power comparisons still require matching type and units.

Use `list_snapshots.maximum_pvp_points_to_next_threshold` to find saved
Forever captures within an inclusive point limit of the reported next PvP
rank threshold. Zero matches a reached positive threshold. Missing, zero
or exceeded thresholds are excluded, and rank and season filters can be
combined with the point limit. Matching rows show the saved distance.
Use `list_snapshots.pvp_cap_distance` with `cap` set to `max_rank`,
`current_week_max_rank` or `previous_week_max_rank` and an inclusive
`maximum_remaining_points`. The filter uses the same captured threshold
arithmetic as the progression summary. Unknown distances are excluded.
Zero includes saved totals at or above the selected positive cap threshold.
Saved comparisons with matching reported season IDs show signed changes to
each reported cap rank and point threshold. They compare remaining points
only when the cap rank and threshold agree and both distances are known.
For matching rank and reported season IDs, saved comparisons also show
the signed change to the reported next-rank threshold. The remaining-point
change requires the same positive threshold and known distances on both sides.

Use `maximum_reported_member_count` in the `list_snapshots.group_context`
filter to cap the captured member count. Combine it with
`minimum_reported_member_count` for an inclusive range in the selected
home or instance category. Zero matches an observed count of zero.

The saved PvP progression comparison reports a signed rank difference when
both captures report the same season ID. It also subtracts the reported
rank-point readings when the rank is unchanged. Different or missing season
IDs leave this calculation unknown. The calculation leaves verified season
continuity and points earned unconfirmed.

The saved action-bar spellbook review lists observed spell slots whose
action ID matched neither a captured base nor reported spell ID. Missing
spellbook or action observations leave the list unknown. An unmatched slot
can reflect incomplete capture and does not establish castability.
The review also flags action IDs that match only a captured base spell ID
with multiple distinct reported IDs. It lists those IDs without selecting
an override or borrowing its resource readings.
When resource readings are present, `spellbook_resource_conflicts_by_action_id`
lists fields with different known values across rows for the same exact action
spell ID. Missing readings do not count as disagreements. The review retains
each original reading and leaves current cast eligibility unknown.
`matched_spellbook_usability_by_action_id` lists saved usability and
insufficient-power flags by spellbook slot for exact reported IDs in the
reviewed action slots. An absent ID has no retained exact reading. The map
is null when either observation is missing. These flags describe the
capture and leave current cast eligibility unverified.
`spellbook_usability_conflicts_by_action_id` lists only flags that
disagree across retained readings for the same exact ID. Missing
readings do not count as disagreements.

The timed quest checklist counts quests with recorded timers that were ready
to turn in at capture and highlights the one with the shortest remaining
timer, breaking ties by quest ID. It checks all retained timed quests,
including those beyond the 20-entry display limit. Failed quests do not count.
Inspecting a saved Forever quest by ID also reports captured fulfilled,
required and remaining quantities by objective type, with separate counts
for objectives whose quantities were unavailable.
For a saved Forever quest ID, `quest_inventory_requirements` accepts one
explicit storage and 1–64 distinct user-supplied item IDs with quantities.
It checks observed stock and reports shortfalls against those quantities.
Missing storage gives unknown quantities. The check leaves objective item
binding and quest readiness unverified.
The review also counts how many item IDs are short and reports whether
the selected storage covers the entire supplied checklist. Both values
remain unknown when that storage was unavailable.
With bags selected, enable `quest_inventory_bank_withdrawals` to see which
shortfalls could be covered by saved character-bank stock first and
account-bank stock second. The guide shows remaining shortages and missing
bank observations. Original source lookups and the original merchant quote still use
bag shortfalls. When both withdrawals and merchant quotes are requested, the
quote includes an `after_observed_bank_withdrawals` estimate for the remaining
shortages. This estimate assumes the suggested withdrawals can be made and
does not count unobserved bank stock.
Catalogue and saved provider source reviews also include an
`after_observed_bank_withdrawals` report when the bag withdrawal plan is
available. It covers only remaining shortages, using the same source
capture or catalogue as the original lookup.
The withdrawal guide shows saved bank slots and suggested quantities in
`suggested_locations`. It uses larger stacks first within each bank.
Set `quest_inventory_bank_withdrawal_offset` to page through the locations
in groups of 50. Per-item withdrawal quantities remain available on every page.
Add `quest_inventory_merchant_snapshot_id` to price the shortfalls from a
saved merchant visit for the same character and client. Quotes use the
inventory capture's wallet and preserve unknown stock or incomplete prices.
Set `quest_inventory_snapshot_id` to check a same-time or newer capture of
the same character and exact client. The quest stays anchored to the
original capture. Inventory quantities and merchant affordability use the
selected stock capture.
Enable `quest_inventory_catalogue_sources` to look up known shortfalls in
the verified local catalogue. The result lists public acquisition references
with their catalogue provenance. Missing stock leaves the source review
unknown, and recorded sources leave current availability unverified.
Set `quest_inventory_source_snapshot_id` to look up the same shortfalls
in a compatible saved provider capture. Source references and coordinates
come from that capture, independently of inventory. The lookup preserves
source provenance and leaves native spawn and routing unverified.

Saved equipment comparisons report current and maximum durability changes
for slots observed in both captures, and list slots seen on only one side.
A missing durability section leaves the comparison unknown. These readings
are by slot and do not establish item continuity or a repair event.
Inspect `equipment_durability` with `equipment_durability_max_percent` to
shortlist slots at or below an inclusive whole percentage from 0 to 100.
The calculation uses the exact captured ratio, with no rounding. Missing
durability data gives null, and the raw capture stays available.
The same threshold on `list_snapshots` finds saved Forever captures with at
least one matching slot and returns the matching slot numbers. Missing or
empty observations do not match.
The character summary's `repair_status` shows whether the saved wallet
covers the repair-all quote and how much copper would remain if paid. The
remainder is null when the wallet is short or either input is missing.
Inspecting `repair_quote` returns the same saved `repair_status` alongside
the raw quote.
Use `repair_min_remaining_copper` with `list_snapshots` to find saved
Forever repair quotes that leave at least that much copper. A reserve of
zero still requires a quote and enough saved copper to pay it.

For an explicit saved character, `resolve_character` accepts inclusive
`captured_from` and `captured_through` timestamps. It selects the newest
matching capture within those bounds. Without an explicit character, active
identity stays unknown and time bounds are rejected.
The optional `exact_client` requires the capture to match the supplied
family, variant, build and interface. It also requires an explicit character.
The owned-gear evaluator accepts `--character-client VARIANT BUILD INTERFACE`
with a named character to apply that exact constraint before collecting gear.
With a named character it also accepts inclusive RFC 3339
`--captured-from` and `--captured-through` bounds, optionally combined
with the exact client constraint.

Inspect `trainer_services` with either `trainer_max_price_copper` or
`trainer_max_price_points` to apply an inclusive ceiling to known lesson
prices. Combine with the recorded `available` status to shortlist lessons
that were available at capture. Budget projections use the retained rows.
Use `trainer_affordable_only` to compare each known lesson price with the
saved copper or pet training-point balance. A missing balance leaves the
shortlist unknown. Apply the availability filter separately to select
lessons reported available at capture.
Set `trainer_balance_reserve` with an affordability filter or budget projection to reserve
copper for general trainers or training points for pet trainers. The reserve
affects shortlist selection when the filter is enabled. Budget projections still report against the
original recorded balance and add an `after_reserve` breakdown when a
reserve is supplied. This checks whether all retained available lessons
fit together after the reserve and lists the cheapest known lessons that
fit the remaining balance. Without the affordability filter, the reserve
does not remove lessons from the review.
The reserve breakdown also reports each retained available lesson's
affordability and the extra balance needed to cover both the lesson and
the reserve. An unknown price leaves the exact shortfall unknown.
The total shortfall includes any gap in the reserve itself. If prices
are missing, `known_lessons_and_reserve_shortfall` gives the shortfall
for known costs. A positive value proves the retained lessons do not
all fit the saved budget.
With `trainer_service_slots`, the after-reserve breakdown also reports how
many lessons fit in the requested order and identifies the first blocker.
A slot excluded by the shortlist is reported as `lesson_not_retained`.
For any explicit slot request, `trainer_filtered_requested_slots` lists
captured slots excluded by the filters, in request order. Slots absent from
the capture are reported separately in `trainer_missing_requested_slots`.
The filtered list is null if the trainer entries or affordability balance
needed to evaluate the filters are unavailable.
The ordered result is null if the captured balance cannot cover the reserve.

Inspect `flight_map` with `flight_max_fare_copper` to shortlist reachable
destinations with known fares at or below the ceiling. The filter excludes
map transitions and can be combined with captured wallet affordability.
Set `flight_route_search.maximum_total_fare_copper` to cap the sum of fares
across all legs. The cap works independently of a saved wallet. If wallet
affordability is also required, the route must satisfy both limits.

Saved pet food location summaries group bag quantities and stack counts by
accepted food item ID. Totals include matching stacks beyond the displayed
location limit and describe the saved bags.
Inspecting the saved `pet_care` section includes the same
`pet_food_locations` report. Missing food observations remain null.
Pet-care comparisons count food IDs in each saved observation and label
whether each changed ID appeared on either side. A one-sided observation
does not establish why a food disappeared from the accepted-food list.

Use `list_snapshots` with `toy_in_view` to find an exact toy item ID in a
saved filtered journal. Optional `collected` and `favorite` flags must
match that row. An absent toy does not establish ownership status.
The same selector works with `inspect_snapshot` on `toy_collection_view`.
It adds a match alongside the saved view. Use one selector per inspection
when choosing between `toy_in_view` and `toy_item_id`.
Character summaries count favourites and collected favourites in the
saved filtered view. Empty views report zero and unavailable views report null.
Inspect `toy_collection_view` with `toy_collected`, `toy_favorite`, or both
to shortlist matching rows without specifying an item ID. The displayed
count still describes the original capture. `toy_matched_count` counts
the retained rows or reports null when the view is unavailable.
Toy comparisons count IDs visible in both saved views and how many of those
had unchanged collected and favourite flags.

Saved mount comparisons count reported usable mounts, favourites, and
mounts with both flags in each capture. These counts describe the saved
journal and do not establish current summoning eligibility.
Character summaries expose the same counts under `mount_observed_flag_counts`.
An unavailable journal returns null and an observed empty journal returns zero counts.
Use `collected_mount` with a mount ID and optional usability or favourite
flags when inspecting `mount_collection`. The lookup reports a match
separately from the original captured view.
To shortlist mounts without an ID, inspect `mount_collection` with
`mount_usable`, `mount_favorite`, or both. `mount_matched_count` counts the
retained rows. The catalogue count still describes the original capture.

Saved pet journal comparisons report how many species were visible in both
captures and how many of those kept the same observed owned and unowned row
counts. The overlap describes the saved views, whose filters may differ.
Species discovery accepts `maximum_owned_rows` and `maximum_unowned_rows`
alongside the existing minimums. Zero matches an observed zero count for
that species, while an absent species does not match.
Use the same `pet_species` selector when inspecting `pet_collection_view`.
The response adds `pet_species_match` while preserving the captured view.
To shortlist without a species ID, inspect `pet_collection_view` with
`pet_has_owned_rows`, `pet_has_unowned_rows`, or both. False requires an
observed zero count for that flag. `pet_species_matched_count` counts the
retained species while the displayed row count describes the original capture.
Character summaries count species with only owned rows, only unowned rows,
or both in the saved view. The breakdown describes visible observations.

Saved mail attachment comparisons show the unloaded message count on each
side, separately from unreadable attachment slots in loaded messages.
These counts describe the captures and do not identify messages or transfers.
Character summaries show the same unloaded count alongside observed item,
currency and unreadable attachment slots.
Attachment search matches carry both coverage counts so observed quantities
can be read alongside gaps in the saved inbox.
Crafting plans that show pending mail reagents also report unloaded messages.
Pending quantities remain separate from available crafting materials.

Merchant discovery accepts `merchant_stock_state` with an existing offer
selector. Use `unlimited`, `limited_in_stock` or `sold_out` to match the
stock reported for the same offer in the saved visit.
The same filter works when inspecting `merchant_stock` with an item,
currency or pet food selector. Quantity plans use only retained offers.
Quantity quotes report `captured_stock_covers_quantity` as true for
unlimited stock, false for sold-out stock, and null for positive limited
stock because the count's unit semantics remain unverified.
The inspection quantity plan identifies the cheapest retained single offer
with unlimited captured stock and a copper-only price. It compares the total
cost after rounding to whole bundles, with numeric slot order breaking ties.
For an exact item or currency search, `lowest_cost_copper_only_bundle_mix`
can combine whole bundles across eligible slots in the same visit. It
minimises total copper, then total units, within the retained unlimited
copper-only offers. Pet food searches can return different products and do
not combine their quantities. All plans are historical quotes.
Set `merchant_combine_bundles: true` on `list_snapshots` with an exact item
or currency and a target quantity. The total copper ceiling, affordability
filter and cheapest-first order then use the combination cost. Offer counts
describe eligible slots, and each plan uses one visit.
Recipe and restock quotes also report per-reagent bundle combinations and a
combined copper budget when every shortage can be quoted. The optimiser
covers up to 1,000 units per reagent. Larger or unknown shortages leave the
combination budget unknown. Single-offer quotes remain available.
Partial quotes identify unpriced item IDs and show the subtotal of priced
reagents. Each unpriced row distinguishes an unknown shortage, a shortage
outside the optimiser range, and a shortage with no eligible offer.
Affordability still requires a complete quote.
The quoted subtotal's wallet shortfall can be calculated even for a partial
quote. A zero subtotal shortfall only covers the priced rows.

Comparing a saved quest by ID reports fulfilled, required and remaining
quantities by objective kind for each capture. The report counts rows
without quantities separately and leaves totals null when no counts are
reported for that kind. Objective slot numbers do not establish identity.
Quest log inspection accepts `quest_objective_min_remaining` and
`quest_objective_max_remaining` independently or together. Both endpoints
are inclusive and must match the same objective as any kind or finished
filter. Missing counts stay unknown, and reversed bounds are rejected.
Use `quest_projected_expiry_by` when inspecting a saved quest log to find
reported timers projected to expire by an inclusive RFC 3339 deadline.
Projection uses the capture timestamp plus saved remaining seconds.
Comparing a saved quest by ID shows both projected expiry timestamps and
whether they differ when both timers are available. A countdown can change
the reported remaining seconds while leaving projected expiry unchanged.
Missing timers stay unknown. Current quest status requires a fresh capture.
Deadline results include `quest_projected_expiry_by_quest_id` for the quests
that passed every filter. The map is empty when no quest matches and null
when the saved quest log is unavailable.
Use `quest_projected_expiry_from` for an inclusive lower bound, alone or
together with the deadline. Reversed windows are rejected.
`character_summary` lists up to 20 saved timed quests, shortest reported
timer first, with historical expiry and turn-in readiness. Quests without
reported timers are counted separately.
Use `quest_list_timed_checklist: true` when inspecting the `quests` section
to build the same checklist from quests retained by your filters.

`compare_snapshots` accepts `saved_world_boss_changes: true` to compare
reported reset durations by unique case-insensitive boss names. Duplicate
names are ambiguous. Names seen on only one side are counted separately.
This is a comparison of captured values, without verified lockout identity.

Saved instance and world boss discovery accept `projected_reset_by` alongside
the exact name filter. The deadline is inclusive and uses the capture time plus
the reported duration. Negative timers are unevaluable. A match describes
a historical projection, with current lockout and loot eligibility unknown.
Add `projected_reset_from` for an inclusive lower bound. Either bound can be
used alone. Reversed windows are rejected.
Matches from a time-bounded search include the historical
`projected_reset_at` timestamp on each returned row.

Use `compare_snapshots` with `loot_window_changes: true` to compare two saved
loot windows. Readable item totals and raw currency totals are separate, and
money and unreadable slot counts retain each capture's scope. Missing rows
stay null even when an explicit zero is recorded on the other side. The
comparison does not identify a shared source or credit inventory or balances.
Use `list_snapshots` with `loot_observation: {"kind":"unreadable_or_non_item"}`
to find captures with at least one such slot. Results identify the saved
slot numbers without assigning items or quantities to them.

Set `loot_estimate_bag_space: true` alongside `loot_requirements` in
`inspect_snapshot` to estimate bag space for the readable portion of
those requests. The report uses one pool of general free slots and stack
limits from the same capture. Unrequested or unreadable loot is outside
the estimate. Same-item merges are hypothetical, and loot eligibility
and placement remain unverified.

In the `mail_attachments` section, pass `mail_attachment_bag_space_item_ids`
with one to 64 distinct item IDs. The estimate covers all readable loaded
attachments matching those IDs and uses the same capture's carried bags.
Unloaded messages and unreadable slots are counted separately. Currency
attachments do not count as items. The estimate leaves retrieval and
placement unverified.

Saved profession recipe-view comparisons report changes in the client-reported
skill-up flag, count and relative difficulty for recipe IDs seen in both
captures of the same profession. Missing observations remain null, and the
comparison does not predict skill gains.
Use `recipe_view_filter.relative_difficulty` to match `optimal`, `medium`,
`easy` or `trivial` from the same profession's saved recipe view. Missing
difficulty stays unknown. This filter can be combined with materials and
skill-up observations.

Use `compare_snapshots` with `recipe_cooldown_changes: true` to compare bulk
cooldown observations for shared recipe IDs in the same profession. The
report counts all changes and shows up to 50 in numeric recipe ID order.
An absent observation stays null, and a recipe present in only one capture
does not establish a cooldown transition.

Saved skill-line discovery and inspection accept exact signed `modifier` and
`temporary_points` predicates. They match on the same skill row as base rank
and category predicates. Missing values remain unknown, explicit zero is
searchable, and the two fields stay separate from base rank.
Snapshot comparisons report changes in those fields for skill IDs seen in both
captures. The delta is available only when both captures report the field.

Local world references: optional NPC/object spawn records can accompany captured
reagent sources and flow into chain acquisition reviews without repeated external
lookups. Provider area percentages, phase IDs and instance-presence sentinels stay
distinct from native maps/visibility. Only relevant cached entities are returned;
missing/truncated coverage remains explicit. No routing, movement or data-provider
installation is performed. [Evidence](../../docs/reference-locations-verification-2026-11-09.md).

Loot requirement review compares captured readable base-item quantities with an
explicit shopping/reagent list. Unknown slots stay explicit; coverage and remaining
needs are conditional on acquiring known displayed items, without pickup rights
or owned-stock credit. All acquisition remains with the player.
[Evidence](../../docs/loot-requirements-verification-2026-11-09.md).

Spell-resource discovery now filters captured current-charge/cast-count ranges
and cooldown enabled/active flags on the same row as spell identity/metadata.
Inspection shares the predicates; unknown versus zero/false remains explicit.
Resource-only saved-capture searches are supported without inferring current
casting eligibility or activity. [Evidence](../../docs/spell-resource-search-verification-2026-11-09.md).

Spell resources: manual capture can retain stable readable charge counts, reported
cast counts and enabled/active cooldown flags. Saved summaries distinguish missing
observations and action-bar references use exact reported spell IDs, preserving
duplicate rows and disagreements. Timers/GCD, recharge and casting eligibility are
not inferred; all casting/targeting remains with the player. Native qualification
is deferred. [Evidence](../../docs/spell-resources-verification-2026-11-09.md).

Loot observation: manual export can inspect an already shown window and retain
stable public item IDs/quantities, raw reported currency IDs/quantities and
money-slot presence, with explicit unreadable/non-item slots. Local
inspection and summary keep displayed loot separate from owned inventory. No
source/player identity, currency-container reward conversion, money amounts,
lootability inference, looting,
rolling or panel action is added. Native qualification remains deferred.
[Evidence](../../docs/loot-window-verification-2026-11-09.md).

Cached crafted-item acquisition lookup now preserves selected-recipe item/charge
capacity semantics, including explicit no-item requirements and unknown limits.
Recipe-ID/known-output guards apply before projection; bulk and selected evidence
stay distinct and learning conflicts stay visible. These are one-craft historical
observations, with current eligibility still unverified.
[Evidence](../../docs/catalogue-recipe-capacity-verification-2026-11-09.md).

Single/bulk recipe preparation distinguishes no required items from unavailable
material capacity. Explicit zero-item fixed recipes retain their saved charge
limit without needing an inventory capture; unknown charges and unsupported
requirements stay unknown. Current readiness and crafting remain with the player.
[Evidence](../../docs/zero-item-recipe-capacity-verification-2026-11-09.md).

Repeated producers in captured reagent chains now consume one modeled saved
charge pool per recipe across their printed steps. Step and stock-plan checks
expose cumulative shortages independently of material feasibility. Unknown counts
stay unknown, newer stock does not replenish them and no craft is executed.
Each step also reports how many of its requested crafts fit within the
captured charges left after earlier steps for the same recipe.
The step and stock-plan charge reports also show the conditional base yield
for those crafts. This estimate does not change the chain's material plan.
The stock plan separately highlights the first step with a known charge
shortfall. Earlier unknown charges or other failed requirements do not hide
that entry. Null means no charge shortfall was observed in the saved plan.
The stock plan also reports `observed_charges_cover_chain`. A known shortage
makes it false even if other steps are unknown. True requires known charge
coverage for every step, and otherwise the result is null.
Bulk recipe reviews and chain steps use the selected recipe's saved crafting
conditions when the exact recipe ID and compatible schematic are in the same
capture. The chain reports condition checks per step and leaves unmatched
recipes unknown. Each step lists the saved unmet condition rows under
`unmet_crafting_conditions`. An observed list with no unmet rows is empty,
and an unavailable list is null. A newer inventory snapshot does not supply
conditions.
The stock plan counts steps with met, unmet and unknown conditions. Its
`observed_chain_conditions.all_met` is false when any step has an observed
unmet condition, true when every step's saved condition check passes, and
null otherwise. Repeated recipe steps are counted separately.
The stock plan's `captured_chain_checks_pass` combines starting materials,
charges and crafting conditions for bags alone and for all observed storage.
A known failure makes that result false, all three passes make it true, and
other cases remain null. These checks describe the saved plan only.
The stock plan's `first_step_to_review` points to the earliest material
checkpoint or observed nonmaterial shortfall, including both when they
occur at the same step. It uses all observed storage for the material
checkpoint. Null means neither checkpoint was reported.
The stock plan also lists positive suggested bank withdrawals by item, using
the character bank before the account bank and showing any bag shortfall
left after those suggestions. It only lists observed quantities for known
bag shortfalls. See starting materials for missing stock observations.
The accompanying location report allocates those suggestions to saved bank
containers and slots, showing the observed stack quantity and suggested
quantity per slot. It lists up to 50 locations and marks omitted allocations
as truncated. The full per-item withdrawal totals remain available.
The report's `total_locations` counts all suggested slots across both banks,
including those beyond the display limit.
Set `reagent_chain_withdrawal_offset` with the chain stock plan to request
another page of up to 50 slots. Keep the recipe, stock and plan inputs the
same between pages. `has_more` indicates locations after the current page,
and `truncated` indicates that the page shows fewer than the full total.
Within each bank, larger stacks are used first to minimise slots per item.
Equal quantities are ordered by numeric container and slot.
The separate `acquisition_shortfalls_vs_observed_storage` list shows positive
starting-material shortfalls after counting all observed storage. It can
include items with no suggested withdrawal and uses the same starting
requirements as the printed chain. An empty list means no positive shortfall
was observed. Uncaptured storage may contain additional stock.
When several saved recipes could produce the same reagent, the chain reports
the uncovered quantity and estimates for up to 20 candidate recipes. It
shows crafts needed at the captured minimum yield, whether that fits the
step limit, and whether the raw saved charge count covers that request.
Candidates within the step limit also show gross direct reagent quantities.
Those quantities exclude nested recipes and do not deduct stock. They are
null when the craft count is unknown or exceeds the step limit.
The separate `direct_reagent_shortfalls_vs_chain_balance` map deducts the
modeled balance at the point where the chain stopped. That balance includes
observed stock and any output already modeled by earlier steps. The map
contains only positive shortfalls and excludes nested producer plans.
Charge estimates do not deduct earlier usage. An explicit producer choice is
still required.
Candidates marked `already_active_in_chain` are on the current dependency
path. Choosing one at that point creates a cycle. Other candidates can
still lead to a cycle deeper in their dependencies.
Set `preview_reagent_producer_choices` in chain mode to rerun the plan
independently for up to four candidates at the first ambiguous reagent.
Each preview adds one explicit choice to those supplied and shows the
resulting steps, external material requirements or next blocker. It returns
null if the original plan does not stop on ambiguity. Further ambiguous
choices remain unresolved. The original draft and player choice stay separate.
With previews enabled, `reagent_producer_preview_offset` pages through
all candidates in numeric recipe ID order, four per request. The preview
reports `total_candidates` and `has_more` independently of the diagnostic
list of up to 20 candidate IDs.
When `reagent_chain_stock_plan` is also enabled, each completed preview
summarises its saved material and preparation checks, acquisition shortfalls
and suggested bank withdrawals. A blocked preview has a null stock summary.
Slot paging still applies to the original plan only.
With `merchant_reagent_plan` enabled, alternative previews also summarise
saved merchant bundle quotes for their external shortages. The summary
distinguishes a complete quote from a partial subtotal and reports
affordability against the saved inventory wallet when available.
With `reagent_chain_captured_sources` enabled, each completed preview also
shows up to 20 saved source records for its external requirements, with
the source snapshot ID and truncation status. It uses the same owner and
client checks as the explicit plan. Blocked previews return a null summary.
With `reagent_chain_catalogue_sources` enabled, completed previews also
show up to 20 acquisition records from the verified local catalogue, with
catalogue provenance and truncation status. Catalogue and captured source
summaries are reported separately.
Chain steps also show a captured default-level craftable count when available,
including its shortfall against that step's requested crafts. This is a
point-in-time observation. It stays separate from cumulative charge checks
and material planning, where earlier steps can supply ingredients.
Each step includes `recipe_tracking` from the recipe capture, with craft and
recraft membership reported separately. Missing tracking observations stay
unknown. Membership alone does not change recipe selection. Conflicting
saved learning flags still reject the recipe.
Each step also shows the cooldown state at recipe capture and an estimated
end timestamp when a positive timer was reported. A recipe can have a
charge available while a timer is running. The timestamp neither replenishes
the modeled charge pool nor establishes current readiness.
Individual recipe reviews additionally report
`base_output_for_charge_covered_crafts_if_completed`. This scales the base
yield range by the smaller of requested crafts and captured charges. It is
null if either input is unavailable, and it checks charges independently
of materials, other crafting conditions and shared cooldowns.
Shared cooldown groups and actual recharge/readiness remain unqualified.
[Evidence](../../docs/chain-charge-accounting-verification-2026-11-09.md).

Bulk recipe discovery can narrow historical cooldown state, charges and timer
together with learned/skill-up and material predicates. Results retain original
cooldown provenance while compatible newer inventory updates stock only. Explicit
unknown state is searchable; missing numeric observations stay unevaluable. No
elapsed recharge, live craftability or complete profession coverage is inferred.
Cooldown-filtered or timer-ordered rows show an estimated end for positive
reported timers, based on the original recipe capture. A timer can run while
a charge was available. Zero and missing timers return no end estimate.
Use `cooldown_filter.estimated_end_by` with an RFC 3339 timestamp to shortlist
estimated ends at or before a deadline. Rows without an estimate are
unevaluable for this predicate, including rows with a reported zero timer.
Use `cooldown_filter.daily` to match an explicit true or false daily flag.
Missing flags remain unknown. The filter uses the captured flag and makes no
projection about server reset times.

Use `order: "reported_base_output_minimum_desc"` with an output item filter
to compare per-craft base yields for that item. Missing quantities sort last,
and recipe IDs break ties. Material and cooldown filters still apply before
paging. The ordering uses the saved schematic's base range and excludes
bonus output and total craftable quantity.
Use `minimum_base_output` to require a positive lower bound per craft.
Missing yield ranges are excluded when this filter is supplied.
Use `reported_base_output_per_reagent_desc` with both `output_item_id` and
`reagent_item_id` to sort by minimum base yield divided by the captured
required quantity of that reagent. Missing yields sort last. The comparison
accounts for the selected reagent only.
[Evidence](../../docs/recipe-cooldown-search-verification-2026-11-09.md).

Bulk fixed-recipe observations now carry optional stable public cooldown/charge
evidence into search, individual/batch reviews and existing nonmaterial planning.
Recipe and stock provenance remain separate. API absence, restrictions, changing
tuples and failures preserve unknown cooldown evidence without discarding valid
materials. This is passive capture and offline planning; crafting stays with the
player. [Evidence](../../docs/bulk-recipe-cooldown-verification-2026-11-09.md).

Offline saved gear evaluation can optimise DPS, DTPS, threat, death fraction,
healing or time out of mana across supplied encounters. It preserves native
improvement units, correct increase/decrease direction, shared budgets, selected-
metric top retention and historical loadout provenance. DPS remains the default.
Explicit role templates and encounter configuration are still required; full
healing/tank/class model accuracy and native eligibility remain unqualified.
[Evidence](../../docs/role-gear-ranking-verification-2026-11-09.md).

Exact positional gear collection includes equipped-source vacancies within the
requested slot scope, extending the feasible layouts beyond only moves whose
source is refilled. Selected-replacement budgets, physical-copy constraints and
resolution guards remain. Legacy mode retains its old exclusions. This is an
offline desired layout; players apply gear themselves.
[Evidence](../../docs/equipped-vacancy-verification-2026-11-09.md).

Inventory scope evidence now survives gear-window aggregation, journal restart
and unified evaluation. Missing banks and unmatched catalogue items remain
visible alongside a computationally complete ranking. Strict metadata checks
reject malformed or changed evidence; legacy absence stays unknown. Observed
sections never establish complete ownership, native eligibility or full BiS.
[Evidence](../../docs/gear-inventory-propagation-verification-2026-11-09.md).

Recipe material discovery can combine original saved schematics/recipe-view
flags with a no-older same-character exact-client/build inventory capture.
`inventory_snapshot_id` requires `material_filter`; stock coverage comes solely
from that inventory snapshot, with explicit timestamps and no old-bank fallback.
This extends discovery without refreshing or inventing recipe knowledge.
[Evidence](../../docs/recipe-search-inventory-verification-2026-11-09.md).

Saved-journal and unified gear evaluation can rank by weighted average DPS gain
or `worst_encounter_gain`, which maximises the minimum observed gain across all
supplied encounters. The selected objective drives bounded top retention across
pages and reuses the same simulation checkpoints. Neither mode establishes
statistical optimality or covers encounters that were not supplied.
[Evidence](../../docs/robust-gear-ranking-verification-2026-11-09.md).

Saved-journal and unified owned-gear rankings now return the retained loadout
and saved physical source selections alongside each ranked gear hash. A checked
journal reference binds the snapshot, search scope and numeric equipment.
Current locations and native equip eligibility remain unverified; players apply
any gear changes themselves. [Evidence](../../docs/ranked-loadout-verification-2026-11-09.md).

MCP equipment review and offline owned-gear evaluation support explicit
positional equipment input. The opt-in preserves paired-slot gaps while keeping
other catalogue/native-category/level/legality guards. Execution requires the
qualified hash and exact engine capability, including cached/checkpoint paths.
Legacy placement remains the default and positional bulk execution is refused.
Native/model/ownership/BiS proof remains open.
[Offline evidence](../../docs/positional-integration-verification-2026-11-09.md).

The offline `tools.evaluate_forever_owned_gear` module connects existing local
MCP all-slot collection to multi-encounter saved-journal ranking. It requires an
explicit saved snapshot, trusted pinned local engine/template, encounter files
and two separate journal destinations. All 17 slots and observed storage are
included with no item/set/swap filters. Re-running resumes bound collection and
reuses result checkpoints. Coordinate coverage, unresolved proposals and saved
simulation completion remain separate; partial/model-unsupported plans never
become verified full owned BiS. It does not infer the active character or perform
game actions. Live testing, installation and deployment remain deferred.
[Offline evidence](../../docs/owned-gear-evaluation-verification-2026-11-09.md).

Optional source-only `recipe_item_sources` capture reads a separately installed
compatible QuestieDB during manual export. It uses already captured fixed
recipe reagent IDs, queries the numerically first 64 and records all-ID count.
Five source-ID categories retain eight unique IDs each with count/truncation;
missing, secret, invalid or unstable evidence stays unavailable or omitted.
Inspect this owner-local section with `inspect_snapshot`. No provider dataset,
network refresh, installation, game action or private name/coordinate capture
is added. Dataset revision, native behaviour, completeness and availability
remain unqualified. Chain joins are available through the opt-in below; source
entity resolution and routing remain unfinished.
[Offline evidence](../../docs/recipe-item-sources-verification-2026-11-09.md).

`reagent_chain_captured_sources: true` adds `reagent_chain.captured_sources` to
an exact standalone chain review. It joins net external shortages by exact item
ID to the recipe capture's saved provider references. An explicit
`reagent_source_snapshot_id` can select another same-character, exact-client/
build capture, even older or from another profession. Its timestamp/coverage
remain separate from inventory and recipe provenance. Missing capture, absent
item, provider miss and unavailable read are distinct; missing/truncated source
categories remain visible. No catalogue/provider/network is needed at query
time. Blocked drafts return null after explicit-reference identity checks.
These annotations do not alter stock, quotes, catalogue annotations or verified
eligibility, and do not execute an acquisition route.
[Offline evidence](../../docs/chain-captured-sources-verification-2026-11-09.md).

`review_forever_recipe` can add `reagent_chain_catalogue_sources: true` in exact
standalone chain mode. This requires a configured verified local catalogue and
adds nested `reagent_chain.catalogue_sources`: net shortage quantities, public
item names, up to eight source records per item, corresponding cached NPC/zone/
faction references, provenance and match/source counters. Blocked chains return
null; complete empty demand returns an empty record list. Missing items and
source metadata stay explicit. References neither alter owned stock/quotes nor
prove availability, eligibility or full world coverage. Common reagents are
absent from this partial cache; missing references do not mean unobtainable.
[Offline evidence](../../docs/chain-catalogue-sources-verification-2026-11-09.md).

Fixed bulk recipe rows can retain `reported_craftable_count` from two stable
public API reads during manual export. Exact-ID review exposes the count,
recipe capture time and saturated requested-craft shortfall separately from
material capacity. Missing or invalid counts are omitted. Inventory overrides
do not refresh this historical default-level observation; counts do not alter
chain, stock, quote or eligibility decisions. Native behaviour remains untested.
[Offline evidence](../../docs/reported-craftable-count-verification-2026-11-09.md).

Complete chain drafts can opt into `reagent_chain_stock_plan`, revalidating
printed steps and reusing ordered-material accounting for starting needs,
saved bank-withdrawal quantities, scoped coverage and nonmaterial unknowns.
Conditional intermediate output reduces initial needs; future acquisitions do
not become owned stock. Checkpoints use all observed storage, with separate bag
readiness. Blocked chains return null. No transfer, current access or crafting
eligibility is verified or executed.
[Offline evidence](../../docs/reagent-chain-stock-verification-2026-11-09.md).

Complete reagent-chain drafts can opt into saved merchant quoting of their
external leaves. Already-net quantities are priced once after shared stock
allocation, with no separate intermediate/root quote. Offer captures retain
character/client guards; affordability uses only the inventory wallet. Blocked
chains and absent offers stay unknown, and no purchase eligibility or action
is inferred. Ordinary non-chain quote behaviour is unchanged.
[Offline evidence](../../docs/reagent-chain-merchant-verification-2026-11-09.md).

Reagent-chain drafts accept up to eight explicit `reagent_producer_choices` to
resolve captured producer ambiguity. Every unique positive item/recipe mapping
must match a fixed output in the same bulk capture. Only uncovered demand
consults a choice; unused valid mappings do not force production. Requested and
consulted choices are reported without claiming completed crafting. Existing
conflict, inventory, yield, cycle and budget guards remain; no fallback or action
is introduced.
[Offline evidence](../../docs/reagent-producer-choices-verification-2026-11-09.md).

An exact bulk recipe review can opt into `suggest_reagent_chain`. The bounded
producer draft reuses observed stock and conditional minimum-yield leftovers,
chooses only unambiguous captured fixed producers, and lists external material
needs. Ambiguity, cycles, missing evidence and step/craft limits block expansion.
Draft completion is not ownership or eligibility; modeled acquisition/output
does not become inventory. No price optimisation or gameplay action is performed.
[Offline evidence](../../docs/reagent-chain-verification-2026-11-09.md).

Bulk recipe discovery can join captured recipe-view learned, skill-up and
minimum reported-point predicates through `recipe_view_filter`. Exact recipe
and profession IDs must agree. Same-row three-valued matching preserves
unknown fields; explicit mismatches win over unknowns. View predicates precede
optional material support, with separate staged counts and bounded paging.
Projected observations omit recipe names and never verify aggregate learning,
guaranteed gains, full knowledge or crafting eligibility. Existing review still
blocks conflicting learning evidence.
[Offline evidence](../../docs/recipe-view-search-verification-2026-11-09.md).

Recipe discovery can require observed fixed material support with
`search_forever_recipes.material_filter`: explicit bags or all-observed-storage
scope and a positive minimum craft count. Counts aggregate once from the same
owner-local snapshot, then each captured row is tested independently. Matched
rows report capacity; unsupported/missing data remains unknown, separately from
insufficient observed counts. No-cost fixed recipes have no finite material
bound. Other filters apply to the same row before material filtering and paging.
This does not infer learning, cooldowns, joint recipe budgets, ownership
completeness or live craftability. Default search is unchanged.
[Offline evidence](../../docs/recipe-material-search-verification-2026-11-09.md).

Restock crafting can opt into `restock_merchant_reagent_plan`. It prices combined
reagent shortfalls once, rather than pricing each recipe against the same stock
independently. Offers default to the inventory capture or an explicit compatible
`restock_merchant_snapshot_id`; the wallet always comes from the acquisition
inventory. Missing offers yield null, and missing applicable unlimited copper-only
quotes leave totals/affordability unknown. Quotes retain their capture timestamp
and do not verify purchase eligibility or execute transactions.

Consumable target lookups can supply `restock_recipes`, one to eight objects
with `snapshot_id` and optional `recipe_id`. Exact IDs use fixed bulk materials;
omission uses the selected schematic. Multiple different outputs may use one
capture. Inputs are exclusive with `restock_recipe_snapshot_ids`. All plans use
the lookup's same compatible inventory and confirmed shortages, round desired
output using captured minimum yield, and combine shared reagent demand. Unknown
shortages, duplicate outputs, unsupported/unreadable/conflicting bulk rows and
restock targets used as reagents reject the request. No crafting is executed.

Item catalogue acquisition sources expose `bulk_recipe_review` separately from
selected-schematic fields. Exact-ID compatible bulk rows use the existing
one-craft planner against the same capture. Learning conflicts and known output
mismatches block the review; missing rows are null, unreadable/budget states
carry no plan, and planner rejection is a generic `review_unavailable` state.
Unsupported required materials remain unsupported inside a completed review.
No cooldowns, conditions, stock or craft eligibility are invented.

Catalogue `only_observed_learned_recipe` searches can use exact-ID learning from
bulk schematics and tracked craft/recraft rows alongside selected/list/known-spell
observations. Fixed or unsupported-readable bulk statuses support positive
learning; unavailable/budget rows are unknown. Conflicting flags or known selected
or bulk output mismatches suppress a positive result. The nested learning record
reports separate bulk status, output match and tracked flags without names.
Selected-only material capacity, cooldowns and conditions keep their original
scope; this join is discovery, not a crafting eligibility check.

`search_forever_recipes` discovers captured bulk material rows by exact recipe,
output item, required reagent and/or status, with same-row filters and bounded
numeric ordering. It reads only the chosen owner-local snapshot. Missing sections
return unknown counts; observed-empty sections return zero. Results retain the
captured status and materials, and their IDs work with recipe planning. Searches
do not refresh captures, consult a catalogue, prove learning/craftability or
execute game actions.

Bulk profession material capture observes the visible local profession's
reported known spell IDs without selecting recipes or changing filters.
`profession_recipe_materials` retains fixed item quantities and optional output
ranges, or an unavailable/unsupported/budget state per ID. IDs are processed in
numeric order; at most 16,384 reagent pairs are retained across at most 4,096
rows. Missing data is not a known-empty material list. `review_forever_recipe`
can select an exact bulk recipe with `recipe_id` on the primary or additional
batch entries. Omission preserves selected-schematic review. Bulk material
plans retain unknown cooldowns and conditions. Same-ID false learning in selected,
filtered or tracked rows blocks review; conflicting selected-schematic materials,
output or profession ID also block it. Native availability, responsiveness,
complete recipe knowledge and craftability remain unverified.

Updated 8 November 2026. This map records source evidence and engineering scope. A callable API or a passing fixture does not qualify a live build or establish authorisation.

Saved buyback quote review can compare selected whole-entry quotes with the same
captured wallet after a reserve. Unknown prices, missing slots and unknown wallet
remain explicit. Captured indices are historical references, not a click sequence;
purchase, recovery eligibility and inventory changes are not performed or inferred.
[Offline evidence](../../docs/buyback-cost-review-verification-2026-11-08.md).

Manual merchant buyback capture records stable IDs/quantities and readable entry
quotes only while the buyback tab is visible. Names and merchant identity are
omitted. Buyback never contributes owned stock, funds or simulator candidates;
current recovery eligibility and repurchase actions remain unverified/unperformed.
[Offline evidence](../../docs/merchant-buyback-verification-2026-11-08.md).

Local item catalogue searches can use saved gossip available/active quest IDs
and explicit completion flags. Quest-ID and dialogue predicates must match one
reward source. Missing lists remain unknown, separate from quest-log observations;
catalogue references do not prove native reward choices or turn-in eligibility.
[Offline evidence](../../docs/gossip-reward-lookup-verification-2026-11-08.md).

Manual capture can observe available/active quest lists from an already visible
gossip frame. IDs and stable public flags survive without titles, dialogue or
speaker identity. Lists remain historical and partial; acceptance/turn-in
eligibility and NPC continuity are unverified. No dialogue action is issued.
[Offline evidence](../../docs/gossip-quests-verification-2026-11-08.md).

Consumable catalogue lookups can join exact recorded effect spell IDs to saved
helpful aura observations. Unknown lists/reference associations remain unknown;
shared matches do not identify a consumed item. No inactivity, effect strength,
stock change, resource use or simulator configuration is inferred.
[Offline evidence](../../docs/consumable-aura-matches-verification-2026-11-08.md).

Manual player aura capture reads helpful/harmful lists independently and retains
only stable public nonprivate spell IDs plus readable application counts. Private,
restricted or changing lists remain unknown. Caster names/GUIDs, instance IDs,
timers and targets are omitted; no aura is cancelled, applied or simulated.
[Offline evidence](../../docs/player-auras-verification-2026-11-08.md).

Opt-in saved comparisons cover reported profession spell sets and player resource
counters. Profession ID and primary-power context must match for derived results;
missing observations stay unknown. Spell additions/removals are reported-set
differences, and counter changes do not prove learning, healing or resource use.
The profession comparison also counts spells reported in both captures and
lists up to the first 50 shared IDs in numeric order, with a truncation flag.
[Offline evidence](../../docs/observation-comparisons-verification-2026-11-08.md).

Visible local profession capture can retain stable reported known spell IDs
without filter changes. Exact crafted-source IDs can support local learned-output
searches; conflicts stay unknown. The spell report does not establish complete
recipe coverage, reagent availability or current craftability.
Inspect `profession_spells` with `profession_spell_id` for an exact spell ID.
The lookup identifies the captured profession and reports true for a visible
ID, false for an ID absent from a recorded set and null when the set is
unavailable. The original capture remains in the response.
Use the same `profession_spell_id` with `list_snapshots` and `family:
"forever"` to find captures where that exact ID was reported. Results identify
the captured profession. Unknown or nonmatching captures are excluded.
[Offline evidence](../../docs/profession-spells-verification-2026-11-08.md).

Manual player-resource observations retain readable stable health and primary
power independently. Restricted, failed or changing components stay unknown;
power units are explicitly unmodified. Summaries expose deficits and percentages
without inferring combat readiness, death or actions to restore resources.
[Offline evidence](../../docs/player-resources-verification-2026-11-08.md).

Local crafted-output searches can match positive saved recipe-list learning by
exact spell ID. Selected/list conflicts and selected output mismatches prevent
positive learned matches. Crafted acquisition sources retain both observations;
material and cooldown planning still requires a compatible selected schematic.
[Offline evidence](../../docs/catalogue-recipe-learning-verification-2026-11-08.md).

Owned-gear `inventory_evidence` records observed and unavailable storage sections
plus catalogue-unmatched base IDs across the whole capture, independently of
query filters and pagination. All sections observed is distinct from complete
ownership, exact simulator resolution and best-in-slot coverage.
[Offline evidence](../../docs/owned-inventory-evidence-verification-2026-11-08.md).

The local ranker retains this evidence in `gear_search.inventory_evidence` and
records snapshot ID, requested slots, candidate filters and maximum simultaneous
swaps. Missing legacy evidence remains null; changing evidence aborts collection.
The report does not verify absolute best-in-slot.
[Ranker evidence](../../docs/rank-inventory-evidence-verification-2026-11-08.md).

`resolve_character` accepts an explicit client family and optional name/realm.
An explicit target resolves its newest valid saved capture; no target returns
unknown active identity without selecting another saved character. It does not
provide live detection or change snapshot-ID requirements of other tools.
[Offline evidence](../../docs/character-resolution-verification-2026-11-08.md).

The [coverage audit](../../docs/forever-coverage-audit-2026-11-08.md) separates
implemented observations from the remaining active-character, complete owned-gear,
local-data and native qualification requirements.

Public pet action-bar observations distinguish logical command and spell
slots, reported flags and unreadable slots. Pet/bar context must remain
consistent; missing metadata is not guessed. Names, textures, GUIDs and target
identity are omitted. No pet command, autocast toggle or spell is issued, and
simulator pet identity still comes from its separate validated observations.
[Offline evidence](../../docs/pet-action-bar-verification-2026-11-08.md).

Visible player-spell rows can include the public API's usability and
insufficient-power pair when two readable observations agree. Saved search
matches flags on the same row; summary counts distinguish unknowns from false.
The capture performs no casting, targeting, explicit loads or cooldown/range
inference and does not establish current cast eligibility.
[Offline evidence](../../docs/spell-usability-verification-2026-11-08.md).

Native linked-item sell-price observations support local reference-value
summaries per observed storage. Quantity arithmetic retains partial/unpriced
coverage and overflow rather than inventing values. Zero remains a reported
price, not proof of eligibility; no wallet credit or sale action is introduced.
[Offline evidence](../../docs/item-sell-price-verification-2026-11-08.md).

Known native slot-category contradictions in projected simulation equipment
now withhold resolved inputs, including baseline conflicts. Candidate reviews
and ranker skips expose the reason. Cleared/vacated/non-simulated slots are
excluded; unknown/conditional categories do not add a block. Draft inspection
remains possible and native equip eligibility remains unverified.
[Offline evidence](../../docs/native-slot-conflict-verification-2026-11-08.md).

Linked-item observations can include stable native equip-location tokens.
Saved equipment drafts annotate known slot categories, with unknown mappings
and two-handed off-hand placement left explicit. Category matches do not
establish class, level, dual-wield, unique-item or complete-set legality.
Category annotations remain separate from equip eligibility; known conflicts
withhold simulator inputs as described above. No game equipment is changed.
[Offline evidence](../../docs/native-equip-location-verification-2026-11-08.md).

Public weapon-enchant list observations cover main hand, off hand and ranged
slots independently, retaining reported type/ID/timer/charge data. Summaries
separate unreadable hands, empty lists and missing details; values remain
historical. Permanent link parsing and simulator inputs are unchanged. No
expiry/effectiveness inference, application or cancellation is performed.
[Offline evidence](../../docs/weapon-enchants-verification-2026-11-08.md).

Linked equipment and container-item observations can include stable native
numeric stat-token maps. Equipment reviews show shared-token differences
and separate one-sided observations; missing tokens are not imputed as zero.
This supports local gear inspection without replacing the pinned simulator
or asserting complete effects, eligibility or performance improvement.
[Offline evidence](../../docs/native-item-stats-verification-2026-11-08.md).

Exact equipped-slot durability capture complements existing low/broken alerts.
Only readable stable current/maximum pairs are saved. Maintenance summaries
show observed percentages and damaged/broken slots with explicit unobserved
coverage. No item continuity, repair price/eligibility or repair action is
inferred. [Offline evidence](../../docs/equipment-durability-verification-2026-11-08.md).

The filtered local profession list can expose optional stable difficulty,
skill-up flags and reported counts. Summary candidates require observed
learning and a true skill-up flag; they are bounded, ID-ordered and retain
unknown metadata. Exact-recipe search can require a flag/count without
treating unknowns as false or zero. These are reported observations, not
guaranteed gains, full profession coverage or crafting eligibility.
[Offline evidence](../../docs/recipe-skill-up-verification-2026-11-08.md).

Reachable flight destinations can include stable observed route-preview node
IDs from the visible panel. Saved route legs retain origin/intermediate/target
sequences as annotations, without splitting a quoted flight into multiple
itinerary legs or imputing segment fares. Unknown, unresolved, changing or
ambiguous previews remain absent. No flight is selected or taken.
[Offline evidence](../../docs/flight-route-preview-verification-2026-11-08.md).

Open flight-map nodes can include stable public map-relative positions.
Saved route legs retain each map's coordinate frame through `map_id` and
optional origin/destination positions. Unknown positions remain absent;
coordinates do not alter fares, reachability or route choice and do not
establish distance, travel time or movement instructions.
[Offline evidence](../../docs/flight-map-positions-verification-2026-11-08.md).

Trainer observations can include the native profession-acquisition flag and
stable derived primary-slot fullness. Requirements review reports known
slot blockers independently of budgets, preserving unknowns and leaving
eligibility unverified. Lesson data must agree across two reads. No filters,
profession changes or training actions are performed.
[Offline evidence](../../docs/trainer-profession-slots-verification-2026-11-08.md).

`list_snapshots.spellbook_spell` searches saved visible player spell rows by
base/reported ID and optional passive/off-spec/reported-learn-level metadata.
Predicates apply to one row; unknowns do not count as false or zero. Output
is bounded and does not establish current usability or complete learning.
[Offline evidence](../../docs/spellbook-search-verification-2026-11-08.md).

Spellbook observations can include a readable stable `level_learned` from
the public spell API, including level-linked adjustments. Existing inspection
and optional spellbook comparison expose it without an eligibility or new-
learning inference. Unknowns and duplicate-value ambiguity remain explicit.
[Offline evidence](../../docs/spell-learned-level-verification-2026-11-08.md).

Fixed-reagent recipe reviews can join their reagent IDs to known pending
item attachments in the selected inventory capture. This is a separate
observation with source time and partial coverage, never a stock credit or
retrieval action. Batch observations reuse the same inventory source without
allocating mail. [Offline evidence](../../docs/recipe-pending-mail-verification-2026-11-08.md).

`compare_snapshots.mail_attachment_changes` compares known loaded pending
attachment quantities/slot counts by kind and ID, retaining unknown slots
and loaded/total message counts. No correspondence or mail actions are
added, and no delivery, retrieval or inventory transfer is inferred.
[Offline evidence](../../docs/mail-comparison-verification-2026-11-08.md).

`list_snapshots.active_pet` searches saved pet presence, family, inclusive
level ranges and signed training-point balance ranges. Unknown data cannot
match a threshold, even zero. Newest-capture/date/character/pagination filters
remain in force; no current-character or live-pet claim is made.
[Offline evidence](../../docs/active-pet-search-verification-2026-11-08.md).

Saved active-pet care can include readable public family ID and level,
checked for consistency during capture. Summary and comparison expose these
as observations, with unknowns preserved and no individual-pet continuity
or level-gain inference. No pet names, GUIDs or pet actions are added.
[Offline evidence](../../docs/active-pet-details-verification-2026-11-08.md).

Active-pet training-point totals and used counters can also be captured
without opening a trainer. Summary preserves their signed difference;
comparisons expose raw before/after observations without a same-pet or
points-earned/spent inference. Unknown data remains absent. No training
action or eligibility claim is added. [Offline evidence](../../docs/pet-training-points-verification-2026-11-08.md).

Character-bound Hunter simulations that model a pet require captured present
pet level 60, matching the pinned engine. Other/unknown pet levels remain
plan-only; no-pet and zero-uptime templates are unaffected. Family, happiness
and training compatibility remain unverified. [Offline evidence](../../docs/simulation-pet-level-verification-2026-11-08.md).

`compare_snapshots.pet_view_changes` compares saved filtered pet-journal
views: shared species count changes and one-sided observed species IDs.
It does not infer collection gains/losses or equal journal filters; missing
views remain null. [Offline evidence](../../docs/pet-view-comparison-verification-2026-11-08.md).

Basic profession observations retain sparse slots and reject unreadable,
malformed or changing public rows as an unavailable section, preserving
other valid export sections. This does not establish full profession or
simulator-template compatibility. See [offline evidence](../../docs/profession-capture-verification-2026-11-08.md).

Equipment reviews now carry saved professions to the local simulator tools.
They also project saved class/race to the direct simulation adapter, which
now shares the ranker's check against known character identity mismatches.
Unknown fields remain unverified and no active-character claim is made.
See [offline identity evidence](../../docs/simulation-character-identity-verification-2026-11-08.md).

Known template profession identity mismatches block simulation and remain
reviewable in plan-only mode. Missing/unmapped observations remain unverified
unless a known captured profession already contradicts the template;
profession ranks, recipes and specialisations are outside this identity check.
See [integration evidence](../../docs/simulation-professions-verification-2026-11-08.md).

The hosted reader and importer support the bank, recipe, reputation, currency, tracked-achievement and quest observations below. See the [hosted deployment evidence](../../docs/container.md#forever-snapshot-rollout-7-november-2026) and the [Windows companion and package checkpoint](README.md#capture-only-installation-checkpoint). Live addon capture remains pending.

Saved searches accept `captured_from` and `captured_through` as inclusive
RFC 3339 timestamps. UTC and explicit timezone offsets are supported.
These bounds combine with character and feature filters before pagination.
They use the recorded capture time, so a recent match still requires
separate evidence of current game state.
Set `latest_per_character: true` to evaluate feature filters only against
the newest valid capture per character, realm and client variant inside
those time bounds. An older matching capture is excluded even if the
newest capture lacks the requested observation. Equal timestamps prefer
the lowest snapshot ID. This option uses the requested client family and
does not identify the character currently in play.

For a saved currency balance, pass `currency_balance` with
`currency_id` and `minimum_quantity` to `list_snapshots` using the
Forever family. The result includes only that currency's quantity,
account scope and inactive flag. A zero minimum requires an observed
balance. Combine with `latest_per_character` to evaluate each
character's newest capture before matching the balance.
Add `minimum_weekly_remaining` or `minimum_maximum_remaining` inside
`currency_balance` to require room below an observed cap. These limits
combine with the held-quantity threshold. Total caps use the captured
held quantity or total-earned basis. Missing limits or counters do not
match, even at a zero threshold. The result reports the cap headroom used
for the filter, which does not establish whether more currency can be earned.

Use `reputation_rank` with `faction_id` and `minimum_reaction`
(1–8) to search standard reputation ranks in saved Forever captures.
The threshold is inclusive. Missing factions and specialised progression
types are excluded. Results include the matched rank and account-scope
flag. This can be combined with time bounds and newest-capture selection.
For renown, use `renown` with `faction_id`, `minimum_level` and an
optional `capped` flag. It requires an observed renown level for that
faction. Standard reputation and unavailable renown detail are excluded.
The result includes the recorded level, capped flag and account scope.
Use `paragon` with `faction_id` and optional `reward_pending` and
`level_too_low` flags to find recorded paragon rewards. Both flags
must match the same faction. Pending status alone does not establish
eligibility to collect the reward. Missing paragon detail is excluded.
Add `reward_quest_readiness` to match the linked reward quest in the
same capture. It accepts `ready`, `not_ready`, `failed` or `unknown`.
The quest must be observed even for an unknown-readiness match.

Use `recipe_cooldown_state` with `available`, `waiting` or `unknown`
to find saved selected recipes by their recorded cooldown state. It combines
with `recipe_id`, `recipe_output_item_id`, `recipe_profession_id`, character
filters and newest-capture selection. Available charges take precedence over
a running recharge timer. Zero charges mean waiting. Without a charge count,
zero remaining seconds means available and a positive timer means waiting.
A selected recipe without either observation is unknown; an absent selected
recipe does not match. Recipe reviews return the same classification in
`captured_cooldown_state`. This describes the recipe capture, not current time
or full craftability. Learning, materials, tools and other conditions remain
separate checks, and no game action is performed.

For a multi-recipe `review_forever_recipe` request, set
`suggest_dependency_order: true` to add `dependency_order`. Known outputs are
matched to fixed input item IDs, then producers are placed before consumers.
Independent recipes retain their input-order priority. All recipe numbers in
the suggestion and dependency edges refer to the submitted batch, starting at
one; checkpoint `suggested_step` identifies the position in the new order.

The suggestion recalculates starting materials, coverage and preparation
checkpoints using captured minimum yields. It does not change the original
ordered plan or the requested craft counts. Insufficient output still requires
external starting stock. Missing fixed requirements or output ranges produce
`incomplete_observations`; a cycle produces `cyclic_dependencies`, with graph-
blocked recipe numbers and no suggested order. A cycle does not prove the batch
cannot be crafted using starting stock. These are conditional material estimates,
not a cheapest-order optimisation, inventory-capacity check or permission to
craft. Recipes with unobserved outputs, including enchant-only captures, keep
their original plan and receive no automatic dependency order.

When both `suggest_dependency_order` and `merchant_reagent_plan` are true,
`dependency_order.merchant_reagent_plan` quotes the suggested order's remaining
starting-material shortfalls. Intermediates supplied by the batch are excluded;
insufficient intermediate output still leaves a purchase requirement. The
top-level merchant plan continues to describe the original order. The nested
plan uses the same captured merchant offers and inventory wallet, and its first
uncovered-recipe budget retains the original recipe number and suggested step.
Cycles, incomplete dependency data or unavailable merchant stock produce a null
nested plan. These are saved quotes and conditional estimates, not purchases or
proof that the batch can be crafted.

Set `selected_recipe_changes: true` in `compare_snapshots` for a compact
before/after view of selected-recipe learning, cooldown, fixed reagent and
output observations. Recipe names are excluded so localisation-only changes
do not appear as recipe progression. `changed_observations` and
`observed_charge_delta` are derived only when both recipe IDs agree. A
different recipe gives `same_recipe: false`; a missing observation gives
`same_recipe: null`. Neither case produces field changes or a charge delta.
Missing charge counts stay unknown. Timer and charge differences are recorded
observations, not evidence of crafting, recharge completion or elapsed-time
simulation. Existing same-character and same-client/build checks still apply.

Use `tracked_achievement` with `achievement_id` and optional
`completed` or `earned_by_character` flags to search saved Forever
tracked achievements. Flags must match the same recorded achievement.
The result preserves account scope and character credit separately.
An achievement absent from the tracked list does not match either
completion state.
Use `achievement_status_changes: true` in `compare_snapshots` to
compare all shared tracked IDs. Completion and character-credit changes
are compared only while the recorded account scope agrees. Scope changes
and IDs present on just one side are listed separately. A missing tracked
list makes the comparison unknown.

| Area | MCP role and current state | Evidence needed next |
| --- | --- | --- |
| Character selection | Exact name and realm filter on saved snapshots. Explicit target takes precedence. | Qualify a supported, fresh active-character signal. Until then a saved capture remains historical. |
| Owned gear | Observe equipment, bags and visible character/account bank in one export. Local MCP joins observed item locations to pinned base-item records. | Qualify real storage visibility, exact modifiers, equip rules and coverage gaps. Account bank is shared within the owner's account. |
| Equipment sets and item management | Compare manually applied changes. Captured item-link required levels can reject known level shortfalls in changed selections and ranking. | Verify Forever equip legality beyond level evidence, unique limits, enchants, suffixes, set bonuses and hand combinations. Apply codes, automated moves and inventory transactions stay blocked. |
| Talents, spellbook and stats | Export available character observations, then evaluate builds offline. | Qualify Forever talent ranks, learned spells, weapon skills, buffs used in sims and supported class/spec rotations. |
| Simulation and best-in-slot | Local catalogue, encounters, staged ranking and offline qualification. Character-bound simulations require captured level 60, matching the pinned fixed-level engine; plan-only review remains available for other/unknown levels. | Qualify other levels/engines, captured builds and variants, and complete legal owned sets. Track phase/acquisition evidence independently. |
| Quest and world navigation | Saved quest log with objective types and counts, failure and turn-in readiness where available. Local simulator reference zones/NPCs. | Qualify native capture and a wider, licensed, build-identified world dataset for offline quest and travel planning. Leave movement and quest actions to the player. |
| Professions and crafting | Saved profession ranks, selected recipe and tracked recipes. Capture/search local filtered recipe lists and compare shared learning observations when profession IDs agree. The MCP compares selected fixed reagent quantities with observed bags and banks; the catalogue can find item/enchant references by recipe spell ID. | Qualify native capture and complete learned recipe enumeration, optional/alternative/variable reagents, tools and stations. Filtered-list absence is unknown. Crafting stays with the player. |
| Reputation | Saved visible faction rows with stable faction IDs, raw standings and thresholds, account scope and progression kind where known. | Qualify native capture and specialised friendship, renown and paragon progress. Collapsed or filtered factions may be absent. |
| Currencies | Saved visible balances keyed by stable currency ID, with account scope, inactive status, cap fields and earnings. | Qualify native capture and separately verify wallet gold and hidden-currency coverage. |
| Achievements | Aggregate achievement points and saved progress for player-tracked achievements, including account-wide and earned-by-character flags. | Qualify native capture and assess broader catalogue coverage and progress with stable criterion identities. |
| Mount collection | Bounded read-only journal capture of collected mount IDs, names, spell IDs, favourite flags and recorded usability. Saved ID lookup, collected counts and compatible-capture comparisons are supported. | Qualify native journal completeness and account/character semantics. Summoning, favourites and filter changes are not performed. |
| Toy journal | Read the filtered result list while the toy panel is visible, retaining item IDs, names and collected/favourite flags. Scope is explicitly partial. | Qualify native capture; do not infer full account coverage. Opening the panel, filters, favourites and toy use remain with the player. |
| Pet journal | Aggregate visible filtered journal rows by species ID and public species name. Search saved observations by species and owned/unowned row thresholds, omitting individual GUIDs and custom names. | Qualify native capture and filter/account semantics. No full collection inference, journal changes, summoning, renaming, caging, release or battles. |
| Economy and trading | Evaluate public price, recipe or loot data if its terms and provenance permit local use. | Select a permitted feed. Purchases, bids, sales, mail and trades remain player actions. |
| Macros, bindings and action bars | Read saved observations where the client supports them. | Qualify visibility and report configuration gaps. No MCP-generated Apply codes or macro execution. |
| Dungeons, raids, combat and PvP | Offline encounter, roster or preparation analysis from appropriate owner-provided data. | No live combat coaching from restricted values, input automation, targeting, queueing or play. |
| Saved instances | Passive updated-cache capture and search of saved dungeon/raid names, difficulties, reset timers and lock/extension flags, with update age. Compare public descriptions across captures, marking duplicates ambiguous. | Qualify native events and returns. Public descriptions cannot establish lockout identity. No refresh requests, unique lockout IDs, acceptance or extension actions. |
| Saved world bosses | Separate passive cache capture of names and reset timers after instance-info updates, with cache age and summary count. Search saved characters by exact name and recorded timer/cache-age limits. | Qualify native events and returns. Missing entries do not prove loot eligibility. No refresh requests or identifiers are exported. |
| Mail attachments | Observe loaded pending item/currency attachment IDs and counts in a visible idle inbox. Search saved characters by kind, ID and summed known quantity, retaining partial coverage. Excludes private correspondence and spendable inventory. | Qualify native loading, idle flags and item/currency returns. Missing/unresolved/unloaded data is unknown. No headers, bodies, senders, COD, refresh, opening, retrieval, deletion or sending. |
| Social, guild and account administration | Public reference or owner-approved offline records only. | Chat, invitations, account changes and distribution of private data stay outside MCP scope. |

New MCP routes are denied until reviewed for data source, owner scope, outputs and side effects. The tool cannot drive game input, bypass protected values, read process memory, intercept traffic or invoke a game API. See [enforced policy](../../docs/operation-policy.md) and Blizzard's [addon policy](https://us.forums.blizzard.com/en/wow/t/ui-add-on-development-policy/24534).

`skill_lines` records the game-exposed skill names, base ranks, rank caps
and category IDs when available. Rows use stable skill-line IDs. The
export leaves collapsed headers as they are, so missing rows do not
establish that a skill is unlearned. Readable `modifier` and
`temporary_points` values are saved separately as reported by the skill
API. They are not added together or used to change base-rank comparisons.
Update the importer and reader before syncing new captures. [Forever skill API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/SkillInfoDocumentation.lua)

`compare_snapshots` reports signed rank and cap changes under
`skill_line_progress` for skills seen in both captures. Skills seen on
only one side are counted separately because collapsed headers can hide
rows. A missing section leaves the progress comparison unknown.

`review_forever_owned_gear` links supported catalogue weapons to their
captured `weapon_skill` row, with base points to the cap. One-handed
and two-handed axes, maces and swords use separate skill IDs. Missing
observations remain unknown. The link does not establish equip eligibility
or change simulation inputs. [Forever weapon skill IDs](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_UIPanels_Game/Camelot/SkillsFrame.lua) · [Pinned simulator weapon types](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/proto/common.proto)

`review_equipment_selection` also reports `checks.catalogue_projected_weapon_skills`
for mapped main-hand, off-hand and ranged weapons in the draft, including
weapons carried forward from the baseline. A projected off-hand clear removes
that slot from the list. The list describes the draft even when other checks
find a conflict.

The local gear ranker includes the same observations as
`projected_weapon_skills` on per-encounter and weighted results. These are
captured character data for interpreting a result. They do not affect the
score.

Owned-gear searches report `catalogue_item_requirement_conflict` for known
catalogue restrictions that conflict with captured character data, even
without a target slot. Null means no conflict was detected from the
available evidence. The separate draft conflict still applies to the
proposed placement.
Enable `exclude_observed_requirement_conflicts` to omit known item conflicts
before paging. Items with unknown eligibility remain in the results.
When the captured level is below the catalogue requirement, both conflict
views include `level_shortfall` with the captured and required levels and
the difference.

Use `minimum_required_level` or `maximum_required_level` to narrow owned-gear
results by the catalogue's recorded required character level before paging.
These inclusive bounds exclude items with no recorded requirement.
Storage coverage and set totals describe the full capture.
Use `storage` to limit matching locations to `equipment`, `bags`,
`character_bank` or `account_bank`. Check storage coverage to distinguish
an empty result from a bank that was unavailable at capture.

Use `observed_enchant_effect_id` to find captured items whose links encode
that exact nonzero effect ID. The filter works before paging and includes
items absent from the catalogue when the link agrees with the captured
base ID. Missing or unreadable modifier data cannot match.
Set `simulator_input_resolved` to `false` in `review_forever_owned_gear` to find
locations that need more evidence, or to `true` to find locations with a usable
simulator item input. The filter applies before pagination and combines with
existing slot, storage and item filters. `inventory_evidence` still describes
the whole capture. Item input availability is separate from equip eligibility.

Use `observed_random_suffix_id` to match the exact signed suffix from the
same link. Zero matches an explicit zero. Combine it with the enchant
filter to find variants that match both fields.

`pet_care` records whether an active pet was present and, when readable,
its game-reported happiness value, localized loyalty label and accepted
food types, plus optional public family ID and level. An empty food list means the game reported none. The capture
omits pet identity and leaves feeding to the player. Update the importer
and reader before syncing new captures. [Forever pet API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/PetInfoDocumentation.lua)

`pet_care.carried_food` groups accepted item IDs and quantities from the
same export's carried bags. An empty map means none of the checked items
were accepted. Unavailable bags or any unreadable classification leave
the map unknown. The importer checks each reported quantity against the
saved bag stacks. The classification does not estimate feeding effects.
In `character_summary`, `pet_food_locations` reports the matching stack
count and up to 50 saved bag slots. A missing classification stays unknown,
while an observed empty map gives zero matching stacks.

Use `list_snapshots` with `family: "forever"` and a `pet_food`
filter containing `minimum_quantity` and optionally `item_id`. The
filter counts accepted units across all classified food or for the
specified item. A zero minimum still requires a captured classification.
Results include the accepted quantity and omit bag locations.

To find vendor sources for those foods, inspect `merchant_stock` with
`merchant_pet_food_snapshot_id` set to a compatible Forever capture for
the same character. This returns saved offers for item IDs previously
classified as accepted carried food. An unavailable classification gives
an unknown match count, and an observed empty classification matches
no offers. `merchant_target_quantity` can estimate bundles per offer.
For one specific food, combine `merchant_item_id` with the pet food capture
and target quantity. The quote can mix whole bundles of that item across
the saved offers. The selected item must appear in the saved accepted-food
classification.
Alternatively, set `merchant_pet_food_target_carried_quantity` instead of
`merchant_target_quantity`. The MCP subtracts accepted units recorded for
that exact item in the food capture and quotes only a positive shortfall
using the saved merchant offers. A missing accepted-item row leaves the
shortfall unknown. An already covered target needs no purchase quote.
Use this for historical sourcing. Pet acceptance can change between captures.

Use the same `merchant_pet_food_snapshot_id` in `list_snapshots` to
search all compatible saved merchant visits. It combines with an exact
item ID and with the stock, price and affordability
filters. A missing food classification returns an error. An observed
empty classification returns no matches.
With an exact item ID and a target quantity, use `merchant_combine_bundles`
to find qualifying whole-bundle mixes within each saved visit. Currency
selectors are separate from pet food searches.
In `list_snapshots`, a `merchant_pet_food_target_carried_quantity` can
replace the purchase quantity too. Matching visits report the recorded
carried units and positive shortfall under `merchant_match.pet_food_restock`.
An already covered target or an item missing from the saved classification
returns no visits.

Add `merchant_target_quantity` (1–1,000 units) to a merchant listing
with a pet food, item or currency selector. Each matching offer must
show unlimited stock and a copper-only price. The listing reports the
cheapest single-offer bundle plan per visit as `lowest_cost_quantity_plan`.
Equal costs prefer the lower merchant slot. Affordability, when requested,
uses the full bundle cost. The price ceiling still applies per bundle.
This estimate uses captured offers and keeps different items separate.
Set `merchant_cheapest_first: true` with a target quantity to rank visits
by that total before pagination. Equal totals use the usual newest-first
order, then the snapshot ID.
Set `merchant_max_total_copper` with a target quantity to cap the whole
purchase cost. The limit includes bundle rounding and combines with
per-bundle price and captured-wallet filters. A zero limit matches free
offers.

`compare_snapshots` reports `pet_care_change` with the observed pet
presence, happiness and loyalty on each side, plus family ID and level when known. When both captures have
food classifications, it compares accepted carried quantities by item ID.
Each capture reports its own total accepted carried units and its signed
training-point balance when recorded. An observed empty food list totals
zero. These are independent observations of the captured pets.
An absent item in an observed classification contributes zero accepted
units. A missing classification leaves the comparison unknown. The
captures can describe different pets, so changes can reflect different
food acceptance as well as changes in bag stock.

`trainer_services` saves the open general, profession or pet trainer's API
list under the player's current trainer filters. It records lesson names,
available, unavailable or known status, and readable rank labels,
level requirements and prices. Pet lessons record `price_training_points`
instead of copper, and the capture includes the active pet's available
points when readable. Training and filter changes stay with the player. Update the importer
and reader before syncing new captures. [Forever trainer UI](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.lua)

Lessons that have yet to be learned can include the required skill rank
and prerequisite ability names, along with the game's recorded met flags.
An empty ability list means the trainer reported zero prerequisites of
that type. Missing detail remains unknown.

Set `trainer_requirements_review: true` with `section: "trainer_services"`
to see the observed level shortfall, unmet skill flag and unmet ability
names for each retained unavailable lesson. Level comes from the same
capture. Pet lessons use the active pet level recorded in the trainer
section when available. Missing requirement data remains unknown. The trainer's saved
availability remains authoritative for that observation.
For character lessons, `captured_skill_rank_comparison` compares the
required rank with a uniquely matching exact skill name in the same
capture. It uses the recorded rank field without applying modifiers.
Missing or ambiguous matches and pet lessons leave this comparison unknown.
`unmet_ability_names_observed_in_spellbook` lists exact name matches
between unmet prerequisites and captured character spellbook rows.
A match preserves the trainer's unmet flag. Missing ability data,
missing spellbook data and pet lessons leave the cross-reference unknown.
The review also lists available slots from the saved panel whose lesson names
exactly match an unmet prerequisite. These are name matches within the
captured panel and do not establish that training a lesson clears the
prerequisite. Filters limit the lessons being reviewed and leave the
saved prerequisite candidates visible in the review.

Set `trainer_wallet_projection: true` with `section: "trainer_services"`
in `inspect_snapshot` to budget lessons the trainer marked available.
The review reports the known-price subtotal. It reports a full total
only when all such prices were captured, and a copper shortfall only
when the opt-in wallet was captured too. It does not establish that the
character could learn all listed lessons together. Pet trainer captures
return no copper projection.

Set `trainer_points_projection: true` for pet trainer captures. It budgets
matching lessons marked available, with a training-point shortfall when
all their costs and the active pet's balance were observed. The projection
uses the saved pet and leaves training to the player.
The projection also shows whether the trainer and active-pet observations
agree on level and spendable training points. For nonnegative raw pet
counters, the comparison floors total minus used at zero to match the
trainer reading. The signed balance remains visible separately. Missing
or incompatible counters leave agreement unknown. Lesson budgets still
use the trainer observation, and pet identity remains unverified.
With `trainer_requirements_review`, pet lessons also report a separate
level shortfall against the active-pet observation when available. The
trainer-panel shortfall retains its own level basis, including when the
two readings disagree.
Both projections include `individual_lesson_affordability` keyed by
captured slot. Each available lesson is compared independently with the
recorded balance, in copper or training points respectively. Missing
prices or balances leave the comparison unknown.
`cheapest_affordable_known_lessons` shows how many available lessons fit
together by taking the lowest observed prices first. It uses the same
currency or training points as the enclosing projection and excludes
unknown prices. This is a budget estimate without modelling prerequisites
or changes to the trainer's offers.

With the same section, `trainer_service_name` matches a case-insensitive
substring and `trainer_service_status` matches `available`, `unavailable`
or `used`. The filters combine, keep the original lesson slots and apply
before the optional wallet calculation. Zero matches and unavailable
trainer data are reported separately.
Use `trainer_service_name` in `list_snapshots` with `family: "forever"`
to find saved visits by a case-insensitive lesson name substring. Matching
captures include the trainer kind, lesson count and player map if known.
Results describe the trainer list and filters recorded at capture time.
When both captures contain the same trainer kind, `compare_snapshots`
reports status changes for shared lessons with a unique exact name and
rank label in each panel. It ignores row order. Missing panels or different
kinds leave the comparison unknown. The captures do not identify the
trainer, so a status change describes the two saved observations.
The same comparison reports price changes separately, in copper or pet
training points. A quote appearing or disappearing has an unknown
delta, while a recorded free lesson retains a zero price.
It also compares observed level, skill and ability prerequisites for
matched lessons. Reordering ability rows alone produces no change.
An unavailable requirement observation remains distinct from an
observed empty list.
Lessons seen only before or only after are listed separately by exact
name and rank label. Repeated matching rows on one side are marked
ambiguous. A one-sided observation can reflect different trainer panels
or filters.
`trainer_service_status` works alone or alongside the name filter.
When both are supplied, the same captured lesson must match both.
Use one price ceiling, `trainer_max_price_copper` for ordinary lessons
or `trainer_max_price_points` for pet training. A zero ceiling matches
free lessons in that price type. Unknown prices are excluded. Any name
or status filter must match the same lesson as the price ceiling.
Set `trainer_affordable_only` to compare the same lesson with the
captured wallet or pet points from that visit. Both price and balance
must be known. Use status `available` as well to require recorded lesson
availability. Search results omit the balance.
Use `trainer_service_slots` to select up to 32 exact slots from the same
capture. Slots missing from that capture are reported separately from rows
excluded by the name or status filters.
With a wallet or points projection, the supplied slot order also gets
an `ordered_lesson_budget`. It counts the consecutive available lessons
with known prices that fit the captured balance and identifies the
first uncovered slot. Slots excluded by filters stop the count.
A missing balance leaves the ordered budget unknown.
For insufficient balance, the first uncovered slot reports
`additional_balance_needed` in the projection's currency after the
earlier lessons' recorded costs.
With `trainer_requirements_review: true`, the first uncovered lesson also
includes the requirement review for that slot when it was recorded as
unavailable.

`instance_context` records the current instance type, public name and
difficulty ID at export. It is also visible in `character_summary`.
Outside an instance the observation contains only `kind: "none"`.
Unrecognised or unavailable context remains unknown. Group membership,
lockout identifiers and active combat state are outside this capture. Update
the importer and reader before syncing new captures. [Forever instance API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/InstanceDocumentation.lua)

In `list_snapshots`, set `family: "forever"` and supply any of
`instance_kind`, `instance_name`, `instance_difficulty_id` or
`instance_map_id`. The optional map ID identifies the instance map across
client languages and is separate from the player UI map and lockout IDs. Name
matching is exact and case-insensitive. Filters combine with the saved
character and recipe selectors before pagination. `instance_kind: "none"`
finds captures taken outside an instance. Captures without an observation
are excluded from filtered results.

When the game's instance-lock warning is already visible in a dungeon
or raid, `instance_context.lock_warning` can include its remaining seconds
and aggregate encounter counts. This is the warning at capture time.
This warning capture does not track saved raid lockouts or accept the prompt.
The separate `saved_instances` section below records updated cache metadata.
The snapshot listing excludes the warning details. [Forever lock warning source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_StaticPopup_Game/GameDialogDefs.lua)

When the warning reports completed encounters, it can also include the
tooltip's boss names and defeat flags. The list is saved only when all
rows are readable and agree with the aggregate counts.
Use `instance_lock_encounter_review: true` on `instance_context` to
separate completed and pending encounters in saved slot order and show
the projected warning expiry. If only counts were captured, encounter
names remain unknown. The review leaves current warning and saved
lockout identity unverified.
Use `list_snapshots.instance_lock_warning` to search saved Forever
warnings by minimum completed or remaining encounters, maximum recorded
warning seconds, or presence of encounter details. Bounds are inclusive.
An empty filter matches any recorded warning. Results include counts and
timer only, alongside instance kind and difficulty. Encounter names stay
in the individual snapshot review.
Set `encounter_name` for an exact case-insensitive name match. Optional
`encounter_defeated` must match the same saved row. Counts-only warnings
cannot satisfy a named search. Results show how many rows matched and
leave encounter names in the individual review.
Use `compare_snapshots.instance_lock_warning_changes` to compare two
warnings with the same saved instance description and encounter total.
Per-slot defeat flags are compared only when both detailed lists have
the same names at the same slots. This leaves encounter completion events
and lockout identity unverified.

The `experience` section records the game's `resting` flag alongside
XP progress when available. The flag describes the character at capture
time. The separately observed `rested_xp` is the saved XP bonus pool.
With `experience: true`, `compare_snapshots` reports the signed change
in that pool and whether the resting flag, XP-disabled flag or progress
status changed. Each comparison requires that observation on both sides.
`single_level_up_xp_delta` adds the XP remaining in the earlier level
to the XP recorded in the next level. It requires a positive starting
level, exactly one level of increase and observed progress counters in both captures. Capped or
skipped levels leave that estimate unknown.

Use `list_snapshots` with `family: "forever"` and `minimum_rested_xp`
to find captures with an observed bonus pool at or above the threshold.
`xp_gain_enabled_only` requires an explicit unlocked XP flag.
Both filters exclude capped captures and can be combined. A zero
threshold still requires an observed pool. Results describe saved XP.
Update the importer and reader before syncing new captures. [Player frame source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_UnitFrame/Mainline/PlayerFrame.lua)

`character_summary` reports the saved `new_mail_indicator` when the
client exposes it. This is the game's new-mail flag at export time. A
clear indicator can coexist with mail in the inbox. The export reads
only the indicator and leaves senders, messages and attachments private.
Update the importer and reader before syncing new captures. [Minimap mail source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_Minimap/Mainline/Minimap.lua)

`hearthstone_bind_name` in `stats` and `character_summary` is the
client's localized bind-location name at export time. It carries no
coordinates. When the game reports a Hearthstone item, the same views
show `hearthstone_item_id` and any readable, enabled item cooldown in
`hearthstone_cooldown_seconds`, rounded up at capture time. A zero means
no remaining timer was observed. Missing data leaves availability unknown,
and a timer alone does not establish usability. Update the
importer and reader before syncing new captures. [Forever bind-location usage](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_StaticPopup_Game/GameDialogDefs.lua)

`compare_snapshots` reports `travel_context_change` when either Forever
capture includes a map ID, bind name or Hearthstone item ID. It compares
map IDs and captured bind names when both values are available. A
cooldown delta requires the same observed item ID and a timer in both
captures. It describes recorded timers rather than current usability.

Use `list_snapshots` with `family: "forever"` and `player_map_id`
to find saves from an exact player map. `player_parent_map_id` matches
only the recorded immediate parent. Both can be supplied to match the
same saved location. The filters run before pagination and return
`player_map_match` with the recorded map ID, name and parent ID.

`bag_capacity` records total and free slots per carried bag, including the
backpack, with the game-reported bag family when available.
`character_summary` reports combined capacity, free slots and free slots
in known general-purpose bags. Unknown family slots are excluded from
the general-purpose count. Specialty slots can have item restrictions.
The bag summary groups free slots by exact recorded mask in
`free_slots_by_family_mask` and reports `unknown_family_free_slots`
separately. Composite masks are kept together so their slots count once.
Update the importer and reader before syncing this section. [Forever container API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ContainerDocumentation.lua)

`compare_snapshots` reports both bag-space observations and signed
changes in total and free slots. If one capture lacks capacity data, its
side and both changes are unknown. General-purpose counts are shown per
capture because a change in bag-family visibility can change those counts.
The comparison lists changed exact masks in `family_space_changes`.
A mask seen in only one capture has a null count on the other side and
no delta. `unknown_family_free_slots_delta` tracks changes in the
unclassified count, which can also reflect changes in visibility.
The carried-item review flags whether free slots were captured and leaves
stack compatibility unknown.

Use `list_snapshots` with `family: "forever"` and
`minimum_free_bag_slots` for total free carried slots, or
`minimum_general_free_bag_slots` for slots in known general-purpose
bags. The limits are inclusive and can be combined. Missing bag
capacity is excluded. Results show the same counts as the character
summary. Total free slots include specialty and unknown bag families.

For a particular recorded family, use `bag_family_space` with
`family_mask` and `minimum_free_slots`. The mask must match exactly.
A zero minimum still requires a bag with that recorded mask. This
matches saved capacity observations and leaves item eligibility unknown.

Inspect the `durability_alerts` section for saved equipment slots flagged
`low` or `broken` by the built-in durability display. An empty map means
the checked slots raised no alert. Exact durability, repair prices and
repairs are outside this capture. Disabled repairs or an unavailable
alert API leave the section unavailable. Update the importer and reader
before syncing this section. [Forever durability display](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_DurabilityFrame/DurabilityFrame.lua)

Export while a repair-capable merchant's goods tab is open to save its
repair-all copper quote in `repair_quote`. A recorded zero is a quote.
A closed merchant, another tab or an unavailable quote leaves the
section unavailable. The export records the price and leaves the repair
to the player. Update the importer and reader before syncing this
section. [Forever merchant source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_UIPanels_Game/Vanilla/MerchantFrame.lua)

`compare_snapshots` reports changed durability alert slots when both
captures contain alert maps. A cleared alert can also reflect a gear swap.
Repair quote deltas require two observed quotes, and zero remains a quote.

`merchant_stock` captures the open merchant's goods tab across its pages.
Each offer identifies the item or currency and records the copper price
per bundle, bundle size and game-reported stock. The stock value `-1`
means unlimited and `0` means sold out. An extended-cost flag means
the copper price is only part of the offer. The snapshot omits buyback,
merchant identity and player eligibility. Empty, incomplete or unavailable
stock leaves the section unavailable. Update the importer and reader
before syncing new captures. [Forever merchant offer API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/MerchantFrameDocumentation.lua)

Extended offers can include `cost_components` per bundle. Item tokens
carry their item ID and currencies carry their localized name, as supplied
by the merchant cost API. Missing components leave the extended cost
unknown. Currency names are recorded without inferring balance matches.
[Forever merchant cost display](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_UIPanels_Game/Mainline/MerchantFrame.lua)

With `section: "merchant_stock"`, use `merchant_cart` with up to 16
distinct `{slot, bundles}` entries to budget their copper together.
Missing offers leave the complete copper total unknown. Known item-token
costs are combined by item ID and compared with captured bags. Missing
offers or unobserved extended costs make the item-token tally incomplete.
Named currency costs remain per line because their names are not stable IDs.
The cart's `resource_budget_status` distinguishes a known shortfall,
covered costs and unknown resources. It describes recorded costs and
balances only. Stock and purchase eligibility are separate.
Offer budgets report the raw captured stock and distinguish sold-out,
unlimited and limited offers. Carts list captured sold-out slots. A
positive limited count does not establish how many requested bundles fit.
Set `merchant_cart_include_repair` to include the same capture's repair
quote in the resource budget. The repair reservation shows the combined
copper and shortfall. Missing repair quotes leave full coverage unknown.
Set `merchant_cart_estimate_bag_space` to estimate the shared bag space
for the requested bundles of observed item offers, grouping the same item
across slots. Currency offers are excluded. Sold-out item offers still
contribute to the hypothetical space estimate and retain their stock flag.
Unobserved slots leave whole-cart fit unknown. The estimate uses bags
before purchase and excludes slots freed by spending item tokens.

Set `merchant_offer_slot` to select an
offer and optionally `merchant_bundles` for the requested bundle count.
The review scales the copper and token costs, compares them with the
opt-in wallet and carried bags from the same capture, and lists named
currency requirements separately. Missing stock, balances or components
remain unknown. The review does not check stock sufficiency, spending
eligibility or current purchase availability.

Each offer budget also reports `captured_resource_capacity`. It shows the
maximum bundles covered by the saved wallet and bags when the costs are
known. A known zero capacity is conclusive for that capture even if
another cost is unknown. An offer with no recorded resource cost has no
resource-based cap. Cart lines show capacity for each offer on its own.
Use the combined cart budget to account for resources shared by lines.

With `section: "merchant_stock"`, use `merchant_item_id` or
`merchant_currency_id` to find offers selling that exact ID. Matching
offers retain their original slots for later budget review. A zero match
count means the stock was captured and had no matching offers. Unknown
stock gives an unknown match count.

`compare_snapshots.merchant_offer_changes` compares copper prices, raw
stock and extended-cost flags for offers with the same product kind,
ID and bundle size. A match must be unique in both captures. The result
compares the saved panels without assuming merchant identity. Missing
stock on either side leaves it unknown. When extended cost observations
differ, `cost_component_change` shows both lists regardless of row order.
An unknown extended cost is null. A copper-only offer has an empty list.
Currency names are compared as captured labels.
`merchant_offer_coverage` separately lists product and bundle keys seen
on only one side. Duplicate rows for a one-sided key are marked ambiguous.
One-sided offers can reflect different panels. Missing stock leaves the
coverage comparison unknown.

Add `merchant_target_quantity` from 1 to 1000 to calculate whole bundles,
units received, surplus and copper for each matching offer. Extended
costs and recorded stock are flagged separately. Use the returned slot
and bundle count in a merchant offer review for the full resource budget.

Set `merchant_reagent_plan: true` in `review_forever_recipe` to match
reagent shortfalls against offers in the same capture as the inventory.
For batches this uses starting materials in the supplied recipe order.
Missing inventory coverage can overstate what needs buying. When no
storage was observed, matching slots can still be listed but their
quantities stay unknown. The plan estimates bundles and copper, with
stock and extended costs reported separately.

For each shortfall, `lowest_unlimited_copper_only_offer` selects the
least copper among captured unlimited offers with no extended cost.
Equal prices use the lowest slot. `unlimited_copper_only_total` sums
those choices when every shortfall is known and covered by such an
offer. Zero is a valid total when no reagents need buying. Limited
offers and offers with extended costs remain visible for manual review.
The plan also reports `unlimited_copper_only_affordable` and
`unlimited_copper_only_shortfall` when the total and the opt-in wallet
from the inventory capture are known. A separate merchant visit supplies
prices only.

Pass `merchant_snapshot_id` with this option to use offers from a separate
visit. It must match the recipe character and exact client/build. The
offer capture can predate the inventory. Its timestamp is reported
separately, and a missing stock section leaves the plan unknown.

Use `merchant_item_id` or `merchant_currency_id` with `family: "forever"` in `list_snapshots`
to search across captures. Supply only one product ID. Results include the matching offer count and
the player's captured map when known. This is the map at export, so it
does not identify the merchant or verify current stock. The filter
combines with the character, recipe and instance filters before pagination.
Set `merchant_max_price_copper` with a product ID to limit the captured
copper price per bundle. A zero ceiling matches zero-copper offers.
Stock and extended costs are separate from this price filter.
With a product ID, set `merchant_in_stock_only` to require positive
or unlimited recorded stock and `merchant_copper_only` to exclude
extended costs. All selected filters must match the same offer.
Set `merchant_affordable_only` with a product ID to require positive or
unlimited recorded stock and a copper-only bundle price at or below the
wallet captured on the same visit. An absent wallet excludes the visit.
Positive limited stock remains a raw observation.

`character_summary` groups these observations under `repair_status`.
Alert counts and the repair quote can be independently unknown. Copper
shortfall is calculated only if the quote and opt-in wallet were captured
together. A zero shortfall means the saved balance covered the saved price.

Use `list_snapshots` with `family: "forever"` and
`repair_max_quote_copper` to find a repair-all quote at or below a limit.
`repair_affordable_only` requires a quote and a wallet recorded on the
same visit, with enough copper to cover the quote. The filters can be
combined with location or character selectors. Results report the
matched quote without returning the wallet balance.

`inspect_snapshot` with `section: "action_bar_observation"` shows loaded
built-in button slots, including observed empty slots and a count of missing
buttons. Multiple buttons can reference the same slot. This works separately
from the legacy `action_bars` collector and covers only the loaded built-in
button families listed by the client. It does not enumerate hidden pages or
custom bars. [Forever action button source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_ActionBar/Shared/ActionButtonUtil.lua)

Set `action_bar_spellbook: true` when inspecting `action_bar_observation`
to compare each observed spell action with base and reported spell IDs
from the same snapshot. Missing spell IDs or spellbook data leave the match
flags unknown. A false flag means no matching captured slot, which does
not establish that a spell is unavailable in game.
The review includes `matched_spellbook_flags_by_action_id` for the
selected spell actions. It keeps distinct base/reported ID and flag
observations without exposing unrelated spellbook entries.

`active_spellbook_slots_without_observed_action` lists slots with observed
false passive and off-spec flags and no matching base or reported spell ID
among captured spell actions. Missing flags exclude a slot. A missing
spellbook or action bar section gives null. This can miss spells reached
through macros, flyouts or bars outside the capture.
The unfiltered review also reports `repeated_observed_action_spell_slots`
by exact action spell ID. It works without a spellbook capture and leaves
base and override IDs separate.

Use `compare_snapshots` with `action_bar_slots: true` to compare two saved
Forever action bar observations. A shared slot can be filled, cleared or
have different recorded action data. Slots seen in only one capture are
listed separately. Missing action bar data leaves the comparison unknown.

Pass `action_bar_slot` with `action_bar_observation` to inspect one slot.
Use `action_bar_spell_id` to find all captured placements of an exact
spell action ID and their observed hotkeys. It combines with the slot
and hotkey selectors. Other action types and different spell IDs are excluded.
Use `action_bar_item_id` instead to match an observed item action. Item
and spell searches are mutually exclusive and keep their ID types separate.
Set `action_bar_item_stock` with the item selector to include captured
equipment and bag quantities when a shortcut matches. A missing storage
section stays unknown. Counts do not establish that the shortcut can be used.
In Forever, add `bag_cooldown_reference_at` to that lookup to group the
matching carried copies by projected cooldown end. The projection uses
the same capture as the shortcut and leaves current usability unverified.
Add `bag_cooldown_target_quantity` to see the gap at the reference time
and the earliest projected end at which known timers cover that many
carried copies. The plan requires a matching saved item shortcut.
Use `action_bar_item_stock_audit` to check every retained item shortcut at
once. Results group captured slots by item ID and report equipment and bag
counts only for those IDs.
Set `action_bar_item_stock_target_quantity` with the audit to show each
shortcut item's captured bag shortfall against the same target. Missing
bag observations leave the shortfall unknown. Equipment and bank stock
do not count toward the target.
Target rows include `observed_hotkeys_by_slot` for the retained shortcut
slots. A missing hotkey section or slot observation is unknown. A captured
button with no reported keys has an empty list.
`action_bar_slot_observed` is false for an unobserved slot and null for an
unavailable section. An observed empty slot has `empty: true`. You can
include its spellbook match, but the reverse list requires the full capture.

`button_hotkeys` links observed built-in buttons to their slots and up to
two keys for the button's binding command, with Blizzard's click-command
fallback. Multiple buttons may reference the same slot. A row with no keys
means neither lookup supplied a key. Missing rows are unknown. Exact slot
inspection returns only buttons for that slot. Update the importer and
reader before syncing this field. [Forever action button source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_ActionBar/Shared/ActionButton.lua)

The `action_bar_slots` comparison also reports `button_hotkey_changes` by
button name. It marks changes to keys and slot references independently.
Buttons seen on just one side are listed separately, and a missing hotkey
map on either side leaves the changes unknown.

Use `action_bar_hotkey` with `action_bar_observation` to find captured
buttons and their slots by exact key, ignoring ASCII case. The match count
is null if the hotkey map was unavailable. Zero means no captured match.
Set `action_bar_shared_hotkeys` to list keys observed on more than one
distinct retained slot. It compares keys without ASCII case and does not
infer whether the bindings are active at the same time.
You can combine this with an exact slot or the forward spellbook check.

Use `inspect_snapshot` with `section: "quests"` for quest details. `character_summary` counts observed and unknown turn-in statuses separately. Its `quest_checklist` groups quest IDs as ready, explicitly not ready, failed, or unknown. A captured failure takes priority in the checklist. A missing quest section gives a null checklist, and an observed empty log gives empty groups. Objective entries are indexed within a quest and include type, completion and available counts. The addon omits objective text, and a missing objective map indicates unavailable data. Older title-only snapshots remain readable. Update the companion and hosted importer/reader before syncing new quest detail. [Forever QuestLog API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/QuestLogDocumentation.lua)

Use `list_snapshots` with `family: "forever"` and `quest_id` to find
captures where that quest was observed. Add `quest_readiness` as
`ready`, `not_ready`, `failed` or `unknown` to narrow the results.
Unknown means the quest was observed without enough readiness evidence.
A missing quest does not match. Results return the ID and recorded status.
Omit `quest_id` to find captures with any quest in the requested state.
The result reports `quest_count` for that state. Empty or unavailable
logs do not match, including when searching for unknown readiness.
Add `quest_waypoint_map_id` to match the next waypoint recorded for
the same quest. It can be used alone or with readiness and a quest ID.
Without an ID, the result counts quests satisfying all supplied filters.
This uses the quest waypoint, independently of the player's captured map.

Use `quest_readiness_changes: true` in `compare_snapshots` to compare
the readiness of quests present in both Forever logs. IDs seen on only
one side are listed separately. A one-sided observation does not establish
why a quest appeared or disappeared. Missing quest logs yield null.

`character_summary.quest_objective_workload` counts unfinished objective
rows by captured kind and counts quests with unavailable objective maps.
It excludes explicitly failed quests. An observed empty map contributes
zero rows, and a missing quest section leaves the whole summary unknown.
It also lists up to 20 quests with the most observed unfinished rows.
This ranks objective counts, without estimating time or difficulty.

On the Rust MCP, pass `profession_skill_line` with `section: "professions"`
to inspect one saved profession and its observed rank cap. Duplicate
matches are ambiguous. Missing sections and unlisted skill lines have
separate observation statuses.

On the Rust MCP, pass `achievement_id` with `section: "tracked_achievements"`
to inspect one saved achievement and its captured remaining work. A false
observation flag means the ID is absent from the tracked list. A null flag
means the list was unavailable. Only player-tracked achievements are covered.

`character_summary.tracked_achievement_workload` lists up to 20 tracked
achievements ranked by the number of observed unfinished criteria.
Each ranked row shows the captured completion, character credit and
account-wide flags separately from the criterion count.
`completed_without_character_credit_ids` lists tracked achievements
observed complete without credit for the captured character, even if
they have no unfinished criterion rows.
A missing tracked list leaves the whole workload unknown.

On the Rust MCP, pass `faction_id` with `section: "reputation"` to inspect
one faction and its standard or special progress. An unlisted faction
has a false observation flag. An unavailable list gives a null flag.

`character_summary.standard_reputation_workload` lists up to 20 standard
factions with the fewest points to the next observed rank, with faction ID
breaking ties. It counts capped factions separately. Invalid brackets and
special reputation types are outside this ranking.

`character_summary.paragon_reward_summary` lists pending rewards by
faction and separates those with a captured level restriction. It includes
the reward quest ID when observed and lists paragon factions with unknown
reward status separately. The result describes the saved panel.
Each reward row also checks its quest ID against the same capture's quest
log and reports captured readiness when the quest was observed. Missing
quest IDs or quest logs leave the observation unknown. A ready quest flag
does not override the separately reported paragon level restriction.

On the Rust MCP, pass `currency_id` with `section: "currency_balances"`
to return one currency and its cap headroom. The observation flag is false
when the saved list lacks that ID and null when the list is unavailable.
An observed balance of zero remains zero.

`character_summary.currency_weekly_cap_summary` counts observed positive
weekly caps with known earnings and how many were reached. It lists up to
20 by the smallest fraction of the cap remaining, with currency ID as
the tie-breaker. Inactive rows and rows flagged as unable to earn weekly
remain visible with their flags. These counters describe the capture.
With `currency_id`, `compare_snapshots` reports `weekly_earned_delta`
when both entries share the same account scope. A decrease records the
counter change without establishing that a weekly reset occurred.
The comparison also reports `weekly_headroom_change` when both entries
share scope and have positive caps. It shows headroom on each side and
whether it crossed zero. A change in the cap itself can cause a crossing.

Add `currency_reward_quantity` to compare a proposed reward against the
tightest recorded cap. An explicit `can_earn_per_week: false` excludes the
weekly limit from the projection. If no applicable positive cap was recorded,
both projected amounts are unknown. This uses the captured counters and does
not advance resets or establish reward eligibility.

On the Rust MCP, use `inspect_snapshot` with a Forever snapshot ID and `spending_budget`
to check explicit costs. A budget can include `wallet_copper` and up to 20
distinct `currencies` rows with `currency_id` and `quantity`. Omit section
and quest selection. Missing balances remain unknown. A known shortage
makes the overall result false even if another balance is missing. The
result compares captured amounts and carries available inactive and account
scope flags. The budget response omits character names, currency names and
other snapshot sections. Use `items` for up to 20 distinct `item_id` and
`quantity` rows. These costs use carried bag stock by base item ID. Bank
counts are shown separately where captured. The budget suggests manual
withdrawals from observed character bank stock first, then account bank
stock. The remaining shortage reflects those observed withdrawals. Both
fields are unknown when bags were unavailable. Overall coverage still
uses carried stock. Vendor eligibility, prices and
purchasing are separate.

On the Rust MCP, add a positive `quest_id` to `inspect_snapshot` with
`section: "quests"` to inspect only that quest. The result reports whether
the quest was observed and lists unfinished objectives by captured row.
Remaining counts use captured required and fulfilled values. A null count
means the client supplied no quantities. A null objective list means objective
details were unavailable, while an empty object means no captured rows were unfinished.
Use the saved readiness flag separately for turn-ins.

`inspect_snapshot` with `section: "spellbook"` lists captured spell slots
in the player bank. Each row preserves the base `actionID` and the reported
spell ID, which may be an override. The export skips hidden and guild skill
lines. Available spell names, passive flags and off-spec flags are captured
as optional metadata. The export also skips
pet banks, future spells and flyouts. `character_summary` counts the
observed slots, which can include the same spell more than once. Update
the importer and reader before syncing this section. [Forever spellbook API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/SpellBookDocumentation.lua)

Pass `spellbook_spell_id` with the spellbook section to return matching
slots by either ID. A false `spellbook_spell_observed` means no captured
slot matched. Null means the spellbook capture was unavailable. The result
describes visible slots and does not establish permanent learning.

Use `spellbook_passive` or `spellbook_off_spec` with true or false to match
the corresponding observed flags. Missing flags do not match either value.
These filters combine with each other and with the spell ID selector.
When combined, `spellbook_spell_observed` reflects the filtered slots.

Use `spellbook_name` for a case-insensitive substring search of captured
spell names. It uses the language of the saved capture and combines with
the other spellbook filters. Slots with unknown names do not match.

Use `compare_snapshots` with `spellbook: true` to compare the observed base
spell IDs and their reported IDs. Slot reordering and identical duplicate
observations leave this comparison unchanged. For the same base and
reported spell IDs, it also shows changes in observed passive and off-spec
flags. Conflicting observations are retained, and missing flags stay unknown.
Missing spellbook data
on either side leaves changes unknown. Additions and removals describe the
observed slots and do not establish permanent learning or unlearning.
Added and removed base IDs also list any names observed on the respective
side. Duplicate names are collapsed and differing names are retained.
When the later capture contains action-bar observations,
`after_action_slots_with_lost_spellbook_match` lists its spell shortcuts
whose exact IDs appeared as base or reported IDs only in the earlier
spellbook. This is a shortcut review cue based on the observed slots.
`after_action_slots_with_gained_spellbook_match` lists later shortcuts
whose IDs appeared only in the later spellbook, whether as base or reported
IDs. The comparison does not establish when a shortcut was placed.
The same comparison reports `observed_usability_changes` for exact
reported spell IDs present in both spellbooks. It compares distinct pairs
of usability and insufficient-power flags, with null for an unreadable
row. Slot order and repeated identical readings do not affect the result.
Differences describe saved observations under their capture conditions.
`observed_resource_changes` uses the same exact-ID matching and keeps
charge counts, reported cast counts and cooldown flags together as
observed on each row. Missing values remain null. A change describes
different readings across captures and leaves current castability unknown.

`inspect_snapshot` with `section: "experience"` shows the saved character's
XP, level threshold and any observed rested XP or XP-disabled flag.
`character_summary.experience_progress` also reports XP to the next level.
Effective level caps and trial caps have separate statuses and omit numeric
progress. The addon reads the player only and omits trial banked XP. Update
the importer and reader before syncing this section. [Forever experience bar](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_StatusTrackingBar/Shared/ExpBar.lua)

Set `quest_xp_projection: true` when inspecting a saved Forever quest by ID.
The tool compares its captured reward with the XP remaining in the same
snapshot. The comparison is unknown for capped or explicitly XP-disabled
characters and when reward or progress data is missing. It covers one
recorded level threshold and does not predict bonus XP or later eligibility.

Use `compare_snapshots` with `experience: true` for two saved captures of
the same Forever character and build. It reports captured levels and XP
before and after. The within-level XP difference requires matching levels
and thresholds. After a level change it remains unknown, since the two XP
counters describe different levels.

`elite` and `suggested_group_size` are separate quest log observations.
`character_summary.observed_quest_difficulty` lists observed elite quests
and quests with a suggested group size above one. Missing fields mean the
client supplied no usable value. An omitted quest in the summary is not
evidence it can be completed solo. Update the importer and reader before
syncing these fields. [Forever quest API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/QuestLogDocumentation.lua)

`timer` records the quest's total and remaining seconds at export time,
rounded down. `character_summary.observed_quest_timers` lists observed timers
with the least time left first. A missing timer means no usable timer was
returned. These are saved observations and do not count down after export.
Each row includes the captured total and `estimated_end_at`, calculated
as capture time plus the recorded remaining seconds. The estimate assumes
the timer continued without changes after capture.
With `quest_id` in `compare_snapshots`, the timer comparison reports
which captures recorded a timer and gives signed remaining and total
second changes when both did. A missing timer does not establish expiry.
Selected quest comparisons also show `objective_counts_by_kind` for
each observed objective map. Counts are split into finished and unfinished
rows. This avoids matching individual objectives by row position, which
can change between exports. A missing map remains unknown.
Update the importer and reader before syncing the new field. [Forever quest tracker](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_ObjectiveTracker/Blizzard_QuestObjectiveTracker.lua)

The selected quest can include `currency_rewards`, indexed by reward row.
Each row contains `currency_id` and the panel's `totalRewardAmount` as
`quantity`. This covers the panel's receive list. Choice rewards use a
separate client path. The amount can include a contextual bonus and does
not establish what a later turn-in will award after caps or eligibility
checks. An empty object is an observed empty list. An absent field means
the list was unavailable or unreadable. Update the importer and reader
before syncing the new field. [Forever quest panel](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_UIPanels_Game/Mainline/QuestInfo.lua)

Select a quest in the game's quest log before exporting to capture
`currency_choices`. Its keys are original choice positions, so item
alternatives can leave gaps. Each row lists the currency ID and displayed
quantity for one alternative. An empty object means no currency choices
were observed for the selected quest. Other quests and unreadable choices
omit the field. The addon does not change selection or choose a reward.
Update the importer and reader before syncing this field.

The selected quest can also include `item_rewards` and `item_choices`, with
base item IDs and quantities. Item choices keep their positions in the same
list as currency choices. Empty maps represent observed empty lists. Missing
maps mean the quest was unselected or that list could not be read. Update
the importer and reader before syncing these fields.

The selected quest's `reward_amounts` can contain displayed `copper`, `xp`
and `honor`. Zero is observed, and missing amounts are unknown. These are
reward panel values at export time. `required_copper` remains a separate
cost observation. Update the importer and reader before syncing this field.

`renown_rewards` lists major-faction reputation rewards by row, with faction
ID and reputation points. This covers the dedicated major-faction API.
An absent list means the client supplied no usable observation. An empty
list is an observed empty result. Update the importer and reader before
syncing this field. [Forever quest API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/QuestLogDocumentation.lua)

Set `quest_renown_projection: true` when inspecting a saved Forever quest
by ID to total its rewards by faction and compare them with captured renown
progress. A capped faction or missing progress has no next-rank estimate.
The calculation covers the next recorded threshold only.

`spell_rewards` lists spell IDs in the order returned by the quest reward API,
before the panel groups or filters them. When
available it also includes the client's numeric `reward_type` and
`is_spell_learned` reward flag. That flag describes the reward and does not
query the character's spellbook. An empty object is an observed empty list.
When available, `name` identifies the spell and `in_player_spellbook` reports
the same player spellbook check the quest panel uses. It includes overrides,
so presence alone does not establish permanent learning.
Update the importer and reader before syncing this field. [Forever quest reward API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/QuestInfoSystemDocumentation.lua)

Set `quest_wallet_projection: true` when inspecting a saved Forever quest
by ID to compare its captured coin reward and required payment. The net
amount requires both observations. The upfront shortfall uses the optional
wallet from the same snapshot. The projected balance is available only when
the wallet covers the full cost and the coin reward is known. This is a
saved-data calculation and does not establish turn-in readiness.

Set `quest_item_catalogue: true` when inspecting a saved Forever quest by ID
to look up item names, required levels and simulator slots in the verified
local catalogue. References keep their captured reward positions. A null row
means the base item ID has no catalogue match. The original reward quantities
stay in `data`. The slot list describes simulator placement and does not
establish equip eligibility or identify item modifiers.
Each matched reward also lists the captured items in its compatible slots.
A null `equipped_slots` means equipment was unavailable. A null item ID
means the slot was empty in the captured equipment section. A known ID
with a null name has no catalogue match.
`default_item_level` comes from the catalogue's default scaling record.
`reward_item_level_delta` subtracts the equipped item's base level from the
reward's base level. The delta is null if either level is unavailable.
This measures item level and does not rank gear or account for scaling,
modifiers, set bonuses or combat performance.
`spell_recipe_references` uses exact spell IDs to match crafted item and
enchant sources. It keeps reward positions and shows up to eight references
per spell with the full match count. A match identifies a catalogue recipe
association. The quest reward and the character's learned recipes remain
separate observations.
For a catalogue recipe match or an exact selected or tracked recipe ID,
`observed_recipe` also reports saved tracking and learned flags from the
same snapshot. Selected, tracked craft and tracked recraft flags stay
separate. Missing learned status stays unknown, even if the reward spell
is present in the player's spellbook.

Set `quest_currency_projection: true` when inspecting a saved Forever
quest by ID to compare its receive rewards with currency caps from the same
capture. Rows for the same currency are summed. Each currency choice is
compared separately with the receive total for its currency. Missing receive
data leaves the combined choice result unknown. Missing recorded cap headroom
leaves the within-cap and above-cap amounts unknown. These comparisons use
captured quantities and are independent of later eligibility or payout.

Pass `quest_waypoint_map_id` with `section: "quests"` to return only
quests whose next waypoint was recorded on that map. The filtered checklist
separates ready, not ready, failed and unknown turn-in status. The response
also counts quests with an unknown waypoint map in the full capture.

`character_summary` can report the player's `captured_map_id` and
`quests_with_waypoint_on_captured_map`. The list matches exact map IDs
from the same export. It does not measure distance or include coordinates.
Missing map or quest data gives null. Update the importer and reader before
syncing this field. [Forever map API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/MapDocumentation.lua)

When the client returns matching metadata, the summary also shows the
localized `captured_map_name` and `captured_map_parent_id`. These fields
can be unknown even when the map ID was observed.
When the map API reports a positive, ordered player level range,
`captured_map_min_level` and `captured_map_max_level` show that range.
It describes the captured map and does not establish individual quest or
instance eligibility. [Forever map API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/MapDocumentation.lua)

Export while the built-in flight map is open to inspect `flight_map` later.
It records the viewed map ID, node names and IDs, recorded availability,
map transition markers and available fares for reachable destinations.
Optional stable map-relative coordinates are included when readable;
opening the map and travelling remain with the player. It covers the open flight map, so availability and fares
can differ at another flight master. Update the importer and reader before
syncing this section. [Forever flight map source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_FlightMap/FM_FlightPathDataProvider.lua)

Use `flight_destination_node_id` with `family: "forever"` in
`list_snapshots` to find captures with a reachable destination and an
unambiguous origin. Results include the captured origin, destination,
map and fare when known. The filter applies before pagination and can
be combined with an explicit character. Routes describe past captures.
Add `flight_origin_node_id` to match a specific departure point before
pagination. This option requires a destination ID.
Add `flight_max_fare_copper` to keep only routes with a recorded fare
within your ceiling. It requires a destination ID. Zero matches free
flights, and unknown fares are excluded when a ceiling is supplied.
Set `flight_affordable_only` to require an observed fare and wallet from
the same capture, with the fare at or below the balance. This option also
requires a destination ID. Results include the fare and omit the wallet.

To review a saved itinerary, use `inspect_snapshot` with
`section: "flight_map"` and `flight_node_id` for the first leg. Supply
one to seven `flight_onward_legs` with `snapshot_id` and
`destination_node_id` per leg. The captures must share the same
character and client/build. Each onward departure must match the
previous destination. The total is unknown if any fare is missing.
Use this without `flight_affordable_only`. Legs retain their separate
capture times and describe historical routes.
The review compares the total with the first capture's opt-in wallet
and reports affordability and any copper shortfall when both are known.
Later captures supply fares only.
`starting_wallet_affordable_prefix_legs` counts consecutive legs from the
start, stopping at the first unknown fare or fare above the remaining
budget. A missing starting wallet leaves the count unknown.
`starting_wallet_first_uncovered_leg` gives the one-based leg number and
whether the fare was unknown or exceeded the remaining copper. It is null
when every leg fits or the starting wallet was unavailable.
Set `flight_wallet_reserve_copper` on the itinerary to check full-route
affordability, copper shortfall and consecutive affordable legs while
preserving that amount in the starting wallet. The fare-only checks
remain available separately.
Set `flight_itinerary_include_repair_quote: true` to report a separate
budget that includes the starting capture's repair-all quote and any
explicit reserve. Onward quotes do not contribute. A missing starting
quote leaves the repair-adjusted affordability and prefix unknown.
Set `flight_departure_map_id` with a destination ID to match the player's
map when the flight panel was captured. This can use a quest waypoint
map ID to look for departures from that area. The filter requires an
exact captured player-map match and returns that map separately from
the viewed flight map.

Use `flight_node_id` to select a recorded node, or `flight_reachable_only`
to keep reachable destinations and exclude map transitions. Both filters
apply before the fare projection and preserve original row keys.
Use `flight_affordable_only` to keep reachable destinations with a recorded
fare at or below the same capture's wallet. Unknown fares are excluded,
and an unavailable wallet leaves the filtered result unknown.

Set `flight_wallet_reserve_copper` on `list_snapshots` with a destination
to keep quotes whose recorded fare leaves at least that amount in the same
captured wallet. This implies affordability, including when the reserve is
zero. The matching row reports the calculated remaining copper and requested
reserve. Unknown fares or wallets cannot satisfy this historical filter.

Set `flight_wallet_projection: true` with `section: "flight_map"` in
`inspect_snapshot` for affordability and copper shortfall by captured
slot. A missing fare or wallet leaves both values unknown. Each comparison
uses the same saved balance independently. The result covers the copper
fare at capture time.

`character_summary.flight_fare_summary` lists up to 20 reachable
destinations with the cheapest observed fares, using node ID to break
ties. It counts unknown fares separately and does not require a wallet.
A missing flight panel or ambiguous departure leaves the summary unknown.

Set `flight_wallet_reserve_copper` with either affordability option to
budget a separate expense such as a saved repair quote. Supply the reserve
amount explicitly. The projection compares fare plus reserve with the
flight capture's wallet, and the filter retains only routes that fit both
costs. An unknown fare or wallet still leaves affordability unknown.

For route discovery, pass `flight_route_search` to `inspect_snapshot`
with `section: "flight_map"`. Supply `destination_node_id` and
optionally up to seven `onward_snapshot_ids` from the same character
and client build. The starting panel supplies the first leg. The search
minimises the sum of observed fares across up to eight legs. Equal fares
prefer fewer legs, then compare each leg in order by panel input position
and destination node ID. It uses only reachable destinations
with quoted fares and excludes map transitions. The plan includes the
saved panel for each leg and checks the starting wallet when available.
Set `max_legs` inside `flight_route_search` to a value from one to eight.
One limits the search to direct flights from the starting panel. Omit the
limit to allow up to eight legs.
Set `objective` to `fewest_legs` to prefer fewer legs among fully quoted
routes. Ties prefer the lower total fare, followed by the same per-leg
ordering. The default objective is `lowest_fare`.
Add `flight_wallet_reserve_copper` to check whether the whole route fits
while preserving that amount. The plan reports `affordable_with_reserve`
separately from fare-only affordability. A missing starting wallet makes
both checks unknown.
Set `include_repair_quote: true` inside `flight_route_search` to
include the repair-all quote from the starting capture. The
`affordable_with_repairs_and_reserve` check includes fare, that quote
and any explicit copper reserve. It is unknown if the starting wallet
or quote was unavailable. Onward quotes do not contribute.
Set `require_affordable: true` inside `flight_route_search` to limit
the search to fares covered by the starting wallet after any reserve
and requested repair quote. The objective then ranks only those routes.
`starting_budget_unknown` means the wallet or requested quote was missing.
`no_affordable_route` means no fully quoted route within the leg limit fit
the saved budget. Without this option, budget checks describe the selected
route without constraining the search.
`no_fully_priced_route` describes the supplied captures. Routes are
historical observations and may have different capture times.
When search evidence shows a reachable path despite no qualifying route,
`saved_path_witness` lists its saved legs and their capture times. It
shows the cheapest fully quoted path if one exists outside the budget
or fare ceiling. Otherwise it shows a shortest observed path and leaves
the total fare unknown. Missing leg fares remain visible in the witness.
When the starting budget cannot be established or upfront expenses exceed
the wallet, `starting_budget_issue` identifies the cause. Path evidence
and any witness remain available for diagnosis, with no effective fare
ceiling or qualifying route.

`compare_snapshots` compares flight maps only when the map ID and
unambiguous current departure node agree. It reports availability
changes and signed copper differences for shared destination IDs.
Changed rows also show the destination name from the later capture and
the observed fare on each side, so a newly reachable destination can show
its current captured fare even when there is no price delta.
Map transitions are excluded. Destinations seen on only one side are
reported separately as coverage differences. A fare difference requires
an observed quote on both sides. Quotes that appear or become unknown
are reported separately from price changes.

For the same map and departure node, `affordability_changes` shows
destinations whose fare fit one capture's wallet and exceeded the other.
Each destination must be reachable and have a quoted fare in both
captures. A missing wallet makes the list null. The list does not expose
wallet balances.

Set `quest_on_captured_map: true` to use that map directly in quest
inspection. It combines with the other quest list filters and is mutually
exclusive with an explicit waypoint map ID. A missing captured map leaves
the filtered quest data and checklist unknown.

Add `quest_difficulty: "elite"` or `quest_difficulty: "suggested_group"`
to filter by observed elite status or a suggested group size above one.
The checklist reflects the filtered quests. Unknown difficulty counts apply
after the optional waypoint filter, while the waypoint unknown count still
describes the full capture. These filters require the quests section without
a quest ID.

Use `quest_readiness` with `ready`, `not_ready`, `failed` or `unknown` to
return only quests in that checklist category. A captured failure takes
precedence over the ready flag. This filter combines with waypoint and
difficulty filters and requires the quests section without a quest ID.

Use `quest_max_remaining_seconds` to include only quests with a recorded
timer at or below the given number of seconds. Zero matches recorded zero
seconds. Quests without a usable timer are counted separately after any
waypoint, difficulty or readiness filters. This uses the timer at export.

Use `quest_reward_item_id` to find quests with a matching fixed item reward
or item choice in the saved log. The unknown count covers quests left by
earlier filters that had no match and at least one unavailable item list.
Item lists are captured for the selected quest, so other quests may have
unknown rewards. This filter requires the quests section without a quest ID.

`quest_reward_currency_id` searches fixed currency rewards and currency
choices the same way. When combined with an item reward filter, quests must
match both. The currency unknown count applies after the item filter.

`quest_reward_faction_id` searches captured major-faction rewards and
`quest_reward_spell_id` searches captured spell rewards. Missing lists count
as unknown. When multiple reward filters are used, each narrows the previous
results in item, currency, faction and spell order.

Use `quest_waypoints: true` with `compare_snapshots` to list changed
next waypoint maps for quests present in both captures. A map becoming
observed or unknown has its own status. If either quest log is unavailable,
the comparison is null. Quests entering or leaving the log are outside
this comparison.

`character_summary.quest_money` lists saved positive quest copper requirements
and `inspect_snapshot` accepts `quest_max_required_copper` for the quests
section. Zero finds quests with an explicitly recorded zero cost. Unknown
costs are counted separately among rows matching the other filters.
Set `quest_within_captured_wallet` to compare each cost with the wallet
in the same capture. An optional copper cap can narrow the results further.
If wallet data is unavailable, the filtered result is unknown.
Set `quest_list_wallet_projection` to total captured costs and rewards
across retained quests. The affordability check compares the combined
cost with the saved wallet before any reward is credited. Unknown costs
and rewards are counted separately.
When every retained quest is ready and both copper amounts are recorded,
`ready_turn_in_order` shows the minimum starting copper and a suggested
order. This assumes each reward can fund subsequent turn-ins and considers
copper only. Travel and timing are outside this calculation.
With a captured wallet, the order also shows how many consecutive quests
fit, and the first blocked quest and its shortfall. This describes the
suggested order without searching for a different partial route.
Set `quest_list_currency_projection` to total known fixed currency rewards
from the retained quests and compare them with the same capture's cap
headroom. Choice rewards are separate, and missing fixed reward lists
are counted. Missing cap information leaves the comparison unknown.
Set `quest_list_renown_projection` to compare known renown rewards by
faction with the next rank in the same capture. A shortfall is measured
against known rewards. Missing reward lists may change the final result.
Set `quest_list_item_rewards` to total known fixed rewards by base item ID
across retained quests. Quest IDs with item choices appear separately,
and missing fixed and choice lists are counted independently.
Use `quest_min_reward_xp` to keep quests with at least that much captured
XP. Unknown rewards are excluded and counted before the copper filters.
Set `quest_list_xp_projection` to total observed XP across retained quests.
Combine it with readiness for a turn-in plan. If known rewards fall short
and some rewards are unknown, coverage of the level threshold remains unknown.
The `ready_turn_in_plan` uses only retained quests explicitly ready for
turn-in with a recorded XP reward. It picks the fewest quests needed
to reach the saved threshold by taking larger rewards first, breaking
ties by quest ID. If known ready rewards fall short, the plan reports
that shortfall and leaves the selected list null. A missing threshold
or explicitly disabled XP gain leaves the entire plan unknown.
Request `quest_list_wallet_projection` alongside the XP projection
to show a separate wallet estimate for just the selected quests. It
uses the same capture's wallet and reports observed costs, unknowns and
the cost shortfall without crediting rewards. The XP selection itself
still prioritises the fewest recorded turn-ins.
When the selected quests have an observed wallet order and the wallet
was captured, the estimate also reports XP from the affordable prefix
and the XP still needed. This follows that order and does not search
for a different subset of quests.

`character_summary.quest_money` also counts quests with an unknown cost.
If the same export includes the optional wallet, each entry shows its
shortfall against that balance. These
are individual comparisons against the captured balance. Update the importer
and reader before syncing captures from the new addon.

When the client exposes a quest's next waypoint during a player export,
the capture can include its map ID. `character_summary` groups saved quest
IDs by that map and lists quests whose waypoint map is unknown. The map
describes the next waypoint at capture time. Update the importer and reader
before syncing captures with this field. The export omits coordinates
and never sets a waypoint. [Forever quest waypoint API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/QuestLogDocumentation.lua)

Offline checks verified zero progress, false versus unavailable readiness, sparse objective rejection without losing the quest title, and real MCP inspection and comparison. Forever `0.26.0-dev` passed package verification (SHA-256 `f8d22f78bc7a87595534ebf7d4ef1a5b703a7687c4c4c87f89ecca3dd6776604`). Native capture remains pending.

Use `inspect_snapshot` with `section: "tracked_achievements"` for Forever achievement progress. The addon reads the player's existing tracked list on export. The result includes completion flags and available criterion quantities. It omits contributor names, dates, guild entries and free-text criterion descriptions. Criterion keys are observed row numbers, with native criterion IDs included when present. Check the ID before interpreting a changed row as progress. The Rust MCP includes `tracked_achievement_work` keyed by achievement ID and then captured criterion row. It lists rows observed as unfinished, with the criterion ID and remaining count when available. An empty map for an achievement means no captured criterion row was unfinished. A missing tracked achievement section returns null. Use the separate achievement completion and character credit flags for completion status. An achievement absent from the tracked list has no completion implication. [Forever objective tracker](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_ObjectiveTracker/Blizzard_AchievementObjectiveTracker.lua)

Offline tests verified tracked-list validation, missing versus empty observations, account completion separate from character credit, omission of identity fields, and a criterion quantity change through the real MCP comparison route. Forever `0.25.0-dev` passed package verification (SHA-256 `25621b3174d6a842b507cee6cac6e614725e04dede289cf34e41698c0d2a44a5`). Native capture remains pending.

Use `inspect_snapshot` with `section: "currency_balances"` for Forever balances. `character_summary` reports the number observed and `compare_snapshots` compares stable IDs. An absent ID can reflect a hidden or collapsed row. Zero is a balance only when observed. The older `currencies` section retains row-number keys and is separate. The Rust MCP also reports `currency_cap_headroom` for observed positive caps at capture time. Weekly headroom subtracts captured weekly earnings. Maximum headroom uses total earnings or held quantity according to the captured cap basis. Zero means the captured counter met or exceeded the cap. A missing positive cap gives null for that limit, and currencies with neither cap are omitted. A missing currency section gives a null result. The figures describe captured counters, including inactive rows. New captures include `can_earn_per_week` when the client returns an accessible boolean. Older captures and unavailable flags omit the field. Weekly headroom remains a comparison against the reported limit. The result does not predict earnings or resets. Transfer history is excluded. [Forever currency API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/CurrencyInfoDocumentation.lua)

Offline tests verified stable output after reordering, observed zero balances, visible inactive rows, duplicate and missing row refusal, and a balance change through the real MCP comparison route. Forever `0.24.0-dev` passed package verification (SHA-256 `707874b2ea122574d6a78151e0a3f4493f8a94d39705347f3d465bb6988bd88a`). Native capture remains pending.

Bank discovery uses the [Forever bank API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/BankDocumentation.lua) and [container API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ContainerDocumentation.lua) from Blizzard's generated UI documentation as mirrored by Gethe. Runtime qualification remains pending.

In the Rust MCP, `inspect_snapshot` with `section: "professions"` reports
`profession_training_progress` by captured slot. Each row identifies the
skill line and shows points remaining to the observed cap. Zero means the
captured rank equals a positive cap. Zero or inconsistent caps give null
remaining points. The result describes the captured cap and does not
identify available trainers or a future rank cap.

Use `list_snapshots` with `family: "forever"` and
`profession_skill_line` to find saved captures containing that profession.
Add `profession_min_rank` for an inclusive lower rank bound. Missing or
ambiguous matches are excluded. Each matching row reports the captured
skill line, rank and cap under `profession_match`. The search can combine
with an exact character or captured recipe filter before pagination.

`compare_snapshots` reports `profession_progress` for Forever captures.
It matches unique skill-line IDs regardless of captured slot, reports
rank and cap changes, and separates one-sided and ambiguous observations.
A one-sided ID only establishes where the profession was observed.
If only one capture has a profession section, the report is null.

Use `inspect_snapshot` with `section: "tracked_recipes"` to read saved
player-tracked recipes, separated into crafts and recrafts by recipe ID.
Optional names reflect available recipe metadata. Empty maps
mean the corresponding tracked list was observed empty. This export leaves
tracking and the profession panel state to the player and covers tracked
recipes only. `compare_snapshots` reports `tracked_recipe_changes` by ID for
crafts and recrafts when both lists were captured. Name changes leave
membership unchanged. If only one capture has tracked recipe data, the
membership comparison is null.
`tracked_recipe_learned_changes` separately lists changes where the same ID
was tracked in the same list and both captures supplied a learned flag.
An empty list means no known changes were found. One missing tracking
section yields null. With both sections missing, the field is omitted.

To find captures where a recipe was tracked, pass `tracked_recipe_id`
and `family: "forever"` to `list_snapshots`. A matching row includes
craft and recraft membership plus each recorded learned flag. The
learned flag is null when the recipe is absent from that group or its
status was unavailable. The filter combines with character and selected
recipe filters before pagination.
With that ID, set `tracked_recipe_kind` to `craft` or `recraft`,
and optionally set `tracked_recipe_learning` to `learned`, `unlearned`
or `unknown`. Both filters must match the same observed tracking row.
Unknown means a row was captured without a learned flag. Results still
show the observations from both groups.

To search without a recipe ID, supply a kind, learning status or both.
Matching visits report `tracked_recipe_counts` for crafts and recrafts.
An excluded group counts as zero. A visit needs at least one matching
entry, so missing and empty tracking sections are excluded.

Pass `tracked_recipe_id` with `section: "tracked_recipes"` to check one
recipe. `tracked_as_craft` and `tracked_as_recraft` are independent. False
means the ID was absent from an observed list, while null means the list
was unavailable. Each returned row can also have a separate `learned`
flag from the recipe API. Missing learned status stays unknown regardless
of tracking membership. Update the importer and reader before syncing this
field. [Forever recipe metadata](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/TradeSkillUITypesDocumentation.lua)

Use `tracked_recipe_learning` with `learned`, `unlearned` or `unknown` to
filter both tracked lists independently. This requires `tracked_recipes`
without a recipe ID. An unavailable capture remains unavailable.

`review_forever_recipe` includes `recipe_tracking` for each reviewed recipe.
Tracking comes from that recipe's own capture, including when the review
uses another snapshot for material counts. It remains separate from the
recipe's captured learned status.
`tracked_craft_learned` and `tracked_recraft_learned` preserve the learned
flags in the corresponding tracking rows when available.

A selected recipe can include its observed `profession` ID, name, rank and
cap. The review reports `profession_points_to_observed_cap` when a positive
cap and consistent rank were captured. This describes the profession reported
for that recipe. Update the importer and reader before syncing this field.

`list_snapshots` accepts `recipe_profession_id` for Forever. It can be
combined with the character and output item filters. Only captures with an
exact observed profession ID match. Older captures may lack that field.

Use `recipe_id` with `list_snapshots` to locate captures of a specific
selected recipe, then pass the snapshot ID to `review_forever_recipe`.
This matches the selected schematic, including older captures without
profession metadata. A tracking entry alone does not match.

`material_and_charge_capacity` shows the smaller of the observed material
limit and saved recipe charges, with the shortfall against the requested
craft count. It reports bags and all observed storage separately. With required
items, either missing limit makes that estimate null. Explicit zero-item fixed
requirements instead use known saved charges directly. Missing charges still
leave the combined estimate unknown. The result does not advance cooldowns
or establish that the character meets the recipe's other requirements.

Each fixed reagent also lists `suggested_bank_withdrawals` to cover its bag
shortfall using observed character bank stock first, then observed account
bank stock. An uncaptured bank remains unknown, and unavailable bags prevent
a suggestion. These per-recipe amounts are independent. For a shared batch,
use `ordered_starting_materials` to account for materials used across recipes.

The selected recipe collector reads the visible local schematic form during `/aoexport`. The panel must already be open on a local item or enchant recipe. Saved recipe reviews also expose the captured relative difficulty, skill-up flag and reported number of skill-ups where available. These are observations at the recipe capture time, including when inventory is taken from a later snapshot. They do not estimate skill-up probability or predict gains from a batch. Item recipes with a captured schematic output range also report `base_output_if_completed`, the base minimum and maximum multiplied by the requested crafts. This is conditional on completing the crafts and excludes bonuses. Use `target_output_quantity` instead of `crafts` to round up from a desired quantity using the captured minimum per craft. This works for individual and batch recipes with a known range, within the existing 1,000-craft limit. Batch reviews also report `combined_base_output_if_completed` by observed output item. If a recipe for the same item lacks a range, the combined range for that item is unknown. Recipes without an observed output item are counted separately and excluded from the totals. Batch reviews also report `ordered_starting_materials`, assuming each recipe completes in supplied order with its minimum captured yield. Earlier recipes can cover later reagents by matching item ID. Each recipe consumes its entire requested batch of reagents before producing output. Unknown yields contribute zero and unsupported material requirements suppress this estimate. Stock shortfalls use observed inventory only. The ordered plan also shows bag shortfalls and suggested withdrawals, using observed character bank stock first and then observed account bank stock. Uncaptured banks remain unknown. If bags were unavailable, the bag shortfall and withdrawal suggestion are unknown. Players withdraw reagents manually. Older or unavailable output ranges and enchant recipes return null. Learned recipes can include saved cooldown seconds rounded up, the daily flag and available charges. Reviews show the requested craft count above observed charges when charges were captured. Cooldowns describe the recipe capture and are not advanced using the snapshot age. Recipe captures can include the panel’s tool, station and area conditions. Reviews report observed unmet rows in `unmet_crafting_conditions`. An empty object means no unmet condition was captured in the observed list. An unavailable list gives null. These conditions are tied to the recipe capture, including when material counts use a newer inventory. Enchant review covers fixed consumed reagents. Target selection and eligibility remain outside the estimate. The planner reports shortfall against the observed inventory and counts each reagent separately in bags, character bank and account bank. Uncaptured storage has a null count and observed storage with no matching item has a zero count. Missing observations, unlearned recipes, optional inputs and non-material requirements can affect actual craftability. [Forever schematic form](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeSchematicForm.lua) · [Forever trade skill types](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/TradeSkillUITypesDocumentation.lua)

Offline checks passed for the Lua capture, Rust import and MCP review, including duplicate material quantities, incomplete inventory, and refusal of malformed reagent lists and salvage recipes. The `0.22.0-dev` Forever package passed byte and Lua export round-trip verification (SHA-256 `796bac24b8072309cdbee98ae8d0ce17dda6af595922c6edefbfc164b738609f`). Caller and failure review identified sparse reagent slots and separate salvage inputs, and the revised guards passed the follow-up probes. Jev advisory triage classified both as material (`jev-1.14.0`, request `9b76e769cba333e61da3c48a5110227aaa3c65ddb7041129f551dad8ff03c1d3`, 439 input/46 output tokens, 0.305 seconds). The Forever addon has not been installed.

Each recipe review includes `captured_preparation` checks for learning,
observed conditions, observed charges and coverage of fixed reagents.
Material checks distinguish bags from all observed storage. False means
the recorded evidence falls short. Null means the check could not be
evaluated. Material checks in each batch entry use starting inventory
independently. The ordered batch plan accounts for shared materials and
outputs from earlier recipes. With a later inventory capture, only the
material checks use that capture. Learning, conditions and charges still
reflect the recipe capture.

For an ordered batch, `ordered_nonmaterial_requirements` identifies the
first recipe with an observed learning, condition or charge shortfall.
It lists each failed check on that recipe and counts recipes with at
least one unknown check. A null shortfall means no failure was observed.
The ordered material checkpoint remains separate and uses the batch's
shared inventory and projected outputs.

Recipe reviews also show `estimated_cooldown_end_at` when a positive
remaining duration was observed. The estimate adds the rounded duration
to the recipe capture timestamp, even when material counts come from a
later export. A charged recipe can already have available charges while
the timer runs. The estimate does not verify present craftability.

Use `inspect_snapshot` with `section: "reputation"` for the Forever faction observations. `character_summary` reports the observed faction count and `compare_snapshots` can compare observations for the same character and build. The Rust MCP also reports `standard_reputation_progress` by faction ID. It gives points earned in the current rank, rank width and remaining points to the next rank when the captured standard standing fits the bracket. Maximum standard reaction is marked capped. Special progression types and inconsistent brackets are omitted from that calculation. Captured renown rows can separately include `renown` with the observed level, points earned in the level, level threshold and maximum status. This detail is absent from older exports and when the renown API fails or returns inaccessible values. Friendship rows can include a separate `friendship` observation with its own standing and rank thresholds. A captured friendship observation without a next threshold represents maximum rank in the client panel. An absent friendship object means details were unavailable. Paragon rows can include the observed counter, reward threshold, pending-reward and level restriction flags, and a positive reward quest ID when accessible. The Rust MCP includes `special_reputation_progress` for factions with those details. It shows remaining points for renown and friendship ranks when the captured numbers support it, and distinguishes pending paragon rewards, level-restricted pending rewards, and no pending reward. Reward claiming remains a player action. When comparing captures by `faction_id`, `special_progress_change` reports renown level and cap changes, friendship standing changes, or paragon reward flag changes. Renown points are compared only within the same uncapped level and positive threshold, with both counters within range. Missing detail or a changed faction type or account scope gives null. The collector does not expand headers, change filters or select factions. [Forever reputation API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ReputationInfoDocumentation.lua) · [Forever reputation panel](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_UIPanels_Game/Camelot/ReputationFrame.lua)

Offline verification covered signed standing values, account-wide and header factions, absent rows, unavailable progression classification, and the real MCP inspection, summary and comparison paths. Forever `0.23.0-dev` passed package verification (SHA-256 `5cb0b7e395ab7248f77e13fd22550f3fa8350004d8f0e013877aaf507cf24339`). Native capture remains pending.

Final ranked owned-gear results preserve per-encounter iteration counts, seed,
request hashes and available numeric uncertainty from the existing simulator
adapter. Paired intervals concern candidate minus equipped baseline; Wilson
intervals concern marginal death fraction. Aggregate ranking uncertainty and
selection-adjusted optimality remain unverified; no extra engine runs occur.

Owned-gear evaluation can select an explicit character with `--character-name`
and `--character-realm` instead of `--snapshot-id`. The existing owner-scoped
MCP resolver chooses the newest matching historical Forever capture once;
the evaluation pins that snapshot ID. No match fails before plan writes.
Missing/mixed selectors are rejected; there is no newest-alt or active-player
fallback. A resumed journal stays bound to its original capture and rejects a
changed selection. Resolution consumes the shared budget, capped at 15 seconds.

Positional Forever equipment reviews accept explicit desired empty occupied
slots through `clear_equipment_slots`, independently for primary/additional
drafts. Previous-item evidence stays attached, and the exact simulator layout
preserves the holes. Invalid/duplicate/uncaptured slots, shirt/tabard and selected
target overlaps are refused. No items move; bag destinations and native legality
remain unknown. Opt-in empty-choice search and saved-plan replay are now
implemented; see ADR 047 and empty-equipment search verification.

The saved empty-ring workflow has also been checked with real pinned-engine
paired simulations and uncached direct references: nine coordinates, seven gear
states, zero warm simulations. This is one Hunter/encounter caller-path check;
native legality, broader model accuracy and full owned BiS remain unqualified.
See real-empty-equipment-verification-2026-11-09.md.

Passive carried-item cooldown observations are available in manual Forever
exports with addon 0.46.0-dev and matching readers. `bag_item_cooldowns` retains
source bag/slot/item linkage, stable public timer/enabled data or explicit unknown
rows. Summary counters distinguish readable, unavailable, disabled and positive
timers. No item is used; zero timers do not establish current usability. The
package remains uninstalled and native qualification pending. See ADR 048 and
bag-item-cooldowns-verification-2026-11-09.md.

Item lookup projects available cooldown rows only onto exact matching carried
bag locations from the same capture. Matching-location counters precede paging;
equipment/bank quantities and separately requested snapshot evidence stay
independent. Missing-section legacy lookup shape is preserved. Timers remain
historical and do not prove current item usability. See [verification](../../docs/item-cooldown-lookup-verification-2026-11-09.md).

Python ingestion and existing-capture reads now enforce the Rust item stack-count
contract: container rows require positive u32 integers; equipment permits absent
or null counts only. Invalid ingestion fails before writes. Count parity is
verified with 120 actual MCP cases; broader schema equivalence remains separate.

Historical bag cooldown inspection supports item IDs, explicit status, enabled
flags and inclusive timer bounds through bag_cooldown_filter. Numeric bag/slot
ordering and bounded pages preserve full pre-pagination matched/unknown/nonmatch
counts. Missing observations remain null; known empty captures remain empty.
This does not establish current item usability. See [verification](../../docs/bag-cooldown-filter-verification-2026-11-09.md).

Base inventory ingestion now follows the existing Rust structure, item-field,
identity, UTF-8 and bank-container contract, rejecting unsupported fields before
writes. Invalid saved captures are refused without rewriting them. Verified with
241 actual MCP structure cases; full snapshot-schema equivalence remains separate.

Saved-character item discovery uses list_snapshots.inventory_item with an
explicit storage and observed quantity bounds. Unknown storage does not match
zero; newest-per-character selection runs before the quantity predicate.
Shared-bank observations remain separate and are never summed across characters.
The owner-local query needs no simulator catalogue and does not identify the
active player or verify present stock/access. See ADR 049 and [verification](../../docs/inventory-item-discovery-verification-2026-11-09.md).

Shopping/reagent discovery can require all 1..64 requested item quantities within
one explicit saved storage, optionally alongside captured profession filters.
Partial stock across characters, storages or times is never pooled. Material
support does not verify current crafting or transfer eligibility. See [verification](../../docs/inventory-requirements-discovery-verification-2026-11-09.md).

Forever talent observations can retain stable native definition IDs, purchased
ranks, native rank limits and an observed staged-change flag. Boosted active
ranks remain separate and uncertain metadata stays unknown. These active-node
fields alone do not establish inactive zero ranks or a full allocation match. Addon 0.47.0-dev remains uninstalled;
see [verification](../../docs/forever-talent-definitions-verification-2026-11-09.md).

Simulator talent observations now use a bundled pinned nine-class definition
map to compare known purchased ranks with explicit template strings offline.
Known conflicts and staged changes stop simulation; plan-only reports can retain
the diagnostics. Unknown data and full allocation/model coverage remain
unverified. See [ADR-051](../../docs/ADR-051-forever-simulator-talent-observations.md)
and [verification](../../docs/forever-talent-mapping-verification-2026-11-09.md).

Direct MCP talent-template review: call `captured_talent_loadout` with an
explicit saved snapshot and optional `simulator_talent_string`. The nested
`simulator_talent_observation` compares known purchased ranks with pinned
simulator definitions. It is not a native import, simulator run, full allocation
proof or game action. Omit the option for existing behaviour. Unknown metadata
and staging stay explicit; class comes only from that saved capture.
[Contract](../../docs/ADR-052-forever-talent-template-mcp.md) ·
[Offline checks](../../docs/forever-talent-mcp-verification-2026-11-09.md).

Typed loot observations distinguish stable reported currency rows and money
presence. Reported currency IDs/quantities are raw API values before container
conversion; money amounts stay unknown. Neither becomes owned stock or funds.
Item requirements distinguish known non-item slots from genuinely unreadable
slots. [Contract](../../docs/ADR-053-forever-typed-loot-observations.md) ·
[Offline evidence](../../docs/forever-typed-loot-verification-2026-11-09.md).

Spellbook namespace/getter guards now reject restricted required surfaces before
lookup/call and omit unreadable optional level/usability metadata independently.
The typed-loot manual capture fixture again includes spellbook. No schema or
behavioural action is added; uninstalled addon 0.49.0-dev is verified offline only.
[Evidence](../../docs/spellbook-guards-verification-2026-11-09.md).

Saved-loot discovery: `list_snapshots.loot_observation` accepts item ID/readable
quantity bounds, raw reported currency ID/readable quantity bounds, or money
presence with no amount fields. It requires at least one matching displayed
row, respects latest-valid captures before feature predicates, and composes with
character/date/profession/inventory filters. It never credits owned stock or
balances. [Contract](../../docs/ADR-054-saved-loot-discovery.md) ·
[Offline evidence](../../docs/loot-discovery-verification-2026-11-09.md).

Explicit simulator talent preparation: `captured_talent_loadout` with
`prepare_simulator_talents: true` can build simulator digits from a queried
allocation only when every pinned class definition and native maximum matches
and staging is explicitly false. Missing or ambiguous evidence yields no string;
inactive zeros are never guessed. It generates no native import or game action.
Matching reader/importer plus uninstalled addon 0.50.0-dev support are required.
[Contract](../../docs/ADR-055-queried-talent-allocation.md) ·
[Offline evidence](../../docs/talent-allocation-verification-2026-11-09.md).

Explicit queried allocations now reach equipment reviews and both MCP/simulator
comparison. Captured inactive zero ranks can reject template points before the
engine runs. Reports use a queried-definition basis/counters when available;
missing definitions remain unknown and legacy active-only captures retain their
old shape. No new addon package or gameplay action.
[Contract](../../docs/ADR-056-queried-allocation-simulation-checks.md) ·
[Evidence](../../docs/allocation-simulation-checks-verification-2026-11-09.md).

## Saved skill-line queries (2026-11-09)

`list_snapshots.skill_line` filters one recorded skill row by optional ID,
category, base rank, cap and points to its observed cap. `inspect_snapshot` with
`section: "skill_lines"` accepts `skill_line_filter: {filter: {...}, offset: 0,
limit: 20}` for paged results (maximum 50). At least one predicate is required.
Missing categories remain unknown; bonuses are preserved without adding them
to base rank. Missing sections and known empty sections stay distinct.
Latest-per-character selection, when requested, precedes feature filtering.
No complete coverage, learning state, training eligibility, simulator change or
live action is inferred. Offline source only; installation/testing deferred.

## Skill capture stability (2026-11-09)

The source collector guards the skill API namespace and getters before use,
then requires two matching bounded projections of reported rows. Changes to
row order/header status, IDs, base ranks, caps or emitted optional metadata
make the section unavailable. Required read errors remain section-local through
Core. No skill headers are expanded, no training occurs, and agreement between
two observations does not prove atomic state, complete skills or native support.
Development addon 0.51.0-dev is packaged and verified offline, not installed.

## Historical saved reset projection (2026-11-09)

With `saved_reset_projection` enabled, supply `saved_reset_reference_at` to
classify saved instance or world boss reset slots at or before that time,
after it, or unknown. This compares historical timestamps only. Missing
sections return a null classification.
The classification highlights the earliest projected reset strictly after
the reference and lists every slot with that timestamp. The next reset is
null when no later projection exists.
`upcoming_projected_resets` lists every later timestamp in chronological
order, with slots sharing a timestamp grouped together.
Use `saved_reset_reference_at` with `character_summary` to merge the
instance and world boss schedules from the same Forever capture. Entries
retain their section, saved slot and name. Coverage is separate for each
section, including unknown timer counts and missing capture data.
Dungeon and raid entries also retain the captured difficulty, lock and
extension flags, raid flag and maximum player count. These describe the
saved row and do not establish current entry eligibility.
Add `saved_reset_through` to limit that schedule to an inclusive end time.
The reference is still exclusive. Coverage counts describe the whole capture,
even when the requested window contains no projected resets.
With a reference time, `saved_reset_kind` limits the schedule to `dungeon`,
`raid` or `world_boss`. Instance types use the captured raid flag, and
coverage still describes both captured sections.
Use `saved_reset_instance_filter` to limit a schedule to instances and
match recorded `difficulty_id`, `locked`, `extended` or `max_players` on
the same row. An empty filter includes both dungeons and raids. Combine
it with an instance kind or time window. Coverage still describes the
whole capture, and a match leaves current lockout status unverified.
`matching_reset_window_counts` counts rows after activity filters by
section. It separates resets at or before the reference, inside the
window, after the window, and unknown. A missing section gives null,
while a captured section with no matching rows gives zero counts.
Set `saved_reset_maximum_update_age_seconds` with the reference time to
keep sections whose cache age at capture was at or below the limit.
`cache_age_filter_matches` reports the result per section and leaves
missing sections unknown. The filter uses the recorded cache age, so it
does not establish present-day freshness.

`inspect_snapshot` with `section: "saved_instances"` or `"saved_world_bosses"`
accepts `saved_reset_projection: true`. Raw `data` remains unchanged and the
new sibling report projects capture timestamp plus nonnegative reported reset
seconds. Negative or out-of-range results remain unknown. Cache update age is
reported separately, without shifting the projected timestamp. Missing sections
remain unknown; known empty sections yield an empty report. This is historical
arithmetic, not a current lockout, extension outcome, entry or loot eligibility
check. No game/API/addon change or installation is involved.

## Final gear encounter context (2026-11-09)

Saved-journal rankings and the unified owned-gear evaluator now retain checked
`encounter_contexts` for visited encounters: index, normalised weight, engine and
simulation-profile digests, source fingerprints, numeric encounter settings and
nullable baseline identity/metrics. `unvisited_encounter_indices` records missing
contexts. Ranked-loadout evidence includes its metric and baseline metric value,
even if top-N excludes the baseline loadout. No raw templates, names, file paths
or private iteration samples are added. Ranking, work budgets and completeness
flags are unchanged; no full BiS, native/model accuracy or statistical proof is
implied. [Contract](../../docs/ADR-059-ranked-encounter-context.md).

## Useful interrupted gear rankings (2026-11-09)

An incomplete encounter page may expose `completed_candidate_ranking` only when
all requested encounters were visited and each has its equipped baseline.
Only review IDs with results in every encounter are ranked, with original
weights and the selected weighted/worst-encounter objective. DPS requires positive
baseline DPS; native role metrics retain valid zero baselines. Counts include
the equipped baseline, and top-N truncation is explicit. Missing results are
never zero-filled or reweighted. The final saved-journal/evaluator ranking can
include these saved loadouts under the explicit completed-candidate scope while
stopping at the same incomplete page. Completed-window counts and full-coverage,
statistical/global/BiS flags remain unverified. No extra simulation or game action
is introduced; checkpoints remain resumable.

## Visible gossip dialogue options (2026-11-09)

The source-only `gossip_options` section records up to128 choices from an already
visible gossip panel, with display label and optional reported option/spell IDs,
raw status/flags and order index. `inspect_snapshot` with section gossip_options
returns the stored choices; character_summary adds observation counts. No option
selection, refresh/close, input/code text, confirmation text or rewards are read
or executed. Required restricted/unreadable/unstable data makes the section
unavailable; optional unreadable metadata stays omitted. Missing IDs remain
unknown. Coverage is this visible panel only, with no NPC identity, present
eligibility or complete dialogue/world proof. Labels are owner-local untrusted
display data, never instructions. Addon0.52.0-dev and compatible updated readers
are required together for future rollout; package remains uninstalled and native
testing deferred.

## Saved dialogue discovery (2026-11-09)

`list_snapshots.gossip_option` takes a nonempty same-row predicate with optional
name, option_id, spell_id, reported_status and reported_flags. Name is an exact
case-insensitive display-label match after trimming the query; it is not an NPC
name or fuzzy service search. Missing requested optional metadata remains
unknown, while a known false predicate excludes a row. Only positive matches
select captures. Results include counts and the first20 numeric slots, with
explicit truncation. Existing owner/character/date/inventory filters compose;
latest-per-character, when requested, selects newest valid captures before this
predicate. A newer empty/missing/nonmatching view suppresses an older match.
No current availability, NPC attribution or dialogue action is inferred. This
query changes no addon/schema/package or deployed installation.

Pass `gossip_option_changes` to `compare_snapshots` to compare exact visible
dialogue rows and their duplicate counts. Capture slot order is ignored, while
an explicit reported order index remains part of the observed row. A missing
panel leaves changes unknown. This compares captured descriptions without
establishing the same NPC, dialogue, or current availability.

## Saved selected-recipe batch preparation

Set `estimate_output_bag_space: true` in `review_forever_recipe` to
compare each requested recipe's minimum and maximum base output with
the captured carried bags. An explicit compatible inventory capture
supplies both stack limits and free slots. Each recipe is evaluated
independently against those bags before crafting. The estimate excludes
bonus output and slots freed by consuming reagents. Hypothetical merges
and actual placement remain unverified. Missing output ranges give null.

Batch reviews also report `combined_output_bag_space_projection`. The
MCP combines quantities for the same item before estimating stacks and
uses the captured free slots once for the whole batch. An unobserved
output item, range or required stack limit leaves the total unknown.
This assumes the outputs remain in the bags together after crafting.

Use `compare_snapshots.selected_recipe_preparation_crafts` with 1 to 1,000
crafts to review the saved preparation on each side. The review uses each
capture's own selected recipe, bags, visible banks, observed charges and
crafting conditions. It reports missing storage and identifies when the
selected recipe IDs differ. An absent recipe gives a null side. The result
describes captured evidence and leaves crafting eligibility unverified.

## Recipe item-capacity shortlist (2026-11-09)

Set `order: reported_cooldown_remaining_asc` on `search_forever_recipes` to put
shorter reported timers first. Zero sorts first and unknown timers sort last.
Equal timers sort by numeric recipe ID. This uses the original recipe capture's
timer, even when newer inventory supplies materials. Use `cooldown_filter` to
select an observed availability state separately, including reported charges.

Set `order: reported_skill_ups_desc` on `search_forever_recipes` to put higher
reported skill-up counts first. The search uses the saved view for the same
profession and recipe ID. Zero sorts ahead of unknown. Equal counts sort by
numeric recipe ID. Filters apply before sorting and paging. The result describes
captured counts, with actual skill gains and craftability unverified.

`search_forever_recipes` accepts order observed_item_capacity_desc together with
material_filter (explicit bags/all_observed_storage and minimum_crafts). Existing
view filters such as learned/can_skill_up and original cooldown filters compose.
All matching rows are sorted before paging by finite supported_crafts descending,
then numeric recipe ID. No-required-item recipes have no finite item capacity
and sort last; null is not treated as infinity or actual craftability. Each
capacity independently reuses the observed stock, so capacities must not be
summed or treated as an allocated progression plan. Charges, conditions, skill
chance, yields and native eligibility do not change the ordering. An explicit
newer inventory may change the order while recipe/view/cooldown provenance stays
original. Omitted/null/recipe_id ordering preserves existing default output.
No addon/schema/deployment or crafting action is introduced.

## Equipped bag item observations (2026-11-09)

Non-backpack positive-capacity bag rows may include equipped_item with item_id
and the API-reported inventory_slot. Required capacity getters are guarded;
optional identity failures omit only identity. Two matching mapping/item reads
must agree with captured capacity/free slots and known family, with final getter
identity checks. Ambiguous duplicate inventory-slot mappings lose both identities.
Missing identity never means an empty bag slot or no quiver. Raw bag inspection
and equipped_bag_item_observations summary expose numeric IDs, without stock
credit, NPC data or gameplay actions. This is a prerequisite for future captured
quiver checks; simulator quiver/ammunition options remain unverified and unchanged.
Addon0.53.0-dev and compatible readers are required together for future rollout.
The package remains uninstalled and native qualification stays deferred.

## Captured quiver reference diagnostics (2026-11-09)

Equipment reviews now project captured_equipped_bag_items as numeric bag/item/
inventory-slot rows, null for missing capacity data and empty for no observed
identities. Standalone simulation preflight and saved-window preparation validate
these fragments and the pinned Hunter quiver enum. Unsupported numeric enum IDs
reject rather than silently falling back to a zero reference effect. Supported
selections and request bodies remain unchanged. Diagnostic comparison reports
an observed item match, not-observed-in-partial-capture or unverified none, with
pinned reference haste values and an explicit below-observed-reference indicator.
These do not prove an applied native bonus or absence of another quiver. Raw
unknown bag IDs are counted as unmapped, not classified as non-quivers. Numeric
observations must agree across prepared groups; final encounter contexts retain
the recomputed diagnostics. Full identity/bonus/ammunition proof remains false.
No quiver optimisation, automatic template change, addon change or live action.

## Hunter ammunition model boundary (2026-11-09)

Hunter review/preparation workflows now accept only the pinned engine's nine
arrow enum names/integer IDs or AmmoNone. Unknown numeric values reject before
engine/cache use instead of falling through a missing reference to zero damage.
Supported requests stay unchanged, and protobuf null/missing follows the existing
none default without proving an unequipped slot. The existing historically named
hunter_quiver_observation context now includes template_ammo_reference with item
ID, bonus_ranged_dps and pinned_template_selection_only basis. This DPS is the
engine's ranged-weapon parameter, not predicted player DPS gain. Identity,
availability and native-effect flags stay false even if a captured item ID
coincides. Standalone CLI and final saved-journal/evaluator context retain the
report. No ammo capture, bag-stock/consumption claim, template correction,
optimisation, addon change or live action is introduced.

## Hunter pet template boundary (2026-11-09)

Request preflight and prepared merging validate pinned pet type, uptime and all
10 pet attack-speed enum selections. Missing/null speed selects the none default;
unknown numbers, malformed types and duplicate aliases reject. Final context
includes template_pet_reference: type, uptime and attack-speed enum name, with
identity, speed, training, happiness and native-effect verification false.
Active simulated pets still require an explicitly captured level of 60. No
actual pet speed, family/training match or native model qualification is implied.
See docs/ADR-067-hunter-pet-template-options.md. Offline only; live testing deferred.

## Encounter improvement floor (2026-11-09)

Owned evaluation and saved-journal ranking accept --minimum-encounter-improvement.
Use 0 to reject simulated point-estimate losses in any requested encounter before
top selection. Units follow ranking_score_unit; baseline also faces the floor.
Positive floors may return no eligible sets even when all simulations completed.
Partial pages require observations against baseline in every requested encounter.
Excluded counts are candidate instances, including duplicates across pages, not
unique gear or incomplete candidates. Default is unconstrained. This adds no
simulation runs or native/BiS proof. See docs/ADR-068-encounter-improvement-floor.md.

## Player trade offer (2026-11-09)

Forever manual exports can observe the player's six normal outgoing item slots
while TradeFrame is already shown. Inspect player_trade_offer or read
player_trade_offer_observation in character_summary. Rows contain stable numeric
item IDs/quantities or unreadable_or_empty; that status never proves empty.
Hidden/unavailable panels omit the section. No target offer/identity, money,
acceptance, enchant slot or trade action is captured or invoked. Offers never
increase owned stock or establish a completed/current trade. Offline source only;
native testing stays deferred. See docs/ADR-069-player-trade-offer-observation.md.

Pass `trade_offer_requirements` to `inspect_snapshot` on
`player_trade_offer` to check an expected outgoing offer. Supply one to 64
distinct positive `{item_id, quantity}` rows. The review totals the readable
slots by base item ID and reports observed shortfalls, excess and unexpected
items. Excess or unexpected items prove a mismatch even with unreadable slots.
So does having more distinct item IDs with shortfalls than unreadable slots,
since each normal slot can account for at most one item ID.
The review also reports `minimum_additional_snapshot_slots` and
`snapshot_slots_can_fit_shortfalls` using the snapshot format's u32 quantity
ceiling per slot. Each item row includes its own minimum slot bound.
`exact_match_possible_in_snapshot_format` requires room for all shortfalls
and no readable excess or unexpected items. It describes what could fit
in the saved format, independently of the match verdict and native stack limits.
Otherwise exact match requires all six slots to be readable item rows. A
partial capture can leave the match unknown. A missing offer gives a null
review. These checks describe the saved outgoing quantities only.
Set `trade_offer_compare_carried: true` with those requirements to report
the same capture's carried quantity and shortfall for each expected item.
The comparison keeps offered quantities and bag quantities separate because
their overlap is unknown. Missing bags leave the carried comparison unknown.

## Personal PvP progression (2026-11-09)

Use `pvp_target_rank` in `inspect_snapshot` on `player_pvp_progression`
to compare a rank with the saved current rank and reported caps. The
report keeps distances from the captured total threshold table and the
reported next-rank threshold separate. Missing thresholds leave the
corresponding distance unknown. The comparison leaves rank eligibility
unverified.

Manual Forever exports can capture the numeric rank progression reported for
faction 2800. Inspect player_pvp_progression or read
player_pvp_progression_observation from character_summary. Rank, points and
next-rank threshold are required; max rank and current/previous weekly caps are
optional stable observations. Zero thresholds and points above a threshold leave
remaining points unknown. No season binding, rank-up/reward eligibility,
scoreboard/player identities or queue actions are inferred or added. Native
qualification stays deferred. See docs/ADR-070-player-pvp-progression.md.

## PvP cap-distance planning (2026-11-09)

Optional captured total_points_by_rank contains at most four referenced current/
cap rank thresholds. Direct `inspect_snapshot` of `player_pvp_progression`
includes the same `player_pvp_progression_observation` as character summary,
alongside the raw captured data. An unavailable Forever observation is null.
The observation adds reported_total_rank_points and
remaining_points_to_reported_caps only when totals are available. Distances use
captured total thresholds plus within-rank points, floored at zero. Missing
current/target thresholds or zero cap totals remain unknown. No rank interpolation,
season binding, earned-point delta, reward eligibility or present cap guarantee.
Two projections use at most eight deduplicated lookup calls. Legacy captures and
passive action boundaries are preserved. See docs/ADR-071-pvp-cap-distance.md.

## Personal group context (2026-11-09)

Manual Forever exports can retain separate explicit home/instance group flags,
raw reported member counts and optional own-player leadership. Inspect
group_context or read group_context_observation in character_summary. Missing
categories remain unavailable, never solo; raw counts are not a full roster.
Two stable projections do not establish the same group identity or current
membership. No member names/GUIDs, roster, chat, invites, role changes, votes or
social actions are captured or invoked. Native testing stays deferred.
With `group_context_changes`, saved comparisons report whether the group,
raid and own-player leadership flags differ for each category. A missing
flag on either side gives null. Changes describe the two saved readings.
See docs/ADR-072-personal-group-context.md.

## Larger owned candidate pools (2026-11-09)

Ranker, window collector and full owned evaluator accept --candidate-scan-limit
1..4096, default256. Pools over the selected budget reject before enumeration;
no scanned prefix is treated as complete. Modifier-aware duplicate collapse,
physical-copy legality and all other time/draft/payload limits remain. The reported
candidate_scan_limit is operational, not a candidate filter. Within unchanged
sources, changing only the budget preserves a successfully scanned pool's scope
and collection binding. Existing source fingerprints still apply. This does not
verify complete ownership, native legality or BiS. See docs/ADR-073-owned-candidate-scan-budget.md.

## Repeated-window page reuse (2026-11-09)

One collection invocation can reuse owned-item pages across windows in a temporary
32 MiB/1,024-entry serialized cache. Fresh stats/baseline reads and source-binding
checks remain; cached pages still pass normal evidence and pagination checks.
Only numeric candidate_page_cache statistics are reported. Full caches fall back
to fresh MCP requests, and a new invocation starts empty. No page cache files,
new public cache input, lower simulation precision or broader BiS claim. See
docs/ADR-074-invocation-owned-page-reuse.md.


## Saved carried-item cooldown comparison

Pass `bag_cooldown_reference_at` alongside `bag_cooldown_filter` in
`inspect_snapshot` to project the end of each enabled cooldown from the
capture time and reported remaining seconds. The comparison is inclusive
at the reference time. Disabled and missing timers leave the projection unknown, and a projected end
leaves current item usability unverified.
Set `bag_cooldown_filter.projected_end_by` for an inclusive deadline.
The filter runs before paging. A disabled or missing timer cannot meet
the deadline and is counted as unknown when the other predicates match.
`find_snapshot_item` also accepts `bag_cooldown_reference_at` for
Forever captures. Each returned bag location uses its own capture time
and enabled timer. Equipment and bank locations have no projection.
Additional snapshots are projected separately.
`bag_cooldown_quantity_projection` totals copies across all matching
carried locations before paging. It distinguishes ends at or before the
reference, later ends, and unknown ends. Missing bags leave the totals
unknown. Disabled and missing timers contribute to the unknown quantity.
Set `bag_cooldown_target_quantity` with that reference to report the
shortfall from known timers and the earliest projected end at which
known timers cover the target. Unknown timers can leave coverage at the
reference inconclusive. The estimate uses all matching carried copies,
even when the location page is empty.

Use `compare_snapshots.bag_cooldown_item_id` to compare two saved readings for
one base item ID. Each side counts reported, unavailable and enabled locations
and shows the minimum enabled timer, including explicit zero readings. Missing
cooldown coverage gives a null side. An observed section with no match gives
zero matching locations. This does not establish physical item continuity or
current item usability.

## Saved durability threshold crossings

Saved durability comparisons include `item_context_by_shared_slot` for
slots with readings on both sides. Different recorded base item IDs show
a change in the item observed at that slot. Equal IDs leave item-instance
continuity unverified. A missing item reading leaves the ID match unknown.

Set `compare_snapshots.equipment_durability_max_percent` to a whole
percentage from 0 to 100. The comparison lists matching slots on each side
and reports threshold crossings only for slots readable in both captures.
A missing section gives unknown crossings. Changes describe saved slot
readings and leave item continuity and repair events unverified.

## Saved dungeon and raid discovery without a name

Use `list_snapshots.saved_instance` with at least one criterion. The name is
optional, so a search can find any recorded locked raid or any observed
dungeon in a projected reset window. `max_players` matches the exact positive
group-size cap reported on the same row. Results describe the saved cache.

## Saved world-boss discovery without a name

Use `list_snapshots.saved_world_boss` with at least one criterion. Omit the
name to find recorded world bosses by reset duration, projected reset window
or cache age at capture. Negative timers cannot satisfy a reset bound. A
cache-age criterion alone can return rows whose reset duration is unknown.

## Saved timed-quest discovery

Use `list_snapshots.quest_max_remaining_seconds` or
`quest_projected_expiry_by` to find captures with matching timed quests.
Other quest filters must match the same quest. Expiry is projected from the
capture timestamp and recorded duration. Unknown timers cannot match.
Each matching capture includes its shortest matching timer, with ties broken
by numeric quest ID, and the corresponding projected expiry when available.

## Saved temporary weapon enchant expiry

Set `list_snapshots.weapon_enchant.projected_expiry_by` to an RFC 3339
deadline. A match requires a temporary enchant and a timer reported on the
same hand and row. The inclusive deadline uses the capture timestamp and
recorded milliseconds. The result describes historical observations.
Set `weapon_enchant_reference_at` in `inspect_snapshot` on
`weapon_enchants` to see projected temporary enchant expiries per hand.
The report compares each known expiry with the reference time. Missing
timers and unobserved hands stay unknown. Current enchants remain unverified.
Saved weapon enchant comparisons group temporary rows by hand and exact
enchant ID and compare sorted projected expiry timestamps. A match
requires equal row counts and known timers on both sides. Matching
timestamps can explain a changed countdown. They leave enchant events
and item continuity unverified.

## Saved trainer affordability after a reserve

Set `list_snapshots.trainer_balance_reserve` with
`trainer_affordable_only: true` to preserve copper at regular trainers or
training points at pet trainers. Each matching lesson must individually
fit the spendable balance from the same capture. The result reports that
balance and its unit. Lesson counts describe individual prices and do not
establish that all matching lessons fit a single budget.

## Explicit character resolution with tied captures

Set `resolve_character.require_unique_latest` with an explicit character to
require a single newest matching capture. If the two most recent matching
captures share a timestamp, the result is `ambiguous_latest_snapshot` with
a null selection. Time, exact client and required sections narrow the matches
before the tie check. Active character identity remains unavailable.

Set `resolve_character.require_latest_coverage` with `required_sections` to
check the newest capture within the requested client and time scope.
Missing sections produce a null selection and per-section capture statuses.
With `require_unique_latest` also enabled, timestamp ties are evaluated
before coverage. Without latest coverage mode, required sections filter
candidates before selection.

## Saved inventory shortfall discovery

Set `list_snapshots.inventory_requirements_shortfall_only` with
`inventory_requirements` to find captures with at least one shortfall in the
selected storage. Each match reports quantities and shortfalls per requested
item. An observed empty section can match. A missing section has unknown
quantities and is excluded. Results describe the saved observations.

Set `inventory_requirements_bank_plan: true` on a Forever bag shortfall
search to show suggested quantities from the character and account banks
recorded in the same capture. The plan reports remaining shortfalls and
whether each bank was observed. Withdrawal eligibility remains unverified.

Use `inspect_snapshot` on `bags` with `bag_restock_requirements` to review
one to 64 distinct positive item quantities on a selected Forever capture.
The review includes suggested bank source locations and supports
`bag_restock_location_offset`. Missing bags leave the plan unknown. Source
locations are historical and withdrawal eligibility remains unverified.

Set `bag_restock_merchant_snapshot_id` to a saved visit for the same
character and exact client to quote quantities still short after observed
bank withdrawals. Affordability uses the inventory capture's wallet. Missing
bags or merchant stock leave the quote unknown. Purchases remain with the player.

Add `bag_restock_wallet_reserve_copper` to the merchant quote to check its
copper-only bundle cost against the inventory wallet after setting copper
aside. The result can identify a known shortfall from partial prices. A
positive affordability result requires a complete quote and observed wallet.

Set `bag_restock_source_snapshot_id` to a compatible saved capture to see
recorded drop, vendor and quest reward references for quantities still short
after observed bank withdrawals. This is independent of merchant quoting.
Missing source observations remain unknown, and references describe saved
data without verifying current acquisition eligibility.

Set `bag_restock_catalogue_sources: true` to look up the same residual
shortfalls in the verified local catalogue. The result has separate source
provenance and distinguishes missing catalogue items from missing source
metadata. The lookup works offline.

Set `bag_restock_pending_mail: true` to show known pending item attachments
for quantities still short after observed bank withdrawals. The inbox
observation comes from the same capture and excludes currency attachments.
Pending items stay separate from usable stock and purchase calculations.

With pending mail enabled, set `bag_restock_mail_snapshot_id` to use a
separate saved inbox for the same character and exact client. The response
identifies the inbox capture and timestamp. Inventory, bank quantities, and
merchant affordability still use their original captures.

Supply `bag_restock_stack_limits` as distinct `{item_id, maximum_stack}`
rows to estimate new stacks for the requested bag shortfall. The estimate
compares only observed general-purpose free slots. It excludes merging into
existing stacks, specialty bag fit and merchant bundle surplus. Missing
limits or bag capacity leave the relevant totals or comparison unknown.

For supplied limits, the estimate also reports the largest matching carried
stack, when observed. A limit smaller than that stack leaves the slot
estimate unknown. Observed quantities do not establish the native stack cap.

With both stack limits and a saved merchant quote requested,
`bag_restock_quoted_bundle_stack_space` includes extra units from whole
copper-only bundles. A bank-covered shortfall needs no quote. Unquoted
positive residuals leave their slot estimate and the total unknown.

Both estimates also show a separate scenario that assumes incoming items
can merge into all recorded stacks of the same base ID. This uses supplied
limits and observed counts. Item variants and native merge compatibility
remain unverified. The original new-stack estimate is still reported.

Restocking merchant quotes include per-item counts of sold-out,
extended-cost, unlimited copper-only and finite copper-only offer slots.
Finite stock quantities remain unverified, so these rows explain a quote gap
without treating finite offers as unlimited.

Set `bag_restock_loot_snapshot_id` to compare positive residual shortfalls
with readable loot in a compatible saved capture. The report identifies the
loot capture and shows how much might be covered if those items were acquired.
Missing loot or no known positive residual yields an unknown or inapplicable
review. Loot observations do not reduce the bank plan or merchant quote.

Set `bag_restock_compare_merchant_snapshot_ids` to one to four distinct
saved visits for the same character and exact client. The review compares
each visit independently against the same post-bank shortfall and inventory
wallet. It reports every tied lowest complete quote. Partial quotes and
unobserved offers remain visible without pooling stock across visits.

Enable `bag_restock_split_merchant_plan` with the visit comparison to
choose the cheapest complete bundle quote for each item, breaking item
price ties by the requested visit order. Each item uses one visit. The
plan excludes travel costs and uses the saved inventory wallet. An
unquoted item leaves the complete plan price unknown.

With split planning enabled, set `bag_restock_cost_per_merchant_visit_copper`
to compare complete shopping plans including an assumed cost per used visit.
Ties prefer fewer visits. The result uses one complete bundle quote per item
and leaves the travel-adjusted plan unknown if no complete plan can be priced.
The visit cost is supplied by the caller.

Use `bag_restock_merchant_visit_costs_copper` for different assumed visit
costs. Supply one cost per merchant snapshot ID in the same order. This
option requires split planning and is mutually exclusive with the uniform
visit cost.

Set `bag_restock_max_merchant_visits` from zero to four with split planning
to limit the travel-adjusted plan. If no visit costs are supplied, the
planner assumes zero travel cost. A cap of zero can cover an already
satisfied request. The unrestricted split plan remains available separately.

Complete travel-adjusted plans report the captured inventory wallet and the
copper remaining or short after purchases and assumed visit costs. Missing
wallet data leaves both amounts unknown.

`bag_restock_wallet_reserve_copper` also works with split planning. The
travel-adjusted result checks whether purchases and assumed visit costs
fit after reserving copper. If no visit costs are supplied, it assumes zero.

A complete travel-adjusted plan includes `merchant_visit_checklist`, grouped
by selected saved visit in request order. Each group lists historical offer
slots, bundle quantities, purchase subtotal and assumed visit cost.

The checklist also shows cumulative cost and the inventory wallet remainder
or shortfall at each stop, using the displayed request order. This order is
a budgeting assumption. A missing wallet leaves those balance fields unknown.

Compared merchant visits and checklist stops include the captured player map
ID when available. Stops also identify the merchant capture time. The map
describes the player's saved location during the visit.

Use `list_snapshots.player_map_name` to match the saved player map label
case-insensitively. This can be combined with merchant item searches or
other saved capture filters. Missing map labels cannot match.

Set `list_snapshots.quest_on_captured_map: true` to find saved quests whose
next waypoint shares the player map recorded in that capture. You can also
filter by the captured map name, quest readiness, or timer. Captures with a
missing player map or no matching recorded waypoint are excluded.

Local-map results also show up to 20 matching quest IDs, titles, and
recorded readiness states in numeric quest ID order. The match count covers
all matching quests. The response marks a shortened shortlist and omits
labels over 256 bytes.

Set `local_quest_offset` with the local-map search to skip matching quests
within each capture. The page holds up to 20 rows, with a separate flag
for more rows after the page. Capture pagination and quest totals are independent.

Set `local_quest_soonest_first: true` to sort each local shortlist by known
remaining time before paging. Ties use numeric quest ID. Quests without a
recorded timer appear last. This order describes the saved readings.

Use `list_snapshots.quest_title` to find quests by full saved title,
ignoring case and surrounding whitespace. Results include up to 20 matching
quest IDs and recorded readiness per capture. Title, ID, waypoint, readiness
and timer criteria apply to the same quest.

Saved quest searches accept `quest_objective_kind`,
`quest_objective_finished`, and inclusive
`quest_objective_min_remaining` / `quest_objective_max_remaining`.
All objective predicates apply to one objective on the matching quest.
Quantity bounds require known objective counts. Results include a bounded
shortlist of matching quests.

Use `list_snapshots.quest_reward_item_id` to find quests with the requested
item in recorded fixed rewards or choices. The shortlist identifies which
kind matched. Other quest criteria apply to the same quest. Reward
eligibility remains unverified.

Use `list_snapshots.quest_reward_currency_id` to find saved fixed or choice
currency rewards. When combined with an item reward filter, both rewards
must be recorded on the same quest. Currency balances do not count as
quest rewards, and reward eligibility remains unverified.

Use `list_snapshots.quest_reward_renown_faction_id` to find quests with a
recorded renown reward for that faction. This can be combined with item or
currency reward filters on the same quest. Faction standing and paragon
progress do not count as quest rewards.

Use `list_snapshots.quest_difficulty` with `elite` or `suggested_group`,
and optionally `quest_max_suggested_group_size`, to shortlist group
content. Suggested group requires a recorded size above one. The size cap
requires a recorded size and matches inclusively. These are saved game hints.

Saved quest searches also accept `quest_min_reward_xp`,
`quest_max_required_copper` and `quest_within_captured_wallet`. Each
active predicate requires a known value. Affordability checks each quest
against the same capture's wallet and does not budget multiple quests.

Set `local_quest_highest_xp_first: true` to order each local shortlist by
recorded XP before paging. Unknown rewards appear last and ties use numeric
quest ID. This option is mutually exclusive with timer ordering.

Use `list_snapshots.pet_species_name` to find exact public species labels
ignoring case and surrounding whitespace. The result includes up to 20
matching species IDs with their observed owned and unowned row counts.
Combining this with `pet_species` requires the name and row bounds to
match the same species. Results describe the saved filtered view.

`review_forever_owned_gear.coverage` reports
`observed_container_occupied_slots` for captured bags, character bank and
account bank. Counts describe the full saved section before filters and
paging. A zero count means the capture recorded no item rows for that
container ID. Unlisted container IDs have unknown coverage. These counts
do not establish whether a bag was equipped, its capacity, or complete ownership.

Use `review_forever_owned_gear.container_id` to limit results to a recorded bag
or bank container before paging. It can be combined with `storage` and item
filters. The coverage counts still describe the full capture, so a missing
container can be distinguished from a recorded container with no item rows.

Set `find_snapshot_item.include_container_quantities` to sum the requested
item's recorded quantities by bag and bank container before paging, without
a catalogue. Each snapshot reports its own totals. A missing storage section
returns null, a recorded section with no container rows returns an empty
map, and a recorded container with no copies returns zero.

Set `compare_snapshots.item_container_changes` with `item_id` to compare
each recorded container across two compatible captures of the same character.
Containers need recorded quantities on both sides for a signed delta.
The report distinguishes missing storage from a captured section with no
container rows. Quantity changes describe observations and do not establish
that an item was moved between containers.

Set `list_snapshots.inventory_item.container_id` to apply the quantity
bounds to a single recorded bag or bank tab within the selected storage.
An absent container cannot match a zero-quantity query. Use
`latest_per_character` to check each character's latest capture in the
requested time window.

`list_snapshots.inventory_requirements.container_id` scopes a full material
list to one recorded bag or bank tab. Shortfall searches can use the same
scope. Missing containers do not qualify for either search. Bank withdrawal
plans require a storage-wide bag search.

`inspect_snapshot.quest_inventory_requirements.container_id` can scope a
quest materials checklist to a recorded bag or bank tab. Supply the item
IDs and quantities explicitly, since the saved objective counts do not
identify required items. Quest bank withdrawal plans require a
storage-wide bag review.

`list_snapshots.collected_mount_name` searches recorded collected mounts
by exact public name, ignoring case and surrounding spaces. Results show
up to 20 matching mount IDs and their saved flags. Combine it with
`collected_mount` to require the name, ID and any flags on the same mount.
Use `collected_mount_spell_id` in `list_snapshots` to find a mount by its
saved spell ID. A mount name or exact mount ID supplied with it must
match the same row. In `inspect_snapshot`, `mount_spell_id` filters
`mount_collection` and can be combined with recorded usability or
favourite flags. Results describe the saved collection.
Set `action_bar_mount_audit: true` on `action_bar_observation` to match
retained spell slots to collected mounts by exact spell ID in the same
capture. The audit only covers captured spell rows and leaves current
mount usability and complete action-bar coverage unverified.
Set `action_bar_toy_audit: true` on the same section to match retained
item slots to exact item IDs in the same capture's filtered toy journal.
Matches preserve the recorded collected and favourite flags. Missing
matches leave journal coverage and current toy usability unverified.
Both audits include `observed_hotkeys_by_slot` when button hotkeys were
captured. An empty list means a button was observed with no reported
keys. A null value means the slot or hotkey section was unobserved.

Set `inspect_snapshot.toy_name` with the `toy_collection_view` section
to look up an exact public toy name, ignoring case and surrounding spaces.
It can be combined with the recorded collection and favourite flags.
Matches cover only the saved filtered view.

`list_snapshots.toy_name` searches across saved Forever toy journal views
by exact name, ignoring case and surrounding spaces. It returns up to 20
matching item IDs per capture with recorded collection and favourite flags.
Combine it with `toy_in_view` to require the name, ID and flags on the same
row.

`list_snapshots.spellbook_spell.name` finds captures containing an exact
saved spell name, ignoring case and surrounding spaces. Optional spell ID,
resource and usability conditions apply to the same slot. Results contain
the matched slot IDs and recorded metadata. They describe the saved visible
spellbook and do not establish current casting eligibility.

`inspect_snapshot.action_bar_spell_name` finds spell shortcuts in the
saved `action_bar_observation` section by exact name. It uses reported spell
IDs from the same capture's spellbook. All rows for a reported ID must
agree with the name. Base-ID aliases alone do not qualify. Combine it
with a slot, spell ID or hotkey to narrow the results.

`compare_snapshots.action_bar_spell_id` compares placement of one exact
reported spell ID across compatible saved Forever action bars. A gain or
loss requires comparable observations of the same slot. Missing slots and
spell rows with unreadable IDs stay unknown. These differences describe
saved placement and do not establish that a shortcut was moved.

`compare_snapshots.action_bar_item_id` provides the same placement
comparison for recorded item shortcuts. You can request both item and
spell placement in one comparison. Each matches its own action kind,
even when the numeric IDs coincide.

`compare_snapshots.merchant_offer_item_id` compares recorded copper prices
for the same item and bundle size across two saved Forever visits. It shows
unchanged prices as well as changes, and distinguishes missing panels,
unlisted bundles and duplicate offers. A delta requires one offer on each
side. Merchant identity and current prices remain unverified.

`compare_snapshots.merchant_offer_currency_id` compares recorded currency
bundles in the same way. Item and currency selectors can be used together.
Offers remain separate even when the IDs and bundle sizes match.

If either unique offer reports an extended cost, the bundle comparison
includes the recorded item and currency components on both sides. A
copper-only offer has an empty component list. Missing, ambiguous or
unreadable components remain unknown. Component ordering does not count
as a change.

`compare_snapshots.trainer_lesson_name` compares one saved lesson name
across two Forever trainer panels, ignoring case and surrounding spaces.
It shows observed status and price by rank, including unchanged rows.
Duplicate ranks are ambiguous. A price delta needs unique rows with
recorded prices from the same trainer kind. Copper and training points
are reported separately. Current price and trainer identity remain unknown.

Named lesson comparisons also show recorded level, skill and ability
requirements where available. Each change flag requires that field on
both sides and the same trainer kind. Reordering ability rows does not
count as a change.

`compare_snapshots.flight_destination_node_id` shows a saved destination's
state, fare and affordability on each side. It uses each capture's own
wallet and reports deltas only for the same map and origin. Missing
panels, ambiguous maps and unlisted destinations remain distinct.

When a visit includes a captured flight preview, the destination comparison
shows its ordered node IDs. The path-change flag requires both previews
and the same map and origin. One missing preview or differing origins
leave the change unknown.

`list_snapshots.flight_destination_name` searches exact saved destination
names, ignoring case and surrounding spaces. A name alone must identify
one reachable destination in the captured panel. Supply
`flight_destination_node_id` too when names collide. Fare, origin and
wallet filters apply to the same saved route.

`inspect_snapshot.flight_route_search.destination_name` accepts an exact
name from the selected saved flight panels. Repeated observations of the
same node ID agree. Different reachable node IDs with the same name need
an explicit `destination_node_id`, even if only one fits the fare budget.
The route search still uses recorded reachable legs with known fares.

When the search runs and finds no qualifying route, `saved_path_evidence`
reports whether the selected captures contain a reachable path within the
leg limit and whether any such path has all fares recorded before budget
filters. Unknown fares are never treated as zero.

The evidence also reports the lowest fully recorded fare across paths
within the leg limit. When a fare ceiling applies, it shows the effective
ceiling and how much the lowest fare exceeds it. This diagnostic uses the
lowest fare even when the route search prefers fewer legs.

`list_snapshots.reputation_rank.faction_name` accepts an exact standard
faction name, ignoring case and surrounding spaces. Name-only searches
require one matching standard faction before the reaction threshold is
applied. Use `faction_id` with the name to disambiguate.

`compare_snapshots.faction_name` resolves an exact saved faction name
across the two captures before comparing progress by faction ID. The name
must identify one ID across both captures. Supply `faction_id` to narrow
the match. Once resolved, the ID is used on both sides, including when the
recorded label changed or one side did not capture the faction.

`compare_snapshots.currency_name` resolves an exact saved currency name
across both captures before comparing balances by ID. A name alone must
resolve to one ID. Supply `currency_id` to narrow the match. Existing
account-scope checks still apply to balance and cap differences.

`list_snapshots.currency_balance.currency_name` searches exact saved
currency names. It resolves a unique matching ID in each capture before
applying quantity or cap filters. Add `currency_id` to disambiguate.

`list_snapshots.renown.faction_name` and `list_snapshots.paragon.faction_name`
find an exact saved name within the corresponding reputation kind. Names
must resolve before level, cap or reward filters are applied. Add
`faction_id` to disambiguate.

Use `list_snapshots.tracked_achievement.achievement_name` to find an exact
saved achievement name before applying completion or character-credit
filters. Use `compare_snapshots.achievement_name` to resolve the name to
one ID across both captures and compare saved progress. Add an
`achievement_id` to disambiguate. Only player-tracked achievements in
the selected captures are covered.

`compare_snapshots.quest_title` resolves an exact saved Forever quest
title across both captures before comparing progress by ID. If the title
matches multiple IDs, supply `quest_id` as well. A quest missing on one side
does not establish completion, failure or abandonment.

Selected quest comparisons report recorded XP, honor, copper reward,
required copper, projected net copper and upfront shortfall where
available. Shortfall uses each capture's wallet. A missing amount stays
unknown, and a delta requires an amount on both sides.

`quest_change.reward_entry_changes` compares fixed items, fixed currencies,
renown rewards and optional item or currency choices in separate groups.
Quantities are grouped by reward ID and sorted, so reordering slots does
not count as a change. Duplicate rows stay separate. A missing group on
either side leaves changes unknown.

Use `quest_min_reward_honor` with `list_snapshots` or when inspecting the
`quests` section to find quests with at least that much recorded honour.
A zero threshold still requires a recorded amount. Combine it with XP,
cost or objective filters to require those conditions on the same quest.

Set `inspect_snapshot.quest_ready_honor_target` in the `quests` section
to check an explicit honour reward target after quest filters. The plan
uses quests recorded as ready, chooses the highest known rewards first,
and breaks ties by quest ID. Unknown rewards may leave target coverage
unknown. The plan does not convert honour rewards into PvP rank points.

If the selected quests have recorded copper costs and rewards, the honour
plan also shows a wallet-friendly order for that selection. It reports
the minimum starting copper, the affordable prefix and its honour total
when a wallet is captured. It does not search alternative quest sets.

`inspect_snapshot.quest_ready_currency_target` accepts a `currency_id`
and positive `quantity` in the `quests` section. It selects recorded
ready quests by their fixed rewards for that currency. The report separates
target coverage before caps from recorded cap headroom for the selected
rewards. Optional choices are excluded, and missing fixed reward lists
can leave coverage unknown.

Set `include_choices: true` in the currency target to consider one
recorded choice per quest. The plan takes the largest matching choice,
breaking ties by recorded position, and reports the position alongside
fixed rewards. Missing fixed or optional reward lists may leave a
shortfall inconclusive. Choice eligibility remains unverified.

When selected quests have recorded copper costs and rewards, the wallet
projection shows the minimum starting copper and the currency from the
affordable prefix, including selected choices. Prefix quantities are
before caps. A missing wallet leaves the prefix unknown. The estimate
covers the selected quests and uses the wallet from the same capture.
The affordable prefix also has a separate cap projection and target
shortfall within the recorded headroom. Unknown headroom leaves that
shortfall unknown. Actual currency credit remains unverified.

`inspect_snapshot.quest_ready_item_target` accepts an `item_id` and
positive `quantity` in the `quests` section. It selects ready quests
from recorded fixed item rewards, or includes one matching optional choice
per quest when `include_choices` is true. The plan describes potential
reward quantities. Current eligibility, inventory space and acquisition
remain unverified.

The selected quests' wallet projection uses recorded copper costs and
rewards to estimate starting copper and items from the affordable prefix.
The shortfall is measured against the reward target after any counted
carried copies. Missing wallet data leaves the prefix unknown.

Set `count_carried: true` in a quest item target to count copies in the same
capture's carried bags. The target then describes the desired total after
rewards. If the bags already cover it, the plan is empty. If bags were not
captured, the reward plan uses the full target and an otherwise unmet target
has unknown coverage.

Set `estimate_bag_space: true` to estimate new general bag slots for the
selected rewards of the target item. It uses agreeing stack limits from
carried copies and counts only recorded general free slots. The estimate
assumes incoming copies use that limit. It shows a new-stack estimate and
a separate estimate if the incoming copies can merge into carried stacks
of the same item ID. Other rewards are outside its scope. Merge eligibility
and actual placement remain unverified.

The same request reports `affordable_prefix_bag_space_projection` for
target item rewards from the selected turn-ins the saved wallet could
fund. It is null when the affordable quantity is unknown or there is no
wallet turn-in order. The full selected-plan estimate remains separate.

`inspect_snapshot.quest_ready_renown_target` accepts a `faction_id` in the
`quests` section. Supply a positive `quantity` or use the recorded
distance to the next rank. The planner considers recorded fixed renown
rewards from ready quests and shows the selected rewards against the
captured faction progress. An unobserved next-rank threshold leaves a
derived target unknown.

For a selected renown plan with recorded copper costs and rewards, the
wallet projection estimates the minimum starting copper and the renown
from the affordable prefix. A missing wallet leaves the prefix unknown.
The projection evaluates the selected quests and leaves other possible
quest combinations and live affordability unverified.

With `find_snapshot_item.bag_stack_groups` enabled, set
`stack_group_container_id` to review stack consolidation in one recorded
container. `stack_group_storage` selects bags or a bank. Only the stack
review is scoped. Item locations and storage totals still cover the full
capture. An uncaptured container returns a null review.

New Forever captures may include `reported_stack_limit` on an item
location. The addon records it when two readable item-info calls agree
for the observed link and the value covers the recorded stack count.
`find_snapshot_item` returns this evidence with the matching location.
Enable `use_reported_stack_limits` with `bag_stack_groups` to estimate
consolidation from captured limits. Every stack in an exact-link group must
report the same limit. Incomplete or conflicting groups stay unestimated.
The summary covers all matching groups before paging. This option is
exclusive with `assumed_max_stack_size`; actual merge eligibility remains
unverified.

With `inspect_snapshot.bag_restock_requirements`, set
`bag_restock_use_reported_stack_limits` to estimate bag space using the
saved carried copies of each requested item. Every recorded carried copy
must report the same limit. Missing or conflicting limits leave that item's
estimate unknown. The projection assumes the limit applies to incoming
copies, including any quoted merchant bundles. This option is exclusive
with supplied `bag_restock_stack_limits`.
Also set `bag_restock_include_bank_stack_limits` to include observed
character and account bank copies in the agreement check. This can supply a
limit for an item with no carried copies. Any recorded copy with a missing
or different limit makes the estimate unknown. Free slots and merge
headroom still refer to carried bags.
Update the importer and reader before syncing captures with this field.
[Forever item API documentation](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ItemDocumentation.lua)
