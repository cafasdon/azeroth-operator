# Forever coverage audit — 8 November 2026

This is a source audit of the user's requested end state: broad local MCP
coverage, the active character by default, private owner-bound data, and complete
owned-gear optimisation. It is not a live qualification or a claim of completeness.
Baseline: `c5a0458`. Live testing, installation and deployment remain deferred.

## Coverage and remaining proof

9 November update: saved loot observations now support conditional comparison
with explicit item requirements, beyond inspection/summary. [Offline evidence](loot-requirements-verification-2026-11-09.md).
Unreadable quantities, actual pickup rights/availability and native qualification
remain unknown; displayed loot is never credited as owned stock.

9 November update: spell resources now support saved-capture discovery and
same-row inspection filtering, closing the preceding capture-to-search gap.
[Offline evidence](spell-resource-search-verification-2026-11-09.md). Current
casting eligibility, fresh active identity and native qualification remain open.

9 November update: readable spell charge/cast-count/cooldown-flag observations
now reach strict local readers, row summaries and exact reported-ID action-bar
references. [Offline evidence](spell-resources-verification-2026-11-09.md). Native
behaviour, complete spell coverage, recharge/timing and actual castability remain
unqualified; no combat automation or gameplay command is introduced.

9 November update: passive visible-loot-window item capture, strict local readers
and item/unknown-slot summary add a previously uncovered system. Displayed loot
stays outside ownership and stock calculations. [Offline evidence](loot-window-verification-2026-11-09.md).
Native availability, source attribution, currency/money semantics, roll rights and
actual loot eligibility remain unqualified; all loot actions stay with the player.

9 November update: selected crafting-source acquisition projections now retain
the review's item-capacity basis, combined saved charge limit and cooldown state.
[Offline evidence](catalogue-recipe-capacity-verification-2026-11-09.md). This
closes the projection gap recorded in the zero-item capacity review without
merging selected/bulk observations or verifying current eligibility.

9 November update: recipe preparation now distinguishes explicit no-item limits
from unavailable material evidence and combines known charges for those recipes
even without inventory. [Offline evidence](zero-item-recipe-capacity-verification-2026-11-09.md).
Other requirements and native/current craftability remain unqualified.

9 November update: repeated producer steps now share a per-recipe captured charge
ledger in chain and stock preparation, closing independent full-charge reuse.
[Offline evidence](chain-charge-accounting-verification-2026-11-09.md). Shared
cooldown groups, recharge, current readiness and full profession planning remain
unqualified.

9 November update: bulk recipe discovery now filters original captured cooldown
state, charges and remaining seconds alongside view/material predicates. Missing
numeric evidence remains unknown and newer inventory never refreshes cooldowns.
[Offline checks](recipe-cooldown-search-verification-2026-11-09.md). Native
availability, present craftability and complete profession coverage remain open.

