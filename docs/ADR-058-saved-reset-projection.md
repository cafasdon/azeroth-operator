# ADR-058: Historical saved reset projection

Date: 2026-11-09. Status: accepted for offline source integration.

## Decision

Add optional Boolean `inspect_snapshot.saved_reset_projection`, default false.
True requires Forever and exactly `saved_instances` or `saved_world_bosses`.
Other existing review arguments retain their own section/combination checks.
Malformed flag types reject with fixed text, without echoing private values.

Keep raw `data` and the inspection envelope intact. Add sibling
`saved_reset_projection` with capture timestamp, reported cache-update age,
recorded/projected/unknown counts and numerically ordered slot rows. Each row
preserves its observation and adds nullable `projected_reset_at` and a status.

For a nonnegative recorded signed i32 duration, compute capture timestamp plus
seconds with checked timestamp arithmetic. Zero projects exactly to capture
time. Negative durations yield `negative_reported_duration`, and timestamp
limits yield `timestamp_out_of_range`, both with null timestamps. Never clamp
negative durations to zero, interpret them as expired, or advance by wall clock.
The cache update age is separate evidence and is not subtracted from durations.

Missing sections yield null rows/counts/cache age. Known empty captures yield
empty rows and zero counts. Existing strict schema validation caps each section
at 128 dense slots. Counts precede no truncation: all validated rows are returned.
False or omitted flag preserves the previous output shape.

## Boundaries

The output is historical arithmetic using recorded values, not a verified
server reset schedule. Cache age, capture timing and native API semantics remain
unqualified. Extension/locked flags stay raw: they neither suppress rows nor
prove extension outcomes. Coverage, current lockout, current availability and
extension-outcome verification flags remain false. Projected times in the past
do not establish availability. No cross-capture join, active-character default,
account sharing, gameplay execution, addon change or external sync is added.

See [offline evidence](saved-reset-projection-verification-2026-11-09.md).
