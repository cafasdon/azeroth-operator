# Gear inventory evidence propagation — 9 November 2026

Scope: expose the existing saved pre-filter inventory evidence through merged
plans, journal status and final owned evaluation. Baseline `70c5110`. No game
operation, live test, installation, deployment or external lookup occurred.

## Author evidence

With `C:/Python314/python.exe`, `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`,
`AZEROTH_POSITIONAL_ARTIFACTS=C:/Temp/codex/forever-positional-v4/artifacts`,
`AZEROTH_QUALIFY_TEMPLATE=C:/Temp/codex/forever-sim-9dc32ac/public-hunter-template.json`:

```text
python -m unittest test_gear_inventory_propagation test_merge_gear_windows test_positional_gear_workflow.PositionalExecutionTests.test_actual_all_slot_positional_workflow_resolves_hole_and_reuses_results -v
Ran 6 tests in 29.480s
OK
```

After the count-consistency fix, the same six tests passed in 30.461s.

Strict synthetic metadata probes cover unknown/extra fields, storage ordering,
duplicates/overlap/missing partition members, boolean/integer confusion, counts,
ID bounds, list length, truncation flags and attempted true proof flags. A valid
51-ID/truncated observation and all-sections/no-reported-gap observation remain
accepted while ownership proof stays false. Journal reopen preserves the exact
evidence and bytes. Changed evidence rejects append without changing the file.
Legacy null/missing evidence stays null.

The merge integration exercises the actual Rust MCP producer. The author-only
actual offline positional engine workflow returns computational completion for
four coordinates/zero unresolved/three gear sets across two encounters, six cold
runs and zero new warm runs. Final evidence equals collection progress, saved
simulation journal coverage and warm output. Missing character/account banks
remain visible under `observed_with_gaps`; no completion flag becomes ownership
or BiS proof. Engine/template identities remain those in the prior positional
qualification evidence. No source Rust build was needed for this Python change.

## Advisory receipt

Independent `gpt-6-astra` high-effort Callers and Failure/security lenses each ran
eight scoped metadata/merge/owned-workflow tests using actual MCP plus synthetic
engine boundaries (33.098s/32.758s). Callers confirmed producer ID bounds, full
propagation, preserved missing-bank evidence under complete coordinates, rejection
of extra/private fields, proof promotion, floats and legacy/current mixing.
Failure additionally checked final evidence equality during collection-incomplete,
unresolved and computationally complete runs (one test, 12.115s).

Failure found producer-impossible counts were accepted: `2**128` locations and
positive unmatched counts without any observed section. The author added a
focused regression and reproduced three failing subcases (`ValueError not
raised`), including nonzero locations with zero distinct IDs. The minimal fix
bounds counters to u64/u32, requires a reported observed section for nonzero
counts, and makes zero location/distinct counts agree. No ownership flag had
been promoted by the fault.

Round 2 Failure/security found no remaining material issue: eight scoped tests
passed in 31.008s. Additional probes rejected eight numeric/storage/zero-count
violations and accepted five valid boundary cases (including 50/51-ID truncation
and numeric ceilings). All-storage-unavailable with zero counts and legacy None
remain valid. Invalid append preserved bytes; even a recomputed matching report
hash did not bypass malformed-evidence rejection on journal read. The connection
was explicitly closed. Callers was omitted in round 2 because the correction
touches only the same reviewed validator/test files and caller paths.

Plan: `jev-1.14.0`, request
`211c3af5204fee634f3a07081cb90f7abfe82612a0ccf20961b7b3d969025a29`,
558 input/47 output tokens, 0.266s; proceed .61, revise .25,
insufficient evidence .14, confidence .42. Advisory uncertainty is retained;
these values are uncalibrated and do not establish correctness.

Count-consistency triage: `jev-1.14.0`, request
`f8ede9642fe1c8130e67704f5553606182495010e4351d376127cb8a035fe95b`,
567/46 tokens, 0.285s; fix .99, insufficient evidence .01,
no material issue .00, confidence .98. Advisory only.

Final advisory: `jev-1.14.0`, request
`d73d7130ad48c589088f61260b3f28ba35a599b4ed659582b373c497b1dd7e2c`,
635/48 tokens, 0.279s. The selected answer was **fix first**, .48 versus scoped
landing .47 and insufficient evidence .05, confidence .22. This is not recorded
as a pass and was not rerolled. It supplied no concrete defect. The author
rechecked the exact schema/proof flags, numeric/storage relationships, cross-
window binding, legacy unknown behaviour and the final report's descriptive
status against source and the passing regressions. The final author verdict is
to land this scoped metadata propagation: independent follow-up and real-caller
evidence leave no concrete unresolved defect, while ownership proof and the
broader goal explicitly remain incomplete. Advisory disagreement is retained.

## Limits

This validates reported metadata, not its truth in game. All observed sections
can still be partial; unmatched items may include non-gear. Zero unmatched rows
does not establish modifier resolution or model support. Legacy evidence cannot
be reconstructed from a completed ranking. Complete owned pool, native/model
qualification, active-character detection and full owned BiS remain open.
