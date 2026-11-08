# Recipe dependency ordering — 8 November 2026

`review_forever_recipe` accepts `suggest_dependency_order: true` with at least
one additional recipe. Existing bounds permit at most eight distinct recipes
from compatible captures using one shared inventory observation.

Known minimum item outputs form producer-to-consumer edges against fixed
reagents. A stable topological sort suggests an order; independent nodes retain
input priority. Missing outputs/requirements suppress suggestions. Cycles also
suppress them, without claiming that crafting is impossible from existing stock.

The original plan remains unchanged. The suggested order reuses the material
and preparation estimators. Its checkpoints use original submitted recipe
numbers plus `suggested_step`. This is conditional on finishing every requested
craft at its captured minimum yield, not an optimiser or craftability assertion.

Verification:

- `cargo build --locked --offline` passed.
- `PYTHONPATH=.;tests python -m unittest test_mcp test_snapshots test_recipe_cooldown test_recipe_dependencies`
  passed 13 tests, including the real Rust stdio server.
- A three-recipe chain submitted backwards produced `[3, 2, 1]`, requiring only
  its base reagent externally. A producer with insufficient output retained
  the intermediate shortfall and mapped the blocked consumer to original
  recipe 1, suggested step 2.
- Cyclic and missing-output fixtures returned no suggestion. Disabled optional
  output and single-recipe rejection were verified.
- Formatting and diff checks passed. No native game operations were performed.

The real-MCP checks also preserve independent input order and remap an unlearned
producer's preparation failure to its original recipe number and suggested step.
Jev 1.14.0 scope advice returned supported, separately from test evidence:
request `347ed91ceba5b09e7395683f6140d51e48efd17e9241bc8342d5d9a8165d8ab3`,
509 input / 46 output tokens, 0.295 seconds.

Independent caller/failure review (gpt-6.1-sol, high effort) found no confirmed
defect. Additional real-stdio probes covered multiple partial-yield producers,
self-dependency, downstream cycle blocking, unsupported required slots, missing
output quantities, schema advertisement, eight/nine recipe limits and rejected
character/build mismatches. Disabled output stayed unchanged.
