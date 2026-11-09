# Ranked saved loadouts — 9 November 2026

Scope: expose actual retained equipment and physical-source selections in
offline saved-journal and unified owned-gear rankings. No game operation,
installation, deployment, external data access or live testing.

The join reads at most 20 referenced checked rows through the existing strict
read-only journal connector, binds snapshot/scope/collection identity and gear
hash, validates strict selections and retained equipment, and closes SQLite on
success or failure. The existing final journal-content comparison still runs
after attachment. Only historical plan data is projected, with false current
location, native eligibility and game execution flags.

## Author evidence

With `C:/Python314/python.exe`, `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`:

```text
python -m unittest test_run_saved_journal test_positional_gear_workflow.PositionalExecutionTests.test_actual_all_slot_positional_workflow_resolves_hole_and_reuses_results -v
Ran 3 tests in 40.039s
OK
```

After the clearance fix, the same three-test command passed again in 54.211s.

The journal tests use the actual local Rust stdio MCP with synthetic owner
captures and a synthetic engine boundary. They exercise the actual module CLI,
multiple saved pages/encounters, cold and checkpoint-only ranking, incomplete
scope and unchanged source bytes. The winning ring 4549 is projected to finger1
with the exact saved bag 0/slot 1 source and matching spec hash. Baseline has no
source selections. Bad/missing/noncanonical window references, review indices,
winning hash and snapshot/scope/binding mismatches reject without changing the
journal.

Adversarial review reproduced invented offhand clears (including floating-point
17.0) passing prior preparation and the new projection. The author added an
actual CLI regression and observed `AssertionError: 0 != 1` before the fix
(14.479s). Preparation now uses a shared strict clear-list/selection/spec guard
and compares saved clears with the actual MCP review. The three preparer tests
passed in 20.149s, including a legitimate two-handed mainhand/offhand-clear plan,
rejection when its clear annotation is omitted, and invented/float/null clears.
An initial positive-fixture typo used `mainhand`; it was corrected to the real
`main_hand` simulator slot before the successful run.

The positional test uses the previously qualified actual offline engine and
public template: four reported coordinates, zero unresolved proposals, three
gear sets across two encounters, six cold simulations and zero warm simulations.
Every result's saved positional spec hashes to its winning gear hash and retains
the explicit owner snapshot. This actual engine run is author-only; reviewers
use numeric/synthetic probes without raid simulations.

## Advisory receipt

Two independent `gpt-6-astra` high-effort Callers and Failure/security lenses
ran two rounds. Both confirmed the clear-metadata fault. Round 2 was clean:
Callers ran five preparation/journal tests in 63.585s and independently rejected
ten malformed/contradictory helper cases while accepting valid legacy and
positional clears. Failure ran three preparation tests in 28.102s, ten direct
edge cases, and an extra actual CLI missing-field compatibility probe in
21.439s. Missing no-clear metadata stayed compatible; missing genuine-clear
metadata was rejected. Both traced fresh preparation before checkpoint reuse.
No reviewer ran raids, native operations, builds or external requests.

Initial plan: `jev-1.14.0`, request
`dd07f697c825e48e7665f83bfa3f55de8c873bceaf82a867625398b4b9153328`,
516 input/49 output tokens, 0.330s. Proceed .65, revise .29, insufficient
evidence .06, confidence .48. Advisory uncertainty is retained; the value is
not independent verification or permission.

Clear-metadata finding triage: `jev-1.14.0`, request
`155628bf151f179b38a746927649820ac1a8a753885ee0e8e6a2374ed6f3cb3c`,
517/46 tokens, 0.282s; fix .98, insufficient evidence .02,
no material issue .00, confidence .97.

Final scoped landing: `jev-1.14.0`, request
`f603725e1742983f1ca71774ee5ffef520094dd618d5a2b7785072c6f8a921d9`,
688/49 tokens, 0.323s; land scoped .92, fix first .06,
insufficient evidence .02, confidence .88. Advisory only.

## Remaining boundaries

Physical locations are those of the saved snapshot and can change. The output
does not prove current accessibility, native equip eligibility, complete owned
pool, combat-model accuracy or statistical optimality. Full owned BiS and active
character selection remain unfinished; live testing remains deferred.
