# Azeroth Operator

Hunter pet template options now reject unsupported attack-speed selections,
including during prepared merging. Reports expose template pet assumptions;
captured identity, training and native effects remain unverified.
[Contract](docs/ADR-067-hunter-pet-template-options.md).

Hunter simulation preflight now validates ammunition against the pinned engine’s
reference choices. Unknown numeric selections are rejected instead of silently
modeling zero ammunition damage. Reports retain explicit template-only ammo
references without claiming equipped ammo or available stock.
[Contract](docs/ADR-066-hunter-ammunition-selection.md).

Gear simulation review now carries partial equipped-bag observations into quiver
diagnostics and final encounter context. Unsupported numeric quiver selections
are rejected before the workflow runs the engine; observed matches remain
identity evidence, not verified native bonuses.
[Contract](docs/ADR-065-quiver-observation-simulation-context.md).

Saved bag-capacity observations can now include the equipped container’s item
ID and reported inventory slot when stable public reads agree. These identities
are inspectable but receive no material-stock or verified quiver-bonus credit.
[Capture contract](docs/ADR-064-equipped-bag-item-observations.md).

Profession recipe search can now order a stock-based shortlist by independent
observed item capacity before pagination. Combine it with saved skill-up and
cooldown filters; it does not predict gains or allocate shared materials.
[Ordering contract](docs/ADR-063-recipe-item-capacity-ordering.md).

Saved dialogue discovery now searches exact display labels, option/spell IDs
and raw status/flags on the same recorded choice. Missing metadata stays unknown;
results identify historical captures, not current NPC services.
[Query contract](docs/ADR-062-saved-gossip-option-discovery.md).

Forever exports can now record the dialogue choices in an already visible NPC
gossip panel, beyond quest lists. Saved choices are inspectable through MCP;
selection and other dialogue actions stay unavailable.
[Capture contract](docs/ADR-061-visible-gossip-options.md).

Interrupted gear searches can now retain a ranking of loadouts tested in every
requested encounter. Missing results receive no score or replacement weight;
unfinished pages stay incomplete and resumable.
[Partial-result contract](docs/ADR-060-completed-candidate-ranking.md).

Final owned-gear rankings now retain each visited encounter’s engine/profile
identity, numeric settings, weight and baseline result. Per-loadout evidence
includes the baseline metric value, so gains remain auditable even with top=1.
[Report contract](docs/ADR-059-ranked-encounter-context.md).

MCP policy reporting now distinguishes enforced offline restrictions from
Blizzard authorisation. Published-source checks and unsuccessful full-EULA
retrievals are explicit; approval and absolute compliance remain unverified.
[Evidence boundary](docs/policy-evidence-verification-2026-11-09.md).

Saved instance and world-boss inspection can now add historical reset-time
projections with `saved_reset_projection: true`. Negative durations remain unknown;
this does not establish current lockout or entry availability.
[Contract](docs/ADR-058-saved-reset-projection.md).

Skill capture now guards restricted API values and compares two bounded row
projections, rejecting observed rank, identity or metadata changes even when
the row count stays the same. This remains partial historical capture.
[Offline evidence](docs/skill-line-capture-verification-2026-11-09.md).

Saved skill-line queries now filter recorded base ranks, caps and points to the
observed cap, with paged inspection and explicit unknown categories. Historical
observations do not establish training eligibility or complete skill coverage.
[Query contract](docs/ADR-057-saved-skill-line-filters.md).

Simulation preflight now checks explicitly captured inactive talent ranks too.
Known conflicts with matching pinned rank limits and explicitly false staging
are rejected before the engine runs. Missing definitions remain unknown; older
captures keep active-node comparison. [Comparison contract](docs/ADR-056-queried-allocation-simulation-checks.md).

Simulator talent preparation can use explicitly captured purchased ranks for
every pinned class definition, including observed zero ranks. Ask
`captured_talent_loadout` for `prepare_simulator_talents: true`; missing,
ambiguous, mismatched or staged evidence produces no string. This prepares a
simulator-only draft, with native completeness, legality and model accuracy
still unverified. [Allocation contract](docs/ADR-055-queried-talent-allocation.md).

Saved-loot discovery can find captures that displayed an item, a raw reported
currency ID or a money slot. Quantity bounds apply only to matching readable
rows in one capture; money amounts remain unknown. With `latest_per_character`,
newest valid selection precedes feature filtering, and displayed loot receives no owned-stock
or balance credit. [Query contract](docs/ADR-054-saved-loot-discovery.md).

Spellbook capture now checks required API/enum/getter values before use and
isolates unreadable optional spell metadata, preserving readable base spell
entries. The previous loot-fixture spellbook exclusion is removed.
[Offline guard evidence](docs/spellbook-guards-verification-2026-11-09.md).

Visible loot captures can distinguish raw reported currency rows and money-slot
presence from item rows when stable public slot types are available. Currency
container rewards and money amounts remain unverified; nothing is credited to
inventory or balances. [Typed loot contract](docs/ADR-053-forever-typed-loot-observations.md).

Forever talent captures can retain stable trait definition IDs, purchased ranks
and native rank limits alongside active ranks, plus an observed staged-change
flag when readable. A bundled nine-class simulator definition map now checks
known purchased ranks against explicit template talent strings before simulation.
Known conflicts or staged changes require a committed matching capture; plan-only
reports remain available. Unknown metadata and full allocation coverage stay
unverified. Matching MCP support and addon
0.47.0-dev are required; the package remains uninstalled.
[Contract](docs/ADR-050-forever-talent-definitions.md) ·
[Offline evidence](docs/forever-talent-definitions-verification-2026-11-09.md).
See the [local mapping contract](docs/ADR-051-forever-simulator-talent-observations.md).

