# Restock reagent quote estimates — 8 November 2026

Baseline `a40cfb5a2f7fab122db3f3656409a2acdf0b97e6`.
Source/offline synthetic verification only. No native capture, purchase,
crafting, installation, deployment or live endpoint testing.

## Contract

Catalogue consumable target lookups with restock recipe selectors can opt into
`restock_merchant_reagent_plan`. Optional `restock_merchant_snapshot_id` requires
that flag. The flag requires restock recipes using either supported selector
form; all existing target, context and recipe validation remains.

The route prices the final combined material shortfalls, using captured merchant
offers in the acquisition inventory or an explicitly selected owner-local offer
capture. Separate offers require the same character and exact client/build.
Their timestamp remains historical; no freshness or transaction eligibility is
inferred. The wallet always comes from the acquisition inventory, never from a
separate offer capture.

`crafting_plan.merchant_reagent_plan` retains the existing bundle rounding,
lowest unlimited copper-only offer, checked total, affordability and shortfall
fields. Missing merchant observations yield null. Missing matching quotes,
limited-only or extended-cost-only quotes leave the unlimited copper total
unknown. Missing wallet leaves affordability unknown. Covered materials create
no purchase demand. Nonnull new plans explicitly mark game execution and
purchase eligibility verification false. The optional offer snapshot ID is
retained beside the quote result. Omission leaves the old output unchanged.

Common material pricing is factored into `material_merchant_plan`. It accepts
already calculated material rows, without fabricating a recipe observation or
ordered plan. Existing individual and dependency-order recipe quote callers keep
their original material selection and checkpoint estimates.

## Author evidence

The author rebuilt the stdio executable from current source with the locked
offline build. With `PYTHONPATH=.;tests`:

`python -m unittest test_restock_reagent_quotes test_recipe_dependencies -q`

Two tests passed, 1.995 s. The actual-MCP quote test combines two restock recipes:
17 reagent units required against 10 shared stock, producing one shortfall of
7. Two bundles of 5 at 2 copper cost 4, unaffordable against the inventory's
3-copper wallet even when the offer capture holds 999 copper. A cheaper offer
changes the quote rather than the chosen wallet. Equivalent standalone batch
quotes produce the same reagent lines and total.

The test also covers default/explicit offers, missing or unmatched offers,
limited/extended-only offers, unknown wallet, already-covered materials, default
output omission, foreign character/realm/build, path traversal and invalid option
dependencies/types. Private markers remain absent and synthetic snapshot files
stay byte-for-byte unchanged. An initial empty-merchant fixture was rejected by
the existing nonempty merchant schema; it was corrected to a valid unmatched
offer, without relaxing validation.

Jev plan advisory: `jev-1.14.0`, request
`2e41277781725f065a7d8556f3ab3ec36df60d1a6738f2e541aa2a013fa817c4`,
518 input / 47 output tokens, 0.280 s; proceed probability 0.88. Advisory only.

Direct bulk-restock, bulk-planning and protocol regressions passed: three tests,
11.510 s, using
`python -m unittest test_bulk_consumable_restock test_bulk_recipe_planning test_mcp.MCPTests.test_rust_stdio_server -q`.
`cargo fmt --check` and `git diff --check` passed. No CI or pipeline was run.

Independent Callers review (`gpt-6-astra`, high effort) was clean. Command
`python -m unittest test_restock_reagent_quotes test_bulk_consumable_restock test_bulk_recipe_planning test_recipe_dependencies test_recipe_pending_mail -v`
passed five tests, 6.596 s. A separate actual stdio probe confirmed the runtime
optional-boolean schema, legacy capture-ID quoting, omission equal to explicit
false, additive-only quote output and a six-unit/two-bundle/four-copper example.
The reviewer traced individual, ordered and dependency-order quote callers;
first-uncovered checkpoint budgets remained intact. No builds or source/live
changes were made. The author's build provides separate source provenance.

Independent Failure/security review (`gpt-6-astra`, high effort) was also clean.
`python -m unittest test_restock_reagent_quotes -v` passed one test, 1.846 s.
Its independent stdio probe checked missing acquisition/targets/captures,
wrong section, empty/duplicate recipes, owner-path escape, malformed offer fields,
family/variant/interface mismatches, a zero-price/zero-wallet quote and an
unobserved bank. Rejections were sanitised. A 12,884,901,885-unit shortfall at
9,999,999,999,999 copper per bundle overflowed safely to unknown quote/total/
affordability. No source changes, build or native/live actions. One round, two
lenses; no confirmed finding required correction.

Post-verification Jev advisory: `jev-1.14.0`, request
`921d6dccd8297524af8ce261d697df7136bc86e73154f17096894b8877576f44`,
589 input / 49 output tokens, 0.295 s; land-scoped probability 0.97. Advisory only.
The broad feature objective and hosted/native qualification remain open.
