# Delivery and verification ledger

## Implemented

- Separate Retail, Classic and Forever boundaries with explicit variant/build/interface.
- Bounded JSON snapshot store with SHA256 integrity and atomic publication.
- Read-only MCP discovery and snapshot inspection; real stdio integration test.

## Required before completion

- Strict versioned snapshot schema and malformed-input coverage.
- Addon export adapters and data-only SavedVariables ingestion.
- Installed-client identification, capability inventory and per-client tests.
- Character, equipment, bags, talents, stats, professions, quests, currencies, achievements, macros, bindings and action-bar coverage where supported; explicit unavailable reasons elsewhere.
- Snapshot discovery, comparison and character analysis tools.
- Player-reviewed build/configuration proposals and export formats; no autonomous game writes.
- Policy evidence for every proposed action; unknown permission means disabled.
- Source distribution on cafasdon GitHub, isolated installation, reproducible dependency setup.
- Benchmarks with fixed fixtures, bounded outputs, measured improvements and regression tests.
- Live Retail export verification with the user's reload; independently qualify Classic and Forever when available. Never label fixture success as live compatibility.

No computer use. No game-memory access, input automation, injection or protected-action bypass.
