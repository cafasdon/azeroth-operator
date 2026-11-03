# Equipment-set implementation plan — 2026-11-04

Next gear-management path: capture and operate native equipment sets through explicit player review, keeping exact-instance equipment operations available separately. No new operation is enabled by this plan.

Primary API reference: https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/EquipmentManagerDocumentation.lua (read2026-11-04). C_EquipmentSet exposes set IDs/info, item IDs/locations, ignored slots and assigned specialisation. It also exposes create/save/rename/delete, spec assignment and use, plus pending/finished events. API presence does not establish live compatibility or blanket authorisation.

Required capture: complete bounded catalogue with stable set ID/name/icon, ignored-slot map, item IDs and packed item locations, equipped/missing/inventory counts and optional assigned spec. Preserve absent/unknown values; distinguish ignored from intentionally empty slots. Resolve exact item identity only where a supported location decoder and GUID read agree. Item ID alone is insufficient for selecting duplicate instances.

Required operations: use captured set; create from reviewed current equipment; save existing set with exact previous/current preimages; rename/icon; delete with exact identity; assign/unassign spec with side effects shown. Do not combine these into an opaque batch.

Verification: API return or event is not enough. Read back set contents/metadata and exact equipped identity where available. Protect ignored slots and current player edits. Missing/locked items, changed membership, combat, partial swaps and exhausted bag space need fixtures. Never clear cursor, repeat an uncertain call or bypass protected restrictions. Native multi-slot handling should be investigated before replacing current two-handed-item refusal.

Unresolved: packed location sentinels/decoder, per-family API behaviour, exact-instance preservation in native sets, return/event timing and taint/hardware constraints. Start with collector/schema fixtures, then compiler/executor/reconciliation, then packaging. Live qualification remains deferred by the user.
