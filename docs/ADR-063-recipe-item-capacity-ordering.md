# ADR-063: Recipe search by independent observed item capacity

Date: 2026-11-09. Status: accepted for offline source integration.

## Decision

Add optional search_forever_recipes.order with recipe_id (default) or
observed_item_capacity_desc. Root null acts as omission. Malformed types and
unknown enum labels reject with fixed argument errors without input echo.
The capacity mode requires explicit material_filter; its existing positive
minimum_crafts and bags/all_observed_storage scope retain their meaning.

Existing material, view and cooldown predicates select matches first. Sort all
matches before pagination by finite material_support.supported_crafts descending
and numeric recipe ID ascending. Equal capacities have deterministic ties.
No-required-item recipes have null finite capacity and follow all finite values,
also ordered by ID. This avoids fabricating infinity, zero or a current limit.
Unknown material support and insufficient observed stock retain existing filter
exclusions/counters. Missing and known-empty sections remain distinct.

The capacity is the existing minimum floor(observed quantity/required quantity)
over fixed item requirements, independently for each recipe. It ignores charges,
shared cooldowns, tools, stations, learning, skill-up probability and output yield.
It is not a combined craft limit or best profession progression. Empty fixed
requirements are explicitly no_required_items, not unknown material data.

Capacity-mode output reports order, independent_observed_required_item_capacity_only
basis, shared_material_allocation_performed:false and
skill_gain_prediction_performed:false. Existing craftability/live/game flags
remain false. Omitted/null/recipe_id requests keep prior response shape. The
old public Rust search_recipe_materials_with_cooldowns signature remains as a
default-order delegate; the new ordered entry point owns implementation.

## Provenance and boundaries

Explicit compatible newer inventory can change capacities/order but does not
refresh recipe-view learning/skill-up flags or original cooldown state. Known
missing storage remains in coverage; no older stock is backfilled. Each recipe
compares the same stock independently, so adding capacities across recipes is
invalid and can double-count shared items. This is a stock-based shortlist over
observed fixed recipes, not a skill-gain optimiser, full knowledge or native
craftability proof. No new collector, schema, addon, simulation, tool route or
game action. [Evidence](recipe-capacity-order-verification-2026-11-09.md).
