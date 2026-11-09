# Forever interaction map

Merchant discovery accepts `merchant_stock_state` with an existing offer
selector. Use `unlimited`, `limited_in_stock` or `sold_out` to match the
stock reported for the same offer in the saved visit.

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
[Offline evidence](../../docs/observation-comparisons-verification-2026-11-08.md).

Visible local profession capture can retain stable reported known spell IDs
without filter changes. Exact crafted-source IDs can support local learned-output
searches; conflicts stay unknown. The spell report does not establish complete
recipe coverage, reagent availability or current craftability.
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
Use this for historical sourcing. Pet acceptance can change between captures.

Use the same `merchant_pet_food_snapshot_id` in `list_snapshots` to
search all compatible saved merchant visits. It replaces an exact item
or currency selector and combines with the stock, price and affordability
filters. A missing food classification returns an error. An observed
empty classification returns no matches.

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
The review also lists retained available trainer slots whose lesson names
exactly match an unmet prerequisite. These are name matches within the
captured panel and do not establish that training a lesson clears the
prerequisite. Slot, name and status filters limit the candidates.

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
Use `action_bar_item_stock_audit` to check every retained item shortcut at
once. Results group captured slots by item ID and report equipment and bag
counts only for those IDs.
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

## Personal PvP progression (2026-11-09)

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
cap rank thresholds. MCP summary adds reported_total_rank_points and
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
