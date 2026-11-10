# Encounter gear leaders: offline evidence

Scope: ADR-086; explicit saved character data, local offline tools only.
No installation, deployment or live test.

Plan advisory: Jev 1.14.0 chose implement (confidence 0.98), hash
ccff58499b980e283eb470ad9656439de33d09e1d124e9b2022fd23f309c6dfd,
553 input / 45 output tokens, 0.329 seconds.

Tests use the real local MCP and journal/loadout readers with synthetic snapshots.
The simulator subprocess is replaced by deterministic numeric output; tests do
not qualify a simulator engine. An encounter specialist outside aggregate top-1
must remain in the optional report with its source bank location. Cached reports
must add zero simulation calls. Other probes cover floors, no candidate, missing
results, empty journals, strict Boolean options, all six role metrics and ties.

Direct checks (`PYTHONPATH=.;tests`):

- `python -m unittest test_run_saved_journal.SavedJournalTests.test_encounter_leaders_flag_is_strict_before_journal_access test_run_saved_journal.SavedJournalTests.test_real_module_empty_journal_and_invalid_options test_run_saved_journal.SavedJournalTests.test_real_mcp_multi_page_shared_budgets_weighted_ranking_and_revalidation`: 3 passed, 59.841 seconds.
- After extraction of the leader helper and partial-result integration,
  the real-MCP saved-journal method passed again (49.134 seconds); a subsequent
  run also passed it while two separate test invocation/fixture errors failed.
- `python -m unittest test_run_encounter_windows.RunEncounterWindowTests.test_partial_encounter_leaders_survive_aggregate_top_limit_for_every_role test_evaluate_owned_gear.OwnedGearEvaluationTests.test_actual_mcp_all_slots_resume_budgets_checkpoint_reuse_and_guards`: 2 passed, 35.342 seconds.
- `python -m unittest test_run_saved_journal.SavedJournalTests.test_encounter_leaders_maxima_and_ties_ignore_input_order`: passed, all six metrics and all six candidate permutations.
- `git diff --check`: passed.

One author command used the wrong class name EvaluateOwnedGearTests; the real
class is OwnedGearEvaluationTests. One synthetic role fixture treated time out
of mana as larger-is-better; it now uses the existing lower-is-better direction.
Neither failure changed production metric semantics. The corrected checks passed.

Independent round 1, requested models gpt-6-astra/high:

- Failure lens: no defect. Three focused tests passed in 0.010 seconds and
  the real-MCP saved-journal integration passed in 50.497 seconds. Independent
  oracle seed 860 passed 600 cases across six metrics, one to eight encounters,
  missing intersections, shuffled rows, hash ties, floors and both objectives.
  Aggregate top results stayed unchanged. Invalid duplicate/out-of-range reviews
  and nonfinite values rejected; missing encounters yielded no candidate pool.
- Callers/privacy lens: no defect. Six scoped tests passed in 95.380 seconds:
  empty journal/invalid options, strict flag, numeric evidence/private-field
  exclusion, partial top-1 role coverage, real-MCP saved-journal integration and
  actual-MCP owned workflow. Both real module help paths exposed the flag.
  Simulator call counts did not increase. No live-drift lens applies because
  no installation or deployment occurred.

Final advisory: Jev 1.14.0 chose ready for the source/offline feature,
confidence 0.98, hash
537265c3bd48dcc6099550c2fdd8b890c3a154e7309021c7d7ac5498313ae95c,
581 input / 44 output tokens, 0.286 seconds. Root accepts the bounded result
from scoped tests, source trace and clean independent review. Advisory
probability does not qualify game behaviour or simulator accuracy.
