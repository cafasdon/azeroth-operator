# ADR 048: passive carried-item cooldown observations

Date: 9 November 2026. Status: source and offline qualification only.

Carried-item maintenance needs more than item IDs and stack counts. Observe
public container cooldown data during the existing manual, out-of-combat
Forever export. Keep this in a separate `bag_item_cooldowns` section so existing
item records, equipment projection and operation preimages remain unchanged.

The pinned generated [Container API](https://raw.githubusercontent.com/Gethe/wow-ui-source/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/ContainerDocumentation.lua)
declares `GetContainerItemCooldown(containerIndex, slotIndex)` returning numeric
start time, duration and enable. Only these read APIs, item-info identity reads
and the public clock are used. No item activation, cursor, target, bank access,
automatic export or event-driven gameplay is introduced.

The section has coverage `captured_carried_items_only`, a captured location
count capped at 4096, and entries with exactly the same bag/slot keys as the
successful captured bags section, including empty bag maps. Each row contains
the matching item ID and status. `reported` requires boolean `enabled` and
unsigned 32-bit `remaining_seconds`; `unavailable` carries no timer fields.
Both readers require exact source keys/IDs, counts, shape, bounds and Forever.
Missing source bags makes a present section invalid. Missing section stays
unknown, separately from a successful empty section.

The collector freezes original item ID/link/count, validates public finite
numeric reads, and reads each cooldown twice with exact identity checks before
and after. API/clock/context pointers must remain consistent. Raw start,
duration and enable plus rounded remaining time must agree. Remaining is
ceil(max(0, start + duration - observed clock)); the raw uptime values are not
exported. A changed rounding boundary, item identity, API, secret or malformed
read leaves an unavailable row. Entirely absent APIs make the section unavailable.
No unsupported value becomes a zero timer. Source links/GUIDs/names and raw
clock values are not duplicated into the new section.

`character_summary.bag_item_cooldown_observation` reports readable, unavailable,
disabled and positive-timer location counts. `current_usability_verified` and
`game_execution` remain false. A zero timer does not establish item usability;
disabled timers are preserved without interpretation. Cooldown observations
remain bound to capture time, not a live countdown or shared/global-cooldown
classification. Existing observed bag quantities remain the sole inventory
input; this section does not add stock or simulator effects.

Item lookup projects `captured_cooldown` only onto matching counted bag locations
from that same section/snapshot, using exact container, slot and item ID.
`bag_cooldown_observation` counts matching bag locations before pagination.
Equipment and bank locations receive no join; multiple explicit snapshot
results remain separate with their original capture time. If the section is
absent, both new projections are omitted, preserving the legacy lookup shape.

Filtered inspection uses optional `bag_cooldown_filter` only with the Forever
`bag_item_cooldowns` section. Select 1..64 distinct positive item IDs, status
reported/unavailable, enabled flag and inclusive unsigned timer bounds. Offset
and a 1..50 limit (default 20) allow pagination-only requests. An empty filter,
reversed bounds, duplicate IDs, invalid types or wrong context reject.

Predicates apply to the same row. Any definite mismatch dominates an unknown
timer/enabled predicate; otherwise missing requested values remain unknown.
Unavailable status itself is known and can be selected without timer predicates.
Counters partition the captured locations before paging. Output rows use numeric
bag/slot order, and counts remain unchanged when paging past the end. Missing
sections have null rows/counts/has_more; observed empty sections have empty rows,
zero counts and false has_more. No filter preserves the original raw shape.
Recorded enabled/zero-timer matches are not current-use eligibility.

Matching new MCP readers and addon 0.46.0-dev are required to consume the new
capture. Older captures remain accepted. The package is not installed or
deployed. Native API response/completeness, current identity and in-game
behaviour remain unqualified; this is not an absolute policy-compliance claim.
