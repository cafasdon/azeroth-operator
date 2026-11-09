# ADR 045: bounded local source-entity locations with scoped chain projection

Date: 9 November 2026. Status: source/offline; no installation.

Existing local reagent references retain source IDs but cannot locate those
entities. Extend optional `recipe_item_sources` with an entity-location cache,
using public reads from the separately installed provider. No provider code/data,
network sync or hard dependency is bundled. Existing licensing caveats remain.

Derive unique NPC-drop/vendor and object-drop IDs only from retained reported item
categories. Count all retained IDs and query the numerically first 32 of each kind.
An optional cache is emitted only when at least one public entity API is available.
Keep existing provider identity/mode/contract checks around the entire capture.
Protected per-entity double reads preserve existence status and optional spawns;
unreadable spawns do not discard known existence or item references.

Bound area enumeration and total point positions independently at 4,096. Validate
dense arrays and exact coordinate tuples, retaining 16 sorted positions including
duplicates, with reported count/truncation. Preserve provider area IDs and percent
coordinates, optional positive provider phase IDs and two-element instance
sentinels. Phase zero normalises away on coordinates. No map conversion, native
phase interpretation, names, GUIDs, paths or deprecated health fields.

Strict readers recompute reference ID sets/counts and exact first-32 cache keys.
They enforce point/count/shape/range limits without demanding post-serialization
floating-point sort order: exporter formatting can collapse nearby values and
change tuple ties. A recorded point is reference data, not a native position proof.

Chain source reviews return one shared projection for only their retained source
IDs. Unrelated cached entities are excluded; absent queried records remain
`not_captured`, never provider absence. Preserve capture provenance, partial
category/point coverage and false frame/visibility/native/routing verification.
No stock credit, movement, route choice, acquisition action or current availability
claim. Full world coverage and native qualification remain open.
