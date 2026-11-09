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

Set `player_pvp_progression_changes` to `true` on `compare_snapshots` to see the
same rank and cap projections used by `character_summary` for each capture.
The comparison requires the same character and client in chronological order.
Missing observations return null. Season continuity remains unverified and the
earned-points delta stays unknown.

If both captures report a season ID, `reported_season_ids_match` says whether
the numbers agree. Older captures or unavailable season reads produce null.
Season continuity and earned points remain unknown even when the IDs match.

The offline build and three scoped MCP tests passed. The PvP fixture verified
that both comparison projections match their character summaries and that a
missing side stays null. Jev 1.14.0 advisory
`71fed3c939807c5b6af1f863d5928e6db480f7dcc1ed06025e59413dfa7bf6cd`
returned integrate .72 at confidence .58 with 407 input and 49 output tokens
in .300 seconds.
Independent review verified capture identity and ordering rejection, both missing
observations, and unknown earned points when the reported rank falls.

Use `minimum_pvp_rank` or `maximum_pvp_rank` on `list_snapshots` with
`family: forever` to find saved observations by reported rank. The bounds are
inclusive and accept zero. Captures without a PvP observation are excluded.
With `latest_per_character`, the latest capture in the requested date range
must satisfy the rank filter. Results carry `pvp_rank_match` as historical
evidence. Current progression remains unverified.

The offline build and two MCP tests passed. The PvP fixture covered zero and
maximum ranks, missing observations, latest-capture selection, reversed bounds
and unsupported clients. Jev 1.14.0 advisory
`d0408b4a34693801337102daa753d1ff12a3b1ac057cf20c93d51479c5b225c2`
returned integrate .63 at confidence .44 with 403 input and 49 output tokens
in .354 seconds.
Independent review also verified a two-sided inclusive range, paging and
rejection of malformed bounds.
