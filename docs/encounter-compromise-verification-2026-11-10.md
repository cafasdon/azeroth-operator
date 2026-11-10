# Saved encounter compromise offline evidence

Scope: ADR-103. Source-only local CLI evaluation through the existing saved MCP
plan path. No installation, real character capture, external data request or live
game test occurred.

Author commands with `PYTHONPATH=.;tests`, TEMP/TMP set to `C:/Temp/codex`:

```
.venv/Scripts/python.exe -B -m unittest test_encounter_compromise
Ran 4 tests in 0.648s
OK

.venv/Scripts/python.exe -B -m unittest test_run_saved_journal.SavedJournalTests.test_real_mcp_multi_page_shared_budgets_weighted_ranking_and_revalidation
Ran 1 test in 69.496s
OK

.venv/Scripts/python.exe -B -m unittest test_evaluate_owned_gear.OwnedGearEvaluationTests.test_actual_mcp_all_slots_resume_budgets_checkpoint_reuse_and_guards
Ran 1 test in 43.895s
OK
```

The deterministic oracle covers 120 pools across one to eight dimensions, two
insertion orders each, exact scores, full paging, hashes, dominated ties, adjacent
large integers, extreme finite floats and detached output. Failure cases cover
candidate and time limits, incomplete source results, malformed options before
journal access and expiry during final hashing.

The actual saved-journal integration uses synthetic owner-local captures and
the existing Rust stdio MCP. It pages three saved gear sets with exact regrets
10/30/40, joins the expected saved item, preserves default output and confirms no
extra engine calls. The owned-gear path enables the feature beside frontier and
leader views, checks the selected saved item and zero regret, and retains the
same engine count and source journal. The synthetic engine output does not
qualify native simulator accuracy.

The saved-journal integration test was then rerun after adding the actual
incomplete-page compromise assertions: one test in 64.172 seconds, OK. Missing
encounter results returned `incomplete_saved_results`, no point-estimate digest
and no compromise gear sets, even when other candidates completed.

## Independent review

One round, two independent gpt-6-astra/high agents: callers/failure and
security/data. Neither found a concrete fault. No implementation correction
was necessary. The callers reviewer ran the then-current three unit tests
(0.517 seconds, OK), and independently checked 49 extreme/subnormal float
vectors, exact ranking/paging, the 4,096-candidate boundary, overflow, duplicate
consistency, empty pools, beyond-end pages and deadline output suppression.
Five invalid options failed before other input access in each entry point.

The security reviewer ran the final four unit tests (0.533 seconds, OK). Its
independent probes checked 4,097 candidates, deadline expiry during scoring and
incomplete source results: every derived count, hash and ranking remained absent.
Fifteen malformed options failed before data access in each entry point.
Contradictory gains for one gear hash failed. Source inspection checked numeric
projection, exclusion of private iteration samples, bound saved-loadout joins
and final stable-journal readback. Both agents reviewed ADR-103.

Both reviewers had separate integration processes whose final handles became
unavailable after pending review turns were explicitly interrupted. Neither
claimed a pass for those processes; no redundant processes were started. The
author's completed integration outputs above are the runtime evidence. Reviewer
probes changed no source and accessed no network, private capture or game state.

Jev did not run because its tool declares external paid inference and no separate
spending approval exists. Author judgement accepted the deterministic evidence
and both clean independent reviews. Agent usage was not exposed by the tool.
`git diff --check` passed. Installed availability, native encounter accuracy,
statistical superiority and unconditional owned best-in-slot remain unproven.