`captured_talent_loadout` also accepts optional `simulator_talent_string` for an
explicit saved Forever snapshot. Its `simulator_talent_observation` compares known
purchased ranks directly through the MCP, without running an engine or returning
a game import. Native import availability remains separate; full allocation and
model coverage stay unverified. [Direct MCP contract](docs/ADR-052-forever-talent-template-mcp.md).

Saved-character inventory discovery can filter `list_snapshots` with
`inventory_item`: an item ID, explicit storage and observed quantity bounds.
Use `latest_per_character` to avoid reviving an older stock observation when a
newer capture is empty or unknown. Quantities stay separate per capture/storage;
shared banks are never totalled across characters. This is historical discovery,
not active-character detection or current access.
[Contract](docs/ADR-049-saved-inventory-discovery.md) ·
[Offline evidence](docs/inventory-item-discovery-verification-2026-11-09.md).

For a reagent set or shopping list, use `inventory_requirements` with one storage
and 1..64 distinct item IDs and positive quantities. A result must cover every
requirement in the same capture; it may also be filtered by a captured profession.
This supports observed material discovery without claiming craftability,
transferability or combined stock across characters.
[Verification](docs/inventory-requirements-discovery-verification-2026-11-09.md).

Forever manual captures can include carried-item cooldown observations linked
to the captured bag slots. Each item has a readable historical timer/enabled
state or explicit unavailable status; summaries keep those counts separate.
This does not establish current usability or activate items. Requires matching
MCP support and addon 0.46.0-dev. The package is uninstalled and native capture
remains unqualified.
[Contract](docs/ADR-048-passive-bag-item-cooldowns.md) ·
[Offline evidence](docs/bag-item-cooldowns-verification-2026-11-09.md).

Use `inspect_snapshot` with section `bag_item_cooldowns` and
`bag_cooldown_filter` to select item IDs, reported/unavailable status, recorded
enabled flags or inclusive timer bounds. Results sort by numeric bag/slot and
page up to 50 rows; unknown predicates remain separate from nonmatches.
[Filter verification](docs/bag-cooldown-filter-verification-2026-11-09.md).

Item lookup includes captured cooldown evidence on matching bag locations when
available. Counts cover all matching bag locations before pagination; separate
snapshot results retain their own capture time and evidence. Equipment/bank
matches receive no borrowed cooldown data, and legacy captures keep their prior
lookup shape. [Verification](docs/item-cooldown-lookup-verification-2026-11-09.md).

Python ingestion now rejects missing or malformed container stack counts before
writing, matching Rust's count contract. Equipment's absent/null count behaviour
is preserved. [Parity evidence](docs/item-stack-count-parity-verification-2026-11-09.md).

Base inventory validation also enforces Rust's item-field, identity, location
and bank-container contract before ingestion. Unsupported extra fields are
rejected; invalid saved captures remain unchanged and are refused on read.
[Inventory structure evidence](docs/inventory-item-shape-parity-verification-2026-11-09.md).

Optional local reagent references can now retain bounded NPC/object spawn
locations. Reagent chains return only the referenced location records, preserving
provider area/phase namespaces, unknown data and truncation. No route or movement
is executed. [Offline evidence](docs/reference-locations-verification-2026-11-09.md).

Loot inspection can compare readable displayed items against an explicit reagent
or shopping list. It reports coverage and hypothetical remaining needs if those
items are acquired, without granting owned stock or assuming loot rights.
[Offline evidence](docs/loot-requirements-verification-2026-11-09.md).

Saved spellbook searches can now combine charge/cast-count ranges and cooldown
flags, or discover matching resource observations without a spell ID. Inspection
uses the same predicates and keeps unknown values separate from zero/false.
[Offline evidence](docs/spell-resource-search-verification-2026-11-09.md).

Spellbook exports can retain readable, stable charge counts, reported cast counts
and cooldown flags. Local summaries and exact reported-ID action-bar references
keep these historical observations separate from current casting eligibility.
[Offline evidence](docs/spell-resources-verification-2026-11-09.md).

Forever manual exports can now observe an already shown loot window. Stable
public item IDs and quantities are inspectable separately from owned inventory;
unreadable and non-item slots remain explicit. No looting or rolling is performed.
[Offline evidence](docs/loot-window-verification-2026-11-09.md).

Crafting acquisition lookups now retain a matching selected recipe's item-capacity
basis, combined item/charge limit and captured cooldown state. Bulk observations
stay separate; these historical limits do not verify current crafting eligibility.
[Evidence](docs/catalogue-recipe-capacity-verification-2026-11-09.md).

Recipe review distinguishes explicit zero-item requirements from unavailable
material evidence. Zero-item recipes can now retain their captured charge limit
even when inventory was not observed; this does not verify current craftability.
[Evidence](docs/zero-item-recipe-capacity-verification-2026-11-09.md).

Reagent-chain preparation now accounts for repeated producer uses against one
saved charge pool per recipe. Per-step requests stay visible beside cumulative
shortfalls; newer stock never refreshes those charges. This remains a conditional
material draft, with current craftability unverified.
[Evidence](docs/chain-charge-accounting-verification-2026-11-09.md).

`search_forever_recipes` can filter captured bulk cooldown state, minimum charges
and maximum remaining seconds alongside material and recipe-view filters. Charges
take precedence when classifying state; missing numeric observations stay unknown.
Cooldown evidence stays with the original recipe capture even when stock is newer.
This does not establish current craftability.
[Evidence](docs/recipe-cooldown-search-verification-2026-11-09.md).

Bulk recipe captures can now retain stable public cooldown and charge observations.
Search and recipe/batch reviews carry them into charge checks, capacity and timer
estimates while keeping recipe and inventory capture times separate.
[Evidence](docs/bulk-recipe-cooldown-verification-2026-11-09.md).

