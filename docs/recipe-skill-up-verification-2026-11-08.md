# Reported recipe skill-ups — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broad in-game coverage remains unfinished.

The public [Forever trade-skill type documentation](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/TradeSkillUITypesDocumentation.lua)
defines `relativeDifficulty`, `canSkillUp` and `numSkillUps` on recipe info.
The difficulty enum is optimal, medium, easy and trivial (native values 0–3).
The manual filtered-local-list collector now optionally records these as
`relative_difficulty`, `can_skill_up` and `skill_ups_reported`. They describe
the recorded recipe at capture time; they do not define complete skill-level
thresholds, guaranteed gains, learnability or craftability.

New fields require readable valid values and agreement across two recipe
info reads with the same recipe ID. Missing, invalid, restricted, throwing or
changing optional metadata stays absent, independently per field. A failed
second identity/read omits all three new fields while preserving the original
row's valid identity/name/learning observations. Existing local-only context,
filtered-list identity and source-change guards remain. No recipe selection,
filter mutation or crafting action is introduced. Native errors and private
source fields are never exported.

Both readers accept optional non-null Boolean `can_skill_up`, unsigned u32
`skill_ups_reported` including zero, and the four difficulty labels. Legacy
rows remain valid, including their existing nullable name/learning contract.
Reported fields are not inferred from each other; a flag and count can
disagree without either becoming proof of a gain.

`list_snapshots.profession_recipe` adds `can_skill_up` and
`minimum_skill_ups_reported`. All predicates match the same exact observed
recipe row, alongside optional profession/learning requirements. Unknown
values do not satisfy false or zero. Existing time, character, newest-capture
and pagination semantics remain. Match output conditionally includes the new
fields; old captures gain no new null properties in that match.

`character_summary.profession_skill_up_candidates` requires learned true and
can-skill-up true. It lists up to 50 candidates ordered by numeric recipe ID,
reports total candidate count/truncation, and separately counts learned rows
with unknown skill-up flags. Candidate metadata can be null. Missing sections
return null; empty/legacy filtered views do not become complete profession
knowledge. `eligibility_verified` is false. This is an observed preparation
aid, not a cheapest levelling plan or a forecast of skill points earned.

Verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test profession_recipe_view --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_recipe_skill_up test_profession_recipe_view
cargo fmt --check
git diff --check
```

Build passed; three Rust tests and two real MCP Python tests passed. Capture
cases cover valid true/false/zero values, unknown rows, restricted/invalid/
throwing/changing metadata and preservation of unrelated stable fields.
Real MCP cases cover strict reader rejection, fixed diagnostics without
private text, same-row filter intersections, false/zero versus unknown,
numeric ordering, truncation at 50 of 55 candidates, learning exclusions,
unknown counts, missing-section null and prior view/listing behaviour.

Development package, not installed:
`local-data/recipe-skillup-qualification/AzerothOperator-0.0.0-recipe-skillup-test-forever-live.zip`.
SHA-256: `d0bf19af93f81a96af08ae4af08a8da3f025b87b31699a2ce9c05909191960e4`.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, and found no concrete defect. Both ran the real MCP and prebuilt
Lua/Rust fixture; callers additionally checked the exposed strict filter
schema through `tools/list` and the prior view module. Failure review ran
30 in-memory boundary cases including nonfinite values and u32 limits,
retaining valid zero/false and rejecting malformed/null metadata. No review
edits, compilation, cleanup actions or live operations were performed.
Author builds and test runs establish current-source fixture evidence;
prebuilt review runs alone do not establish native compatibility.

Jev advisory supported scoped offline landing: `jev-1.14.0`, request
`8f543af275e39dea6a31fcb3aa23424e0bce86290c72fbbc5dd0edb0f3b482c8`,
728 input / 52 output tokens, 0.294 seconds. Probabilities were 0.93 landing,
0.05 insufficient evidence and 0.02 needs fix. Deterministic tests and
independent review, not advisory confidence, establish the recorded evidence.
