# Reported profession spells — 8 November 2026

The Forever manual exporter adds `profession_spells` with fixed coverage
`reported_known_profession_spells`, the observed native `profession_id`, and
`entries` keyed by positive u32 spell IDs with empty object values. Maximum
4,096 IDs; an observed empty list is distinct from an unavailable section.
No names, crafter identity, GUIDs or arbitrary recipe metadata are copied.

The public [Forever TradeSkillUI documentation](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/TradeSkillUIDocumentation.lua)
defines `GetProfessionSpells(professionID, skillLineID?)` returning `knownSpells`.
The collector passes the native ID observed through `Professions.GetProfessionInfo`,
omitting the optional skill-line restriction. It does not equate that ID with
the simulator Profession enum. The report is not labelled a complete recipe index.

## Native fallback correction — 14 November 2026

Build 70938 returned zero profession spells for the open local Leatherworking
page, while the filtered recipe list held 20 rows. A guarded native probe found
`GetAllRecipeIDs` available with 592 IDs; `GetRecipeInfo` explicitly marked 20
as learned. This is runtime evidence beyond the older generated documentation.
If the profession-spell result is empty and the unfiltered list API exists,
the collector now reads that list and retains only exact, public recipe IDs
with `learned == true`. Missing recipe information or learning flags cannot
establish learning. Two known sets and the local context must still agree.
No filters, recipe selections or crafting calls are changed.

Direct profession-spell, bulk-material and cooldown capture tests passed.
The profession-spell regression includes secret/malformed/duplicate/sparse/
oversized fallback data and changing learned state. One independent review
found no confirmed medium or high issue. Installed capture evidence follows
in the dated pilot repair report; source tests alone do not prove that result.

Capture requires the existing local crafting page to be visible, local mode,
no external list, no inspection, no NPC crafting and a stable data source.
The profession ID must remain the same. Two public ID sets must agree; ordering
changes are accepted, while duplicates, sparse/map tables, restricted values,
invalid IDs, exceptions, excess size or context changes make this section
unavailable. Other valid sections survive. No filters, profession selection,
crafting calls, command intake or gameplay actions are added.

Rust and Python validate the exact ID-only payload and Forever family.
`inspect_snapshot` returns it. `character_summary.profession_spell_observations`
reports the count, first 50 numerically ordered IDs and truncation. Missing
section returns null. Complete recipe knowledge and craftability remain unverified.

## Local catalogue integration

An exact reported ID matching a catalogue crafting-source spell can support
`only_observed_learned_recipe`. Item acquisition `recipe_learning` now includes
`profession_spell_reported`. A known false selected/list flag conflicts with
positive reported-spell evidence and leaves combined learning unknown. Known
selected-output mismatches still suppress positive matches. The existing selected
schematic `learned`, material capacity, yield and cooldown remain unchanged.
Missing reported IDs never establish unlearning. Catalogue associations remain
reference data; the native report can contain non-recipe profession abilities.

## Offline verification

- `cargo build --locked --offline`: passed.
- `cargo test --locked --offline --test profession_spells --test profession_recipe_view`:
  two Rust tests passed, including Lua capture of stable/reordered/empty sets,
  invalid/secret/sparse/duplicate/oversized/changing data, unavailable API,
  local/visibility/NPC/inspection/data-source gates and preservation of equipment.
  Protected craft/filter stubs fail if called.
- `PYTHONPATH=.;tests python -m unittest test_profession_spells test_catalogue_recipe_learning`:
  two real MCP tests passed. The new test checks missing/empty/full 4,096-ID
  observations, 50-ID summary bounds, exact ID joins, conflicts/output mismatches,
  strict reader rejection and unchanged selected-only material semantics.
- A source-description/family-rejection test delta was added afterwards and
  rechecked before landing: the two MCP tests passed in 4.420 seconds. Formatting
  and diff checks passed. No native API behaviour was asserted from tests.

Independent Failure/security review (`gpt-6-astra`, high effort) found no concrete
defect. It executed the prebuilt Lua/Rust fixture and real MCP test, and rejected
23 additional malformed Python payloads while accepting both u32 boundaries.
It checked the primary API signature and no-action/unknown semantics. Reused
executables and synthetic restricted sentinels do not qualify native behaviour.

Independent Callers review (`gpt-6-astra`, high effort) also found no concrete
defect, executing both fixture and MCP test and 13 additional malformed-value
probes. It checked the actual `Professions.GetProfessionInfo` helper and native
`ProfessionInfo` definition, confirming that the observed numeric profession ID
is distinct from the optional Profession enum. Both catalogue consumers retain
exact spell joins and selected-only planning. One clean round per lens.

Development package, not installed:
`local-data/profession-spells-qualification/AzerothOperator-0.0.0-profession-spells-test-forever-live.zip`.
SHA-256 `c4f1b69a6f28dc90069bf8ea03f589ac847e793787cb79278994cba31ec1498e`.

Jev plan advisory: resolved `jev-1.14.0`, request
`526387e8a11bff6d4f5ebd6acd7b8761219d89451baa4beb2eb8d1ff8d54e379`,
550 input / 47 output tokens, 0.320 seconds; proceed probability 0.91.
Advisory probabilities do not prove source correctness or native scope.

Post-verification advisory: `jev-1.14.0`, request
`bbca5297876203cf61e285cb01c6de106dfb0acf87db29110f009bc4511b13ee`,
596 input / 50 output tokens, 0.352 seconds; land-scoped probability 0.98.
The actual source/API checks, tests and reviews establish the scoped evidence.

No game interaction, native capture, install or deployment was performed.