Multi-encounter and owned-gear evaluation now accept `--rank-by` for DPS,
damage taken, threat, death fraction, healing and time out of mana. Role scores
use explicit units and retain weighted/worst-encounter objectives and private
checkpoints. DPS remains the default. [Evidence](docs/role-gear-ranking-verification-2026-11-09.md).

Positional gear searches now include equipped-item moves that leave an empty
source slot, when both slots are in the requested search scope. Legacy placement
keeps its existing exclusions. These are saved offline layouts, not game moves.
[Evidence](docs/equipped-vacancy-verification-2026-11-09.md).

Owned-gear evaluation now keeps inventory evidence visible through plan merging,
journal restart and final rankings: observed/missing storage and bounded unmatched
catalogue IDs. Computational completion remains separate from ownership proof.
[Evidence](docs/gear-inventory-propagation-verification-2026-11-09.md).

Recipe discovery can use a newer same-character inventory capture with
`search_forever_recipes.inventory_snapshot_id` and `material_filter`. Recipes and
skill-up flags stay tied to the original recipe capture; stock and missing bank
coverage come only from the explicit inventory capture.
[Offline evidence](docs/recipe-search-inventory-verification-2026-11-09.md).

Local character analysis and player-operated configuration tools for World of Warcraft.

## Client subdivisions

- `clients/retail/`: Retail adapter, addon and capability evidence.
- `clients/classic/`: Classic adapter, addon and capability evidence. Classic variants must be identified explicitly; Era and progression clients are not interchangeable.
- `clients/forever/`: Forever adapter, addon and capability evidence.
- `shared/`: client-independent snapshot contracts, policy enforcement and planning.
- `mcp/`: shared MCP interface, requiring an explicit client identity.
- `tests/`: shared tests and separate client qualification suites.

Status: read-only stdio MCP foundation implemented and integration-tested. No live game client qualified yet.

`python -m tools.run_forever_encounter_windows --help` compares saved windows
across up to eight DPS encounter profiles under shared work budgets. Repeat
`--encounter-lookup` and optional positive `--encounter-weight`. Complete,
comparable results produce weighted gains and encounter sensitivity; missing
profiles stay unknown. Both saved-window runners offer `--checkpoint-only`.
[Evidence](docs/encounter-window-execution-verification-2026-11-09.md).

`python -m tools.run_forever_saved_windows --help` executes explicitly selected
retained windows after fresh local MCP/model checks. It groups identical requests,
uses private numeric checkpoints, and reports completed, missing and uninspected
work under run/iteration/time budgets. Fixed-run rankings support paired DPS,
tank and healing metrics. This is not full owned BiS or native qualification.
[Evidence](docs/saved-window-execution-verification-2026-11-09.md).

The fixed-run simulator and gear ranker accept `--simulation-journal ROOT` for
private numeric checkpoints that do not evict completed results. It is separate
from `--cache`; `--simulation-journal-mib` sets a 1–512 MiB ciphertext budget
(default 64). Engine, request, metric mode and adapter/validator identity must
match for reuse. Quota, corruption and conflicting results stop safely.
[Evidence](docs/simulation-journal-verification-2026-11-09.md).

`python -m tools.merge_forever_prepared_windows --journal PATH PREPARED_JSON...`
groups compatible prepared windows against their exact saved journal rows.
Identical requests share a global review ID and retain original references;
changed profiles, provenance labels or source material reject. This prepares
a request set without executing simulations or proving full BiS.
[Evidence](docs/prepared-window-merge-verification-2026-11-09.md).

`python -m tools.prepare_forever_gear_window --help` prepares simulator requests
from one exact retained journal window. It rechecks saved selections through the
local MCP, requires identical gear inputs, and applies the existing character,
pet, profession and native-slot model guards. No candidate enumeration or engine
execution is performed. [Evidence](docs/gear-window-preparation-verification-2026-11-09.md).

The offline gear collector can opt into `--retain-equipment-specs` to save the
numeric equipment behind each resolved draft hash. Inspect an exact saved window
with `--include-equipment-specs` to retrieve those inputs. Retention must stay
consistent across resume; old hash-only reports remain readable with unavailable
equipment details. This does not create or run a qualified simulation job.
[Evidence](docs/retained-gear-inputs-verification-2026-11-09.md).

`inspect_snapshot` accepts `currency_filter` for saved Forever `currency_balances`:
captured account/inactive/weekly-earning flags, positive-cap presence and weekly/
total headroom bounds. Missing earning observations remain unknown; zero cap
values do not become zero headroom. [Evidence](docs/currency-filter-verification-2026-11-08.md).

`inspect_snapshot` accepts `reputation_filter` on the saved Forever `reputation`
section: progression kind, account-wide flag, capped state, rank distance and
paragon reward status. Rank predicates require an explicit compatible kind;
missing progress stays unknown. [Evidence](docs/reputation-filter-verification-2026-11-08.md).

`inspect_snapshot` accepts `achievement_filter` for the saved Forever
`tracked_achievements` section: completion, character credit and account-wide
flags, plus same-row criterion ID, completion and maximum remaining units.
Missing criterion observations remain unknown; an unlisted achievement is not
inferred incomplete. [Evidence](docs/achievement-filter-verification-2026-11-08.md).

`inspect_snapshot` can filter the saved Forever `quests` section by
`quest_objective_kind`, `quest_objective_finished` and
`quest_objective_max_remaining`. Supplied objective predicates match the same
row. Missing lists/counts remain unknown; empty observed lists do not match.
These filters combine with existing quest rewards/readiness and projections.
[Offline evidence](docs/quest-objective-filter-verification-2026-11-08.md).

