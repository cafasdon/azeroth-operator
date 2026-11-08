# Gear-plan resolution accounting — 8 November 2026

Scope: offline source, validated saved metadata and synthetic fixtures. No
native game, deployment, simulator execution or CI. This extends the path from
collection coverage towards complete search accounting; simulation ranking,
qualified ownership/models and full BiS remain unfinished.

The merged-window and journal-status outputs add `resolution_accounting`, with
basis `validated_local_plan_report_metadata`. For each nonoverlapping window:

| Field | Derivation |
| --- | --- |
| `baseline_coordinates` | One if coordinate zero is in the window; otherwise zero. |
| `pre_review_excluded_coordinates` | Considered minus baseline coordinates minus proposed alternatives. |
| `proposed_alternative_coordinates` | Reported proposed alternatives. |
| `resolved_alternative_draft_instances` | Resolved rows minus the reused baseline row. |
| `equivalent_alternative_coordinates` | Reported equivalent proposals. |
| `unresolved_proposed_coordinates` | Proposed minus resolved alternatives minus equivalent proposals. |
| `baseline_review_instances` | One per report, including baseline reuse outside coordinate zero. |

Baseline coordinates, pre-review exclusions, resolved alternative instances,
equivalent alternatives and unresolved proposals partition the covered
coordinates. Proposed is a subtotal, not an additional partition member.
Resolved instances can share a gear hash across windows; they are not a global
distinct count or simulated results. Pre-review exclusions and unresolved
proposals do not establish a particular legality reason, current in-game
eligibility or complete feasible-set coverage. Raw selection/error text and
private payload fields are not exposed. Checksums remain non-authenticating.

The journal sums validated single-report summaries while streaming in its
existing read-only transaction. No schema migration, new stored field, native
call, simulator call or extra per-report data structure is added. Existing
reports work without modification. Collector journals bound to the old source
fingerprint require a new journal after code updates, as ADR 010 specifies.

The deterministic fixture covers all nine coordinates yet retains three
unresolved proposed alternatives: baseline 1, pre-review excluded 2, resolved
alternative instances 2, equivalent 1, unresolved 3. Its resolved instances
reuse the same alternative gear hash, keeping the distinct-identity count
separate. It checks reverse-order aggregation, out-of-order journal appends,
idempotent retry, CLI readback and byte-identical rejection of inconsistent
counts. A seeded 100-case independent arithmetic oracle varies proposed,
resolved and equivalent counts across baseline/nonbaseline/final windows.

Direct command from the project root:

```powershell
$env:PYTHONPATH='.;tests'
python -m unittest test_gear_resolution_accounting test_merge_gear_windows test_gear_window_journal -q
```

Author result: 11 tests passed, 44.345 seconds. The actual collector/restart
regressions also passed, including the 1,089-coordinate fixture:

```powershell
python -m unittest test_collect_gear_windows -q
```

Result: three tests passed, 43.725 seconds. `git diff --check` passed.

Independent round one used Callers and Failure/security reviews configured as
`gpt-6-astra`, high effort. Per-review token usage was not exposed. Both were
clean; no correction round was needed.

Callers traced the actual producer through merger/journal and controller
progress, and ran:

```text
python -m unittest discover -s tests -p test_gear_resolution_accounting.py -v
2 tests passed
python -m unittest test_merge_gear_windows.MergeGearWindowTests.test_real_producer_and_merger_module test_gear_window_journal.GearWindowJournalTests.test_streaming_more_than_64_windows_and_gap_bounds -v
2 tests passed in 22.074s
```

Its independent empty/zero-coordinate/65-window probes preserved the distinction
between 1 baseline coordinate, 65 baseline review instances and 64 unresolved
proposals despite complete coverage.

Failure/security ran with `PYTHONPATH=tests;.`:

```text
python -m unittest test_gear_resolution_accounting test_merge_gear_windows.MergeGearWindowTests.test_coverage_grouping_gaps_and_invalid_metadata test_merge_gear_windows.MergeGearWindowTests.test_module_rejects_numeric_overflow -v
4 tests passed in 7.317s
python -m unittest test_gear_window_journal.GearWindowJournalTests.test_resume_idempotency_budget_and_scope_rollback test_gear_window_journal.GearWindowJournalTests.test_foreign_schema_corruption_and_busy_lock -v
2 tests passed in 5.642s
```

Its 66-window streaming probe covered 257 coordinates with baseline 1, proposed
256, resolved instances 32, equivalent 192, unresolved 32 and baseline reviews
66. Status left database bytes unchanged and feasible coverage false. An empty
out-of-range window counted only its reused baseline review. Reviewers closed
SQLite connections and made no source, build, native, simulator or deployment
changes; previously protected temporary directories stayed untouched.

Scoped advisory: `jev-1.14.0`, request
`ea61091c4d05ee1eea23d446d4fbdf68e50a3d0722aa3da35bdad4da1b796f98`,
732 input / 52 output tokens, 0.313 s; land-scoped 0.97, fix-first 0.02,
insufficient evidence 0.01, confidence 0.96. Advisory only, not proof or release
permission. The broad feature objective remains incomplete.
