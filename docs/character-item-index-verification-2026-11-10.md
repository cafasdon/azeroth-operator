# Historical character item index: offline evidence

Scope: ADR-097. Actual local stdio executable with synthetic owner-local stores.
No game, installed profile, external source, deployment or live test changed.

Direct author commands:

- `cargo build --offline`: passed, 24.06 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_character_item_index_mcp`:
  1 passed, 0.410 seconds.
- `PYTHONPATH=.;tests python -B -m unittest test_character_item_index_mcp test_item_cooldown_lookup`:
  2 passed, 0.877 seconds. TEMP/TMP used C:/Temp/codex for legacy fixtures.
- `git diff --check`: passed without output.

The new SDK test covers eight selected captures and five exact identities:
three successive captures of one character, tied latest captures of another,
the same name on another realm, observed empty storage and unobserved storage.
It asserts selected/candidate IDs, no stale bank fallback, separate account-bank
counts, no pooled total, primary-first ordering, unknown versus zero, explicit
historical limitations, and absence of item links in the new projection.

Twenty-four seeded input permutations (seed 97), a moved primary capture for
the same character, and an out-of-range location page preserve the index.
Schema/default compatibility, single-capture mode, invalid flag shapes,
duplicate/excess/foreign-store captures, exact-build/family refusal and unchanged
saved bytes are checked through the actual MCP. Existing cooldown/location/
multi-capture behaviour also passed its scoped test. The implementation reuses
the successful per-capture query's complete quantity map rather than rescan.

Independent review round: callers and failure/privacy, each requested on
gpt-6-astra at high effort, both clean. Callers reran the actual SDK unittest:
1 passed, 0.497 seconds. Failure/privacy reran it independently: 1 passed,
0.418 seconds. No source findings or corrections were needed.

Independent actual-executable SDK probes confirmed UTC-instant ordering and
equivalent-offset timestamp ties, exact-case identity separation, primary-first
ordering even for an older capture, all storage absent, and stable quantities
with container/stack/cooldown/large-offset pagination options. Removing the new
index from a combined-options response reproduced the legacy response exactly.
Both lenses checked incompatible client components and invalid/missing capture
IDs. Failure/privacy also changed a synthetic stored file; content-hash refusal
returned no partial identity, private marker or path. One caller probe initially
used a Retail fixture with unsupported banks, then corrected it to test the
actual family refusal. No validator or guard changed.

Root judgement: ready for source integration within the explicit historical
capture scope. One clean round, two independent lenses. Paid Jev did not run
under the user's spending stop. Active identity, current/full ownership and
native acceptance remain unproven; live tests and deployment remain deferred.
