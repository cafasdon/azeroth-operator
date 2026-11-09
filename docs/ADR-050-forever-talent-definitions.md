# ADR 050: Forever native trait-definition observations

Date: 9 November 2026. Status: source/offline capture qualification.

Owned-gear results depend on the build. The adapter currently validates an
explicit template talent string but does not compare it with native allocated
talents. Existing captured entry IDs are not simulator definition IDs. The
pinned WoWSims class talent tables include definition IDs, providing a future
mapping reference, but observation and model conversion need separate proof.

Use the existing current-player C_ClassTalents/C_Traits APIs for Forever rather
than assuming old global talent functions or choosing another player's data.
Primary pinned [SharedTraits API](https://raw.githubusercontent.com/Gethe/wow-ui-source/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/SharedTraitsDocumentation.lua)
defines nullable entry definitionID, maxRanks, node ranksPurchased,
entryIDsWithCommittedRanks and ConfigHasStagedChanges. No import strings, UI
loading, talent purchase/refund, commit/rollback or spec selection is introduced.

Forever capture now reads a bounded base graph twice: exact active config,
1..16 distinct tree IDs, dense per-tree node arrays, at most 4096 unique nodes
across those trees, and exact active node/entry/rank identities. Public values
and fixed API context are checked before comparisons. Secret, malformed,
incomplete, duplicate or changing base data makes the section unavailable.
The original Retail collector remains unchanged.

New root observed_tree_ids and observed_node_count describe the queried arrays.
Every retained active node keeps the existing entry_id/rank/tree_id fields.
Optional definition_id/purchased_rank/max_rank form an all-or-none tuple, kept
only when two public reads agree. Positive purchased ranks require exactly one
committed entry matching the active entry. Ambiguous, unavailable or subtree
entries without a definition omit the tuple, not the stable base node. Explicit
zero purchased rank is valid. Active rank may exceed purchased/native max due to
native effects; it is never coerced into spent talent points.

Four protected staged-state samples surround the observation phases. The optional
staged_changes_observed flag appears only when all are readable and agree; it
is an observed flag, not a whole-model certificate. The final staged sample is
inside the complete base-graph bracket. Forever config names are not emitted.
The existing unsupported talent-import status remains unchanged.

Readers require Forever for the new metadata, canonical observed tree IDs,
node membership, count bounds and complete definition tuples with purchased
rank no greater than native max. A summary reports coverage counts and observed
staged state while leaving allocation_match_verified and
simulation_mapping_verified false. This does not independently verify the old
talent schema's complete cross-reader parity.

No automatic simulator conversion or known talent-profile match is introduced.
Next work must bind a pinned class-specific definition/rank mapping, handle
unknown/granted/boosted entries and pending changes, and separately verify the
model request. Native availability/completeness and current character remain
unqualified. Addon 0.47.0-dev and matching readers are source/offline only and
remain uninstalled; no deployment or live game test.
