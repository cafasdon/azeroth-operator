# Quiver observation simulation context — 2026-11-09

Baseline c9b996b. Previous goal turn made progress with equipped-bag capture.
This unit connects that observation to numeric simulation diagnostics and fixes
unsupported numeric quiver enum acceptance in the review preflight. Broad MCP,
full owned-BiS, native model and fresh active-identity goals remain incomplete.
No live client/account data, installation or deployment used.

## Concrete preflight failure

Pinned source9dc32ac8ca0d8d1b76337861d38b93c0f884af11 selects quivers by the
protobuf enum's string name; an unknown numeric enum yields no map entry and
zero-valued haste. Author hand-built quiverBonus999999: request_from accepted it.
New regression expected ValueError and failed (.001s). Added strict pinned enum
validation before review workflows execute, without blocking partial capture
absence/mismatch or changing valid template selections. The guard then passed.

## Direct checks

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
$env:AZEROTH_TEST_SIM_SOURCE_ZIP='C:/Temp/codex/forever-sim-9dc32ac/source.zip'
$env:AZEROTH_TEST_REAL_ENGINE='C:/Temp/codex/forever-positional-v4/artifacts/wowsimcli-positional.exe'
$env:AZEROTH_TEST_PUBLIC_TEMPLATE='C:/Temp/codex/forever-sim-9dc32ac/public-hunter-template.json'
python -m unittest test_hunter_quiver_observations test_equipped_bag_identity test_prepare_gear_window test_merge_prepared_windows
python -m unittest test_run_saved_journal.SavedJournalTests.test_real_mcp_multi_page_shared_budgets_weighted_ranking_and_revalidation
```

Ordinary MCP build7.40s. Ten scoped tests passed34.930s with explicit source ZIP,
qualified engine and public template, no skipped tests. Five quiver entries match
the pinned ZIP exactly. Real engine compares QuiverNone versus QuickdrawQuiver
with200iterations,45 simulated seconds, seed20261109; the latter mean DPS is
higher. Earlier focused four-test run with the same real engine passed1.101s.
This qualifies limited offline plumbing only, not native mechanics, optimality,
all classes or performance speedup. Engine SHA-256 remains
833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983.

Actual MCP self-equipment review confirms numeric projection/null/empty states.
An initial test invoked an empty baseline without the required catalogue and was
correctly refused. The replacement self-review fixture initially put a stack
count on equipment, correctly rejected by the strict reader. Removing that
invalid test-only count yielded1testpassed,.832s; no production guard relaxed.
An earlier ten-test batch had that fixture error and one optional source skip;
it is not presented as green. Corrected final batch above is green.

Merge tests reject malformed/mismatched bag observations and unsupported quiver
enums with recomputed request hashes, preserve legacy unknown observations, and
show request bodies/digests stay identical when adding diagnostic metadata.
Helper cases cover supported names/numbers, duplicate aliases, booleans/floats,
bounded pairs, private-field rejection, repeated IDs, unknown versus empty and
no input mutation. Actual MCP journal run passed1test36.766s, retaining item8217
reference diagnostics across two encounters, top1, partial work and checkpoint
restart. It uses controlled engine doubles for known multi-page rankings; the
separate real-engine check supplies limited engine evidence.

## Advisory plan

Jev resolved jev-1.14.0, request
4ae50188fe352db21606813b2ffc15fb0a0a2836f5443b7959527332e9d94085:
proceed.68/revise.29/insufficient.03, confidence.52;645input/47output,.318s.
This is uncertain advice, retained without reroll. Author proceeded with strict
numeric validation and diagnostic-only partial comparison, not a bonus proof.
Independent reviews below assess the concrete implementation.

## Independent reviews

Two independent read-only reviewers assigned gpt-6-astra high (runtime IDs/usage
not exposed) found no concrete correctness/privacy defect. Callers ran five
scoped helper/request/merge/actual-MCP tests,.681s, plus48 independent diagnostic
combinations across six enum selections in name/numeric form and missing/empty/
matching/unmapped captures. For each unchanged template, request bytes and
current-source checkpoint keys remain identical across observation-only changes;
inputs unchanged and no checkpoint directory created.

Failure/privacy ran four cheap helper/request/merge tests,.087s, and192 valid
selection/alias/capture cases through helper and request_from. Another37 malformed
fragments/duplicate-alias/private enum cases reject, while four protobuf-null/
default cases stay capture-unavailable with none reference, without inferring
observed absence. Prepared reports recompute diagnostics instead of forwarding
arbitrary metadata. Both bounded documentation readbacks were clean. Neither
reviewer ran the engine or repeated heavy journal tests. No builds/source writes/
Git changes/network/game/deployment. Live drift omitted because native testing
and installation remain deferred. Final format/diff checks passed.
Ordinary MCP SHA-256:
d66022be8a19026caec8550d9683312f83a880a306edf1f79be5f552bdfcce23.

Final advisory Jev resolved jev-1.14.0, request
96c056c972544aa02320fdf5def97db55191996046a755d96b993511c76a5a3d:
integrate.92/revise.06/insufficient.02, confidence.88;638input/52output,.371s.
Initial uncertain advice remains recorded. Author integrates only the supported
reference/preflight/reporting scope on concrete source, real-caller and limited
engine evidence. Full quiver/ammo/native/owned-BiS qualification remains open;
the broad goal stays active.
