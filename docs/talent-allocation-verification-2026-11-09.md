# Queried talent allocation and preparation — 2026-11-09

Baseline a1f7e27. The preceding goal turn made progress with saved-loot discovery.
This unit addresses the missing inactive-rank evidence for simulator preparation.
The original broad MCP/native/owned-BiS objective remains active and incomplete.
No game, installation, deployment or hosted endpoint access performed.

## Source and ownership

Pinned public SharedTraitsDocumentation.lua from Gethe revision
9dc84f0d3bbae1768ac3405b3716000bd93eda06 documents entryIDs,
entryIDsWithCommittedRanks, ranksPurchased, GetEntryInfo definitionID/maxRanks
and subtree fields. The existing local cache was read; no inferred legacy talent
encoding or third-party private data was used. Pinned map remains SHA-256
33dc07ad7ba838a4bb6f95d825df44235b0e1a00e92eb3dfdb9c68d8d96c18b3.

One scoped gpt-6-astra high writer owned only the Lua collector/new Rust fixture,
without builds or live actions. Parent owns strict readers, MCP preparation,
Python/engine tests, packaging and documentation. Agent token usage unavailable.

## Author checks and concrete corrections

- Initial scoped Lua allocation/legacy-definition tests: 4+3 passed, .63s/.52s,
  compile 21.04s. Covers active boosted rank7/purchased3, inactive/unselected zeros,
  metadata uncertainty, duplicate/budget/secret/subtree/pointer/context changes,
  empty and 4096-entry boundaries, and omission beyond the entry budget.
- Author found an absent boosted active choice could create a group rejected by
  the reader. Specific red fixture failed with `Active entry missing from
  definition allocation`, .57s after 1.36s compilation. Added only the collector
  membership check; whole group omits while base entry remains. Green allocation
  and old-definition tests: 4+3 passed, .74s/.60s after 1.49s compilation.
- Ordinary MCP builds passed at 15.12s and 2.92s as implementation progressed.
  Actual MCP tests reached all nine prepared strings and refusal cases but found
  default Boolean deserialization echoed a malformed private string flag.
  A field-specific Boolean deserializer now emits a fixed error; real Booleans
  only remain accepted. Red canary assertion then green full targeted Python
  checks: 5 passed, 4.443s after 7.91s rebuild.
- Author added a 65-entry/single-node malformed group. Both readers accepted it,
  producing a red Python assertion and red actual MCP rejection assertion.
  Added only matching per-node 64-entry guards; global 4096 stays unchanged.
  Rebuilt ordinary MCP, 11.45s. Expanded Python suite with explicit real-engine
  environment: 3 passed, 1.889s. Rust allocation/legacy tests subsequently passed
  4+3, .60s/.51s after 15.23s compilation.

Commands were direct scoped invocations, no CI/full-suite replay:

```powershell
cargo test --locked --offline --test forever_talent_allocation --test forever_talent_definitions
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
python -m unittest test_talent_allocation test_mcp_talent_simulator test_forever_talent_definitions
```

Python cases cover exact prepared ordering for all nine classes, old comparison
and omitted-mode behaviour, missing/extra definitions, native maximum conflicts,
unknown/true staging, missing class/allocation, malformed linkage/counts/fields,
unsupported families and mutually exclusive modes. Invalid raw hash-valid
snapshots reject in both readers without private-field/path leakage.

## Actual offline engine

Explicit local qualified positional engine SHA
833ff767872622329beed4317bea8127caa2e5911f174f8bc336d845b8e94983;
public Hunter template SHA
761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0.
The opt-in test creates synthetic complete native-rank observations from the
public template, requests the string through actual MCP, then compares two
uncached seeded engine runs: 200 iterations, seed20261109, 45 seconds. Returned
metrics match exactly after substituting the prepared zero-padded string.
This is request/model integration, not independent accuracy or native proof.

## Uninstalled package

