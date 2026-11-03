# Equipment-set implementation plan — 2026-11-04

Next gear-management path: capture and operate native equipment sets through explicit player review, keeping exact-instance equipment operations available separately. No new operation is enabled by this plan.

Primary API reference: https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/EquipmentManagerDocumentation.lua (read2026-11-04). C_EquipmentSet exposes set IDs/info, item IDs/locations, ignored slots and assigned specialisation. It also exposes create/save/rename/delete, spec assignment and use, plus pending/finished events. API presence does not establish live compatibility or blanket authorisation.

Required capture: complete bounded catalogue with stable set ID/name/icon, ignored-slot map, item IDs and packed item locations, equipped/missing/inventory counts and optional assigned spec. Preserve absent/unknown values; distinguish ignored from intentionally empty slots. Resolve exact item identity only where a supported location decoder and GUID read agree. Item ID alone is insufficient for selecting duplicate instances.

Required operations: use captured set; create from reviewed current equipment; save existing set with exact previous/current preimages; rename/icon; delete with exact identity; assign/unassign spec with side effects shown. Do not combine these into an opaque batch.

Verification: API return or event is not enough. Read back set contents/metadata and exact equipped identity where available. Protect ignored slots and current player edits. Missing/locked items, changed membership, combat, partial swaps and exhausted bag space need fixtures. Never clear cursor, repeat an uncertain call or bypass protected restrictions. Native multi-slot handling should be investigated before replacing current two-handed-item refusal.

Unresolved: packed location sentinels/decoder, per-family API behaviour, exact-instance preservation in native sets, return/event timing and taint/hardware constraints. Start with collector/schema fixtures, then compiler/executor/reconciliation, then packaging. Live qualification remains deferred by the user.

## Source observation checkpoint — 2026-11-04

Implemented the read-only `equipment_sets` section in the Retail collector, Core export, Rust validator and Python section allowlist. Coverage is explicitly `native_catalogue_raw_slots`. Each set preserves metadata/counts and all nineteen slot records. Absent fields mean unknown; zero/negative raw sentinels are preserved without guessing empty/missing semantics. IDs may be zero. An absent assigned spec stays absent. No GUID or exact-instance claim is made.

Catalogue IDs are unique, bounded to 128 and cross-checked against the native count before and after capture. Changed membership, missing API maps, restricted values and invalid fields make the entire section unavailable. This is not an atomic server snapshot. Numeric bounds are defensive accepted-data limits, not claims about game limits: set/item/icon identifiers fit unsigned 32-bit values; raw slots additionally accept signed 32-bit sentinels. Packed values outside that range are refused. Keeping raw numbers within this range also avoids Lua 5.1 `tostring` rounding larger integers during JSON serialisation.

Verification uses the actual Lua 5.1 collector and Core exporter through Rust import. Fixtures cover zero IDs, empty catalogue, ignored true/false, absent data, assigned/unassigned spec, lost items, sentinel preservation, maximum unsigned raw location and malformed/partial/changed/restricted observations. Rust rejects incomplete slot maps and fields that imply unsupported empty/exact coverage. The existing full exporter fixture still has fourteen available sections plus the unavailable new section when no equipment-set API is supplied.

Advisory Jev review: `jev-1.14.0`, request `abc626c61fa7449845c0c341d52bb837b9f7ab9d9e00e38df5b7fc03433cda2f`, 1215 input/154 output tokens, 0.296 seconds; disposition `insufficient_evidence` (reported confidence 0.26). This is not a passing review. Direct source inspection identified and tightened the large-number serialisation boundary, with a producer roundtrip test. The configured adversarial-review rule and OCR skill files remain absent; neither workflow is claimed. No game action or new installation occurred; hosted importers must be upgraded before this section is installed.
`python -m tools.verify_source` passed after the numeric-bound correction: format, locked build, all Rust tests, Clippy with warnings denied, and 31 Python protocol/tool tests. This proves the fixture/source contract, not game compatibility.

## Observed item identities — source only

The raw slots can now include `observed_item`: kind (`equipment` or carried `bag`), slot, optional bag, item ID, GUID and lock flag. Resolution uses the native `EquipmentManager_GetLocationData` return contract, then normal ItemLocation/C_Item reads. Only explicit player/non-bank flags and bounded equipment/carried-bag coordinates qualify. The item ID must match the raw set item ID, and two ID/GUID reads must agree. Missing APIs, unsupported locations, restricted values and changing/mismatched identities omit this optional observation without discarding the raw catalogue.

This identifies the item observed at the set's reported location. It does not expose the set's internal persistent item identity, promise an atomic snapshot, or prove which instance a later native set swap will use. Ignored, empty/missing sentinel and bank slots remain unresolved. A future Apply must recheck both catalogue contents and item identities; post-apply verification must check actual equipped GUIDs.

Primary references checked 2026-11-04: [native location decoder](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_FrameXML/Shared/EquipmentManager.lua), [native equipment manager caller](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_FrameXML/Mainline/EquipmentManager.lua), and [C_Item declarations](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/ItemDocumentation.lua). The native decoder is not copied or replaced. Fixtures mock its documented source return shape; live decoding remains unqualified.

Tests extend the actual producer/import roundtrip with same-ID worn and bagged items carrying different GUIDs, locked items, absent/changing/secret identities, wrong item IDs, missing APIs and out-of-bounds/bank locations. Nested Rust validation rejects malformed locations, unknown kinds, equipment carrying a bag index, mismatched IDs, null locks and control characters in GUIDs.

Advisory review: `jev-1.14.0`, request `e2befe707491b73b040ce37196bb440080fba847cd321eca786079f23f083cf2`, 1203 input/131 output tokens, 0.300 seconds; `no_material_issue` with reported confidence 0.38. Advisory only; direct source/fixture checks remain the evidence. No game action, new addon installation or hosted deployment is included.
`python -m tools.verify_source` passed for the identity changes: format, locked build, all Rust tests, Clippy with warnings denied and 31 Python tests. The configured adversarial-review file is still absent; no such workflow is claimed.
