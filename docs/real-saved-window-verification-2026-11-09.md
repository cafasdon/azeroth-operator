# Real simulator qualification of saved-window execution — 9 November 2026

This records the qualification at `14e9650`. The test was later extended with
[real combined-journal checks](real-saved-journal-verification-2026-11-09.md),
including more actual simulator invocations per test; the counts below describe
the earlier recorded version.

The saved-window encounter CLI now has an opt-in regression that exercises the
actual Rust stdio MCP and existing local simulator. No game client, native
interaction, installation, deployment or network data lookup was used.

## Recorded inputs and boundary

- Simulator: pinned WoWSims Forever revision
  `9dc32ac8ca0d8d1b76337861d38b93c0f884af11`, executable SHA-256
  `0e195f35c247d78f05c7d87c2c017e7052d7ca532500a84ae19faba133ae99a5`.
- Public Hunter template SHA-256
  `761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0`.
  Its rotation, talents, professions, consumables and buffs are template inputs,
  not observations of the user's character.
- Synthetic owner capture: level-60 Orc Hunter, professions Engineering and
  Blacksmithing, stipulated level-60 active pet, equipped items 2042 and 22347,
  and one alternative ring 4549 in a bag. This sparse synthetic equipment is
  deliberately different from the public template's equipment.
- Synthetic encounters: 45 and 90 seconds, zero duration variation and execute
  proportions, one explicit default target, 1,000 iterations, seed `20261109`.
  A third profile repeats the 45-second request. Weights are 1:3:1.
- Fresh temporary capture store, gear journal and per-user result checkpoints.
  The test requires the local catalogue and existing debug MCP executable.

## Actual command and result

```powershell
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
$env:AZEROTH_QUALIFY_ENGINE='C:/Temp/codex/forever-sim-9dc32ac/wowsimcli.exe'
$env:AZEROTH_QUALIFY_TEMPLATE='C:/Temp/codex/forever-sim-9dc32ac/public-hunter-template.json'
C:/Python314/python.exe -m unittest test_real_saved_windows -v
```

Initial result: **one test passed in 20.294 seconds**. After strengthening the
paired uncertainty assertions, the same real-engine command passed in
**19.346 seconds**. A separate invocation without the
two opt-in variables skipped one test in 0.000 seconds, with no downloads.
`cargo build --locked --offline` also succeeded before the passing run.

The test invokes `python -m tools.run_forever_encounter_windows` as separate
processes with no simulator or MCP stubs. It asserts:

1. Four invocations limited to one new run each perform 1,000 new iterations
   each. The first three withhold weighted rankings; the fourth completes the
   four unique gear/encounter requests.
2. A checkpoint-only invocation performs zero new runs and reports six hits
   across three profiles, including the repeated encounter. Its weighted result
   exactly matches the completed invocation.
3. Four fresh, uncached direct simulator runs with the same prepared requests
   exactly match every saved mean DPS and DPS standard deviation. Both
   encounter baselines have positive DPS. Independent sample-difference maths
   also matches saved paired standard errors, baseline delta intervals and the
   complete observed leading-pair result for both unique encounters.
4. The weighted alternative gain matches an independently calculated weighted
   sum from the individual encounter DPS ratios.
5. Substituting a synthetic level-20 capture rejects before reporting results
   and leaves the existing checkpoint SQLite file byte-for-byte unchanged.
6. Source gear journal bytes remain unchanged; CLI output excludes the private
   temporary path. Game execution, verified BiS and verified feasible-set
   coverage remain false.

There are eight actual simulator runs: four through resumed CLI execution and
four direct reference runs. This is one fixed-seed qualification of the
execution and persistence path, not a performance comparison, statistical
accuracy study, or proof that the sparse synthetic character models real play.
Full owned BiS, automatic all-window execution, native capture qualification,
complete equipment/model coverage and encounter mechanics remain unfinished.

## Adversarial evidence check

One independent `gpt-6-astra` reviewer at high effort covered failure/evidence
quality and callers. No product guard or production path changed, so one lens
was used. It ran the default skip command and found no blocking implementation
defect, but demonstrated that reversing checkpoint sample order preserves
aggregate mean/stddev while changing paired uncertainty:

```text
Mean/stddev assertions pass: True
Fresh paired error: 0.0
Permuted checkpoint paired error: 0.18174991893542358
Fresh interval: [10.0, 10.0]
Permuted checkpoint interval: [9.64377015888657, 10.35622984111343]
```

This was an evidence gap, not observed checkpoint corruption. The author
confirmed from the assertion loop that only the two aggregate fields were
checked. The expanded test independently computes paired uncertainty from the
same four direct reference runs; no extra engine runs were needed per test.

Jev triage was advisory: `jev-1.14.0`, request
`eb2e98ed477b3a0c18a14a777230f835f8ff04832617480962ff09f743cc2c26`,
522 input / 53 output tokens, 0.318 seconds; strengthen-test 0.98,
wording-only 0.01, insufficient-evidence 0.01, confidence 0.97.

The same reviewer checked the additions and reported no findings. Its synthetic
execution of the actual added assertion block passed for valid comparison data
and rejected reversed checkpoint samples. Its default discovery command was:

```text
python -m unittest discover -s tests -p test_real_saved_windows.py -v
Ran 1 test in 0.000s
OK (skipped=1)
```

No new caller or production code was introduced by the added assertions; the
same failure/evidence lens reviewed the follow-up. The reviewer did not run the
real engine; the real-engine evidence above is the author's direct execution.

Final scoped Jev advisory: `jev-1.14.0`, request
`ae107bf5b80c91f2caf3cb6c01a78c2c19170d3c17af28c41eb2d7e03a6c1b3a`,
645 input / 54 output tokens, 0.278 seconds; land-scoped 0.89, fix-first 0.08,
insufficient-evidence 0.03, confidence 0.83. Advisory only, not proof or approval.
