# Unified observed-owned gear evaluation — 9 November 2026

Base source: `7ec69b1`. ADR 027 combines all-slot collection and saved-journal
multi-encounter simulation. Full owned optimisation and native qualification
remain incomplete; this is an offline workflow increment, not a BiS result.

## Author evidence

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_evaluate_owned_gear test_run_saved_journal test_collect_gear_windows -v
```

Six direct tests passed in **79.415s** before the independent-review fixes.
These exercised the actual Rust stdio MCP, real private journals and synthetic
simulation boundary. Existing saved-journal/collector tests cover resumed large
products, metadata/content drift, shared engine budgets and incompatible binds.

After the scoped fixes below:

```text
python -m unittest test_evaluate_owned_gear -v
Ran 3 tests in 15.937s — OK
python -m tools.evaluate_forever_owned_gear --help
Exit 0; all-slot workflow options; no narrowing candidate/slot filters
```

The workflow test covers all 17 slots, unrestricted observed-storage selection,
bounded collection and one-new-run resumption across weighted profiles, zero-new-run checkpoint
reuse, unchanged source/plan bytes and separate incomplete/resolved flags.
It checks output-path/type/hash/options before creating new plans, inherited
deadlines, early-error rejection and explicit cancellation propagation.

The first sparse-ring fixture deliberately remains unresolved. Actual MCP
review of a ring in equipment slot 12 with slot 11 empty returns null simulator
spec and an unresolved slot. Existing `review_equipment_catalogue` withholds
such requests because the engine fills the first free paired slot. All four
reported coordinates can be collected and all saved resolved requests simulated
while `status` remains `unresolved_plans`. A separate all-17-slot fixture with
both ring positions filled reaches reported computational completion. Neither
fixture becomes verified ownership/native/full BiS.

Initial author tests also reproduced a missing preflight directory-type check:
an existing regular file used as results directory could create a plan before
late rejection. The directory/file check now rejects before collection; its
failing assertion then passed in the scoped workflow test.

## Actual offline pinned engine

The opt-in `test_real_owned_workflow` verifies engine/template hashes before
running and defaults to skip without explicitly provided local paths. No data
or executable download occurs. Its recorded final run used:

- Engine SHA-256: `0e195f35c247d78f05c7d87c2c017e7052d7ca532500a84ae19faba133ae99a5`.
- Public template SHA-256: `761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0`.
- Synthetic level-60 Orc Hunter, matching professions/pet level, sparse saved
  equipment 2042/22347 and one carried 4549; this is not a native character export.
- 45/90-second encounters, weights 1:3, 1,000 iterations and seed 20261109.

```text
python -m unittest test_real_owned_workflow -v
Ran 1 test in 5.547s — OK
slots=17; coordinates=4; unresolved=1; cold_runs=4; warm_runs=0
top_weighted_gain_pct=0.2909875101977293
```

Cold and checkpoint-only ranked rows were exactly equal; original capture and
plan bytes stayed unchanged. The unresolved paired-slot model gap stayed visible
and `reported_space_results_complete` stayed false. The observed gain is only a
point estimate in this synthetic model, not a recommended build or statistical
optimality. The engine is the separate offline simulator, not a game client.

## Review round one and corrections

Two independent `gpt-6-astra` agents at high effort: Callers and Failure/security.
Both ran the original focused workflow test, passing in 14.188s and 12.812s.
Callers additionally ran the two saved-journal tests in 31.029s.

Callers reproduced collection-stage expiry surfacing as
`ExceptionGroup(BrokenResourceError)` after 2.058s with one saved window out of
25 coordinates, zero engine calls and a 20-second total budget. Author replay
of its bounded synthetic actual-MCP script reproduced the same failure at
2.029s. The orchestrator now revalidates committed binding/identity and uses the
remaining original deadline. Author replay returned `collection_incomplete`
at 2.434s, one saved window and one engine call. The permanent regression also
asserts early errors and task cancellation are not recovered.

Failure/security reproduced an input engine named `plans.sqlite-journal`
being removed by SQLite before simulation when the main journal was
`plans.sqlite`. Author's new direct regression failed with the input missing.
Reserved main/sidecar namespace checks now reject before opening the journal;
the regression passed in 0.040s for `-journal`, `-wal` and `-shm`, keeping every
input byte and leaving no plan DB. Resolved and existing hardlink aliases are
included. No real engine or user file was used in destructive probes.

## Review round two

The same two independent high-effort lenses rechecked the fixes. Neither found
a remaining concrete defect. Callers reran its original actual-MCP deadline
probe: returned `collection_incomplete` in 2.467s, one committed window/coordinate
out of 25 and one engine call. Changing the template before resume then rejected
with zero additional engine calls; coordinate/full-space/BiS flags stayed false.
Its three focused regression tests passed in 17.669s.

Failure/security reran the original real-CLI sidecar collision: exit one,
empty stdout, protected engine bytes unchanged and no plan journal created.
Twelve independent exact-name, hardlink, symlink and simulation-root overlap
probes across all three sidecar suffixes rejected without losing input bytes.
Its three focused regressions passed in 17.200s. Both reviewed recovery binding,
deadline, early-failure/cancellation and completion guards. Runtime stayed frozen.

Both subsequently checked the completed usage/ADR/evidence docs and found no
material discrepancy. A wording refinement describes unrestricted observed-
storage selection rather than claiming the fixture independently qualifies
bank contents. Neither reviewer reran the actual engine; real-engine results
above are explicitly author evidence. Their probes used no HTTP, native game,
deployment, builds or repository/Git mutations. Synthetic files were cleaned
and owned processes exited.

## Advisory receipts

All are advisory only, not proof, permission or calibrated probabilities:

- Plan: `jev-1.14.0`, SHA `10b52b1369ac1c12f502018641935349f5d688f780e055279a0177420761b1f4`,
  729 input/47 output tokens, 0.272s; proceed 0.81, revise 0.18,
  insufficient-evidence 0.01, confidence 0.72.
- Author model-gap/preflight triage: `jev-1.14.0`, SHA
  `996ebbc22a26a1e7a8d3f25ba1a621cc00d5fe704afd778a7848454b82368e9a`,
  787/100 tokens, 0.281s; retain guard/test gap 1.0, confidence 1.0;
  fix directory preflight 0.99, insufficient 0.01, confidence 0.98.
- Round-one finding triage: `jev-1.14.0`, SHA
  `7e5068ad9c3aec635fa81791a26e151c3e23cfb96f2ad668bab0aeabe0291467`,
  767/87 tokens, 0.308s; fix each concrete finding 1.0, confidence 1.0.
- Final scoped landing: `jev-1.14.0`, SHA
  `d82f2499cd78009bad865a8c0b74d96754ca52fd237e4012b4b34167c4dfb901`,
  1,011/49 tokens, 0.295s; land-scoped 0.98, fix-first 0.02,
  insufficient-evidence 0.0, confidence 0.97.

No game testing, addon install/deployment, HTTP/data sync or CI was performed.
Complete ownership, active-character identity, broader model/native legality and
statistical optimality remain unqualified. The broad goal remains active.
