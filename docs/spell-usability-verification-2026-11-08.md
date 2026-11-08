# Saved spell-usability observations — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broad in-game coverage remains unfinished.

The public [Forever spell API](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/SpellDocumentation.lua)
defines `C_Spell.IsSpellUsable(spellIdentifier)` with Boolean returns
`isUsable` and `insufficientPower`. Manual visible-player-spellbook capture
now optionally saves them as `usability: {usable, insufficient_power}` on
each observed row, querying the reported spell ID rather than its base ID.
Both flags must be readable Booleans and agree across two calls. Missing API,
partial/invalid/restricted results, exceptions or either changing flag omit
only the pair, preserving other valid spell metadata and unrelated sections.
Private error text is never exported. Existing out-of-combat export and
visible/non-guild skill-line scope remain in force. No spell casting,
targeting, explicit loading or game action is introduced.

Both readers accept only an optional non-null object with exactly the two
required non-null Boolean fields. Legacy rows remain valid with no usability
observation. New exports require updated readers before syncing. These are
raw API flags, not a complete spell-eligibility model; no independent field
is inferred from the other.

`list_snapshots.spellbook_spell` adds nullable optional `usable` and
`insufficient_power` predicates. They apply to the same matching base/reported
spell row as passive/off-spec/learn-level predicates, before pagination.
Unknown flags do not match false. Existing newest-per-character selection
continues to happen before feature filtering, so an older matching capture
is not substituted for a newer capture with missing spell data. Returned
bounded match entries include usability only when observed. Legacy entry
shapes remain unchanged, including numeric slot ordering and truncation.

`character_summary.spellbook_usability_observations` reports observed, known,
unknown, usable, unusable and insufficient-power row counts with partial
coverage. Counts refer to rows, not deduplicated spell identities. A missing
spellbook returns null; an observed empty list has zero counts. Known flags
are counted literally, including native false values; unavailable pairs are
not counted as false. `cast_eligibility_verified` is false. These describe
capture time, not the current character in play or a current cast decision.
Cooldowns, target validity, range and other conditions are not tested or
inferred. Existing simulator, action-bar and spell-learning logic is unchanged.

Verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test spell_usability --test spell_learned_level --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_spell_usability test_spellbook_search test_spell_learned_level
cargo fmt --check
git diff --check
```

Build passed; four Rust tests and three real MCP Python tests passed. Actual
Lua export/Rust import covers usable, insufficient-power and both-false pairs,
reported-ID queries, absent API, nil/partial/non-Boolean/restricted results,
errors and second-read errors/restrictions, changes to either flag, metadata
and equipment preservation and absence of private text. MCP cases cover
strict schema agreement, same-row intersections, unknown-versus-false,
base/reported IDs, passive/learn-level intersections, summary counts,
missing/empty/legacy spellbooks, invalid predicates, newest-capture ordering
and fixed private diagnostics. The preceding search regression initially
found an unnecessary null field added to legacy match entries; output was
made conditional rather than weakening the regression. Native learn-level
comparison and prior search behaviour remain passing.

Development package, not installed:
`local-data/spell-usability-qualification/AzerothOperator-0.0.0-spell-usability-test-forever-live.zip`.
SHA-256: `605e1a0615738ae7d02d83e83a762a8bc4fc538a8e218c075292457a3cb2d54f`.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, and found no concrete defect. Both executed real MCP tests, the
preceding search module and the prebuilt Lua/Rust fixture, traced actual
callers and confirmed the public API signature. Failure review independently
tested 25 duplicate spell-ID rows with known false/false and one unknown row:
counts were 26 observed/25 known/one unknown, false matching excluded the
unknown, and 25 matches truncated to numeric slots 1–20. No review edits,
compilation, cleanup actions, deployment or game operations occurred. Prebuilt
test runs do not establish native compatibility; current source build/tests
and native live qualification remain separate evidence classes.

Jev advisory supported scoped offline landing: `jev-1.14.0`, request
`763ff5e6c6ab0bb7e18a3d151c00e4b8b4b413ab1d7e6ab201f0d3254c7bcb5f`,
895 input / 52 output tokens, 0.292 seconds. Probabilities were 0.96 landing,
0.03 insufficient evidence and 0.01 needs fix. Actual tests and independent
source/caller review, not advisory confidence, establish the scoped evidence.
