# Offline gear-window collection verification

Date: 8 November 2026. Scope: source and synthetic local stdio MCP only.
No native WoW interaction, installation, deployment, simulator execution or CI.

The new `python -m tools.collect_forever_gear_windows` entry point collects
bounded plan-only windows into an explicit journal. The real-module fixture
uses two ring candidates and a nine-coordinate product. One invocation commits
four coordinates; a later invocation finishes the remaining windows. Repeating
against complete progress makes zero collection calls and leaves bytes unchanged.

Tests reject changed templates/filters, unbound journal adoption, invalid limits
and a template changed after collection. A controlled stalled collector times
out without changing prior committed bytes. Shared merger/journal tests reject
mixed collection fingerprints without replacing existing rows. Private fixture
markers are absent from CLI stdout/stderr.

Direct test command (PowerShell, project root):

```powershell
$env:PYTHONPATH='.;tests'
python -m unittest test_collect_gear_windows test_gear_window_journal -q
```

Plan advisory: `jev-1.14.0`, request
`c3d0442f53cca1ef8e585bb894068ece05f99848ee46aa8b4cd38d6e53d5d4f7`,
605 input / 47 output tokens, 0.310 s; proceed 0.70, revise 0.24,
insufficient evidence 0.06, confidence 0.55. Advisory only.

Author command above passed eight tests, 37.390 seconds. The old pre-compaction
test-process handle was no longer available, so this is a fresh completed run,
not an inferred result from its prior output. Ranker evidence/deduplication and
level regressions passed seven tests, 7.460 seconds:

```powershell
python -m unittest test_rank_inventory_evidence test_gear_draft_dedup test_simulator_level -q
python -m tools.collect_forever_gear_windows --help
git diff --check
```

Independent round-one Callers and Failure/security reviews were configured as
`gpt-6-astra`, high effort. Per-review token usage was not exposed. Callers found
no actionable issue and ran:

```powershell
$env:TEMP='C:\Temp'; $env:TMP='C:\Temp'; $env:PYTHONPATH='tests'
python -m unittest test_collect_gear_windows test_merge_gear_windows test_gear_window_journal -v
```

Result: 11 tests passed, 65.955 seconds. It traced the real module and shared
renderer, old unbound formats, bound mixing and the documented caller contract.

Failure/security's independent synthetic gap probe seeded [0,2) and [4,8).
With page size four, the observed requests were exactly (offset 2, limit 2),
then (offset 8, limit 1), ending at nine covered coordinates. Executable and
catalogue drift rejected before collection with unchanged journal bytes;
cancellation preserved committed progress. A 0.2-second SQLite contention
deadline stopped in 0.29 seconds, demonstrating bounded checks rather than a
hard real-time promise.

Failure/security's final round was clean. Its commands and output were:

```text
python -m unittest discover -s tests -p test_collect_gear_windows.py -v
Ran 2 tests in 21.949s — OK
python -m unittest discover -s tests -p test_gear_window_journal.py -v
Ran 6 tests in 18.450s — OK
python C:/Temp/codex/gear-resume-failure-gap-review-20261108.py
PASS gap clamp: seeded=[0,2),[4,8); window-size=4; calls=[(2, 2), (8, 1)]; covered=9; complete=True
```

The retained gap probe used a synthetic collector, not an executable. Inline
drift, busy-deadline and cancellation probes each returned PASS as described
above. Neither reviewer changed source, built code or touched protected old
temporary directories. No confirmed finding required a correction round.

Final scoped advisory: `jev-1.14.0`, request
`70ebb736347b279b0aaf77c1851e5079ced449f0027746994fcc2252e8b71e07`,
653 input / 52 output tokens, 0.319 s; land-scoped 0.84, fix-first 0.15,
insufficient evidence 0.01, confidence 0.76. This used the completed Callers and
interim Failure/security probe report; advisory only, not proof or permission.

Full owned BiS, simulation aggregation, active-character detection and native
qualification remain unfinished; the broad feature objective remains active.

## Product beyond the old whole-request limit

A subsequent real-module regression captures 32 distinct catalogue ring IDs in
one synthetic owner-local inventory. Each requested ring slot has those 32
candidates plus the baseline option: 33 squared, or 1,089 reported coordinates.
This exceeds the old 1,024-coordinate whole-request limit. The test launches
the actual controller module and offline MCP executable, not a collector stub.

One invocation saves 128 coordinates, then a fresh process resumes the remaining
eight windows. Read-only SQL independently checks the nine stored intervals
against arithmetic ranges of width 128, with a final 65-coordinate interval.
Expanding those intervals equals exactly integers 0 through 1,088, without gaps
or overlaps. Every report stays within 129 resolved drafts including the reused
baseline. A final invocation performs zero recollection and leaves journal bytes
unchanged. All four BiS/feasible-set/simulation/game flags remain false.

```powershell
$env:PYTHONPATH='.;tests'
python -m unittest test_collect_gear_windows.CollectGearWindowTests.test_real_module_covers_product_above_old_request_limit -v
```

Author result: one test passed, 19.071 seconds. This runtime is fixture-specific,
not a simulator benchmark or general search latency claim. Candidate filters
and item legality can still leave unresolved or excluded sets; arithmetic
coordinate coverage does not close the full-owned-BiS requirement.

Independent Failure/Callers review (`gpt-6-astra`, high effort; token usage not
exposed) ran the same exact command: one test passed, 20.015 seconds. It traced
the real controller, ranker, journal/merger and fixtures, checked the bounded
documentation claims, and found no actionable issue. One lens was used for this
test-only increment; no runtime guard or production code changed.

Scoped Jev advisory: `jev-1.14.0`, request
`99aa884d4fab1c862ae107c5757244fbb648107ddcf56336b2ed84b3b1a79602`,
620 input / 52 output tokens, 0.327 s; land-scoped 0.79, fix-first 0.12,
insufficient evidence 0.09, confidence 0.68. Advisory only. No simulator or native
game execution, installation, deployment or CI ran.
