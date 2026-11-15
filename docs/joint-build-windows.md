# Native joint search beyond 64 combinations

`tools.joint_build_windows` evaluates bounded portions of a larger declared
matrix. The original `evaluate_joint_build_case` still rejects a matrix above
64 coordinates. The new window API keeps the same fixed level-60/DPS/encounter
model and preserves all earlier identity, native-engine and privacy boundaries.

## Evaluation

`evaluate_joint_build_window(manifest, engine, offset='0', limit=64,
expected_search_id=None, cache=None)` selects at most 64 coordinates. The full
ordered pools and canonical base request determine one global search ID.
Candidate IDs and native request hashes remain stable across page sizes and
offsets. A different seed, trial count, actor context, source pin or pool changes
the identity. Existing reference constraints remain distinct from native-game
qualification; use the gear and talent generators for constrained model pools.

The result has `kind:joint_build_window_v1`, `total_decimal`, half-open
`start_decimal`/`end_decimal`, `next_offset` and `window_complete`. Counts and
rows refer to this window. `declared_matrix_complete` is true only if that single
window covers the entire matrix from zero. A terminal call returns no rows,
does not call the comparison runner, and proves no prior coverage. Its simulation
and objective flags are false.

Only selected coordinates are materialised and schema-validated. All reference
hashes/categories are bound, but a bad native fragment outside this window is
not approved by its successful neighbours. It rejects when selected. No failed
window yields a successful partial ranking. Existing numeric cache entries can
support a retry without claiming the failed window completed.

Each call retains the 100,000-distinct-request-trial and 120-second comparison
budgets, strict engine hash checks and the 256-KiB escaped response cap. A cache
hit is numeric result reuse; it does not mean a new simulation ran. The existing
cache remains owner-private and capped at 128 entries. No new result database,
history folder or automatic receipt persistence exists.

## Incremental reported coverage and top-K

`merge_joint_build_windows(manifest, windows, summary=None, top_k=16)` accepts
1–32 nonempty successful window receipts. It validates the expected full search,
coordinate selection/IDs, request hashes, seed/trials, finite numeric metrics,
counts, order/ranks and false proof flags. It checks repeated request metrics
where their rows remain available. Overlap and duplicate coverage reject.

The result has `kind:joint_build_summary_v1`. `coverage_ranges` coalesces touching
intervals, with at most 512 disjoint intervals. `reported_evaluated_decimal`
counts coordinates exactly once. `next_offset` is the first gap, and
`reported_matrix_complete` means the supplied reported intervals cover the full
declared range. `top_results` retains at most `top_k` (1–64), ordered by observed
mean DPS and then original ordinal. `top_k_truncated` is explicit. Keep the same
`top_k` when continuing a summary: discarded rows cannot fill a larger board.

Supply the previous summary with new, disjoint receipts. The merger validates
its identity, intervals, counts and retained top rows before adding coverage.
It does not mutate either input. All combined merge input is capped at 16 MiB,
with 512-KiB manifests, 256-KiB individual receipts/summaries and 256-KiB output.
CLI file reads also enforce these individual bounds and single-link ordinary
files. Contiguous traversal keeps one interval even for a very large matrix.

**Receipt trust:** these are unauthenticated user-supplied reports. Their hashes
bind a case and coordinate, not proof that a simulator ran. A supplied previous
summary can invent historical coverage or scores. The merger states this limit
and sets `simulation_performed`, `objective_evaluated` and all proof flags false.
It cannot audit metrics in discarded historical non-top rows. Preserve trusted
local receipts when an audit is necessary. No population-best, game-best or
statistical-separation certificate follows from complete reported coverage.

## CLI and MCP

```powershell
./.venv/Scripts/python.exe -B -m tools.joint_build_windows prepare --input "ABS_MANIFEST"
./.venv/Scripts/python.exe -B -m tools.joint_build_windows page --input "ABS_MANIFEST" --engine "ABS_ENGINE" --offset 0 --limit 64 --expected-search-id "SEARCH_ID"
./.venv/Scripts/python.exe -B -m tools.joint_build_windows merge --input "ABS_MANIFEST" --windows "ABS_RECEIPT" --top-k 16
./.venv/Scripts/python.exe -B -m tools.joint_build_windows merge --input "ABS_MANIFEST" --windows "ABS_NEW_RECEIPT" --summary "ABS_PREVIOUS_SUMMARY" --top-k 16
./.venv/Scripts/python.exe -B -m tools.joint_build_windows serve --input "ABS_MANIFEST" --engine "ABS_ENGINE"
```

