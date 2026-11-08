# Trainer profession-slot observations — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture.

The public [Forever trainer UI](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.lua)
uses the second `GetTrainerServiceCost` return as a profession-acquisition
flag and checks the second primary profession slot before enabling training.
The collector now saves readable Boolean `is_profession` for non-used,
non-pet lessons. The flag does not classify every tradeskill recipe and is
not inferred from names. Missing/invalid/restricted returns remain absent.

Optional `trainer_services.primary_profession_slots_full` records whether
both primary slots were occupied. Two valid derived fullness readings must
agree around collection. Primary indices must be positive, distinct and
compact when present; missing API/errors, restrictions, invalid/sparse or
duplicate observations leave fullness unknown. Empty or single-primary
observations are known not-full. Stability refers to occupancy/fullness, not
raw index or profession identity: changing indices with unchanged fullness
does not change the reported slot constraint. No identity is exported here.
Pet trainers omit both fields, and used lessons omit the acquisition flag.
Both readers reject null/non-Boolean fields and invalid pet/used placement.

The collector also compares two complete sanitised lesson observations.
Changed exported names, prices, flags or other captured requirements make
only the trainer section unavailable, preserving other valid sections.
No private API error text is exported. Existing visibility/kind/count checks
remain in force. No panel/filter/categorisation changes, profession unlearning,
selection or training actions are introduced.

With `trainer_requirements_review: true`, the saved requirements review adds
`profession_slot_checks`. It includes only selected non-used lessons, the
recorded fullness and per-lesson flag/blocker observations. A true flag with
known fullness reports that Boolean blocker; a false flag reports no slot
blocker; an absent flag or missing required fullness remains unknown. Pet
trainers return null. `eligibility_verified` remains false. Monetary
projections retain their original price/status inputs; affordability does
not establish learnability. Existing unmet-level/skill/ability checks are
unchanged. Legacy exports remain valid and new fields require updated readers.

Author verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test trainer_profession_slots --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_trainer_profession_slots test_pet_training_points
```

Build passed; three Rust tests and two real MCP Python tests passed. New
cases cover known full/free/empty slots, restricted/sparse/duplicate/changing
occupancy, missing APIs, unknown/restricted/error flags, changed lesson
name/price/flag, pet and used exclusion, section isolation, strict schema
agreement, known/unknown blocker truth cases, selected-row scope and
unchanged wallet projections. Nested requirement comparison is also traced
by inspection; the new fixture does not separately mutate every nested field.

Development package, not installed:
`local-data/trainer-profession-qualification/AzerothOperator-0.0.0-trainer-profession-test-forever-live.zip`.
SHA-256: `a84505b3a3186a9c0a9232678bc5ea3885252b7949750fbef11c4fb9047bd99f`.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, with no confirmed finding. Callers re-ran real MCP tests and passed
five independent name/status/empty/intersection selection cases, preserving
context and used-row exclusion. Failure review independently ran the Rust
fixture executable and real MCP test, then traced secret screening, sanitised
recursive comparison, schema placement rules and fixed Core failure reasons.
Both raised occupancy-versus-index wording as an interpretation point; after
clarification both found no false blocker under the occupancy-only contract.
No edits, compiled probes, cleanup actions or live operations were performed.

Jev advisory assessment agreed that stable occupancy need not assert raw
index stability and supported offline scoped completion, leaving broad/native
completion open: `jev-1.14.0`, request
`af4c083bab8a8ad1ac30147e0b3d33e156c4c16c4e58a7af3eacb39176a150e7`,
838 input / 100 output tokens, 0.312 seconds. Tests and actual caller probes,
not this advisory result, establish the scoped verification.
