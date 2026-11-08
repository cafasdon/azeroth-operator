# Saved buyback cost review — 8 November 2026

`inspect_snapshot` adds optional `buyback_slots` and
`buyback_wallet_reserve_copper`, requiring `section: merchant_buyback`. Slots
must be 1–12 distinct integers in 1–12. Reserve defaults to zero, is bounded by
the existing copper limit, and cannot be supplied without slots. Other section
selectors retain their own constraints. Unrequested inspection output is unchanged.

Example:

```json
{"snapshot_id":"<saved snapshot ID>","section":"merchant_buyback","buyback_slots":[1,2],"buyback_wallet_reserve_copper":1000}
```

The response adds `buyback_cost_review`. Both the quotes and optional wallet
come from the same validated owner-local Forever capture. Each selected whole
entry quote is counted once, regardless of its quantity. Selection order is
retained as captured references; duplicate slots are rejected. Lines report
whether the saved slot was observed and its saved item data, or null.

The review returns missing slots, slots with unknown prices, known subtotal,
nullable selected total, captured wallet, reserve, spendable balance after reserve,
reserve shortfall and known/total shortfalls versus that spendable balance.
The total is unknown if any selected slot or price is unobserved.

`affordable_from_captured_wallet` is false when known costs exceed the spendable
balance or the wallet cannot cover the reserve, including with other unknown
quotes. It is true only when every quote is known and the captured wallet covers
the quote and reserve. Otherwise it is unknown. An unavailable wallet remains
unknown even for a zero quote; a captured zero wallet can cover a known zero
quote with zero reserve. A reserve shortfall still prevents a positive verdict.
Missing buyback section makes the review null, while an observed empty list
produces explicit missing selected slots.

This is historical arithmetic, not a transaction or game action. Eligibility and
action-performed flags remain false. No capacity, equip legality, merchant/item
continuity, current price, recoverability or ownership change is inferred. Buyback
indices can change after a purchase, so these captured indices must not be treated
as an execution sequence. No item is purchased, restored, sold or fed to simulation.

## Offline evidence

`cargo build --locked --offline` passed. The new real stdio MCP test and existing
buyback observation test passed:

`PYTHONPATH=.;tests python -m unittest test_buyback_cost_review test_merchant_buyback`

The new test covers exact quote totals rather than quantity multiplication,
wallet/reserve equality and deficits, partial-price lower bounds, zero/unknown
wallets, zero prices, missing slots/sections, observed empty lists, maximum totals,
bad types/bounds/duplicates/sections/family, unchanged default output and byte-for-
byte unchanged store contents. Formatting and diff checks passed.

The existing `test_mcp.MCPTests.test_rust_stdio_server` protocol regression also
passed (6.368 seconds), for three distinct targeted Python/MCP tests overall.

Independent Callers and Failure/security reviewers (`gpt-6-astra`, high effort)
found no concrete defect in one round. Both executed the new real MCP test.
Callers independently checked published schema, omitted reserve, nonsequential
captured order, preserved raw data, incompatible selectors and malformed types.
Failure/security passed 168 independent affordability oracle cases and rejected
21 malformed argument/context/owner-scope cases, including traversal and missing
IDs. Both traced same-capture, hash-validated reads and no action path. Review used
the existing executable; author compilation is separate current-source evidence.

No addon collector, native API, game action, installation or deployment changed.
No new addon package was needed for this analysis-only feature.

Jev plan advisory: `jev-1.14.0`, request
`f332a5a1812e26691f2473c41b652075d35e391fee34493bc3818a56971dc8cc`,
523 input / 47 output tokens, 0.292 seconds; proceed probability 0.98. Advisory
confidence does not verify current game eligibility or successful transactions.

Post-verification advisory: `jev-1.14.0`, request
`4d64e06446a3759d2dbc78545cc9a2c345f2d8510b4aa5c7c922d410f4cf609a`,
577 input / 50 output tokens, 0.317 seconds; land-scoped probability 0.98.
Tests and independent source/caller checks supply the actual evidence.