| System | Implemented source evidence | Remaining requirement |
| --- | --- | --- |
| Character selection | Explicit name/realm selection and saved snapshot listing in `src/main.rs` and `src/lib.rs`; selection contract in `clients/forever/README.md` | Fresh verified active-character signal and default selection; newest saved snapshot does not establish the character in play. Explicit targets must continue to override the default. |
| Inventory and maintenance | Equipment, bags, character/account banks, capacity, durability, native stat/location/price observations in `addon/Core.lua` and `addon/Collectors.lua` | Native availability qualification for each storage; partial observations cannot establish complete ownership or accessibility. |
| Owned gear and legality | Owned-location review, projected equipment and catalogue resolution in `src/planning.rs` and `src/forever_catalogue.rs` | Complete owned pool and full-set legality evidence. Existing guards include native level/category conflicts, catalogue requirements, weapon pairing, unique items and shared limit categories; these are partial evidence, not full live equip eligibility. |
| Simulation and optimisation | Pinned engine adapter, encounter ranking, deduplication, caching and shortlist qualification under `tools/` | Complete feasible combination coverage, broader classes/levels and encounter accuracy. A shortlist winner or weighted winner is not unconditional owned best-in-slot. See `docs/forever-simulation-research.md`. |
| Local reference data | `tools/sync_forever_catalogue.py` and local catalogue lookup | Establish which additional world, acquisition and encounter datasets are needed, their version/licence/provenance, and bounded refresh behaviour before adding them. Cached simulator data is not a complete world database. |
| Professions | Skill lines, recipes, reagents, skill-up metadata and recipe review | Qualify native filtered-list coverage; distinguish observed recipes from complete profession knowledge and feasible progression. Crafting remains a player action. |
| Quests and travel | Quest objectives/rewards, saved flight panels, route previews, positions and itinerary analysis | Complete world/acquisition routing and actual encounter/travel behaviour remain unproven. Captured quotes and paths are historical. |
| Pets, spells and configuration | Pet care/action observations, spellbook metadata, talents, bindings, macros and action-bar review | Native continuity, completeness and current eligibility remain unproven. Saved usability or range flags do not authorise commands or casting. |
| Economy and services | Merchant stock, trainer services, repair quotes, mail attachments and spending review | Complete service availability and current transaction eligibility remain unproven. Reference values and pending attachments are not spendable funds or owned inventory. |
| Collections and progression | Reputation, currencies, tracked achievements, mounts, pet/toy views, experience, saved instances/world bosses | Filtered/visible/tracked observations are partial. Unlisted rows do not prove non-ownership, non-completion or current lockout availability. |
| Privacy and policy | Owner-store requirement and offline policy capabilities in `src/main.rs`; strict readers in `src/schema.rs` and `shared/snapshots.py` | Verify each new source/output against the same owner boundary. No cross-user dataset, automatic gameplay execution or absolute policy-compliance guarantee is established by these checks. |

## Implementation order

1. Close character selection before adding defaults that could act on stale
   identity. A resolver must distinguish explicit selection, verified fresh
   activity, stale activity and unknown activity. No guessed fallback to the
   most recently saved character. Native signal qualification stays deferred.
2. Close the owned-gear completeness contract. A result must expose observed
   storage, unavailable storage, resolved and unresolved items, legality
   blockers, searched combinations, omitted combinations and encounter scope.
   Only complete qualified coverage can support an absolute best-in-slot claim.
3. Extend the local reference-data contract around concrete missing tasks:
   acquisition sources, profession progression and encounter mechanics. Keep
   public reference data separate from private character snapshots; preserve
   versioned provenance and explicit unknowns when a source cannot answer.
4. Expand remaining systems from a task-level inventory, rather than counting
   optional fields as proof of full system support. Qualify inspection, search,
   comparison and planning separately for each system. Any player-applied
   configuration workflow needs its own receipts and native acceptance.

## Evidence boundaries

The audit inspected the current section registry, MCP routes, selection
documentation, catalogue legality checks and simulation research record. It
does not retest their historical results or assert that every callable route is
listed here. No game action, external data sync or live deployment was performed.
The broad goal remains incomplete; this map keeps its largest missing outcomes
visible alongside the implemented observations.

The subsequent [profession bulk-data audit](profession-bulk-data-audit-2026-11-08.md)
pins the documented per-recipe schematic API and separates the existing
known-spell ID capture from the missing bulk material capture. Native
availability and responsiveness remain unqualified.

The subsequent [owned-gear search gap audit](owned-gear-search-gap-audit-2026-11-08.md)
verifies the current 1,024-combination and 128-alternative limits with synthetic
real-MCP probes. Identical-copy collapse works, but a successful single-swap
search does not establish full two-slot or full-set coverage. Complete owned
optimisation remains an open requirement.

The subsequent [combination-window increment](gear-combination-windows-verification-2026-11-08.md)
adds opt-in bounded coordinate execution for larger products. Per-call budgets
remain; cross-window aggregation, a persistent search journal and complete
owned-gear/model qualification remain unfinished.

Later [plan-window aggregation](gear-window-aggregation-verification-2026-11-08.md)
and a [local transactional journal](gear-window-journal-verification-2026-11-08.md)
group reported inputs and preserve coverage across restart. This is conditional
planning metadata; automatic collection, simulation ranking aggregation and
complete owned-gear/native/model proof remain open.

