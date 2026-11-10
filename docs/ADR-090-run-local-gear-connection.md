# One MCP connection per saved gear collection run

Date: 10 November 2026. Status: source/offline only.

Gear collection previously launched and initialized a new stdio reader for each
combination window. Reuse one initialized connection for the collection run,
opened lazily only when a window needs work. A completed journal starts none.
Standalone rank_forever_slot.collect retains its owned connection by default;
only the internal collector supplies a borrowed initialized session. No new
CLI/MCP argument selects an arbitrary session or external server.

Keep all per-window source/config/template/snapshot binding checks, candidate
page cache binding, identity checks, 45-second request cap, total monotonic
deadline and journal transactions. Each window keeps one deadline at the smaller
of 45 seconds and the remaining run budget; startup shares that deadline with
the first collection. Recompute remaining window time after initialization,
without a new 45-second allowance. Never renew the run deadline. Binding changes
reject before a window append.

Enter and close MCP SDK/AnyIO scopes in the collection task. The per-window
wait_for child task borrows the session without owning its cancellation scopes.
Close the run's AsyncExitStack in finally through aclose, rather than inject
controller errors into SDK task groups. The latter would turn existing plain
TimeoutError/ValueError failures into exception groups. A failed/expired startup
or later window closes the reader; committed prior windows remain resumable.
Do not retry a failed connection silently or create a process per window again.

No inventory, catalogue or result response is newly cached. Existing exact
snapshot reads and freshness/ownership/model limitations remain. This changes
connection overhead only, not legal search space or simulation quality.

Existing collection bindings fingerprint collector/ranker source. Their hashes
change with this implementation; old journals remain evidence and can reject
resumption under changed source. This feature does not migrate or relabel them.
Use a newly bound journal when necessary. Native qualification and live
installation remain deferred.
