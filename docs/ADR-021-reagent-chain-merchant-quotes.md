# ADR 021: merchant quotes for external chain materials

Date: 9 November 2026. Status: source/offline implementation.

Allow `merchant_reagent_plan` with `suggest_reagent_chain` for one exact bulk
recipe. This supersedes ADR 019's initial prohibition of that combination.
Batch and dependency-order combinations remain disallowed in chain mode.

Only complete chain drafts have a complete external leaf demand. Those demands
already account for observed stock, prior conditional outputs and shared
consumption. Adapt each unique positive external quantity directly into the
existing merchant budget helper's shortfall input. Never subtract stock again,
price intermediate products separately, or round each producer's shared demand
independently. The resulting quote is nested at
`reagent_chain.merchant_reagent_plan`; no top-level standalone root-material
quote is generated in this mode. Ordinary recipe and batch quote behaviour is
unchanged.

Merchant offers default to the inventory capture or an explicit compatible
merchant capture. Existing same-character and exact-client/build validation
remains mandatory. Affordability always uses the inventory capture's wallet,
never a wallet from a separate merchant capture. Record both capture times.

Reuse existing bundle rounding, lowest recorded unlimited copper-only offer
selection and checked u64 arithmetic. Limited, extended-cost or overflowing
alternative offers can be shown but do not support the unlimited copper-only
total or invalidate another usable quote. A required leaf with no usable quote,
or overflow when summing selected line costs, leaves the aggregate unknown. A missing wallet
leaves affordability unknown. Blocked chain or absent merchant observations
produce a null quote; no prefix is priced as a complete chain. A complete draft
with no external demand has zero cost when the merchant observation exists.

These are historical quotes for conditional material models. They do not verify
current stock, availability, purchase or craft eligibility, actual yields or an
optimal crafting route, and do not execute purchases or crafting. Pricing may
help compare explicitly chosen producer alternatives; it never chooses one by
cost automatically.
