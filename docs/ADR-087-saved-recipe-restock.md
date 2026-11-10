# Recipe restock from saved bag stock

Date: 10 November 2026. Status: source/offline only.

Add read-only MCP tool `plan_forever_recipe_restock` with explicit owner-local
snapshot_id, positive uint32 target_bag_quantity and optional exact bulk
recipe_id. Omit the ID for the captured selected recipe. No catalogue is needed.
Use one validated Forever capture; do not select an active character, combine
characters or accept arbitrary file paths. Reuse existing exact recipe,
conflicting-observation, output and material review logic.
With an omitted recipe ID, validate a same-ID bulk schematic when its status
is fixed/unsupported, while retaining richer selected output metadata. Unknown
or budget-exhausted bulk entries contain no contrary schematic and do not erase
the usable selected observation.

Unlike target_output_quantity in recipe review, the target is final carried
base-item quantity, not newly produced units. Count the output item ID in saved
bags only. Known empty bags yield zero; unavailable bags reject. Existing
bank, equipped and mail output stock never reduces the target shortfall.
Base IDs do not establish item variant equivalence; this remains explicitly false.

Require fixed item inputs and a known positive minimum/maximum output yield.
Subtract any same-output-item reagent use from minimum yield before computing
the ceiling craft count. A positive shortfall with nonpositive minimum net
output rejects. Preserve the existing 1,000-craft review bound. Checked arithmetic
rejects overflow. Enough saved stock yields zero crafts and null recipe_review.

For positive crafts, return the existing exact material/condition review.
Its gross batch reagent needs do not credit intermediate output recycling.
Bank reagents can remain in that review; this does not count bank output as
existing carried stock or execute withdrawals. Conditional projected final bag
quantities assume all specified inputs are consumed and output enters bags.
This does not prove material sufficiency, learned status, current cooldowns,
native crafting eligibility, bag placement or successful completion.

Register this deliberate new read-only route in the source offline router and
standalone profile renderer. Existing route contracts and runtime installation
stay unchanged. No crafting, acquisition, sync, network, public indexing,
installation or deployment occurs. Native acceptance remains deferred.
