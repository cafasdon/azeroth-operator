# Encounter improvement floor — offline evidence, 2026-11-09

Scope: ADR-068. Previous turn was progress: committed/pushed 5e52e6b. This turn
inspected current clean worktree and identified missing per-encounter constraints
in the real owned-gear evaluator. No live game, addon, deployment or external sync.

Author checks (PYTHONPATH=.;tests, TEMP/TMP=C:/Temp):

```
python -m unittest test_run_saved_journal
Ran 5 tests in 59.866s — OK
python -m unittest test_role_gear_workflow
Ran 2 tests in 32.407s — OK
python -m unittest test_completed_candidates test_run_saved_journal.SavedJournalTests.test_real_mcp_multi_page_shared_budgets_weighted_ranking_and_revalidation
Ran 3 tests in 40.775s — OK
python -m unittest test_completed_candidates test_role_gear_workflow.RoleGearWorkflowTests.test_bad_floor_rejects_before_other_owned_inputs test_run_saved_journal.SavedJournalTests.test_encounter_floor_validation_precedes_journal_access
Ran 4 tests in .048s — OK
```

Synthetic saved captures use the actual local MCP executable and pinned local
catalogue; the engine boundary is stubbed with deterministic metrics. These tests
establish orchestration and ranking, not native encounter accuracy. The added
zero/impossible floor runs use checkpoints only and add no engine calls. Tests
cover all six role metrics, baseline eligibility, empty eligible results despite
complete simulation, inclusive thresholds, top=1, invalid/private/nonfinite/huge
inputs and real module CLI failure with no engine/checkpoint creation.

Two independent gpt-6-astra/high reviewers reproduced a first-round fault:
partial completed-candidate ranking truncated before journal floor filtering.
With weights 9:1, A +100/-1 could hide eligible B +10/+10 at top=1. Fixed by
applying the floor inside partial ranking before truncation, carrying instance
exclusions to journal reporting and retaining completed/omitted counts unchanged.
Scoped regression also checks exact +10 and permitted -10 boundaries.

Jev plan advisory: jev-1.14.0, request
6c3872ceddffd7ebf896964769bae75f6cec472e4745bb464ca434f49b135871;
proceed .81, revise .15, insufficient .04, confidence .72; 501 input / 47 output
tokens, .269s. Finding triage request
2925631e877de759479273220a2c8d9d3ce8d5bdceee812c5ca13608237e58e2:
fix .97, insufficient .03, none .00, confidence .96; 434 input / 45 output,
.312s. Advisory only; reproduced failure and direct tests drive the decision.

No threshold qualifies complete ownership, feasible-set coverage, statistical
optimality or native eligibility. Broad MCP coverage goal remains active.

Second independent round clean: Callers ran nine scoped tests in 83.203s;
Failure ran nine in 82.329s plus latest edge assertions. Both exercised actual
journal execution with synthetic page I/O and real partial ranking: floors
None/-10/0/10/11 at top1/2 preserve eligible sets and exact exclusion counts
0/0/1/2/3. Six role/missing-encounter/immutability probes and nine malformed
rejections passed. Both lenses reviewed the final docs. No further faults.
Real module --help confirms both entry points expose the option.

Final Jev advisory jev-1.14.0 request
70fcb907cc3e4b406302b1275cccace6efd5fd22e066604eef290ff9b6b71521:
integrate .73, revise .25, insufficient .02, confidence .60; 511 input / 50
output tokens, .288s. Author accepts bounded source integration based on direct
tests, the reproduced-and-fixed partial ordering fault and both clean reviews.