The CLI prints one bounded result. It does not overwrite summary files itself.
Optional `--cache-root` uses the existing private numeric cache. MCP startup
fixes the manifest, engine and cache location. Calls expose only window selection
or reported receipt merging with the expected search ID. They cannot substitute
a file path, executable or base request. Native calls share one busy guard;
cancellation cannot admit a second native batch while the first worker remains
active. Merge is pure and starts no simulation. Both tools return fixed errors
without rejected payloads. No MCP client configuration changed in this slice.

## Offline evidence, 15 November 2026

```powershell
$env:AZEROTH_JOINT_ENGINE='C:/Temp/codex/forever-positional-v4/artifacts/wowsimcli-positional.exe'
./.venv/Scripts/python.exe -B -m unittest tests.test_joint_build_windows tests.test_joint_build_optimizer -v
./.venv/Scripts/python.exe -B -m tests.benchmark_joint_build_windows --engine 'C:/Temp/codex/forever-positional-v4/artifacts/wowsimcli-positional.exe' --probe 'C:/Temp/codex/forever-positional-v4/artifacts/equipment-probe.exe' --catalogue 'T:/Projects/Azeroth Operator/local-data/forever/catalogue.json' --cache-root 'T:/Projects/Azeroth Operator/local-data/joint-build-benchmark/cache' --output 'T:/Projects/Azeroth Operator/local-data/joint-build-windows/benchmark.json'
```

All **17 focused tests passed in 19.352 seconds**, with the native option enabled.
They cover original full-case behaviour, 125-coordinate Cartesian pages,
identity/page-size stability, gaps/overlaps, exact ties/top-K, malformed reports,
equivalent requests, bounds, actual CLI, and SDK busy/cancel/override behaviour.
A synthetic `128^5` tail exercises bounded selection and counts only its three
selected coordinates. It does not qualify that huge space or execute it natively.

The native benchmark first generated 128 valid gear variants from seven explicit
armour-slot domains and two valid talent allocations. The existing modifier
probe qualified generated equipment. Four 64-coordinate windows then evaluated
all **256 builds**, at two native trials each. An independent direct Cartesian
construction sent all 256 requests through the same pinned native adapter.
**All 256 means matched** within relative 1e-12 / absolute 1e-10. Incremental
top-16 ordering matched the exhaustive result exactly. The final 64-coordinate
window replay had 64 cache hits and unchanged numeric results.

The [full numeric record](benchmarks/joint-build-windows-2026-11-15.json) retains
model/rule/generator IDs and the summary. Generation took 17.76 seconds; native
windows took 26.61, 27.06, 26.85 and 28.84 seconds. The independent full traversal
took 99.95 seconds. These shared-host execution timings are not a speedup claim.
The compact summary was 9,355 canonical JSON bytes with one coverage interval.
The private cache stayed at 128 entries. Two trials test execution/accounting;
they cannot establish statistical superiority of the observed leader.

One independent round used caller/failure and data/guard lenses, Sol/high.
Neither found a code defect. Callers also ran an unmodified native CLI/MCP pair,
observed identical two-row receipts, a pure merge and bounded override errors.
Data review independently crossed ordinals 63/64 and 127/128 with the generated
256-coordinate manifest; all four native means matched direct Cartesian inputs.
Reported coverage retained two separate ranges with the first missing offset at
zero, and no execution/proof flag became true.
Native eligibility, ownership, complete game data and model accuracy stay
unqualified. No game client, addon or live-character test occurred.

Jev's advisory design call resolved to `jev-1.14.0`, request
`abe4370f3575a59cb54eb171f47acc1568edc106c9689287d36450a827fd3985`,
844 input / 47 output tokens, 0.327 seconds, choice `proceed`.
The release advisory resolved to the same model, request
`7d5c92188898702927fd47d61da550cd17e36b614489cdbe61161fd75e3d013c`,
798 input / 45 output tokens, 0.305 seconds, choice `ship` (probability 0.89).
Both are advisory, not verification. The independent review left no finding
that needs a follow-up ticket.
