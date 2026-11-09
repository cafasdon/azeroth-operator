# ADR 034: enumerate equipped-source vacancies in exact positional searches

Date: 9 November 2026. Status: source/offline; no deployment.

Rust selection review already removes an equipped source from its previous slot.
The qualified positional engine preserves empty slots. The search controller
nevertheless excludes every move whose equipped source is not also a selection
target, omitting representable layouts from all-owned positional evaluation.

Allow these drafts only in explicit positional mode and only when the emptied
source slot also belongs to the requested search slot set. Map requested simulator
slots through the adapter's existing pinned slot-index mapping. Keep target
selection, source uniqueness, physical-copy equivalence, native/catalogue
category/level/modifier/class and weapon pairing checks unchanged. Legacy mode
and single-slot searches retain their prior exclusions.

Preserve `max_swaps` as a bound on selected replacements, matching existing CLI
help and enumeration. A move is one selection even though its source empties;
do not charge an additional implicit replacement. Existing coordinate accounting
and retained-spec hashes describe the newly resolved layouts. Collection software
fingerprints change, so old bound collection journals cannot silently resume
under the changed search policy.

Use ordinary MCP review, retained journal preparation and qualified-engine gates.
The result remains a historical desired layout with source selections and empty
equipment slots, not a move sequence, displaced-item storage assignment, native
eligibility proof or executable gameplay action. Full owned BiS remains open.
