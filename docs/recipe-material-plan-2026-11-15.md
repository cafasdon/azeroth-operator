# Explicit recipe material plans — offline acceptance, 15 November 2026

## Behaviour

`recipe_material_plan`, the `recipe-material-plan` CLI action and
`plan_local_forever_recipe_materials` calculate conditional material arithmetic
from the existing first-party reference. The input selects a root recipe and
craft count. Optional choices explicitly map ingredient IDs to producer recipes.
Unselected ingredients remain unexpanded requirements, even when only one
producer is recorded. No captured inventory is subtracted.

The planner discovers the selected recipe DAG before it calculates quantities.
Each selected producer must have exactly one matching output with an observed
positive fixed yield. Unknown/variable yields, wrong or multiple outputs, cycles,
unused choices and one producer selected for multiple item IDs fail. The root
does not need an output yield because the request specifies crafts, not units.

A consumer-first topological pass combines every consumer's demand before it
rounds a producer's craft count. This handles shared intermediate items at
unequal depths. For example, two requirements of one item and a fixed yield of
two need one producer craft, rather than two separately rounded crafts. Steps
then appear in deterministic producer-before-consumer order, with required,
produced and excess quantities for selected intermediates.

Limits are 100,000 root crafts, 32 explicit choices, eight selected-producer
links, 64 recipes and 256 reagent edges. The choice limit also bounds the actual
reachable recipe count to 33. Products, sums and rounded outputs stop above
2^53−1. The existing 64 KiB response limit applies. Invalid plans return no partial
material plan. CLI JSON choices reject duplicate keys. Declared integer fields
reject floats, strings and booleans; the previously recorded FastMCP extra-field
behaviour remains tracked in issue #82.

`plan_generated` identifies successful arithmetic only. Coverage, ownership,
inventory subtraction, craftability, unlock and variant-equivalence claims stay
false. The existing manifest identifies the exact incomplete capture/build.
Unexpanded requirements are not shortages, base materials, unowned items or
acquisition instructions. No game action is generated.

## Offline evidence

Thirteen new tests, seven recipe dependency tests and twelve original local
reference tests passed. They used synthetic validated addon snapshots and the
real snapshot builder. Tests cover direct multiplication, explicit alternatives,
shared and chained demand, yield/excess arithmetic, unknown root yields, empty
recipes, cycles, wrong/unknown/variable/multiple outputs and invalid choices.

Exact eight-link and 256-edge limits passed; the next values failed. Maximum
choice count and unsigned item/recipe IDs passed. Product, accumulated-demand
and rounded-output overflow failed. A large public-field response hit the
actual response cap. SQL traces were reads, file bytes stayed unchanged and
private sentinel fields did not appear. Actual CLI and offline stdio SDK calls
matched direct results and rejected invalid declared arguments.

One review round used two Astra/high reviewers for callers/failures and
security/data/arithmetic. Independent models matched 60 and 120 seeded DAG plans,
including shared demand, longest depth, craft ordering and excess quantities.
Neither review found a new medium, high or low defect. No infrastructure drift
lens applied because no hosted service changed.

## Local registration and limits

The local `azeroth-local` enabled-tool list adds the material planner as its
fourth tool. No other configuration value changed. Explicit UTF-8 byte writes,
an ignored private backup, semantic comparison, exact readback and native Codex
parser checks protected staged and saved configuration files. Saved launch
parameters passed an SDK call with the database path replaced by a synthetic
fixture database. All four tools appeared and the plan matched the direct call.

No actual owner database, owner capture, game or hosted service was used in
acceptance. Live tests remain deferred. The arithmetic feature does not complete
the game's recipe database or establish a native crafting result.

An implementation test initially left its own SQLite handle open during scratch
cleanup. The corrected test closes the handle. Automatic approval review then
blocked an explicit cleanup command for `local-data/tmprg6jmam6`, with only
`blocked by policy`. No retry occurred. That synthetic scratch folder may remain;
it is not product state or a user-defined project policy.

## Advisory record

Jev `jev-1.14.0` selected proceed for the design: request
`b2dece91a9c51ae03c168a936e4d8f46412aaecf64f319c8e380be990a494d4a`,
610 input / 47 output tokens, 0.301 seconds. It selected ship after the checks:
request `6942c143c50f1644f0fab47a98808e3251c02121cc90e1755ab57026cfe8ff7f`,
723 input / 44 output tokens, 0.318 seconds. These are advisory records; the main
session accepted direct and independent evidence as the release basis.
