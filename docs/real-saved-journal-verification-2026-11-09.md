# Real simulator qualification across saved pages — 9 November 2026

This extends the opt-in qualification introduced at `14e9650` to the automatic
saved-journal runner added at `67e157c`. Production code is unchanged. The
test runs actual Python module entrypoints, the local Rust stdio MCP and the
existing pinned simulator. It uses no simulator or MCP response stubs.

## Fixture and entrypoint

The engine SHA-256 is
`0e195f35c247d78f05c7d87c2c017e7052d7ca532500a84ae19faba133ae99a5`;
the public Hunter template SHA-256 is
`761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0`.
Both are asserted before execution. The pinned revision remains
`9dc32ac8ca0d8d1b76337861d38b93c0f884af11`.

As in the earlier qualification, the capture is deliberately synthetic: level-60
Orc Hunter, stipulated professions and active pet, sparse equipped items 2042
and 22347, and bag ring 4549. The public rotation, talents, buffs and consumables
are template inputs, not observations of the user's character. The encounters
are 45 and 90 seconds with one default target, zero duration variation and
execute proportions, 1,000 iterations and seed `20261109`. The 45-second profile
repeats, and weights are 1:3:1.

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
$env:AZEROTH_QUALIFY_ENGINE='C:/Temp/codex/forever-sim-9dc32ac/wowsimcli.exe'
$env:AZEROTH_QUALIFY_TEMPLATE='C:/Temp/codex/forever-sim-9dc32ac/public-hunter-template.json'
C:/Python314/python.exe -m unittest test_real_saved_windows -v
```

The initial extended command passed one test in **55.679 seconds**. A separate
invocation with both opt-in variables absent skipped one test in 0.000 seconds
and performed no download.

## Additional assertions

The existing checks remain: four one-new-run encounter invocations, complete-only
weighting, checkpoint-only reuse, four uncached direct engine reference runs,
exact aggregate metrics, independently calculated paired uncertainty and
weighted gains, and unsupported-level rejection.

The extension collects the same gear work through the real MCP into two
one-coordinate saved windows, then invokes `tools.run_forever_saved_journal`
with page size one:

- With the existing checkpoints, both saved pages complete with zero new runs
  and nine page-local hits across three profiles. Repeated baseline work and
  the repeated encounter reuse exact cached results.
- Combined gear identities, weighted gains and per-encounter mean DPS match
  the direct-engine-qualified original comparison. Saved references identify
  window 0 for the baseline and window 1 for the ring alternative. Explicit
  assertions also require exactly one row per distinct gear hash and the
  expected descending score order with gear-hash tie breaking.
- A new checkpoint directory requires four invocations with one new run and
  1,000 new iterations each. Only the fourth marks every saved resolved result
  complete. The ranking and metrics match the original comparison exactly.
- A checkpoint-only rebuild then performs zero new runs, reports nine hits
  and returns identical combined rows and references.
- Both source journals remain byte-for-byte unchanged. A synthetic level-20
  capture rejects through both module paths and leaves both checkpoint SQLite
  files byte-for-byte unchanged. Verified BiS, feasible-set and global-ranking
  claims remain false; game execution remains false.

There are twelve actual simulator runs per passing test: four original resumed
runs, four independent uncached references and four cold combined-runner runs.
Warm/checkpoint-only rebuilds and unsupported-level probes add none.

This is one fixed-seed execution/persistence qualification with sparse synthetic
gear. It does not establish encounter accuracy, real-character modelling,
complete ownership, full feasible search or owned best-in-slot. No game client,
installation, deployment or network data lookup is involved.

## Independent evidence review

One independent `gpt-6-astra` reviewer at high effort covered failure/evidence
quality and callers. Production code is unchanged, so one lens was used. It
verified the default opt-in skip path and traced the real combined, page,
encounter, saved-window and checkpoint callers without invoking the real engine.
It found no blocking defect, but showed that dictionary conversion in the
combined-output helper hid duplicate rows and ignored output ordering:

```text
ascending ranking: accepted by check_combined
duplicate gear: accepted by check_combined
```

The author reproduced both cases by extracting the actual helper with the AST
and injecting synthetic results. Existing synthetic integration tests already
covered ordering and uniqueness; this was a gap in the new real-engine
qualification. Explicit row-count and ordered gear-key assertions were added
before dictionary conversion.

Jev triage: `jev-1.14.0`, request
`5306fb6961df704d5c7c8e68ef9ccf804f34b15e6e2f8f64cda553fca6266290`,
585 input / 54 output tokens, 0.328 seconds; strengthen-test 0.97,
none-material 0.03, insufficient-evidence 0.0, confidence 0.96. Advisory only.

The same independent reviewer then exercised the updated helper with bounded
AST/in-memory probes:

```text
valid sorted: accepted
ascending ranking: rejected
duplicate gear: rejected
```

No new findings remained. No new caller or production file was touched, so the
same failure/evidence lens handled the follow-up. The author reran the complete
opted-in real-engine command with the stronger assertions: **one test passed in
56.392 seconds**. Each passing version executed twelve real simulator runs as
described above; the second invocation was justified by the new assertions.

Final scoped Jev advisory: `jev-1.14.0`, request
`62b1043da4d57eb16fedc6b333a89dba861238aa69077c517bef70ed19fea4b0`,
830 input / 54 output tokens, 0.269 seconds; land-scoped 0.96, fix-first 0.01,
insufficient-evidence 0.03, confidence 0.94. Advisory only, not proof or approval.
