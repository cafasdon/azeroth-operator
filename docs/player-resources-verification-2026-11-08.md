# Player resources — 8 November 2026

Forever manual `/aoexport` can include `player_resources` with coverage
`readable_player_resources`. At least one of `health` and `primary_power` must
be observed. Each has unsigned 32-bit `current` and `maximum`, with current no
greater than maximum. Primary power also has numeric `power_type` 0–63 and fixed
`units: unmodified`. No names, target data or other resource pools are added.

The collector calls public player-unit APIs only. Health uses
`UnitHealth("player", false)` and `UnitHealthMax("player")`, avoiding predicted
health. Primary power uses the readable `UnitPowerType("player")` ID and passes
that explicit ID and `true` to `UnitPower` and `UnitPowerMax`. The ID must agree
before/after each pair and across both reads. Unused power token/colour returns
are ignored. [Forever primary Unit API documentation](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/UnitDocumentation.lua).

Each component requires two equal public valid observations. Missing APIs,
restricted values, exceptions, invalid numbers or changing observations omit that
component independently. If neither survives, the section is unavailable. Secret
values are screened before arithmetic/comparison; no restricted value is
transformed to recover it. The existing Core out-of-combat manual-export guard
is unchanged. There is no event stream, gameplay input or external command intake.

Rust and Python readers reject unknown fields, null components, incomplete pairs,
invalid bounds/units, coverage without observations and non-Forever usage.
`inspect_snapshot` returns the section; `character_summary` adds
`player_resource_observations` or null for a missing section. Each observed
component reports current, maximum, missing amount and percentage. Zero maximum
with zero current is preserved, but percentage is null. Primary power retains its
type ID and raw-unit label. A missing component remains null in the summary.

These are historical, independently sampled observations, not an atomic health/
power transaction. Only primary power is covered. No display-unit conversion,
death determination, readiness, regeneration/time-to-full, potion/food advice,
spell affordability or simulator resource mapping is inferred.
`readiness_verified` remains false.

## Offline verification

- `cargo build --locked --offline`: passed.
- `cargo test --locked --offline --test player_resources --test addon`: three
  Rust tests passed. Lua fixtures cover stable pairs, secret/error/partial values,
  invalid/NaN/infinite/out-of-range values, changed current/maximum/type, type
  transitions between reads, absent APIs, zero capacities, type 63, independent
  component failure, full section failure, and combat export refusal. Other
  equipment observations survive resource failures. Action stubs fail if called.
- `PYTHONPATH=.;tests python -m unittest test_player_resources`: one real stdio
  MCP test passed. It checks missing versus partial observations, zero/max-u32
  values, summaries, inspection, strict Python/Rust malformed-payload rejection,
  family gating and diagnostics without a synthetic private marker.
- Formatting and diff checks passed.

Independent Callers and Failure/security reviewers (`gpt-6-astra`, high effort)
found no concrete defect in one round. Both ran the prebuilt Lua/Rust fixture
and real MCP test, and checked the primary API signatures, strict parsers,
independent omission and absence of resource-driven actions. Callers' Python run
passed in 0.614 seconds; Failure/security's in 0.785 seconds. The author build
above establishes current-source compilation; synthetic restricted sentinels
and reused fixture binaries do not qualify native secret-value behaviour.

Development package, not installed:
`local-data/player-resources-qualification/AzerothOperator-0.0.0-player-resources-test-forever-live.zip`.
SHA-256 `2a2f3a6f054ac9520a0efaf17f89a89965d60df99332247ba2a4818af53edebf`.

Jev plan advisory: resolved model `jev-1.14.0`, request
`4e48232d04fcd63acce7e690d951d2ce741a08db08c803f9a5ce15e6673460ca`,
527 input / 47 output tokens, 0.363 seconds; proceed probability 0.67, revise
0.29, insufficient 0.04. This was advisory; primary API inspection, targeted tests
and independent review establish source evidence separately.

Post-verification advisory: `jev-1.14.0`, request
`9467b5707bb858a0ac9f421408deab1d14eaeafa4811adc20178f090d79948e6`,
544 input / 50 output tokens, 0.327 seconds; land-scoped probability 0.95.
Tests and independent reviews, not that probability, support source landing.

No live capture, game operation, installation or deployment was performed.
