# ADR-074: Reuse owned-gear pages within one checked collection run

Date: 2026-11-09. Status: offline source integration.

Each saved gear window previously re-read the same complete owned-item pages.
With larger candidate pools, repeated enumeration adds avoidable MCP work even
when the actual proposal window is small. Reuse only raw owned-gear pages within
one collect_windows invocation; keep every window's planning and validation.

OwnedGearPageCache stores JSON bytes, not shared mutable dictionaries. Bounds are
32 MiB of serialized keys/payloads and 1,024 entries. Oversized or uncacheable
pages are served fresh without truncation. Cache hits decode independent copies.
Only numeric hit/miss/entry/byte statistics reach the normal collection report.
No files, global cache, cross-user store or persistent page cache are added.

The controller creates the cache from its checked request binding and verifies
that binding before and after each window. This includes source, binary,
catalogue and configuration fingerprints. Ranker caching is an internal optional
Python argument, restricted to explicit snapshot combination windows. The trusted
controller owns fresh binding computation; it is not a public MCP or CLI cache
input. Keys include binding, backend path, snapshot and full request arguments.

Every ranker invocation still reads stats and validates the baseline through the
real MCP before any cached page lookup. Snapshot content hashes therefore remain
checked. Cached pages still pass ordinary catalogue/inventory evidence,
pagination, modifier, physical-copy and work-budget checks. Source changes fail
before a new journal append. A new invocation starts with an empty cache.

This reduces repeated owned-page requests; it does not imply a proportional
wall-clock or simulator speedup. No iteration count, model precision, candidate
scope, journal coverage, ownership or best-in-slot qualification is changed.
No live test, addon installation, deployment or gameplay is included.
