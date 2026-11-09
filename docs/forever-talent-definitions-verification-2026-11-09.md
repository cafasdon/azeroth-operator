# Forever talent-definition observations — 9 November 2026

## Finding and boundary

The native talent collector previously retained active entry IDs/ranks/tree IDs,
while the simulator accepted an independently supplied talentsString. No observed
allocation comparison occurred. This unit adds stable definition/purchased-rank
observations needed for that comparison; it does not implement the mapping or
qualify simulator build fidelity. See ADR 050 for the exact capture contract.

Pinned primary reference: Gethe/wow-ui-source revision
9dc84f0d3bbae1768ac3405b3716000bd93eda06, SharedTraitsDocumentation.lua SHA-256
6757fa202575c2362165b77bce6c1c81526a5c3f7a983294de3fc68db52f63db.
Its public entry API distinguishes definition IDs from entry IDs and permits
nil definition IDs for subtree selection. Local cached documents and pinned
WoWSims class JSON were inspected; no data lookup occurs at runtime.

## Author verification

- `cargo fmt`: passed.
- `cargo test --locked --offline --test forever_talent_definitions`: three
  Lua 5.1 actual Core/export/decode tests, initially .40s after 19.99s build.
- Author found the final staged getter outside the last graph check. New fixture
  stage_mutates_final changes an inactive node on that final getter. Red:
  incomplete_changed_or_invalid_base_graphs_fail_the_whole_section failed with
  stage_mutates_final. Move the final staged sample before the last graph read.
  Green: three tests, .39s after 1.14s incremental compilation, all passed.
- `cargo build --locked --offline`: ordinary MCP rebuilt, 8.01s.
- `cargo build --locked --offline --example verify_addon`: passed, 1.91s.
- With `PYTHONPATH=.;tests`,
  `python -m unittest test_forever_talent_definitions test_snapshots`: seven
  tests, .683s, OK. Actual Rust stdio inspection/summary and Python reader.
- `cargo test --locked --offline --test talent_metadata`: one existing Retail
  metadata test, .02s after 2.07s compilation, passed.

Fixtures check purchased versus boosted active rank, purchased zero, nullable
subtree definition, optional secret/getter/choice uncertainty, staged-state
uncertainty, complete graph/context/API changes, dense/unique ID lists, budgets,
forbidden-action counters and no Forever config-name emission. Actual MCP tests
cover 32 malformed new-metadata captures, absent staging, unknown tuples, legacy
capture behaviour, zero/4096 active-node boundaries and unchanged source bytes.
All data is synthetic and temporary; no game or real character was used.

## Uninstalled package

Forever/live/interface 16001, version 0.47.0-dev:

`dist/forever-talent-definitions-0.47.0-dev/AzerothOperator-0.47.0-dev-forever-live.zip`

SHA-256 be58264a6ef36ebdfdcb3fe9296bff532eb5721f3b35e0b671e2e888aa2d0bad.

`python -m tools.verify_addon_packages --directory dist/forever-talent-definitions-0.47.0-dev --verifier target/debug/examples/verify_addon.exe`
passed source/rendered-manifest and packaged Lua 5.1 roundtrip checks. Forever
capture fixture=true, changed-interface rejection=true, live_qualified=false.
No installation, deployment, native allocation or game action was performed.

## Independent review

One clean round, gpt-6-astra high Callers and Failure/privacy. Both ran the three
prebuilt new Rust tests (.51s/.44s), passed. Callers ran the old Retail test
(.02s), passed. Actual MCP Python test passed for both (.656s/.458s). Callers
verified archive SHA and packaged Lua equality with source after line-ending
normalisation, and traced the unchanged unsupported Forever import and explicit
simulator template behaviour. No concrete fault found. Native/deployment lens
omitted because those operations remain deferred; agent token usage unavailable.

## Advisory receipts

Jev 1.14.0, advisory only:

- Plan eb7fc0bf40077501cfd1650d7cb698e5c4c7f850d2a4c447ecff7cdb173b6729:
  proceed .50 / revise .37 / insufficient .13; 753 input, 47 output, .283s.
  Author retained the uncertainty and limited this unit to observations, adding
  committed-entry ambiguity checks and explicit unverified-model flags.
- Final-stage bracket finding 7659184ad750192fc3bb1af0471951de88cfa0bcae1283bdec3245a97c96884e:
  fix .95 / dismiss .04 / insufficient .01; 564 input, 44 output, .316s.
  The concrete red fixture and minimal ordering correction establish the fix.
- Final bounded-source disposition 277105b2a653bb8a004ce02c33fba61befb3ec32b776b54dde3926ca5cc47b1e:
  Jev 1.14.0 land_bounded_unit .98 / revise .01 / insufficient .01;
  626 input, 56 output, .301s. Advisory only. Author accepts the source unit
  on the concrete tests and independent reviews; live testing and simulator
  talent mapping remain unverified. Final cargo fmt --check passed.
