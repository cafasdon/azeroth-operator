# Weapon-enchant observations — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broad in-game coverage remains unfinished.

The public [Forever item API](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ItemDocumentation.lua)
defines `C_Item.GetWeaponEnchantInfo(weaponSlot)` returning enchant records.
[Item constants](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ItemConstantsDocumentation.lua)
define main hand/off hand/ranged values 0/1/2 and types none, permanent,
temporary and imbue. The [buff UI](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_BuffFrame/BuffFrame.lua)
iterates these lists and divides `timeLeft` by 1000 for displayed seconds.
No legacy positional global return shape is assumed.

Manual Forever capture adds `weapon_enchants`, with coverage
`public_weapon_enchant_lists` and a `hands` map. Each readable hand contains
`entries` keyed by native list index (accepted compact 1–32). Required fields
are Boolean `has_enchant` and labelled `enchant_type`. Only true rows may
carry optional positive u32 `enchant_id`, u32 `charges`, and u32
`remaining_milliseconds`. Zero charges and timers are retained literally;
they do not establish depletion, expiry or whether the effect applies.

Two list reads must agree on row count, reported presence/type and readable
ID (including its unknown state). Failure, nil, restriction, invalid shape,
unsupported type, sparse/excessive lists or changing identity/status leave
that hand absent, preserving other hands and equipment. Missing API makes
the section unavailable. Unexpected/missing slot enum values leave that hand
unknown. Detail values are screened before comparisons. Invalid/unreadable
optional values are omitted. Changed charges are omitted independently.
The later timer is retained only when both readings are valid and the rounded
millisecond value does not increase; natural countdown is allowed. Increasing
or unknown timers remain absent. This stability rule does not prove effect
or item-instance continuity. False rows ignore all optional native details.

Both readers enforce exact fields, types, bounds, placement, hands, coverage
and Forever-only availability. Empty hands and empty lists are valid observed
shapes, distinct from missing hands or a missing section. Update readers
before syncing new exports. Native compatibility remains unqualified.

`character_summary.weapon_enchant_observations` lists each observed hand's
row count and at most 32 `active_enchants`, numerically ordered by captured
row index. Here active means only the captured `has_enchant: true` flag.
Optional details can be null. Missing hands are explicitly listed; missing
sections return null. These are historical records, not a running countdown,
proof of complete effects or current effectiveness. No weapon item ID/GUID
association is asserted. `effect_on_performance` remains unverified. Existing
permanent-link/catalogue handling and simulation inputs are unchanged.
No enchant/buff application, cancellation, selection or other game action
is introduced.

Verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test weapon_enchants --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_weapon_enchants test_native_item_stats
cargo fmt --check
git diff --check
```

Build passed; three Rust tests and two real MCP Python tests passed. Actual
Lua export/Rust import covers multiple enchant types, false rows with private
unused details, empty/ranged lists, millisecond rounding/countdown, changed
charges or increasing timer, errors/nil/restrictions, invalid/sparse/excessive
lists, changing type/ID/status, invalid/unknown details and equipment isolation.
Real MCP cases cover strict schema/family rejection, explicit empty/missing
hands and sections, numeric ordering, unknown detail nulls, zero values,
maximum timer and private diagnostic masking. Prior native item-stat review
still passes. The maximum three hands with 32 active rows each (96 total)
and u32-boundary details also pass the real MCP reader/summary. No native/API
timing behaviour is claimed from fixtures.

Development package, not installed:
`local-data/weapon-enchants-qualification/AzerothOperator-0.0.0-weapon-enchants-test-forever-live.zip`.
SHA-256: `f34fcbb4ea0720ca27064a9862ab07327cfcf358515be5567466898baf32ae39`.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, and found no concrete defect. Both ran the real MCP test and prebuilt
Lua/Rust fixture, traced actual callers and checked API/enums/millisecond
semantics against the public sources. Additional independent Python probes
rejected 35 and 32 malformed detail/index cases respectively and accepted
boundary/omitted details. Callers verified package hash and documentation.
No review edits, compilation, cleanup actions, deployment or game interaction
occurred. Native qualification is not inferred from prebuilt fixture runs.

Jev advisory supported scoped offline landing: `jev-1.14.0`, request
`8949c8527c2a1f458d949cd48d8f39ccafa24b066f448f9854f2908dc255b2ac`,
825 input / 52 output tokens, 0.305 seconds. Probabilities were 0.96 landing,
0.02 needs fix and 0.02 insufficient evidence. Actual tests and independent
source/caller review, not advisory confidence, establish the scoped evidence.
