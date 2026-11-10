# Bulk recipe material observation comparison

Date: 10 November 2026. Status: source/offline only.

`compare_snapshots.recipe_material_changes` accepts an optional object with
`offset` (uint32, default 0) and `limit` (1..50, default 20). Omission or null
preserves the previous response. Existing owner-local snapshot validation,
same-character/exact-client and chronological guards run before this report.
This option is Forever-only. Invalid objects return the existing generic
comparison error; object deserialisation errors do not echo caller text.

The report compares `profession_recipe_materials` only when both sections exist
and their profession IDs agree. Missing sections and different professions yield
explicit statuses with unknown counts and rows, not empty successful results.
The section coverage and observed row counts accompany each side.

For the numeric union of recipe IDs, compare only status, fixed reagent map,
output base item ID and output quantity range. Optional missing fields become
null; unsupported/unavailable/budget-exhausted statuses remain observations.
Cooldowns and reported craftable counts do not change this report. Existing
cooldown and profession-view comparisons retain their separate meanings.

Counts cover the complete saved union, independently of the page: unchanged
shared rows, changed shared rows, observed only before, observed only after.
Rows include the changed fields and before/after projections. Numeric ordering
and offset/next_offset allow every retained difference to be retrieved, including
past the old cooldown report's 50-row display bound. An offset past the end
returns an empty page without changing full counts.

No omitted row establishes learned/unlearned state, and no observation change
proves a game recipe definition changed. This does not establish complete
profession coverage, native craftability, current stock or live behaviour.
Only numeric recipe information and fixed status codes enter the new report;
raw unavailable reasons and private paths do not. There are no game actions,
new data sources, installation or deployment changes.

Direct validation uses `cargo build --offline` and the actual executable via
`PYTHONPATH=.;tests python -B -m unittest test_recipe_material_compare_mcp`.
