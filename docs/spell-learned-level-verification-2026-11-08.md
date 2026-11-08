# Reported spell learn levels — 8 November 2026

Source and offline verification only. No deployment, installation, game
interaction or native capture.

The public [Forever spell API](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/SpellDocumentation.lua)
exposes `C_Spell.GetSpellLevelLearned`, whose reported value may vary with
level linking. The existing visible player spellbook collector now calls
it for the captured reported spell ID, not the base ID. Two readable
unsigned-32-bit integer returns must agree before optional `level_learned`
is exported. Zero remains an API-reported zero; it is not interpreted as
unknown or free access. Missing API/namespace, errors, invalid/restricted or
changing values leave the field absent while preserving the observed spell.
Restrictions are checked before arithmetic or equality. No explicit data
load, tooltip fetch, cooldown observation, spell action or new combat
automation is introduced. Hidden and guild spellbook lines remain excluded.

Both readers accept the optional non-null unsigned-32-bit field and reject
booleans, negative/fractional/string/overflow values and null. Existing
exports remain valid. Existing `inspect_snapshot` spellbook filtering returns
the new metadata for base or reported spell-ID matches.

`compare_snapshots.spellbook: true` adds
`spellbook_change.changes.observed_level_learned_changes`. It matches only
shared exact `(base_spell_id, spell_id)` keys and compares deduplicated sets
of reported levels. Missing values contribute null; duplicate conflicting
observations remain sets rather than a selected threshold. Names, slot order
and identical duplicate rows do not produce a metadata change. Changed
reported IDs are handled by the existing reported-spell-ID comparison,
without crossing identities here. An unavailable spellbook leaves the
existing overall changes null. No numeric level delta, eligibility, learning
gain or current usability is inferred. Captured learn-level metadata does
not qualify talent ranks, combat rotations or simulator correctness.

Author verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test spell_learned_level --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_spell_learned_level test_active_pet_details
```

Build passed; three Rust tests and two real MCP Python tests passed. New
coverage uses actual Lua/Core/Rust import for stable/zero/maximum values,
missing APIs, first/second-read restrictions/errors, invalid numeric values
and changing results, preserving other sections and omitting private markers.
Real MCP inspection and comparison cover exact IDs, duplicate level sets,
unknown-to-zero, changed effective IDs, translated names/relocated slots,
missing views and independent rejection of malformed stored values by
both readers. Existing addon and active-pet paths passed.

Development package, not installed:
`local-data/spell-level-qualification/AzerothOperator-0.0.0-spell-level-test-forever-live.zip`.
SHA-256: `e8502bb1ae3ac819dcc89a410f7ff624a7a9fd5b013dcc3366ac0bce2d72e9f2`.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, with no concrete feature finding. Callers re-ran the new Rust/MCP
tests and existing Forever addon case, then passed eight independent MCP
assertions covering default opt-out, legacy rows, duplicate count/order,
ambiguity, identity boundaries, empty books and known-to-unknown transitions.
Failure review re-ran both new tests, used offline restricted-value proxies
with throwing metamethods, and verified invalid-type omission, non-Forever
no-call behavior, combat/interface guards and real MCP identity boundaries.
These proxies do not qualify native secret-value behavior.

The failure review's compiled probe artefacts remain at
`C:\Temp\codex\azeroth-spell-review` as offline evidence. Automatic approval
review rejected recursive cleanup with exactly `rejected: blocked by policy`,
giving no more specific reason. No deletion retry or bypass occurred. Both
probe commands exited successfully and the MCP context closed; no background
process was launched or execution session left running.

Jev advisory scope assessment supported offline feature completion while
leaving broad/native completion open and retaining the temporary evidence:
`jev-1.14.0`, request
`7437615f10582d4c84f0fa019ffa970bec515e67bfedf2f3fdc529a8e7105fa5`,
732 input / 51 output tokens, 0.290 seconds. Tests and actual caller probes,
not this advisory result, establish the scoped verification.
