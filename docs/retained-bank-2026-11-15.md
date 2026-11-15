# Retained character bank — offline acceptance, 15 November 2026

## Behaviour

Each private character row now keeps one latest observed character bank with its
own client/build, character, capture time and snapshot ID. A later capture that
omits the bank keeps the current bank unknown and preserves that independent
observation. New bank data replaces old data; an observed empty bank replaces it
with zero. There is no list of historical versions or database schema migration.

Character and bank timestamps are ordered independently within the existing
write transaction. An older imported capture can supply a newer bank observation
than the stored bank, without replacing the latest character capture. Different
snapshots with equal timestamps reject the transaction. Legacy rows derive a
bank observation from their current bank during reads without a write.

`last_observed_character_bank` is separate from `storage`. It uses
`observed_quantity`, `observed_item_locations` and optional `observed_matches`.
The `current` and `available_stock` flags are always false. Current storage is
never populated from retained observations. Callers must not add retained counts
to current counts: the same item can appear in both at different capture times.
An equal snapshot ID means the observation belongs to the latest saved capture,
not that the bank remains current in-game. Build mismatch stays explicit.

The existing account/family/variant/realm/name key, account labels, shared-bank
ownership and mail rules remain unchanged. Retained data is never projected into
shared-bank rows. The existing transaction, database and response limits apply.

## Offline evidence

The 24 existing account-inventory tests passed, along with 14 new retained-bank
tests. The new suite covers omission, observed empty, unknown banks, independent
backfill, stale and duplicate imports, equal-time conflicts, shared-bank conflict
rollback, legacy rows, account/variant/realm/character separation, build changes,
item filters and absence of retained data in shared-bank rows.

Repeated replacements leave one character row and one unnested retained
observation. An oversized retained-bank import hit the existing SQLite quota
and kept the prior database byte-identical. A large detail query hit the actual
256 KiB response cap rather than truncate; its summary still worked, and reads
left database bytes unchanged.

An actual local stdio SDK subprocess read synthetic captures from an independent
working directory. Both MCP tools returned unknown current bank quantities and
the separately dated retained quantity, with false current/available-stock
flags. Existing MCP schema rejection and read-only checks also passed.

No game, actual private pilot database, original owner capture, live service or
configuration was accessed by the new acceptance tests. No live test claim is
made; native alternate-character and bank-window coverage remain untested.

One review round used two Astra/high reviewers for callers/failures and
security/data. Independent probes tested all 24 orders of four captures and all
120 orders of five captures, CLI import/get/find from an independent directory,
equal-time conflict rollback, old-build provenance and shared-row separation.
Both reviewers found no medium, high or low defect. No infrastructure drift
lens applied because no live service changed.

The first review probe left the ignored folder `local-data/review-bank-8sddapxt`
after its own unclosed SQLite handle prevented cleanup. Automatic approval review
then rejected its combined cleanup/re-run command with `blocked by policy`.
The command did not execute and cleanup was not retried. A corrected independent
probe closed its handles and removed its separate scratch directory. Product
assertions passed in both probes; the leftover folder is not product state.

## Advisory record

The first design advisory, Jev `jev-1.14.0`, selected revise with no generated
rationale: request `0065ec957d3d98e64a5e41f6ac04652e0be10729f1644ab2da573a60c8a18373`,
560 input / 47 output tokens, 0.307 seconds. A concern-classification call selected
freshness with a diffuse distribution: request
`702cfa0295cadbda5dca04ac2e849e6f99ad2a0c122dea4c53e26ec7760ce684`,
659 input / 73 output tokens, 0.279 seconds. These advisories remain recorded as
concerns, not passes. The final implementation uses separate observed-count keys
rather than a second current-storage-shaped object. The main session judged
that explicit separation plus false flags and independent caller tests address
the freshness concern. Independent review, not advisory probabilities, decides
whether that contract holds.

After implementation and independent review, the release advisory selected ship:
request `37eb15638b10be3b5784bd3c55fe3af889ed012f3b5b8a0651dca12a0efd173e`,
same Jev version, 668 input / 44 output tokens, 0.331 seconds. It does not erase
the earlier design concerns. The main session accepted the explicit output
contract, offline caller evidence and independent review as the release basis.
