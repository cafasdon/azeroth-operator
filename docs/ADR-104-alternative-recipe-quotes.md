# Independent alternative recipe restock quotes

`plan_forever_recipe_restock.alternative_recipe_ids` compares one to seven
explicit alternatives with the primary recipe for the same saved base-item bag
target. It supports a concrete profession task: compare the additional copper
needed for different known fixed-input ways to restock an item. The option is
independent of the existing `merchant_quote` flag, which still describes only
the primary plan. Omitted/null alternatives retain the old behaviour.

The IDs must be distinct and must not repeat the resolved primary ID. This mode
excludes joint `additional_targets` and `stock_schedule`. It does not combine
recipes, allocate shared resources between methods or search a global recipe
catalogue. Each recipe independently sees the same original bags, merchant
offers and wallet. Banks, mail and output from other candidates receive no credit.

Each candidate reuses the existing saved restock checks: fixed reagents, known
output and yield, base-item quantities, minimum net yield, gross batch inputs
and at most 1,000 crafts. A known different output item rejects the comparison.
A recipe whose saved restock inputs cannot support a plan remains an explicit
`restock_inputs_unavailable` row. This preserves uncertainty without treating
that method as infeasible or eliminating a potentially cheaper method.

Known plans reuse the existing copper-only bundle-mix merchant quote for their
positive bag shortfalls. If no purchase is needed, the additional purchase cost
is zero even without a merchant observation. Unknown wallet amounts do not hide
known prices; affordability stays null. Costs do not value owned reagents, time,
skill gains, leftover bundle units, travel or non-copper currencies.

`lowest_known_quote_recipe_ids` and `lowest_known_purchase_copper` are provisional
over known quotes only. `lowest_requested_quote_recipe_ids` and its cost remain
null unless every requested method has a known quote. All minimum-cost ties are
retained in numeric recipe-ID order. This identifies the cheapest saved purchase
quote within the explicit set, not a current craft recommendation or economic
profit optimum. Unaffordable known quotes remain comparable and retain their
separate saved-wallet affordability flag.

The new argument masks malformed input values in schema errors. Bounds and
conflicting modes fail before store access; a selected-primary duplicate is
checked after exact resolution. The final response, including existing optional
views and snapshot metadata, is capped at 256 KiB when this mode is requested.
One owner-local capture supplies all evidence. No private data leaves the store,
no external data source is queried, and no game action or installation occurs.
Native output equivalence, craftability, current prices and complete recipe
coverage remain unverified.
