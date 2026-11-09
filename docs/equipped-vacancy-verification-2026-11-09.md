# Positional equipped-source vacancies — 9 November 2026

Scope: extend feasible offline gear enumeration to moves leaving an equipped
source empty within requested slots. Baseline `a7f8367`. No game input, live test,
installation, deployment or external lookup occurred.

## Author verification

The new actual Rust MCP regression first returned one draft instead of expected
baseline plus vacancy draft: `AssertionError: 1 != 2`, 0.358s. A level-60 synthetic
Hunter has ring 4549 in finger1 and finger2 empty. Positional search now retains
the ring1→ring2 draft with exactly one equipment source, `vacated_equipment_slots`
equal to `[11]`, empty simulator index 10 and ring ID at index 11. It journals and
merges with complete reported coordinates and zero unresolved proposals.
Legacy mode and out-of-scope single-slot search still return only baseline.

An initial test artifact omitted the required `best_in_slot_verified: false`
plan-report metadata and was correctly rejected by the existing merger. The
fixture was corrected; no merger guard changed.

The initial draft additionally charged vacated slots against `max_swaps`. Author
readback of existing CLI help established that it counts selected replacements.
A new expectation for one move under `max_swaps=1` reproduced `1 != 2` (1.270s),
then the extra charge was removed. Ordinary enumeration still enforces the
selection count. No broader native or legality guard was relaxed.

Using `C:/Python314/python.exe`, `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`:

```text
python -m unittest test_equipped_vacancies.EquippedVacancyTests.test_actual_mcp_vacancy_scope_budget_and_retained_plan test_gear_combination_windows test_gear_draft_dedup -v
```

The combination-window oracle/direct-offset and real-MCP large-pool CLI tests
and physical-copy/dedup tests cover the existing enumeration/collapse boundaries.
Final corrected command: four tests passed in 10.087s.

The parent separately ran both new tests with explicit local artifacts:

```powershell
$env:AZEROTH_POSITIONAL_ARTIFACTS='C:/Temp/codex/forever-positional-v4/artifacts'
$env:AZEROTH_QUALIFY_TEMPLATE='C:/Temp/codex/forever-sim-9dc32ac/public-hunter-template.json'
C:/Python314/python.exe -m unittest test_equipped_vacancies -v
```

Two tests passed in 9.638s. The actual offline qualified engine workflow uses
the public Hunter template, 1,000 iterations/seed 20261109, 45/90-second encounters
and a synthetic equipped ring/weapon/ranged baseline. It completes four cold
simulations for two gear sets and zero new warm simulations. Saved-loadout
selection still points to equipment slot 11, with empty finger1 and the ring in
finger2. Cold/warm ranking layouts are identical, source snapshot bytes unchanged
and ownership/BiS/game proof flags false. This all-slot workflow has no explicit
swap cap; the later selection-count correction does not change that path.
Engine/template identities are the preceding positional qualification hashes.
Reviewers do not execute real-engine raid simulations.

## Advisory receipts

Two independent `gpt-6-astra` high-effort Callers and Failure/security reviews
ran the bounded MCP/window/dedup probes, then followed up on the selection-count
correction in the same source/test files. Callers' original four tests passed
in 10.106s and corrected vacancy test in 1.647s. Its independent multi-slot
out-of-scope probe reached the new guard and retained only baseline, while valid
positional max-one-selection search retained the moved ring. Separate coordinate
windows journaled, prepared through actual MCP and attached the exact saved
loadout, preserving source bytes and empty slot without simulating.

Failure's final four tests passed in 9.458s. Independent ring/trinket probes
retained three drafts at selected-replacement budget 1 (baseline plus either
individual move) and four at budgets 2–4 (including both). A source-out-of-scope
move was excluded while the independent valid move remained; four disjoint
coordinate windows merged complete with zero unresolved proposals. An observed
native required level 61 at player level 60 rejected the vacancy. Both confirmed
unchanged physical-source/resolution checks and changed software bindings.
Neither reviewer found a remaining concrete issue or ran native/game operations,
builds, downloads or actual raid simulation.

Plan: `jev-1.14.0`, request
`ff3005e4ca3a8262d6ddb692eff3501f691a55e3a076c71fbac080a56756e565`,
574 input/47 output tokens, 0.277s; proceed .73, revise .20,
insufficient evidence .07, confidence .60.

Selection-count contract correction: `jev-1.14.0`, request
`03f5547aefd2a4cba106a1c0d9b97892a7c39ef88d0086a52c069dfb37dedee5`,
590/51 tokens, 0.300s; preserve established contract 1.00, confidence 1.00.
Both are uncalibrated advisory values, not correctness proof or permission.

Final scoped landing: `jev-1.14.0`, request
`6abd223ab56957b975bae2f25d6c99c293a86bdefd2132925dd6e7c9f3a732ab`,
727/49 tokens, 0.299s; land scoped .99, fix first .01, insufficient evidence .00,
confidence .98. Advisory only.

## Remaining limits

This closes one feasible-layout omission. It does not enumerate arbitrary manual
unequips, prove complete owned storage, qualify all item/effect/class/encounter
models or establish native equip eligibility/statistical optimality. Source
locations remain historical and displaced items have no assigned storage slot.
Active-character identity and full owned BiS remain incomplete; live work stays
deferred.
