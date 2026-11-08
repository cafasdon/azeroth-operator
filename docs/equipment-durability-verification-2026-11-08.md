# Exact equipped durability — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broad in-game coverage remains unfinished.

The public [Forever character UI](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_UIPanels_Game/Camelot/PaperDollFrame.lua)
reads current and maximum values from `GetInventoryItemDurability(slot)` and
compares them to identify damaged equipment. The new manual Forever export
section `equipment_durability` uses that public read for slots 1–19; no UI
opening, selection, repair or other game action is introduced.

Section shape is `coverage: readable_equipped_slots_only` with an `entries`
object keyed by canonical slot strings. Each observed slot has required
`current` and `maximum` unsigned u32 integers, maximum positive and current
at most maximum. Two valid readable pair observations must agree. Missing,
partial, restricted, throwing, invalid or changing readings omit that slot
while preserving other readable slots and unrelated sections. Missing API
makes the section unavailable. Restricted values are screened before numeric
comparison and error text is not exported. Existing low/broken alerts and
merchant repair quotes are unchanged.

Both readers enforce the exact root/row fields, coverage, slot range, bounds
and Forever-only availability. Empty observed entries are valid; missing
sections remain distinct. Update readers before syncing new exports.
Unobserved slots may be empty, non-durable,
restricted, unavailable or changing. No absent value is treated as full
durability, zero durability or proof of an empty equipment slot.

`character_summary.equipment_condition` reports observed/unobserved counts,
up to 19 numeric-slot-ordered pairs and percentages, damaged slots
(`current < maximum`), broken slots (`current == 0`) and the minimum observed
percentage. Missing sections return null; empty observations have a null
minimum and 19 unobserved slots. Percentages are floating-point descriptive
values, not a threshold classifier. No base-item/GUID association or continuity
is asserted, including across other sections or captures. Conditions describe
saved slot observations, not present game state. Numeric durability does not
establish repair availability, copper cost or stat/weapon effectiveness.

Verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test equipment_durability --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_equipment_durability test_recipe_skill_up
cargo fmt --check
git diff --check
```

Build passed; three Rust tests and two real MCP Python tests passed. Actual
Lua export/Rust import covers full/partial/broken pairs, nil/absent API,
restrictions, errors, second-read errors/restrictions, changed current or
maximum, nonfinite/fractional/overflow/invalid bounds and isolation from
equipment observations. Real MCP tests exercise inspection, numeric ordering,
fractional percentages, empty/missing sections, all 19 slots at u32 maximum,
strict schema/family rejection and diagnostics without private text. Existing
addon round trips and the preceding skill-up summary remain passing.

Development package, not installed:
`local-data/equipment-durability-qualification/AzerothOperator-0.0.0-equipment-durability-test-forever-live.zip`.
SHA-256: `cec0d718ae3741748d2a963fe1ae1fc953fd13a2e282908d58ba389523f52d26`.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, and found no concrete defect. Both executed the prebuilt Lua/Rust
fixture and real MCP test. Callers also ran the existing addon alert/repair
fixture and rejected nine noncanonical slot keys plus six invalid values.
Failure review rejected eight extra malformed cases and checked five real
MCP summaries for finite percentages, 19-slot coverage, identity-free output
and unchanged alert/quote-based repair status. Callers checked capabilities
still report offline execution and the configured owner-store boundary.
No review edits, compilation, cleanup actions or live operations occurred.
Author build/tests establish current-source evidence separately from review
runs of existing executables; neither establishes native qualification.

Jev advisory supported source/offline landing: `jev-1.14.0`, request
`395a168ca26c8833e8e39709d2dd1fcf0350ccb93ed8e31be3e1377ca2e92790`,
847 input / 52 output tokens, 0.300 seconds. Probabilities were 0.91 landing,
0.05 needs fix and 0.04 insufficient evidence. The actual tests and independent
review, not advisory confidence, establish the scoped evidence.
