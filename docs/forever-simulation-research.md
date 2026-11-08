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

1. Qualify reuse across broader gear pools. Persistent caching now covers both
   fixed runs and bulk shortlists with `--cache`. Bulk keys include adapter
   identity, the full ordered pool and shortlist settings. Entries store no
   requests or character names, use user-bound DPAPI on Windows and have size
   and count limits. Small-pool reuse is measured; broader search quality remains
   a separate qualification task.
2. Broader shortlist validation. Use multi-slot pools, close contenders and
   independent seeds across supported encounter mechanics. Measure missed
   contenders and uncertainty as well as elapsed time before changing culling.
3. Extend encounter sensitivity qualification. Multi-profile DPS runs now show
   each requested draft's gains, losses, observed range and sign changes, with
   shortlist omissions kept unknown. These are point estimates only. A
   weighted winner depends on the supplied weights. A staged union can miss
   a balanced set that reaches no individual shortlist; use exhaustive
   evaluation when complete weighted coverage is required.

Do not lower iteration floors or add parallel processes solely to improve one
timing. The engine already schedules parallel work, and existing processor
limit checks did not demonstrate an improvement. Native encounter accuracy and
live game verification remain deferred.

The encounter-sensitivity extension passed five focused tests, including the
command-line output path for ordinary and weighted finalist comparisons and
non-DPS exclusion. An independent caller/failure review also checked
non-contiguous finalist IDs, duplicate gear, reused profiles and entirely
omitted reviews through the ranker chain. No remaining findings were reported.
The real local adapter check in
`benchmarks/encounter-sensitivity-2026-11-08.json` uses only public fixtures;
it validates report integration, not the accuracy of a native boss model.
The independent review used gpt-6.1-sol at high effort. Jev 1.14.0 scope advice
returned supported (request `b26904fddf7e6e1638be0364538d02bdfa9097c050b18e0068b65a46b6dc275c`,
509 input / 46 output tokens, 0.285 s), separate from the verification evidence.

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
