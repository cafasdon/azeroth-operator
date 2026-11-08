# Native category conflicts in simulation — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broad in-game coverage remains unfinished.

The preceding native equipment-location milestone added observational
categories without changing simulator qualification. This extension reuses
the same category function to withhold a resolved Forever EquipmentSpec
when the projected equipment contains a known false category match. For
example, a saved `INVTYPE_HEAD` item that the catalogue places in finger1
retains a draft for review but cannot be qualified as that simulated ring.
This is a conservative contradiction check, not proof that the native data
or catalogue is correct, or that a matching set is legal in the game.

`checks.native_slot_category_conflicts` contains numeric-slot-ordered records
with slot, item ID, observed token and category candidates. It covers the
projected simulated slots, including unchanged baseline and same-slot
selections. It does not exempt a contradiction merely because the item is
already recorded there. Equipment moved out of a slot, a projected cleared
off-hand and non-simulated shirt/tabard slots do not contribute. Unknown,
empty or unmapped tokens, and conditional two-handed off-hand placement,
do not add a conflict. Existing catalogue checks independently still apply.
No positive equip-eligibility inference is introduced.

With a target simulator-slot filter, owned-gear rows add nullable
`native_slot_category_conflict`. Single- and multi-slot ranking skip these
candidates with that explicit reason. A conflicting projected candidate
also receives the same skip reason after full-draft review. A conflicting
baseline stops collection with a fixed actionable error: it cannot serve
as a qualified comparison reference. This applies to plan-only ranking too,
consistent with its existing requirement for a resolved baseline; the MCP
equipment draft itself remains available for inspection.

The direct request adapter rejects a nonempty conflict report even if it
was combined with a copied non-null EquipmentSpec. For legacy reports without
aggregate diagnostics, it also checks existing false category annotations
and selected-item native location metadata against nonempty projected slots.
Cleared/vacated and non-simulated targets do not contribute. The fallback's
token mapping is cross-checked against the real Rust category caller. Its typed exception,
`ConflictingNativeSlotCategory`, is handled by both CLIs with fixed messages.
No raw native data, private details or paths are included in these messages.
No simulator process is started for these failures. Old reports without
native evidence cannot be retroactively checked: refresh captures and reviews
to use the additional observations. No report authenticity, complete native
placement, class/level/proficiency, hand-pairing, unique-limit or full-set
legality guarantee is claimed.

Verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --lib planning::tests
$env:PYTHONPATH='.;tests'
python -m unittest test_native_slot_conflicts test_native_level_requirements test_native_equip_location test_simulator_level
cargo fmt --check
git diff --check
```

Build passed; four Rust tests and ten Python tests passed. New integration
cases use the real rebuilt Rust MCP and pinned local catalogue, without
downloads. Cases include known category conflict, matching/empty/unknown/
missing metadata, baseline and same-slot conflicts, moved-source projection,
two-handed off-hand clearing, exclusion of non-simulated slots, owned-gear
diagnostics, single/multi ranking skips and a rejected baseline. Direct
adapter and CLI cases reject copied-spec conflicts before any engine can
run and mask private/path details. Legacy false annotations and raw native
metadata cannot bypass the direct gate; unknown/conditional and empty projected
targets remain accepted by this check. A 425-case cross-language probe compares
25 token categories across 17 simulated slots against the real Rust caller.
Prior native category, level/ranker and
fixed-level simulation regressions remain passing. The initial test harness
assumed unwrapped async errors and guessed a runner mock/CLI option; tests
were corrected to actual ExceptionGroup wrapping, `rank_encounter` and the
plan-only parser contract without changing those existing behaviours.

No addon source changed in this unit. No development package was installed
or deployed; the source reader and local companion are the qualification scope.

Jev plan advisory supported implementing this scoped check:
`jev-1.14.0`, request
`fcc425fa2237b6f708638d71b5b6b95f019bf454cd044909ab8654dd73fafc47`,
611 input / 47 output tokens, 0.397 seconds. Probabilities were 0.87 proceed,
0.12 revise and 0.01 insufficient evidence. Tests and independent review
remain necessary; the advisory is not live qualification.

First adversarial round used independent callers and failure/security lenses
on `gpt-6-astra`, high effort. Callers found no defect and verified a real
MCP batched baseline/replacement/retained-conflict projection. Failure review
found a concrete legacy-report bypass: a prior false category annotation,
or contradictory selected-item metadata, could accompany a copied spec with
no new aggregate diagnostic and still reach request construction. The author
independently reproduced the acceptance before fixing the direct adapter.
The fallback now rejects these representations while respecting projected
empty/non-simulated targets; regression evidence is described above.

Jev finding triage selected fixing the scoped bypass: `jev-1.14.0`, request
`def8f1955b3100d1266b886d2c8744bb1384b0380ad19d2c1620f3d31ca848b7`,
654 input / 47 output tokens, 0.312 seconds (0.98 fix, 0.01 outside scope,
0.01 insufficient evidence). This records advisory triage, not a substitute
for reproduction or re-review.

Second-round callers and failure/security reviews, again `gpt-6-astra` high,
confirmed no new defect. Both ran all four new tests, including the Rust/Python
matrix (43.811 and 43.693 seconds respectively). Independent probes rejected
legacy false annotations, raw metadata conflicts, empty aggregate arrays,
and fake clear/vacated flags with populated projected slots. Actual empty
projected slots, displaced prior observations only, absent evidence and
conditional two-handed off-hand cases remained accepted by this check.
Callers traced direct and ranker request paths; both reviewers invoked the
real direct CLI with a nonexistent engine and confirmed fixed native-conflict
errors, no stdout/private markers/paths and no engine invocation. No review
edits, compilation, cleanup actions, deployment or game operations occurred.

Post-fix Jev advisory supported scoped offline landing: `jev-1.14.0`, request
`52348a736cd74db0eedb19272e1db348dd6035122ef54455073a022e40349b87`,
878 input / 52 output tokens, 0.286 seconds (0.90 landing, 0.09 insufficient
evidence, 0.01 needs fix). The actual reproduction, fix, caller tests and
independent second review round establish the scoped evidence, not advisory
confidence. Full native coverage and legality remain unproven.
