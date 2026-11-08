# ADR 018: saved-journal execution and bounded combined rankings

Date: 9 November 2026. Status: source/offline implementation.

## Problem

The page runner returns a continuation cursor and page-local comparisons. The
caller still needs to execute each page and reconcile its results with earlier
pages. Per-page winners cannot identify the highest observed weighted DPS across
all saved resolved gear requests.

## Decision

Add `tools.run_forever_saved_journal`. Each invocation starts at the beginning,
visits up to a configured page budget, and delegates to the existing page and
encounter executors under one absolute work deadline and shared new-run and
iteration limits. Already completed numeric work is reused through the private
simulation journal. Exhausted run/iteration budgets switch later work to
checkpoint-only mode. The first incomplete page stops traversal; retry the same
invocation to make further progress. No persisted ranking state is trusted.

Keep only the best 1–20 distinct simulator gear hashes, sorted by weighted DPS
point estimate then gear hash. Record a concrete saved-window offset and local
review reference for each retained set. Duplicate gear does not consume another
leaderboard place. This bounds ranking memory without retaining every result or
every per-iteration sample across pages. Page-size and page-progress limits
remain independently bounded; at most 64 progress summaries are returned.

Comparability requires identical per-encounter engine, profile, fingerprints
and encounter settings across pages. Observed baseline gear, exact request,
iterations, seed, mean and standard deviation must match whenever available.
The source snapshot/scope must agree with the whole journal. A new content
fingerprint in `journal_status` hashes the ordered validated row boundaries and
report digests. Compare complete journal status before and after execution to
reject persistent source drift even when aggregate coverage counts are equal.
This is a local consistency guard, not authentication or a lock against
transient change followed by restoration.

Claim `all_saved_resolved_results_complete` only after reaching the saved end,
processing all saved rows and completing every comparable weighted result.
Otherwise rank only observations from completed pages and label that scope.
An empty journal, missing results, nonpositive baselines, time or page budget
exhaustion cannot produce the complete claim. All verified global-ranking,
feasible-set and BiS claims remain false. Source coverage gaps and unresolved
coordinates remain visible in whole-journal metadata.

## Consequences

The caller can execute and compare across saved pages without assembling offset
lists or manually merging page winners. Repeated invocations rebuild the
ranking from fresh reviews and durable numeric checkpoints. Large journals can
still exceed the scan/preparation deadline; there is no hard real-time promise.
Full owned BiS additionally requires qualified ownership, legal sets, metadata,
model and encounter coverage. Live testing and deployment remain deferred.
