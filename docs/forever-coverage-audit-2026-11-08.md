# Forever coverage audit — 8 November 2026

10 November quest reference update: optional local-provider starter/finisher
references now cover the first 64 saved quests through strict export/readers,
saved inspection/discovery and summary. Both readers bind references to the
same source quest IDs/count/prefix. No provider database is bundled or synced;
IDs do not establish native identity or current availability. See
[offline verification](quest-giver-references-verification-2026-11-10.md).

Saved turn-in grouping now uses those retained references with true/false/
unknown readiness, namespace-qualified giver filters, numeric groups and
bounded pages. It preserves unknowns and truncation rather than implying
eligibility or routing. [Offline evidence](quest-turn-in-groups-verification-2026-11-10.md).

Opt-in group reward/budget views now retain saved XP/copper/fixed-item/currency/
renown/honor unknowns and deduplicate quests across alternative finishers before
paging. No rewards are credited or choices applied. [Offline evidence](quest-group-rewards-verification-2026-11-10.md).

Saved instance progress follow-up (10 November). The Forever RaidFrame uses
the same passive saved-instance API, but does not interpret its two progress
positions in Lua. Native interpretation remains pending.
[Forever RaidFrame source](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_RaidFrame/Mainline/RaidFrame.lua#L226)

The Lua fixture and Rust MCP stdio readback passed with readable progress,
and the fixture preserved the base row with missing or restricted progress.
Both readers rejected malformed pairs. Jev's advisory scope check was
inconclusive (0.47, `jev-1.14.0`, 400 input and 20 output tokens,
0.318 seconds, request
`662f9090faff357c4c55f7f99a723162f10e2240d363ae9a0b90857e7a37b731`).

This is a source audit of the user's requested end state: broad local MCP
coverage, the active character by default, private owner-bound data, and complete
owned-gear optimisation. It is not a live qualification or a claim of completeness.
Baseline: `c5a0458`. Live testing, installation and deployment remain deferred.

## Deferred native acceptance checklist

The next native milestone is one owner-local capture that survives export,
persistence, import and MCP readback on the same exact Forever build. Record
the client variant, build and interface, addon version and package hash,
reader revision and snapshot ID for each case. Retain raw evidence in the
owner's store. Project evidence can use redacted results and identifiers
needed to reproduce the check.

The owner opens the relevant panel and runs the addon export manually.
After persistence and import, `inspect_snapshot` identifies recorded sections
and unavailable reasons. `list_snapshots.required_sections` can confirm
that the required sections coexist in one capture. Use
`any_missing_sections` and the comparison capture statuses to distinguish a
recorded refusal from an absent section. The `capabilities` section schema
defines accepted names. Presence alone leaves panel completeness unknown.

| Interaction | Native case and expected MCP readback |
| --- | --- |
| Identity and capture | Capture two explicitly chosen characters. Verify exact client metadata and owner-store isolation, and resolve each named character with time and section constraints. Fresh active identity requires separate evidence covering login, switch, logout and stale observations before it can become the default. |
| Equipment, bags and banks | Capture ordinary carried bags, then character and account banks while the respective storage is available. Check known item IDs, counts and numeric locations, an observed empty container, and a separate unavailable capture. Check exact item variants against the local catalogue before treating gear as resolved. |
| Owned gear and encounter planning | Review an owned pool with known modifiers, one unresolved item and an unavailable storage case. Compare search coverage against the saved pool and check the modelled class, level and encounter separately. A ranked result can only claim coverage of the pool and model actually evaluated. |
| Professions and quests | Capture a visible selected recipe, an available profession list and a quest with known objectives. Check fixed reagents, quantities and one known shortfall against the saved inventory. Repeat with the panel closed or a required reading absent to check the unknown result. |
| Services, dialogue and travel | Capture player-opened merchant, trainer, gossip and flight panels separately. Check a known row against the saved inspection, including reported currency or price where available, then repeat without the panel. Availability and prices describe the capture. |
| Spellbook, action bars and equipment state | Check one exact spell shortcut against its saved spellbook ID, and verify charges or usability only where the client supplied them. Check a recorded weapon enchant and durability reading against the client panel. Unobserved slots and hands remain separate from observed empty ones. |
| Resources, auras, groups and PvP | Compare readable player-only rows with visible state and check an unavailable or restricted case. Verify helpful and harmful lists separately, group categories independently and reported PvP season IDs only when captured. |
| Loot, trade and mail | Use already visible windows. Verify readable loot, the player's own offer and loaded attachment rows in separate captures. Check that the corresponding MCP reports exclude private details and give no ownership credit before a separate inventory observation. |
| Collections and progression | Compare one known row for reputation, currencies, tracked achievements, mounts, pets or toys against the visible panel. Record journal filters or tracked scope and verify that omitted rows do not imply non-ownership or non-completion. |
| Saved instances and world bosses | Compare previously populated saved-cache entries with the client display, including a reset time and unavailable cache case. Record the cache age and check that an omitted row gives no eligibility claim. |

For each case, record **verified**, **partial**, **unavailable** or **untested**
for the exact build and interaction. A partial result records which rows or
fields were verified and which were unknown. A qualification entry links the
native evidence, the saved capture and the MCP readback. At this checkpoint,
these native cases remain untested. The existing offline evidence below
qualifies its fixtures only.

### Offline package baseline

Source `503ed28` passed a fresh development package verification using
the local packager and the Rust `verify_addon` example. The synthetic
Forever `live` package used interface `16001` and version `0.0.0-dev`.
The four-file ZIP had SHA-256
`3b1ef1813cc426645994360babefcd70bbbe02956898742423c16d24d24af4a7`.
The verifier passed the packaged Lua 5.1 round trip, the Forever
capture fixture and changed-interface rejection.

Three separate local MCP stdio tests passed for character resolution,
weapon enchant and section coverage, and the offline tool catalogue.
These tests used saved fixtures. The package was temporary and was
neither installed nor published. Native qualification stays untested.

## Coverage and remaining proof

9 November update: optional local provider NPC/object spawn references now reach
strict snapshots and scoped reagent-chain acquisition output. [Offline evidence](reference-locations-verification-2026-11-09.md).
Provider area/phase namespaces remain separate from native maps. Native acceptance,
dataset revision/accuracy, coordinate-frame mapping, visibility and actual routing
remain unqualified; this is bounded reference coverage, not a full world database.

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

Recipe search can order saved recipes by captured cooldown time remaining.
Three offline MCP tests passed, including unknown timers, charge-aware state,
filter paging and original timer provenance with newer material counts.
Jev 1.14.0 advisory `5a0b95f72034b7488f82a1fdc01946a4277334181d71069aaae65d398e5ddc24`
favoured integration at .48 confidence, with 396 input and 49 output tokens
in .312 seconds.

Recipe search can now order matches by captured skill-up counts from the same
profession and recipe. Offline MCP tests verified zero and unknown ordering,
numeric ties, filters, paging and mismatched profession evidence. Jev 1.14.0
advisory `2a6133f5c4a67b8b4b34606a4de16b29216139a5372847f686a979dd63a550e6`
favoured integration at .33 confidence, with 397 input and 49 output tokens
in .301 seconds.

Owned-item review now accepts `simulator_input_resolved` to page through
resolved inputs or investigate unresolved locations. Full-capture evidence stays
separate from filtered totals. Offline build, caller tests and independent
failure probes passed, including 54 pagination combinations and six rejected
malformed boolean inputs. Jev 1.14.0 advisory
`1e46afe077da10daff0981a115ccd9d811291b80e50404ee6c6a58137e1bcf8c`
returned integrate .82 at confidence .73, with 407 input and 49 output tokens
in .288 seconds.

Owned-item review now exposes `simulator_item_resolution`. The fixed tags
distinguish catalogue gaps, unreadable item links, unavailable modifier values,
unresolved suffix or enchant references, and other simulator resolution gaps.
The first applicable cause is shown when multiple gaps exist. The ranker carries
recognised unresolved tags into plan exclusions. A resolved tag describes the
availability of a simulator item input. Native equip eligibility is assessed
separately.

Offline build and five scoped tests passed. The actual plan CLI returned the
three expected reasons for an incomplete link, unknown suffix and unknown enchant.
Independent Callers and Failure reviews checked legacy responses and unrecognised
tags. Jev 1.14.0 advisory `03d5a63b14280f586cf304981f12baa58d26c9cf89796e2853fa4bb0b9088e12`
returned integrate .88 at confidence .82 with 394 input and 49 output tokens in .410s.

Owned-gear reports now include `unresolved_candidate_locations_by_slot` for
locations that reached item resolution and were omitted before enumeration.
One physical item may appear under multiple eligible slots. The merge and journal
report the pool counts once across windows. Older reports show `null` for this
evidence. Coordinate completion applies to the enumerated pool, with candidate
resolution gaps and unavailable inventory reported separately.

Offline verification passed 13 tests across rank collection, window merge,
page reuse and owned evaluation. The local MCP fixture exercised a known ring
with an incomplete link through the final evaluator. Both independent review
lenses checked legacy handling, scope agreement and malformed counts. Jev 1.14.0
advisory `5f1b606447d95c8f1e4b100def27cc3f1eced7613f63df785b6e7ca4a87539b0`
was inconclusive at .11 confidence, with 418 input and 53 output tokens in .284s.
Integration rests on the direct tests and independent reviews.

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
historical and do not prove current item usability. See [verification](item-cooldown-lookup-verification-2026-11-09.md).

Python ingestion and existing-capture reads now enforce the Rust item stack-count
contract: container rows require positive u32 integers; equipment permits absent
or null counts only. Invalid ingestion fails before writes. Count parity is
verified with 120 actual MCP cases; broader schema equivalence remains separate.

Historical bag cooldown inspection supports item IDs, explicit status, enabled
flags and inclusive timer bounds through bag_cooldown_filter. Numeric bag/slot
ordering and bounded pages preserve full pre-pagination matched/unknown/nonmatch
counts. Missing observations remain null; known empty captures remain empty.
This does not establish current item usability. See [verification](bag-cooldown-filter-verification-2026-11-09.md).

Base inventory ingestion now follows the existing Rust structure, item-field,
identity, UTF-8 and bank-container contract, rejecting unsupported fields before
writes. Invalid saved captures are refused without rewriting them. Verified with
241 actual MCP structure cases; full snapshot-schema equivalence remains separate.

Saved-character item discovery uses list_snapshots.inventory_item with an
explicit storage and observed quantity bounds. Unknown storage does not match
zero; newest-per-character selection runs before the quantity predicate.
Shared-bank observations remain separate and are never summed across characters.
The owner-local query needs no simulator catalogue and does not identify the
active player or verify present stock/access. See ADR 049 and [verification](inventory-item-discovery-verification-2026-11-09.md).

Shopping/reagent discovery can require all 1..64 requested item quantities within
one explicit saved storage, optionally alongside captured profession filters.
Partial stock across characters, storages or times is never pooled. Material
support does not verify current crafting or transfer eligibility. See [verification](inventory-requirements-discovery-verification-2026-11-09.md).

Forever talent observations can retain stable native definition IDs, purchased
ranks, native rank limits and an observed staged-change flag. Boosted active
ranks remain separate and uncertain metadata stays unknown. No simulator talent
conversion or allocation match is verified. Addon 0.47.0-dev remains uninstalled;
see [verification](forever-talent-definitions-verification-2026-11-09.md).

Simulator/native talent bridge: pinned local nine-class definition positions now
check known purchased ranks in equipment reviews before simulation. Staged or
known conflicting observations require correction; partial/unknown and full
allocation/model coverage stay unverified. No automatic talent actions or live
qualification. See [ADR-051](ADR-051-forever-simulator-talent-observations.md).

Direct MCP purchased-talent comparison now reaches the existing talent review
route via an optional simulator string and an explicit saved Forever snapshot.
It preserves native import unavailability, partial/unknown evidence and false
full-allocation/model flags. No simulator or game runs through this query.
[Contract](ADR-052-forever-talent-template-mcp.md) ·
[Offline checks](forever-talent-mcp-verification-2026-11-09.md).

Typed loot extension: raw reported currency IDs/quantities and money-slot
presence can now be captured separately. No container reward conversion, money
amount, owned balance or pickup right is inferred. Item requirement review
separates known non-item rows from unreadable rows. Native qualification remains
pending. [Contract](ADR-053-forever-typed-loot-observations.md).

The spellbook restricted-enum/getter gap found during typed-loot verification is
now fixed in source, and its earlier fixture exclusion is removed. Optional spell
metadata failures preserve base rows. Offline guards/package checks do not prove
native availability or all-collector secret-state completeness.
[Evidence](spellbook-guards-verification-2026-11-09.md).

Saved-loot discovery now finds item/raw-currency quantities or money presence in
one historical owner capture, retaining unreadable rows and false ownership,
reward, balance and live flags. Latest-valid selection remains before feature
predicates; no character pooling or game action. Native/hosted qualification
remains pending. [Contract](ADR-054-saved-loot-discovery.md).

Queried allocation bridge: optional observations now include inactive explicit
purchased zeros, with all-entry identity/rank linkage and bounded capture.
Explicit MCP simulator preparation requires exact pinned definitions/caps and
known committed state; missing data yields no string. Full native/model/legality,
current identity and owned-BiS proof remain open. Uninstalled package only.
[Contract](ADR-055-queried-talent-allocation.md).

Simulation preflight now consumes queried allocation observations, including
explicit inactive zeros. Known committed conflicts reject before engine/cache use; missing
or model-unmapped evidence remains explicit, with no complete native/model/BiS
claim. [Contract](ADR-056-queried-allocation-simulation-checks.md).

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

## Policy evidence checkpoint (2026-11-09)

The capabilities report now separates enforced offline controls from unverified
Blizzard approval and incomplete applicable-terms evidence. Three official
opening posts were rechecked; direct full EULA requests were refused. No action
is enabled or installation changed. Fresh active-character selection, complete
owned-pool/native/model proof and full owned-BiS remain open; no newest-snapshot
fallback or partial-winner substitution closes them. See
[dated evidence](policy-evidence-verification-2026-11-09.md).

## Final gear encounter context (2026-11-09)

Saved-journal rankings and the unified owned-gear evaluator now retain checked
`encounter_contexts` for visited encounters: index, normalised weight, engine and
simulation-profile digests, source fingerprints, numeric encounter settings and
nullable baseline identity/metrics. `unvisited_encounter_indices` records missing
contexts. Ranked-loadout evidence includes its metric and baseline metric value,
even if top-N excludes the baseline loadout. No raw templates, names, file paths
or private iteration samples are added. Ranking, work budgets and completeness
flags are unchanged; no full BiS, native/model accuracy or statistical proof is
implied. [Contract](ADR-059-ranked-encounter-context.md).

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

## Saved base skill targets (2026-11-10)

Saved base skill inspection accepts an explicit target rank and reports base
points still needed, whether the captured base rank reached it, and whether
it exceeds the captured cap. Bonuses are excluded and no training eligibility
is inferred. See ADR-076-saved-base-skill-targets.md. Native testing deferred.

Skill planning can also order matching saved rows by points to the observed cap
or supplied target before paging. Numeric IDs resolve equal distances. Targets
are required for target-distance order; reached targets have zero distance.
This supports closest-goal inspection without implying cheapest training or
craft feasibility. Offline source only; native testing remains deferred.

## Passive owned-auction cache (2026-11-10)

The manual Forever exporter and both snapshot readers now support a bounded,
numeric-only owned_auctions section for the already-loaded cache in an open
AuctionHouseFrame. Existing inspection and section-status discovery apply.
Hidden/restricted/changing/invalid cache observations stay unavailable. No
query, refresh, transaction, third-party identity, inventory credit or wallet
credit is introduced. Current ownership, cache freshness/completeness and
native availability remain unverified. See ADR-077-passive-owned-auctions.md.
Update readers before exporter installation; live testing remains deferred.

Owned-auction discovery now supports exact same-row item, auction ID and raw
status predicates plus bounded inspection pages. Latest-per-character selection
precedes matching, numeric slots determine page order, and whole-match quantities
remain historical reported units with no stock/copper credit. Missing sections
remain unknown. No capture, transaction or native qualification is added.

Saved auction comparison now reports numeric IDs observed on only one side,
field changes on common IDs, full counts and bounded ordered detail. Missing
cache observations keep differences unknown. Slot reordering is ignored;
disappearance does not prove a sale/cancellation and totals never establish
stock or proceeds. Existing identity/client/owner gates remain. Offline only.

## Passive battlefield slot status (2026-11-10)

The manual Forever exporter supports a status-only battlefield_queue_slots
observation from two passive getters. Existing owner-local inspection and
capture-status discovery apply. Only reported capacity and allowlisted raw
status tokens are retained, with bounded two-pass agreement and strict readers.
It is not full queue coverage, current membership, eligibility or acceptance
proof. No join/accept/leave/query/refresh, polling, names, timers, teams or
scoreboard data is introduced. Other queue systems and native qualification
remain open. See ADR-078-passive-battlefield-queues.md. Live testing deferred.

Saved battlefield summary now groups all five raw status tokens into counts
and numeric slot lists, preserving null, zero-capacity and all-none cases.
Non-none counts are not active-player counts or membership/acceptance proof.
No new capture, action or native qualification was introduced.

Battlefield discovery now finds saved exact raw tokens by optional slot and
minimum matching count. Latest-per-character selection precedes matching;
missing/empty/nonmatching captures do not resurrect old observations. Matches
retain source capacity and all bounded numeric slot matches. No new capture,
queue action, current-membership proof or native qualification is introduced.

Battlefield comparisons now show positional token differences and reported
capacity changes, keeping missing/unrecorded separate from none and known zero.
All changed numeric slots are retained within the existing 32-slot bound.
Slot identity/continuity and join/leave events are explicitly unverified; no
new capture, action, membership proof or native qualification was introduced.

## Passive player life-state flags (2026-11-10)

Manual Forever exports now support player-only dead, ghost, dead-or-ghost and
feign-death Boolean observations, with bounded two-pass agreement and strict
readers. Existing inspection and capture-status discovery apply. Unreadable
flags never become false. No classification, present-state/eligibility proof,
other-player observation, public indexing or release/resurrection action is
introduced. See ADR-079-passive-player-life-state.md. Native testing deferred.

Saved player life flags now support raw character summaries, same-capture
Boolean discovery (including false) and before/after flag differences. Latest
selection precedes search; missing sides retain unknown comparisons. No life
classification, death/resurrection event, eligibility, gameplay action, new
capture or native qualification is introduced.