The later [resumable collection controller](gear-window-collection-verification-2026-11-08.md)
automates bounded offline plan-window collection with explicit snapshot and
input binding. Simulation ranking aggregation, full owned-gear/model proof,
active-character qualification and native testing remain open.

Later [resolution accounting](gear-resolution-accounting-verification-2026-11-08.md)
exposes reported baseline, exclusion, resolved-instance, equivalent and unresolved
counts across merged windows and restart status. It distinguishes collected
coordinates from fully resolved proposals, without establishing feasible-set
coverage or simulation ranking.

New [coordinate outcomes](gear-coordinate-outcomes-verification-2026-11-08.md)
make individual saved-window gaps diagnosable with fixed reason codes and local
review references. Legacy payloads retain explicit unavailable details; this
does not promote collected or resolved plans into simulation/BiS qualification.

Later [quest objective filtering](quest-objective-filter-verification-2026-11-08.md)
extends saved quest discovery by exact objective kind, finished flag and remaining
units on the same objective row, composable with reward/readiness searches and
existing projections. Missing observations stay unknown; native and complete
world-database coverage remain unqualified.

Later [achievement filtering](achievement-filter-verification-2026-11-08.md)
adds local discovery by completion/character credit/account-wide flags and
same-criterion progress. Tracked-only coverage and unknown quantities/IDs stay
explicit; no complete achievement catalogue or native qualification is implied.

Later [reputation filtering](reputation-filter-verification-2026-11-08.md) adds
visible-faction discovery by progression kind, caps/rank distance and captured
paragon reward state. Explicit kind requirements keep different progression
units separate; missing details remain unknown and native eligibility is unproven.

Later [currency filtering](currency-filter-verification-2026-11-08.md) adds
visible-currency discovery by recorded flags, positive caps and weekly/total
headroom. It retains cap basis and unknown earning observations; current earning
eligibility, reset behaviour and complete currency coverage remain unqualified.

On 9 November, [optional retained gear inputs](retained-gear-inputs-verification-2026-11-09.md)
add canonical numeric material to saved resolved drafts. This reduces future
recollection needs but does not persist a qualified character/encounter job,
execute simulations or complete global ranking/full owned BiS.

Later [window request preparation](gear-window-preparation-verification-2026-11-09.md)
rechecks retained saved selections via local MCP and applies existing adapter
guards before producing bounded requests. It skips candidate enumeration but
still needs fresh reviews; engine execution, resumable result storage and global
simulation ranking/full owned BiS remain unfinished.

Later [prepared-window aggregation](prepared-window-merge-verification-2026-11-09.md)
checks selected source journal rows, groups identical gear requests under one
shared profile and preserves source references/coverage gaps. Request grouping
does not execute simulations, authenticate artifacts or complete global ranking.

Later [numeric simulation checkpoints](simulation-journal-verification-2026-11-09.md)
preserve fixed-run results beyond the optional cache's 128-entry eviction limit.
Current callers retain model/engine guards and can reuse completed numeric work;
automatic prepared-set execution, scalable scheduling/global ranking and full
owned BiS remain unfinished.

On 9 November, [saved-window execution](saved-window-execution-verification-2026-11-09.md)
connects fresh MCP preparation, compatible request grouping and private numeric
checkpoints under explicit work budgets. It ranks selected completed observations
and retains missing/uninspected work. Native qualification, full ownership/model
coverage, automatic all-window scheduling and full owned BiS remain unfinished.

Later [saved-window encounter comparison](encounter-window-execution-verification-2026-11-09.md)
adds shared budgets across up to eight DPS profiles, weighted gains only after
complete comparable results, and point-estimate sensitivity. Checkpoint-only
inspection still revalidates source/model inputs. This is bounded selected-work
comparison, not full ownership/encounter qualification or unconditional BiS.

The [real simulator saved-window qualification](real-saved-window-verification-2026-11-09.md)
subsequently exercised this path with the pinned executable and public Hunter
rotation against sparse synthetic captures: four resumed unique requests,
six checkpoint hits on retry, and exact agreement with four uncached reference
runs. This qualifies persistence and execution for that fixture only; live
and full owned-BiS qualification remain pending.

