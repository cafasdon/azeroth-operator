# ADR-079: Passive player life-state flags

Manual Forever exports may capture player_life_state using UnitIsDead,
UnitIsGhost, UnitIsDeadOrGhost and UnitIsFeignDeath with the literal player
unit token only. The checked Forever unit API declarations describe Boolean
results for these getters. No roster, other unit, player GUID, corpse position
or third-party identity is queried.

The section has exact coverage reported_player_life_flags and four required
Boolean fields: is_dead, is_ghost, is_dead_or_ghost and is_feigning_death.
Two matching projections and stable family/getter/restriction-guard checks
are required. Missing, nil, restricted, non-Boolean, failing or changing reads
make the section unavailable; false is retained only when explicitly observed.
The existing global export guard still refuses combat. A malformed global
restriction guard may refuse the entire export through existing Core handling.

These are independent raw flags. The readers do not normalise their relations,
classify alive/dead, reinterpret feign death or infer an action requirement.
Stable repeated flags are not proof of atomic state, present life state,
resurrection eligibility, character continuity or native qualification.
Every Boolean combination remains representable as an observation. Unknown
section/field data is rejected by both strict readers, including personal
extras; Retail and Classic cannot contain this Forever-only section.

Existing owner-local storage, inspect_snapshot and capture-status discovery
apply. No public-index input, automatic monitoring or gameplay action is added.
The collector never calls release, corpse retrieval, resurrection, soulstone,
movement or other control APIs. No death/health interpretation modifies
inventory, progression, simulation inputs or current-character selection.
Update both readers before installing the exporter. Source support does not
establish Blizzard approval or native compatibility. Installation, deployment
and live game testing remain deferred.

Reference checked 10 November 2026:
[Mirrored Forever unit API declarations](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/UnitDocumentation.lua).
