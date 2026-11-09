# ADR-071: PvP cap distances from captured total thresholds

Date: 2026-11-09. Status: offline source integration.

Generic saved comparisons already show PvP field changes. Computing progression
across ranks requires total-point thresholds; subtracting within-rank points alone
would be incorrect. The pinned Camelot PVPRankFrame.lua in ADR-070 queries
GetTotalReputationForRenownLevel(2800, rank) for current, weekly and maximum ranks.
It adds the current rank's total threshold to within-rank earned points.

Extend player_pvp_progression with optional total_points_by_rank. Keys are canonical
u32 rank strings including zero, limited to the captured current rank or its three
reported caps. Values are u32 total points. At most four distinct ranks are queried
per projection, with two complete projections; matching ranks share one lookup.
Invalid/unreadable/changed values omit only that rank. A missing/replaced optional
getter omits the map; required namespace/family/guard changes still invalidate the
section. Unstable optional caps cannot leave unreferenced totals in the export.

Strict readers reject null, empty, oversized or unreferenced maps. Legacy captures
without totals keep their existing summary shape. When totals are present, summary
retains reported_total_rank_points (current threshold plus within-rank points) and
remaining_points_to_reported_caps. Each distance is max(0, cap threshold minus
current total). Zero or missing cap thresholds and missing current thresholds give
null. Zero is not interpreted as a rank/reward unlock; the UI treats a zero weekly
total as no reported maximum. No interpolation or season-wide ladder is invented.

These are historical arithmetic projections, not earned-points deltas, season
binding, guaranteed eligibility or a promise about today's caps. No scoreboard,
competitor identity, queue, reward action, refresh or automatic gameplay is added.
No native qualification, installation or deployment is included.
