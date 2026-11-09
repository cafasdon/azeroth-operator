# Role-aware gear evaluation — 9 November 2026

Scope: all six existing simulator ranking metrics through the encounter/page/
journal/unified offline workflows. Baseline `0e30605`. No game operation, live
testing, installation, deployment or external data lookup occurred.

## Source change and deterministic checks

Removed the upper encounter runner's forced `rank_by='dps'` assignment. A pure
role aggregator validates selected values, complete/matching review sets and
weights and projects numeric improvements. Default DPS aggregation and fields
stay on the existing path. Both objectives now prune using the selected metric.
Metric validation runs before other inputs at all four entrypoints.

The helper and its contract tests were implemented in parallel by a
`gpt-6-astra` high-effort scoped writer, then read and tested by the author;
caller wiring and integration tests were written by the author. Actual token
usage was not exposed by the agent tool.

With `C:/Python314/python.exe`, `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`:

```text
python -m unittest test_role_metrics test_run_encounter_windows test_run_saved_journal -v
Ran 19 tests in 37.908s
OK
python -m unittest test_role_gear_workflow -v
Ran 2 tests in 14.386s
OK
```

The 12 pure tests cover all five native-unit role scores, zero baselines,
negative threat, unequal weights, numeric projection/input immutability, invalid
metrics/weights/IDs/values, missing or inconsistent review sets and overflow.
Existing encounter and journal tests cover DPS defaults, duplicate profiles,
shared budgets, partial work, normalisation, resume and saved provenance.

The actual Rust MCP/CLI workflow test uses synthetic owner snapshots and engine
output, including zero DPS in every result. All five non-DPS metrics still rank.
Two encounters, weights 9:1 and three gear sets exercise partial first-run work,
positive and negative direction, weighted/worst winners, native units, saved
loadouts and false proof flags. First tank-family completion uses six requests;
TPS/death reuse those summaries. Healing uses six requests; OOM reuses them.
All checkpoint-only reranks use zero new simulations; total synthetic engine
calls remain twelve. Source snapshot bytes and private-output checks pass.

## Actual offline engine checkpoint

An author-only bounded probe and the following retained opt-in test use the
qualified positional engine and SHA-pinned cached public source archive:

```powershell
$env:AZEROTH_POSITIONAL_ARTIFACTS='C:/Temp/codex/forever-positional-v4/artifacts'
$env:AZEROTH_QUALIFY_SOURCE_ARCHIVE='C:/Temp/codex/forever-sim-9dc32ac/source.zip'
C:/Python314/python.exe -m unittest test_real_role_workflow -v
```

One test passed in 6.191s. Source archive SHA:
`e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712`.
The public protection-Warrior APL SHA is
`befce19883766e21937c3c904a996dae2f7c4f56efe1b1f97ea41fb615f3399d`;
gear-source SHA is `21ba1f3f145900d4e32500fa9bfd7a16252bea120c50751933e9d3e412788402`.
The synthetic capture uses the preset's base IDs with zero enchants/suffixes,
not an exact fully modified preset. Engine SHA remains
`833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983`.

Level-60 Orc Warrior, public protection talents/APL, explicit tank assignment,
synthetic incoming attacks and healing model, 1,000 iterations/seed 20261109,
45/90-second encounters. A single baseline window intentionally leaves full
collection incomplete. Two actual cold runs return DTPS
199.30383114805176/195.21347191713411. TPS reranking uses zero new runs and
returns 1056.3122208352856/1034.3602637330303; death fractions are zero, also
reused without new runs. Saved-scope completion is distinguished from incomplete
overall collection and ownership/BiS proof stays false. These are limited
plumbing observations, not a combat-accuracy benchmark.

## Advisory receipt

Independent `gpt-6-astra` high-effort Callers and Failure/security reviews found
no concrete defect. Callers ran 21 scoped tests in 54.406s and all four real
module help invocations. Its independent two-page top-1 probes for every non-DPS
metric selected a high weighted/weak worst-profile candidate under weighted
gain and a balanced candidate under worst-encounter gain. Failure ran the pure
and actual-MCP synthetic tests, then a one-window/one-page stress variant
(2 tests, 97.993s), all passing. Missing/nonfinite/negative-healing data, wrong
result family, falsely complete review sets, changed gear and exhausted shared
budgets rejected or withheld ranking. DPS/tank/healing/paired-tank checkpoint
identities stayed distinct; inappropriate summary families were rejected.

After adding the opt-in actual-engine test, Callers read it and the documents
without executing raids. It independently matched the source archive, APL,
gear and engine hashes, public protection talents and upstream healer stats-
only comment. It confirmed the base-ID-only capture and limited baseline scope;
exact DTPS/TPS values and test timing remain attributed to the author. No
reviewer built, deployed, used HTTP, operated the game or ran real raid engines.

Plan: `jev-1.14.0`, request
`f73fd69bc0556ddf8dc090acce5c6847d342a806c97cfab7970f2e17e1671bfd`,
640 input/47 output tokens, 0.310s; proceed .79, revise .18,
insufficient evidence .03, confidence .68. Advisory only, uncalibrated.

Final scoped landing: `jev-1.14.0`, request
`8d9179ddaa9e0a1d877efb485a2f368727e4ab89c0a53177ab2b8be35ae9e2e3`,
776/49 tokens, 0.301s; land scoped .97, fix first .01,
insufficient evidence .02, confidence .96. Advisory only.

## Limits

Scores are point estimates in stated units. Role objectives require appropriate
explicit encounter/rotation settings and do not establish current equip legality,
complete ownership, statistical optimality or full owned BiS. The pinned source's
`sim/priest/healer/healer_test.go` explicitly calls its default suite stats-only
with no healing rotation. No default healer rotation or healing model accuracy
is qualified here. Live and broader class/encounter qualification remain open.
