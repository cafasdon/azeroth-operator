# Private saved player map position and target geometry

Date: 10 November 2026. Status: source/offline only.

Add Forever-only `player_map_position` with exact fields coverage
`reported_player_map_position`, positive uint32 map_id and finite x/y in [0,1].
Both readers reject extra/missing fields, wrong families and nonnumeric values.
Missing or failed observation remains unknown; zero coordinates are reported
values, not a fallback. This is private owner-local character data, never a
public knowledge record or a verified current/active location.

The manual exporter reads `C_Map.GetBestMapForUnit("player")` and
`C_Map.GetPlayerMapPosition(map,"player")`, then public Vector2DMixin GetXY.
Two complete readings must agree, with stable map at each boundary and fixed
family, restriction getter and map API identities. Vector getter readings must
agree. Restricted, nil, invalid or changed observations refuse the section.
The existing outside-combat export gate remains. No other unit, polling,
map opening/preload, waypoint setter, world-position lookup or movement action
is introduced. These boundary checks do not prove uninterrupted continuity.

The pinned primary API declaration is
[MapDocumentation.lua](https://github.com/Gethe/wow-ui-source/blob/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/MapDocumentation.lua).
It declares nullable map ID and vector returns. This does not qualify actual
Forever availability, acceptance or behaviour. Cache copy SHA-256:
b73a2ed1d1552c1b38bc59df0a586a0986dfad54991efc905abc87799cecce57.

Existing raw inspection supports the section. Forever character_summary adds
player_map_position_observation with capture status and raw position or null,
plus false current/active/distance/route/execution flags.

Optional inspect_snapshot.player_map_target accepts exact map_id/x/y and needs
section player_map_position without coverage_sections. It replaces data with a
saved target preview. Unknown positions preserve not_recorded or
reported_unavailable; different maps return no geometry. On the same map,
diagram_delta reports target-minus-saved x/y and Euclidean separation in
normalised map axes. The axes need not share a physical scale. This value is
neither yards nor a travel distance, path, arrival, accessibility or safety
claim. Zero separation is not proof of arrival or current position. No target
is written back to the game. Invalid target errors do not echo their values.

Update readers before any exporter installation. Source/offline tests do not
establish native capture or route acceptance. Installation, deployment and live
tests remain deferred.
