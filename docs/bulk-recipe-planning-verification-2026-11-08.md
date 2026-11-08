# Bulk recipe planning — 8 November 2026

Baseline `9c0df4461ce0cc27623b0f4b012a805578ee7ec7`.
Source and offline synthetic verification only. No native capture, installation,
deployment or gameplay action.

## Behaviour

`review_forever_recipe` and each `additional_recipes` entry accept optional
positive u32 `recipe_id`. An explicit ID selects only that capture's
`profession_recipe_materials.entries` row. Omission preserves the original
selected-schematic calculation and result shape. A selected schematic does not
serve as a fallback for a missing, unavailable or budget-exhausted bulk row.

Fixed rows supply captured consumed item quantities and optional base output
ranges. Unsupported required slots produce an explicitly unsupported review;
they do not become a zero-material recipe. A known-empty fixed map remains
separate. Output targets require a captured minimum yield and still enforce
the 1–1,000 craft bound. Results retain the exact selected recipe ID, bulk source,
native profession ID and captured material status. Recipe names are not invented.

Bulk observations establish positive learned metadata at capture time. Same-ID
selected observations that disagree on learning, requirement type, fixed
materials, jointly observed output metadata or native profession ID block the
review. A known false learning flag for the exact recipe ID in any saved filtered
list or tracked craft/recraft group also blocks it. Missing observations do not
become conflicts or negative facts. Positive membership in another profession's
known-spell or filtered list does not itself establish an exclusive-profession
conflict; that API property has not been established. The result's profession
provenance remains the explicitly selected bulk section.
No cooldown, condition, skill-up or selected-schematic metadata is copied into
the bulk review; unknowns remain null/unknown rather than zero or satisfied.

The shared material calculation takes the recipe value independently of its
source snapshot. No synthetic snapshot or fabricated selected section is passed
through the store. Existing inventory identity/build/chronology guards use the
original capture. Batches may choose multiple distinct IDs from one bulk capture,
or mix selected and bulk observations, against the same compatible inventory.
Duplicate recipe IDs still fail. Existing material aggregation, dependency
ordering and merchant estimates consume these same validated reviews.

This is a conditional offline estimate, not current availability, complete
recipe coverage, craft eligibility or authority to craft. Every result retains
`craftability_verified: false` and `game_execution: false`.

## Author verification

The current-source stdio executable was rebuilt with `cargo build --locked --offline`.
Direct tests, with `PYTHONPATH=.;tests`:

`python -m unittest test_bulk_recipe_planning test_recipe_dependencies test_recipe_cooldown test_recipe_pending_mail -q`

Five tests passed, 2.915 seconds. The new test exercises actual MCP requests for
single and same-capture batch selection, mixed selected/bulk batches, targets,
material capacity, newer inventory, producer ordering, merchant shortfall cost,
unsupported and known-empty rows, missing/unreadable/budget rows, invalid primary
and nested selectors, craft limits, duplicate IDs, conflicting observations and
wrong character/build/older inventory. Reads leave the synthetic owner store
byte-for-byte unchanged. Explicit bulk results and errors contain no unrelated
selected-schematic private marker.

Existing selected-recipe dependency, cooldown and pending-mail behaviours also
passed after factoring the common calculation. These checks establish offline
caller behaviour, not native data or performance qualification.

The unchanged real-stdio protocol smoke check also passed: one test, 7.517 s,
using `python -m unittest test_mcp.MCPTests.test_rust_stdio_server -q`.
`cargo fmt --check` and `git diff --check` passed.

Jev plan advisory: `jev-1.14.0`, request
`f5fbc7ce0c1d0258140c6d9c9dd620b4ce0fa4c9b7b35829f243ed6800e5f871`,
528 input / 47 output tokens, 0.283 seconds; proceed probability 0.70. Advisory
probabilities do not replace source checks, regressions or independent reviews.

## Review and correction

Round one used independent `gpt-6-astra` high-effort Callers and Failure/security
reviewers. Both reproduced the same missing conflict checks: explicit false
learning in tracked craft/recraft rows was emitted beside positive bulk learning;
a false exact-ID filtered row was ignored when its profession ID differed.
The author extended the actual MCP regression, reproduced success where an error
was expected, then removed the filtered-profession gate and checked both tracked
learning flags. The five scoped tests passed after the fix, 4.413 s. Additional
unknown/positive compatibility cases passed in the expanded new test, 0.877 s.

The proposed extension to reject all differing-profession positive memberships
was not adopted: neither a concrete failure nor an exclusive-membership API
guarantee was established. The implementation does not claim exclusive profession
membership. Positive and missing metadata remain distinct from known false
learning. Existing selected-schematic profession conflict checks remain scoped
to the selected schematic itself.

Jev triage advisory: `jev-1.14.0`, request
`a31828abb9478cf09d230330be452277c302c97eceaff4537c85d9a68e90a9c9`,
531 input / 61 output tokens, 0.301 s; fix-false-learning probability 1.00.
The author independently inspected the tracking collector: both groups' learned
flags come from the exact ID's `GetRecipeInfo`, so their explicit false values
do contradict the bulk learned observation.

Independent failure probes also exercised eight maximum-u32-quantity recipes at
1,000 crafts, with exact combined demand 34,359,738,360,000; nine-entry batches,
future and foreign-realm recipe captures rejected; unsupported batch material
and ordering coverage remained unknown. The caller reviewer checked runtime
schemas, bulk-only captures and unchanged catalogue/restock callers.

Round two: both Callers and Failure/security clean. Each reran the new real MCP
test (1.172 s and 0.703 s respectively). Independent probes covered primary and
nested false/true/unknown tracking, other-profession false lists and unrelated
false controls; Failure/security additionally crossed fixed and unsupported
rows. Callers also checked eight selected-route compatibility cases. Both
accepted the narrowed positive-membership claim with no remaining concrete flaw.

Post-correction Jev advisory: `jev-1.14.0`, request
`a3623a5b9f617944adcb236792dd1d496fe238b3463f9a3f8aa41b26e12695b1`,
539 input / 49 output tokens, 0.281 s; land-scoped probability 0.99. Advisory
only. The broad feature-coverage objective and native qualification remain open.
