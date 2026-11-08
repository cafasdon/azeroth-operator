# Reputation progression filtering — 8 November 2026

Scope: offline source and synthetic local stdio MCP. No new native API, external
database query, game action, installation, deployment, simulator execution or CI.
Broad coverage, full owned BiS and native qualification remain unfinished.

The existing `inspect_snapshot` adds optional nested `reputation_filter` for
Forever `reputation`, without an exact `faction_id`. Empty or incompatible
filters reject. Omission preserves existing whole-section and exact-faction
semantics; no route/profile addition is needed.

Fields:

- `kind`: standard, renown, friendship, paragon or unknown, matching the captured
  progression classification exactly.
- `account_wide`: independent captured boolean.
- `capped`: requires explicit standard, renown or friendship kind.
- `maximum_remaining_to_next_rank`: u64, requiring the same explicit rank kind.
  Standard reputation points, renown points and friendship standing are not
  silently mixed. A known capped faction has no next rank and does not match a
  distance filter, even a zero bound.
- `paragon_reward_status`: pending, pending_level_restricted or no_pending_reward;
  requires explicit paragon kind. These are captured reward states, not claim
  eligibility, current rewards or commands to turn in a quest.

The matcher reuses the existing standard/special progress maps, including their
range checks and unavailable states. Missing detail and inconsistent ranges
remain unknown when the requested predicate needs that progress. Known kind,
flag or progress mismatches defeat another unknown predicate for the same
faction. Kind-only/flag-only filters need no progress details. All supplied
predicates match one faction row.

Only definite matches remain in the section. `reputation_filter_unknown_count`
reports uncertain faction matches, or null for an unavailable section. The
visible-only coverage and full faction rows remain, and both derived progress
maps retain only matching faction IDs. An absent visible row is not inferred
unowned or unreached. Existing capped/rank derivations are historical estimates,
not proof of complete reputation or current progression eligibility.

The real MCP fixture includes standard/renown/friendship rank gaps and caps,
negative standing, inconsistent ranges, missing details, all three paragon
states, account-wide conjunctions, observed unknown kind, missing versus empty
sections, default/exact selection and invalid requests. It asserts both progress
maps contain no filtered-out factions and snapshot filenames/bytes stay unchanged.

Commands from the project root:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_reputation_filters test_achievement_filters test_quest_objective_filters -q
cargo fmt --check
git diff --check
```

Build passed, 22.25 seconds. Initial new MCP test passed, 0.549 seconds. Three
focused MCP tests passed, 1.255 seconds; formatting and whitespace checks passed.

Independent round-one reviews were configured as `gpt-6-astra`, high effort,
Callers and Failure/security. Per-review token usage was not exposed. Both were
clean; no source correction round was needed.

Callers ran `python -m unittest discover -s tests -p test_reputation_filters.py -v`:
one passed, 0.454 seconds. Its independent actual-stdio probe passed advertised
schema, null/omission equivalence, false predicates, inclusive/exclusive distance
bounds, exact equality of retained maps to filtered default maps, and unchanged
exact-selector response shape. Its initial synthetic fixture omitted mandatory
common row fields and was correctly rejected; the corrected fixture passed.

Failure/security ran the same discovery command with
`PYTHONDONTWRITEBYTECODE=1`: one passed, 0.583 seconds. Its inline actual-stdio
probe reported:

```text
PASS: 8 boundary/conjunction cases; 7 malformed filter cases; 3 store rejection/privacy cases; filtered derived maps; snapshot bytes unchanged.
```

Cases included zero remaining for all three rank kinds, u64 maximum, capped plus
distance contradictions, unknown plus account-flag mismatch, fractional/string/
boolean/array filters, unsupported keys, malformed stored rows, missing IDs and
traversal. Error responses did not expose private fixture text. Both reviewers
made no source, build, native-game or deployment changes and left protected old
temporary directories untouched.

Scoped advisory: `jev-1.14.0`, request
`ee236e8e65a1c1e8536a755a865be8d9ea44e16bca4415104356e9bd5e6fafd7`,
816 input / 52 output tokens, 0.321 s; land-scoped 0.96, fix-first 0.02,
insufficient evidence 0.02, confidence 0.94. Advisory only, not proof or permission.
