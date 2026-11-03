# Action-bar qualification gap

Investigated 2026-11-03 after the first live Retail export reported action_bars unavailable on build70688/interface120105.

The collector requires GetActionInfo and MAX_ACTION_BUTTONS. The fixture defines MAX_ACTION_BUTTONS=180, but this is a fixture input, not evidence of a real client constant. A bounded scan of the current Gethe/wow-ui-source live archive found no MAX_ACTION_BUTTONS definition or reference. Therefore the fixture cannot establish real slot enumeration.

Blizzard source mirrored at [ActionButtonUtil.lua](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_ActionBar/Shared/ActionButtonUtil.lua) defines NUM_ACTIONBAR_PAGES=6 and NUM_ACTIONBAR_BUTTONS=12. Its handling also distinguishes bonus, vehicle, temporary shapeshift and override pages. Their product is not evidence that the whole slot space is72. No hardcoded180/216 or truncated72-slot fallback was introduced.

[ActionBarFrameDocumentation.lua](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/ActionBarFrameDocumentation.lua) provides per-spell/flyout/pet searches and bar-index queries. These do not establish a complete enumeration of arbitrary item/macro/spell slots. No GetActionSlots/GetNumAction/MAX_ACTIONBAR/NUM_ACTION_SLOTS/MAX_ACTION_SLOT/NUM_ACTIONBAR_SLOTS match was found in the bounded current source scan. This is absence from that mirror, not proof no supported API exists.

Next implementation needs a build-specific supported enumeration contract or an explicitly partial export schema with coverage metadata. A partial visible-button observation must not silently replace full action-bar capture. Keep current unavailable behaviour until that contract is established. No action-bar mutations or UI frame execution are involved.
