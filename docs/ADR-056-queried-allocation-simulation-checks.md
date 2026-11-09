# ADR-056: Check queried allocation in simulation preflight

Date: 2026-11-09. Status: offline source integration.

## Concrete gap

ADR-055 can prepare a simulator string, but previous equipment-review projection
and explicit-template comparison still used only active entries. A saved queried
definition with zero purchased points and no active node could therefore be
changed to one point in a simulator template without rejection. The author
reproduced that path through actual MCP equipment review and request_from.

## Decision

Project optional definition_allocation alongside the existing numeric talent
observation in equipment reviews. The new group contains only public IDs/ranks,
counts and the fixed queried-coverage label; names and import text remain excluded.
It stays bound to the selected validated owner capture and receives no current
character or native acceptance claim.

When the group is present, Rust MCP comparison and Python simulation preflight
use all its explicit definition ranks, including zero, rather than the active
subset. Each known pinned definition is compared only when its native maximum
agrees with the pinned maximum. Purchased rank is compared, never active boosts.

For known false staging, any known rank conflict rejects simulation. True staging
continues to reject; unknown staging stays unverified as before. Missing pinned
definitions are not filled with zero. Unmapped definitions and cap disagreements
remain explicit unknown model evidence. A known partial match is not full-build
or model proof; all existing full allocation/native/model verification flags stay
false. Preparation's stricter exact-set requirement is unchanged.

## Coherent comparison reports

For a queried group, basis becomes
`captured_queried_definition_allocations_and_pinned_definitions`. Counts and the
single conflicts list refer to that comparison universe:

- queried_definition_count: all captured definition entries.
- compared_definition_count: pinned definitions with matching native maximum.
- unmapped_definition_count: captured definitions absent from the pinned map.
- unknown_node_count: distinct queried nodes containing unmapped definitions.
- missing_pinned_definition_count: pinned definitions not explicitly captured.
- rank_limit_disagreement_count: matching IDs with different native maximum.

Definitions can share a node, so unmapped definitions and unknown nodes can have
different counts. Conflicts retain only numeric ID, captured purchased rank and
template effective rank. Template digits use the existing pinned effective-rank
rules. With no allocation group, the old basis, fields and active-node semantics
are preserved exactly.

Standalone exported equipment reviews do not receive implicit trust from a prior
MCP call. The Python checker validates the group before comparing: exact keys,
coverage/counts, canonical u32 IDs, u32 ranks/native maxima, node/tree/active-entry
linkage, global identity uniqueness, 64-entry/node and 4096-entry limits. This
standalone validation is independently checked against strict snapshot readers;
it adds no imports or runtime dependencies to direct script entrypoints.

## Callers and limits

The existing direct adapter, ranker and saved-window preparation already call
the shared comparison before engine execution or cached-result retrieval. They
now receive the projected allocation automatically. Plan-only collection retains
diagnostics via its existing non-throwing comparison mode. Existing source
fingerprints bind the changed validator, preventing mixed-version preparation
contexts; old journals are not rewritten or silently rebased.

No new flag, schema field, addon API, game action, template rewrite, network call
or addon package. Existing uninstalled 0.50.0-dev capture support is still needed
for allocation observations. Captures without it retain their explicit limits.
Native availability/atomicity, complete mechanics, legality, active identity and
full owned-BiS remain open.
See [offline evidence](allocation-simulation-checks-verification-2026-11-09.md).