`python -m tools.collect_forever_gear_windows --help` exposes bounded offline
plan collection into an explicit local journal. Supply a trusted absolute MCP
executable, snapshot ID, template and slots. Repeating the same request resumes
the first uncovered coordinate; completed journals perform no recollection.
Template, executable, collector source, configuration and selection fingerprints
must match. Budgets and page size can change; changed inputs require a new journal.
This does not run simulations or establish full owned BiS.
[Offline evidence](docs/gear-window-collection-verification-2026-11-08.md).

Merged windows and journal status include `resolution_accounting`: baseline,
pre-review exclusions, proposed alternatives, resolved draft instances,
equivalences and unresolved proposals. Counts use validated report metadata;
resolved instances can repeat between windows. Complete coordinate coverage can
still contain unresolved proposals and does not prove full BiS.
[Accounting evidence](docs/gear-resolution-accounting-verification-2026-11-08.md).

New windowed plans also retain a bounded coordinate outcome ledger. Use
`python -m tools.journal_forever_gear_windows --journal PATH --window-offset N`
to inspect an exact saved window start. It returns baseline/resolved/equivalent/
excluded/unresolved outcomes, window-local review references and fixed diagnostic
codes, without raw selections. Older reports return unavailable details rather
than inferred outcomes. [Evidence](docs/gear-coordinate-outcomes-verification-2026-11-08.md).

`python -m tools.journal_forever_gear_windows --journal PATH --add REPORT`
stores validated plan windows transactionally in an explicitly chosen local
SQLite journal. Omit `--add` to inspect saved coverage and the next uncovered
offset after restart. Exact retries are idempotent; scope conflicts, overlaps
and payload-budget overruns reject without replacing earlier reports. This
persists planning progress; the separate collector above can resume collection.
Neither journal storage nor the collector runs simulations.
[Offline evidence](docs/gear-window-journal-verification-2026-11-08.md).

`python -m tools.merge_forever_gear_windows` aggregates local windowed plan-only
reports. It checks scope/baseline/count consistency, rejects overlapping ranges,
reports gaps and groups repeated simulator-input hashes with original report
references. Reported coordinate coverage remains separate from feasible-set,
simulation and full-BiS verification. Raw selections and filenames are omitted.
[Offline evidence](docs/gear-window-aggregation-verification-2026-11-08.md).

The local gear ranker accepts explicit-snapshot `--combination-offset` and
`--combination-limit` windows for larger candidate spaces. Each page retains
the existing draft/time limits and baseline, with a scope fingerprint and next
offset. Omission preserves the original whole-request limits. Window results
remain partial; automatic cross-window ranking and full owned BiS are unfinished.

Restock crafting plans can opt into `restock_merchant_reagent_plan` to price
combined reagent shortfalls from saved merchant offers. An optional
`restock_merchant_snapshot_id` supplies compatible offers; affordability always
uses the chosen inventory's wallet. Bundle rounding and incomplete/limited/
extended-cost quotes remain explicit. These are historical estimates, with no
purchase or craft eligibility verified.
[Offline evidence](docs/restock-reagent-quotes-verification-2026-11-08.md).

Consumable restocking accepts `restock_recipes` selectors containing a saved
snapshot ID and optional exact bulk `recipe_id`. Several recipes can come from
one capture; selected and bulk recipes can share one compatible inventory.
Confirmed shortages determine output targets, with shared reagent totals and
explicit unknown cooldowns/conditions. The legacy capture-ID list still works;
the two selector forms are mutually exclusive.
[Offline evidence](docs/bulk-consumable-restock-verification-2026-11-08.md).

Catalogue item acquisition sources can include a separate `bulk_recipe_review`
for one craft from compatible saved bulk materials. It reports material stock,
shortfalls and unknown preparation requirements through the existing planner.
Missing rows, unreadable data, learning conflicts, output mismatches and rejected
reviews remain explicit; selected-schematic fields retain their original scope.
[Offline evidence](docs/catalogue-bulk-materials-verification-2026-11-08.md).

Local catalogue learned-recipe filters also use bulk schematic and tracked
craft/recraft learning observations. Conflicting exact-ID learning flags and known catalogue
output mismatches leave combined learning unknown. Unavailable bulk materials
do not imply unlearning, and learned output discovery does not establish
craftability or supply uncaptured conditions/cooldowns.
[Offline evidence](docs/bulk-catalogue-learning-verification-2026-11-08.md).

`search_forever_recipes` searches one owner-local capture's bulk recipe rows by
exact recipe, output item, required reagent or captured status. Filters match
the same row, with numeric recipe ordering and pages of at most 50 records.
Missing capture data remains distinct from an observed empty list. Returned
recipe IDs can be supplied directly to `review_forever_recipe`; search results
do not establish craftability or complete profession coverage.
[Offline evidence](docs/recipe-search-verification-2026-11-08.md).

Forever manual exports can record bulk fixed-item material observations for the
visible local profession's reported known spell IDs. Inspect
`profession_recipe_materials` or its summary. Unreadable recipes, unsupported
required slots and the retained-material budget remain explicit. This does not
establish craftability. `review_forever_recipe` accepts an optional `recipe_id`
for an exact bulk-observed recipe, including each additional batch entry.
Omitting it retains selected-schematic review. Bulk reviews preserve unknown
cooldowns/conditions and reject conflicting captured observations.
Native availability and responsiveness remain unqualified.
[Offline evidence](docs/profession-recipe-materials-verification-2026-11-08.md).
[Planning integration](docs/bulk-recipe-planning-verification-2026-11-08.md).

