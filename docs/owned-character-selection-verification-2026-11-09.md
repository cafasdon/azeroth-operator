# Explicit character owned-gear evaluation — 9 November 2026

## Scope and intended behaviour

Connect the existing owner-scoped `resolve_character` tool to the unified
owned-gear evaluator. Either supply snapshot ID, or name and realm together.
The latter selects one matching saved Forever capture and pins its ID for the
entire evaluation. Mixed/incomplete selectors and missing targets fail before
journal writes. Source journals retain existing snapshot/content bindings;
newer captures do not silently retarget an existing journal. No active-player
claim, newest-alt fallback, game command, installation or deployment.

## Active identity finding

Current `addon/Core.lua` captures on a manual export and instructs the player
to reload or log out to flush SavedVariables. `tools/sync_exports.py` reads those
saved files. Neither provides a fresh in-process identity/session signal.
`resolve_character` therefore still returns `active_identity_unavailable` when
no explicit target is provided. Offline history cannot qualify current play.

## Contract and budgets

`--character-name NAME --character-realm REALM` replaces `--snapshot-id` only
for this evaluator. The MCP family is fixed to Forever and uses its configured
owner store, not a caller-provided alternate data directory. Selection becomes
`explicit_saved_character`; all active/native/full-BiS verification flags stay
false. No character names or realms are added to the evaluation output.
Resolution consumes the overall deadline and has a 15-second ceiling; temporary
stderr is suppressed and failures retain the existing generic CLI diagnostic.

## Author evidence

- `PYTHONPATH=.;tests python -m unittest test_owned_character_selection test_evaluate_owned_gear`:
  5 tests, 16.361 seconds, OK. Actual local Rust stdio MCP and synthetic owner
  captures; newer unrelated alt ignored, case-insensitive explicit match,
  missing target, invalid/mixed selectors, expired deadline. Full evaluator
  test compares named selection with the same snapshot's cached ranking and
  verifies zero new simulations and unchanged journal bytes.
- `python -m unittest test_character_resolution`: 1 test, .211 seconds, OK.
  Existing route checks family and independent-owner isolation, missing/default
  active identity, malformed arguments and discovery.
- No game process or external database was queried. Simulator output in the
  workflow fixture is synthetic; these are selection/plumbing tests.

## Plan advisory

Jev 1.14.0 request
`80fa43f38b919320c50f1237a451eb15055d9d5fa6f6ec669371de6162fdd3b3`:
proceed .81 / revise .16 / insufficient .03; 492 input / 47 output tokens,
.261 seconds. Advisory only; no active identity or full BiS qualification.

## Independent review and disposition

One clean round with gpt-6-astra high Callers and Failure/privacy reviewers.
Callers ran six selector/resolver/workflow tests in 15.382s, OK, and traced
snapshot binding and inherited deadlines. Existing collection-cancellation
fixture emitted AnyIO unclosed-memory-stream ResourceWarnings while passing;
no selection-path failure was reproduced. Failure ran two selector tests in
.143s, actual MCP identity/owner decoys, seven malformed/error/live-response
cases, and a 50ms slow-initialisation deadline probe cancelling within one
second. Both ran the real module help entry point. No concrete faults found.
Agent token usage unavailable. Native/deployment review omitted because no
installation or live operation is authorised for this unit.

Final advisory Jev 1.14.0 request
`33905a62449c072544153d8af57c809fa0031abea8b65a242db9a132f1a6ec49`:
land .75 / fix .03 / insufficient .22; 524 input / 47 output tokens,
.317 seconds. Author disposition: land on direct scoped test and independent
review evidence. The broad MCP goal remains active; current-player identity,
full feasible owned-set coverage and native qualification remain open.
