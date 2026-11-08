# Captured simulation-level binding — 8 November 2026

The pinned engine's [constants](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/core/constants.go)
fix `CharacterLevel` at 60. Its [character constructor](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/core/character.go)
uses that constant rather than a per-template level. Previously the ranker
checked captured class/race but could simulate a different captured level using
the engine's fixed level, making the result unsuitable as that character's rank.

Equipment reviews now carry `captured_character_level` from positive saved
stats, or null. Both primary and additional draft reviews carry the same source
level. `request_from` requires exact integer 60, protecting direct adapter,
ranker fixed/bulk, weighted finalist, audit and confirmation paths before engine
execution. Missing, null, boolean, string, float and unsupported levels reject.
The ranker separately verifies captured stats before dispatch. Plan-only review
continues to work at other/unknown levels, with explicit unsupported/unverified
level metadata and the fixed simulator level. Class/race mismatches still reject.

Legacy exported reviews lack the new metadata and fail closed. Re-exporting an
existing saved level-60 snapshot with the updated MCP supplies it; no fresh
game capture is needed when the stored level exists. The direct CLI now gives
that remedy via a dedicated fixed diagnostic without printing user paths or
exception data. Other levels are not newly supported by the simulator.

Offline verification:

- `cargo build --locked --offline` passed.
- `cargo test --locked --offline --lib planning::tests` passed four tests.
- With `PYTHONPATH=.;tests`, `python -m unittest test_simulator_level test_encounter_sensitivity test_native_level_requirements test_simulation_comparison test_shortlist_qualification`
  initially passed 16 tests. After the CLI diagnostic correction,
  `python -m unittest discover -s tests -p test_simulator_level.py` passed all
  four level tests, including the actual standalone module subprocess.
- `python -m unittest test_mcp test_snapshots test_gear_draft_dedup` passed
  11 real MCP/reference/draft regressions.
- Both module help entry points passed. Formatting/diff checks passed.

Independent Failure and Callers reviews (gpt-6.1-sol, high effort) found no
guard correctness issue. Real debug-MCP probes checked 60/20/null primary and
batch metadata, absent/nonpositive stats, real ranker plan-only subprocesses,
24 adapter type checks, class/race guards and mixed-review rejection before
fixed/bulk/audit/confirmation calls. Raw-request qualification intentionally
remains a simulator-fixture tool, without a captured-character claim.

Caller review identified an unhelpful standalone legacy-review error. The
dedicated safe diagnostic was added and re-reviewed. The second round passed
four level tests and the original real CLI probe: exit one, empty stdout, an
explicit re-export remedy, no paths/private exception text and zero runner calls.

Jev 1.14.0 advisory triage recommended the diagnostic correction (request
`0124f4c8422e0a52dc6f717a3dc402254a478b1ead0d2072c723b6b95b95e2fd`,
528 input / 45 output tokens, 0.313 seconds). It is separate from test evidence.

No engine math, cache keys, staged culling or iteration floors changed. No game,
installation, deployment or live verification was performed.

Jev 1.14.0 advisory claim assessment returned supported while leaving broader
coverage incomplete (request
`27d35d210c0431cf8878cfd6395b8d5ecf1ef6f5f64de9453e840c8b0621ad12`,
614 input / 46 output tokens, 0.319 seconds), separate from verification.
