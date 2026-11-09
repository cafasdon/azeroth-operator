# Recipe observed item-capacity ordering — 2026-11-09

Baseline5ff4b83. Previous goal turn made progress adding dialogue discovery.
This unit improves task-level profession/item planning with an independent stock
shortlist, without substituting it for complete progression or owned-gear BiS.
Broad objective remains active and incomplete. No live client/account data,
installation, deployment or external sync used.

## Direct author checks

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
python -m unittest test_recipe_capacity_order test_recipe_search_inventory test_recipe_cooldown_search
```

Ordinary build25.43s. Three actual stdio MCP tests passed,2.261s. The new golden
fixture has capacities10/5/3/3/null with numeric IDs deliberately out of capacity
order. Page1 limit1 yields the global highest capacity rather than sorting the
first ID page. Numeric tie order, counts and next offsets are asserted.

Newer explicit inventory reverses ranks to30/30/2/1/null while original cooldown
seconds100 and source timestamps stay intact. A charge1 recipe still shows
item capacity5, proving item-only semantics rather than silently combining
nonmaterial limits. All-observed storage adds bank100 to bag3 for capacity103;
missing bags yield only the no-required-item row, with null finite capacity.
Missing/empty recipe sections, learned/skill-up and cooldown composition,
default/null/explicit recipe_id shape equality, missing required material filter,
wrong family, foreign-owner inventory, malformed private enum values and
unchanged stored bytes are covered. Existing stock-refresh and cooldown search
tests remain green. The fixed top-level rejection text was then clarified to
name the required material_filter for capacity order; ordinary rebuild8.27s.
Final source is exercised by the additional checks below. No CI/full-suite run.

## Advisory plan

Jev resolved jev-1.14.0, request
f8922c11d351f8e9095749e06ce52a56e34ead6c6e0eea77cf4a09d5a37366c9:
proceed.87/revise.12/insufficient.01, confidence.80;585input/47output,.337s.
Advisory only; source and actual caller evidence determine integration.

## Final caller checks and independent reviews

On the final rebuilt source, directly ran:

```powershell
python -m unittest test_recipe_capacity_order test_recipe_view_search
```

Two tests passed,1.527s, including the clarified rejection text's caller path and
existing view/material/profession projection behaviour. No implementation fault
or red/green source correction was needed in this unit.

Two independent read-only reviewers assigned gpt-6-astra high (resolved runtime
IDs/usage unavailable), Callers and Failure/privacy, ran the new actual-MCP test,
1passed each (.403s/.324s). Callers independently checked74 recipes across50-row
pages against an exact capacity oracle, including8589934590 (above u32), numeric
ID ties and null last. Maximum offsets, advertised schema, legacy/default/null
exact output parity and fixed malformed enum errors passed. It traced legacy
public wrappers to explicit RecipeId defaults.

Failure/privacy independently made50 actual MCP calls over72 synthetic recipes
and six scope/minimum combinations, with7-row pages, multiple-stack u64 sums,
max-u32 recipe-ID ties, null/no-item rows, maximum offsets, five incompatible
same-store inventory joins and private malformed values. Zero-charge recipes
remain sortable strictly by item capacity, not actual craftability. Bytes stayed
unchanged. Both bounded documentation readbacks were clean, asking only for the
final receipts now appended here. No reviewer builds/source writes/Git changes/
network/game/deployment. Live drift omitted because installation/native testing
remain deferred. Final formatting/whitespace checks passed.
Ordinary MCP SHA-256:
2b48525a7bc9ae174a5687dada5224e866d706db8061b7e66263dd6d8a3c41da.

Final advisory Jev resolved jev-1.14.0, request
ffbf31c9457d9bfb0ede0702cd693396398acc02e18e66f3882ceecd3926a404:
integrate.90/revise.08/insufficient.02, confidence.85;581input/52output,.292s.
Author integrates the bounded independent-capacity ordering on actual-caller and
independent-oracle evidence. Broad objective remains active and incomplete.
