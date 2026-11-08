# Forever coverage audit — 8 November 2026

This is a source audit of the user's requested end state: broad local MCP
coverage, the active character by default, private owner-bound data, and complete
owned-gear optimisation. It is not a live qualification or a claim of completeness.
Baseline: `c5a0458`. Live testing, installation and deployment remain deferred.

## Coverage and remaining proof

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