`cargo build --locked --offline --example verify_addon` passed, 1.94s.
Package/verify commands used Forever/live/interface16001/version0.50.0-dev in
dist/forever-talent-allocation-0.50.0-dev. Exact four-file source/manifest checks,
packaged Lua5.1 roundtrip, Forever capture and changed-interface refusal passed;
live_qualified=false. Archive SHA:
519470c357c5a6adc080a1edd84ecf8ae4146b0b8b70d2070ae62a43d7bf8cda.
Collectors SHA 1e2b1770a50b2b9fe1432affc3fb3b64f92e38f04d3f734d7a9aa9946d6f588f;
Core unchanged 28f8eded6884753ce25ff2aa4ee08db3e803424cd2df093faf15d27eb401b03f.
No installation or native qualification follows from this package.

## Independent review

Two read-only gpt-6-astra high reviewers used prebuilt tests/actual MCP, no builds
or source mutations. Initial Callers tests: 4+3 Lua and ten Python checks (one
engine skip); independent flag/schema/old-reader projection probe passed.
Failure: 4+3 Lua, two Python checks, 117 independent reader-parity mutations
(116 reject, one valid accept) and ten malformed private-canary flags passed.

After the author-confirmed per-node limit correction, both reviewed the fix and
ran independent rebuilt-binary probes. Node sizes 63/64 accepted,65 rejected;
65 entries split across nodes accepted; 4096 total accepted,4097 rejected.
Failure again rejected ten malformed flags without leakage. Both final code
rounds were clean; actual-engine test skipped in reviewer environments without
explicit opt-in paths. Author ran it as recorded above. Native/deployment drift
lens omitted because those operations stay deferred. Final doc read-back follows.

## Advisory receipts

Jev 1.14.0, advisory only; uncertainty retained rather than re-rolling:

- Plan9ea68a225d010b85a1f39342f81052fd0ac5c76580f310af5964603ad3b15093:
  proceed .67/revise .15/insufficient .18;743 input,47 output,.365s.
- Active membership debd560f5f1682a4a639d5b617ccf0361c4aeb90dc195c6f3922a130d5103e5f:
  fix .99/dismiss .00/insufficient .01;496 input,44 output,.311s.
- Flag echo3894e12b0c3b8b15d5a99b09f3b620a854ecf66bd62488e42f16c86d9897a621:
  fix .82/dismiss .05/insufficient .13;458 input,44 output,.374s.
- Per-node cap78dbd83fda8bfe0220d7cf0bfe8fbe4dc4b95e2548bbdc1088ea0b0f9ec6997a:
  fix .97/dismiss .02/insufficient .01;458 input,44 output,.293s.

Doc read-back: Failure found no overclaim or contradiction. Callers found the
Python environment prefix was pseudocode rather than executable PowerShell.
Replaced it with the actual separate environment assignments and test command.
Author executed that Python invocation: 6 tests, 3.348s, OK with one explicit-engine
skip (the actual engine run was performed separately above). No runtime source
changed. Triage Jev1.14.0 request
e8f21049cbf55a5deb74169c2fdd6417828d2c69f4cdf9a4f6b50ed7655e5adc:
fix .95/dismiss .02/insufficient .03;426 input,44 output,.309s.

Final documentation read-back is clean after the PowerShell correction. Callers
confirmed command syntax and distinction between the six-test run with one skip
and the separate actual-engine run. Failure's contract/privacy doc read-back was
already clean; it was not repeated for this command-only correction. No runtime
source changed during documentation rounds. Final code reviews were clean after
the per-node guard correction. Formatting/diff checks passed.
Reviewed ordinary executable SHA-256:
ec43d8fb3848d85abe24e3e5edab74a1456bad0470e063d96a8ee19aeabb8b25.
Reviewers did not independently build source; parent build/test records establish
the local verification chain, not hosted or native deployment.

Final Jev1.14.0 advisory request
cb812f787ca9c2a25ca895c63e216e8d76b9adb4208a89550234d1bc26c2e8a8:
land_bounded_unit .96/revise .02/insufficient .02;682 input,56 output,.340s.
Author accepts source integration on concrete tests and clean independent review.
Full native/model/legality/active identity and broad owned-BiS goal stay open.
