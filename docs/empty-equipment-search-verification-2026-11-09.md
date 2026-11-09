# Empty-equipment search verification — 9 November 2026

## Result and boundary

Opt-in positional collection and owned evaluation enumerate removal choices
for occupied requested gear slots. Saved plans replay these exact choices
through the owner-local MCP and simulator adapter; final saved loadouts retain
explicit clear intent separately from automatic off-hand clears. No in-game
operation, deployment, installation or addon package change.

This closes the missing removal-choice representation in the bounded reported
search. Complete inventory, all legal options, active character, native equip
legality, model accuracy and statistical full-owned BiS remain unverified.

## Author evidence

Windows Python 3.14; existing local Rust stdio MCP; pinned local catalogue;
qualified positional engine SHA
`833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983`.
Engine capability calls were real. Numeric simulator responses were deliberately
synthetic to make all-empty gear win; this tests workflow fidelity, not DPS or
engine accuracy. No external data or private real characters were used.

`PYTHONPATH=.;tests`, `AZEROTH_POSITIONAL_ENGINE` set to the existing local engine:

- `python -m unittest test_empty_gear_workflow`: 2 tests, 11.380s, OK, before
  adding partial-prefix resume assertions.
- `python -m unittest test_empty_gear_workflow test_saved_explicit_clears test_evaluate_owned_gear`:
  12 tests, 28.272s, OK, with partial-prefix resume.
- Final occupancy fix: `python -m unittest test_saved_explicit_clears test_empty_gear_workflow`:
  10 tests, 12.566s, OK. Two occupied rings produce nine coordinates and seven
  distinct simulator gear states. A one-window prefix remains incomplete;
  subsequent collection completes. The all-empty winner has no item selections
  and explicit clears [11,12]. Warm execution makes zero new simulations.
  Removing the opt-in rejects mixed resume and leaves journal/source unchanged.
- Scoped writer ran 20 reader/preparer/merge/journal tests before the final
  occupancy correction, all OK; final focused malformed suite has eight tests.

An early integration attempt coincided with a dependency source edit and was
correctly stopped by the existing request-binding recheck. No guard was relaxed;
the test was rerun after source freeze.

## Review and correction

Round one: independent gpt-6-astra high Callers and Failure/privacy reviewers.
Callers ran nine new workflow/reader tests (10.760s), six existing merger/
preparer tests (31.526s), and actual MCP coordinate probes: one occupied slot=2,
two slots with max_swaps1=3, max_swaps2=4, absent head plus occupied ring=2.
All passed. CLI help entry points loaded.

Failure reproduced a saved-report defect: empty_choices_by_slot could declare a
clear for an already-empty retained baseline slot. After recomputing its hash,
merge reported complete coordinates for that impossible proposal. Author
reproduced `True` before fixing it. Actual MCP replay would reject later, but
saved journal admission and coverage were wrong.

Smallest correction: require each declared choice boolean to equal its retained
baseline slot occupancy. New regression rejects the original input. Round two
Failure reran eight focused tests (.005s), confirmed the original reproduction
rejects, and confirmed an absent head with flag false remains accepted alongside
an occupied ring removal. Clean. Callers omitted in round two: same validator
and test files, no new caller paths. Native/deployment lens omitted because live
work remains deferred. Agent token usage not exposed.

The author also found an intermediate contextual gap before independent review:
explicit clear IDs needed to be in requested slots with declared choice true.
Reader now enforces that mapping and has malformed regression cases.

## Advisory receipts

All resolved Jev 1.14.0, advisory only:

- Plan `05ad698cebc1cfb11fb483a7d46e3287f0f93d4e9c1078492c99fa42924a9800`:
  proceed .73 / revise .20 / insufficient .07; 610 input, 47 output, .265s.
- Author contextual finding `7684b9a8d2624748e0b2d714f2b8e1e65cc270d687ceb9a4793f7b7e5212d261`:
  fix .94 / dismiss .01 / insufficient .05; 482 input, 44 output, .276s.
- Independent occupancy finding `13fa404bd54be87632a9980e0710722aec25296845ac973bc6c7a5e2e1af4fce`:
  fix .99 / dismiss .00 / insufficient .01; 516 input, 44 output, .305s.

Final advisory `accfcacf6bd9ec9b24d0ecf9a921bdee16ba101bc079c2271394b3c18e50cb8c`:
Jev 1.14.0 land .93 / fix .01 / insufficient .06; 628 input, 47 output,
.297s. Author disposition: land on direct scoped tests and independent review;
the broad MCP coverage goal remains active.
