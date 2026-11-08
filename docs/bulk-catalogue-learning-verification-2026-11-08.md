# Bulk catalogue recipe learning — 8 November 2026

Baseline `f9c7ca7efdcfb3d5208135bd4856c831f18eff8f`.
Source/offline synthetic verification only. No native capture, installation,
deployment, public data sync or game action.

## Behaviour

The existing local catalogue `only_observed_learned_recipe` filter now considers
exact recipe-ID bulk schematic and tracked craft/recraft learning observations,
alongside selected recipe, filtered recipe list and reported known-spell evidence.
Both item and enchant crafted sources use the same helper, before pagination.
No native-profession to simulator-profession enum conversion is inferred.

Bulk `fixed_items` and `unsupported_required_slots` rows support positive learned
metadata under their capture contract. Missing, `unavailable` and `budget_exhausted`
rows leave that flag unknown, even if unreadability arose from an unlearned API
row. Tracked flags use the exact ID's captured `GetRecipeInfo` result. Tracking
membership alone is not learning; missing/null learned flags remain unknown.

Any true/false learning contradiction yields combined `learned: null`.
Uncontradicted false evidence remains false; all unknown evidence remains null.
Known bulk output IDs are compared to the same crafted catalogue item's ID or
enchant application item ID. A known mismatch also suppresses combined learning,
even when another observation is positive. Missing output IDs do not imply a
mismatch. Existing selected-output mismatch behaviour remains.

The item acquisition-source nested `recipe_learning` result adds separate bulk row presence, status,
positive/unknown bulk learned flag, bulk output comparison and tracked group
flags. It exports no recipe names, raw errors or private identity. This enriches
discovery; selected-only source `learned`, material capacity, cooldown and
condition outputs retain their original scope. A learned catalogue source is not
proof of material availability, output correctness at runtime or craftability.

The route still validates the chosen owner-local snapshot and pinned catalogue.
No new tool, capture contract or game execution path was added.

## Author evidence

The author rebuilt the actual Rust stdio executable using
`cargo build --locked --offline` before real MCP verification.
With `PYTHONPATH=.;tests`, direct command
`python -m unittest test_bulk_catalogue_learning test_catalogue_recipe_learning test_profession_spells -q`
passed three tests, 14.587 seconds.

The new actual-MCP test checks 42 bulk-status/tracked-flag combinations against
an independent flag oracle, plus selected/list/known-spell contradictions and
enchant application output matches, mismatches and absent IDs. Unavailable and
budget rows remain unknown, true/false conflicts suppress positive filtering,
tracked-only positives work, source material capacity stays null without a
selected schematic, and private name markers do not enter responses. The
synthetic owner store remains byte-for-byte unchanged. It uses the existing
pinned local catalogue and skips rather than downloading if absent.

Existing selected/list catalogue and reported known-spell integration tests also
pass. They cover old source field scope and existing output mismatch filtering.

Direct recipe-planning and full stdio protocol regressions passed: two tests,
9.272 s, using
`python -m unittest test_bulk_recipe_planning test_mcp.MCPTests.test_rust_stdio_server -q`.
`cargo fmt --check` and `git diff --check` passed. No CI or pipeline was run.

Jev plan advisory: `jev-1.14.0`, request
`91bb3891bf7ee42a8d21c03582a2a84de025032ef55b4bc5d686d146c214131b`,
538 input / 47 output tokens, 0.262 s; proceed probability 0.90. Advisory only;
not source verification or native qualification.

## Independent review

One round, independent `gpt-6-astra` high-effort Callers and Failure/security
lenses: both clean. Callers ran the three learning integration tests: three
passed, 15.778 s. Its catalogue-derived oracle checked 100 crafted items across
five pages for both bulk-only and tracked-only learning, exact order/counts and
same-source recipe/profession constraints. Recraft-only enchant probes checked
20, 8 and 0 rows at successive offsets with exact catalogue order and `has_more`.

Failure/security ran the new real MCP test: one passed, 10.390 s. Seven
independent probes covered tracked name-only and unrelated-ID evidence,
selected/bulk wrong outputs against tracked positives, unknown bulk with explicit
negative/positive tracking and unrelated output mismatches. Expected learning
and filters matched; private markers remained absent. Both reviewers inspected
the capture/schema provenance for the newly recognised positive flags.

Reviewers used the existing executable without builds or mutations. The author
build is separate current-source evidence; none of these checks proves native
operation or complete recipe knowledge.

Post-verification Jev advisory: `jev-1.14.0`, request
`ca399f9fc3d027d34029158fefb132dd37e053e08a7ced190cf22555a93c5e31`,
590 input / 49 output tokens, 0.358 s; land-scoped probability 0.98. Advisory only.
The broad goal and native qualification remain open.
