# Historical character item index

Date: 10 November 2026. Status: source/offline only.

`find_snapshot_item.character_item_index` is an optional boolean, default false.
It supplements the existing per-capture item results for up to eight explicitly
selected owner-local, exact-client-compatible Forever snapshots. It does not
discover every character, read another store or select the character in play.
Omission/false leaves the existing response unchanged.

The index reuses full `quantity_by_storage` counts from the existing item query;
it does not reread snapshots or repeat inventory scans. Item location pagination,
container details, cooldown options and stack-group options retain their existing
meanings. Index quantities stay independent of location page size and offset.

Group by exact saved name and realm, consistent with existing identity checks.
Do not case-fold, trim or infer aliases. Within each group use only a unique
maximum capture timestamp among the explicitly selected snapshots. Equal latest
timestamps are ambiguous even if some counts agree: report candidate snapshot
IDs, null selected ID and null quantities. Older selected captures count toward
provenance but never fill missing storage in a newer capture. Missing storage
stays null; an observed empty storage or no matching item gives zero.

The primary input's character appears first. Other rows sort by exact name and
realm; tied candidate IDs sort lexicographically. Primary means the explicitly
selected input character, not active/current. Each row retains capture timestamp,
selection status, selected/candidate IDs and older-capture count.

Equipment, bags and character-bank quantities form a per-character map. Account-
bank quantities remain a separate observation per row, never an additive shared
pool. Account identity is not captured. No cross-character total is returned:
asynchronous captures can show the same physical item before and after transfer.
No transfer reconciliation, simultaneous ownership, complete character coverage,
current inventory, native identity or item-variant equivalence is established.
No game action or inventory mutation occurs.

Direct validation: `cargo build --offline` and
`PYTHONPATH=.;tests python -B -m unittest test_character_item_index_mcp`.
Native acceptance, active-character qualification and deployment remain deferred.
