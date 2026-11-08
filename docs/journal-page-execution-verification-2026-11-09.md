# Saved-journal page execution — 9 November 2026

This source/offline increment selects the next saved gear-window page and
delegates to the existing encounter executor. It reduces manual offset handling
for journals larger than one execution batch. It does not aggregate past-page
rankings, certify full owned BiS or run the game.

## Author evidence

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_run_journal_page -v
```

Initial two tests passed in 4.990 seconds. The indexed selection fixture stores
81 validated retained reports and reads windows 0–63, then 64–80, without source
journal mutations. Invalid limits/cursors and a corrupted selected report hash
reject. These synthetic report rows exercise paging; they do not constitute
81 qualified gear simulations.

The integration test collects three one-coordinate reports through the actual
Rust MCP using a synthetic level-60 Hunter and two bag rings. A synthetic
simulator boundary returns numeric results. A one-new-run budget first leaves
the selected two-window page incomplete, so its incoming cursor is retained.
Retry completes the page and returns cursor 1. The next invocation selects
window 2 and reuses the baseline checkpoint; there are three new engine-boundary
calls in total. No later saved row yields no continuation. An explicit cursor
after window 2 returns an empty page without additional engine calls.

Assertions preserve source journal bytes, omit private markers, and retain false
prior-page, global-ranking, feasible-set, BiS and game-execution claims. Module
`--help` is invoked as a real subprocess. No real simulator or live game was
needed for this wrapper change; the underlying real-engine qualification is
recorded separately in `real-saved-window-verification-2026-11-09.md`.

## Review and shared-deadline correction

Two independent `gpt-6-astra` agents at high effort reviewed Callers and
Failure/security. Callers reproduced a P2 budget defect: with a one-second
budget and 0.8 seconds of selection, rounding remaining work up to one second
renewed the delegated deadline to 1.8 seconds. A result at 1.5 seconds was
accepted because the wrapper also permits five seconds for finalisation.

The author reproduced the missing inherited deadline with a failing regression:

```text
python -m unittest test_run_journal_page.JournalPageTests.test_selection_cannot_renew_subsecond_work_budget -v
AssertionError: None != 1.0 : Selection must not renew the original work budget
Ran 1 test in 0.005s
FAILED (failures=1)
```

The surgical fix adds an optional inherited absolute deadline to the encounter
executor, validated and clamped to its own configured budget. The page runner
passes its original deadline instead of manufacturing an integer remaining
budget. Existing direct CLI defaults remain unchanged.

Jev triage: `jev-1.14.0`, request
`db6a73fb795e20ed5cbf2b8e883b01f9ac6dd3e66243bb3d65b2f725196f24b6`,
529 input / 50 output tokens, 0.272 seconds; fix-first 1.0. Advisory only.

After the fix, page and encounter tests passed six tests in 13.600 seconds.
Additional focused inherited-deadline validation/clamping and the original
regression passed two tests in 0.009 seconds. Expired, Boolean, nonfinite and
text deadlines reject before delegated work; shorter inherited deadlines are
preserved and longer ones clamp to the configured limit.

Failure/security's first round also ran two tests in 4.793 seconds and `--help`
with exit zero. Independent temporary-journal probes confirmed nonzero partial
cursor preservation, changed valid selected-report rejection, malformed next-row
cursor rejection and expired-budget rejection. It explicitly closed SQLite
handles. No reviewer performed builds, Git mutation, real-engine execution,
network calls, game access, installation or deployment.

Round two reran both lenses because the fix reached the existing encounter
executor. Callers passed six page/encounter tests in 14.93 seconds plus encounter
CLI help. Failure passed three page tests in 5.162 seconds and three encounter
tests in 8.730 seconds. Their independent probes traversed the real page and
encounter executors with a synthetic saved-executor boundary: 0.8 seconds of
selection preserved deadline 1.0, no second encounter began at the deadline,
the incomplete cursor repeated, and standalone encounter budgets were unchanged.
Both lenses reported no remaining findings. The additional focused validation
test described above was added after these combined runs and passed separately.

Final scoped Jev advisory: `jev-1.14.0`, request
`5780babf081c03f6a13884b891c6b6a300b2fdd4851111282dc6de8773b15c89`,
838 input / 54 output tokens, 0.336 seconds; land-scoped 0.99, fix-first 0.01,
insufficient-evidence 0.0, confidence 0.98. Advisory only, not proof or approval.