The [saved-journal page runner](journal-page-execution-verification-2026-11-09.md)
then adds indexed next-window selection and an explicit continuation cursor,
with partial-page retries using existing checkpoints. It supports processing
larger journals without manual offset lists, while leaving past-page/global
ranking and full owned BiS unverified. Automatic invocation scheduling and
global result aggregation are still unfinished.

The [saved-journal ranking increment](saved-journal-ranking-verification-2026-11-09.md)
subsequently traverses pages automatically and combines their completed weighted
DPS observations into bounded distinct-gear leaders. It distinguishes all saved
resolved requests from completed-page observations and revalidates source and
cross-page model identity. Repeated invocations reuse checkpoints. Full owned
inventory, feasible sets, model/encounter qualification and absolute owned BiS
remain unfinished; cached numerical results alone cannot establish them.

The [real combined-journal qualification](real-saved-journal-verification-2026-11-09.md)
subsequently checks two saved pages against pinned actual simulator results,
including warm reuse, cold bounded resumptions, exact metrics and gear draft
references. It remains a fixed-seed sparse synthetic fixture; broad native,
ownership, feasibility and model qualification remain open.

[Material-supported recipe discovery](recipe-material-search-verification-2026-11-09.md)
then adds an optional same-snapshot bags/all-observed-storage filter to bulk recipe
search. Independent exact-integer and planner comparisons cover missing storage,
large captures and paging. It answers material-support questions without
inferring learning, live craftability, complete knowledge or profession
progression; those broader requirements remain unfinished.

[Recipe-view discovery predicates](recipe-view-search-verification-2026-11-09.md)
subsequently combine captured learned/skill-up observations with optional
material support through exact same-profession joins. Missing and conflicting
evidence is not promoted into verified learning or gains. Full levelling
progression and native profession qualification remain open.

[Captured reagent-chain drafts](reagent-chain-verification-2026-11-09.md)
then connect an exact bulk recipe goal to bounded unambiguous intermediate
producers and external material needs, with a shared stock ledger and explicit
blockers. The model does not execute actions, establish acquisition availability,
verify yields/ownership or optimise the full crafting route. These and native
profession qualification remain unfinished.

[Explicit captured producer choices](reagent-producer-choices-verification-2026-11-09.md)
then let a caller resolve reagent-chain ambiguity without selecting recipes by
guesswork. Same-output/capture validation, demand-driven usage and existing
producer/context/budget guards remain. Conditional drafts still do not verify
availability, craftability, ownership or optimality.

[Captured chain-leaf merchant quotes](reagent-chain-merchant-verification-2026-11-09.md)
then connect complete external material demand to saved bundle prices and the
inventory wallet. Historical quote totals preserve missing/overflow unknowns,
context guards and false purchase/craftability flags. Current availability,
actual acquisition and whole-route optimisation remain unqualified.

[Chain stock preparation](reagent-chain-stock-verification-2026-11-09.md)
then adds minimum starting needs for printed steps and historical bank withdrawal
quantities, with separate bags/all-observed coverage and unknown nonmaterial
checks. Future acquisitions are excluded from owned preparation stock. Native
bank access, current readiness, actual transfers and global route quality remain
unfinished.

[Reported recipe craftable counts](reported-craftable-count-verification-2026-11-09.md)
then add optional stable public count observations to retained fixed bulk rows.
Exact reviews preserve recipe-capture provenance and requested-craft deltas,
independently of material capacity. No bank scope, learning, charge, cooldown or
current eligibility is inferred. Native availability, count accuracy and export
responsiveness remain unqualified; broad profession progression stays open.

[Chain acquisition references](chain-catalogue-sources-verification-2026-11-09.md)
then join net external shortage IDs to pinned cached public source records in
one review. Match gaps, absent metadata and truncation remain visible. This
connects crafting analysis to existing local reference data, but does not fill
missing common reagents, establish full world/acquisition coverage, choose a
route or verify current eligibility. These larger requirements remain open.

[Optional owner-local reagent references](recipe-item-sources-verification-2026-11-09.md)
then add source-only capture through a separately installed compatible provider's
public API. This can preserve source IDs beyond the simulator cache, but only
the bounded fixed-reagent subset queried during export; the provider database is
not bundled and its revision/completeness is unverified. Inspection is supported;
chain integration, source/route resolution and native acceptance remain open.

