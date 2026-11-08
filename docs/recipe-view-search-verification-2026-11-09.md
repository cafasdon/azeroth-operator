# Recipe-view predicates in bulk discovery — 9 November 2026

Base source: `6381836`. The existing read-only `search_forever_recipes` route
accepts optional `recipe_view_filter` with `learned`, `can_skill_up` and
`minimum_skill_ups_reported`. At least one non-null predicate is required.
Minimum zero requires an actual recorded numeric value; missing is not zero.

This joins exact recipe IDs from bulk materials to the same snapshot's filtered
recipe view only when profession IDs match. Requested predicates use
three-valued AND: any explicit mismatch is nonmatching; otherwise missing data
is unknown; all known matches pass. The view observations are not aggregate
learning evidence across sources. Existing exact recipe review can still
reject conflicting selected or tracked learning. No skill-up probabilities,
guaranteed gains or profession progression sequence is invented.

ID/output/reagent/status filters run first, then view predicates, then optional
material support, then numeric pagination. Separate unknown/nonmatching view
counts apply to rows passing the first stage. Material counts apply only after
the view stage. Missing bulk observations yield null counts; observed-empty
bulk observations yield zero. Missing/empty/incompatible views make applicable
bulk rows unknown. Default omission/null and both earlier public Rust search
helpers remain compatible.

Records project only source label, learned flag, skill-up flag, reported points
and relative difficulty. Names are excluded. Coverage states whether a view
was captured and its profession matched; verified learning and skill gains stay
false, alongside existing false craftability and game-execution flags.

## Author evidence

```text
cargo build --locked --offline
Finished dev profile in 22.33s
cargo fmt -- --check
exit 0
```

Actual local Rust stdio MCP checks used synthetic owner snapshots, no catalogue
or game process:

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_recipe_view_search test_recipe_material_search test_recipe_search -v
```

Four tests passed in **2.268 seconds**. The new test constructs 27 view rows
covering every combination of missing/false/true learned and skill-up flags,
and missing/zero/two reported points, plus one unsupported material row. A
separate Python truth table checks 35 nonempty view predicate combinations,
each with and without material support filtering. Every returned page, numeric
order, matched count, staged unknown/nonmatch/material count and projected field
is checked against that reference. Private recipe and selected-schematic names
remain absent. Capture bytes stay unchanged.

The same test verifies missing, empty, wrong-profession and partial views;
missing/empty bulk lists; null versus omitted filters; narrowed exact IDs;
rejection of empty/all-null/invalid filters; and the separate planner's rejection
of an explicit false selected learning flag even when its view reports true.
It uses the established `AZEROTH_TEST_BINARY`/platform path convention.

```text
python -m unittest test_recipe_skill_up test_bulk_recipe_planning -v
Ran 2 tests in 2.067s
OK
```

These checks cover existing skill-up listing and exact/batch recipe review
callers. No native capture change, external lookup, live test, installation,
deployment or CI was performed. Full profession knowledge, current eligibility
and reliable levelling progression remain unfinished.

## Independent review

Two independent `gpt-6-astra` reviewers at high effort covered Callers and
Failure/security. Neither confirmed a defect; no correction round was needed.

Callers traced schema/router/read validation, both preserved public helpers,
filter staging and separate planner gates, and ran:

```text
python -m unittest discover -s tests -p 'test_recipe*search.py' -v
Ran 4 tests in 3.300s — OK
python -m unittest discover -s tests -p 'test_bulk_recipe_planning.py' -v
Ran 1 test in 0.664s — OK
```

Independent actual-MCP probes passed schema exposure, combined exact
ID/output/reagent/status filters, false over unknown, maximum u32 observed
thresholds, null unused predicates, recorded zero points, no-required-items
support with no bags, view-only IDs not expanding the bulk search domain and
maximum usize offset retaining counts while ending pagination. Private names
were absent. Documentation was added during the review and subsequently checked
for the option in README, interaction map and this evidence record.

Failure/security passed the new test in 1.152 seconds. Its independent actual
MCP probe checked 4,096 joined rows with maximum u32 recipe ID and reported
points, including an exact final one-record page; maximum usize offset; seven
additional malformed filter shapes; and hash-valid, schema-invalid captures
with missing profession, noncanonical recipe IDs and an injected private field.
All malformed inputs rejected. An exact output-ID mismatch left view counters
zero, confirming the staging boundary. Names were absent. The 4,096-row
boundary query took 0.142 seconds in that probe, not a general latency guarantee.
Reviewers made no source/build/Git changes or native game/network/deployment
calls. Temporary synthetic probes were cleaned and no SQLite was opened.

Scoped Jev advisory: `jev-1.14.0`, request
`22fc4ccfa32a2802bde95944368af0cea87144a83701e5918da1e34ab19a8a1e`,
890 input / 54 output tokens, 0.305 seconds; land-scoped 0.80, fix-first 0.19,
insufficient-evidence 0.01, confidence 0.70. The advisory is recorded without
rerolling; direct scoped evidence and clean independent reviews support this
source increment. It is not proof, permission or broad profession qualification.
