# ADR-070: Passive personal PvP rank progression

Date: 2026-11-09. Status: offline source integration.

Forever's [pinned Camelot PvP rank UI](https://raw.githubusercontent.com/Gethe/wow-ui-source/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_UIPanels_Game/Camelot/PVPRankFrame.lua)
reads C_MajorFactions.GetMajorFactionProgressionInfo(2800) for personal rank,
points, next-rank threshold and progression caps. Existing reputation capture
does not call this API. Add a separate player_pvp_progression manual-export
section rather than assume a visible faction-list row covers this progression.

Required fields are coverage=reported_pvp_rank_faction, faction_id=2800, rank,
rank_points and next_rank_threshold. Optional max_rank, current_week_max_rank and
previous_week_max_rank retain reported caps. All numeric progression fields accept
zero through u32 maximum; no cap/rank relationship is guessed. Two complete
numeric projections must match for required fields. Optional invalid, restricted
or unstable values are omitted independently. Required failure omits the section.
Namespace, getter, family and restriction guard must stay fixed through capture.

Strict owner-local Rust/Python readers permit this section only for Forever.
MCP inspection returns the historical numeric observation. Summary computes
remaining_points_to_reported_next_threshold only when threshold is positive and
points do not exceed it; otherwise null. This does not establish rank-up
eligibility, rewards, current progression or a season-wide points requirement.

Read no scoreboard, opponent, team, names, reward descriptions or queue state.
Do not refresh data, open panels, queue, join, accept or initiate gameplay. Do not
infer atomic season binding from a separate season API. The reference gives
API usage, not native availability, policy approval or complete PvP coverage.
Live testing, installation and deployment remain deferred.

The collector may include `reported_season_id` using the positive season number
displayed by the pinned Forever PvP UI. It reads the season around both
progression projections and retains the ID only when both reads agree and the
getter stays stable. Missing, restricted, zero and changing values omit the ID.
Nine offline Lua export tests and the real stdio MCP fixture passed, including
legacy, equal-ID and different-ID comparisons and malformed season values.
Jev 1.14.0 advisory `4a632094cf77c648f376a10ff1faf456caacba6242c744e0a8dface896ff0d8a`
returned integrate .63 at confidence .44, with 429 input and 49 output tokens
in .307 seconds.
