# Saved currency filtering — 8 November 2026

Scope: offline source and synthetic local stdio MCP. No new native API, external
database query, gameplay action, installation, deployment, simulator or CI.
Broad system coverage, full owned BiS and native qualification remain unfinished.

`inspect_snapshot` adds optional nested `currency_filter` for Forever
`currency_balances`. It requires at least one predicate and excludes exact
`currency_id` selection. The existing exact currency reward projection remains
separate; supplying a reward quantity without its exact currency ID still
rejects. Wrong sections/families and spending-budget conflicts reject. Omitted
filters preserve existing response shape; no new route/profile is needed.

Fields `account_wide`, `inactive` and `can_earn_per_week` match captured booleans.
`has_weekly_cap` and `has_total_cap` test whether the corresponding observed cap
counter is positive. `maximum_weekly_remaining` and `maximum_total_remaining`
are independent u64 inclusive bounds against the existing derived headroom.
All predicates conjoin on one currency; no quantities are summed across IDs.

Weekly headroom saturates cap minus earned-this-week at zero. Total headroom
uses quantity or total-earned according to the captured `cap_uses_total_earned`
flag, preserving the existing `maximum_basis`. A zero recorded cap means no
positive cap to compare, so it never matches a numeric headroom bound, even zero.
It is not silently treated as zero remaining. Positive cap presence alone is
not proof of a particular reset schedule or current game availability.

Missing weekly-earning flags remain unknown when requested; they are not false.
A known mismatching predicate on the same currency defeats an otherwise unknown
predicate. Only definite matches remain. `currency_filter_unknown_count` reports
unknown matches, or null for an unavailable section. Observed empty sections
remain empty with zero unknowns. Visible-only coverage and full rows remain;
`currency_cap_headroom` keeps only retained IDs, with no fabricated headroom for
rows lacking a positive cap. No earning, exchange or spending command is emitted.

The actual MCP fixture checks total-earned versus current-quantity basis,
weekly/total conjunctions, positive cap presence, zero caps, over-cap saturation,
known false dominance, missing flags, account/inactive predicates, empty/missing
sections, default/exact selection and malformed/conflicting requests. Saved
owner-store filenames and bytes remain unchanged.

Commands from the project root:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_currency_filters test_reputation_filters test_achievement_filters test_quest_objective_filters -q
cargo fmt --check
git diff --check
```

Build passed, 20.41 seconds. New direct MCP test passed, 0.548 seconds. Four
focused MCP tests passed, 2.016 seconds; formatting and whitespace checks passed.

Independent round one used Callers and Failure/security reviews configured as
`gpt-6-astra`, high effort. Per-review token usage was not exposed. Both were
clean; no source correction round was needed.

Callers ran `python -m unittest discover -s tests -p test_currency_filters.py -v`:
one passed, 0.460 seconds. Its independent Python-stdin real-stdio probe verified
advertised nested schema/unknown-property rejection, omitted/null equivalence,
false versus missing earning flags, lifetime-earned cap basis and zero headroom,
matching-only derived IDs, unchanged exact reward projection, u64 maximum
excluding zero caps, and fractional predicate rejection. For its independent
fixture, earning=false retained only ID 2 with three unknowns; zero total
remaining retained only ID 3 on total-earned basis. Exact selection plus reward
quantity one preserved the existing projection (one above recorded headroom).

Failure/security ran `python -m unittest test_currency_filters -v`: one passed,
0.511 seconds. Its independent bounded Python-stdin real-stdio probe returned:

```text
PASS 26 real-stdio probes: u64 max/overflow, saturating caps, null/missing eligibility, false dominance, guards, malformed snapshots, owner scope, error redaction, byte immutability; temporary stores cleaned.
```

Both reviewers used the existing executable without source/build/Git changes,
network calls, installation or game activity. Their own temporary synthetic
stores were cleaned; previously protected directories stayed untouched. Error
privacy probes concern stored fixture identity/path data, not user-supplied
predicate spelling in parameter-validation messages.

Scoped advisory: `jev-1.14.0`, request
`f03b5c42e09789c10eac846a1bd35f78591efa37f77a9b08a96475f744429e4f`,
834 input / 52 output tokens, 0.334 s; land-scoped 0.97, fix-first 0.02,
insufficient evidence 0.01, confidence 0.96. Advisory only, not proof or permission.
