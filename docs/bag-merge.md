# Full-stack merge development contract

Source-only Retail executor: `addon/BagMerge.lua`. It is deliberately not loaded by a packaged TOC or exposed through player Apply yet.

The operation identifies both carried stacks by location, GUID, item ID, exact link and count. Locations and GUIDs must differ; item ID and link must match. Both stacks must be unlocked, the cursor empty and the player outside combat, casting and channelling. Character and client identity must match.

Native `C_Item.GetItemMaxStackSize(ItemLocation)` metadata must be present, equal for both stacks and stable across reads. The complete source quantity must fit. Destination bag compatibility and exact preimages are checked before pickup. Matching descriptions do not prove hidden native stack compatibility.

Execution performs one source pickup and one target placement. Rejected placement, exceptions and delayed observations never trigger retries or cursor clearing. Completion requires an empty source and cursor plus the original target GUID, link and item ID with exactly the combined quantity. Target GUID survival is an unqualified assumption; any other result remains unknown.

Offline test `tests/bag_merge.rs` covers quantity conservation, capacity overflow/missing/changing metadata, changed links, locked stacks, duplicate preimages, rejected placement and later read-only completion. Native functions are mocked. This does not qualify native protection, timing or stack restrictions.

Next: compiler/wire decoder, review/receipt lifecycle, packaging and compatible deployment. Partial merges, sorting and family-specific adapters remain outstanding. No in-game testing is requested while deferred by the user.

Review evidence: Jev1.14.0 advisory returned insufficient_evidence (probability0.89, confidence0.83), request b3ad4cbdedd82530f21d67d1c5fc58019053ecc850b9c01c6aa9c13a03ccb37f, 767 input/116 output tokens,0.321s. This is not a review pass or native verification. Required adversarial-review instruction file was unavailable at its configured path. Direct source review confirms two bounded writes and observation-only completion; native assumptions remain explicitly unqualified.
