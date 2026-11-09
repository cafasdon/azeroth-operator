# Queried allocation in simulation checks — 2026-11-09

Baseline 60eceb3. The preceding goal turn made progress with queried allocation
and preparation. This unit closes its explicit-template validation gap while
keeping the original broader coverage/native/owned-BiS goal active. No game,
installation, deployment, hosted endpoint or external data access performed.

## Author checks

1. New actual MCP equipment-review test obtained the saved character's review,
   used an independent synthetic equipment request fixture, then added one point
   to a definition captured as inactive/zero. It failed red because
   ConflictingTalentObservation was not raised (.139s). The test concerns talent
   preflight, not real owned-gear qualification.
2. Added the numeric allocation projection, strict standalone validation and
   matching queried comparison in Rust/Python. `cargo fmt; cargo build --locked --offline`
   passed (12.11s). Focused Python checks: 12 tests,3.173s,OK with two opt-in engine
   skips. Expanded all-class/CLI/standalone tests:14 tests,4.198s,OK with two skips.
3. Tests compare complete Rust/Python reports across all nine classes with
   matching ranks, inactive-zero conflicts, true/unknown staging, missing pinned
   definitions and unchanged snapshot bytes. The real direct CLI refuses the
   conflict before accessing a nonexistent engine, without private canary/path
   echo. A missing definition is not credited as observed zero.
4. An opt-in actual-engine fixture initially requested an empty review without
   the required catalogue; MCP correctly rejected that fixture. It now selects
   one synthetic observed item and uses independently pinned public engine gear.
   This explicitly does not assert owned-gear provenance.

## Review finding and red/green correction

Both independent reviewers found Rust double-counted unmapped queried nodes:
it precomputed distinct nodes, then the old loop added each unmapped definition.
One extra entry gave Rust2/Python1; two entries on one node gave Rust3/Python1.
Author added the exact two-entry parity case; focused red failed (.980s).
Minimal correction keeps the old per-definition increment only when no queried
allocation exists. Rebuilt ordinary MCP in5.88s. Focused checks:12 tests,4.250s,
OK with two explicit-engine skips. Legacy counters are unchanged.

## Actual offline engine

The explicit environment selected positional engine SHA
833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983
and public Hunter template SHA
761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0.
Synthetic ranks derived from that template flowed through actual MCP equipment
review. Validation with versus without allocation metadata produced exactly the
same request and returned metrics from two uncached 200-iteration runs of a
45-second simulated encounter, seed20261109. Final post-fix engine test passed (.637s). This is one model/input
integration example, not independent model truth or native client evidence.

Commands (PowerShell, direct scoped checks):

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
python -m unittest test_allocation_simulator_identity test_simulator_talent_identity test_mcp_talent_simulator
```

Direct-script and module `run_forever_sim --help` entrypoints passed. No new
package is needed; native capture source and snapshot schema were not changed.

## Independent review

Two gpt-6-astra high read-only reviewers, Callers and Failure/privacy, independently
reproduced the counter issue. Callers traced adapter/ranker/collector/saved-window
and cache paths, plus script imports and legacy tests. Failure independently
checked 32 malformed standalone/snapshot cases: missing/extra keys, numeric and
count bounds, foreign trees, linkage, ranks, duplicate entries, 65 entries/node,
4097 global entries, null allocation and invalid staging; all rejected with no
private sentinel in errors.

After the correction, both reviewed the minimal fix and probed the rebuilt MCP:
one unmapped entry => nodes1/definitions1; two sharing a node =>1/2; two different
nodes =>2/2; three on two nodes =>2/3. Python and Rust reports agree. Failure's new
suite rerun:4 tests,1.288s,OK with one explicit-engine skip. Final code round clean;
all owned MCP processes exited. Neither reviewer rebuilt source or ran a game.
Agent token usage unavailable. Native/deployment drift lens omitted because
those operations remain deferred. Documentation read-back follows below.

## Advisory record and design revision

Jev1.14.0 initial plan request
e4cf10f86c803fd538e4cf5d8c67e6457ee8e71ed90deedf8029fd2f01f30b56:
revise .47/proceed .34/insufficient .19, selected-confidence .20;
619 input,47 output,.273s. This is retained as uncertain negative advice, not a
pass and not re-rolled. Author revised the proposed nested/mixed-count report to
one explicit comparison universe when allocation exists, preserving legacy
reports when absent. The concrete red bypass, independent parser probes and
cross-language regression evidence above support the final author judgement.

Counter finding triage Jev1.14.0 request
0ef9afa1b27089441a76bd300e1445f145f746a6836e3ec90049e7fccbf49966:
fix .97/dismiss .01/insufficient .02;457 input,44 output,.357s.
All TypeSafe results are advisory, not calibrated/native-proof or permission.

Final documentation read-back is clean in both independent lenses. Clarified that
45 seconds is the simulated encounter duration, not elapsed runtime or timeout.
No runtime changes were needed after the final counter-fix review round.
Reviewed ordinary MCP executable SHA-256:
016be3bd6eb4061ed74dbcfb9024b3e0618cf91869d8ea5613a0008968efd6dc.
The parent rebuilt it after Rust changes; reviewers used that binary rather than
independently building. This remains local source evidence, not deployment.
Final cargo fmt --check and git diff --check passed.

Final Jev1.14.0 advisory request
a5bca77ad4e4464b19494263bcba93c209dc847da0c1f0241d4ecdb0b78f4f15:
land_bounded_unit .72/revise .23/insufficient .05, selected-confidence .58;
669 input,56 output,.371s. This is a new post-revision/code-review disposition,
not a reroll or retroactive pass for the initial plan. Author accepts this scoped
source correction on concrete red/green tests and final clean independent reviews;
uncalibrated advisory uncertainty and full native/model/BiS gaps remain explicit.
