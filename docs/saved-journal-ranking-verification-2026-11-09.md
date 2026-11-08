# Saved-journal ranking — 9 November 2026

Base source: `3cd4566`. This increment connects automatic saved-page traversal
to a bounded combined weighted-DPS ranking. It remains offline; no game,
installation, deployment, public data sync or CI was used.

## Author checks

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_run_saved_journal test_gear_window_journal.GearWindowJournalTests.test_content_identity_binds_report_bytes_beyond_coverage_metadata -v
```

Two tests passed in 24.744 seconds at that revision. The main integration test
uses the actual local Rust stdio MCP, synthetic level-60 Orc Hunter captures,
three retained one-coordinate windows, rings 4549 and 4550, and synthetic
simulator numeric outputs. Two encounters use weights 1:3 and three iterations
per unique request. It asserts:

- Six invocations limited to one new run each perform exactly three new
  iterations each and finish six unique requests. Only the sixth invocation
  claims all saved resolved results complete.
- The combined ordering is ring 4549 at approximately +25%, baseline at zero,
  and ring 4550 at approximately −2.5%. A baseline repeated across pages appears
  once. References point back to windows 1, 0 and 2 respectively.
- A checkpoint-only rebuild performs zero new runs and reads eight page-local
  result instances. Rebuilding with page size one and top limit two returns the
  same leading sets and references.
- A one-page limit leaves the full-journal claim false and labels its ranking
  `completed_pages_only`. The checkpoint-only negative probes add no simulator
  calls. Source journal bytes remain unchanged and output omits private markers.
- A changed final journal content fingerprint rejects, even when other summary
  metadata stays equal. Changing the template between pages also rejects.
- Global qualification, feasible-set, BiS and game-execution flags remain false.

The focused SQLite identity test makes a valid synthetic report-body change,
updates its row digest and length, and observes a changed journal content hash
while every previous summary field remains equal. This confirms the new hash
binds more than aggregate coverage counts.

```text
python -m unittest test_run_journal_page test_gear_window_journal -q
Ran 10 tests in 24.291s
OK
```

An additional test invokes the actual new module in a child process against an
empty, valid SQLite journal with nonexistent MCP/engine/template paths. It
reports no saved work, no simulation and no complete-result claim, creates no
checkpoint folder and leaves the journal bytes unchanged. Invalid top, page,
page-count and iteration budgets exit one with empty stdout and sanitised
stderr. This test passed in 7.189 seconds:

```text
python -m unittest test_run_saved_journal.SavedJournalTests.test_real_module_empty_journal_and_invalid_options -v
Ran 1 test in 7.189s
OK
```

The real-engine path below this wrapper was qualified separately in
`real-saved-window-verification-2026-11-09.md`. These checks do not repeat that
qualification or establish simulator accuracy for a real character.

## Independent review

Two independent `gpt-6-astra` reviewers at high effort cover Callers and
Failure/security. Callers found no concrete defect and traced all existing
status consumers and the page, encounter, saved-window, preparation, merge and
simulator call chain. It ran:

```text
python -m tools.run_forever_saved_journal --help
exit 0
python -m unittest discover -s tests -p test_run_saved_journal.py -v
Ran 1 test in 28.450s — OK
python -m unittest discover -s tests -p test_run_journal_page.py -v
Ran 3 tests in 5.550s — OK
python -m unittest discover -s tests -p test_gear_window_journal.py -v
Ran 7 tests in 18.048s — OK
```

The additional empty-journal test was added after its discovery run and passed
in the separate author command above. Its independent invalid inherited
deadline probe rejected NaN, infinity, Boolean and expired values before
journal I/O. No reviewer edits, builds, Git mutations, real simulator runs,
native game actions, network calls, installation or deployment were authorised.

Failure/security also found no confirmed defect. Its main integration discovery
passed one test in 28.458 seconds; content-identity and subsecond-budget
regressions passed two tests in 0.040 seconds. Independent synthetic page-boundary
probes confirmed deterministic gear-digest tie ordering, duplicate removal,
bounded top-two output, checkpoint-only propagation after exhausted budgets,
one inherited deadline, incomplete-page exclusion, completed-page observation
retention, changed context rejection, empty-journal handling and rejection of
11 invalid budget/limit/weight inputs before source I/O. This did not separately
stress disk-quota exhaustion. No further review round was required because
neither lens confirmed a fault.

Scoped Jev advisory: `jev-1.14.0`, request
`4d80267caa8966c30bb32a293342f76f263b1e47c006a830f34d9539144cc96f`,
853 input / 54 output tokens, 0.309 seconds; land-scoped 0.84, fix-first 0.12,
insufficient-evidence 0.04, confidence 0.76. Advisory only, not proof or approval.

The broad goal remains active. All saved resolved results can be complete even
when the journal omits coordinates or ownership/model evidence is partial.
Those are separate requirements for full owned best-in-slot qualification.
