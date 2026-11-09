# Hunter pet options offline verification — 2026-11-09

Scope: ADR-067. No live game access, addon installation or deployment.

Author reproduced unknown speed 999999 being accepted before the change:
`python -m unittest test_hunter_pet_options` failed with ValueError not raised.
After the shared parser, supported names/integers, aliases, null/defaults,
quoted uptime, inactive/malformed/private inputs and input preservation pass.
Real `python -m tools.run_forever_sim` rejects private malformed speed with a
generic ValueError report, no stdout, path/value echo or checkpoint creation,
before consulting a missing engine.

Pinned source ZIP SHA256:
e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712.
The enum-inventory test matches all four HunterOptions enum groups. This does
not establish full Hunter model accuracy or native effect magnitudes.

Commands used PYTHONPATH=.;tests and explicit pinned local ZIP, engine and public
template paths for optional engine tests:

```
python -m unittest test_hunter_pet_options test_simulator_pet_level test_hunter_ammo_selection test_hunter_quiver_observations
Ran 18 tests in 1.765s — OK, no skips
python -m unittest test_merge_prepared_windows.MergePreparedWindowTests.test_context_hash_source_row_and_count_rejections
Ran 1 test in 0.073s — OK
python -m unittest test_run_saved_journal
Ran 4 tests in 51.638s — OK
```

Saved-journal checks use synthetic captures and the real local MCP executable;
final encounter context retains the pet reference and false proof flags.
Existing active-pet captured level-60 checks remain covered.

First independent gpt-6-astra/high Callers review found no caller fault.
Failure/privacy review reproduced a weak generic merge assertion: unrelated
context consistency rejected the mutation even with pet validation disabled.
Fixed each mutation to require its specific pet parser error. No production
fault or private-value echo was reproduced; Failure ran 114 rejection probes.

Jev advisory triage: jev-1.14.0, request SHA256
399dca6dd265c12da9db12cf3004a6a9b6832b70eb8bc5af635eae13a38e8c0a;
fix .92, insufficient .06, none .02, confidence .88; 424 input / 46 output
tokens, .306 seconds. Advisory only. Earlier pre-compaction plan receipt was
unavailable; it was not rerolled.

Second independent round clean: Callers ran 15 tests in 50.835s without skips;
Failure ran 11 tests in .529s and the changed actual-MCP journal test in 39.405s.
Disabling the pet parser now makes the merge regression fail with the expected
specific-error mismatch, proving the test detects removal of the guard.
Both lenses reviewed the final documentation; no further fault found.

Final Jev advisory was uncertain: jev-1.14.0, request
ab1ae40c5f57b4dca4a349aeed2066bc81728067f69cbb9c9d32d4d15189a576;
integrate .47 / revise .33 / insufficient .20, confidence .20, 561 input / 50
output tokens, .335s. Not rerolled or treated as proof. Author judgement favours
bounded source integration based on the reproduced fault, pinned source parity,
specific guard-removal regression and two clean independent second-round lenses.

The preceding feature-list reply made no authoritative progress. This turn
revalidated current worktree state, reproduced the open regression and completed
the bounded source change. Broad MCP coverage remains active and incomplete.
