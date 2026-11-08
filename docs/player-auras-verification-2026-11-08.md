# Public player aura observations — 8 November 2026

Forever manual capture adds `player_auras`, coverage
`readable_nonprivate_player_aura_lists`. Its `groups` can contain `helpful`,
`harmful` or both. Each observed group has `entries`, a compact map of up to
256 row numbers, with positive u32 `spell_id` and optional u32 `applications`.
Duplicate spell IDs and reported zero applications are preserved. At least one
group must be observed. Empty observed lists differ from absent/unavailable ones.

## Capture and privacy

The [primary Forever UnitAura documentation](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/UnitAuraDocumentation.lua)
defines `GetUnitAuras` with a unit, filter and optional maximum count, returning
an aura table with potentially restricted contents, and `AuraIsPrivate` by spell
ID. Capture uses `GetUnitAuras("player", "HELPFUL"/"HARMFUL", 257)` and requires
a readable table of no more than 256 rows. Querying one beyond the export bound
avoids silently treating a capped list as complete. Nil is unavailable, not empty.
The original indexed-API plan was refined to this bounded table API after checking
the primary metadata, avoiding ambiguity around an indexed nil terminator.

The player must publicly exist and be visible before/after collection. For each
group, two reads must have the same row count and spell IDs at each index. Every
retained spell ID must be public and `AuraIsPrivate` must return public false.
A private/restricted/invalid/error/oversized/unstable list makes that whole group
unavailable, without exporting its contents or a private-specific reason. A second
read failure does not retain the first list. The other group can survive.

Public valid application counts survive only when both reads agree. Invalid,
restricted, missing or changing counts are omitted independently of spell ID.
Unused aura fields are never copied: names, source units, caster GUIDs, instance
IDs, icons, descriptions and timers are not exported. No private anchors, provider
switches, filters, aura cancellation, casting or input actions are used. The
existing manual out-of-combat Core gate is unchanged.

This is not an atomic cross-group view, proof of instance continuity, all effects
on the player, or a complete buff/debuff diagnosis. Zero applications is the
native reported counter, not an imputed single stack. Missing timers do not mean
permanent effects. There is no dispel, effectiveness, expiry or simulation mapping.

## MCP reading

Rust and Python validate exact fields, Forever family, bounded compact row maps,
positive spell IDs and nonnull optional application counts. `inspect_snapshot`
returns the section. `character_summary.player_aura_observations` is null if the
section is missing; each missing group is null. Observed groups report row count,
distinct spell count, first 50 numerically ordered unique IDs with truncation,
and rows with unknown application counts. Raw duplicate rows remain inspectable.
Effectiveness and cast-eligibility verification flags remain false.

## Offline evidence

- `cargo build --locked --offline` passed.
- `cargo test --locked --offline --test player_auras --test addon`: three Rust
  tests passed, including actual Lua/Core capture fixtures for public/private/
  restricted/error data, changed IDs/counts/applications, missing API, sparse/map
  and oversized lists, empty lists, player visibility/presence changes, combat
  refusal, preservation of unrelated equipment and forbidden action stubs.
- `PYTHONPATH=.;tests python -m unittest test_player_auras test_player_resources`:
  two real MCP tests passed in 1.247 seconds. The aura test covers missing,
  partial and empty groups, duplicate spells, literal zero and unknown counts,
  maximum 256 rows/u32 counters, 50-ID summary bounds, malformed payload and
  family rejection, inspection and diagnostics without a private marker.
- Formatting and diff checks passed. No live client, native restricted-value
  behaviour, addon installation or deployment was tested.

Independent Callers and Failure/security reviews (`gpt-6-astra`, high effort)
found no concrete defect in one round. Both executed the prebuilt Lua/Rust
fixture and real MCP test. Failure/security additionally rejected 40 malformed
payloads through both Python and Rust readers without private-marker leakage.
Review verified primary signatures, strict parsing, unknown/empty distinctions
and no added action or simulation path. Reused binaries and synthetic secret
sentinels do not qualify native secret-value behaviour; author compilation above
is the separate current-source build evidence.

Development package, not installed:
`local-data/player-auras-qualification/AzerothOperator-0.0.0-player-auras-test-forever-live.zip`.
SHA-256 `44233698d2b60bd584e41ef24d05b38a5019ecae33f7568cc35f9d7f88f540d6`.

Jev plan advisory: `jev-1.14.0`, request
`fb4bb1150178d6f3072903322f18685ffcde52c7957cd9c2e369f26528f7d6d4`,
577 input / 47 output tokens, 0.314 seconds; proceed probability 0.83. This was
advisory before the bounded-table refinement; actual primary API checks and
tests establish scoped source evidence separately.

Post-verification advisory: `jev-1.14.0`, request
`f61768fc7bd07d8e1938ee2ae2d07007cb87a99fdaedd6aecd5c2d21812e40b9`,
629 input / 50 output tokens, 0.429 seconds; land-scoped probability 0.92.
Actual tests and independent review establish the scoped source evidence.
