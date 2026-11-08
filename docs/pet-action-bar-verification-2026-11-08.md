# Pet action-bar observations — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broad in-game coverage remains unfinished.

The public [Forever pet action UI](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_ActionBar/Shared/PetActionBar.lua)
defines ten logical slots, uses `PetHasActionBar()`, and reads
`GetPetActionInfo(slot)` in this order: name, texture, isToken, isActive,
autoCastAllowed, autoCastEnabled, spellID, checksRange, inRange. New manual
Forever capture adds `pet_action_bar` with coverage
`readable_pet_action_slots`, Boolean `pet_present`/`has_action_bar`, and
partial `entries` keyed by canonical slot strings 1–10.

The local `UnitExists("pet")` and `PetHasActionBar()` observations must be
readable Booleans and agree before/after collection. A bar reported without
a pet unit is unsupported/unknown, not inferred to be a normal pet. When no
bar is reported, no slots are queried and the slot constant/action API are
not required. An existing bar requires the expected ten-slot UI constant and
the read API. Missing APIs, restricted/invalid context or context changes make
only this section unavailable, preserving other valid sections. This does
not establish pet type, ownership history or continuity across captures.

Each readable slot has `kind: command` with a public `PET_[A-Z0-9_]+` token
of at most 64 bytes, or `kind: spell` with a positive u32 spell ID. Pet/spell
names, textures, GUIDs and target identities are not saved in this section.
Unknown/nil slot identity, errors, restrictions, invalid values or changes
between two reads leave that slot absent. Absence is not an empty-slot or
missing-ability assertion. Commands are data tokens, never dispatched or
resolved through arbitrary global names.

Optional public Boolean `is_active`, `autocast_allowed`, `autocast_enabled`
are kept only when each agrees across both identity-consistent reads.
Changed/unreadable flags are omitted independently while stable identity and
other flags remain. A readable false `range_check_performed` carries no
`in_range`. A true range check requires a readable Boolean result and both
readings must agree on the pair; otherwise both range fields are omitted.
False/unavailable range checks are not interpreted as out of range. These
are native reported flags, not current target validity or cast eligibility.

Both readers enforce exact root/row fields, context relationships, slot and
token/ID bounds, optional non-null Booleans and paired range semantics.
Only Forever supports this section. Empty entries with no bar and an existing
bar with all unreadable slots are distinct states. Updated readers are needed
before syncing new exports; native compatibility remains unqualified.

`character_summary.pet_action_bar_observations` reports context, observed and
unobserved slot counts, numeric command/spell slot lists, reported-active
slots, enabled-autocast spell slots and unknown-autocast spell slots. Missing
sections return null. Without a bar, unobserved-slot count is null rather
than an invented missing ten-slot bar. With a bar, omitted slots count as
unobserved. Lists refer to slots, not unique spells or complete pet knowledge.
`cast_eligibility_verified` is false. Reported active/autocast does not prove
behaviour, casts or effectiveness. The separate `pet_care` evidence remains
the simulator's pet-identity/level input; this bar does not qualify it.
No commands, targeting, autocast toggles, spell loading or other gameplay
actions are introduced. Existing export remains out of combat and manual.

Verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test pet_action_bar --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_pet_action_bar test_active_pet_details test_pet_training_points
cargo fmt --check
git diff --check
```

Build passed; three Rust tests and three real MCP Python tests passed.
Actual Lua export/Rust import covers commands/spells, private unused names/
textures, sparse readable slots, missing/no-pet/no-bar and changed contexts,
restricted/invalid/changing identities, errors, changed/unknown optional flags,
paired range behaviour and source/equipment isolation. Known no-bar contexts
also work with absent slot API/constant. Real MCP cases cover strict schema/
family rejection, range/identity placement, numeric summary order, absent and
empty states, ten populated slots with maximum IDs, false versus unknown
autocast and fixed private diagnostics. Existing pet detail and signed
training-point behaviour remain passing.

Development package, not installed:
`local-data/pet-action-bar-qualification/AzerothOperator-0.0.0-pet-action-bar-test-forever-live.zip`.
SHA-256: `a34ed642cb2224869d75485c7606e13159dd0ff7dee702b1371e32bd40399fc2`.

Jev plan advisory selected proceeding with scoped implementation:
`jev-1.14.0`, request
`88482631fcf0d0e44aef2adb00c035673d4ff168fba2dd61db581aef7e97fa15`,
750 input / 47 output tokens, 0.300 seconds (0.81 proceed, 0.13 revise,
0.06 insufficient evidence). Actual tests and independent review remain
necessary; advisory confidence is not native qualification.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, and found no concrete defect. Both ran the real MCP test and prebuilt
Lua/Rust fixture and checked the primary UI signature. Callers also ran five
simulator pet-level tests and injected a populated new section into the prior
pet-detail integration fixture; existing consumers remained passing. Failure
review accepted/rejected eight independent token/ID/range boundaries. Both
reviewers checked the final adjustment moving the slot-constant requirement
inside the existing-bar branch and re-ran the fixture with missing-constant
no-pet/no-bar cases. No review edits, compilation, cleanup actions, deployment
or game operations occurred. Prebuilt test execution does not establish native
client compatibility or source provenance; author source builds/tests and
native qualification remain separate evidence classes.

Post-verification Jev advisory supported scoped offline landing:
`jev-1.14.0`, request
`d26e5a41744155d95d244bd874872ebc66a2763ef6b5fbbdf64e7b61796ec6ba`,
958 input / 52 output tokens, 0.307 seconds (0.94 landing, 0.03 needs fix,
0.03 insufficient evidence). Actual tests and independent source/caller
review, not advisory confidence, establish the scoped evidence.
