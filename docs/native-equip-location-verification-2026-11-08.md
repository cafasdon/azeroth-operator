# Native equipment-location observations — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broad in-game coverage remains unfinished.

The public [Forever item API](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ItemDocumentation.lua)
defines the fourth `GetItemInfoInstant(itemInfo)` return as `itemEquipLoc`.
Manual Forever collection now optionally saves it as `equip_location` on
linked equipment/container items. Each full-link read must report the
observed base item ID. Two readable token observations must agree. Nil,
restriction, invalid format, excessive length, errors, mismatched IDs or
changing values leave the field absent, preserving valid item identity and
other observations. No explicit load, tooltip query or game action is added.
Unused native return values, including localised names, are discarded.

Both readers require a nonempty link and an optional non-null location
string: either empty or an ASCII `INVTYPE_[A-Z0-9_]+` token of at most 64
bytes. Empty is preserved as reported, not inferred to mean unequippable.
Unknown future tokens are valid observations with unknown placement. The
generic item contract supports all storage sections and client families;
new native collection and category annotations are enabled only for Forever.
Updated readers are required before syncing these exports.

Each Forever equipment-review change adds `native_slot_category` with the
observed token, candidate slot-category numbers, nullable category-match
flag and `eligibility_verified: false`. This uses the project's Forever
1–19 slot convention: standard armour/accessories map to their slot, chest
and robes share 5, rings 11/12, trinkets 13/14, cloak 15, explicit main-hand
weapons 16, off-hand/shield/held items 17, ranged/thrown/relic categories 18
and tabards 19. Generic weapons are category candidates for 16/17.
Two-handed weapons retain both 16/17 as potential categories but return
unknown for off-hand matching rather than assuming an ability is absent.
All native placement remains unqualified pending game testing.

Unmapped, empty or absent tokens have null candidates/matches. No bag-slot,
profession-gear or other omitted mapping is invented. Non-Forever reviews
return a null category annotation. These annotations neither approve nor
reject a draft, alter `checks.slot_compatibility` (still unknown), nor change
catalogue legality, simulator inputs or ranking filters. Category agreement
does not establish armour/weapon proficiency, dual-wield, level, profession,
unique limits, hand pairing or full-set legality. Saved observations do not
establish current ownership or live placement acceptance.

Verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test native_equip_location --test native_item_stats --test item_required_level --test addon
cargo test --locked --offline --lib planning::tests
$env:PYTHONPATH='.;tests'
python -m unittest test_native_equip_location test_native_item_stats test_native_level_requirements
cargo fmt --check
git diff --check
```

Build passed; nine Rust tests and three real MCP Python tests passed. Actual
Lua export/Rust import covers equipment/bags, known/empty/future tokens,
missing links/APIs, restricted IDs/tokens, malformed/overlong/missing values,
changed values, errors and private unused native returns. Real MCP cases
cover 25 known categories with matching/mismatching targets, both ring slots,
conditional two-handed off-hand placement, unknown/bag/empty/legacy tokens,
strict schema rejection and unchanged overall compatibility. Native stat
comparisons and native level/ranker behaviour still pass with local data.

Development package, not installed:
`local-data/equip-location-qualification/AzerothOperator-0.0.0-equip-location-test-forever-live.zip`.
SHA-256: `53c64f1cd58c369a00e2e0bf12385390834b1478dc49bc246363aad5352e5795`.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, and found no concrete defect. Both ran the real MCP and prebuilt
Lua/Rust fixture; callers also ran adjacent native-stat/level tests. Additional
caller probes covered all four storage readers and four source reviews with
the local catalogue, plus a deliberately conflicting native/catalogue category:
the annotation reported false while existing simulator checks stayed identical.
Failure review confirmed Retail/Classic annotations are null, generic off-hand
category matching leaves eligibility unknown, and six Python/Rust byte-boundary
pairs agree (tokens and ASCII/UTF-8 links). No review edits, compilation,
cleanup actions, deployment or game interactions occurred. Prebuilt fixture
execution does not establish native compatibility or source provenance;
author builds/tests establish current-source evidence separately.

Jev advisory supported scoped offline landing: `jev-1.14.0`, request
`f91150a1e0cf4a6200aa6b0f54da9d7a4c363eb7352835da29681acfdbd0ec6e`,
921 input / 52 output tokens, 0.376 seconds. Probabilities were 0.98 landing,
0.01 needs fix and 0.01 insufficient evidence. Actual tests and independent
review, not advisory confidence, establish the scoped verification.
