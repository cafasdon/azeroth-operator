# Joint recipe restock: offline evidence

Scope: ADR-091, source and synthetic owner-local snapshots only. No installation,
deployment, live test, acquisition, crafting or external data request.

Direct commands:

- `cargo build --offline`: passed, final build 10.60 seconds.
- `cargo test --test recipe_restock_batch --offline`: 5 passed.
- `cargo test --lib recipe_restock --offline`: existing 4 tests passed.
- `PYTHONPATH=.;tests python -B -m unittest test_recipe_restock_mcp test_recipe_restock_batch_mcp`: 2 passed, 1.783 seconds.

Rust cases cover consumer demand compensation, producer credit once, shared
reagent totals, variable yield, request-order invariance, same-item net yield,
finite cyclic count solutions without order, divergent cycles, no crafts,
duplicate IDs/outputs and joint count limits. Input snapshot value stays fixed.

Actual SDK stdio tests cover old omitted/null mode compatibility, selected/bulk
conflict reuse, eight targets and rejected ninth/empty/duplicate/invalid/foreign
requests, cyclic status and divergent counts, missing bag data and unchanged
files. A separate brute-force integer oracle enumerates feasible craft pairs
for 48 target/initial-stock combinations; all reported counts/final ranges match.
Known bank output does not reduce bag-target counts. All eligibility/execution
flags stay false. No simulator is involved.

One combined Cargo command used the recipe_restock name filter for both library
and integration targets: it ran the four old tests and filtered all five new
ones. The explicit integration-target command above then ran all five. No source
or guard changed because of that invocation correction.

Plan/triage/final judgements use the main model with independent agent evidence.
No paid Jev call is authorised under the spending stop.

Independent round 1, requested gpt-6-astra/high, found no defect:

- Failure/math: existing compiled integration executable passed all five tests.
  An independent real-MCP oracle (seed910091) covered 160 two-recipe cases over
  the full 0..1000 count domain: 112 exact count/range matches and 48 correct
  refusals. Cases included self-consumption, nonpositive net yield, surplus
  stock, variable yields and cross-cycles. A slow-convergence cycle returned
  counts1000/999 with null order in both input permutations. Removing one saved
  output unit needed1001/1000 and correctly refused both permutations.
- Callers/privacy: 29 malformed/unavailable requests and eight invalid snapshots/
  selected-bulk conflicts refused. Valid call afterward returned [4,2]. All nine
  synthetic files stayed fixed. Errors omitted the private recipe-name sentinel.
  Source trace confirmed one owner-store read, hash/path/schema checks, closed
  nested request records and no alternate inventory input.

Review commands used the existing compiled recipe_restock_batch executable
(`--nocapture`, output9eeaf3) and inline Python asyncio SDK probes through
`python -B -` / `.venv/Scripts/python.exe -B -`. Actual oracle output227035:
`cases=160, accepted_exact_oracle_matches=112, rejected_without_admitted_solution=48`.
Cap probe84af41 printed both accepted permutations and both refusals. Caller
outputs0dd69f/c50369 printed29 and8 refusals, [4,2] health and unchanged files.
Full synthetic probe scripts remain in those tool records; no new probes were
run just to collect the evidence receipt.

Root final own-model judgement: ready for the bounded source/offline feature,
based on scoped direct tests, independent integer reference and clean review.
No live-drift lens applies because no installed state changed. Native crafting,
cyclic seed/order resolution, actual acquired-stock effects and complete game
coverage remain unproven. No source edits/builds/network/paid calls/private
snapshot access or game activity occurred in independent review.
