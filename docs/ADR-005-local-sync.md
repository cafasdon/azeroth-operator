# ADR 005: bounded local saved-export synchronisation

Date: 2026-11-03. Status: installed and first scheduled pass verified.

Use a current-user Windows scheduled task every five minutes and at logon. The
task invokes windowless Python for deployment orchestration; all snapshot parsing,
validation, canonicalisation, acknowledgement and transport stay in Rust. It runs
only during the user's interactive logon and never captures game state itself.

Configuration names explicit client account roots. Read only the addon's known
SavedVariables filename. Reject linked ancestors and cap directory entries and
exports. The file can be incomplete while WoW saves: validation fails and a later
pass retries. No game reload, input, memory or arbitrary Lua execution is involved.

The runner serialises passes with an OS file lock; Task Scheduler also ignores
overlaps. Rust gives SSH 60 seconds and reaps it; the outer 65-second timeout kills
the process tree if Rust fails to return. Eight exports and a 540-second pass
budget fit below the task's ten-minute execution limit. A synthetic Windows parent
and child process test verifies timeout cleanup.

Snapshot acknowledgements are durable and destination-config-bound. Skips explicitly
do not claim remote freshness. An explicit upload repairs a deleted remote object.
The receiver remains idempotent when a timed-out upload actually succeeded.

Pin installed executable and runner copies by hash, independent of build outputs.
Write only the latest status atomically, without account names or file paths.
Configuration/scan failures produce a non-zero task result; the previous status
timestamp must not be mistaken for a fresh successful check. No automatic game
export exists; a player must export and persist SavedVariables in the client.
