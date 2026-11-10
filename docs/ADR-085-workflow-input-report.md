# Saved workflow input profiles

Date: 10 November 2026. Status: source/offline only.

`character_summary` accepts optional boolean `include_workflow_coverage`.
Omission and false preserve the existing response. True adds a Forever-only
`workflow_coverage` object for the exact requested owner-local snapshot ID.
Null and nonboolean values fail without echoing their value.

The report defines 24 curated saved-input profiles across gear, professions,
quests, travel, commerce, mail, pets, spells, collections, progression,
instances, PvP, groups, loot and trade. These are neither an exhaustive game
feature count nor minimal invocation prerequisites. Tools can accept partial
inputs. An observed section can be empty, filtered or insufficient for a task.

Each profile distinguishes observed, reported-unavailable and not-recorded
sections; lists missing required inputs and helpful inputs; and names a read
entrypoint, additional argument needs and unchecked conditions outside the
snapshot. The owned-gear entrypoint needs local catalogue configuration and
can be absent from tools/list without it. Metadata does not invoke these tools.

All readiness, execution, complete ownership, active identity, native eligibility
and complete game coverage flags remain false. Status depends only on section
presence, not payload semantics. No raw payload or unavailable reason enters
the new report. The existing authorised summary is unchanged outside this field.

Snapshot recency is not active identity. `resolve_character` without an explicit
character still returns active_identity_unavailable. The current SavedVariables
protocol has no proven fresh active signal; this feature does not replace that
unfinished requirement. No game action, collector, network sync or public
index write is added. Installation and live acceptance remain deferred.
