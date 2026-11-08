# ADR 015: Budgeted execution of selected saved gear windows

Date: 9 November 2026. Status: source/offline implementation.

A new explicit CLI connects ADRs 012–014: fresh MCP preparation of selected
retained windows, compatible request aggregation, and private numeric checkpoints.
It does not execute supplied prepared JSON. Every invocation reruns the existing
snapshot/equipment/character/pet/profession/native guards before using results.
An explicit absolute trusted engine and exact SHA are required.

One invocation selects up to 64 distinct exact window starts. Preparation is
bounded to 16 MiB and each MCP window has at most 45 seconds within the overall
work budget. Only missing unique requests execute: at most 128 new runs and
100,000 new iterations per invocation, with lower configurable limits. Default
new-run limit is 16; default work budget is 180 seconds, configurable 1–3,600.
Budget values are not job identity, so a retry may adjust them.

The engine is verified before the invocation, by each new fixed run before and
after execution, and before final reporting. Post-run verification precedes
checkpoint persistence/return, preventing ordinary executable drift from leaving
an unverified result reusable under the expected engine hash. This is drift
detection, not process attestation against malicious substitution/re-signing.

Checkpoint hits are read directly after fresh preparation and initial identity
verification; the final check covers the invocation. Each key retains the full
request/engine/mode/software identity from ADR 014. No checkpoint is evicted.
Corruption/identity/model failures stop safely; already verified results remain.

Completed numeric rows use the existing comparison arithmetic, extracted into
a non-mutating helper. Paired candidates retain a common baseline and paired DPS
leader/runner-up statistics. Rank choices are DPS, damage taken, threat, deaths,
healing and time out of mana. Numeric summaries are streamed one at a time,
retaining baseline and at most two leader sample arrays, rather than loading all
paired histories. Output is bounded to 16 MiB and excludes full requests/paths.

The work deadline stops new work. Completed-row comparison and final identity
checks share a bounded five-second finalisation allowance. Checked filesystem/
SQLite/subprocess deadlines are not hard real-time promises. Preparation failure
can stop without a progress object; numeric checkpoints from earlier invocations
remain available. Expired work after a completed read/run retains that completed
row in the structured report when finalisation succeeds.

Reports distinguish completed results, missing results and requests not inspected
within the budget. Cached-only retries report no new simulation. Ranking is
complete only for the explicitly selected resolved requests, not an unconditional
best-in-slot claim. Source coordinate gaps/unresolved proposals remain visible;
native legality, ownership/model/encounter completeness, automatic all-window
orchestration and multi-encounter global ranking remain separate requirements.
