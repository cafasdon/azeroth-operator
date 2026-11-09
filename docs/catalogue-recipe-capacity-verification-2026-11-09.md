# Catalogue recipe capacity projection — 9 November 2026

Baseline `67ff817`. Bounded acquisition integration of three existing selected
review fields. No addon/schema/new request, sync, external lookup, installation,
deployment or live game test.

## Regression and author checks

The new actual Rust stdio test failed `KeyError: material_capacity_basis` before
the three-field projection. Direct review already carried that field. The fix
reuses the existing matched selected one-craft review; no calculations or identity
guards are reimplemented.

`cargo fmt` and `cargo build --locked --offline` passed; ordinary build 5.33s.

```text
C:/Python314/python.exe -m unittest test_catalogue_recipe_capacity test_catalogue_recipe_learning test_catalogue_bulk_materials test_bulk_catalogue_learning test_recipe_zero_item_capacity -v
Ran 5 tests in 12.639s
OK
```

Environment: `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`; existing pinned local
catalogue, no automatic download. The new test checks 14 synthetic captures:
free/zero/missing-charge/required/missing-stock/unsupported selected observations,
wrong recipe/output, unknown output, unlearned/conflicting learning, bulk-only,
both and missing recipes. Projected fields exactly match direct selected review,
while different same-ID bulk charges stay independent. Missing/mismatching
selected evidence yields nulls. Source timestamps, absent synthetic private
markers and byte-identical captures are asserted.
Pinned catalogue SHA-256, unchanged before/after checks:
`5ca68229623b8bc1f9a085ed3b5f564ab82ff66055503749e324d640ac04640e`.

## Advisory and review

Plan advisory: `jev-1.14.0`, request
`f3a5b5e3d2c5b4395cba6395bf12ae0661929d291dc9c36ca722f2ec4cd23884`,
544 input / 47 output tokens, 0.291s. Proceed 0.92, revise 0.05,
insufficient evidence 0.03, confidence 0.88. Uncalibrated advisory only.

One round, two independent `gpt-6-astra` high-effort lenses, both clean. Actual
agent token usage was not exposed. Callers ran the five author modules plus
`test_profession_spells`: six tests passed in 12.166s. It traced the real saved
acquisition route, exact-ID/output guards, independent bulk review and additive
response compatibility; no strict downstream response decoder was found.

Failure/security ran:

```text
C:/Python314/python.exe -m unittest test_catalogue_recipe_capacity -v
Ran 1 test in 1.136s — OK
C:/Python314/python.exe -m unittest test_recipe_zero_item_capacity test_recipe_cooldown test_bulk_recipe_cooldown -v
Ran 4 tests in 1.813s — OK
git diff --check -- src/forever_catalogue.rs
Exit 0
```

It verified only computed selected-review fields were added, privacy markers
were absent, source bytes unchanged, missing/mismatched selection stayed null,
and unknown-output/learning-conflict observations retained existing semantics.
The capacity regression matrix has 96 inputs and 192 storage-scope checks;
Callers' final prose called these 192 combinations, but the test code and author
record distinguish them. Both exercised the frozen parent-built executable;
neither rebuilt, edited, used HTTP/native/game operations or opened SQLite.
Executable SHA-256 recorded by Failure/security:
`fb08bb090189a3dcf84d35ab3d598e46334d7ccacd0492b26dedc452b5588432`.
No actionable finding and no fix round.

Final scoped advisory: `jev-1.14.0`, request
`9a1718023ff0baf52608713aa886fe95c57528a20a97d1b9dd1dab3a9570d988`,
859 input / 52 output tokens, 0.279s. Land scoped 0.90, fix required 0.05,
insufficient evidence 0.05, confidence 0.85. Uncalibrated advisory only. Author
disposition: land this source/offline integration; the broader goal remains open.
