# Forever simulator talent observation checks — 2026-11-09

Scope: local pinned public data, synthetic saved captures, actual local Rust
stdio MCP and offline simulator checks. No game, addon installation or deployment.
Previous goal turn made progress: native definition observation source was
committed and pushed as bab39f5. The current unit connects those observations
to the simulator's local definition positions; the broad coverage goal stays open.

## Author evidence

- `cargo fmt; cargo build --locked --offline`: ordinary MCP rebuilt, 4.77s.
- `python -m tools.build_forever_talent_map --source-zip C:/Temp/codex/forever-sim-9dc32ac/source.zip --check clients/forever/talent-map.json`:
  matches, nine classes, SHA 33dc07ad7ba838a4bb6f95d825df44235b0e1a00e92eb3dfdb9c68d8d96c18b3.
- `PYTHONPATH=.;tests python -m unittest test_simulator_talent_identity test_simulator_professions test_simulator_character_identity`:
  12 tests passed, 1.391s (before adding the ranker caller test).
- `PYTHONPATH=.;tests python -m unittest test_simulator_talent_identity test_collect_gear_windows test_prepare_gear_window`:
  11 tests passed, 48.607s. Includes actual local MCP projection, private config
  name exclusion, all nine classes' first/last positions in every tree, active
  boosted versus purchased ranks, effective clamping, malformed/ambiguous input,
  rank-limit and unknown/staged cases, real direct CLI early refusal, ranker
  refusal/plan-only reporting and existing saved collection/preparation callers.

The generator initially rejected two source name spellings
(`twohandedWeaponSpecialization` versus `two_handed_weapon_specialization`);
normalising case and underscores preserves identity while spell/rank/number
checks remain exact. It then refused the source Priest third-tree size mismatch.
Reading pinned character.go established the first-two-offset rule documented in
ADR-051. The emitted map preserves actual and engine-declared sizes separately.

## Advisory and independent review

Plan: Jev 1.14.0 request ec00ab46699d1ddee6710be7227fbf1c47064da17d6918686429e5a3e92b5ad0;
proceed .72 / revise .26 / insufficient .02; 512 input, 47 output, .294s.
Advisory only; author retained partial/native/model uncertainty explicitly.
Independent reviewer outcomes and final disposition follow below when complete.

## Author corrections and additional evidence

- Actual pinned positional engine/public Hunter template opt-in test:
  `python -m unittest test_simulator_talent_identity.TalentIdentityTests.test_opt_in_actual_engine_keeps_matching_request_and_metrics`
  passed, .495s. Two independent 200-iteration/seed-20261109/45-second runs
  produced exactly equal requests and returned metrics for absent versus matching
  observations; a changed purchased rank was rejected before creating a request.
  The test's first attempt assumed a nonempty first talent tree; the public
  template starts with an empty tree, so the fixture now explicitly uses zero.
  This checks request preservation, not independent model truth.
- Author found Windows checkout drift: core.autocrlf=true, no attributes, new
  map requires byte-for-byte checksum. A temporary isolated Git repository with
  real add/checkout-index reproduced CRLF checksum failure without attributes.
  Adding only the scoped map `text eol=lf` attribute preserved exact source bytes
  and the pinned SHA. The working repository's filtered/raw blob hashes also
  agree: bb35a6e4b69bbc6385aee01d80c05f36f7f11ebb.
- Checkout triage: Jev 1.14.0 request
  f27cf80b4c358915686c92eb36b905d2b88c1d225b2630bb8c08ba1de92fe228;
  fix .95 / dismiss .01 / insufficient .04; 472 input, 44 output, .276s.
- Independent Callers found preparation evidence did not bind the new identity
  module/map. Execution did re-prepare and reject mismatches, so this was a
  provenance fault rather than an execution bypass. Author confirmed missing
  paths and missing merger fingerprints. Added those two paths to preparation
  unchanged checks and hashes, and required equal dependency hashes in merging.
  Collector binds the module and computes talent observations before its final
  binding recheck. Old prepared artifacts need re-preparation; journals persist.
- Provenance triage: Jev 1.14.0 request
  7c947b627b429a8f41dd0a0bfd1603cb70c799757dcfe11bf6ad7d06a4b274b0;
  fix .98 / dismiss .00 / insufficient .02; 464 input, 44 output, .287s.
- Regression red: focused merge test failed because talent_identity_sha256 was
  absent from FINGERPRINTS. After correction, test_merge_prepared_windows,
  test_prepare_gear_window and test_run_saved_windows: 9 tests passed, 29.174s,
  including independently changed module and map digest rejection.

Primary source reference: pinned
[character.go](https://github.com/wowsims/forever/blob/9dc32ac8ca0d8d1b76337861d38b93c0f884af11/sim/core/character.go),
class proto files, Go class constructors and ui/sim/talents/trees/*.json in the
same checksum-verified local archive. No third-party site data or personal
capture content was sent externally.

## Final author and adversarial disposition

- Final changed-caller checks (actual prep fingerprint assertions and collector
  reorder): one focused actual CLI preparation test plus test_collect_gear_windows,
  4 tests passed, 50.596s. Six changed command module --help entrypoints and
  cargo fmt --check passed.
- Two independent gpt-6-astra high reviewers, Callers and Failure/privacy, ran
  read-only source/real-MCP probes. Round one found the dependency provenance
  defect above. Round two both found no remaining actionable fault.
- Callers round two: preparation/merge tests 5 passed, 27.863s; independently
  injected second-read digest drift for module and map, each rejected before
  journal/MCP access. Verified import entrypoints and LF attribute.
- Failure/privacy round two: preparation/merge/talent tests 10 passed, optional
  engine test skipped without its explicit environment; injected dependency
  digest drift, missing/different merge digest checks and exact SHA assertions.
  Earlier failure review also rejected duplicate definitions, excessive ranks,
  tampered map/source ZIP and five malformed/staged CLI cases without canary or
  path leakage. It inspected pinned engine rank/offset semantics independently.
- Actual optional engine test was executed by the author, not the reviewers.
  Agent token usage unavailable. Native/deployment drift lens omitted because
  installation and live testing remain explicitly deferred.
- This establishes offline source integration and pinned position comparison,
  not complete talent allocation, live character identity or simulation truth.
- Final Jev 1.14.0 advisory request
  f2faa4113cc127038f534aee9a0c3882866fcaf1abc07b7edb6c0759945e1851:
  land_bounded_unit .93 / revise .05 / insufficient .02; 642 input, 56 output,
  .298s. Author accepts bounded source integration on concrete tests and the
  clean second review round, keeping the original broad goal active.
