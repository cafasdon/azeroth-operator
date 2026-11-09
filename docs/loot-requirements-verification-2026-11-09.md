# Conditional loot requirements — 9 November 2026

Baseline `1b22a4e`. Opt-in saved loot comparison on existing owner-local inspection.
No addon/schema/package change, external lookup, installation, deployment or
native/live game test.

## Author checks

`cargo fmt` and `cargo build --locked --offline` passed; ordinary build 15.35s.

```text
C:/Python314/python.exe -m unittest test_loot_requirements test_loot_window test_snapshot_unavailable -v
Ran 3 tests in 1.202s
OK
```

Environment: `PYTHONPATH=.;tests`, `TEMP=TMP=C:/Temp`. Actual Rust stdio tests use
an independent arithmetic oracle over mixed, missing, empty, unknown-only,
64-slot maximum and 64-distinct-item captures. They check numeric slot order,
request ordering, duplicate base-item aggregation, zero readable quantities,
u64-max demand versus 64 × u32-max readable units and all conditional fields.
Fifteen malformed request variants plus wrong-section/family cases reject.
Missing review stays null, omitted/null options preserve legacy output and source
bytes remain unchanged. Item search still sees two bag units and recipe capacity
zero despite abundant displayed loot; no stock credit is granted.

## Advisory and review

Plan: `jev-1.14.0`, request
`7ba1ab4f0faee7970bbce033bf6f6e8450305ab13bc1fef34feb9e7161f7a7c8`,
663 input / 47 output tokens, 0.310s. Revise 0.69, proceed 0.26,
insufficient evidence 0.05, confidence 0.53. Author removed the proposed full-
window quantity field, retaining readable-row lower bounds and conditional demand
arithmetic only. The initial advisory remains recorded; no favourable reroll.

Round 1 used independent `gpt-6-astra` high-effort Callers and Failure/security
lenses. Callers ran the three author modules (1.136s) and independent actual-MCP
probes for schema/read-only annotations, u32-max IDs/u64-max demand, 65 distinct
requirements, null/absent sections, five incompatible-selector cases, malformed
captures/path/missing-hash privacy, null opt-out and immutable source bytes.
Failure/security ran the new test (0.537s), then independently rejected 12 malformed
stored captures, six malformed requirements and five path/context combinations.
Its maximum-value probe confirmed exact conditional arithmetic and timestamps.
Neither found a runtime fault, ownership credit or unintended action.

Callers did find a flaky test assertion: omitted/null responses were compared
including per-call `age_seconds`. Its same-capture two-query probe across a second
boundary differed only in that field. Author confirmed the existing `Utc::now()`
calculation and changed only the test: assert integer ages, exclude them from
stable response equality. The new test passed again in 0.410s.

Triage: `jev-1.14.0`, request
`a17538af3da9aac13722ad582ce34f3ff61fe482ab231d8d345c85f8bda3dfc8`,
507 input / 47 output tokens, 0.298s. Fix required 0.98, insufficient/not material
0.01 each, confidence 0.97. Author accepted the concrete finding. Round 2 Callers
was clean: new test passed in 0.464s, and the original same-capture clock-boundary
probe showed ages 58509920/58509921, original full equality false and corrected
stable equality true, both age types integer. Failure/security omitted because
runtime and its reviewed security boundaries are unchanged. Actual agent usage
was not exposed. All probes used synthetic data and parent-built executables;
no builds, project edits, network, native/game operations, deployment or SQLite.

Final source/offline advisory: `jev-1.14.0`, request
`889113fca2bc54bbdc4848de4060cfcb89c0ba229027bcaced6e052054f10621`,
957 input / 52 output tokens, 0.309s. Land scoped 0.81, fix required 0.12,
insufficient evidence 0.07, confidence 0.72. Uncalibrated advisory only; the
earlier revision and uncertainty remain recorded. Author disposition: land
conditional saved-loot comparison; native rights/availability and wider coverage
remain open.
