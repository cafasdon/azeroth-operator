# Talent operation design — 2026-11-04

Status: API/UI source investigation complete for the initial saved-loadout switching path. No talent mutation implemented or deployed.

Primary implementation evidence (Blizzard UI source mirror, live branch, read 2026-11-04):
- https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/ClassTalentsDocumentation.lua
- https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentsFrame.lua

The working config returned by GetActiveConfigID is not equivalent to the selected saved loadout. The UI deliberately avoids recording the working config as the selected saved config. It updates selection on successful completion or NoChangesNecessary. LoadConfig(configID, autoApply) returns Error, NoChangesNecessary, LoadInProgress or Ready. LoadInProgress is a request in progress, not proof of a completed change. A generic immediate applied receipt would therefore be incorrect.

Implementation requirements:
1. Capture active spec, working config ID, selected saved config ID, staged-change state and a bounded catalogue from GetConfigIDsBySpecID. For each saved config, capture name and an exact content representation; ID/name alone cannot detect edits to the same saved loadout.
2. Compile a player-reviewed request bound to client, character, current spec, current committed talent contents and exact desired saved-loadout contents. Refuse staged player edits rather than silently replacing them.
3. Execute only through ordinary player-triggered APIs, with client/spec/content and game-state preconditions checked again. Respect refusal and protected restrictions. Do not implement a fallback bypass.
4. Record request uncertainty before the call. NoChangesNecessary still needs content readback. LoadInProgress needs bounded, observation-only event reconciliation; no automatic repeated writes, commit retries or timer-triggered application. Ready is staged, not applied.
5. Verify committed node/entry/rank state and lack of staged changes before reporting completion. Last-selected ID alone is not proof. Preserve an unknown result across reload if completion was never observed.
6. Loadout-specific action bars can change as part of the native load. Capture the shared-action-bar setting and expose that effect in review. The current unavailable action-bar export cannot prove action-bar preservation.

Next work: implement catalogue capture and nested schema/fixtures; then compiler and a completion-aware executor. Importing newly generated builds, node allocation, create/rename/delete loadouts and specialisation changes remain part of the wider goal and require their own preconditions and verification. This evidence establishes API behaviour, not blanket policy authorisation.

Saved-loadout capture is now implemented in source as optional talents.saved_loadouts: active spec, selected saved ID and up to 128 saved entries with name, canonical import string and serialization version. Capture omits the entire catalogue if any entry is unavailable/staged or IDs repeat; it does not publish a partial catalogue as complete. Existing active talent capture remains available. Rust validates nested keys, IDs, bounds and import alphabet. Actual Lua producer roundtrip tests verify two distinct saved builds and selected ID, reject malformed contents and omit duplicate-ID captures. Not packaged/deployed; importer and reader must support the new field before installation. Shared-action-bar metadata and live catalogue qualification remain outstanding.

Shared-action-bar metadata is now captured as required uses_shared_action_bars on every catalogue entry, validated as a boolean and roundtrip-tested. Confirmed in https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/SharedTraitsDocumentation.lua (TraitConfigInfo, read 2026-11-04). Missing metadata causes catalogue omission. This replaces the earlier outstanding metadata item; live action-bar contents and loadout switching remain unqualified.

Talent preparation compiler added in source. It binds the working config, selected saved ID, current committed import string, desired saved contents/name, spec, serialization version and shared-action-bar flag. Staged/unavailable current talents, absent targets, mismatched spec/version and transport over 8192 bytes are refused. AO-TALENT-1 has eighteen fields; no Lua decoder/executor or MCP exposure yet. Tests verify binding, changed target content producing a different operation ID and uncertainty refusals. Fixture capture timestamp uses UTC, not the local calendar date. Targeted tests and Clippy passed.

AO-TALENT-1 Lua decoding is now implemented and tested against the Rust compiler. The roundtrip preserves committed/desired build contents, selected config ID and shared-action-bar flag. Malformed build alphabet, noncanonical numeric fields and invalid boolean values are rejected. Targeted tests and Clippy pass. Decoder is not packaged or connected to Apply yet; no talent write is possible from this change.
