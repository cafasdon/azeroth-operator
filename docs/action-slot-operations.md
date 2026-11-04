# Action-slot operations — development record

2026-11-04: Source-only ActionOperations.lua implements one Retail move/swap between currently observed slots. It is deliberately not yet included in the package, dispatcher or MCP. Compiler, transport, review UI, receipt enums and completion reconciliation remain required before enabling it.

Checks: exact client/build/interface/character/realm; no combat/casting/channel/cursor; source occupied; distinct bounded slots; both currently observable; exact kind/ID/subtype; macro name/icon/body preimages. The source pickup explicitly requests removal rather than copy. Before placement it checks the cursor, source vacancy and unchanged destination. Swaps verify the new target and displaced cursor before returning it to the source. Final readback checks both slots and empty cursor. Failure retains cursor/uncertainty; no automatic retry, ClearCursor, action invocation or protected-function bypass.

Primary reference: https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_ActionBar/Shared/ActionButton.lua, read 2026-11-04. Native OnDragStart calls PickupAction with an explicit copy/removal flag; OnReceiveDrag calls PlaceAction. This establishes a call pattern, not addon taint/protection permission. Mainline/ActionButton.lua and guessed ActionBarDocumentation.lua paths returned404 and were not evidence.

Cursor identity assumptions need further qualification before enabling: spell ID in fourth GetCursorInfo return; item/macro ID in second. The fixture models these explicitly, but does not establish real-client behaviour. Current supported executor kinds are spell/item/macro; flyouts, equipment sets, mount/pet/special actions remain required future work with their own identity contracts. Empty/missing observation is never inferred from an absent slot.

Tests execute the real Lua module with move-to-empty and spell/macro swap, changed macro, missing slot, combat, occupied cursor, rejected placement and secret values. Full source gate passed: format/build, Rust tests, Clippy and31Python tests. No live action was attempted.

Jev advisory: request fdda146351c1f79062b0d5c88f02802f26857296fa43b9b7e32924048fe5c7c5, jev-1.14.0,941input/116output tokens,0.291seconds; insufficient_evidence (reported confidence0.77), not a pass. Configured adversarial-review file absent; workflow not run. Live tests remain deferred by the user.

## Compiler and decoder — 2026-11-04

Source-only ActionRequest compiler and ActionImport decoder now bind two explicitly observed slots, source/target contents and macro preimages to snapshot/client/character identity. AO-ACTION-1 has23colon-separated fields with hex text and SHA256 operation ID; total size is bounded at8192bytes. Empty target is accepted only from an explicit empty observation. Missing target/source, identical locations, unsupported kinds, missing macro contents and oversized data are refused. Macro body changes produce a different operation ID. Empty-string subtype is refused as ambiguous with the absent subtype transport representation.

Rust-to-Lua tests preserve newline/colon/pipe text and distinguish move from swap; malformed fields and missing preimages are rejected. Full source gate passed including31Python tests. No MCP registration, dispatcher/UI integration or deployment yet. Executor cursor assumptions remain unqualified.

Jev advisory request c20e1a8ef52410679718bfd12f2fb9dc7f4b528ddfcb97c5566b251937a752f5, jev-1.14.0,886input/116output tokens,0.288seconds: insufficient_evidence (reported confidence0.42), not a pass. Deterministic checks establish transport behaviour only.

## MCP preparation integration — 2026-11-04

prepare_action_operation is now exposed by source Rust MCP (thirteenth tool). Its description and result explicitly state that addon execution is disabled. Actual stdio protocol tests prepare a macro-to-empty-slot request, verify exact macro body and reject an unobserved target. Importer qualification fixture now includes partial action observations. Gateway/probe tool lists updated; probe count derives from discovery rather than a stale literal.

The protocol fixture caught that the Python reference store omitted the new observation section; its section allowlist is now aligned. This reference store remains a structural validator; Rust owns detailed nested validation. Full source gate subsequently passed (including31Python tests). Source only, not deployed. Existing installed addon does not accept AO-ACTION-1.

Jev advisory request9bf990ca41800e6c3cd3a0e92ff00734b794c82f84dd6830bc902a590f7b39f9, jev-1.14.0,817input/116output tokens,0.293seconds: insufficient_evidence (reported confidence0.43), not a pass. No claim of live or deployed action-bar control.

Primary cursor evidence follow-up (2026-11-04): Blizzard_ClickBindingUI/Blizzard_ClickBindingUI.lua, AddFromCursorInfo, reads GetCursorInfo's fourth result as spellID and second as macroID. Source: https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_ClickBindingUI/Blizzard_ClickBindingUI.lua. This supports the executor's spell/macro tuple mapping; item identity and actual action-pickup/taint behaviour still require qualification. No ClearCursor or click-binding writes from that reference were adopted.

Observation and MCP preparation deployed in0.8.0-dev; see action-observation-deployment-2026-11-04.md. Action executor and decoder remain outside package/dispatcher.

## Review/apply/receipt source integration — 2026-11-04

AO-ACTION-1 now routes through the source reviewed-byte dispatcher to ActionOperations. The shared pre-effect checkpoint records move_action_slot/outcome_unknown, duplicate IDs are refused, and a returned unknown can use existing bounded read-only observation reconciliation. Rust accepts the new receipt kind in both checkpoint and snapshot validation. UI displays source/target slots, move versus swap and exact macro name/icon/body.

End-to-end offline fixture runs the Rust compiler, real Lua decoder, review dispatcher, executor, AO.Capture and Rust importer. It confirms slot swap, empty cursor, persisted applied receipt and duplicate Apply refusal. UI fixture verifies displayed slot/macro details. A first test incorrectly assumed companion import merges the operations checkpoint into an old export; actual implementation intentionally requires AO.Capture, and the test now exercises that path. Full source gate passed, including31Python tests.

