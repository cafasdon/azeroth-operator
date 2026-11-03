# Action-bar qualification gap

Investigated 2026-11-03 after the first live Retail export reported action_bars unavailable on build70688/interface120105.

The collector requires GetActionInfo and MAX_ACTION_BUTTONS. The fixture defines MAX_ACTION_BUTTONS=180, but this is a fixture input, not evidence of a real client constant. A bounded scan of the current Gethe/wow-ui-source live archive found no MAX_ACTION_BUTTONS definition or reference. Therefore the fixture cannot establish real slot enumeration.

Blizzard source mirrored at [ActionButtonUtil.lua](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_ActionBar/Shared/ActionButtonUtil.lua) defines NUM_ACTIONBAR_PAGES=6 and NUM_ACTIONBAR_BUTTONS=12. Its handling also distinguishes bonus, vehicle, temporary shapeshift and override pages. Their product is not evidence that the whole slot space is72. No hardcoded180/216 or truncated72-slot fallback was introduced.

[ActionBarFrameDocumentation.lua](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/ActionBarFrameDocumentation.lua) provides per-spell/flyout/pet searches and bar-index queries. These do not establish a complete enumeration of arbitrary item/macro/spell slots. No GetActionSlots/GetNumAction/MAX_ACTIONBAR/NUM_ACTION_SLOTS/MAX_ACTION_SLOT/NUM_ACTIONBAR_SLOTS match was found in the bounded current source scan. This is absence from that mirror, not proof no supported API exists.

Next implementation needs a build-specific supported enumeration contract or an explicitly partial export schema with coverage metadata. A partial visible-button observation must not silently replace full action-bar capture. Keep current unavailable behaviour until that contract is established. No action-bar mutations or UI frame execution are involved.

## Explicit partial observation implemented — 2026-11-04

New source-only section action_bar_observation records coverage=loaded_builtin_buttons_only, observed_buttons, missing_buttons and a map of observed slots. Each slot has empty=true, or empty=false with action kind/id/subtype. It uses Blizzard ActionButtonUtil.ActionBarButtonNames, NUM_ACTIONBAR_BUTTONS and each loaded named frame's action field, matching the lookup pattern in ActionButtonUtil.GetActionButtonBySpellID (primary source above, re-read 2026-11-04). Reads only; no frame clicks or protected attribute changes.

The existing action_bars section remains unchanged. Missing loaded frames are counted; no observed frame means unavailable. Duplicate slot references are queried once. Hidden pages, custom replacement bars and pet/stance/special bars are not claimed. Slot ceiling4096 and total button ceiling2048 are defensive ingestion/work limits, not game slot contracts. Restricted values fail the section. This observation is groundwork for explicit slot preimages, not action-bar mutation or a complete layout backup.

Actual Lua-producer-to-Rust roundtrip tests cover occupied and empty slots, missing frames, duplicate references, no MAX_ACTION_BUTTONS, restricted values, unsupported family and malformed coverage/count/data. Full source gate passed: format/build, Rust tests, Clippy and31Python tests. Not packaged/deployed; hosted importer/reader and companion must accept the new section before installation.

Jev advisory request e12d3a6f50b9a16ff2eb39c11681ea14d8cd810c736f7cd5088909b37c1625d6, jev-1.14.0,922input/116output tokens,0.289seconds: insufficient_evidence (reported confidence0.30), not a pass. Real client semantics remain unqualified. Configured adversarial-review workflow file is absent.
