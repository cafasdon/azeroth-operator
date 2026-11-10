# Saved source references for recipe restock

Date: 10 November 2026. Status: source/offline only.

`plan_forever_recipe_restock.captured_sources` is an optional boolean, default
false. It joins positive bag shortfalls to `recipe_item_sources` within the
same validated owner-local snapshot. There is no external query, provider
refresh, new dataset, purchase, movement or game action.

Joint targets use combined ordered starting-material shortages, so shared
inputs and requested producer output are counted once. Bank stock is not
credited. Single targets retain conservative whole-recipe gross inputs. A
zero-craft target has no material demand. Cyclic/unavailable whole-recipe order
returns `material_order_unavailable` with a null source review, even if the
separate individual-craft search finds a witness. Per-craft recycling output is
not credited to this source review.

The adapter uses the existing inventory shortfall source resolver. Records
retain provider provenance, exact reference IDs, unknown and truncated category
lists, and scoped provider-area locations where captured. An explicit empty
reference category differs from an unrecorded category. Missing source section,
item not captured, unavailable and not-in-provider remain distinct. Relevant
unretained entity locations remain `not_captured`; no coordinate system is
relabelled as a native map or travel route. References do not prove a drop rate,
vendor price, current availability, acquisition eligibility or complete world
coverage. No bank item, pending reward or reference counts as owned inventory.

The option composes with `merchant_quote` and `stock_schedule`, while each keeps
its own arithmetic basis. Omission/false preserves prior output. It adds no new
MCP route or cross-snapshot selection path. The selected snapshot ID binds the
parent report; the nested source capture timestamp and provider metadata bind
the reference observation. Live tests and deployment remain deferred.

Direct validation: `cargo build --offline` and
`PYTHONPATH=.;tests python -B -m unittest test_recipe_restock_sources_mcp`.
