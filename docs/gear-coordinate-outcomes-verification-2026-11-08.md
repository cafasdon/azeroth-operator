# Saved gear coordinate outcomes — 8 November 2026

Scope: offline source, synthetic inventory and local stdio MCP. No native game,
simulation execution, installation, deployment or CI. Full owned BiS remains
unfinished, alongside global simulation ranking and native/model qualification.

Resolution totals previously left no safe way to locate individual unresolved
coordinates. The ranker now associates each coordinate of an explicit bounded
window with baseline, resolved, equivalent, excluded or unresolved status.
Resolved/equivalent entries reference window-local review numbers; equivalence
can point to the reused baseline. Excluded entries identify pre-review physical
source conflicts. Unresolved entries retain only one of the four existing fixed
native/category/catalogue/unresolved reason codes, not raw conflict payloads.
They do not establish current game eligibility or complete legality.

The optional ledger is under `gear_search.combination_window.coordinate_outcomes`.
The collector persists it in ordinary journal artifacts. Merge validation
requires sequential exact decimal coordinates, exact row fields, known statuses
and reasons, baseline at zero, sequential resolved review numbers, and equivalent
references to already available reviews. Its status partition must match the
validated counts. Older reports without a ledger still work; explicit null or
malformed ledgers reject. No stored schema/version migration is needed.

`python -m tools.journal_forever_gear_windows --journal PATH --window-offset N`
inspects one exact saved start offset. The query checks the stored row size,
digest, metadata and ledger in the existing read-only transaction with existing
limits/deadline. Missing offsets reject; an interior coordinate is not silently
treated as a window start. This checks the selected row, not every other row in
the journal. Global status remains a separate operation. Returned details omit
paths, selections and private extras. All execution/BiS/feasible flags stay false.

An old report returns `coordinate_outcomes_available: false` and null details;
absence does not mean no unresolved coordinates. Updated collector source still
requires a fresh bound journal under ADR 010's existing drift rule.

Author checks include the actual new inspection module invocation, legacy
absence, invalid/interior starts, malformed coordinates, booleans/forward review
references, unknown reasons and inconsistent status counts. The larger actual
controller fixture checks ledgers for all 1,089 coordinates across nine saved
windows, without raw selections in inspection output.

Direct author commands with `PYTHONPATH=.;tests`:

```text
python -m unittest test_collect_gear_windows.CollectGearWindowTests.test_real_module_resume_identity_and_no_recollection test_merge_gear_windows -q
4 tests passed in 44.408s
python -m unittest test_gear_coordinate_outcomes test_gear_resolution_accounting test_gear_window_journal -q
10 tests passed in 27.594s
python -m unittest test_collect_gear_windows test_gear_draft_dedup test_rank_inventory_evidence -q
6 tests passed in 57.850s
```

An additional independent arithmetic assertion uses `divmod(coordinate, 33)`
for the 32-ring fixture: every positive diagonal selects the same physical
source twice and must be excluded with `reused_source_location`. No off-diagonal
coordinate may have that reason. With that assertion added, the actual large
module test passed once more, 23.610 seconds. These are collection/diagnostic
tests, not simulator timing or full feasible-set evidence. Whitespace checks
passed.

Independent round-one reviews were configured as `gpt-6-astra`, high effort,
Callers and Failure/security. Per-review token usage was not exposed. Both were
clean; no correction round was needed.

Callers ran these direct commands:

```text
python -m unittest discover -s tests -p test_gear_coordinate_outcomes.py -v
2 passed in 6.806s
python -m unittest discover -s tests -p test_collect_gear_windows.py -v
3 passed in 48.970s
python -m unittest discover -s tests -p test_gear_draft_dedup.py -v
1 passed in 4.002s
python -m unittest discover -s tests -p test_merge_gear_windows.py -v
3 passed in 27.151s
```

Its actual-MCP inline probe hid only advertised batching to compare legacy
single-review collection. The complete seven-field tuple matched batch-eight
collection exactly. For maximum swaps one and two, default and full-window first
six fields matched, default metadata stayed unchanged, and nonzero offset three /
limit four with collapse disabled validated correctly.

Failure/security ran with `PYTHONPATH=tests;.`:

```text
python -m unittest test_gear_coordinate_outcomes test_merge_gear_windows -v
5 passed in 38.648s
python -m unittest test_collect_gear_windows.CollectGearWindowTests.test_real_module_covers_product_above_old_request_limit -v
1 passed in 20.376s
```

Its bounded inline probe reported `PASS 17 bounded probes: unknown keys/privacy,
read-only, corrupt hash, empty windows, baseline equivalences`. Extra private
fields and corrupted digests rejected; empty windows and nonzero windows with
baseline-equivalent alternatives remained valid. Inspection preserved database
bytes. Both reviewers explicitly closed SQLite connections and left previously
protected temporary directories untouched. Neither made source, build, native,
simulator or deployment changes.

Scoped advisory: `jev-1.14.0`, request
`b05578a828a6cd8896a20e48e12bb56205df9652a51e7f69bbafa531e49bc000`,
848 input / 52 output tokens, 0.282 s; land-scoped 0.98, fix-first 0.01,
insufficient evidence 0.01, confidence 0.97. Advisory only, not proof or permission.
