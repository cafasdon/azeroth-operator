# Positional MCP integration — 9 November 2026

Scope: explicit exact-slot simulator input from the actual local Rust stdio MCP
through retained gear plans, current preparation and offline saved simulation.
No game action, addon/client installation, deployment or live test occurred.

## Inputs and contract

- Contract: `azeroth_equipment_preserve_slots_v1`.
- Qualified engine SHA-256:
  `833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983`.
- Public Hunter template SHA-256:
  `761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0`.
- Incumbent engine SHA-256:
  `0e195f35c247d78f05c7d87c2c017e7052d7ca532500a84ae19faba133ae99a5`.
- Local positional artifacts:
  `C:/Temp/codex/forever-positional-v4/artifacts`.
- Synthetic explicit-owner level-60 Orc Hunter observation, empty paired slots,
  equipped 2042/22347, bag ring 4549, matching profession/pet inputs.

The engine build/conversion provenance is recorded in
[the preceding qualification](positional-engine-verification-2026-11-09.md).
It is not rebuilt or automatically installed by the integration.

## Author verification

`cargo fmt` and `cargo build --locked --offline` passed after the Rust changes
(build 13.45s). Python commands used `C:/Python314/python.exe`,
`PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`.

```powershell
$env:AZEROTH_POSITIONAL_ARTIFACTS='C:/Temp/codex/forever-positional-v4/artifacts'
$env:AZEROTH_QUALIFY_TEMPLATE='C:/Temp/codex/forever-sim-9dc32ac/public-hunter-template.json'
$env:AZEROTH_QUALIFY_ENGINE='C:/Temp/codex/forever-sim-9dc32ac/wowsimcli.exe'
C:/Python314/python.exe -m unittest test_positional_gear_workflow -v
```

Final result: **4 tests, 11.287s, OK**. Actual MCP checks cover omitted/false
legacy equality, primary/additional exact drafts, native-category and required
level conflicts, malformed booleans, Retail rejection and snapshot preservation.
Capability tests reject duplicate/extra fields, numeric boolean substitutions,
nonintegral capability count, oversized response and wrong hash before adapter
cache lookup. Bulk positional execution is rejected before launch. Review
contract and wire flag must agree in both directions.

The actual offline workflow uses two encounter durations (45/90s), weights 1:3,
1,000 iterations, seed 20261109. It reports **4 coordinates, 0 unresolved
proposals, 3 distinct resolved gear sets, 6 cold runs, 0 warm runs**. The warm
checkpoint-only run returns identical rankings. Mode-changing resume and bad
engine identity reject without altering the existing plan/snapshot; the bad
engine run creates no new plan journal. Reported-space completion is true while
ownership, global ranking, game execution and BiS verification remain false.

Other scoped commands:

```text
python -m unittest test_prepare_gear_window test_merge_prepared_windows test_simulation_journal -v
11 tests, 23.177s, OK
python -m unittest test_run_saved_windows test_merge_gear_windows test_evaluate_owned_gear -v
10 tests, 37.849s, OK
python -m unittest test_real_positional_engine -v
2 tests, 3.481s, OK
```

The engine comparison retains bit-identical canonical legacy/positional metrics
against the incumbent. The sparse exact ring2 case returns mean DPS
650.883932442968. The adapter now rejects incumbent positional requests rather
than allowing an ignored wire flag. These raid simulations were author-only;
reviewers ran numeric capability/MCP/synthetic probes only.

## Independent adversarial review

Two `gpt-6-astra` high-effort read-only lenses: Callers and Failure/security.
Both ran the actual MCP and capability/cache/bulk tests (2 tests each, 1.333s
and 1.209s). Callers independently collected, journaled, prepared and merged a
ring2-hole capture: legacy had one resolved baseline and one unresolved draft;
positional had two resolved requests with exact flags. Missing layout metadata
was rejected. Old prepared JSON without the new `layout_sha256` fingerprint
was rejected; regeneration and changed-binding requirements are documented.

Failure/security independently verified malformed layout metadata rejection and
bad engine rejection before any collection call/output path creation. It found
a reverse contract/spec mismatch in direct exported review input: a contract
marker without `preserveSlots` silently produced a legacy request and could use
an unqualified matching-hash cache. The author reproduced it with a new test:
`AssertionError: ValueError not raised`. The minimal fix requires marker and
flag agreement; the four-test final run above passes.

Round 2 Failure/security found no remaining flaw: the three non-raid tests
passed in 1.252s. An independent 81-case contract/flag matrix accepted exactly
the three expected combinations and rejected the other 78. The original
second-ring downgrade and snake-case/mixed aliases were rejected. Callers was
omitted because the fix introduces no new file or caller path.

## Advisory receipts

Uncalibrated guidance only; independent probes determine the findings.

- Plan: `jev-1.14.0`, request
  `3e22ce835bb367f91fb59f2e56c2fec2d943273045aae3817110e47d2e7a80ce`,
  822 input/47 output tokens, 0.304s; proceed .92, revise .05,
  insufficient evidence .03, confidence .88.
- Reverse-mismatch triage: `jev-1.14.0`, request
  `686092bf51398f05ca7855d98e65f8decb9397087ebaa7a78756194683f86259`,
  498/46 tokens, 0.277s; fix .98, insufficient .02, no material issue .00,
  confidence .97.
- Final scoped landing: `jev-1.14.0`, request
  `56cac3bf570d8cfcb618c4a287169d42142df4c08ab4381b96cccbfc8a5c526c`,
  622/49 tokens, 0.257s; land scoped .96, fix first .03,
  insufficient evidence .01, confidence .94.

## Remaining boundaries

This qualifies the explicit positional path on limited synthetic/public inputs.
It does not prove complete observed ownership, all item/model/class/encounter
accuracy, native legality, statistical optimality or full owned BiS. Active
character selection is a separate unfinished requirement. Live testing remains
deferred. Legacy placement stays the default. Old prepared artifacts must be
regenerated and changed collection bindings require a new journal; existing
journals are preserved on rejection.
