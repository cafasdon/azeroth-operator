# Worst-encounter gear ranking — 9 November 2026

Scope: an explicit aggregation objective for the saved-journal and unified
owned-gear evaluators. No game operation, installation, deployment or live test.
Baseline source: `8cbf599`.

`weighted_gain` remains default. `worst_encounter_gain` maximises the minimum
percentage DPS gain against the same captured baseline across all supplied
encounters. Weighted gain and then gear hash break minimum-gain ties. The chosen
objective drives top retention across pages; this is not a resort of a previously
weighted-pruned shortlist. Reports identify objective, score and min/max gain.
Every positive-weight encounter participates in the minimum; weights only break
ties in that mode. Point-estimate and partial-scope limitations remain explicit.

## Author verification

Python: `C:/Python314/python.exe`, `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`.

```powershell
$env:AZEROTH_POSITIONAL_ARTIFACTS='C:/Temp/codex/forever-positional-v4/artifacts'
$env:AZEROTH_QUALIFY_TEMPLATE='C:/Temp/codex/forever-sim-9dc32ac/public-hunter-template.json'
C:/Python314/python.exe -m unittest test_run_saved_journal test_positional_gear_workflow.PositionalExecutionTests.test_actual_all_slot_positional_workflow_resolves_hole_and_reuses_results -v
```

Initial run: 4 tests in 47.194s, three passed; the new invalid-choice CLI test
incorrectly expected exit 1 from argparse, which correctly returned 2. The test
expectation was corrected, without changing runtime code. Its focused rerun:

```text
python -m unittest test_run_saved_journal.SavedJournalTests.test_real_module_empty_journal_and_invalid_options -v
Ran 1 test in 8.740s
OK
```

The real Rust stdio MCP/synthetic engine journal test uses three saved pages and
two encounters. Baseline DPS is 100 in each. Ring 4549 gives gains 10%/30%; ring
4550 gives 20%/-10%. Under weights 9:1 and top limit 1, default weighted ranking
chooses 4550 (17% average), while worst-encounter ranking chooses 4549 (10%
minimum, 12% average). This verifies that the two objectives can select different
gear, including with cross-page bounded retention. Both rerankings have zero new
engine calls and retain exact saved item/source provenance. The total synthetic
engine call count remains six. Default cold/warm equality, smaller-page ranking,
partial traversal, changed-input rejection and unchanged journal bytes remain
covered. Direct malformed objective values reject before journal access.

The actual offline positional-engine test uses the previously qualified engine
and public Hunter template, 1,000 iterations/seed 20261109 and two encounters.
It completes four coordinates/zero unresolved/three gear sets with six cold
simulations. Default warm and worst-encounter checkpoint-only ranking each use
zero new simulations, with complete reported scope and the correct descending
minimum-gain scores. This real-engine execution is author-only; reviewers use
synthetic engine/MCP probes. Engine/template identities remain those recorded in
[the positional integration evidence](positional-integration-verification-2026-11-09.md).

## Advisory receipt

Independent `gpt-6-astra` high-effort Callers and Failure/security passes found
no runtime defect. Both independently reproduced the initial argparse test
expectation issue and confirmed the corrected assertion/readback. Callers ran
the current three journal tests with additional real-MCP checkpoint probes:
43.537s, OK. It verified robust top 1 with one-window pages (4549, 10%, complete),
one-/two-page partial scope, explicit weighted selection (4550) and zero new
simulations in every extra checkpoint probe. Legacy argument namespaces retain
the default, and malformed owned-workflow objectives reject before collection.

Failure/security's corrected CLI test passed in 7.933s. Independent probes
covered 16 malformed objectives across both entry points, invalid weights,
equal-minimum weighted ties, hash ties independent of page arrival, partial
second pages, changed baselines, nonfinite gains and a `1e-200` encounter weight
that still controls the minimum. Both traced all supplied encounters completing
before page ranking, unpruned downstream inputs, unchanged raw cache/binding
semantics and historical loadout protections. No reviewer ran raids, builds,
game actions, deployment, external requests or source edits.

Plan: `jev-1.14.0`, request
`00f6d4660dc2b30d5cf49c07539bdde12ea5e22bc986f77c977ed6cd31f4e9b6`,
567 input/47 output tokens, 0.343s; proceed .72, revise .23, insufficient
evidence .05, confidence .58. Uncalibrated advisory guidance only.

Final scoped landing: `jev-1.14.0`, request
`cd3e5a7b6a98dd6241f51abd2961ecacc333c37e2ce5d8fc4ba492de21f4e786`,
697/49 tokens, 0.283s; land scoped .92, fix first .03,
insufficient evidence .05, confidence .88. Advisory only.

## Limits

The minimum is over supplied encounter point estimates, not over all possible
fights or random outcomes. Full owned pool, native eligibility, combat-model
accuracy, statistical optimality, active-character detection and full owned BiS
remain unverified. Objective changes reuse raw numeric checkpoints, but still
perform existing current MCP preparation and hash/capability checks. Live
testing stays deferred.
