# Saved quest turn-in groups

Date: 10 November 2026. Status: source/offline only.

Use the previously captured quest-giver references to group saved quests by
retained provider finisher NPC/object IDs. This is a review of historical data,
not a route optimiser or a turn-in executor. It needs no new collector, provider
query, database sync, policy permission or installed dependency.

Add optional `inspect_snapshot.quest_turn_in_review` on section
`quest_giver_references`. With the option omitted/null, raw inspection is
unchanged. With a valid page, replace `data` with the review; retain snapshot
identity, capture time and section availability in the existing wrapper.
Require Forever and reject coverage-only inspection or any other section.
Read from the same validated owner-bound snapshot; no newest-character guess.

Optional distinct positive `quest_ids` (1..64) and exact Boolean
`ready_for_turn_in` select quests. A missing ready flag is unknown, including
when selecting false. Report unknown-ready count after the ID selection and
before readiness filtering. IDs absent from saved quests do not create rows.
Optional `giver_kind` is `npc` or `object`; an exact positive `giver_id`
requires its kind. Giver filters restrict edges, not the selected quest count.

Group by kind (NPC first) and numeric giver ID, with numeric quest IDs and
separate true/false/unknown readiness ID lists. Deduplicate IDs; group counts
are computed before paging. Offset is uint32 and limit 1..50, default 20.
Keep selected reference-status counts, including quests outside the captured
64-ID prefix as `not_captured`. Selected quests without a retained edge matching
the giver filters are ungrouped, with full count and at most 64 numeric IDs.
These may be unreadable, absent from the provider, omitted by bounds, explicitly
empty or simply filtered out; ungrouped is not a claim of no finisher.
Count truncated reference categories after quest/kind filters, before exact
giver-ID filtering. A zero-match query can still have omitted unknown edges.

Missing source quests or references produce explicit status and null groups
and projection counts; known empty captures produce empty groups and zero.
No names, quest text, coordinates or private errors are added. Current
readiness, native identity, turn-in eligibility, complete finisher coverage and
optimal routing remain unverified. No gameplay action is emitted or executed.
Installed enablement, deployment and native testing remain deferred.
