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
   A new four-slot fixture below adds six repeated-seed native audits; close-tie
   uncertainty and more classes remain unqualified.
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

## Four-slot repeated-seed qualification

The new `tools.qualify_forever_shortlist` module runs fresh staged and fixed
comparisons for an explicit request pool, without persistent caching. It reports
canonical gear review IDs, missed fixed point-estimate contenders, per-seed
timings, engine hashes, input digest and stage telemetry. It does not infer
confidence, legality or global optimality. CLI details are in the Forever guide.

[Recorded public-fixture results](benchmarks/forever-four-slot-shortlist-2026-11-08.json)
cover 81 explicit gear sets, four varied slots and three seeds for each of
`Default/Raid Target` and `Default/Movement`. The Hunter template hash and exact
item choices are in the record. This is a synthetic simulator pool, not owned
gear or an equip-legality qualification. It was built as follows:

- Use the existing public Hunter template, its APL, talents and buffs.
- Enumerate the Cartesian product in slot order 4, 6, 7, 9 (zero-based simulator
  chest, hands, waist, feet). Each slot has the three recorded base item IDs.
  Replace each varied slot's item object with just its ID, dropping modifiers.
  Replace the first whole combination with the exact public baseline, retaining
  all its original modifiers. The other 80 inputs remain synthetic alternatives.
- Use each pinned catalogue preset's `targets[].target` records, 45 seconds,
  three seconds of variation, execute proportions 0.2/0.35 and 1,000 fixed
  iterations. Pass the resolved equipment and profile through `request_from`.
- Invoke the qualification CLI for each profile with seeds 100000000, 200000000
  and 300000000, `--top 5` and the recorded executable hashes. Run profiles
  sequentially. These seed choices are not proof of independent samples.

| Profile | Seed | Fixed seconds | Staged seconds | Fixed top five missed |
| --- | ---: | ---: | ---: | ---: |
| Raid Target | 100000000 | 18.05 | 5.45 | 0 |
| Raid Target | 200000000 | 18.63 | 5.35 | 0 |
| Raid Target | 300000000 | 18.70 | 6.01 | 0 |
| Movement | 100000000 | 19.38 | 6.60 | 0 |
| Movement | 200000000 | 18.58 | 6.51 | 0 |
| Movement | 300000000 | 18.07 | 6.07 | 0 |

All six retained the same ordered top five. Each staged pass reduced 80
alternatives to five and refined those five at higher precision. The fixed
pass ran all 81 sets at 1,000 iterations; staged finalists reached roughly
16,000–17,300 iterations. Different final precision prevents a claim that these
are identical workloads. No culling rules or iteration floors were changed.
This adds evidence for the existing acceleration rather than a new first-run
speedup. Native boss-model accuracy and live game testing remain deferred.

Five qualification tests cover canonical aliases and missed-contender reports,
seed replacement without caller mutation, duplicate-only pools, aggregate
budget rejection, post-normalisation deduplication and controlled malformed
input/native-timeout CLI failures. The module's `--help` entry point runs.
Two existing bulk-context tests also pass.

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

The required independent Failure/Callers review used gpt-6.1-sol at high effort
and ran two rounds. Round one reproduced a budget false rejection when input
seeds differed but were replaced before execution, plus an uncaught malformed-
party CLI error. Budget accounting now normalises a cloned seed before counting
unique requests; CLI failures include malformed structure and native subprocess
errors. Round two re-ran both original probes: 101 input seed aliases collapsed
to one actual fixed run per requested seed, caller seeds stayed unchanged, and
malformed parties exited with a controlled error. Five scoped tests and module
help passed. No remaining concrete findings were reported.

Jev 1.14.0 advisory triage recommended correcting the findings (request
`27147d5f7c09ce5b716c13df69e8ac229bda13af61d3489289687ab2cd16ae43`,
467 input / 45 output tokens, 0.454 seconds). Its separate limited-claim review
returned supported (request
`136d40b7edab12d4d64f3324edf8af434d2b60d22d9df07ba20bcf5619f6d440`,
591 input / 46 output tokens, 0.308 seconds). These are advisory judgements,
separate from test and native benchmark evidence.
