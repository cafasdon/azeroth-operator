# Tracked achievement filtering — 8 November 2026

Scope: offline source and synthetic local stdio MCP only. No native API change,
external data query, game action, installation, deployment, simulator or CI.
Broad system coverage, full owned BiS and native qualification remain incomplete.

`inspect_snapshot` now accepts an optional nested `achievement_filter` for the
Forever `tracked_achievements` section. It must contain at least one predicate
and cannot combine with `achievement_id`. Other sections and client families
reject. Existing exact selection and omitted-filter responses retain their
meaning; no MCP route/profile addition is required.

Predicates are `completed`, `earned_by_character`, `account_wide` (booleans),
`criterion_id` (positive u32), `criterion_completed` (boolean) and
`criterion_max_remaining` (u32). Achievement flags conjoin independently; account
completion does not imply character credit. All supplied criterion predicates
must match the same criterion row. Remaining units saturate `required - quantity`
at zero; zero does not infer the criterion's completed flag.

Missing IDs or quantities on a potentially matching criterion leave its outcome
unknown. A known mismatching predicate defeats other missing observations on
that row. A definite witness wins over unknown sibling criteria. Empty observed
criteria are a known nonmatch when criterion predicates are requested. Flag-only
filters do not require any criterion. Only definite matches are retained.

`achievement_filter_unknown_count` counts unknown achievements for the combined
filter; it is null when the section is unavailable. The existing tracked-only
coverage stays in the returned section. Entire matching achievement rows remain,
and `tracked_achievement_work` is recalculated for those achievements, including
all their unfinished criteria rather than only the criterion that matched.
Missing sections stay null; absent/untracked achievements do not become known
incomplete or known unearned. No completion, claim or gameplay action is emitted.

The actual MCP test covers cross-row false conjunctions, missing IDs/counts,
known false dominance, witness precedence, saturation, independent account and
character credit, filtered work, missing versus empty sections, legacy/default
selection, invalid types/ranges and section/family/selector conflicts. It checks
that owner-store filenames and bytes remain unchanged.

The first author test caught a source bug: mutable JSON indexing created an
`entries: null` wrapper when the section was unavailable. The fix uses a
non-inserting lookup. The same missing-data assertion then passed; no fixture
expectation was relaxed.

Direct commands:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_achievement_filters test_quest_objective_filters -q
cargo fmt --check
git diff --check
```

Initial build: 20.67 seconds; corrected-source build: 5.35 seconds. Two focused
MCP tests passed, 0.688 seconds. Formatting and whitespace checks passed.

Independent round one used Callers and Failure/security reviews configured as
`gpt-6-astra`, high effort. Per-review token usage was not exposed. Both were
clean; no reviewer finding required a correction round.

Callers ran `python -m unittest discover -s tests -p test_achievement_filters.py -v`:
one test passed, 0.329 seconds. Its independent local stdio probe confirmed
whole-row retention, both matching and nonmatching unfinished sibling criteria
in derived work, tracked-only provenance, explicit-null/default behaviour,
absent exact-ID semantics and unchanged store bytes. Advertised schema included
optional nested fields, unknown-property rejection, booleans and positive/
nonnegative integer constraints.

Failure/security ran:

```text
python -B -m unittest discover -s tests -p test_achievement_filters.py -v
Ran 1 test in 0.358s — OK
```

Its independent `PowerShell @'…'@ | python -B -` probe, seed 20261108, returned:

```text
PASS: 1295 filter combinations x 32 achievements matched independent three-valued truth tables; 23 malformed snapshot/argument/selection/path cases rejected without fixture identity/path echo; store bytes unchanged.
```

The reference explicitly used Kleene AND/OR, covering predicate presence,
integer boundaries, empty criteria, missing IDs/counts, contradictory captured
flags, witness precedence and false dominance. Neither reviewer edited source,
built, accessed the game, deployed or made network calls. Previously protected
temporary directories stayed untouched.

Scoped advisory: `jev-1.14.0`, request
`3ac18f0ee5be15d1356a683683eec5f2e25637cc586c57e0811fbc709de4c04b`,
855 input / 52 output tokens, 0.294 s; land-scoped 0.98, fix-first 0.01,
insufficient evidence 0.01, confidence 0.97. Advisory only, not proof or permission.
