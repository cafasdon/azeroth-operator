# Simulation character identity — 8 November 2026

Scope: source and offline integration only. No deployment, installation,
game interaction or live character capture.

The owned-gear ranker already compared captured class and race with the
template. The direct simulation adapter only checked template class/spec
consistency, leaving saved character identity unchecked. Equipment reviews
now project `captured_character_identity` with `class` and `race` from the
validated saved stats section. Absent stats produce two null fields.
Names, realms and other stats are not added to this projection.

The shared adapter helper checks each known captured field independently.
Known contradictions stop request construction with a fixed diagnostic
before the engine runs. The ranker uses the same helper and retains its
existing class/race rejection even for plan-only runs. Unknown/unmapped
fields stay `unverified`; the known other field still gets checked. Old
review exports without the projection remain usable as unverified identity
assumptions. No template values are automatically changed.

The existing pinned Forever `common.proto` class/race mapping is retained,
including numeric and symbolic enum representations and the captured
`Scourge` alias for Undead. Direct adapter results add one
`character_identities` entry per supplied equipment review. Ranking retains
its existing `template_identity.class` and `.race` statuses.

This check does not validate captured talents, pets, skills or current
character activity. Separate level and profession checks remain in force.

Author verification:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_simulator_character_identity test_simulator_professions test_simulator_level test_encounter_sensitivity
```

Build passed; 17 tests passed. New coverage includes actual Rust MCP stdio
readback for missing/known stats, alias and enum comparisons, malformed and
partial inputs, input immutability, shared ranker behavior, and actual direct
module CLI subprocesses with a matching first review and class/race mismatch
in the second. A nonexistent engine remains untouched, and diagnostics
contain neither input paths nor private observation values.

`cargo test --locked --offline --lib planning::tests` also passed all four
planning tests. Format and whitespace checks passed.

Independent caller and failure/security reviews ran on `gpt-6-astra`, high
effort. Both completed without a concrete finding. Caller probes confirmed
actual MCP primary/additional draft projections, class and race mismatch in
an additional draft stopping both fixed/bulk paths before engine entry,
and actual ranker module plan-only rejection. Failure probes passed 651
supported identity/malformed-type checks plus review ordering, partial/null
observations, output sanitisation and immutability checks.

Jev advisory scope assessment selected offline scoped completion while
leaving the overall MCP goal and live qualification open: `jev-1.14.0`,
request `ee7cd988c3acee67c246468f5f02cac6aca3ce61938b3d83c91ad2811bdac9e1`,
628 input / 49 output tokens, 0.302 seconds. Tests and real caller probes
provide the evidence; this advisory does not establish native correctness.
