# Simulation profession identity — 8 November 2026

Source and offline integration only; no deployment, installation, game
interaction or live capture.

Problem: a saved character equipment draft could be simulated with a public
template's unrelated professions. Reviews now carry `captured_professions`
from the validated owner-scoped snapshot, with null preserving an unavailable
section. Both direct simulation and owned-gear ranking check the template's
two profession identities before any engine call. Known mismatches stop
simulation with a fixed actionable diagnostic, without paths or names.
Plan-only ranking remains available and reports the mismatch.

The mapping uses the pinned [Profession enum](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/proto/common.proto)
and the same skill-line mapping already used for catalogue item requirements
in `src/forever_catalogue.rs`. Numeric and symbolic protobuf enum spellings
are accepted; duplicate and invalid template professions are rejected.
Secondary skills do not occupy the two primary profession positions.

Missing observations, unknown skill lines or an ambiguous captured primary
set remain `unverified`. An explicitly empty observation is known empty.
Unknown rows never suppress a known captured identity missing from the
template: that remains a contradiction even if a full comparison is impossible.
Comparisons disregard profession order. A successful match establishes only
identity, not trained ranks, recipes, specialisations or consumable eligibility.
No template values are silently changed. Old exported reviews without the
new field remain usable as an explicitly unverified assumption.

Rank output reports `template_identity.professions`; direct adapter output
reports one `profession_identities` entry per input review. Both expose
`ranks_and_recipes: "unverified"` independently of an identity match.

Verification commands:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_simulator_professions test_simulator_level test_encounter_sensitivity
cargo test --locked --offline planning::tests
```

The Python tests exercise actual Rust MCP stdio review outputs for unavailable,
empty and sparse captured professions, direct request construction and input
immutability, mismatches and enum validation, an actual standalone adapter
subprocess that stops before touching a nonexistent engine, and ranking/plan-only
CLI paths with simulation entry points asserted untouched on mismatch.

Initial author checks: 13 focused Python tests and four Rust planning tests
passed. The independent caller and failure lenses then identified a mixed
known/unmapped bypass: an early unknown-row return ignored a known captured
profession absent from the template. Author reproduced it and replaced the
early return with a known-subset comparison plus an incomplete flag. The
four profession tests passed after the correction.

Second review round: callers and failure/security found no further concrete
flaw. Caller verification included actual MCP primary/additional batch
readback, both module entry points, 13 scoped tests and ordering probes.
Failure verification included four scoped tests and 35 assertions over
mixed-known ordering, duplicate/unknown rows, plan-only status, all ten enum
mappings and malformed rows following unknowns. All passed.

Jev advisory triage recommended the narrow correction: `jev-1.14.0`, request
`24a189e382b96815902cd6e5fbe87a8531b32c6062fddd2f56d1c8176ed111ad`,
531 input / 46 output tokens, 0.317 seconds. This informs triage, not proof.
