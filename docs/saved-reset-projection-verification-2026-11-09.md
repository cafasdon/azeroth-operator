# Historical reset projection — 2026-11-09

Baseline b99acbc. Previous goal turn made progress by correcting skill capture.
This unit adds task-level reset-time projection over existing saved observations.
The broad MCP/native/full-owned-BiS goal remains active and incomplete. No live
client, account data, addon installation, hosted MCP or deployment used.

## Author evidence

```powershell
cargo test --locked --offline --lib saved_resets
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
python -m unittest test_saved_reset_projection test_skill_line_filters
```

Initial unit fixture used i64::MAX, outside the existing signed i32 schema; it
correctly failed validation (compile 5.78s). Corrected fixture to i32::MAX and
added a direct Chrono maximum-date overflow check. One Rust test passed with
56 unrelated filtered out, compile 3.98s, runtime .00s. Ordinary MCP build passed
in 15.80s. Initial Python fixture included an unsupported client version field;
Rust correctly refused the stored snapshot. Removing that test-only field gave
2 scoped actual-MCP tests passed in 1.001s. Neither failure required production
source changes or relaxing a guard.

Checks cover day rollover, signed negative/zero/max duration, checked timestamp
bounds, separate cache age, numeric slot order, retained extended/locked flags,
missing versus known-empty sections, unchanged raw data, false-flag compatibility,
wrong sections/family, foreign owner IDs, malformed flags with private canaries,
incompatible skill filters and unchanged stored bytes. Tests use synthetic data
and the real local Rust stdio server. No full-suite or CI replay.

## Advisory plan

Jev resolved jev-1.14.0, request
`a739db2cf80b0704cde24cfcee2fd1dc8589e8ffa68daa3e97e9d11daf4f540f`:
proceed .82 / revise .12 / insufficient .06, confidence .73; 459 input /47 output
tokens, .347s. Advisory only; independent checks determine source integration.

## Independent review and source identity

Two independent read-only gpt-6-astra high reviewers, Callers and Failure/privacy,
ran the prebuilt Rust test (1 passed /56 filtered) and actual-MCP test (1 OK).
Callers additionally checked 128 rows against independent datetime arithmetic,
fractional seconds/leap day/signed endpoints, numeric order, maximum cache age,
raw preservation, optional schema and omitted flag. All 23 other Boolean review
modes rejected composition. Failure/privacy ran 152 additional actual MCP calls:
100 independent calendar trials, 20 malformed stored sections, 23 mixed modes
and nine malformed private flags. No canary leakage; synthetic bytes unchanged.
Both documentation readbacks found no concrete contradiction. No confirmed
review fault. Reviewer usage/runtime model IDs unavailable; no builds, source
writes, Git mutations, network, game or deployment. Live drift lens omitted
because installation and live testing remain deferred.

Ordinary MCP SHA-256:
`74fbb8b5f0228ce11f4bc40a87ca84311636d10a1a845cb59f819f12c3a343dd`.
`cargo fmt --check` and `git diff --check` passed.

Final advisory Jev resolved jev-1.14.0, request
`aa157b01d544190849f4c56146bbef33c387e7f436417c6e23ac1e3e2ef5dfe7`:
integrate .91 / revise .05 / insufficient .04, confidence .86; 525 input /52
output tokens, .303s. Author accepts the bounded source feature on direct and
independent evidence, without native qualification or broader completion.
