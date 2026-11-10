# ADR-078: Passive battlefield queue slot observations

Manual Forever exports may read GetMaxBattlefieldID and the first return of
GetBattlefieldStatus for each reported slot. The mirrored Forever queue UI uses
these same status getters. This is an out-of-combat, player-requested export,
not a queue workflow or a polling task. No UI module is loaded or shown.

The new battlefield_queue_slots section contains coverage
reported_battlefield_queue_slots, reported_slot_count (0..32) and a contiguous
entries map containing that many rows. Each row has only reported_status:
none, queued, confirm, active or locked. These are allowlisted raw tokens;
unknown tokens make the section unavailable instead of being guessed. The
slot count describes the getter's capacity, not the number of active queues.
A zero-slot observation is distinct from unavailable data, and a none token
is distinct from a missing row. Every reported slot is retained, including none.

The collector requires two identical status projections with matching capacity
and checks the family, restriction guard and getter identities around reads.
Changing, missing, restricted, malformed or over-budget data makes the section
unavailable. Agreement is not atomicity, freshness, current queue membership
or native qualification. The limit is an exporter bound, not a game limit.
Names, queue identifiers, timers, teams, group members, scores and all other
returns are excluded. No scoreboard query, join, accept, leave, refresh, hook,
role assignment or other action is invoked. Native API availability remains
unverified. It does not cover dungeon finder, world PvP, rated distinctions,
social listings or every possible queue system.

Both snapshot readers enforce the same exact fields, family, count, slot and
token contract. Existing owner-local storage, inspect_snapshot and section
capture-status discovery apply. Data is not a public knowledge input. Saved
confirm/active tokens never authorise an action or prove current eligibility.
No queue-management or automation route is enabled by this source feature.
Update readers before installing the exporter. Installation, deployment and
live game testing remain deferred. No Blizzard approval is claimed.

Reference checked 10 November 2026:
[Forever queue-status UI source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_QueueStatusFrame/Mainline/QueueStatusFrame.lua).
The source's slot loop and first-return status comparisons establish this
narrow data projection, not a full return contract or game-client acceptance.
[Forever-specific overrides](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_QueueStatusFrame/Camelot/QueueStatusFrameOverrides.lua)
also show why other queue systems must be qualified separately.

Saved summary (10 November): character_summary.battlefield_queue_observation
reports the captured capacity, per-token counts and numerically sorted slot
lists for all five allowed status tokens. reported_non_none_slot_count counts
every token other than none; it is not a queued/active player count. An absent
section leaves the summary null. Zero-capacity observations retain zero counts
and empty lists, while all-none observations preserve their nonzero capacity.
The output explicitly leaves current membership, complete queue coverage,
queue identity and acceptance eligibility unverified, with execution/live false.
It uses saved data only and does not invoke another getter or action.

Historical search (10 November): list_snapshots accepts battlefield_queue with
required reported_status (one of the five exact raw tokens), optional slot
(1..32) and minimum_matching_slots (1..32, default one). It counts matching
saved slots after any exact-slot restriction. A threshold above one combined
with an exact slot cannot match, rather than implying additional hidden slots.
Missing/zero/nonmatching sections do not satisfy the predicate. No none token
is invented for unobserved slots. Null outer filter preserves default listing.

Latest-per-character selection precedes this feature filter; an old matching
capture is not substituted when the selected newer capture is empty, missing
or nonmatching. Existing explicit capture-time and composition predicates
remain available. Matching metadata preserves source capacity and coverage,
full matching count and every matching slot in numeric order (at most 32).
It explicitly withholds current membership, full queue coverage, queue identity,
acceptance, live and execution verification. Strict redacting parsing rejects
unknown fields and invalid types/tokens/bounds. Owner/family gates remain.
This adds no capture, queue action, timer or public-data indexing path.