Not packaged or deployed. Installed0.8.0-dev still has no action decoder/executor; hosted preparation continues reporting addon_execution_enabled=false. Actual item cursor contract and protected/taint behaviour remain open. No in-game testing requested or performed.

Jev advisory b677ae70c8fe252529383add0fb3d23d71ade5f1fb82afe90dd8de594f2ca8a0, jev-1.14.0,871input/116output tokens,0.295seconds: insufficient_evidence (reported confidence0.40), not a pass. Configured adversarial-review file absent; workflow not run.

## Development package candidate — 2026-11-04

Retail0.9.0-dev now packages seventeen files, adding ActionOperations and ActionImport before the dispatcher. Upgrade/installed-file validators retain all earlier manifest shapes. Compiler metadata requires addon0.9.0-dev and distinguishes execution support from live_qualified=false. Importer qualification includes move_action_slot receipts. No installation occurred; importer/reader/companion upgrade must precede addon installation.

Full source gate passed after package/schema integration (31Python tests). Additional targeted tests then passed for delayed final placement, read-only receipt promotion, expiry with unknown retained, replay refusal and unavailable placement API before any pickup. Packaged Lua5.1 loading/export and interface-change rejection passed. Final ZIP SHA256307c3d3e7baf0f00fd62abdea83009e7084542d2f1b9475a69ea747c0284785a (earlier candidate db821604 was superseded before deployment).

Independent implementation evidence for item cursor mapping: https://github.com/kemayo/wow-bankstack/blob/master/core.lua reads cursor type/item ID from the first/second GetCursorInfo results. Read2026-11-04. This supports tuple interpretation, not Blizzard authorisation or this executor's live behaviour. BankStack's retries/automatic cursor handling were not adopted. Native spell/macro tuple references are recorded above. Taint/protected-function behaviour remains unqualified and will not be bypassed.

Jev advisory90b4990ba27129135dd2e5f018b0c610e6422b07b7ca0e7a2a0ca3772897cc7c, jev-1.14.0,900input/115output tokens,0.298seconds: investigate_supported_issue (reported confidence0.66), not a pass and no specific source finding supplied. Follow-up inspection added a pre-write PickupAction/PlaceAction availability guard and zero-write refusal test. Deterministic evidence supports a development candidate only; no live-success claim.


## Copy executor — 2026-11-04

Source-only `copy_action_slot` reconstructs a spell/item/macro cursor by exact ID using `C_Spell.PickupSpell`, `C_Item.PickupItem` or `PickupMacro`, then places it once into an explicitly empty observed destination. It inherits client/character, combat/cast/channel, empty cursor, supported type and macro-content preconditions. Both slots are checked again after pickup. Final and delayed readback require the original source to remain identical, target to match it and cursor to be empty. No source-removing pickup, action invocation, cursor clearing or retry is introduced.

Primary implementation reference: [Blizzard SecureHandlers source mirror](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_RestrictedAddOnEnvironment/SecureHandlers.lua), read 2026-11-04. `PickupAny` dispatches spell/item/macro IDs to those pickup functions. This supports API choice, not native taint/protection qualification or policy approval.

Offline fixtures cover all three kinds, occupied destination refusal before writes, unavailable API, wrong cursor, changed source after pickup, rejected placement with retained cursor, delayed read-only confirmation and duplicate preimage refusal. Full source verification passed: format, locked build, Rust/Lua tests, Clippy with warnings denied and 38 Python tests. Existing move/swap tests pass. This is not exposed by MCP or the shared decoder/review/receipt path yet and is not deployed.

Jev advisory: request `93254a567a0d02c688ed65f3f09b9ec164fd295db03f4c51f0ff411fd1fb1ecd`, resolved model `jev-1.14.0`, 985 input/116 output tokens, 0.310 seconds. `insufficient_evidence` (probability 0.78, confidence 0.67); no concrete finding or review pass. Configured adversarial-review file was absent. Direct source review and deterministic fixtures support only the stated offline behaviour. In-game testing remains deferred by the user.


## Copy preparation and transport — 2026-11-04

`prepare_action_operation` accepts optional `operation: "move" | "copy"` inside its operation request. Omission preserves legacy move/swap semantics and AO-ACTION-1 bytes. Copy uses AO-ACTIONCOPY-1 with the same 23 bounded fields and 8192-byte limit; the distinct magic participates in the SHA256 operation ID. Compiler and decoder both require an explicitly empty target. Source/macro identities, client/character and snapshot preimages remain bound. This hash is an identifier, not authentication.

The source MCP reports copy `addon_execution_enabled=false`, `requires_addon_version=null` and `game_execution=false`. The shared import dispatcher still refuses the new format: review/Apply/receipt integration is next. No package or deployment changed.

Verification: full source gate passed (format/build, all Rust/Lua tests, Clippy, 38 Python tests). The actual stdio MCP caller prepares copy and legacy move with distinct IDs. Actual Rust compiler → Lua decoder → executor composition preserves source and places a copy with mocked spell APIs. Occupied-target copy is rejected by both compiler and decoder; unsupported operation enum is rejected. Existing move/swap and malformed-field checks pass. Two fixture mistakes were corrected before the gate passed: observed-button count and missing interface return in GetBuildInfo. Neither was a product defect.

Jev advisory request `7ab91be04baff225b532c209d0016fff2e8e947fad75972f9a3b9971ef999496`, resolved model `jev-1.14.0`, 1090 input/154 output tokens, 0.289 seconds: `insufficient_evidence` (probability 0.45, confidence 0.18), no concrete finding or review pass. Configured OCR skill and adversarial-review file were absent; those workflows did not run. Direct source inspection and deterministic checks support the narrow offline claims above, not native qualification.
