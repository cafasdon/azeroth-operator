# Captured reagent-chain drafts — 9 November 2026

Base source: `cd3c823`. `review_forever_recipe` now accepts optional
`suggest_reagent_chain` for one exact bulk recipe. The draft expands captured
fixed intermediate producers, shares a modeled stock ledger and uses reported
minimum output quantities. It never executes crafting or acquisitions. Detailed
assumptions and bounds are in ADR 019.

## Author evidence

```text
cargo build --locked --offline
Finished dev profile in 23.76s
```

After updating the tool description, a second incremental offline build finished
in 4.52 seconds. `cargo fmt -- --check` and `git diff --check` passed.

Actual local Rust stdio MCP checks used synthetic owner snapshots without the
local catalogue, demonstrating no external reference lookup dependency:

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_recipe_reagent_chain test_bulk_recipe_planning -v
```

Two tests passed in **0.981 seconds**. The new fixture requests one root output
requiring three units of an intermediate and one of a second intermediate. The
first producer yields at least two units per craft and consumes two raw units;
the second consumes three raw units. Starting with five raw units yields steps
`(102,2), (103,1), (101,1)`, two external raw units, one intermediate left over
and one conditional root output. An independent Python ledger replays every
step with observed plus required external stock, asserts sufficient inputs
before each modeled consumption, and matches every final projected quantity.

Existing dependency and material-search callers also passed three tests in
1.676 seconds (`python -m unittest test_recipe_dependencies test_recipe_material_search -v`).

One initially owned intermediate reduces the producer count and eliminates
external needs. A separate later compatible inventory capture has the same
effect; a different-character inventory rejects. Repeated producer expansion
reuses earlier leftovers. A target-output request derives the expected craft
counts and external shortage. Exactly eight chain steps succeed; nine block.

Negative fixtures cover ambiguous producers, cyclic uncovered demand, unknown
minimum yield, explicit child learning conflict, unobserved inventory, craft
count overflow of the 1,000 limit and unsupported materials. Blocked results
have no complete external list or final projection. A no-required-item recipe
can draft without inventory. Batch/merchant/dependency-order combinations and
invalid option types reject; omission preserves the old review shape.

Capture bytes remain unchanged and private selected-schematic names are absent.
Ownership, reported yield, optimisation, craftability, live and execution
verification flags remain false. The test uses the explicit binary override or
platform default; no Linux-native run was performed.

This qualifies bounded source arithmetic and guards, not native recipe
availability, real yields, current cooldowns, full ownership, acquisition
availability or best crafting/levelling routes. Live testing, installation and
deployment remain deferred; no CI or external data sync ran.

## Independent review and root-error documentation correction

Two independent `gpt-6-astra` reviewers at high effort covered Callers and
Failure/security. Callers found one documentation mismatch: the initial README
said conflicting learning always returns a blocked chain. It reproduced a root
recipe with explicit false selected learning returning the existing ordinary
tool error before expansion. The author confirmed the initial root-plan call
precedes chain construction, added a root-conflict test alongside the existing
blocked-child test, and ran the updated real-MCP test: one test passed in
0.384 seconds. Documentation now distinguishes whole-review root/request/context
errors from blocked recursive expansion; production code required no correction.

Jev triage: `jev-1.14.0`, request
`d9b03abb31ac63e712afe38680b0b5f81983f4e30f672ccb683d944550a66a93`,
591 input / 58 output tokens, 0.340 seconds; fix-docs 1.0. Advisory only.

Callers otherwise found no implementation regression. It ran the new chain,
legacy bulk and legacy dependency modules, and independently checked optional
Boolean/default-false schema, omitted/false response equality, bank-only split
stock, eight typed/mode guard rejections and older/realm/build/interface/family/
variant inventory rejections. Round two used Callers only for the documentation
and test correction, since no new product file or behaviour changed. The updated
test passed independently in 0.364 seconds, and the corrected README/ADR contract
was confirmed clean. Failure/security was not repeated for this prose correction.

Failure/security found no actionable defect. Its new scoped test passed in
0.422 seconds. A separate actual-Rust-MCP Python probe generated 200 DAGs using
seed `91753`, one to eight recipes, shared intermediate demands, varying yields
and initial stock. Every completed chain was replayed from observed initial
items plus reported external requirements, checking each consumption and the
final balances:

```text
PASS seed=91753 cases=200 complete=196 step_budget_blocked=4;
replay conservation, partial inventory labels, privacy, bounded steps
```

Targeted probes confirmed selected material/output conflicts and tracked false
learning block expansion; 32 producer candidates report only 20 IDs with a
truncation flag; covered self-consumption can succeed while uncovered recursive
demand blocks. Maximum-u32 reagent quantity times 1,000 crafts reports exactly
4,294,967,295,000 units, and a minimum output of 4,294,967,294 times 1,000 reports
4,294,967,294,000 units. A missing stack count correctly rejects at the existing
Rust schema boundary; the reviewer corrected its initial probe expectation.
Partial bank coverage stayed explicit, private identity/link/recipe marker
values were absent, and blocked outputs suppressed complete projections.
Reviewers did not edit source, build, mutate Git, access HTTP/game APIs, install
or deploy. Synthetic directories were cleaned; no SQLite was opened.

Final scoped Jev advisory: `jev-1.14.0`, request
`a8d432b442311203eb988cd720a3ba16ec4060ae262d51b3c9b104571f995e89`,
992 input / 54 output tokens, 0.333 seconds; land-scoped 0.99, fix-first 0.01,
insufficient-evidence 0.0, confidence 0.98. Advisory only, not proof or approval.