Fixed bulk recipe rows can also retain a stable public reported craftable count.
Exact recipe reviews show its capture time and shortfall against requested
crafts separately from material capacity. This historical default-level API
observation does not verify current craftability, cooldowns or charges; an
inventory override does not refresh it.
[Offline evidence](docs/reported-craftable-count-verification-2026-11-09.md).

Recipe chains can opt into `reagent_chain_catalogue_sources` to attach cached
public acquisition records to their net external shortages. The configured
catalogue must match its embedded pin. Missing items/source metadata stay
explicit, lists are bounded, and no current availability or eligibility is
inferred. The cache lacks common reagents and is not a full world database.
[Offline evidence](docs/chain-catalogue-sources-verification-2026-11-09.md).

Source-only manual Forever exports can optionally read reagent source IDs from
a separately installed compatible QuestieDB. Inspect `recipe_item_sources` for
bounded NPC/object/container drops, quest rewards and vendors. Missing data,
truncation and unavailable providers remain explicit; no provider database is
bundled, and native behaviour and dataset completeness remain unqualified.
[Offline evidence](docs/recipe-item-sources-verification-2026-11-09.md).

Chain reviews can add `reagent_chain_captured_sources: true` to join net external
shortages to those saved references without a catalogue or installed provider
at query time. An explicit `reagent_source_snapshot_id` must match character and
client/build; otherwise references come from the recipe capture. Inventory never
silently replaces their provenance. Missing and truncated evidence stays explicit.
[Offline evidence](docs/chain-captured-sources-verification-2026-11-09.md).

`python -m tools.evaluate_forever_owned_gear` combines all-17-slot observed gear
collection and saved multi-encounter simulation in one resumable offline command.
It uses an explicit owner snapshot and separate plan/result journals, with
shared time/work budgets and independent coordinate, resolution and simulation
completion flags. Missing ownership/model/native proof still prevents a BiS
claim. See the [workflow](docs/ADR-027-owned-gear-evaluation-workflow.md) and
[offline evidence](docs/owned-gear-evaluation-verification-2026-11-09.md).

The evaluator accepts either `--snapshot-id` or `--character-name NAME
--character-realm REALM`. The latter resolves the newest matching historical
Forever capture through the owner's MCP store, then pins its snapshot ID for
the run. Missing targets fail before journal writes. Resolution shares the
total time budget and has a 15-second ceiling. Neither selector identifies
the character currently in play.
[Offline selection evidence](docs/owned-character-selection-verification-2026-11-09.md).

Ranked saved-journal gear results include `saved_loadout`: the exact retained
equipment, selected source locations and cleared slots, tied to the snapshot
and source report hash. Locations describe the saved plan; refresh observations
before relying on them in game. [Evidence](docs/ranked-loadout-verification-2026-11-09.md).

Ranked gear also retains `simulation_evidence_by_encounter`: iterations, seed,
request hashes and available native metric deviations or intervals. Paired
intervals compare each candidate with the equipped baseline; death intervals
describe the candidate's marginal death fraction. Neither is an interval for
the weighted ranking or a selection-adjusted BiS guarantee. Missing intervals
remain explicitly unavailable. No additional simulations are needed.
[Offline evidence](docs/ranked-simulation-evidence-verification-2026-11-09.md).

Use `--ranking-objective worst_encounter_gain` on saved-journal or unified
evaluation to maximise the lowest DPS gain across the supplied encounters.
The default remains weighted average gain. Switching objectives reuses existing
numeric checkpoints; neither objective proves statistical or full owned BiS.
[Evidence](docs/robust-gear-ranking-verification-2026-11-09.md).

The positional engine extension now connects MCP review, retained plans and
offline evaluation. Opt in with `positional_simulator_equipment: true` on
`review_equipment_selection`, or `--positional-equipment` on collection/evaluation.
Execution requires the qualified engine hash and exact capability response,
including before cached results are reused. Legacy placement remains the default;
bulk positional execution is refused. Native legality, ownership and full BiS
proof remain open.

Positional Forever equipment reviews also accept `clear_equipment_slots`, on
the primary draft or each additional draft. Only distinct captured occupied
simulator slots may be cleared, and they cannot also be selected targets.
The review retains the previous items and projects exact empty slots; it does
not move gear or assign bag destinations. Collection and owned evaluation accept
`--include-empty-slots` with `--positional-equipment` to enumerate removals of
occupied requested slots, including simultaneous removal/replacement choices.
Standalone collection also requires `--retain-equipment-specs`. Saved replay
revalidates explicit clears against the original capture; checkpoint reuse
preserves the choice and changing the opt-in rejects a mixed journal resume.
[Search contract](docs/ADR-047-empty-equipment-search.md) ·
[Search verification](docs/empty-equipment-search-verification-2026-11-09.md).

The saved empty-ring path also passed an opt-in real-engine offline check:
seven gear states, paired iterations, zero warm simulation runs and agreement
with uncached direct runs. This qualifies the tested workflow, not native
legality, general model accuracy or full owned BiS.
[Real-engine evidence](docs/real-empty-equipment-verification-2026-11-09.md).
[Contract](docs/ADR-046-explicit-empty-equipment-review.md) ·
[Offline evidence](docs/explicit-empty-equipment-verification-2026-11-09.md).
[Contract](docs/ADR-028-positional-equipment-engine.md) ·
[Integration evidence](docs/positional-integration-verification-2026-11-09.md).

`inspect_snapshot` can review selected saved buyback entry quotes against the
same captured wallet, with a copper reserve. Missing quotes and wallet data stay
explicit. This is a cost comparison, not a repurchase plan or eligibility check.
[Offline evidence](docs/buyback-cost-review-verification-2026-11-08.md).

Forever manual exports can inspect an already open merchant buyback tab:
stable base item IDs, quantities and readable entry quotes. Buyback stays outside
owned inventory and simulator candidates; no repurchase or recovery is attempted.
[Offline evidence](docs/merchant-buyback-verification-2026-11-08.md).

