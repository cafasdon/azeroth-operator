# ADR 111: Scale joint evaluation through bounded windows and compact summaries

Date: 15 November 2026. Status: accepted.

The whole-case evaluator deliberately caps matrices at 64 coordinates. Generated
gear/talent pools need larger products without unbounded native work, output,
memory or stored history. Raising the old cap would remove those controls.

Keep the original default/full-case API unchanged. Add an explicit window path
with the same global search identity and coordinate IDs. Materialise at most 64
coordinates per call. Preserve source/engine/context checks, the 100,000-trial
and 120-second comparison limits, output caps and existing private-cache policy.
Other coordinates remain unvisited, not excluded or validated by implication.

Add a pure incremental merger over supplied window receipts and a prior compact
summary. Validate their case/coordinate bindings, numeric shapes and coverage
accounting. Reject overlaps and retain only fixed top-K results plus compressed
coverage intervals. Keep work, input, interval count and output bounded. Do not
create a result archive, new database or automatic filesystem persistence.

Receipts and historical summaries are unauthenticated inputs. A merger can check
consistency but cannot prove they came from native execution. Label its coverage
as reported and keep execution, native eligibility, ownership, statistical
superiority and global-optimum flags false. Coordinate/window execution is
separate evidence. Missing old non-top rows cannot be retrospectively audited.

Test the full path using reference-valid gear/talent pools, actual native
windows, an independent Cartesian native traversal, and incremental top-K
agreement. Test terminal/gap/overlap/drift and huge synthetic offsets separately.
