# ADR 047: saved empty-slot search choices

Date: 9 November 2026. Status: offline integration.

Extend collection and unified owned evaluation with `--include-empty-slots`,
requiring positional layout. Collection also requires retained equipment specs;
the evaluator already retains them. Without the opt-in, preserve prior choices.

Each occupied requested slot has one additional removal alternative. An
already-empty slot has no redundant clear choice. A removal counts as a change
for maximum simultaneous swaps, and may combine with replacements or moves of
other observed items. Existing physical-source, target and catalogue guards
remain active. Candidate location counts remain physical item counts; separate
`empty_choices_by_slot` booleans describe the added alternatives. Coordinate
accounting includes both. Scope and collection binding prevent mixed resumes.

Persist nonempty `explicit_clear_equipment_slots` separately from the existing
combined `cleared_equipment_slots`. Pure-removal alternatives may have empty
item-selection arrays; the baseline may never have explicit clear intent.
Require canonical distinct native slot IDs, retained positional holes, no
selected target conflicts, declared removal choices in requested slots, and
combined clears equal to explicit clears plus any valid automatic off-hand
clear. Declared choices must exactly match retained baseline occupancy.

Before simulation the preparer reissues explicit clear intent through the MCP
against the bound historical snapshot and requires exact spec/hash/clear union.
No fabricated empty observations or live state claims are accepted. Final saved
loadouts retain explicit intent. Legacy reports without explicit metadata keep
their existing off-hand-only clearing validation.

Numeric checkpoints still bind the complete simulation request, including exact
holes. A completed reported coordinate space remains distinct from complete
owned inventory, native legality, all valid equipment options, engine accuracy
and statistical optimality. Full owned BiS and active-character identity remain
unverified. No game operations, addon change, installation or deployment.
