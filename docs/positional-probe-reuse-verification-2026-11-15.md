# Bounded positional probe reuse: 15 November 2026

Base: `8174d3e`. The native joint evaluator's first warm profile spent 9.70 of
11.65 seconds in repeated positional capability checks. The capability command
emits a static JSON map in the sole hash-qualified executable. Running the same
command 32 times in a comparison supplies no additional contract information.
Engine integrity still needs a current byte hash for every request/cache hit.

## Change and guard

`compare()` owns a new ephemeral set. The first positional request validates the
qualified SHA, current file hash, deadline, strict capability response and final
file hash/deadline before recording success. Later requests check the qualified
SHA, current file hash and deadline again, then reuse only the static contract
result. The set never persists or crosses comparisons. No failed, malformed,
timed-out or post-probe-tampered result enters it. Standalone/default `run()` and
the bulk adapter retain their previous behaviour. No MCP argument exposes the
set. There is no new global memo or stored engine-trust decision.

The pinned Go capability source is a literal map, with source SHA
`4cf9b198226748b83b20690f1c8c7bff62132f08fbf74911f85f4dc0061f59e5`.
The existing qualified executable remains
`833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983`.
No game, addon, network fetch, private capture or local MCP configuration changed.

## Tests and callers

```powershell
$env:PYTHONPATH='.;tests'
$env:AZEROTH_POSITIONAL_ARTIFACTS='C:/Temp/codex/forever-positional-v4/artifacts'
$env:AZEROTH_JOINT_ENGINE='C:/Temp/codex/forever-positional-v4/artifacts/wowsimcli-positional.exe'
$env:GOMAXPROCS='2'
./.venv/Scripts/python.exe -B -m unittest tests.test_positional_probe_reuse tests.test_simulation_cache tests.test_simulation_comparison tests.test_joint_build_optimizer -v
./.venv/Scripts/python.exe -B -m unittest test_positional_gear_workflow.PositionalExecutionTests.test_capability_and_request_guards_precede_cache_or_bulk_execution -v
./.venv/Scripts/python.exe -B -m unittest discover -s tests -p test_run_saved_windows.py -v
```

Results: **28 passed in 11.949s**, **1 passed in 0.840s**, and **4 passed in
6.719s**. The native options above were enabled. No native test in the 28-test
group skipped. This includes real CLI/MCP and cold/warm native-cache calls.

The seven new tests cover one probe per batch, fresh probes in the next batch,
real encrypted disk-cache reuse, actual engine-byte changes before a second
cache read, a forged set with the wrong qualified pin, malformed/duplicate/large
responses, post-probe changes, deadlines and unchanged standalone probes.
Synthetic tests replace only the qualified constant and child launch. Their
actual file hashing and SQLite/DPAPI behaviour remain real. The separate native
case wraps real child calls and checks positive unchanged scores.

## Matched performance measurement

```powershell
./.venv/Scripts/python.exe -B -m tests.benchmark_positional_probe_reuse --engine 'C:/Temp/codex/forever-positional-v4/artifacts/wowsimcli-positional.exe' --cache-root 'T:/Projects/Azeroth Operator/local-data/joint-build-benchmark/cache' --output 'T:/Projects/Azeroth Operator/local-data/joint-build-benchmark/probe-reuse.json'
```

The benchmark loads the actual baseline adapter from fixed commit `8174d3e`
into a separate module. It alternates baseline/candidate on the same complete
32-request public case, at three recorded seeds, with all results already in
the same private cache. Counters wrap and delegate to the real hash and child
calls. No memory tracer runs inside either timed interval. The complete result
objects, including coordinate identities, ranks and numeric values, match
exactly for all **96 coordinates**.

| Seed | Baseline seconds | Candidate seconds | Ratio |
| --- | --- | --- | --- |
| 20261115 | 14.832 | 3.494 | 4.25x |
| 20271115 | 12.835 | 3.780 | 3.40x |
| 20281115 | 12.527 | 3.567 | 3.51x |

Each baseline batch performed 32 capability subprocesses and 97 complete engine
hash checks. Each candidate batch performed **1 capability subprocess and 66
complete hash checks**. No native simulation was necessary for these cache hits.
Total measured time fell from 40.193 to 10.840 seconds, a 3.71x ratio.
The [full numeric record](benchmarks/positional-probe-reuse-2026-11-15.json)
includes source/engine hashes, identities, counters and setup.
The candidate record retains both its measured CRLF-byte hash and its LF
content hash, which matches the checked-in Git blob.

These are three matched measurements of this local adapter/cache path on the
i9-12900HK host, with `GOMAXPROCS=2`. They do not establish cold-simulation speed,
better combat accuracy, search quality or superiority over another product.
Baseline always precedes candidate within each pair; no randomised timing study
or statistical confidence interval for the timing ratio is claimed.

## Independent review and small fixture repair

One full round used two Astra/high lenses: callers/failures and
security/data/guards. Neither found a new medium/high. Security independently
confirmed that subprocess failure/timeout leaves the set empty and that a
forged set cannot bypass the actual binary hash. Callers checked old cache,
comparison, positional and saved-window paths. No live-drift lens applied.

Callers found one pre-existing low: the deadline saved-window test's fake work
omitted `hunter_quiver_observation`. The same test failed with the baseline
adapter; real `merge_prepared()` always supplies that key. A trivial two-line
correction adds `None` to the fake and asserts it survives the result. All four
saved-window tests pass. A scoped caller re-check passed the exact test in
0.010s and found no added medium/high. No open low needs a follow-up ticket.
Production saved-window code did not change.

Jev was advisory and resolved to `jev-1.14.0`:

| Decision | Request SHA-256 | Input/output tokens | Seconds | Result |
| --- | --- | --- | --- | --- |
| Batch reuse design | 9c4fe0d626feda6739d302c1acd7344062dc633aa25b8216bd54f76c53803452 | 646 / 47 | 0.295 | proceed |
| Findings and fixture disposition | 8a5f1876e149a515515d4369e766f66c2c480023d8e4bf47612aecc935cabc26 | 601 / 50 | 0.298 | proceed |

All model, native-legality, ownership, active-character, statistical-superiority
and global-optimum limits from the joint evaluator remain in force. Live tests
stay deferred.