Local item lookups can find catalogue quest rewards tied to saved gossip
`available` or `active` lists, optionally matching a known completion flag.
Missing observations remain distinct from empty lists; references do not prove
current rewards, quest eligibility or a particular reward choice.
[Offline evidence](docs/gossip-reward-lookup-verification-2026-11-08.md).

Forever manual exports can observe quest IDs and readable flags from an already
visible gossip panel. Inspect `gossip_quests` or its summary; available and active
lists retain separate unknown/empty states. No quest is selected, accepted or
turned in, and dialogue text and speaker identity are omitted.
[Offline evidence](docs/gossip-quests-verification-2026-11-08.md).

Consumable catalogue lookups with an owner-local capture can match recorded
effect spell IDs to saved helpful aura IDs. Matches can be shared across items;
they do not prove item use, effectiveness or inactivity when absent. Stock,
restock plans and simulator inputs retain their existing meaning.
[Offline evidence](docs/consumable-aura-matches-verification-2026-11-08.md).

Forever manual exports can include public nonprivate player aura lists. Inspect
`player_auras` or read `character_summary.player_aura_observations` for reported
spell IDs and readable application counts. Missing lists remain unknown;
no caster identity, effectiveness or cast eligibility is inferred.
[Offline evidence](docs/player-auras-verification-2026-11-08.md).

`compare_snapshots` can opt into `profession_spell_changes` and
`player_resource_changes`. Spell differences require the same profession;
power deltas require the same type and units. Missing context stays unknown,
and observed differences do not establish learning or resource-use events.
[Offline evidence](docs/observation-comparisons-verification-2026-11-08.md).

Forever manual exports can capture the visible local profession's reported
known spell IDs without changing filters. Inspect `profession_spells` or its
summary; exact catalogue crafting-source IDs can support learned-output searches.
This remains separate from complete recipe coverage and craftability.
[Offline evidence](docs/profession-spells-verification-2026-11-08.md).

Forever manual exports can include `player_resources`: readable stable health
and primary power, independently. `character_summary.player_resource_observations`
shows observed amounts, deficits and percentages; zero maximum leaves percentage
unknown. Power uses unmodified API units. These are historical observations,
with no readiness inference or resource-use action.
[Offline evidence](docs/player-resources-verification-2026-11-08.md).

Catalogue crafting searches can use exact-ID learning observations from saved
profession recipe lists as well as the selected schematic. Conflicts and known
output mismatches remain unknown; list observations do not supply materials,
cooldowns or proof of craftability.
[Offline evidence](docs/catalogue-recipe-learning-verification-2026-11-08.md).

Forever owned-gear reviews include `inventory_evidence`: observed/unavailable
storage sections and bounded IDs missing from the base-item catalogue, counted
before filtering or paging. Seeing all sections does not prove complete ownership,
exact simulator resolution or best-in-slot coverage.
[Offline evidence](docs/owned-inventory-evidence-verification-2026-11-08.md).

Local ranker reports retain that evidence in `gear_search.inventory_evidence`,
along with the source snapshot, selected slots and candidate restrictions.
Older MCP responses leave it null; inconsistent evidence aborts collection.
All report modes keep `best_in_slot_verified: false`.
[Ranker evidence](docs/rank-inventory-evidence-verification-2026-11-08.md).

`resolve_character` resolves an explicitly named saved character within the
configured owner's store and client family. Without an explicit character it
returns `active_identity_unavailable`, with no selection. Saved recency never
establishes current play; automatic active-character detection remains pending.
[Selection evidence](docs/character-resolution-verification-2026-11-08.md).

Forever exports can include `pet_action_bar`: partial native command/spell
slots with reported active, autocast and range flags. Inspect the section or
read `character_summary.pet_action_bar_observations`. Missing slots stay
unknown. Pet/spell names and target identities are omitted; the MCP issues
no commands or autocast changes.
These are historical observations, not pet identity or cast eligibility.
[Offline verification](docs/pet-action-bar-verification-2026-11-08.md).

Forever visible spellbook rows can include stable API `usability` flags.
Search saved rows with `spellbook_spell.usable` and `insufficient_power`, or
read `character_summary.spellbook_usability_observations`. Unknown flags do
not match false. These are historical, partial spellbook observations; they
do not prove current cooldown, target, range or cast eligibility.
[Offline verification](docs/spell-usability-verification-2026-11-08.md).

Forever linked-item captures can include stable `sell_price_copper` from the
native item API. `character_summary.captured_vendor_price_values` reports
quantity-based reference values separately for each observed storage, keeping
missing prices and overflow explicit. These values are not sale eligibility,
transaction proceeds or wallet funds; no item is sold.
[Offline verification](docs/item-sell-price-verification-2026-11-08.md).

Known native slot-category conflicts now withhold resolved Forever simulator
equipment, including conflicting baselines. Owned-gear reviews and ranking
report these conflicts separately; unknown or conditional categories do not
create a new blocker. Raw drafts remain inspectable. This is a conservative
simulation qualification check, not proof of live equip legality.
[Offline verification](docs/native-slot-conflict-verification-2026-11-08.md).

Forever linked-item captures can include stable `equip_location` tokens.
Equipment reviews annotate the selected item's native slot category, keeping
unknown tokens and conditional two-handed off-hand placement explicit.
A category match does not prove equip eligibility; overall compatibility
and simulation checks retain their separate rules.
[Offline verification](docs/native-equip-location-verification-2026-11-08.md).

