# Forever simulation research — 2026-11-08

## Iterations and encounter accuracy

One iteration simulates a fight under a specified encounter and character
configuration. Repeating it samples random outcomes; it does not add missing
boss mechanics. More samples reduce sampling error, but cannot correct an
inaccurate encounter model.

[SimulationCraft statistical behaviour](https://github.com/simulationcraft/simc/wiki/StatisticalBehaviour)
documents fixed iteration counts and stopping at a target error. Its
[raid events](https://github.com/simulationcraft/simc/wiki/RaidEvents) include
movement, add waves and sequential dungeon pulls. These SimC features do not
establish that the Forever engine supports the same mechanics.

## How established tools save work

[Raidbots Smart Sim](https://support.raidbots.com/article/55-smart-sim)
screens combinations at low precision, then refines survivors. Its published
stage targets are 1%, 0.2% and 0.05% error. Small enchant differences can require
many contenders to reach expensive stages.

[Raidbots sidegrades](https://support.raidbots.com/article/61-top-gear-sidegrades)
groups close results using estimated error rather than treating every small
numerical difference as a meaningful upgrade.

The [pinned Forever stage implementation](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/core/bulk/stage.go)
already supports staged precision and carries prior samples forward using
iteration seed offsets. Replacing it with independent full runs would discard
that benefit.

## Current local implementation

The ranker already rejects unresolved gear, deduplicates identical simulator
inputs, calls the engine's bulk path, reuses identical encounter exports within
one invocation and optionally confirms the leading pair on a fresh seed.
It supports separate encounter rankings and explicitly weighted combinations.
Movement, dynamic adds and custom-boss inputs are available where the pinned
engine implements them. See [the Forever guide](../clients/forever/README.md)
for input bounds and existing offline measurements.

Existing recorded 72-set checks took 24.49 versus 6.73 seconds for Raid Target
and 24.98 versus 6.82 seconds for Movement, retaining the fixed run's ordered
top five. These are prior fixture measurements with different final iteration
budgets, not new benchmarks or a universal speedup claim.

This review found that the bulk adapter accepted candidates with different
encounter or template inputs even though it sends only one baseline context
to the engine. The comparison now rejects all differences except the first
player's equipment before launching the adapter. Otherwise a change to
movement, rotation, buffs or talents could be silently discarded.

## Next improvements to qualify

1. Extend caching to staged bulk results after qualifying their full keys.
   Fixed-run persistent caching is now implemented with `--cache`, keyed by
   the full request, engine checksum and metric modes. It stores no requests
   or character names, uses user-bound DPAPI on Windows and bounds entries.
   Bulk entries would additionally need adapter identity and shortlist settings.
2. Broader shortlist validation. Use multi-slot pools, close contenders and
   independent seeds across supported encounter mechanics. Measure missed
   contenders and uncertainty as well as elapsed time before changing culling.
3. Encounter robustness reporting. Show when winners change across duration,
   movement or add-wave profiles; keep unsupported mechanics explicit. A
   weighted winner depends on the supplied weights. A staged union can miss
   a balanced set that reaches no individual shortlist; use exhaustive
   evaluation when complete weighted coverage is required.

Do not lower iteration floors or add parallel processes solely to improve one
timing. The engine already schedules parallel work, and existing processor
limit checks did not demonstrate an improvement. Native encounter accuracy and
live game verification remain deferred.

## Change verification

`python -m unittest discover -s tests -p test_simulation_comparison.py` passes
two tests: seven rejected context changes and a gear-only comparison preserving
duplicate mappings and caller inputs. The engine process is mocked; this checks
the adapter payload contract, not simulation accuracy. Both module CLI help
entry points also run successfully.

Jev advisory triage returned `addressed` (0.98 reported confidence, uncalibrated),
resolved model `jev-1.14.0`, 507 input / 49 output tokens, 0.31 seconds,
request SHA-256 `9b0f4479241a81140aa477e257fcdc8729132da93e228b2e514c59ff18e9dcfd`.
This is advisory evidence, separate from tests and independent review.

The first independent review reproduced Python's boolean/integer equality
edge case. The guard now compares canonical JSON, preserving that distinction.
Jev triage recommended the fix (`jev-1.14.0`, 438 input / 45 output tokens,
0.268 seconds, request SHA-256
`e176fadb5a00e2cf8bec711f7e258f43956738bad5c5f3d07b0160ad1151ce53`).

The second independent failure/caller review found no remaining scoped issues.
It exercised the ranker-to-request-to-bulk call chain with a mocked adapter,
rejected type changes, added fields and non-finite context, and checked key
reordering, duplicate mappings and the all-duplicate no-launch path. Both review
rounds used `gpt-6.1-sol` at high effort. No native game testing was performed.
