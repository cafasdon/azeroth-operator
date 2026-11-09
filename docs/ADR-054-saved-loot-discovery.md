# ADR-054: Discover historical displayed-loot observations

Date: 2026-11-09. Status: accepted for offline source integration.

## User task and interface

Find which saved capture displayed an item or raw currency ID, or contained a
money slot, without manually inspecting every historical loot window. Extend
the existing owner-scoped read-only `list_snapshots` route with optional
`loot_observation`. No additional route, addon API or data-section schema.

Examples:

```json
{"family":"forever","loot_observation":{"kind":"item","item_id":10,"minimum_quantity":5}}
{"family":"forever","loot_observation":{"kind":"reported_currency","reported_currency_id":10,"minimum_quantity":0,"maximum_quantity":0}}
{"family":"forever","loot_observation":{"kind":"money"}}
```

The strict tagged alternatives are:

- `item`: positive u32 item_id; minimum_quantity defaults to one; optional
  maximum_quantity. Bounds are u64 and minimum must not exceed maximum.
- `reported_currency`: positive u32 reported_currency_id and the same bounds.
- `money`: presence only, no ID or quantity/amount fields accepted.

Forever family is required. Unknown or mixed fields/tags, wrong numeric types
and reversed bounds reject before scanning. Omit the filter for old results;
explicit null follows the existing optional-parameter convention.

## Matching and returned evidence

Within one hash/schema-valid capture, sum only matching readable rows of the
selected kind and exact ID. Item and currency namespaces never mix, even if
IDs coincide. Money matches one or more money rows and has null quantity.
At least one actual matching row is required, including for a zero minimum;
missing sections, known-empty windows or unknown rows cannot prove absence.
There is no pooling across captures, characters, inventories or currency wallets.

Each matched summary adds `loot_observation_match`: kind, coverage, basis,
observed_quantity (null for money), numeric matching slots and their recorded
quantities, matching_slot_count, unreadable_slot_count and
additional_matching_rows_unknown. Item/currency reports include requested bounds
and the selected ID. Slots are ordered numerically. Existing snapshot identity,
capture time and character provenance remain in the surrounding summary.

Quantity bounds apply to the **recorded readable sum**, not the actual total
behind unknown rows. Unknown matching possibilities remain explicit even when a
maximum predicate passes. Raw currency basis is
`raw_slot_info_before_container_conversion`; this is not a reward transformation.
Money has no copper estimate, even when several money slots are recorded.

Output explicitly keeps counts_toward_owned_inventory,
counts_toward_owned_balances, ownership_verified, lootability_verified,
currency_rewards_verified, money_amounts_verified, source_identity_verified,
game_execution and live false. No item acquisition, location/source attribution,
spendable funds, recipe material or current ownership is inferred.

## Existing discovery semantics

The filter composes with character, time, profession, inventory and other saved
capture predicates. `latest_per_character` selects the newest valid capture
within the requested family/date window for each character/realm/client variant
before feature predicates. A newer valid capture with missing or nonmatching
loot suppresses an older match. Invalid/hash-mismatched files are excluded and
counted invalid under the pre-existing store contract, rather than becoming
authoritative new observations. Sorting and pagination follow matching.

No active-character inference or default changes. This is a historical discovery
query, not a current loot-window signal. Private owner directories remain
separate; no external data source, network request, simulator or game action.

## Verification limits

Direct Rust tests and actual local stdio MCP fixtures cover strict alternatives,
quantity namespaces, duplicated rows, u64 sums, zero/presence, unknowns,
latest/date/character/composition, pagination and separate-owner stores.
Independent reviewers also probe invalid filters and persisted snapshots.
Hosted deployment, native capture acceptance, full loot/reward semantics and
fresh active-character identity remain unqualified.
See [dated evidence](loot-discovery-verification-2026-11-09.md).
