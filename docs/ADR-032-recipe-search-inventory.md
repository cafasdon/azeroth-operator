# ADR 032: separate recipe discovery stock from recipe evidence

Date: 9 November 2026. Status: source/offline; no deployment.

Recipe review already permits newer inventory observations, but recipe discovery
only evaluates stock saved alongside the recipe. Users should be able to search
previously observed schematics after exporting changed bags without pretending
the older recipe knowledge was recaptured.

Add optional `inventory_snapshot_id` to `search_forever_recipes`, requiring a
`material_filter`. Read both snapshots through the existing owner-local store.
Reuse the existing comparison guard: same character name/realm, exact client
family/variant/build/interface and no older inventory timestamp. Use only the
explicit inventory snapshot for material quantities and storage coverage.
Missing inventory sections remain missing, including when the recipe capture
contains those sections. Keep original recipe rows, view filters and profession
coverage entirely tied to `snapshot_id`.

Opt-in results expose inventory ID plus recipe/inventory capture timestamps.
Omitted/null inventory selection preserves the old result shape and behaviour.
Keep existing public Rust search functions as forwarding wrappers around a new
inventory-aware function. Invalid requests return generic errors without raw
identity/path data. The same fixed-reagent support calculation remains in use;
no game action, external lookup, learning or craftability guarantee is added.
