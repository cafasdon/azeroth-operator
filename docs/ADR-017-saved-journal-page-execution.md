# ADR 017: bounded saved-journal page execution

Status: accepted, 9 November 2026.

## Problem

Saved-window simulation accepts at most 64 explicit offsets. Larger journals
can retain work beyond this bound, but the caller must assemble each next list
manually. A completed batch does not establish a global ranking or prove that
previous batches were executed.

## Decision

Add an offline journal-page CLI around the existing shared-budget encounter
executor. Select the next 1–64 saved rows in indexed coordinate order after an
optional exclusive cursor. Default page size is eight windows. Validate every
selected retained report under one read-only transaction, cap report bytes at
16 MiB, and reuse existing preparation, request deduplication, model guards,
engine identity and private checkpoints.

An incomplete page returns its incoming cursor and explicitly requests the same
page. A complete page returns the last selected start coordinate when later
saved work was observed; otherwise it returns no continuation. Checkpoint-only
inspection uses the same selection and fresh revalidation. Empty pages do not
launch MCP or simulator processes and do not claim completed simulation work.

Re-read the selected page and next-row metadata after execution, rejecting
changed selection, selected content or next-row observation. Previously
completed numeric checkpoints remain available. This is a point-in-time guard,
not cross-process locking or authentication of cursors. An explicit cursor may
skip earlier work, so prior-page verification and global ranking stay false.
Missing coordinate ranges, unresolved proposals and unqualified ownership are
still reported by the delegated selected-work result; reaching the last stored
row never proves full inventory or feasible-set coverage.

## Consequences

The caller can process arbitrarily many saved windows over bounded invocations
without constructing explicit offset lists. Page results remain scoped to that
page. Persistent all-page ranking, automatic scheduling across invocation
budgets, complete ownership/legality/model qualification and full owned BiS are
separate unfinished work. No journal schema or live-game behaviour changes.
