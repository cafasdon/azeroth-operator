# ADR 039: distinguish no fixed-item limit from unknown capacity

Date: 9 November 2026. Status: source/offline; no deployment.

Explicitly empty fixed reagent lists already cover any requested item count.
Their finite material capacity is null, as it is when material evidence is
unavailable. Combining that null with saved charges previously discarded a
known charge limit, including zero, for recipes with no required items.

Preserve existing finite `material_capacity` shape. Add per-scope
`material_capacity_basis`: `no_required_items` only for fixed-item recipes with
explicitly empty requirements, `observed_required_items` for numeric observed
capacities, otherwise `unavailable`. Missing storage never creates an empty
requirement list, and unsupported requirements cannot take the no-item branch.

For explicit no-item requirements, the item-and-charge limit equals the saved
charge count when known, irrespective of storage observations. Otherwise retain
the minimum of known item capacity and known charges. Missing charge counts
remain unknown; timers are not charge counts and elapsed time does not refill
them. Original recipe evidence remains authoritative with newer stock.

This applies through existing selected/bulk review and its nested batch/chain
callers, with no new request field, schema, addon or permission. Capacities are
conditional fixed-item/charge arithmetic, not current craftability, learning,
tool availability, shared cooldown groups or complete ownership proof.
