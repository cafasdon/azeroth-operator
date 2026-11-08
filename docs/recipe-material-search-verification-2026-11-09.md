# Material-supported recipe discovery — 9 November 2026

Base source: `55834d9`. The existing read-only `search_forever_recipes` now
accepts optional `material_filter` with required `scope` (`bags` or
`all_observed_storage`) and positive u32 `minimum_crafts`. It aggregates chosen
inventory once, then tests fixed reagent rows independently, without invoking
full planners repeatedly. The legacy public Rust helper and no-filter result
shape remain available. No snapshot schema, capture API or gameplay path changes.

Material support is a lower bound from observed item quantities, not learning,
craftability or complete ownership. Rows with unavailable/unsupported/budget
material states, or nonempty requirements with no observed selected storage,
are unknown. Explicitly empty fixed requirements match with null finite capacity
and `no_required_items: true`. Missing sections differ from observed-empty maps.
Unknown and insufficient-observed counts apply after existing same-row filters,
before paging. Each row uses the same stock independently; matching recipes do
not form a jointly affordable batch. Equipment, pending mail and merchant offers
do not become material stock.

## Author evidence

```text
cargo build --locked --offline
Finished dev profile in 8.36s
cargo fmt -- --check
exit 0
```

The rebuilt debug MCP was exercised directly through the Python MCP SDK:

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
$env:AZEROTH_TEST_BINARY='T:/Projects/Azeroth Operator/target/debug/azeroth-operator.exe'
C:/Python314/python.exe -m unittest test_recipe_material_search test_recipe_search test_bulk_recipe_planning -v
```

Four tests passed in **2.344 seconds** after the harness correction below.
Tests assert bags/bank totals, per-recipe limiting reagents, positive minimums,
missing/empty/partial observations, free-required-material rows, same-row filters,
numeric paging, unknown counts, planner agreement and unchanged capture bytes.
A row with an explicit false selected learning flag still matches its independent
material support, while exact recipe review rejects its learning conflict.
Craftability and complete-ownership flags stay false. Stored private marker
values are absent from results; a rejected extra-field probe omits its marker
value as well.

The largest fixture has 4,096 rows and exactly 16,384 reagent pairs. Bags and a
bank each supply u32-max stacks; returned capacities are exactly **8,589,934,590**,
without u32 truncation. First/last pages and complete matched counts are checked.
The two new tests also passed independently in 0.689 seconds.

## Independent review and test-harness correction

Two independent `gpt-6-astra` reviewers at high effort covered Callers and
Failure/security. Product source remained unchanged throughout review.

Callers found a concrete harness defect: both new cases initially ignored
`AZEROTH_TEST_BINARY` and hardcoded the Windows executable. The author reproduced
this by pointing the override to a nonexistent candidate: both tests still
passed in 0.810 seconds. The surgical fix uses the existing override/platform
suffix convention at both sites. The same probe now fails before MCP startup
with two `FileNotFoundError` errors in 0.044 seconds. An explicit valid candidate
passes the four-test command above. This verifies override handling on Windows;
no Linux execution was performed.

Jev triage: `jev-1.14.0`, request
`4d65d1aa2849df406bbeef91cf3b60f711f5018e152717b9622d8c82bb398c84`,
519 input / 50 output tokens, 0.322 seconds; fix-first 1.0. Advisory only.

Round two used Callers only for the test-only fix, with no new product caller.
Its independent missing-candidate probe failed before MCP startup, then:

```text
python -B -m unittest discover -s tests -p test_recipe_material_search.py -v
Ran 2 tests in 0.907s — OK
python -B -m unittest discover -s tests -p test_recipe_search.py -v
Ran 1 test in 0.620s — OK
```

An independent real-MCP probe passed 96 comparisons with the existing recipe
planner across both scopes, requested counts 1/2/7, required/empty material sets
and all eight storage-presence masks. Explicit null equalled omission across
eight captures. Tool schema exposes the two scopes, integer minimum one,
required nested fields and rejection of additional properties. Callers found no
remaining defect.

Failure/security's two new tests passed in 0.877 seconds. Its independent
seed-91713 integer oracle checked all pages of 4,096 captured rows for both
scopes and minimums 1, 7, 31 and u32-max. Examples:

```text
bags minimum=31: matches=1549 unknown=240 insufficient=2307
all_observed_storage minimum=31: matches=3856 unknown=240 insufficient=0
bags minimum=4294967295: matches=769 unknown=240 insufficient=3087
all_observed_storage minimum=4294967295: matches=769 unknown=240 insufficient=3087
```

Nine malformed inventories and nine additional invalid minimum/limit inputs
rejected. A separate 1,024-stack fixture totalled 4,398,046,510,080 units and
returned exact capacity 1,024 for its large per-craft requirement. usize-max
offset returned an empty terminal page. Private character/realm/GUID/link marker
values were absent, capture bytes stayed unchanged and the session closed.
No Failure/security defect was confirmed; that lens was not repeated for the
test-only caller fix. Reviewers did not edit, build, mutate Git, open network
connections, access the game, install or deploy anything. No SQLite was opened.

Native recipe completeness, bank availability, current crafting eligibility and
full profession progression remain unqualified. This source increment performs
no live game test, external lookup, installation or deployment.

Final scoped Jev advisory: `jev-1.14.0`, request
`d6aaa52e337506fbca51175077bc1bb4de9ac16270772e6b1bf7d297f16a6f73`,
945 input / 54 output tokens, 0.310 seconds; land-scoped 0.76, fix-first 0.15,
insufficient-evidence 0.09, confidence 0.64. This uncertain advisory is retained
without rerolling. Direct bounded checks and independent clean product review
support the source increment; the advisory is not proof or approval.
