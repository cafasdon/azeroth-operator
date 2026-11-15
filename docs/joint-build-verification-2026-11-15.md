# Native joint optimiser: offline verification, 15 November 2026

Base: `718feb8`. This change connects the pure joint reference planner to a
real local native simulator. No game, private capture, external optimiser,
Blizzard data mining or live character test occurred. No MCP client configuration
was changed. Source support and successful stdio calls are separate from
installation and native-game qualification.

## Inputs and evidence

- Engine: `C:/Temp/codex/forever-positional-v4/artifacts/wowsimcli-positional.exe`,
  SHA-256 `833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983`.
- Upstream WoWSims revision: `9dc32ac8ca0d8d1b76337861d38b93c0f884af11`.
- Public BM and MM Hunter fixtures are checked in under `tests/fixtures/`.
  The WoWSims MIT licence accompanies the derived descriptor assets.
- The descriptor is 115,868 bytes, 16 dependency-first files, SHA-256
  `3d06a6abeec9567db5c891b4f39c7d5827ec68503e9a1df95f329c3128600225`.
  A second local reflection export produced identical bytes. Independent
  review matched all recorded proto/generated-Go hashes to the qualified source.
- Catalogue lock SHA-256 over checked-in LF bytes:
  `2383a1aebf9745a88fb257dad343d323e9c9a16c91ab808dd3817c2b1a254300`.

The descriptor manifest records the exporter, dependencies and offline command.
No surviving local protoc existed. Export used pinned generated Go reflection
and local module replacements with network fetching disabled. The original
positional engine did not change. LF attributes preserve every byte-hashed text
asset on Windows checkouts.

## Direct tests and real callers

```powershell
$env:AZEROTH_JOINT_ENGINE='C:/Temp/codex/forever-positional-v4/artifacts/wowsimcli-positional.exe'
$env:GOMAXPROCS='2'
./.venv/Scripts/python.exe -B -m unittest tests.test_joint_build_optimizer -v
./.venv/Scripts/python.exe -B -m unittest discover -s tests -p test_build_search.py -v
```

Final author result: **9 optimiser tests passed in 6.422s**, including the real
native CLI and SDK stdio success path. **9 planner tests passed in 4.022s**.
These are focused direct tests, not a CI pipeline or complete project suite.

The tests include independent Cartesian ordering, a non-separable interaction
trap with a worse intermediate state, stable ties, all five subtree categories,
canonical-equivalent request deduplication, strict unknown fields/scalars/enums,
case and reference drift, budget rejection, invalid numeric results and failure
before execution. Actual MCP calls reject missing/stale identities and input,
path and executable overrides. Concurrent native calls yield one success and
one fixed error. The worker retains the busy guard if a caller cancels its wait.

The 32-coordinate synthetic schema-equivalence case needs only four native
request evaluations: 28 coordinates share canonical requests. This test uses
a controlled numeric adapter double, not simulated combat. The independent
interaction fixture's full search finds score 10; either isolated change from
the score-5 baseline gives score 4. These establish combination handling, not
superiority over a commercial optimiser.

The explicit prepared public case also passed the actual `prepare` CLI and
reported 32 coordinates, 32 unique requests and 6,400 iterations without a run.

## Native exhaustive reference

```powershell
$env:GOMAXPROCS='2'
./.venv/Scripts/python.exe -B -m tests.benchmark_joint_build_optimizer --engine 'C:/Temp/codex/forever-positional-v4/artifacts/wowsimcli-positional.exe' --cache-root 'T:/Projects/Azeroth Operator/local-data/joint-build-benchmark/cache' --output 'T:/Projects/Azeroth Operator/local-data/joint-build-benchmark/benchmark.json'
```

The cache directory received current-user/SYSTEM/Administrators-only ACLs before
the run. The benchmark used 200 trials per request, 45-second encounters and
seeds 20261115, 20271115 and 20281115. Each seed covered all 32 combinations of
two gear, talent, rotation, timing and consume fragments. Each independently
constructed `itertools.product` traversal re-ran the native requests without the
planner/materialiser/deduplication helpers. **96 of 96 coordinate means matched**
within relative tolerance 1e-12 and absolute tolerance 1e-10. Warm results matched
the earlier numeric results exactly. All 96 warm coordinate reads were hits.

The observed leader was ordinal 8 on all three seeds, with means 1062.0024,
1060.6934 and 1052.4843 DPS. These are model estimates from public fragments.
They establish neither legal combinations nor population-best or pilot results.

The [numeric ledger](benchmarks/joint-build-2026-11-15.json) records all setup
hashes, identities, seeds, means, counts and timings. The original cold runs took
22.97–26.73 seconds per matrix while focused checks shared the host. Untraced
warm replays took 10.20–11.59 seconds. Original instrumented warm timings remain
separately labelled. The driver now measures memory in another replay, outside
both wall intervals. Python traced peaks were below 3.66 MB and exclude the
native process. CPU and native peak-memory measurements were not collected.
These figures do not establish isolated comparative performance or a speed lead
over another product. They identify engine-validation overhead as future work.

A separate warm `cProfile` run took 11.65 seconds: 32 positional capability
subprocesses consumed 9.70 seconds including their hash checks. It counted 97
engine hash reads. Preflight consumed 0.44 seconds. This measured bottleneck is
the next performance target; the first slice preserves those existing checks.

The final private numeric cache contained 96 entries, within its existing
128-entry cap. No request/profile history was introduced. No cache ACL bypass
was needed.

## Independent review

Round 1 used two Astra/high agents: callers/failures and security/data/claims.
One medium was reproduced by both reviewer and author: a sole player in an
inactive second raid party produced a successful empty-raid score of zero.
The evaluator now accepts exactly one party and only default/one active party.
The regression rejects inactive, negative and excessive party counts before
engine verification or simulation. Valid zero/default and one counts remain
accepted. The scoped round-2 caller re-check returned no medium/high issue and
retained the valid native scores 1012.7505 and 969.9114 DPS.

One low provenance issue concerned automatic CRLF conversion of the exporter
and public fixtures. Two LF attribute rules fixed it. `git check-attr` confirmed
the rules. No open finding needs a follow-up ticket. No production/live-drift
lens applied. Reviews stayed within one full round and one scoped re-check.

Jev was advisory only. All calls resolved to `jev-1.14.0`:

| Decision | Request SHA-256 | Input/output tokens | Seconds | Disposition |
| --- | --- | --- | --- | --- |
| First executable slice | d4488544c84da398541a8909cbf7e24b96b508ad990b6b049dd255b135cf2089 | 602 / 47 | 0.295 | proceed |
| Inactive-party finding | 75617db84460a781ac9b13754f39051ae0017e6efb5066fa7ace6c62c455db4a | 551 / 56 | 0.328 | medium fix |
| Final finding disposition | 9df351bb4d4f390a9817387d90cbbad756a56a1c5876bb36dd8a494362c146ec | 638 / 46 | 0.310 | accept |

Discovery/research used Sol/medium; implementation and adversarial judgements
used Astra/high. Those are requested native model names. Per-agent resolved
model identifiers and token totals are not exposed by this harness. The account
meter changed from 15% to 17% weekly used during this work; it also includes other
concurrent chats, so it is not a task cost measurement.

## Remaining limits

No full legal talent generator, native field-effect qualification, complete
owned inventory, active-character selection, lower-level model, calibrated
statistical stopping, larger-space joint ledger or matched competitor benchmark
is implied. The existing private capture guards and game-action exclusions did
not change. All such proof flags remain false. See optimisation-machine.md for
the acceptance gates and the executable next dependencies. Live tests remain
deferred.
