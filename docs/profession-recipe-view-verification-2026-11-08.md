# Profession recipe-list capture — 8 November 2026

The [Forever profession helpers](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_ProfessionsTemplates/Blizzard_Professions.lua)
enumerate `C_TradeSkillUI.GetFilteredRecipeIDs` and obtain recipe details through
`GetRecipeInfo`. They distinguish local crafting from external crafting lists.
The [API documentation](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/TradeSkillUIDocumentation.lua)
documents optional recipe details. This is source evidence, not native-build
qualification. An unfiltered all-recipe API was not established in this change.

`profession_recipe_view` captures the currently visible local crafting page.
It requires explicit local mode, no external list, no NPC crafting and no
inspection state. `IsDataSourceChanging` must be explicitly readable and false.
Context is checked before and after enumeration, with the
profession ID and ordered recipe list checked for changes. No profession/page,
recipe, filter or crafting action is invoked. Only recipe IDs, optional names
and learned flags are exported; crafter identity and GUID fields are omitted.

The strict section contract is Forever-only:

- `coverage` is `filtered_local_recipe_list`.
- `profession_id` is a positive unsigned 32-bit identifier.
- `entries` contains at most 4,096 canonical positive unsigned 32-bit recipe IDs.
- Row names are optional/null, nonempty when present and at most 256 UTF-8 bytes.
  Learned flags are optional/null booleans. Unknown fields are rejected.

Missing recipe detail remains an empty row. Invalid or restricted values,
duplicate/sparse IDs, nonlocal contexts and changing list/context make the
section unavailable. Valid empty lists remain empty observations. Summary
counts distinguish learned, unlearned and unknown learning, with null when
the section is missing. Absence from this filtered list proves neither missing
knowledge nor craft eligibility. Selected-recipe material reviews are unchanged.

Offline evidence:

- `cargo build --locked --offline` passed.
- `cargo test --locked --offline --test profession_recipe_view` passed one
  Lua-export-to-Rust-import test covering local rows, unknown details, empty
  lists, linked/guild/NPC/inspection/hidden contexts, restricted IDs/names,
  malformed/duplicate/sparse/oversized IDs and changing profession/list.
- With `PYTHONPATH=.;tests`, `python -m unittest test_profession_recipe_view test_mcp test_snapshots test_packaging`
  passed 13 tests, including real MCP readback, summary semantics and strict
  Python/Rust schema rejection of non-Forever, wrong coverage/types, private
  fields, UTF-8 overlength names and noncanonical recipe keys.
- Formatting and diff checks passed.
- Development package: `local-data/recipe-view-qualification/AzerothOperator-0.0.0-recipe-view-test-forever-live.zip`.
  SHA-256 `0c2a1256d8be870738257364ed7a1d6582fcf20f8048ea70318c67d9e3e4776f`.

The package was not installed. Live game testing remains deferred.

## Saved-character recipe search

`list_snapshots.profession_recipe` requires a positive recipe ID and optionally
matches profession ID and the existing `RecipeLearning` enum. Every match
requires that exact recipe row in the captured local filtered list. Missing or
null learning flags match unknown; explicit false matches unlearned. Absence
from the list does not match any learning state. The response includes compact
`profession_recipe_match` metadata, preserving the filtered coverage label.

Existing owner-root validation, character/family/date/newest selection and
pagination remain in place. Feature matching runs after newest-capture
selection, so a missing-newest view suppresses the older recipe observation.
Time bounds are applied before newest selection. Other filters combine by
conjunction; no other recipe list is substituted for this one.

`cargo build --locked --offline` passed. With `PYTHONPATH=.;tests`,
`python -m unittest test_profession_recipe_view test_mcp test_snapshots` passed
11 tests. The real-MCP fixture covers same recipe/profession matching,
true/false/unknown/absent distinctions, nullable learning, default output,
missing-newest suppression, date-bound selection, family/ID/enum rejection
and strict unknown-argument fields. Formatting and diff checks passed.

Independent reader/failure review (gpt-6.1-sol, high effort) found no concrete
flaw. `python -m unittest discover -s tests -p test_profession_recipe_view.py -v`
passed one test in 0.200 seconds. A separate real stdio probe passed 45 Python/
Rust boundary cases and four owner-boundary rejections, including numeric and
UTF-8 bounds, 4,096/4,097 rows, canonical recipe keys, nullable metadata,
unknown fields and wrong client families. Explicit null and missing learning
flags both counted unknown. External-root hashes and path tricks were rejected.

Capture/security review (gpt-6.1-sol, high effort) reproduced one consistency
gap in the initial collector: a stable list was accepted while the native
data-source-changing flag was true. The [Forever frame source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_Professions/Blizzard_ProfessionsFrame.lua)
defers handling during that state. The collector now requires explicit false
at both context checks; missing, unknown or restricted flags are unavailable.
The Lua regression passed after adding rebuilding-at-start/end, unknown,
restricted and missing-API cases. The development package was regenerated.

Jev 1.14.0 advisory scope advice returned supported (request
`8ecf620393b5655180e87496aca9be9e23aa9d0fcb65d5682c584d3fa2496360`,
567 input / 46 output tokens, 0.299 seconds). Separate triage recommended
correcting the source-consistency gap (request
`74e91f870f52b2b1319c12cf89a56843054e24aa78b1bcff224eee4cc8ce87ca`,
527 input / 45 output tokens, 0.286 seconds). These are advisory judgements,
separate from source and runtime evidence.

Second capture/security review round found no remaining concrete flaw. Its
actual collector probe reported `changing=true accepted=false reads=0`, then
`changing=false accepted=true reads=2` with the expected recipe and profession.
The revised Lua test passed one test, including a rebuild beginning at the end
of enumeration. Reader code did not change in this correction, so its passing
boundary evidence remains scoped to the final schema. No additional callers
were introduced by the fix.

Jev 1.14.0 advisory search-scope assessment returned supported, separately from
runtime evidence (request
`7c751514cbd588730f829f06249e1a6b582d10a6485a7c0654bc4fc8af23ce9e`,
559 input / 46 output tokens, 0.303 seconds).

Independent search review ran two lenses (gpt-6.1-sol, high effort), with no
concrete findings:

- Failure: `python -m unittest test_profession_recipe_view -v` passed one test.
  An additional 39 real-MCP calls checked malformed and maximum IDs, nullable
  optionals, observed-null learning, newest profession/learning changes, date
  boundaries and filtered pagination. All passed.
- Callers/data boundaries: `python -m unittest discover -s tests -p test_profession_recipe_view.py -v`
  passed one test in 0.496 seconds. Real stdio probes confirmed conjunction with
  selected-recipe, output and tracked-recipe filters; separate newest selection
  for live/ptr variants; compact metadata; pagination; unchanged defaults;
  absent-row exclusion; seven malformed arguments; and exclusion of five
  correctly hashed invalid snapshots. Existing owner containment, hash/schema
  validation and read-only tool annotations remain in the real call path.

Additional probes used disposable synthetic owner stores and made no game,
installation or source changes. The committed regression is the retained
reproducible test; inline probes are supplementary review evidence.
