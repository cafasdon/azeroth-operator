# Completed-candidate ranking — 2026-11-09

Baseline 9d21960. Previous goal turn made progress retaining final encounter
provenance. This unit makes fully tested candidates usable within an unfinished
page, without treating a partial search as full owned-gear optimisation. Broad
goal stays active. No game, account data, addon installation or deployment.

## Author checks

```powershell
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
python -m unittest test_completed_candidates test_run_encounter_windows test_run_saved_journal
```

Ten scoped tests passed in 53.829s. Unit cases use complementary missing
candidates to distinguish intersection from union and reweighted scores;
weighted .25/.75 yields -2.5 percent for a candidate with +20/-10 percent gains.
Other tests cover absent encounters/baselines, nonpositive DPS, all five native
role metrics with zero baselines, weighted versus worst-encounter reversal,
digest ties, top-N counts and private metadata/sample exclusion.

Actual Rust stdio MCP and production window/encounter/journal paths run over
synthetic owner-local stores. Controlled engine subprocess doubles supply known
numeric metrics, including interrupted baseline-only intersections. No engine
performance or model/encounter accuracy is claimed. Additional real cached
request intersection and unified-evaluator assertions are recorded below.

## Advisory plan

Jev resolved jev-1.14.0, request
`ca071c493f909459a631590bdeca840937b545fbc67ad4370ac0f66fb05963bf`:
proceed .90 / revise .09 / insufficient .01, confidence .85; 583 input /47 output
tokens, .286s. Advisory only; independent callers and failure review required.

## Additional real-path assertions

```powershell
python -m unittest test_run_saved_journal.SavedJournalTests.test_real_mcp_multi_page_shared_budgets_weighted_ranking_and_revalidation
python -m unittest test_evaluate_owned_gear.OwnedGearEvaluationTests.test_actual_mcp_all_slots_resume_budgets_checkpoint_reuse_and_guards
```

The first passed in 33.402s, the second in 14.392s. The first uses actual
prepared/cached requests and hides only one alternative's second encounter
checkpoint through the real journal interface, with checkpoint-only execution.
The fully compared alternative and baseline remain ranked, the missing candidate
is excluded, saved item4549 and its 130-DPS second encounter remain inspectable,
completed-window count stays0, stop reason is incomplete_page and new_runs0.
Original journals stay byte-identical. The unified evaluator retains prior
collection/resolution/simulation boundaries and resumes without extra runs.

An initial combined invocation used a wrong unittest class selector for the
second test; the correct name is printed above. The first new assertion also
initially compared exactly25 against the existing floating result
24.999999999999996; only the test changed to assertAlmostEqual. No production
math or guard changed in response. Both corrected focused commands passed.

## Independent adversarial review

Two independent read-only reviewers assigned gpt-6-astra high (resolved runtime
IDs/usage not exposed), Callers and Failure/privacy, found no concrete fault.
Callers ran four cheap scoped helper/deadline/finalisation tests, .012s, plus
480 seeded independent oracle comparisons across six metrics, two objectives,
2..8 encounters, missing candidates and top/hash ties. Its production journal
orchestration probe retains earlier complete-page leaders, adds complete
intersection candidates, excludes a missing winner, and never requests a third
page or advances the incomplete cursor. Completed count stays1/2.

Failure/privacy ran 240 independent seeded oracle cases and eight boundary
cases for missing/unvisited baselines and invalid/duplicate reviews. Its
production journal orchestration probe excludes an unmatched 9999-DPS candidate,
retains original .25/.75 weights and saved-review identity, stops after one
incomplete page, keeps completed_window_count0/completeness false, and strips
private samples. That probe's first exact25 expectation was likewise corrected
to math.isclose without source changes. Production orchestration probes mock
journal/page I/O; they are not real-engine/native qualification.

Both bounded documentation readbacks were clean, pending only author receipts
now recorded above. No builds/source writes/Git changes/network/game/deployment
by reviewers. No live-drift pass because installation/testing remain deferred.
Final git diff whitespace check passed. No pipeline or unrelated suite replay.

Final advisory Jev resolved jev-1.14.0, request
`e6473264e4282c1064918cb220b68d5c59306e883f088c6cee3d42064795cba6`:
integrate .86 / revise .11 / insufficient .03, confidence .79; 587 input /52
output tokens, .362s. Author accepts the bounded source feature on scoped real
caller and independent-oracle evidence; broader completion remains unproven.