Forever exports can include `weapon_enchants`, recording public enchant lists
for main hand, off hand and ranged slots. `character_summary` exposes the
reported enchant types, IDs, timers and charges with explicit unobserved hands.
Timers describe capture time; zero or missing details do not establish expiry
or effectiveness. No enchant is applied or cancelled.
[Offline verification](docs/weapon-enchants-verification-2026-11-08.md).

Forever item captures can include optional `native_stats` read for the exact
item link. Equipment reviews compare numeric tokens observed on both items
and keep tokens observed on only one side separate, without assuming zero.
Missing maps remain unknown; raw stat differences do not establish performance,
complete item effects or simulation compatibility.
[Offline verification](docs/native-item-stats-verification-2026-11-08.md).

Forever exports can include `equipment_durability`, with stable readable
current/maximum pairs for equipped slots 1–19. Inspect the section directly
or use `character_summary.equipment_condition` for per-slot percentages,
damaged/broken slot lists and the lowest observed percentage. Unobserved
slots remain explicit; this is partial coverage and does not establish
repair eligibility or price. [Offline verification](docs/equipment-durability-verification-2026-11-08.md).

Forever's visible local profession list can include stable reported recipe
difficulty, skill-up flags and skill-up counts. `character_summary` adds
`profession_skill_up_candidates`: up to 50 learned rows with a known true
skill-up flag, sorted by recipe ID, with totals and unknown counts.
`list_snapshots.profession_recipe` accepts `can_skill_up` and
`minimum_skill_ups_reported` alongside the exact recipe ID. Unknown values
do not match false or zero. These are historical observations from a filtered
list, not guaranteed gains or proof of craftability.
[Offline verification](docs/recipe-skill-up-verification-2026-11-08.md).

Forever flight-map snapshots can include optional `entries.<slot>.map_position`
with numeric `x` and `y` in `[0,1]`. Saved route legs expose these as
`origin_map_position` and `destination_map_position` when known. Coordinates
refer to that leg's recorded `map_id` and UI frame; they do not establish
world distance, travel time or movement instructions. Missing positions stay
absent, including on legacy captures. Fares and route selection are unchanged.
Only the open flight map at the recorded flight master is observed.
[Offline verification](docs/flight-map-positions-verification-2026-11-08.md).

Reachable destinations can also include `observed_route_node_ids`, a compact
object keyed `"1"` through `"N"` containing the origin, intermediate nodes and
destination from the panel's readable route preview. Saved route legs retain
this annotation when known. The preview describes one quoted flight; its
segments do not become separate itinerary legs or individual fare quotes.
Unknown, incomplete or changing paths stay absent.
[Offline verification](docs/flight-route-preview-verification-2026-11-08.md).

Hosted read-only MCP: `https://mcp.cafas.com.br/scopes/azeroth/mcp`, using a dedicated
bearer token. A separate restricted SSH credential supports the Rust CLI
`azeroth-operator upload <export-file>`. Configure the absolute
`AZEROTH_UPLOAD_SSH_CONFIG` path to its dedicated SSH config; normal administrator
SSH configuration is never used as a fallback. Uploaded bytes are validated and
confirmed by matching content hashes.

For repeatable incremental uploads, use `azeroth-operator sync <retail|classic|forever> <export-file> <receipt-dir>`.
Missing exports report `waiting_for_export`. Matching local acknowledgements skip
SSH connections; failed transfers remain retryable. Receipts bind the snapshot to
the dedicated SSH configuration contents. Keep that configuration self-contained
and use a different receipt directory if its destination changes indirectly.
A skipped upload does not recheck remote storage. Use `upload` to resend after
remote deletion or restoration. This command does not capture live game state.

