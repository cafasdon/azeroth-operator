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
