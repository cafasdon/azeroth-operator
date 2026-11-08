# Catalogue bulk material reviews — 8 November 2026

Baseline `a2c4533c8ca90a00a210f6df05b169d637d365c1`.
Source/offline synthetic verification only. No native capture, data sync, game
operation, installation or deployment.

## Contract

Each item acquisition crafted source adds a separate `bulk_recipe_review`.
The exact source recipe ID selects the saved bulk observation; the existing
selected-only source fields keep their meanings and do not become bulk aliases.
The parent lookup retains its original owner-local capture/provenance.

Missing bulk row yields null. Otherwise the wrapper has `status` and `review`:

- `learning_conflict`: contradictory captured learning; no plan.
- `catalogue_output_mismatch`: known selected or bulk output contradicts the
  same crafted catalogue item; no plan.
- `unavailable` or `budget_exhausted`: retained capture state; no plan.
- `reviewed`: the existing planner reviewed the exact bulk recipe for one craft
  against this same capture. Fixed and unsupported material rows stay distinct
  inside the review. A reviewed estimate is not craft eligibility.
- `review_unavailable`: the planner rejected the observation or the exact ID
  cannot be represented. No raw error or invented empty plan.

The positive consistent learned gate remains explicit. The source-specific
output/conflict gates run before per-recipe caching. Each request computes at
most one review per recipe ID; the cache does not persist across snapshots or
requests. No expected catalogue output is inserted into an unknown native
output field. Missing output range, inventory, bank, conditions and cooldowns
remain unknown according to the existing planner contract.

Full structured bulk review data contains captured numeric materials/output,
observed locations/stock and preparation flags. It does not copy selected names,
tracked names or raw errors. The existing owner containment, hash and schema
checks precede analysis. No new tool, native API, write or gameplay authority was
added. `craftability_verified` and `game_execution` remain false.

## Author evidence

The actual stdio executable was rebuilt from current source using
`cargo build --locked --offline` before verification. With `PYTHONPATH=.;tests`:

`python -m unittest test_catalogue_bulk_materials test_bulk_catalogue_learning test_catalogue_recipe_learning -q`

Three tests passed, 14.567 s. The new actual-MCP test checks missing/fixed/
unsupported/unavailable/budget rows, output mismatch and absent output, false
tracked learning, selected-material conflict and unavailable inventory. Each
successful nested review is exactly equal to a standalone `review_forever_recipe`
response for the same recipe/capture/count. It independently checks reagent
stock and shortages, unknown account bank, unknown cooldown/conditions, unchanged
legacy material fields and false execution/eligibility flags. Private selected
and tracked name markers remain absent; the synthetic store stays byte-for-byte
unchanged. The existing pinned local catalogue is used without downloading.

Direct recipe-planning and full stdio protocol regressions passed: two tests,
9.282 s, with
`python -m unittest test_bulk_recipe_planning test_mcp.MCPTests.test_rust_stdio_server -q`.
Formatting/whitespace checks passed. No CI or pipeline was run.

Jev plan advisory: `jev-1.14.0`, request
`ed24468e99b30338753adeba4d5c7c74fffc026aa9bb8485708dbaa77f9da8d8`,
523 input / 47 output tokens, 0.305 s; proceed probability 0.85. Advisory only.

## Independent review

One round, independent `gpt-6-astra` high-effort Callers and Failure/security
lenses: both clean. Callers ran
`python -m unittest test_catalogue_bulk_materials test_bulk_catalogue_learning test_catalogue_recipe_learning test_profession_spells -v`:
four passed, 16.638 s. Independent stdio probes checked selected-output mismatch,
requirements/profession conflict, neighbouring recipe ID isolation and successive
captures returning quantities 5 then 11 without cross-request cache leakage.
Private selected/profession names were absent.

Failure/security ran `python -m unittest test_catalogue_bulk_materials -v`:
one passed, 1.492 s. Its six-case independent stdio script used actual shared
catalogue recipe 1252256, producing item references 10192 and 250507. Each known
output arrangement reviewed the matching source and blocked the mismatching
source; unknown output preserved unknownness and permitted conditional material
review for both. False learning blocked the shared recipe, unrelated false
learning did not, and a material conflict stayed isolated while recipe 2675
remained reviewable. The script exited zero in 3.483 s, also asserting path
traversal rejection, private-marker absence and unchanged snapshot bytes.

Reviewers used the existing executable without builds, source edits or live
actions. The author build supplies separate current-source evidence; no native
qualification follows from these checks.

Post-verification Jev advisory: `jev-1.14.0`, request
`8d05616bc3b4e464a2a46cfb781a51645b025f8ecdd4b7c743d25ef1c4cf6817`,
571 input / 49 output tokens, 0.290 s; land-scoped probability 1.00. Advisory only.
The broad feature objective and native qualification remain open.