`tools/sync_exports.py --config <absolute-json-path>` performs one bounded pass
over explicit account roots. Configuration contains absolute `binary`,
`ssh_config` and `state` paths, plus `clients` entries with `family` and `accounts`
(the client's `WTF/Account` directory). Only `SavedVariables/AzerothOperator.lua`
is read from each account. Linked paths are rejected. At most eight exports are
processed, with a process lock and per-transfer timeout. The latest result is
written atomically to `state/status.json`; it contains no account names or paths.

On Windows, `deploy/install_sync_task.ps1 -Config <path> -PythonWindowless <pythonw.exe>`
installs a current-user, limited-privilege task every five minutes and at logon.
The runner is copied to a hash-named local runtime directory. Pin the Rust binary
to a separate installed copy before scheduling; do not point production tasks at
an executable being rebuilt. The task runs only while the user is logged on,
does not wake the computer, rejects overlapping runs and has a ten-minute limit.
It never triggers exports or reloads: run `/aoexport` and reload/log out in WoW
to persist a new snapshot. Remove the task with
`Unregister-ScheduledTask -TaskName AzerothOperator-ExportSync -Confirm:$false`.

The companion is Rust; in-game adapters use Lua. Build with `cargo build --locked`. Run `target/debug/azeroth-operator.exe` for stdio MCP. Set `AZEROTH_DATA_DIR` to the existing absolute snapshot directory private to this installation’s owner. The MCP and local import CLI require this setting. Tools: `capabilities`, `inspect_snapshot`, `list_snapshots` (explicit family, optional offset/limit).

Import a JSON snapshot or the addon's narrow SavedVariables export with `target/debug/azeroth-operator.exe import <export-file>`. Import is a local CLI operation, not an arbitrary-path MCP tool. It reads data only; never executes Lua. See `addon/README.md` for the export workflow and outstanding live-client qualification.

Analysis tools: `character_summary` reports observed counts and unknowns; `compare_snapshots` requires the same character and exact client/build, rejects reversed capture order, and returns at most 100 changed JSON paths per section with truncation metadata. Missing sections are not treated as empty inventories. These are snapshot observations, not recommendations about optimal equipment.

Pass an optional `item_id` to `compare_snapshots` for that base item's before
and after quantities and signed change by storage. A storage change requires
both captures to include that storage. The net change requires all supported
storage sections in both captures. The comparison reports quantity changes
without inferring which individual copies moved.

`review_equipment_selection` creates a player-review JSON draft bound to one
`snapshot_id`. Supply 1–19 `selections`, each with `target_slot`, `expected_item_id`
and a `source`: either `{"kind":"bag","bag":0,"slot":1}` or
`{"kind":"equipment","slot":16}`. IDs must match the exact observed locations;
duplicate targets and reused locations are rejected. The draft shows previous
items, selected items and equipment slots that would be vacated. It does not
apply changes, emit a game import or claim compatible/optimal gear. Game legality,
requirements, capacity and current ownership remain unknown. Schema version 1
does not identify item GUIDs, account or region. This tool is included in the deployed seven-tool
server release; see the dated verification log for deployed qualification.

With a verified local Forever catalogue, an empty selection list reviews the
captured equipment as a simulator baseline.
Forever drafts can also reference observed character-bank and account-bank
locations. See the [equipment review guide](docs/equipment-review.md).

Synthetic performance baseline: `cargo run --release --locked --example benchmark` measures warm-cache listing and inspection over 200 generated snapshots. It uses no account data.

The 2026-11-03 paired Windows benchmark measured listing medians of 91.6ms before
and 69.2ms after borrowing item fields during validation and resolving the store
root once per listing. Each snapshot still receives path-containment, SHA256 and
schema checks on every call; no metadata freshness cache is used. Four alternating
pairs and all samples are recorded in `docs/benchmarks/listing-2026-11-03.json`.
This is a synthetic local result, not a claim about hosted Linux request latency.

The Python prototype is retained temporarily as a reference and independent protocol test harness. Install its test dependencies with `python -m pip install -r requirements.txt`. Set `AZEROTH_TEST_BINARY` to the Rust executable to test that server instead of the prototype.

For a local development addon ZIP: `python tools/package_addon.py --family retail --variant live --interface 120105 --version 0.1.0-dev`. Explicit client metadata is required. Generated `dist/` files are ignored; manifests include per-file and archive hashes. These packages are development artifacts, not certified compatibility claims.

For the pinned upstream packaging path, run `python3 -m tools.package_bigwigs`
with the same client/version arguments in Linux/WSL. It uses isolated staging,
disabled networking during packaging and exact output-file checks. See
[addon packaging](docs/addon-packaging.md) for prerequisites and the packaged Lua
fixture verifier. No release is published automatically.

## Boundaries

Read ordinary addon exports and official API data; analyse and prepare player-reviewed imports. No autonomous gameplay, computer use, memory access, injection, packet interception, protected-action bypasses or arbitrary Lua execution. In-game writes remain disabled pending operation-specific policy review and client verification. A callable API is not proof of Blizzard authorisation.

No character data, account identifiers, credentials or local exports belong in Git.

## Verification

Run `python -m tools.verify_source` for the local source gate after installing requirements.txt and the Rust toolchain. It checks formatting, builds the Rust binary, runs Rust tests and Clippy, then runs the Python suite against that exact target path. Each step has a ten-minute ceiling with process-tree cleanup on timeout; the first failure stops the gate. It does not package, install, upload or qualify a live game client.

Run `cargo build --locked`, `cargo test --locked`, `cargo clippy --all-targets -- -D warnings`, and `python -m unittest discover -s tests -v` from the project root. The Python suite always exercises the Rust stdio server from target/debug (or the explicit AZEROTH_TEST_BINARY override), with a separate Python-reference test. Missing Rust binaries fail the test rather than silently using the reference implementation. These checks do not establish runtime game compatibility.

The current MCP source exposes an explicit offline-analysis allowlist. It refuses legacy tools that generated in-game Apply codes. Historical addon deployments and tests remain documented separately. See the [enforced operation policy](docs/operation-policy.md).


Inspect an explicit installation without launching the game:

```powershell
python -m tools.inspect_client --client 'T:\Games\World of Warcraft\_retail_' --family retail --variant live
```

The probe reads only `.flavor.info` and the parent `.build.info`, with byte limits and link/reparse rejection. Conflicting active versions are reported rather than selected. Family is checked against the product code; variant remains caller-supplied, interface remains unknown and installation metadata does not qualify live APIs. Classic accepts `wow_classic` and `wow_classic_era` products with an explicit variant; unknown products fail closed. Missing directories report `not_installed`; malformed or unreadable metadata exits non-zero. No account data is read.


Check installed addon integrity against a retained package manifest:

```powershell
python -m tools.verify_installed_addon --client 'T:\Games\World of Warcraft\_retail_' --manifest 'T:\Projects\Azeroth Operator\dist\bigwigs-0.1.5\AzerothOperator-0.1.5-dev-retail-live.manifest.json'
```

Exit zero means the four installed files matched the supplied manifest during this read. Missing, changed, extra or oversized files produce nonzero without repairs. The manifest is caller-trusted, not signed authentication; concurrent edits can invalidate the observation. Retail and Forever product checks are supported; Classic installation verification remains unavailable. No live API compatibility is implied.

Operation-by-operation scope, client subdivisions and policy evidence limits are recorded in [operation policy](docs/operation-policy.md). This is a project boundary, not Blizzard certification.


`character_summary.carried_item_review` reports observed carried quantities and repeated item IDs, with up to20 groups and20 locations per group. Totals cover the whole exported bag section even when details are truncated. Locations retain exported bag/slot keys. Different variants can share an ID: this is not item identity, stack compatibility, a move plan or an optimiser. Missing bag data remains null; an observed empty bag map reports zero. Hosted rollout was verified with a synthetic public HTTPS readback on 2026-11-03.
