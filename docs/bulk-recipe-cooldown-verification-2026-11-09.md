# Bulk recipe cooldowns — 9 November 2026

Scope: optional cooldown/charge capture for known fixed bulk recipes through
strict import and offline MCP planning. Baseline `351879e`. No game operation,
installation, deployment or live test occurred.

## API and implementation reference

Checked the current [Forever schematic UI source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeSchematicForm.lua)
on 9 November: its line 518 consumes the same four-value recipe cooldown tuple
used by the existing selected-recipe collector. The generated TradeSkillUI
documentation omits this function, so UI usage is the source reference, not
proof of native availability or permission. Optional absence remains supported.
No craft/refresh/filter operation or new live MCP connection is introduced.

The shared Lua helper and Lua tests were implemented by a scoped
`gpt-6-astra` high-effort writer; parent implemented strict readers, planning
propagation and MCP tests. Actual agent token usage was not exposed. A proposed
whole-tuple drop for restricted components was rejected during author review:
existing selected-recipe fixtures establish independently valid public timer/
charge retention, and that contract was preserved.

## Author verification

`cargo fmt` and `cargo build --locked --offline` passed (final ordinary build
4.13s). Direct checks:

```text
cargo test --locked --offline --test bulk_recipe_cooldown_capture --test profession_recipe_materials
2 tests passed, 0.01s and 0.31s
cargo test --locked --offline --test addon forever_bank_export_distinguishes_observed_empty_and_unavailable
1 test passed, 0.22s; 1 unrelated test filtered out
python -m unittest test_bulk_recipe_cooldown test_profession_recipe_materials test_reported_craftable_count test_recipe_search_inventory -v
4 tests, 1.678s, OK
```

Python uses `C:/Python314/python.exe`, `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`.
Lua tests use the actual source through Lua 5.1/mlua, including export and Rust
decode. They cover valid tuple order, rounding, zero values, charges-only values,
u32 limits, missing/throwing/restricted getters, partial restricted/malformed
components, unstable values, known-recipe IDs and unchanged local context.
Unsupported/budget rows retain no extra field. Existing selected-recipe partial
secret-value expectations still pass; actual native secret behaviour is untested.

Actual Rust stdio MCP tests check strict Python/Rust malformed-field parity,
search retention, charge checks/capacity, batch shortfall and source timestamps.
Newer inventory and a same-ID selected schematic cannot overwrite explicit bulk
cooldown. Timer-end estimates use the recipe timestamp, with no elapsed-time
recharge. Missing observations stay unknown, source JSON bytes stay unchanged
and synthetic private identity/error markers are absent from responses.

## Review and regression

Two independent `gpt-6-astra` high-effort Callers and Failure/security lenses ran
the four Python tests (1.624s/1.578s) and the three authorised prebuilt Lua/Rust
test filters (one pass each). Callers independently checked 53 additional
field/type/boundary cases through Python validation and actual MCP, and exact
collector bytes in the four-file capture-only package rendering.

Failure reasoned that getter lookup outside `pcall` could discard valid recipe
data when an optional namespace lookup throws. The author added a concrete
throwing-`__index` fixture and reproduced `selected: optional getter lookup must
preserve recipe`. Moving lookup/secret/type screening into each protected read
fixed it. The three final Rust/Lua tests above passed. This is a synthetic
availability regression, not evidence of native namespace behaviour or leakage;
Core already sanitises collector failures.

Round 2 Failure/security found no further concrete issue: all three rebuilt
prebuilt test filters passed and four stdio tests passed in 2.209s. Callers was
omitted because only the same reviewed helper/test files changed. Reviewers did
not build, edit, deploy, use HTTP or operate the game.

## Package checkpoint

Built `verify_addon` offline, then packaged Forever/live/interface 16001 version
`0.42.0-dev` under `dist/forever-cooldown-0.42.0-dev`. Exact manifest/source byte
verification and the packaged Lua 5.1 export round trip passed. Receipt reports
Forever capture fixture true, changed-interface rejection true and live
qualification false. Archive SHA-256:
`999ff45013ac0f57338f9472e3f86561bdbc51ee4d1473b7fbe989d9835927af`.
This package has not been installed.

## Advisory receipts

Plan: `jev-1.14.0`, request
`f2bce67b96508c8db385b93d6735d7ef944fd285d0f9740b9d676e1907a263dd`,
681 input/47 output tokens, 0.375s; proceed .86, revise .10,
insufficient evidence .04, confidence .79.

Optional-lookup triage: `jev-1.14.0`, request
`8a5255f89a4b453b4ed1c5f7ea41787ddfccd0e022702c56e647a5817b08cdd7`,
581/46 tokens, 0.312s; fix .90, insufficient evidence .09,
no material issue .01, confidence .85. Advisory only, uncalibrated.

Final scoped landing: `jev-1.14.0`, request
`d437af9c359a42059389782775c93075c686b32838e89f13a67d2b462a380190`,
763/49 tokens, 0.322s; land scoped .53, fix first .29, insufficient evidence .18,
confidence .30. This low-confidence advisory is retained without rerolling;
author landing relies on concrete scoped tests/reviews, not a native-qualification
claim. All native and full-profession limits below remain open.

## Limits

Only known fixed-row evidence is extended. API availability, bulk responsiveness,
filtered profession completeness and native capture remain unqualified. Saved
charges/timers do not prove current learning, tool/station access, material
accessibility, craftability or skill-up probability. No automatic crafting or
policy-compliance guarantee is introduced. Full profession progression and the
broad MCP coverage goal remain open.