[Captured chain acquisition references](chain-captured-sources-verification-2026-11-09.md)
then join complete net external demand to an explicitly scoped owner capture,
with independent source timestamps, partial/truncated categories and missing
data statuses. This closes the preceding capture-to-chain join gap, without
filling source entities/coordinates, world coverage, optimal routing, current
availability or native qualification. Full acquisition planning remains open.

[Unified observed-owned gear evaluation](owned-gear-evaluation-verification-2026-11-09.md)
then connects all-slot collection and saved multi-encounter ranking through one
resumable offline command with inherited deadlines. Reported coordinates,
unresolved plans and completed saved simulations are accounted separately. A
real pinned-engine check preserves an unresolved paired-slot placement rather
than treating the resolved winner as full owned BiS. Complete ownership, native
legality, unsupported models and statistical optimality remain open.

[Isolated positional engine qualification](positional-engine-verification-2026-11-09.md)
then demonstrates exact paired-slot conversion and limited canonical/sparse
simulation compatibility under an explicit contract. It does not yet connect
that contract to MCP/adapter requests; existing unresolved guards remain active.
Strict engine-capability wiring is the next step, with native/full model/owned
coverage and BiS proof still open.

[Positional MCP integration](positional-integration-verification-2026-11-09.md)
subsequently connects explicit exact-slot reviews, retained collection and
saved multi-encounter evaluation. Qualified hash/capability checks precede
execution and checkpoint reuse; legacy placement remains the default. The
synthetic second-ring case resolves all four reported coordinates and ranks
three resolved gear inputs across two encounters. Complete ownership, native
legality, model accuracy and statistical optimality remain unverified.

[Ranked saved loadouts](ranked-loadout-verification-2026-11-09.md) then attach
exact retained equipment and saved physical-source selections to ranked journal
results. This makes the recommendation inspectable beyond a hash/reference;
current locations, native equip eligibility and full owned BiS remain open.

[Worst-encounter ranking](robust-gear-ranking-verification-2026-11-09.md) adds
an explicit objective that maximises minimum DPS gain across supplied encounters,
alongside the existing weighted-average default. It uses the same completed
numeric simulations, retaining partial-scope and point-estimate limitations.
This improves encounter-sensitive recommendations without claiming full owned
or statistical optimality.

[Recipe inventory refresh](recipe-search-inventory-verification-2026-11-09.md)
then extends fixed-material discovery to an explicit newer compatible inventory
capture. Recipe/view evidence retains its original provenance and missing bank
data is never backfilled from the older recipe capture. Actual current learning,
craftability, full profession coverage and native qualification remain open.

[Inventory evidence propagation](gear-inventory-propagation-verification-2026-11-09.md)
closes a reporting gap between saved gear plans and final optimisation results.
Observed/unavailable storage and unmatched catalogue IDs now remain visible
through aggregation and restart, even when all reported simulations finish.
Legacy evidence stays unknown and full owned-pool/native/model proof stays open.

[Equipped-source vacancy search](equipped-vacancy-verification-2026-11-09.md)
then closes an enumeration omission for explicit positional searches: equipped
items can move between requested slots without refilling their old slot. Exact
saved layouts survive preparation and offline simulation. Other feasible-set,
native/model/ownership and full owned BiS requirements remain open.

[Role-aware gear evaluation](role-gear-ranking-verification-2026-11-09.md)
removes the upper workflow's forced-DPS restriction, carrying all six existing
simulator metrics through multi-encounter journal ranking and unified evaluation.
Native-unit role improvements handle zero baselines and preserve objective/
checkpoint boundaries. A limited actual offline Warrior baseline qualifies
metric plumbing; broader healer/tank/class/encounter accuracy and full BiS remain
open, including upstream healer presets that provide stats without a rotation.

[Bulk recipe cooldowns](bulk-recipe-cooldown-verification-2026-11-09.md) extend
known fixed-recipe material captures with optional stable public timers/charges.
These reach search, charge coverage and batch nonmaterial checks without moving
their provenance to newer stock captures. Native API responsiveness/completeness,
current craftability and full profession progression qualification remain open.
