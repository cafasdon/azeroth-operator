# ADR-065: Partial bag identities in quiver simulation context

Date: 2026-11-09. Status: accepted for offline source integration.

## Decision and model boundary

Project equipped bag identities from strict saved bag_capacity rows into MCP
equipment reviews as captured_equipped_bag_items. Null means section unavailable;
[] means no positive identities were captured, not no quiver. Each numeric row
contains bag1..5, item_id positive u32 and inventory_slot1..255, with unique bag
and inventory slots. Standalone preflight rejects malformed/extra fields too.

Use pinned9dc32ac proto/hunter.proto and sim/hunter/ammo_auto_gen.go reference
entries inline in the existing fingerprinted adapter: none plus five quiver
identifiers/haste values. Accept documented enum names or their integer IDs,
with existing snake/camel alias handling and duplicate-alias rejection. Null
selection follows the existing protobuf-option default. Reject unsupported
numeric enum values: protobuf can preserve an unknown number, but the engine's
string-keyed quiver lookup otherwise produces a zero-valued effect silently.
This guard belongs to the single-player review/preparation workflows; raw engine
or lower-level arbitrary-request APIs are not newly qualified by it.

For Hunter, report template item/reference haste, captured identity count,
observed pinned quiver IDs, unmapped observed-item count, highest observed pinned
reference haste and nullable below-reference comparison. Comparison distinguishes
capture unavailable, unverified none, observed item match and not observed in
partial capture. A partial mismatch is diagnostic only, never inferred absence
or a reason to silently modify a user-selected hypothetical template. Matching
an ID does not establish max applied bonus or native effect semantics. Repeated
quiver item IDs remain physical rows but do not add reference haste together.
Other specialisations receive not_applicable after fragment validation.

## Pipeline

Standalone request_from validates before engine/cache execution; the CLI adds
hunter_quiver_observations without raw private metadata. Preparation carries
only the validated numeric fragment and checks equality across draft reviews.
Prepared merging validates it, requires consistent observations across groups,
and recomputes diagnostics from request options. Legacy missing fields remain
unknown; conflicting null/empty/value groups reject rather than backfill.

Saved-window output carries the diagnostic. Encounter aggregation requires the
same observation context across profiles; saved-journal contexts check it across
pages and retain it in final output, including the unified evaluator's existing
simulation object. Numeric simulation requests/ranking remain unchanged for
valid selections. The existing adapter/preparer/MCP source fingerprints bind
new logic; no separate untracked reference module is introduced.

## Limits

Complete equipped-bag identity, applied quiver bonus and ammunition identity
remain unverified. No quiver candidate search, optimisation, template correction,
new engine effect, addon/schema, gameplay route or live deployment. The mapping
is a pinned simulator reference, not full world data or game-effect proof.
[Offline evidence](quiver-simulation-context-verification-2026-11-09.md).
