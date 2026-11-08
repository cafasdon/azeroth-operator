# Gear plan-window journal — 8 November 2026

Baseline `f5ed2b4be69a3d227fc42054d7bd8beb29b644ce`.
Source/offline synthetic verification only. No native game interaction,
simulation, installation, deployment or external data sync.

## Contract

The new module accepts an explicit `--journal` path and optional `--add` report.
Input uses the merger's shared strict 4 MiB JSON parser and report validation.
A positive coordinate window is required. SQLite application ID, version and
exact schema identify the application format, without claiming authentication.
Foreign/nonregular/symlink database files reject. New POSIX files use mode 0600;
current-UID/private-mode checks apply where supported. Windows ACLs are inherited
from the caller's chosen location and have not been independently qualified.

Canonical reports and their SHA-256 are retained in the local database. Exact
retry does not add a row; conflicting same-offset or overlapping windows reject.
Other scopes/baselines reject against the stored anchor. Indexed decimal keys
preserve coordinates beyond SQLite's signed-integer range. Appends are atomic
with a five-second busy timeout. The default payload budget is 64 MiB, explicit
maximum 512 MiB, with a million-row upper bound. Database/index/journal overhead
is separate from that payload budget. There is no automatic pruning or deletion.

Read-only status revalidates each stored payload and its digest, bounds and
scope, and computes global gaps. It streams beyond the merger's 64-artifact
input limit, returns at most 64 gap details and preserves the full gap count and
next uncovered offset. SQL and streaming checks have a 45-second deadline;
failure does not emit a partial completeness result. SQLite value/row retrieval
is bounded at 4 MiB plus 4 KiB framing, with 64 KiB SQL and bounded schema-query
results. This uses SQLite's documented runtime length limit for strings, BLOBs
and encoded rows ([official limits documentation](https://www.sqlite.org/limits.html)).
Python 3.11+ supplies the required connection-limit API.

Status emits only saved scope/snapshot references, counts, gaps and false
feasible-set/BiS/simulation/game-execution claims. It does not expose raw report
fields, invoke MCP or a simulator, discover files, or collect the next window.
Appending confirms storage and denies BiS/game-execution claims. Saved reports
are user-local data, not trusted executable instructions.

## Author evidence

With `PYTHONPATH=.;tests`, direct command
`python -m unittest test_gear_window_journal -q`
passed the initial four functional tests, 18.784 s. They cover out-of-order
resumption, idempotent retry, byte-identical rollback after scope/quota/overlap
failure, foreign schema/data protection, busy locking, stored corruption,
abandoned uncommitted transaction recovery and actual module restart/status.
They stream 66 reports to cover 257 coordinates and separately verify 65 gaps
with only 64 returned details. Forced deadline failure leaves saved data intact.
Private fixture markers do not enter status or CLI failure output.

Initial author fixture issues were corrected: a foreign-file comparison reused
the wrong byte snapshot, and Python SQLite context managers did not close test
connections for Windows cleanup. Explicit connection closing fixes subsequent
fixture lifecycle. Automatic approval review later rejected manual recursive
cleanup of `C:/Temp/tmpr5g1wvbg` and `C:/Temp/tmphd9l__6r` as "blocked by policy";
those earlier synthetic test folders were left untouched. No alternate deletion
was attempted. A reviewer fixture also remains at
`C:/Temp/gear_review_zx0qznns/oversized.sqlite`; it was not manually cleaned.

Jev plan advisory: `jev-1.14.0`, request
`81ab6d1342acf8fdcba6a728f9a9453ec7f9c415d3acea70266242137fde5d1b`,
648 input / 47 output tokens, 0.307 s; proceed probability 0.86. Advisory only.

## Independent review and correction

Round one used independent reviews configured as `gpt-6-astra`, high effort;
per-review token usage was not exposed. Callers was clean: six explicit journal
and merger parser/metadata tests passed, 22.061 s. It also checked module import
without inherited `PYTHONPATH`, 41-digit coordinate ordering, out-of-order
appends and byte-identical baseline-mismatch rejection. Its first broader test
invocation was interrupted after identifying a later local-MCP producer test;
the explicit six-test command completed. No native game action was involved.

Failure/security ran four journal tests (16.094 s) and reproduced one resource
fault: an 8 MiB stored BLOB with a false size field was fetched into Python before
the 4 MiB check. The author added the regression and reproduced a validator-call
count of one before rejection. Connection-level SQLite length limits now reject
oversized BLOB/text values before Python row validation, including duplicate and
anchor append paths. SQL length and schema-result bounds were also installed.
The schema/version is unchanged. A valid exactly-4-MiB report still fits.

Final author tests: six passed, 17.611 s. Round-two Failure/security review was
clean: six passed, 17.399 s. Its independent original 8 MiB corruption now raises
SQLite DataError with zero Python row-validator calls and unchanged database
bytes. Tests cover oversized report/digest/start/end values across all retrieval
paths. Callers was omitted in round two because the resource guard/test fix
reached no additional caller. No builds, simulations or live changes were made
by reviewers.

Shared-merger strict parser and metadata regressions passed separately: two
tests, 4.213 s. The actual journal module help entry point and whitespace checks
passed. No CI or pipeline ran.

Jev finding triage: `jev-1.14.0`, request
`156e09815da09af1589e23bb538b9d36506621517bcb541de8e78178dd98d4ee`,
600 input / 45 output tokens, 0.312 s; fix probability 0.92. Advisory only.

Post-correction Jev advisory: `jev-1.14.0`, request
`ed06ca2b219ee1df4e67ca70e5111c8062fd2971b16d9ae5eb7dc6f594497af4`,
700 input / 49 output tokens, 0.290 s; land-scoped probability 0.96. Advisory only.
Automatic collection, simulation aggregation, full owned BiS, active-character
qualification and the broad feature objective remain unfinished.
