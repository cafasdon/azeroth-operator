# Saved-window encounter comparison — 9 November 2026

Scope: offline source, real local MCP and private SQLite checkpoints with
synthetic engine outputs only. No real engine, native game, installation,
deployment, network data sync or CI. Broad coverage and full owned BiS remain open.

The new `tools.run_forever_encounter_windows` takes the explicit MCP, gear
journal/window offsets, template, engine/hash and private checkpoint root used
by the single-profile executor. Repeat `--encounter-lookup` up to eight times and
optionally supply one positive `--encounter-weight` per profile. Run, iteration,
time and storage budget options retain the single-executor bounds.

Budgets are shared rather than reset per profile. Exhausted new-work budgets
switch later visits to checkpoint-only mode. The core executor also exposes
`--checkpoint-only`, revalidating saved/model inputs and engine identity without
starting new engine work. Its optional inherited deadline preserves one common
work/finalisation boundary. New metadata identifies the non-encounter simulation
profile and current file fingerprints for comparability.

Real MCP fixture: baseline/ring candidate, a 45-second profile (+10% synthetic
candidate gain), a 90-second profile (-10%), and repeated first profile. Weights
1:3:1 normalise to 0.2:0.6:0.2; candidate weighted gain is -2%, so baseline leads.
One allowed new run across all profiles advances four unique requests over four
invocations. Incomplete visits have no weighted result. A fifth invocation makes
zero new runs and six checkpoint hits, including the repeated profile. Separate
checks cap global new iterations at three and verify checkpoint-only starts no
engine even when results are missing. Journal bytes/private output boundaries
remain intact. Sensitivity records +10%, -10%, +10% and the sign reversal.

These are orchestration/arithmetic fixtures, not measured game performance or
proof of encounter accuracy. Complete weighted output covers only the selected
resolved requests. Source ownership/model gaps and false BiS/feasible/game flags
remain explicit. No externally supplied prepared jobs are executed.

Author commands with `PYTHONPATH=.;tests`:

```text
python -m unittest test_run_encounter_windows test_run_saved_windows -q
6 tests passed in 18.725s
python -m unittest test_run_encounter_windows test_encounter_sensitivity -q
7 tests passed in 11.712s (before the new deadline regression)
python -m unittest test_simulation_comparison test_simulation_journal -q
8 tests passed in 5.299s
```

Both module help paths and whitespace checks passed. Weight tests cover huge
finite values, zero/negative/NaN/infinite/boolean values, count mismatch and
normalisation underflow. Defaults use equal positive weights.

## Review and correction

Independent round one used `gpt-6-astra`, high effort, Callers and
Failure/security. Per-review token usage was not exposed. Callers was clean:
encounter discovery tests two passed, 13.291 seconds; saved-window tests four
passed, 7.569 seconds. Independent in-memory saved-boundary probes rejected
engine/profile/template/coverage/gear/count changes, duplicate review IDs and
zero/nine profiles. Partial results withheld both aggregates; nonpositive baseline
withheld weighting; budget exhaustion preserved the common deadline while moving
later profiles to checkpoint-only. No real engine ran.

Failure/security found one P2: final sensitivity/weight/encoding work did not
check the common deadline after child execution returned. Its deterministic
probe advanced a real weighted calculation to 6.25 seconds with a one-second
work budget plus five-second finalisation allowance, and still returned complete.
The author regression reproduced the missing rejection (`ValueError not raised`).
The surgical fix checks the common finalisation deadline before aggregate
processing and after bounded report encoding. Existing work/checkpoint semantics
are unchanged. This is a controlled-clock defect test, not a measured six-second
weighted calculation.

Jev finding triage: `jev-1.14.0`, request
`ac92b1f3b1b1aca2440878bf008866469f2d6e7ccb52b109a760ddf3d6994438`,
593 input / 48 output tokens, 0.307 s; fix-first 1.0. Advisory only.

Post-fix author command `python -m unittest test_run_encounter_windows test_encounter_sensitivity -q`
passed eight tests, 10.197 seconds. Round two used Failure/security only because
the surgical fix touched no new caller or file. Its direct discovery command:

```text
python -m unittest discover -s tests -p test_run_encounter_windows.py -v
3 tests passed in 10.675s
```

The independent original probe now rejects. Inclusive boundary probes returned:

```text
5.999999 complete_for_selected_resolved_requests
6.0 complete_for_selected_resolved_requests
6.000001 rejected
6.25 rejected
expired saved result: rejected before final calculations
```

Round two was clean. Both reviewers made no source/build/Git changes, real
engine runs, network calls, game actions or deployment. Their synthetic fixtures
and SQLite handles were cleaned; protected old directories stayed untouched.

Post-fix scoped advisory: `jev-1.14.0`, request
`621e0e57117a0c4e1b3be2bc047bc1ec8f2f23e9bc05f7dcb79f96f173d08f66`,
997 input / 52 output tokens, 0.310 s; land-scoped 0.72, fix-first 0.18,
insufficient evidence 0.10, confidence 0.58. This uncertain advisory is recorded
without rerolling; independent scoped tests and clean review support the bounded
source increment. It is not proof, permission or broad model qualification.
