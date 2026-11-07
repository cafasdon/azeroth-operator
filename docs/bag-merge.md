# Full-stack merge development contract

Source-only Retail implementation. `prepare_bag_operation` accepts `operation: "merge"` with two captured occupied locations. It refuses `amount` and an empty-target precondition. The local MCP reports `addon_execution_enabled: false` and no minimum addon version while packaging is pending.

The operation identifies both carried stacks by location, GUID, item ID, exact link and count. Locations and GUIDs must differ; item ID and link must match. Both stacks must be unlocked, the cursor empty and the player outside combat, casting and channelling. Character and client identity must match.

Native `C_Item.GetItemMaxStackSize(ItemLocation)` metadata must be present, equal for both stacks and stable across reads. The complete source quantity must fit. Destination bag compatibility and exact preimages are checked before pickup. Matching descriptions do not prove hidden native stack compatibility.

Execution performs one source pickup and one target placement. Rejected placement, exceptions and delayed observations never trigger retries or cursor clearing. Completion requires an empty source and cursor plus the original target GUID, link and item ID with exactly the combined quantity. Target GUID survival is an unqualified assumption; any other result remains unknown.

Offline test `tests/bag_merge.rs` covers quantity conservation, capacity overflow/missing/changing metadata, changed links, locked stacks, duplicate preimages, rejected placement and later read-only completion. Native functions are mocked. This does not qualify native protection, timing or stack restrictions.

## Transport and player review

`AO-BAGMERGE-1` carries 21 colon-delimited fields, including exact client/character identity and both stack preimages. The compiler derives the operation ID from these fields and the snapshot ID. The ID identifies a receipt and does not authenticate the input. The Lua decoder accepts bounded canonical fields and rejects duplicate stack instances, mismatched descriptions and out-of-range combined counts.

The source review panel displays both stack preimages and the expected combined quantity. Apply requires unchanged reviewed bytes. Before execution the dispatcher checkpoints `outcome_unknown`, then records the result. The read-only verifier can promote a delayed result for up to 60 seconds. Expired or exceptional attempts retain their unknown receipt, and a saved receipt blocks replay of the same operation ID after reload.

Offline verification exercises the real Rust stdio MCP preparation endpoint, the compiler-to-Lua review/Apply and SavedVariables import round trip, and the review panel event fixture. Packaging, compatible deployment, and native testing are follow-up work. Partial merges, sorting and family-specific adapters remain outstanding.

2026-11-07 review: the caller and failure passes checked MCP preparation, CLI import, source/package separation and replay after an uncertain pickup. The failure pass caught an exception message that suggested retrying. The merge exception now tells the player to inspect the cursor and both slots and to avoid retrying. The scoped lifecycle and UI tests passed again after the fix. Jev advisory triage classified the finding as material (model `jev-1.14.0`, request `93f4fc24077ed51c1063ceb56454832a8ee6ec39e1cbbc1df433383232dc298b`, 485 input/45 output tokens, 0.315 seconds).

Review evidence: Jev1.14.0 advisory returned insufficient_evidence (probability0.89, confidence0.83), request b3ad4cbdedd82530f21d67d1c5fc58019053ecc850b9c01c6aa9c13a03ccb37f, 767 input/116 output tokens,0.321s. This is not a review pass or native verification. Required adversarial-review instruction file was unavailable at its configured path. Direct source review confirms two bounded writes and observation-only completion; native assumptions remain explicitly unqualified.
