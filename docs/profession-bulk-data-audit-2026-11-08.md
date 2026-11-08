# Profession bulk-data audit — 8 November 2026

Scope: identify the source evidence for collecting more recipe material data
locally. Repository baseline: `4870afca434227e01f8b94a798a7cafb2261e52a`.
This audit changes no runtime behaviour and does not qualify native capture.
Live testing, installation and deployment remain deferred.

## Upstream evidence

The inspected upstream commit is
[`9dc84f0d3bbae1768ac3405b3716000bd93eda06`](https://github.com/Gethe/wow-ui-source/commit/9dc84f0d3bbae1768ac3405b3716000bd93eda06),
dated 7 November 2026, 04:46:13 UTC. The pinned files matched the initially
inspected branch files byte for byte.

| Pinned generated documentation | SHA-256 |
| --- | --- |
| [TradeSkillUIDocumentation.lua](https://github.com/Gethe/wow-ui-source/blob/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/TradeSkillUIDocumentation.lua) | `e3392e58403283ef5e307268ce51781ef25e7e0d955a199906b9b37278503fd2` |
| [TradeSkillUITypesDocumentation.lua](https://github.com/Gethe/wow-ui-source/blob/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/TradeSkillUITypesDocumentation.lua) | `c1fcec130e314052873d0246311f1a554141b0cf5ace55ea849ad8c7275be29b` |

The function documentation establishes these signatures:

- Lines 557–570: `GetProfessionSpells(professionID, skillLineID?)` returns a
  numeric `knownSpells` table.
- Lines 639–652: `GetRecipeInfo(recipeSpellID, recipeLevel?)` returns nullable
  `TradeSkillRecipeInfo`.
- Lines 753–767: `GetRecipeSchematic(recipeSpellID, isRecraft, recipeLevel?)`
  returns `CraftingRecipeSchematic`.

The type documentation describes reagent choices, variable quantities,
required flags and required quantities at lines 243–258. The schematic at
lines 281–297 includes recipe ID, recipe type, optional output item ID,
minimum/maximum output quantities, reagent slots and recraft status.

Neither inspected file contains `GetAllRecipeIDs`. That absence does not prove
the function cannot exist at runtime. It supplies no evidence for depending on
it here. The documented signatures also do not establish that every reported
known spell is a recipe, every schematic is loaded, or every recipe can be read
without selection in the native client.

## Current implementation gap

`addon/Collectors.lua` captures the visible local profession's reported known
spell IDs in `profession_spells`, with stable context and two matching ID sets.
Its rows contain empty objects, not schematics. Both strict readers enforce
that shape, with a maximum of 4,096 entries.

The separate `selected_recipe` collector reads the currently displayed
schematic. It records supported fixed-item materials, optional output ranges,
cooldowns and conditions. Required alternative or variable material slots are
reported as unsupported rather than silently converted into fixed materials.

Consequently, the current known-spell capture cannot supply material plans for
all reported recipes. A catalogue learning match does not fill that gap.
Adding schematic fields to known-spell rows would also change the existing
strict contract; old readers reject nonempty rows.

The manual exporter limits its complete JSON payload to 4 MiB before hex
encoding. A 4,096-ID limit alone therefore supplies no evidence that a full
schematic export will fit or that synchronous native calls will be responsive.

## Unresolved evidence for bulk capture

The next implementation investigation has a documented per-ID read API to
exercise offline, rather than a need to manipulate recipe filters. Remaining
questions include partial per-recipe failures, stable identity across reads,
secret values, unsupported slot types, payload size and bounded execution.
Known spell IDs with no readable recipe information remain distinct from
successfully observed schematics and from known-empty material lists.

Offline doubles can establish schema parity, failure isolation, read counts
and payload bounds. They cannot establish native data availability, frame
responsiveness, complete profession knowledge or craft eligibility. Reading
recipe data also supplies no authority to craft, change filters or select a
recipe. No external character data was uploaded in this audit.

## Verification

The existing real-stdio profession-spell integration test passed: one test,
2.060 seconds. It covers summary/catalogue joins, strict Rust/Python readers
and owner-local synthetic captures. The existing direct Rust collector fixture
also passed: one test, 0.05 seconds, after an offline locked build. It exercises
stable public local context and rejects invalid or changing observations.
These are checks of the current
implementation, not proof that bulk material capture has been implemented.
