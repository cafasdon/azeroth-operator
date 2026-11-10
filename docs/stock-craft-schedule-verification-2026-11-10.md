# Saved-stock craft schedules: offline evidence

Scope: ADR-092. Synthetic snapshots and actual local stdio MCP only; no live
game, installation, deployment or acquisition.

Direct commands:

- `cargo build --offline`: passed; complete feature build 8.55 seconds, final schema-correction build 10.53 seconds.
- `cargo test --lib recipe_stock_schedule --offline`: 4 passed, 95 filtered, 0.01 seconds runtime.
- `PYTHONPATH=.;tests python -B -m unittest test_recipe_stock_schedule_mcp`: 1 passed, 0.658 seconds; final schema assertions passed in 0.585 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_recipe_restock_mcp test_recipe_restock_batch_mcp test_recipe_stock_schedule_mcp`: 3 passed, 1.770 seconds.

Rust cases include a greedy dead end followed by successful backtracking through
a seeded cycle, no initial seed despite bank stock, budget-unknown versus
exhaustive failure, stable paged digest, self-input recycling, invalid bounds and
8,000-craft iterative depth (8,001 states). Replay validates complete witnesses.

Actual SDK cases cover default/null compatibility, cyclic static graph plus
complete resource witness, pages, out-of-range offset, budget exhaustion, empty
schedule, single-recipe recycling despite gross batch shortfall, foreign-store
refusal, malformed options and unchanged files. An independent breadth-first
reachable-state reference covers 48 two-recipe stock/target/cross-input cases;
the returned complete schedules replay with no negative stock and meet targets.

Author corrections: an optional scope annotation initially attached before the
report variable existed; compilation refused it and it moved after report
construction. Empty array options initially became a default struct under
Serde's sequence support; the MCP field now checks object shape before typed
deserialization and redacts parse failures. Short nearby request-struct context
was used after long-signature patch contexts failed without edits. No input
guard was weakened or whitelist widened.

Plan/triage/final judgements use the main model. No external paid Jev call is
authorised under the spending stop. Independent review and final judgement follow.


Independent round 1 (requested gpt-6-astra/high):

- Failure/math: no concrete failure. Existing compiled unit tests passed 4/4.
  Independent three-recipe BFS through the real MCP (seed 921103) matched 400
  cases: 176 witnesses and 224 exhaustive failures. All 176 witness digests were
  rebuilt independently; 40 target permutations and 40 budget-one cases passed.
  A maximum-budget probe stopped at exactly 50,000 visited states, no steps and
  decision_complete false. A 101-step witness paged across nine requests retained
  its digest despite target reordering; replay used 100 crafts per recipe and
  finished at {10:101,20:100} against targets of 100. Records eaf3cc, 783726 and
  e7b9b0 retain exact commands/source/output. An initial invalid synthetic source
  label was refused; the corrected fixture ran the oracle.
- Callers/privacy: no blocking data fault. Actual MCP test passed in 0.610 seconds,
  compiled unit filter passed 4/4. SDK probe 48c211 confirmed selected-mode
  omission/null equivalence, old response preservation after optional fields
  were removed, seven malformed options and three invalid/missing path/IDs
  refused without sentinel echo, closed defaults/read-only annotations and one
  validated owner-store read. No public export/index path was added.

Caller noted a numeric schema precision gap: descriptions/runtime had correct
limits but machine schema lacked custom minimum/maximum. Root accepted this
API-contract issue and added exact Schemars ranges for budget and limit plus an
actual tools/list assertion. Runtime/search logic did not change. The tool
now says budget exhaustion to distinguish graph exhaustion.

Round 2: fresh gpt-6-astra/high caller review found no defect. Its narrow SDK
probe exited zero in 1.51 seconds and printed:
PASS: bounds/defaults/closed object; omitted=null; 3 valid objects; 13 invalid/redacted inputs; budget description
Actual schema shows limit 1..50/default20, state_budget 1..50000/default10000,
offset uint32/default0, additionalProperties false. An initial probe wrongly
expected an explicit offset.maximum; it was corrected to the existing uint32
format and actual overflow refusal. No source changed because of that probe
mistake. The failure/math lens was not repeated for this metadata-only fix;
its algorithm evidence above remains applicable.

Root final own-model judgement: ready for the bounded source/offline feature.
No paid Jev call ran under the spending stop. No live-drift lens applies because
no installed state changed. Native conditions/charges, current ownership,
variants, native craftability and full game coverage remain unproven. Reviews
used existing executables and synthetic data, with no builds, source edits,
external calls, real private data or game actions.
