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
