# Player-applied bag operations

Development source, 2026-11-03. The executor moves a complete stack from one carried bag slot to an explicitly empty carried slot. Source GUID, item ID and count must match; both locations are checked before pickup and again before placement. Combat, casting, occupied cursor, locks, invalid carried locations and identical locations are refused. No automatic retry or cursor clearing occurs after a partial effect.

Verified Lua fixtures cover success, stale count/GUID, combat, occupied destination, replay against changed locations, failed placement and a destination becoming occupied between pickup and placement. Runtime asynchronous updates and specialised bag acceptance remain unqualified; a refused placement retains the cursor item and fails readback.

API reference checked 2026-11-03: https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/ContainerDocumentation.lua — PickupContainerItem takes bag and slot; SplitContainerItem additionally takes amount. This is API evidence, not blanket policy approval.

The Rust snapshot-bound compiler and AO-BAG-1 Lua decoder are implemented and roundtrip-tested. The format carries sixteen colon-separated fields: magic, operation ID, snapshot ID, family, variant hex, build hex, interface, character hex, realm hex, source bag, source slot, target bag, target slot, item ID, GUID hex and full-stack count. Destination emptiness is an explicit request precondition, rechecked at execution. Missing GUIDs and captured occupied destinations are refused.

Shared review/receipt dispatch, MCP preparation and packaging are implemented in source. The protocol test calls prepare_bag_operation; Lua dispatch tests require review, verify the pre-execution uncertainty receipt and refuse replay. Full source gate and 31 Python tests pass. Retail 0.5.0-dev (thirteen files) passed staged Lua 5.1 loading/export and changed-interface refusal. ZIP SHA256: 683a0d255198bf0fdd59f7d7b80d97338c1b4d183f89c696bdfeac5c9fd19da5.

Pending: deployment and live tests. No bag mutation is deployed yet. Swaps, merging, splitting, sorting and bank transfers remain subsequent work within the full operational goal.

The dispatcher error-path fixture also verifies that an executor exception preserves outcome_unknown and blocks replay. Advisory claims review: Jev 1.14.0, request 65b9ff671d0495ed95cd33b857e9d39ab2dc5b695efb39c139de3cd2ddfb988d, 836 input/116 output tokens, 0.304 seconds. Result insufficient_evidence (reported confidence 0.42), not a review pass. Deterministic fixture results support only source behaviour; live qualification is outstanding. The configured adversarial-review file is absent; that workflow was not run.

## Stack splitting — source only

The bag executor now supports split_bag_item internally, with amount strictly between zero and the captured full count. It rechecks the original GUID and remaining quantity after splitting, then verifies a distinct destination GUID, matching item/link, exact split amount and empty cursor. Tests cover quantity conservation, invalid amounts, failed placement and incorrect destination count. Existing full-stack move tests still pass.

The distinct-GUID and synchronous-readback behaviour are fixture expectations, not live qualifications. Compiler/transport, MCP optional amount, review text and receipt enum now support splits in source. AO-SPLIT-1 extends the full-stack fields with an amount; full moves retain their original AO-BAG-1 format and operation IDs. Rust-to-Lua and shared dispatch tests cover amount preservation and receipts. Deployment and live split verification remain pending. Installed 0.5.0-dev is unchanged, preserving the pending live move test.

2026-11-04 package/UI verification: review fixtures distinguish full moves from splits, display source GUID, target slot, split amount and remaining quantity, and invalidate Apply after edits. Retail 0.6.0-dev ZIP d09acb556804b24f4493655b89d3fabea9e1929acf19794b56c0ebeda4836ba8 passed staged Lua loading/export and interface-change refusal. Importer qualification fixture now includes split_bag_item uncertainty receipts. Package is not installed; live split behaviour remains unqualified.

Release candidate 0.6.1-dev includes bounded Apply diagnostics. ZIP SHA256 7268c23ab78e85263e9f1b05db8164a6a038d54a78095607b588004478afe5c0; staged package checks and complete source gate (31 Python tests plus Rust suites) passed. This supersedes 0.6.0-dev for release preparation but is not installed.

Advisory claims review: Jev 1.14.0, request 4213d021af318ee18ea6fdba334ee1a8f63e95f8dfd69c204f0620173966dc46, 873 input/116 output tokens, 0.34 seconds, insufficient_evidence (reported confidence 0.34). Not a review pass. Fixture evidence supports development source behaviour only; split GUID lifecycle and live timing still require game verification.

## Inventory observation timing — 2026-11-04

Source change following the unresolved live bag attempt: accept either the exact original source GUID/item/count/link or the expected post-pickup state between pickup and placement. Requiring immediate source disappearance was an unnecessary synchronous assumption. This is a modelled failure mode, not a proven diagnosis of the player's unknown receipt.

Final readback still requires an empty cursor, unlocked target, exact target identity/content/count and source vacancy (move) or quantity conservation with a distinct target GUID (split). If that readback is not yet conclusive, return outcome_unknown. The receipt reader can observe the same operation for up to 60 seconds during export and promote to applied only when every condition matches. It never performs game writes. Expiry or reload leaves the saved unknown receipt intact; no retry or cursor clearing is added. An earlier unknown operation is never enrolled in this new in-memory reconciliation.

Fixtures cover retained source during pickup, delayed full-move and split updates, wrong destination GUID/count, locked destination, changed character, expiry, occupied destination races, rejected placement and duplicate-operation refusal. Full source gate passed (Rust, Clippy, format/build, 31 Python tests); additional delayed split fixture passed separately. This code is not yet packaged or installed. In-game qualification remains deferred by the player.

Advisory Jev review: request d102834defcc0a7305c92c8ef57f9e2dd8438f07fe652b40ec1cfe57b50c3515, jev-1.14.0, 952 input/135 output tokens, 0.309 seconds, insufficient_evidence (reported confidence 0.77), not a pass. Tests establish the stated fixture behaviours, not live API semantics. Configured adversarial-review file is absent; that workflow was not run.


Deployment follow-up (2026-11-04): PR #7 merged as 76aaa9cdc796a89ce3f6f9b14c8613f524f96f5c. Retail 0.7.1-dev package SHA256 fb61947532f71ab781ba76c54b7f9614be057af8c04e8f38e622d37e07b37a57 passed packaged Lua5.1 export and changed-interface rejection. Installed fifteen files independently match its manifest. Backup local-data/addon-backups/addon-update-yynl_cx3. No schema or hosted binary change was needed. Running-client loading and real bag semantics remain unverified; no game input or retry occurred.
