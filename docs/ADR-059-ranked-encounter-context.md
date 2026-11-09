# ADR-059: Preserve final gear-ranking encounter context

Date: 2026-11-09. Status: accepted for offline source integration.

## Problem

The saved-journal runner checked engine/profile fingerprints, encounter settings
and baseline identity across pages but omitted these checked collections from
the final report. A standalone recommendation retained only encounter indices
and request hashes. With top=1 the baseline could disappear entirely, so the
reported gain’s denominator and encounter scope were not directly auditable.

## Decision

Add `encounter_contexts` to saved-journal reports, retained transitively in the
unified evaluator’s existing `simulation` object. One entry per visited encounter,
ordered by its one-based index, contains normalised weight, `engine_sha256`,
`simulation_profile_sha256`, source `fingerprints`, `encounter_settings` and
nullable `baseline`. Settings use the existing numeric/hash projection: duration,
variation, target count, execute proportions, requested iterations and full
encounter/settings digest. The digest identifies omitted detailed settings; this
is not a complete encounter description or a verified mechanic model.

Baseline retains the already checked gear/request digests, iterations, seed,
mean DPS and DPS deviation, plus the selected metric value for non-DPS ranking.
It is retained even when excluded by top-N. An encounter visited without a
readable baseline has null baseline; no synthetic zero is introduced. Empty work
has an empty context list. `unvisited_encounter_indices` lists requested indices
without a visited context, not every incomplete simulation.

Each ranked-loadout encounter evidence row additionally reports `metric` and
`baseline_metric_value` from the existing validated role-metric reader. Baseline
request digest links the row to its context. Normalised encounter weights and
metric direction retain their previous meaning. No private samples, template
payloads, names, account data or local paths are copied.

## Boundaries

Existing cross-page checks, ranking/ties, work limits, checkpoint bindings,
partial-page treatment and false global/BiS/feasible-set flags remain unchanged.
At most eight contexts exist. A context does not imply that all requested gear
simulations for that encounter finished. No active-character selection, new
simulation, game action, addon change or deployment is introduced by this report
projection. Native/model accuracy and selection-adjusted statistical optimality
remain unverified. [Evidence](ranked-encounter-context-verification-2026-11-09.md).
