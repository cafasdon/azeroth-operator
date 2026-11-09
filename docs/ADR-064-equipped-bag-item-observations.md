# ADR-064: Observe equipped container item identity

Date: 2026-11-09. Status: accepted for offline source integration.

## Gap and primary evidence

Pinned simulator9dc32ac Hunter construction uses template quiver/ammunition
options to modify ranged speed/DPS. Existing captured bag capacities contain no
container item IDs, so they cannot establish those bonus identities. This unit
adds passive observations, not a simulator check or full ownership conclusion.

Pinned [Container API metadata](https://github.com/Gethe/wow-ui-source/blob/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/ContainerDocumentation.lua)
describes ContainerIDToInventoryID as a possibly unavailable mapping. Combine
that mapping with the already-used GetInventoryItemID(player,slot) getter;
never hard-code a bag-to-inventory mapping or infer missing item IDs as empty.
Native Forever availability remains unqualified.

## Capture and reader contract

Existing Forever bag_capacity rows can optionally include equipped_item with
exactly item_id (positive u32) and inventory_slot (1..255). Only non-backpack rows
with positive total_slots may carry it. Both values are required together;
explicit null, extra fields and duplicate inventory slots reject in Rust/Python.
Repeated item IDs in distinct inventory slots remain valid physical observations.

Guard required Container namespace/count/free getters before use. Optional
mapping and item getters must be public functions. Read mapping/item twice with
capacity/free/known-family rechecks, then check final namespace/function identity
after the last getter call. Omit metadata if unstable, malformed, restricted or
unavailable; retain the base row unless required APIs fail or change. Duplicate
observed mappings discard all implicated identities, even if a later optional
item read fails, preserving capacities.
A positive ID is an API-reported mapping observation, not verified bag category,
quiver bonus, complete equipment identity, accessibility or live ownership.
Two matching reads do not prove atomic state or native semantics.

## Exposure and boundaries

Existing raw inspection exposes the optional fields. Character summary adds
numeric bag/slot/item rows with false complete-identity/quiver/stock-credit/game
flags. Missing section yields null summary; a section without identities yields
zero rows, without proving no bags/quivers exist. No names/GUIDs are collected.
Item lookup, material stock, gear candidates and simulator inputs are unchanged.
No bonus is applied, removed or rejected from this observation yet. Ammo identity
and full model/owned-BiS qualification remain open.

Compatible strict readers must accompany future addon0.53.0-dev rollout; old
readers can reject the new optional object. This unit installs/deploys nothing.
[Offline evidence](equipped-bag-identity-verification-2026-11-09.md).
