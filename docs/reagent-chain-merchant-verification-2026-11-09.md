# Saved merchant quotes for reagent chains — 9 November 2026

Base source: `84e0865`. The existing `merchant_reagent_plan` option now composes
with an exact recipe's reagent-chain draft. A complete chain's combined external
shortfalls feed the existing merchant budget helper directly. No second stock
subtraction, separate intermediate acquisition cost or new market-data source
is introduced. The quote is nested in the chain; ordinary quote paths remain
unchanged. ADR 021 supersedes the earlier prohibition of this combination.

## Author evidence

```text
cargo build --locked --offline
Finished dev profile in 10.29s
```

After correcting the argument comment to advertise the new composition, an
incremental build passed in 5.33 seconds. Formatting was applied with `cargo fmt`.

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_reagent_chain_merchant test_recipe_reagent_chain test_recipe_producer_choices test_bulk_recipe_planning test_recipe_dependencies -v
```

Five tests passed in **2.001 seconds** through the actual local Rust stdio MCP,
with synthetic owner captures and no catalogue/network lookup. The new test
derives seven total raw units for two intermediate producers, allocates five
observed units once, and quotes only the remaining two. A three-unit bundle
costing seven copper gives one bundle, one extra unit and seven copper total.
It does not quote intermediate items or report zero by subtracting stock again.
The root's standalone quote would be 400 copper; that alternative remains
unchanged in non-chain mode and is absent from chain mode.

Other assertions cover:

- Wallet six versus cost seven yields a one-copper shortfall. A separate offer
  capture with wallet 99,999 does not change affordability. A later explicit
  inventory with wallet one determines a six-copper shortfall; both timestamps
  identify their source.
- Sufficient observed material quantities yield no external demand and zero quote.
  Missing wallet leaves affordability/shortfall null; missing merchant section
  and blocked chain leave the entire quote null.
- Limited, extended-cost, unquoted and multiplication-overflow cases leave
  aggregate copper cost and affordability null.
- Explicit producer alternatives quote their different external needs at seven
  versus six copper. No producer is selected by price automatically.
- Wrong merchant character rejects even for a blocked chain; an explicit
  merchant capture still requires the merchant-plan flag. Omitting quoting
  adds no nested quote; ordinary review retains its original standalone quote.
- Snapshot bytes are unchanged, private markers absent, and purchase eligibility,
  craftability, live and execution verification remain false.

The old test's expected rejection of the newly supported merchant combination
was replaced by this positive integration coverage. Batch and dependency-order
guards, ordinary recipe quote tests and producer-choice guards still run.

## Independent review

Two independent `gpt-6-astra` reviewers at high effort covered Callers and
Failure/security. Neither reported a code defect. Callers ran the five scoped
modules above in 2.017 seconds and independently checked schema switches,
shared-leaf aggregation, target-output requests, exact build/interface/variant
rejection and incompatible batch/dependency/root/choice combinations.

Failure/security passed the new test in 0.347 seconds and the chain test in
0.400 seconds. Independent actual-MCP probes returned:

```text
shared_rounding: leaves [(10,4),(11,1)], total 16, affordable true
partial_unknown: total null, affordable null
sum_overflow: individual lines 18446744065119617025 each; total null
wallet_only_other_capture: total 16, affordable null
wrong_build: rejected, private marker absent
```

Four shared units round to two three-unit bundles costing fourteen, plus a
second leaf costing two. An unquoted second leaf invalidates the full aggregate.
Two individually valid u64 lines do not wrap when summed. Inventory wallet
absence is not filled from the merchant capture. The reviewer also traced the
blocked-chain and zero-second-subtraction paths and false nested action flags.
No reviewer edits, builds, Git mutations, HTTP/native game actions, installation
or deployment occurred; synthetic captures were cleaned and no SQLite opened.

Callers noted the old ADR/README merchant prohibition before documentation was
updated. The author confirmed the old text contradicted the new supported
branch; ADR 021 and current README/ADR 019 now describe the new composition,
and original verification is explicitly versioned as historical evidence.

Jev documentation triage: `jev-1.14.0`, request
`532b509862de96c2b65134f5743a3d05995a8773e3078129370a8f5903c42407`,
535 input / 47 output tokens, 0.278 seconds; update-docs 1.0. Advisory only.

Documentation follow-up found one additional overbroad unknown condition:
unusable alternatives do not invalidate a usable quote. Callers reproduced a
seven-copper unlimited quote surviving cheaper limited and extended offers.
The author added mixed-alternative and overflowing-alternative fixtures; the
updated real-MCP test passed in 0.367 seconds. The first still quotes seven;
the second ignores an overflowed line and selects a valid one-copper-per-unit
line, totalling 4,294,967,294,995 copper. Current README/ADR 021 now restrict
unknown aggregates to a required leaf without a usable quote or overflow in
the selected-line sum. Production code required no change.

Jev wording triage: `jev-1.14.0`, request
`9c6a451ef2f87f43e99062f839df3e2dfd9d06c2b04a5d06b745c7b299fe00e8`,
550 input / 50 output tokens, 0.287 seconds; fix-docs 1.0. Advisory only.

The same Callers reviewer confirmed the final README/ADR 021 contract and
reran the updated test: one test passed in 0.352 seconds. No findings remained.
Follow-up review used Callers only because the corrections changed prose and
test coverage, with no new product caller or logic; Failure/security's clean
source review was not repeated.

Final scoped Jev advisory: `jev-1.14.0`, request
`de23eafec86b98903eedf82d45007032c40ff2c1e375ed3b8d2fad97ab57b39e`,
970 input / 54 output tokens, 0.266 seconds; land-scoped 0.98, fix-first 0.02,
insufficient-evidence 0.0, confidence 0.97. Advisory only, not proof or approval.

This source increment does not verify current merchant availability, purchases,
actual yields, full ownership, craftability or the cheapest overall crafting
route. Live testing, installation and deployment remain deferred. No data sync
or CI ran.
