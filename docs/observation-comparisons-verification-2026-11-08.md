# Profession and resource comparisons — 8 November 2026

`compare_snapshots` adds two optional boolean selectors, default false. Existing
calls retain their default output: neither new derived field is emitted without
its selector. These use the same configured owner store and the existing exact
character/client/build and chronological-order checks. Both require Forever.

## Reported profession spells

`profession_spell_changes: true` compares `profession_spells` only when both
sections are present and native profession IDs match. Otherwise the derived
field is null. It reports before/after set counts, newly/no-longer reported
spell counts, and their first 50 IDs in numeric order. Separate truncation flags
retain the full difference counts. Empty observed sets and unchanged sets are
valid; missing sections are not treated as empty.

These are differences between reported ID sets, not learned/unlearned events.
`learning_verified` and `complete_recipe_knowledge_verified` remain false.
Different professions do not produce inferred gains or losses.

## Player resources

`player_resource_changes: true` requires both `player_resources` sections;
otherwise the whole derived field is null. Within it:

- `health` contains observed before/after pairs, `current_delta` and
  `maximum_delta` when both health observations exist, otherwise null.
- `primary_power` contains those counters only when both primary-power
  observations have the same type ID and units, otherwise null.
- `primary_power_type_changed` reports the type comparison when both exist,
  otherwise null. A changed type never yields a numerical power delta.

Unsigned u32 observed counters subtract into signed i64 differences. Maximum
changes remain separately visible; zero maximum is not a division or readiness
inference. No percentage or elapsed-time event model is added. These historical
counter differences do not establish healing, damage, spending, generation,
regeneration, death, actions or cast eligibility. Resource-event and readiness
verification flags remain false. The two components are independently sampled.

## Offline verification

`cargo build --locked --offline` passed. Targeted real stdio MCP verification:

`PYTHONPATH=.;tests python -m unittest test_observation_comparisons test_profession_spells test_player_resources test_mcp.MCPTests.test_rust_stdio_server`

The new comparison test covers default output compatibility, both selectors,
numeric set ordering and 51-ID differences bounded to 50, unchanged sets,
missing sections/components, differing professions/power types, zero maxima,
maximum negative u32-sized deltas, family gating, identity/build/variant rejection
and reversed chronology. Existing observation read/summary and protocol tests
cover the touched route. Formatting and diff checks passed.

The author's four-test run passed in 8.159 seconds. Independent Callers and
Failure/security reviewers (`gpt-6-astra`, high effort) each ran the new real
MCP test and found no concrete defect in one round. Additional probes covered
published boolean defaults, nonboolean flag rejection, empty sets, power-only
data, positive max-u32 deltas, and raw on-disk malformed payloads bypassing the
Python writer. Null/overflow/noncanonical/private-field/invalid-unit payloads
were rejected with sanitised errors; traversal-like IDs did not affect server
responsiveness. Review reused prebuilt binaries; author compilation above is
the separate current-source build evidence.

No addon collector, native API call, simulator input, deployment or game action
was changed. No development addon package was needed for this analysis-only unit.

Jev plan advisory: `jev-1.14.0`, request
`829a6560cefb44d2e7f875fefb1155e0336ad55c11cf9d8014d36ad5c775b6b3`,
526 input / 47 output tokens, 0.336 seconds; proceed probability 0.98. Advisory
probabilities do not verify source or establish the historical cause of a change.

Post-verification advisory: `jev-1.14.0`, request
`7cdbd01fa571e02b04167e278a0dbc02fc2e376b5709b08395c4d61e72f49e45`,
561 input / 50 output tokens, 0.323 seconds; land-scoped probability 1.0. This
is uncalibrated advisory confidence; tests and reviews supply the actual evidence.
