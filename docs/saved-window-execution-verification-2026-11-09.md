# Saved-window execution verification — 9 November 2026

Scope: offline source, real local MCP/SQLite/Windows checkpoint protection, and
synthetic engine responses through the actual subprocess boundary. No real
simulation engine, native game, installation, deployment, external sync or CI.
Broad system coverage and full owned BiS remain incomplete.

Command shape:

```powershell
python -m tools.run_forever_saved_windows --mcp ABSOLUTE_MCP --journal GEAR_JOURNAL --window-offset N --template TEMPLATE_JSON --encounter-lookup ENCOUNTER_JSON --engine ABSOLUTE_ENGINE --engine-sha256 HASH --simulation-journal PRIVATE_ROOT
```

Window offsets repeat for a bounded selected set. Options include `--paired`,
six `--rank-by` metrics, `--max-new-runs`, `--iteration-budget`,
`--time-budget-seconds` and `--simulation-journal-mib`. The tool consumes no
external prepared-request file and revalidates through local MCP on every retry.
It does not act in the game, install software or infer the active character.

Actual MCP fixture: level-60 Hunter, baseline and one ring alternative, two
retained coordinate windows. With one allowed new run per invocation, the first
pass checkpoints baseline and reports missing candidate; the second reuses
baseline and completes candidate; the third hits both with no engine launch.
The synthetic engine boundary is called exactly twice across all three passes.
Ranking shows the fixture candidate's +10 DPS and zero paired gap standard error.
Work-set identity stays stable, gear journal bytes stay unchanged and no private
paths/names appear in output. Unsupported level 20 rejects despite existing
matching numerical checkpoints, and leaves those checkpoint bytes unchanged.

An extracted pure comparison helper preserves existing numeric formulas without
mutating paired sample arrays. New output reports missing separately from not
yet inspected work; partial source coverage stays separate from completed selected
requests. BiS/feasible/game flags remain false. No actual performance, real
encounter accuracy or full-model qualification is claimed from the stub results.

Initial author commands with `PYTHONPATH=.;tests`:

```text
python -m unittest test_run_saved_windows test_simulation_journal test_simulation_cache -q
18 tests passed in 16.739s
```

## Independent review and correction

Round one used `gpt-6-astra`, high effort, Callers and Failure/security; per-review
token usage was not exposed. It reproduced two P2 defects:

1. A completed checkpoint read could cross the work deadline, then comparison
   used that expired deadline and raised before reporting progress. The author
   reproduced this with a deterministic clock, a completed baseline and an
   uninspected candidate: `Comparison deadline exceeded` before the fix.
2. A synthetic engine changed its temporary executable bytes while returning
   DPS 999. The final runner check rejected, but `adapter.run` had already stored
   that result. Restoring expected bytes allowed a checkpoint-only retry to use
   the rejected result. The author reproduced the missing rejection at the
   adapter boundary (`ValueError not raised`).

The surgical fixes give completed-row formatting and final checks a shared
work-deadline-plus-five-second allowance, and verify the engine again inside
the fixed-run adapter before checkpoint commit/return. Earlier individually
verified results are not deleted. Both failing regressions passed after fixing.

Post-fix author commands:

```text
python -m unittest test_run_saved_windows test_simulation_cache test_simulation_journal -q
20 tests passed in 16.763s
python -m unittest test_simulation_comparison -q
2 tests passed in 0.012s
```

Jev finding triage: `jev-1.14.0`, request
`23dda79906e353c9132e19b3b477d84616c8c384a1ee791f037dc9744250925b`,
712 input / 51 output tokens, 0.316 s; fix-both 0.99. Advisory only.

Round two reran both lenses because the adapter persistence fix reaches existing
fixed-run/cache callers. Both were clean. Callers command:

```text
python -B -m unittest test_run_saved_windows test_simulation_cache test_simulation_comparison test_simulation_journal -q
22 tests passed in 17.127s
```

Its independent original one-second / 1.001-second cached-baseline probe now
returned the completed row with no engine calls. All six metric routes, paired
comparison, baseline mapping and false claim flags had also passed round-one
synthetic cached-result probes.

Failure/security command:

```text
python -m unittest discover -s tests -p test_run_saved_windows.py -v
4 tests passed in 7.492s
```

Its independent original engine-drift reproduction now returned:

```text
first_attempt_rejected: Engine SHA-256 mismatch
subprocess_calls=2 new_runs=1 checkpoint_hits=0 completed=true dps=100
verified_checkpoint_retry: accepted without engine execution
prior_verified_checkpoint_preserved=true rejected_new_checkpoint_absent=true sqlite_byte_identical=true
```

Restoring expected bytes required fresh synthetic execution, then allowed reuse
of the verified result. A separate already verified checkpoint stayed readable
and byte-identical. The deadline test reports one completed row, one uninspected
request and `time_budget`, without starting further work. All SQLite handles and
review-owned temporary fixtures were cleaned. No real engine, game, network or
deployment actions occurred; protected old directories stayed untouched.

Post-fix scoped advisory: `jev-1.14.0`, request
`c2363326379928f7db569405b2ca0c510b09c1d84fc4d1ed9c374550ec2bf928`,
954 input / 52 output tokens, 0.308 s; land-scoped 0.85, fix-first 0.11,
insufficient evidence 0.04, confidence 0.77. Advisory only, not proof or permission.
