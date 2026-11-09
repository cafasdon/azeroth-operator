# ADR 049: saved-character item quantity discovery

Date: 9 November 2026. Status: source/offline integration.

Item lookup already reports locations in explicitly selected snapshots. Add an
optional `inventory_item` predicate to list_snapshots so an agent can discover
which historical character captures record a needed base item without fetching
every snapshot or consulting an external catalogue.

Require item_id (positive u32), storage (equipment, bags, character_bank or
account_bank) and minimum_quantity (u64); optional maximum_quantity is inclusive
and cannot be below minimum. There is no all-storage option. Equipment/bags
apply to supported client families; bank filters require Forever.

Count matching equipment locations as one each and matching container stack
counts with checked u64 addition. Missing/unavailable storage stays unknown and
does not match even a zero minimum. Captured empty storage has an observed zero.
Amounts describe the saved observation, not all owned/accessibly transferable
items or current location. No duplicate account-bank ownership is inferred.

Match each capture independently. Existing character, time, family, other
feature predicates and pagination compose normally. Existing latest_per_character
selects the newest valid capture for each name/realm/variant within requested
time bounds before feature filtering. A newer zero or missing observation thus
suppresses older positive stock; an explicitly older time bound may select the
older historical capture. No default active identity or guessed current player.

Each matching metadata row adds `inventory_item_match`: item ID, storage,
observed quantity and basis `same_snapshot_single_storage`, with false ownership,
access, shared-scope verification, cross-character aggregation and game execution
flags. No item payloads, GUIDs, links, filesystem paths or cross-capture totals.
Shared-bank matches remain separate even when two characters record the same
amount. Ordinary listing output is unchanged without the predicate.

The owner-local store, validation, containment and scan limits remain in force.
No new tool route, addon, sync, external data call, game operation, installation
or deployment. Native completeness, active identity, transfers and full owned
BiS remain outside this historical discovery contract.

## All-item requirements

Optional `inventory_requirements` extends discovery to a shopping or reagent
list. It requires one storage and 1..64 distinct items with positive u32 item_id
and positive u64 quantity. It is mutually exclusive with inventory_item. Bank
family restrictions and other listing predicates remain unchanged.

Scan the selected section once, accumulating only requested IDs with checked
u64 addition. Every requirement must be covered by observed quantities in that
same capture/storage. Partial lists in different characters, storage sections or
capture times are never pooled. Missing, unavailable, empty or short storage
does not qualify. Newest-before-filter selection still prevents old-stock
revival; explicit historical date bounds remain supported.

`inventory_requirements_match` retains only requested item IDs and required /
observed quantities in request order, with basis same_snapshot_single_storage
and all_requirements_covered_by_observed_quantities=true. Ownership, access,
shared-scope, aggregation, craftability, transferability and execution remain
unverified/false. The predicate can compose with captured profession/recipe
filters, but material support and those observations do not establish current
craftability. An omitted character may be unobserved or nonmatching; omission
does not prove they lack the items or cannot craft.
