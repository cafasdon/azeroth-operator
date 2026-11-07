# Forever interaction map

Updated 7 November 2026. This map records source evidence and engineering scope. A callable API or a passing fixture does not qualify a live build or establish authorisation.

The hosted reader and importer support the bank, recipe, reputation, currency, tracked-achievement and quest observations below. See the [hosted deployment evidence](../../docs/container.md#forever-snapshot-rollout-7-november-2026) and the [Windows companion and package checkpoint](README.md#capture-only-installation-checkpoint). Live addon capture remains pending.

| Area | MCP role and current state | Evidence needed next |
| --- | --- | --- |
| Character selection | Exact name and realm filter on saved snapshots. Explicit target takes precedence. | Qualify a supported, fresh active-character signal. Until then a saved capture remains historical. |
| Owned gear | Observe equipment, bags and visible character/account bank in one export. Local MCP joins observed item locations to pinned base-item records. | Qualify real storage visibility, exact modifiers, equip rules and coverage gaps. Account bank is shared within the owner's account. |
| Equipment sets and item management | Compare and recommend manually applied changes. | Verify Forever slot legality, unique limits, enchants, suffixes, set bonuses and hand combinations. Apply codes, automated moves and inventory transactions stay blocked. |
| Talents, spellbook and stats | Export available character observations, then evaluate builds offline. | Qualify Forever talent ranks, learned spells, weapon skills, buffs used in sims and supported class/spec rotations. |
| Simulation and best-in-slot | Local catalogue and encounter presets available. Pinned simulator baseline verified offline. | Map captured builds and item variants into simulator inputs and evaluate complete legal owned sets. Track phase and acquisition evidence independently for obtainable upgrades. |
| Quest and world navigation | Saved quest log with objective types and counts, failure and turn-in readiness where available. Local simulator reference zones/NPCs. | Qualify native capture and a wider, licensed, build-identified world dataset for offline quest and travel planning. Leave movement and quest actions to the player. |
| Professions and crafting | Saved profession ranks and a selected recipe. The MCP compares fixed required item quantities with observed bags and banks for a requested craft count. The pinned catalogue can find item and enchant references by recipe spell ID. | Qualify native capture, learned recipe enumeration, optional/alternative/variable reagents, tools, stations and cooldowns. Crafting stays with the player. |
| Reputation | Saved visible faction rows with stable faction IDs, raw standings and thresholds, account scope and progression kind where known. | Qualify native capture and specialised friendship, renown and paragon progress. Collapsed or filtered factions may be absent. |
| Currencies | Saved visible balances keyed by stable currency ID, with account scope, inactive status, cap fields and earnings. | Qualify native capture and separately verify wallet gold and hidden-currency coverage. |
| Achievements | Aggregate achievement points and saved progress for player-tracked achievements, including account-wide and earned-by-character flags. | Qualify native capture and assess broader catalogue coverage and progress with stable criterion identities. |
| Economy and trading | Evaluate public price, recipe or loot data if its terms and provenance permit local use. | Select a permitted feed. Purchases, bids, sales, mail and trades remain player actions. |
| Macros, bindings and action bars | Read saved observations where the client supports them. | Qualify visibility and report configuration gaps. No MCP-generated Apply codes or macro execution. |
| Dungeons, raids, combat and PvP | Offline encounter, roster or preparation analysis from appropriate owner-provided data. | No live combat coaching from restricted values, input automation, targeting, queueing or play. |
| Social, guild and account administration | Public reference or owner-approved offline records only. | Chat, invitations, account changes and distribution of private data stay outside MCP scope. |

New MCP routes are denied until reviewed for data source, owner scope, outputs and side effects. The tool cannot drive game input, bypass protected values, read process memory, intercept traffic or invoke a game API. See [enforced policy](../../docs/operation-policy.md) and Blizzard's [addon policy](https://us.forums.blizzard.com/en/wow/t/ui-add-on-development-policy/24534).

Use `inspect_snapshot` with `section: "quests"` for quest details. `character_summary` counts observed and unknown turn-in statuses separately. Its `quest_checklist` groups quest IDs as ready, explicitly not ready, failed, or unknown. A captured failure takes priority in the checklist. A missing quest section gives a null checklist, and an observed empty log gives empty groups. Objective entries are indexed within a quest and include type, completion and available counts. The addon omits objective text, and a missing objective map indicates unavailable data. Older title-only snapshots remain readable. Update the companion and hosted importer/reader before syncing new quest detail. [Forever QuestLog API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/QuestLogDocumentation.lua)

On the Rust MCP, pass `profession_skill_line` with `section: "professions"`
to inspect one saved profession and its observed rank cap. Duplicate
matches are ambiguous. Missing sections and unlisted skill lines have
separate observation statuses.

On the Rust MCP, pass `achievement_id` with `section: "tracked_achievements"`
to inspect one saved achievement and its captured remaining work. A false
observation flag means the ID is absent from the tracked list. A null flag
means the list was unavailable. Only player-tracked achievements are covered.

On the Rust MCP, pass `faction_id` with `section: "reputation"` to inspect
one faction and its standard or special progress. An unlisted faction
has a false observation flag. An unavailable list gives a null flag.

On the Rust MCP, pass `currency_id` with `section: "currency_balances"`
to return one currency and its cap headroom. The observation flag is false
when the saved list lacks that ID and null when the list is unavailable.
An observed balance of zero remains zero.

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
lines, pet banks, future spells and flyouts. `character_summary` counts the
observed slots, which can include the same spell more than once. Update
the importer and reader before syncing this section. [Forever spellbook API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/SpellBookDocumentation.lua)

Pass `spellbook_spell_id` with the spellbook section to return matching
slots by either ID. A false `spellbook_spell_observed` means no captured
slot matched. Null means the spellbook capture was unavailable. The result
describes visible slots and does not establish permanent learning.

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
and counts quests with an unknown cost. If the same export includes the
optional wallet, each entry shows its shortfall against that balance. These
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
craft count. It reports bags and all observed storage separately. Either
missing limit makes that estimate null. The result does not advance cooldowns
or establish that the character meets the recipe's other requirements.

Each fixed reagent also lists `suggested_bank_withdrawals` to cover its bag
shortfall using observed character bank stock first, then observed account
bank stock. An uncaptured bank remains unknown, and unavailable bags prevent
a suggestion. These per-recipe amounts are independent. For a shared batch,
use `ordered_starting_materials` to account for materials used across recipes.

The selected recipe collector reads the visible local schematic form during `/aoexport`. The panel must already be open on a local item or enchant recipe. Saved recipe reviews also expose the captured relative difficulty, skill-up flag and reported number of skill-ups where available. These are observations at the recipe capture time, including when inventory is taken from a later snapshot. They do not estimate skill-up probability or predict gains from a batch. Item recipes with a captured schematic output range also report `base_output_if_completed`, the base minimum and maximum multiplied by the requested crafts. This is conditional on completing the crafts and excludes bonuses. Use `target_output_quantity` instead of `crafts` to round up from a desired quantity using the captured minimum per craft. This works for individual and batch recipes with a known range, within the existing 1,000-craft limit. Batch reviews also report `combined_base_output_if_completed` by observed output item. If a recipe for the same item lacks a range, the combined range for that item is unknown. Recipes without an observed output item are counted separately and excluded from the totals. Batch reviews also report `ordered_starting_materials`, assuming each recipe completes in supplied order with its minimum captured yield. Earlier recipes can cover later reagents by matching item ID. Each recipe consumes its entire requested batch of reagents before producing output. Unknown yields contribute zero and unsupported material requirements suppress this estimate. Stock shortfalls use observed inventory only. The ordered plan also shows bag shortfalls and suggested withdrawals, using observed character bank stock first and then observed account bank stock. Uncaptured banks remain unknown. If bags were unavailable, the bag shortfall and withdrawal suggestion are unknown. Players withdraw reagents manually. Older or unavailable output ranges and enchant recipes return null. Learned recipes can include saved cooldown seconds rounded up, the daily flag and available charges. Reviews show the requested craft count above observed charges when charges were captured. Cooldowns describe the recipe capture and are not advanced using the snapshot age. Recipe captures can include the panel’s tool, station and area conditions. Reviews report observed unmet rows in `unmet_crafting_conditions`. An empty object means no unmet condition was captured in the observed list. An unavailable list gives null. These conditions are tied to the recipe capture, including when material counts use a newer inventory. Enchant review covers fixed consumed reagents. Target selection and eligibility remain outside the estimate. The planner reports shortfall against the observed inventory and counts each reagent separately in bags, character bank and account bank. Uncaptured storage has a null count and observed storage with no matching item has a zero count. Missing observations, unlearned recipes, optional inputs and non-material requirements can affect actual craftability. [Forever schematic form](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeSchematicForm.lua) · [Forever trade skill types](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/TradeSkillUITypesDocumentation.lua)

Offline checks passed for the Lua capture, Rust import and MCP review, including duplicate material quantities, incomplete inventory, and refusal of malformed reagent lists and salvage recipes. The `0.22.0-dev` Forever package passed byte and Lua export round-trip verification (SHA-256 `796bac24b8072309cdbee98ae8d0ce17dda6af595922c6edefbfc164b738609f`). Caller and failure review identified sparse reagent slots and separate salvage inputs, and the revised guards passed the follow-up probes. Jev advisory triage classified both as material (`jev-1.14.0`, request `9b76e769cba333e61da3c48a5110227aaa3c65ddb7041129f551dad8ff03c1d3`, 439 input/46 output tokens, 0.305 seconds). The Forever addon has not been installed.

Use `inspect_snapshot` with `section: "reputation"` for the Forever faction observations. `character_summary` reports the observed faction count and `compare_snapshots` can compare observations for the same character and build. The Rust MCP also reports `standard_reputation_progress` by faction ID. It gives points earned in the current rank, rank width and remaining points to the next rank when the captured standard standing fits the bracket. Maximum standard reaction is marked capped. Special progression types and inconsistent brackets are omitted from that calculation. Captured renown rows can separately include `renown` with the observed level, points earned in the level, level threshold and maximum status. This detail is absent from older exports and when the renown API fails or returns inaccessible values. Friendship rows can include a separate `friendship` observation with its own standing and rank thresholds. A captured friendship observation without a next threshold represents maximum rank in the client panel. An absent friendship object means details were unavailable. Paragon rows can include the observed counter, reward threshold, pending-reward and level restriction flags, and a positive reward quest ID when accessible. The Rust MCP includes `special_reputation_progress` for factions with those details. It shows remaining points for renown and friendship ranks when the captured numbers support it, and distinguishes pending paragon rewards, level-restricted pending rewards, and no pending reward. Reward claiming remains a player action. When comparing captures by `faction_id`, `special_progress_change` reports renown level and cap changes, friendship standing changes, or paragon reward flag changes. Renown points are compared only within the same uncapped level and positive threshold, with both counters within range. Missing detail or a changed faction type or account scope gives null. The collector does not expand headers, change filters or select factions. [Forever reputation API](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ReputationInfoDocumentation.lua) · [Forever reputation panel](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_UIPanels_Game/Camelot/ReputationFrame.lua)

Offline verification covered signed standing values, account-wide and header factions, absent rows, unavailable progression classification, and the real MCP inspection, summary and comparison paths. Forever `0.23.0-dev` passed package verification (SHA-256 `5cb0b7e395ab7248f77e13fd22550f3fa8350004d8f0e013877aaf507cf24339`). Native capture remains pending.
