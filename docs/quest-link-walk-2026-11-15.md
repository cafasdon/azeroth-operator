# Quest-link walk — offline acceptance, 15 November 2026

## Contract

`World.walk_quest_links`, the `quest-links` CLI action and MCP
`walk_forever_quest_links` traverse recorded outgoing quest relationships in the
pinned public world cache. They preserve each source field and position: grouped
and alternative prerequisites, children, group peers, exclusions, breadcrumbs,
parent/next-chain and availability-related references remain distinct links.
They do not become one uniform prerequisite list or an eligibility calculation.

The root has depth zero. Default limits are three hops, 64 nodes and 128 edges;
maximums are eight hops, 128 nodes and 256 edges. Declared integer fields reject
booleans, floats, strings, zero and out-of-range values. Breadth-first traversal
visits each quest once, with deterministic field/position order and no repeated
expansion of cycles or shared nodes. It follows recorded links only, not reverse
links inferred by scanning other quests.

Nodes contain exact public summaries, source record hashes, depth, found/status
and link counts. Missing target records remain explicit nodes. Unsupported
targets have null target IDs; malformed grouped fields also have null positions.
Neither case becomes an empty or satisfied prerequisite. Unknown roots report
`root_not_found`. Available nodes with no recorded links are distinct terminals.

Frontier entries group omitted-link counts by source quest and stop reason:
`depth_limit`, `node_budget` or `edge_budget`. Counts distinguish data gaps from
budget stops. `stored_graph_complete` is false when either exists, and its scope
is only `reachable_recorded_outgoing_quest_links`. The existing 256 KiB response
cap applies without silent truncation.

The source revision/cache hash, generation counters and record counts remain
attached. Complete-world, current availability, character-state, quest-eligibility,
route and live/game-execution claims remain false. No character inventory, quest
log, gameplay operation, localisation or dynamic correction is added.

## Offline checks

Ten new tests passed through synthetic checksum-pinned caches and the real
`World` constructor. They cover every recognised relationship field, positions,
cycles/shared references, missing/unsupported targets, malformed groups, unknown
roots, exact traversal boundaries, dense/wide graphs, types and ranges. The actual
response byte limit failed a large synthetic response. Existing get/search
results stayed unchanged. Actual CLI and stdio MCP calls ran under external
network and file-write audit guards.

The existing offline public world MCP test also passed, including get/search,
paging, public-source hashes and the new three-tool source inventory. It reads
public reference data only. A separate local-cache walk of quest 2, Sharptalon's
Claw, returned seven nodes and 19 edges at default depth. Its depth frontier kept
`stored_graph_complete` false. Missing/unsupported counts were zero. Every found
node's hash matched its exact source record and the cache bytes stayed unchanged.

No original owner data, game or hosted service was accessed. Live and native
quest qualification remain deferred. A cached graph cannot prove current faction,
phase, level, quest completion or quest eligibility.

One review round used two Astra/high reviewers for callers/failures and
security/data/semantics. Independent models matched 324 and 200 graph/budget
cases, including exact references, positions, hashes, count conservation, bounds
and complete reachability. Four incompatible CLI option scopes rejected with
empty stdout and unchanged cache bytes. No medium or high defect remained.
A low stale two-tool documentation count was corrected to three tools in four
prose lines, with no runtime change. No open new low finding remained. A live
infrastructure drift lens did not apply.

The local `azeroth-world` allowlist now enables the third tool. Only that list
changed. Explicit UTF-8 bytes, an ignored private backup, semantic comparison,
exact readback and native parser checks protected staged/saved configuration.
An actual SDK launch from an independent directory used those saved parameters
and the existing public pinned cache. All three tools appeared; quest 2 returned
the same seven nodes and 19 edges with explicit incomplete/no-live-state flags.
Cache bytes remained unchanged. No app restart or hosted deployment occurred.

## Advisory record

Jev `jev-1.14.0` selected proceed for the design: request
`1587e642dffd1ee3ad7c333744635d00970d7363e54ca6c19338e2db47d4147b`,
630 input / 47 output tokens, 0.292 seconds. This was advice, not test evidence
or authority to publish.

The low documentation finding was classified as trivially fixed by the same
Jev version: request
`b4aaa971c0700459c49143fa5650fbe02181d3b5270ecbdbebef80040af1a486`,
500 input / 54 output tokens, 0.315 seconds. The main session accepted the
reproduced count mismatch and the prose-only correction.

The release advisory selected ship: request
`d713e9441de7ea979df4f5740de7e622e39a7bf6d611523037c20c5d30122c8b`,
same Jev version, 667 input / 44 output tokens, 0.270 seconds. Direct and
independent evidence, not advisory probabilities, formed the release basis.
