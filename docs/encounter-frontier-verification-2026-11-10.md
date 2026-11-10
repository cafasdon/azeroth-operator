# Saved encounter frontier: offline evidence

Scope: ADR-098. Synthetic captures, the actual local MCP and pinned local
catalogue, existing private journal fixtures and controlled fake engine outputs.
These checks do not qualify native simulation accuracy or game behaviour.
No external download, actual game action, installation or deployment occurred.

Direct author commands used `PYTHONPATH=.;tests`, with TEMP/TMP=C:/Temp/codex
for the legacy integration fixtures:

- `python -B -m unittest test_encounter_frontier`: final 5 passed, 0.262 seconds.
  Independent exhaustive dominance reference: 160 random pools, 1..8 dimensions,
  20 candidates each, three shuffled orders each (seed 98), including negative
  improvements and tied vectors. Paged membership/count/hash agree with reference.
  Also: 4096 tied sets, overflow to unknown, role improvement direction, empty
  known pool, malformed flags/vectors/bounds and inconsistent duplicates.
- `python -B -m unittest test_run_saved_journal.SavedJournalTests.test_real_mcp_multi_page_shared_budgets_weighted_ranking_and_revalidation`:
  changed integration passed, 62.061 seconds before final helper corrections.
- `python -B -m unittest test_evaluate_owned_gear.OwnedGearEvaluationTests.test_actual_mcp_all_slots_resume_budgets_checkpoint_reuse_and_guards`:
  changed integration passed, 43.184 seconds.
- Both module `--help` entrypoints passed and expose the new options.
- `git diff --check`: passed.

The saved-journal integration returns the two encounter trade-offs in separate
one-set pages, retains one stable frontier fingerprint, supplies saved loadouts
and keeps engine calls at six after checkpoint-only frontier requests. An
incomplete encounter page returns unknown frontier membership and no sets.
The owned evaluator forwards the option into its final simulation report;
existing aggregate results and checkpoint counts remain unchanged.

First independent review round requested callers and failure/math/privacy on
gpt-6-astra at high effort. Callers ran both modified integration methods:
2 passed, 99.466 seconds, and checked invalid options before source access,
default values and empty-journal CLI unknowns. Failure/math compared all 720
permutations of six vectors against an exhaustive reference, with stable digest,
retained ties and conflicting dominated-gear duplicate refusal; two existing
numeric/privacy projection tests also passed.

Review found a deadline fault: final report construction could return known
results after the analysis deadline. Root reproduced it and added checks before
and after sorting/hash construction. Regressions cover elapsed time before
report, an empty eligible pool and expiry inside hashing. No partial set/hash
survives expiry. Root additionally reproduced loss of strict dominance between
integer gains 2**53 and 2**53+1 after float conversion. Original numeric values
now drive comparisons; exact ratios canonicalise fingerprints without that
rounding. Both findings have direct regression tests.

Final independent round: both original lenses clean. Callers reran 5 helper
tests (0.294 seconds) and independently exercised runner.execute with mocked
complete saved pages: adjacent large integer gains retained only the strictly
better set, top=1, one page call and no new runs. A clock expired after the last
candidate returned unknown membership/no sets/hash and no new runs.

Failure/math/privacy reran 5 tests (0.314 seconds). An independent exact Fraction
oracle checked 96 pools over 1..8 dimensions in both orders, page size 7, extreme
finite/subnormal floats, signed zero, adjacent large integers and mixed numeric
types. Exact duplicates at capacity stayed known; conflicting duplicate gains
failed. Exact deadline and expiry inside canonical serialisation suppressed
membership, counts, digest and cursor, including subsequent reports. Integer/
float equal values and signed zero shared canonical hashes.

Root's final saved-journal integration rerun passed in 69.195 seconds after
both helper fixes, including complete pages, saved loadouts, incomplete-page
unknowns and unchanged engine-call counts. Final diff check passed. No repeat
owned-evaluator run was needed: its CLI/plumbing did not change after its passed
integration; the final saved-runner integration exercises the changed helper.

Root judgement: ready for source integration after two review rounds, with the
deadline and precision defects fixed. No external Jev call occurred. Native
accuracy, full ownership/legality and live acceptance remain unproven. This
feature and its evidence do not establish the broad goal's completion.
