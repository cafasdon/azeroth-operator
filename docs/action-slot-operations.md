# Action-slot operations — development record

2026-11-04: Source-only ActionOperations.lua implements one Retail move/swap between currently observed slots. It is deliberately not yet included in the package, dispatcher or MCP. Compiler, transport, review UI, receipt enums and completion reconciliation remain required before enabling it.

Checks: exact client/build/interface/character/realm; no combat/casting/channel/cursor; source occupied; distinct bounded slots; both currently observable; exact kind/ID/subtype; macro name/icon/body preimages. The source pickup explicitly requests removal rather than copy. Before placement it checks the cursor, source vacancy and unchanged destination. Swaps verify the new target and displaced cursor before returning it to the source. Final readback checks both slots and empty cursor. Failure retains cursor/uncertainty; no automatic retry, ClearCursor, action invocation or protected-function bypass.

Primary reference: https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_ActionBar/Shared/ActionButton.lua, read 2026-11-04. Native OnDragStart calls PickupAction with an explicit copy/removal flag; OnReceiveDrag calls PlaceAction. This establishes a call pattern, not addon taint/protection permission. Mainline/ActionButton.lua and guessed ActionBarDocumentation.lua paths returned404 and were not evidence.

Cursor identity assumptions need further qualification before enabling: spell ID in fourth GetCursorInfo return; item/macro ID in second. The fixture models these explicitly, but does not establish real-client behaviour. Current supported executor kinds are spell/item/macro; flyouts, equipment sets, mount/pet/special actions remain required future work with their own identity contracts. Empty/missing observation is never inferred from an absent slot.

Tests execute the real Lua module with move-to-empty and spell/macro swap, changed macro, missing slot, combat, occupied cursor, rejected placement and secret values. Full source gate passed: format/build, Rust tests, Clippy and31Python tests. No live action was attempted.

Jev advisory: request fdda146351c1f79062b0d5c88f02802f26857296fa43b9b7e32924048fe5c7c5, jev-1.14.0,941input/116output tokens,0.291seconds; insufficient_evidence (reported confidence0.77), not a pass. Configured adversarial-review file absent; workflow not run. Live tests remain deferred by the user.
