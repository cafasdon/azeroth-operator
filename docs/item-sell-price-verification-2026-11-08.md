# Captured item sell-price references — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broad in-game coverage remains unfinished.

The public [Forever item API](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ItemDocumentation.lua)
defines `sellPrice` as the eleventh `C_Item.GetItemInfo(itemInfo)` return;
the API may return nothing. Manual Forever collection now optionally saves
that numeric per-item reference as `sell_price_copper` for linked equipment
and container items (including visible bank paths through the common helper).
Each query uses the exact full link and verifies the base ID through
`GetItemInfoInstant`. Two valid price readings must agree. Missing APIs/links,
nil results, restrictions, errors, changing values, invalid types/bounds or ID
mismatch omit only this field, preserving valid item and unrelated metadata.
No explicit load, merchant selection, item use or sale is introduced.

Both readers require a non-null unsigned integer from zero through
9,999,999,999,999 copper and a nonempty link. The generic inventory item
contract supports all storage sections/client families; new native collection
is Forever-only. Unknown is absent; zero is a literal reported price and
does not establish sellability, eligibility or realised proceeds. Unused native
returns and error text are never exported. New exports need updated readers.

`character_summary.captured_vendor_price_values` contains a value or null
for equipment, bags, character bank and account bank independently. Missing
sections are null. Equipment locations count one unit; container locations
use observed stack quantities. Each storage reports observed/priced/unpriced
location and unit counts, `known_price_subtotal_copper`, and
`observed_total_copper` only when every observed location has a price and
arithmetic succeeds. Empty observed sections total zero; a nonempty section
with no prices has a zero subtotal over priced locations but an unknown
observed total. Missing prices are never imputed as zero.

Multiplication and addition use checked u64 arithmetic. Overflow makes both
monetary results null and sets `arithmetic_overflow`, while retaining coverage
counts. Valid storage bounds keep unit-count sums below u64 maximum. Callers
must preserve integer precision when parsing large JSON values. No total is
claimed over unobserved inventory or combined across storage sections.
This is captured item-API price arithmetic, not a merchant transaction quote,
proof of complete economic value or funds available to spend. The explicit
`sale_eligibility_verified` flag is false. Wallets, purchase/repair budgets,
equipment legality and simulator inputs remain unchanged.

Verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test item_sell_price --test item_required_level --test native_item_stats --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_item_sell_price test_native_item_stats test_native_level_requirements
cargo fmt --check
git diff --check
```

Build passed; five Rust tests and three real MCP Python tests passed. Actual
Lua export/Rust import covers equipment and bag prices, zero/upper bound,
missing API/link/return, wrong/restricted IDs, private unused returns,
invalid/negative/fractional/nonfinite/oversized prices, changed values and
second-read errors/restrictions with item preservation. Real MCP cases cover
all storage readers, unit quantities, partial/fully unpriced/zero/empty/missing
coverage, exact near-u64 reference arithmetic, multiplication and sum overflow,
strict schema rejection/private diagnostics and an unchanged wallet. Native
item stats and required-level/ranker checks remain passing with local data.

Development package, not installed:
`local-data/item-sell-price-qualification/AzerothOperator-0.0.0-item-sell-price-test-forever-live.zip`.
SHA-256: `f46122c41efbd9b32308cd95afa0b174515fb704594f32417fa74ac22628103d`.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, and found no concrete defect. Both ran the real MCP test and prebuilt
Lua/Rust fixture. Callers independently compared priced/unpriced reviews and
direct simulator requests: price metadata survived as observation while other
review fields and requests stayed identical. Failure review rejected 12 extra
invalid prices and four malformed/oversized links, accepted zero/upper bound,
and checked maximum valid unit count (285,873,023,155,200) stays below u64.
Both checked the eleventh API return and separated reference arithmetic from
sale eligibility/proceeds. No review edits, compilation, cleanup actions,
deployment or game operations occurred. Native compatibility remains unproven.

Jev advisory supported scoped offline landing: `jev-1.14.0`, request
`a6fc92aa7a7dd545a79ad0d3b4c2b8321ce3db46553630aebe79c01c3bfdc809`,
928 input / 52 output tokens, 0.345 seconds. Probabilities were 0.96 landing,
0.03 needs fix and 0.01 insufficient evidence. Actual tests and independent
caller/source review, not advisory confidence, establish the scoped evidence.
