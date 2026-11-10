# Bounded exact public reference walks

The local public knowledge companion can follow recognised cached reference fields
across records, for example item → NPC → zone. This is a graph over the pinned
catalogue, not a world-route planner or proof of current acquisition availability.

`get_forever_knowledge_record` accepts `include_references=true` and
`reference_depth=2` or `3`. This returns `catalogue_reference_walk` instead of the
default paged `catalogue_references` preview. Depth one and omitted options retain
the existing response. Deep walks require `reference_offset=0`.

Breadth-first traversal expands each resolved record once. Cycles and repeated
edges retain their exact source JSON pointers. Nodes carry identities, names,
shortest discovered depth and exact source-row SHA-256 hashes, without duplicated
full child records. Fetch a node through the existing exact-key tool for its body.
Missing targets and invalid IDs remain explicit edges; no similarity fallback or
inferred joins fill gaps. Listed suffix options remain catalogue possibilities,
not owned variants.

Node and edge budgets default to 64 and 256, with maxima of 256 and 1,024.
Traversal has a five-second analysis deadline. Exhaustion returns explicit partial
status, discovered unexpanded records and depth-boundary records. A node-budget
stop can retain the triggering edge with `target_node_included=false`.
`complete_within_requested_depth` means the recognised cached walk was processed;
it does not mean all endpoints resolve or the world graph is complete. Consult
`unresolved_reference_count` and edge status separately.

The existing 256 KiB whole-response limit rejects oversized responses without
silent truncation. The index is checked once per lookup. No embedding query,
index rebuild, private snapshot access, external lookup, installed configuration
change or native game action is added. Search retains its direct-reference shape.

CLI equivalents are `get --include-references --reference-depth 3`, optionally
with `--reference-node-budget` and `--reference-edge-budget`. Custom walk budgets
require depth above one; walk options are rejected for other CLI actions.
