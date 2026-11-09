# ADR-057: Saved skill-line discovery and inspection

Date: 2026-11-09. Status: accepted for offline source integration.

## Decision

Expose existing Forever `skill_lines` observations through two optional MCP
arguments. No collector, snapshot schema or installed addon changes are needed.

`list_snapshots.skill_line` takes a predicate object. `inspect_snapshot` with
`section: "skill_lines"` takes `skill_line_filter: {filter: {...}, offset: 0,
limit: 20}`. The filter supports optional `skill_line_id`, `category_id`,
`minimum_rank`, `maximum_rank`, `minimum_cap`, `maximum_cap`,
`minimum_points_to_cap` and `maximum_points_to_cap`. At least one is required.
IDs are positive u32 values; numeric bounds are u32, inclusive and ordered.
Unknown fields and malformed values reject with fixed argument errors.

All predicates apply to the same recorded row. Rank means base `rank`, cap means
`max_rank`, and points to observed cap means their difference. Optional modifiers
and temporary points remain in the raw observation without addition to base rank.
Missing category is unknown only when requested and no known predicate is false.
Known false dominates unknown. Missing rows do not imply unlearned skills.

Discovery requires at least one true row and returns `skill_line_match` with
captured/matching/unknown/nonmatching counts, the first 20 matching rows ordered
by numeric skill ID, and `matches_truncated`. Counts precede truncation.
Existing character/date/storage/profession predicates compose without pooling.
When requested, latest-per-character selection runs before skill filtering;
a newer valid empty or missing section suppresses an older matching capture.

Inspection returns matching rows in numeric order. Offset defaults to zero and
limit to 20, with limits 1..50. Counts precede paging; large offsets safely return
an empty page. Missing sections return null rows/counts/has_more; known empty
sections return empty rows, zero counts and false has_more. The existing strict
section reader caps observations at 512 rows. Explicit null arguments preserve
the old unfiltered behaviour. Other inspection review modes cannot be combined.

## Boundaries

This is historical, owner-local observation. Discovery includes capture timestamps;
inspection retains the existing age metadata. Neither establishes the current
character, complete skill coverage, training eligibility, effective rank,
learning state, equipment eligibility or simulator accuracy. An observed zero cap
is merely a recorded numeric value. Collapsed or partial capture remains partial.
No skill classification is inferred from IDs or categories. No training, game
execution, simulator input change, network sync or deployment occurs.

## Verification

See [offline evidence](skill-line-filters-verification-2026-11-09.md).
