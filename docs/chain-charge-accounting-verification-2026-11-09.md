# Repeated producer charge accounting — 9 November 2026

Baseline `b738889`. Scope: original-capture cumulative charges for repeated
producer steps and their optional stock preparation. No addon/schema/API change,
installation, deployment, external data lookup or live game testing.

## Regression and author checks

The actual Rust stdio MCP reproduced two one-craft uses of producer 102 with one
saved charge. Both legacy per-request checks were true and no cumulative check
existed. The new regression failed `None is not False` at the second step's
cumulative check before source changes. A per-recipe shared ledger fixes this
without changing the conditional material-draft definition.

`cargo fmt` and `cargo build --locked --offline` passed; ordinary build 11.47s.

```text
C:/Python314/python.exe -m unittest test_chain_charge_accounting test_reagent_chain_stock test_recipe_reagent_chain test_recipe_dependencies test_bulk_recipe_cooldown -v
Ran 5 tests in 1.474s
OK
```

Python environment: `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`. The expanded new
test alone passed in 0.277s. An independent prefix-sum oracle checks 18 scenarios:
craft counts 1, 2 and 1,000 against missing/zero/one/two/three/u32-max charges.
It verifies per-step and stock-plan parity, first-shortfall positions, no-stock-
plan output, original cooldown versus newer inventory, unchanged source bytes,
synthetic privacy markers and false craftability/game-execution flags.
Historical per-request fields remain separate from cumulative evidence. After
adding an ordinary unique-batch exact response-shape assertion, the new test
passed again in 0.294s.

## Advisory and review

Plan advisory: `jev-1.14.0`, request
`4db633b27688c8e84b0fee71626b235fe12b0ca39908b0a0763684d62c0cd34f`,
523 input / 47 output tokens, 0.291s. Proceed 0.65, revise 0.32,
insufficient evidence 0.03, confidence 0.48. This uncertainty is retained;
the advisory is not test evidence or authorisation.

One independent review round, two `gpt-6-astra` high-effort lenses; actual agent
token usage was not exposed. Callers ran:

```text
C:/Python314/python.exe -m unittest -v test_chain_charge_accounting test_reagent_chain_stock test_recipe_dependencies test_bulk_recipe_planning test_recipe_reagent_chain
Ran 5 tests in 1.654s
OK
```

Its independent actual-MCP probe derived eight printed steps at 1,000 crafts
each, with recipe 102 requested four times against 3,000 charges. Cumulative
counts 1,000/2,000/3,000/4,000 yielded the first shortfall at printed step six;
chain/stock accounting matched exactly. Duplicate ordinary batch IDs rejected.
Caller tracing confirmed at most 8,000 cumulative crafts and unchanged ordinary
batch/dependency wrapper semantics.

Failure/security ran:

```text
C:/Python314/python.exe -m unittest test_chain_charge_accounting test_recipe_reagent_chain -v
Ran 2 tests in 0.626s
OK
```

Independent actual-MCP probes verified isolated recipe pools, a cyclic blocked
draft retaining `[102,102,103]` with the repeated producer's cumulative deficit
and null stock plan, zero timer without charges remaining unknown, rejection of
build/realm/reversed-inventory/crafts-1001 inputs and unchanged capture bytes.
Both reviewers found no actionable fault. Neither built, edited project files,
used HTTP, opened SQLite or operated the game; the parent's ordinary offline
build establishes correspondence to the reviewed source. No fix round needed.

Final source-only advisory: `jev-1.14.0`, request
`0f36bc411815d9378593ef93d7be985998e963d8ff2c890c3066b40a64bb9ab4`,
784 input / 52 output tokens, 0.287s. Land scoped 0.83, fix required 0.06,
insufficient evidence 0.11, confidence 0.74. Uncalibrated advisory only; all
uncertainty remains recorded. Author disposition: land the bounded correction.
Native qualification, shared cooldown groups and broad profession/owned-gear/
active-character coverage remain open; the overall goal is not complete.
