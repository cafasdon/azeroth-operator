# Merchant buyback observations — 8 November 2026

Forever manual capture adds `merchant_buyback` with coverage
`visible_merchant_buyback` and compact `entries` numbered 1–12. Each row requires
positive u32 `item_id` and `count`. Optional `buyback_price_copper` is a nonnull
integer from zero through 9,999,999,999,999. Missing price remains unknown.

## Capture source and boundaries

The [primary MerchantFrame API definition](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/MerchantFrameDocumentation.lua)
provides `C_MerchantFrame.GetBuybackItemID`. The [native merchant UI source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_UIPanels_Game/Mainline/MerchantFrame.lua)
reads price and quantity as the third/fourth `GetBuybackItemInfo` returns, uses
`GetNumBuybackItems`, and displays the price directly for the listed entry.
The exporter requires public visibility and selected tab 2, checks count before/
after each full read and compares two complete ID/quantity sequences. Maximum
12 entries. Changed context, count, IDs or quantities, missing required APIs,
restricted/invalid data and exceptions make the section unavailable.

Public valid prices survive only if both reads agree. Unreadable/invalid/changing
prices are omitted independently; zero is preserved. An observed empty list is
retained even if unused item-detail APIs are unavailable. Names, textures, links,
merchant identity, usability and binding flags are discarded. No tab is selected,
purchase/buyback/sale API called, or gameplay action issued. Manual capture stays
combat-gated. Other valid equipment observations survive buyback failure.

## Reading and accounting

Rust and Python enforce the exact Forever payload and compact bounded entries.
`inspect_snapshot` returns the section; `character_summary` adds
`merchant_buyback_observations`, or null when missing. The summary reports entry
and unit counts, distinct numeric base item IDs, priced/unknown counts, known
subtotal, and a total only when every entry quote is known. Empty observed lists
have zero totals. Each entry quote is added once, never multiplied by quantity.
Validated maximum subtotal is 119,999,999,999,988, within u64.

`included_in_owned_inventory: false` describes this list's exclusion, not absence
of other owned copies of the same base item. `buyback_eligibility_verified` stays
false. Existing item searches, owned-gear pools, materials, restock and budgets
continue to enumerate only their existing equipment/bag/bank stores. Quotes do
not become wallet funds. No buyback item enters simulator equipment candidates.

These are historical base-ID observations, not full link/modifier identity,
merchant continuity, current stock, affordability, space/equip eligibility,
complete sale history, recovery guarantees or transaction receipts.

## Offline verification

- Current `cargo build --locked --offline` passed.
- `cargo test --locked --offline --test merchant_buyback --test addon`: three
  Rust tests passed. Lua/Core fixtures cover stable/changed/secret/invalid
  IDs/counts/quantities/prices, exceptions, wrong/hidden/closed/changing tabs,
  missing APIs, empty lists, combat refusal, preserved equipment and forbidden
  buyback/purchase/sale stubs. An explicit three-unit entry with quote 10 totals
  10, not 30.
- `PYTHONPATH=.;tests python -m unittest test_merchant_buyback`: one real MCP
  test passed in 0.952 seconds. Coverage includes summaries, raw inspection,
  missing/empty/partial/zero quotes, max 12/u32/price values, strict reader/family
  rejection and private-marker suppression. Buyback-only items contribute zero
  owned quantities/candidates; the same base ID in bags plus buyback counts the
  single bag copy only. The pinned catalogue was present, so owned-gear checks ran.

Independent Callers and Failure/security reviewers (`gpt-6-astra`, high effort)
found no actionable defect. Both executed the prebuilt fixture and real MCP
test, checked primary signatures/quote arithmetic and traced inventory/budget/
simulator exclusion. Both rechecked the final explicit exclusion flag and
same-ID owned-copy test against the rebuilt server. Review's prebuilt fixtures
and synthetic secret sentinels do not qualify native game behaviour.

Development package, not installed:
`local-data/merchant-buyback-qualification/AzerothOperator-0.0.0-merchant-buyback-test-forever-live.zip`.
SHA-256 `15d48be0d20e847c3696243da13219b3afb55458898723a4007e04da2e0fc1cd`.
No native capture, installation, deployment or game operation was performed.

Jev plan advisory: `jev-1.14.0`, request
`5d1ee85eb28baa67a6fe4212c79ca4c23d30bf7d7bcaff8501c3c683ac88ba00`,
571 input / 47 output tokens, 0.332 seconds; proceed probability 0.92. Advisory
confidence does not verify native recovery or item eligibility.

Post-verification advisory: `jev-1.14.0`, request
`f50d3393d3529197d4dd68c84760f49258d4865b8773935d19faa584c60bff95`,
593 input / 50 output tokens, 0.307 seconds; land-scoped probability 0.97.
Scoped tests and independent source/caller checks establish the actual evidence.
