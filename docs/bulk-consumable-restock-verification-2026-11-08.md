# Bulk consumable restocking — 8 November 2026

Baseline `b4ed56a732b784191ad86408297b68e08707e089`.
Source/offline verification only. No native capture, external data sync,
installation, deployment or game action.

## Contract

`lookup_forever_catalogue` adds optional `restock_recipes`, a list of one to
eight strict objects containing `snapshot_id` and optional positive u32
`recipe_id`. It requires existing consumable targets and is mutually exclusive
with `restock_recipe_snapshot_ids`. The legacy list maps to selectors without
an ID and retains its previous output meaning.

An explicit ID selects only the exact bulk row, requiring fixed item materials
and a known output item. It never falls back to the selected schematic. The
existing planner applies bulk learning/observation conflict checks, minimum-yield
output rounding, 1–1,000 craft bound and exact character/client/build/chronology
inventory checks. Selectors without ID retain selected-schematic planning.

Each output must have a distinct positive confirmed shortage in the same saved
inventory. Multiple recipe IDs can come from one capture; selected and bulk
captures may mix. An output cannot consume the same shortage twice. Material
demands combine against the one shared inventory rather than each recipe's old
stock. The existing prohibition on a restock target also being a reagent remains.
Unknown shortages, missing outputs, unsupported/unavailable/budget bulk rows,
invalid selectors and conflicting or incompatible captures reject safely.

Bulk plan rows add source and native profession provenance. Missing cooldowns,
conditions and charges remain unknown. Legacy rows retain their existing shape
and selected observation scope. The result remains conditional, with
`craftability_verified: false`; no material or game state is changed. The
existing owner-store/hash/schema boundary and fixed safe route errors remain.

## Author evidence

The author rebuilt the stdio executable from current source using
`cargo build --locked --offline`. With `PYTHONPATH=.;tests`:

`python -m unittest test_bulk_consumable_restock -q`

One actual-MCP integration test passed, 2.261 s. It plans two different recipes
from one bulk capture against a newer shared inventory: shortages 4 and 5 yield
4 and 3 crafts respectively, requiring 17 shared reagent units against 10
observed units, a shortfall of 7. Old recipe inventory contains 999 units and
does not substitute for the chosen inventory.

The test also checks exact legacy/new selected-selector result equality, mixed
selected/bulk plans, unknown bulk preparation fields, invalid/duplicate/oversize
selectors, mutually exclusive lists, missing targets, unknown bank coverage,
already-covered targets, wrong output, false learning, wrong character/build,
future recipe capture and target-as-reagent rejection. Private selected/tracked/
foreign markers remain absent, and synthetic owner-store files remain unchanged.
The existing pinned local catalogue is used without automatic downloading.

Jev plan advisory: `jev-1.14.0`, request
`63ce51072ee4198afc2060d6ecf39daddd2a33f4a009afdd09fd430b79471d57`,
537 input / 47 output tokens, 0.286 s; proceed probability 0.93. Advisory only.

Direct existing bulk-planning, consumable-stock/aura and protocol regressions
passed: three tests, 9.973 s, with
`python -m unittest test_bulk_recipe_planning test_consumable_aura_matches test_mcp.MCPTests.test_rust_stdio_server -q`.
`cargo fmt --check` and `git diff --check` passed. No CI or pipeline was run.

Independent Callers review (`gpt-6-astra`, high effort) traced lookup validation,
consumable inventory, exact recipe selection and aggregation; no finding.
`python -m unittest -v test_bulk_consumable_restock test_bulk_recipe_planning`
passed two tests, 3.703 s. A separate stdio `list_tools` probe confirmed both new
and legacy selector fields in runtime schema. The first schema-probe setup lacked
the required store/catalogue environment; corrected setup passed. No source
changes, builds or live actions were made by the reviewer; the author build is
separate source provenance.

Independent Failure/security review (`gpt-6-astra`, high effort) was also clean.
`python -m unittest test_bulk_consumable_restock -v` passed one test, 2.114 s.
An independent stdio script passed 34 cases covering yield rounding and the
1,000-craft boundary, malformed quantities, character/realm/variant/interface
mismatches, selected/bulk/tracked/list conflicts, missing acquisition, wrong
sections, both selector forms, unknown captures, duplicate mixed outputs and
self-reagent rejection. Private markers remained absent and snapshot bytes
unchanged. One review round, two lenses; no confirmed finding required a fix.

Post-verification Jev advisory: `jev-1.14.0`, request
`d1079760524e4eb4c3c157ce0710d0388e3f67b43253fd7e7a4c50d55be13fb6`,
584 input / 49 output tokens, 0.310 s; land-scoped probability 0.58, fix-first
0.41, insufficient-evidence 0.01. This advisory was uncertain and supplied no
concrete failure scenario. The author judged the bounded offline change against
the actual source, direct tests and two independent clean reviews; it was not
re-evaluated until a more favourable probability appeared. Advisory confidence
is not verification or native qualification. The broad goal remains open.
