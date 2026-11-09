# Direct MCP simulator talent review — 2026-11-09

Previous goal turn made progress: bb1ce04 connected captured purchased ranks to
offline simulator commands. This unit exposes that comparison through the
existing read-only MCP tool. The broad systems/owned-BiS/native goal stays open.
No game, addon installation, deployment or hosted endpoint access performed.

## Author checks

- `cargo fmt; cargo build --locked --offline`: ordinary MCP rebuilt, 16.86s.
- `PYTHONPATH=.;tests python -m unittest test_mcp_talent_simulator test_simulator_talent_identity`:
  8 tests, 2.023s, OK with one opt-in actual-engine test skipped. The engine and
  simulator adapter were not changed by this unit; earlier actual-engine proof
  remains dated in ADR-051's evidence, not represented as a current run.
- Final new boundary assertion plus gateway profile regression:
  `PYTHONPATH=.;tests python -m unittest test_mcp_talent_simulator test_gateway_profile`:
  4 tests passed, 1.369s.

The first new fixture attempt omitted required stats race/client_version and
the Retail export format. Strict Rust validation correctly rejected those
fixtures; the fixture fields were corrected, without relaxing source guards.

Actual stdio MCP tests assert schema isolation, all 467 definition comparisons
across nine classes, 54 separate tree-boundary mismatch probes, partial/unknown
metadata, native maximum disagreement, unknown/true staging, no active-rank
substitution, empty and 4096-node graphs, omitted option/legacy captures, missing
class, original Retail string and malformed/unsupported/duplicate-ID requests.
The observations agree with the Python checker for the supported modern input
contract. Snapshot bytes are unchanged and all execution/model/full-allocation
verification flags stay false. Errors exclude private canaries and local paths.

Source trace: the Rust stdio route remains in the existing gateway tool
allowlist; Docker copies clients before compilation and embeds the pinned map.
No hosted HTTP claim follows from this local trace. The old two-tool Python
server is a legacy separate entry point and is not promoted by this change.

## Advisory and independent review

Plan: Jev 1.14.0 request
cb5884e0c88884818cd3073e594eb5796375632067f8542f09422667477082a4;
proceed .91 / revise .07 / insufficient .02; 540 input, 47 output, .313s.
Advisory only. Author retains explicit partial-allocation and native/model gaps.
Independent outcomes and final author disposition follow below.

## Independent review and disposition

- Two independent read-only gpt-6-astra high agents, Callers and Failure/privacy,
  completed one clean round. No actionable findings or code correction required.
  Native/deployment drift lens omitted because those operations remain deferred.
- Callers ran both new actual stdio tests (1.322s), traced tool schemas, old
  Retail/Forever callers, gateway allowlist and Docker include path. A separate
  deterministic seed-20261109 probe made 180 mixed Python/Rust parity calls
  across all classes; all matched, including shortened strings, capped ranks,
  unknown definitions, native maximum disagreements and staged/unknown states.
- Failure/privacy ran both new tests (1.436s); independent seed-610913 probe
  matched Python across 160 cases. Thirteen malformed raw hash-valid snapshot
  variants were rejected without private markers or local paths. Empty/zero
  strings and explicit null input retained their stated semantics. No owner
  override, game execution, full-allocation or model-verification claim found.
- Reviewers tested the existing ordinary binary and did not independently build
  it. Author compiled it after all Rust changes; only tests/docs changed after
  that build. Reviewed executable SHA-256:
  814aa9e604f219c5c9470038d651011a8f10930f1b23ff6b7a106f4e8e8a9127.
  No hosted availability claim follows; existing hosted smoke tests do not yet
  assert the new optional parameter. Agent token usage unavailable.
- Final Jev 1.14.0 advisory request
  b0bd1641fe4d908aa9c07f30f7a61bf54c07b75482f2753f4386e01aaa019d61:
  land_bounded_unit .88 / revise .08 / insufficient .04; 592 input, 56 output,
  .282s. Author accepts this bounded source feature on the local evidence and
  independent review. Final cargo fmt --check and git diff --check passed.
