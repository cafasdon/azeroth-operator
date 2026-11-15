# Local recipe dependencies — offline acceptance, 15 November 2026

## Contract

`LocalForever.recipe_dependencies` and
`get_local_forever_recipe_dependencies` walk the first-party SQLite reference.
Edges alternate from a recipe to its recorded ingredient item, then from the
item to recipes with a recorded output of that item. The graph keeps observed
reagent quantities and optional output quantity ranges. Producer recipes are
alternatives, not an automatic choice or an assertion of usable variants.

The root recipe has depth zero. Each graph edge adds one hop. Defaults are four
hops, 64 nodes and 128 edges; maximums are eight hops, 128 nodes and 256 edges.
Inputs reject booleans, floats, strings and out-of-range values. Existing read-only
database validation and the 64 KiB response limit remain effective.

Each node gives its exact record key, observed public fields, depth, expansion
status and omitted edge count. An item with no recorded producer reports
`no_observed_producer`; it is not declared a base material or unobtainable item.
Stops instead report `partial` with explicit frontier reasons: `depth_limit`,
`node_budget` or `edge_budget`. A visited set prevents repeated expansion of
cycles and shared dependencies. Sorted breadth-first order is deterministic.

`stored_graph_complete` refers only to the reachable stored ingredient/producer
graph. The entire index still covers one incomplete capture. All recipe coverage,
craftability, unlock prerequisites, ownership, variant equivalence and generated
plan claims remain false. No aggregate material plan, inventory subtraction,
learning route, acquisition route or game operation is generated.

## Offline checks

Seven new tests and all twelve existing local-reference tests passed. Synthetic
validated addon captures passed through the actual snapshot store and builder.
Tests cover a multi-step chain with exact quantities/yield ranges, alternative
producers, cycles, a shared subgraph, each traversal budget, exact-bound completion,
unknown roots, recipes without observed reagents and unknown producers.

Direct calls rejected every invalid input type and bounds case. A real oversized
public-field response hit the existing byte cap. SQL trace statements were reads
only; source/database files stayed byte-identical. Private sentinel identity,
inventory counts and source-only recipe state did not appear in graph output.

The real CLI returned the same graph as the direct method. An offline stdio SDK
process exposed the read-only tool with strict integer fields and returned that
graph. Invalid declared integer values failed. FastMCP ignores undeclared extra
arguments, as it does for the existing local tools. Such arguments cannot override
the configured database or depth. Callers must use the declared argument names.
The server test blocked network use. No original owner capture,
actual pilot database, game or hosted service was used. Native recipe and complete
progression coverage remain untested.

One review round used two Astra/high reviewers for callers/failures and
security/data. Independent graph-oracle probes checked 216 and 144 graph/budget
cases respectively. Exact edge membership, closure completeness, terminal labels,
omitted counts and all three traversal bounds matched the independent models.
No medium or high defect remained. Infrastructure drift did not apply because
no hosted service changed.

The caller review found the low-severity ignored-extra-arguments behaviour above.
Declared values still determine the graph; extras cannot override the configured
database or declared depth. The main session accepted low severity because the
behaviour matches existing tools, affects undeclared input only and exposes no
new data or path. It is queued in
[issue #82](https://github.com/cafasdon/azeroth-operator/issues/82), with review on
29 November 2026. Documentation now describes strict declared integer fields
rather than a schema that rejects every unknown field.

The local `azeroth-local` configuration enables the third tool. Only its
`enabled_tools` list changed. Explicit UTF-8 byte writes, a private ignored backup,
semantic comparison, native parser checks on staged/saved files and exact byte
readback protected the configuration. The saved launch parameters passed an SDK
test with the database argument replaced by a synthetic database. All three tools
appeared and the graph matched the direct call. This is not acceptance against
the actual pilot database, game or hosted service.

The planning advisory used Jev `jev-1.14.0`, request
`c3f88e57b37eac744987f9c6e632d557e4ffcf1385f3a85fd3735ecc89cdae3d`,
527 input / 47 output tokens, 0.280 seconds. It selected proceed. This was advice,
not test evidence or authority to publish.

Finding triage used the same Jev version, request
`da743661aa80ffe8f55c9e2d501f1bc1317846980939e351d832d0acd8828ae2`,
594 input / 51 output tokens, 0.297 seconds. It selected low. The main session
accepted the reproduced scope and retained the finding in the follow-up issue.

The release advisory selected ship: request
`85ae4ebca8883e5670138c766ad410db89038609e7b8da2256e17b52cd8754bf`,
same Jev version, 698 input / 44 output tokens, 0.306 seconds. The main session
accepted the direct and independent checks as the release basis. Live tests
remain deferred at the user's request.
