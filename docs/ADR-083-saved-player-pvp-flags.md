# Saved player PvP flags

Date: 10 November 2026. Status: source/offline only.

Add manual Forever export of three independent raw player flags from
`UnitIsPVP("player")`, `UnitIsPVPFreeForAll("player")` and
`UnitIsPVPSanctuary("player")`. The section `player_pvp_flags` contains exactly
coverage `reported_player_pvp_flags`, `is_pvp`, `is_free_for_all` and
`is_sanctuary`. All flags are required Booleans; none are inferred from another.
All eight combinations remain raw, including apparently inconsistent ones.

Require two matching projections and stable getter/family/restriction-guard
identities, including raw final checks after restriction callbacks. Missing,
restricted, nil, non-Boolean, error or changing data leaves the section
unavailable, never false. The existing whole-export combat refusal remains.
Only the literal player unit is queried. No zone name, location, GUID, other
unit, timer, flag toggle, combat right or gameplay action is read or issued.

Both readers enforce the exact Forever-only shape. Existing section inspection
and discovery work without extra collectors or dependencies. Character summary
adds `player_pvp_flag_observation`, preserving missing as null and observed
false as false. `list_snapshots.player_pvp_flags` accepts a nonempty set of the
three optional exact Boolean predicates; false matters, null predicates are
ignored, and empty/all-null filters reject. All predicates match one capture
after latest-per-character selection; stale matches cannot replace newer data.

`compare_snapshots.player_pvp_flag_changes: true` returns separate observations
and raw differences. A missing side leaves changes/count null; equal recorded
flags yield an empty difference map and zero. Existing owner/character/client/
time gates apply. Omitted/false leaves comparison output unchanged. No current
state, character continuity, zone safety, combat eligibility or PvP event is
verified or inferred. Data stays owner-local and outside public indexing.
Readers must be updated before exporter installation. Deployment and native
qualification remain deferred.

## API evidence

The inspected Forever generated unit documentation at commit
[`9dc84f0d3bbae1768ac3405b3716000bd93eda06`](https://github.com/Gethe/wow-ui-source/commit/9dc84f0d3bbae1768ac3405b3716000bd93eda06)
(7 November 2026 04:46:13 UTC) declares these Boolean returns and unit arguments
at [lines2224–2269](https://github.com/Gethe/wow-ui-source/blob/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/UnitDocumentation.lua#L2224).
It marks `UnitIsPVP` as potentially restricted by unit identity. These source
declarations do not qualify installed native behaviour or policy approval.
The pinned file's SHA-256 is
`93004ac1b260127741471b7861360cdf7536afcb18d4c1db73b7259faa43e61c`.
Only the source declaration was downloaded and inspected; none was executed.
