# Delivery and verification ledger

## Implemented

- Separate Retail, Classic and Forever boundaries with explicit variant/build/interface.
- Bounded JSON snapshot store with SHA256 integrity and atomic publication.
- Read-only MCP discovery and snapshot inspection; real stdio integration test.
- Rust CLI import of bounded JSON or strict hex-encoded SavedVariables; code-bearing files rejected.
- Family-filtered MCP snapshot listing with pagination and corrupt-file count.
- Initial Lua exporter for equipment, bags and basic stats; Lua 5.1 fixture roundtrip tested. Not installed/live-qualified.
- Expanded collectors and unavailable reasons tested with fixtures; local deterministic development ZIPs verified. Retail development addon installed with matching hashes; live qualification still pending.
- Snapshot comparisons with client/character/capture-order gates and bounded differences; basic observed character summaries.

## Required before completion

- Strict versioned snapshot schema and malformed-input coverage.
- Finish addon adapters, packaging and export coverage; data-only SavedVariables ingestion implemented.
- Installed-client identification, capability inventory and per-client tests.
- Character, equipment, bags, talents, stats, professions, quests, currencies, achievements, macros, bindings and action-bar coverage where supported; explicit unavailable reasons elsewhere.
- Deeper character/equipment analysis beyond observed counts; comparison and discovery implemented.
- Player-reviewed build/configuration proposals and export formats; no autonomous game writes.
- Policy evidence for every proposed action; unknown permission means disabled.
- Source distribution on cafasdon GitHub, isolated installation, reproducible dependency setup.
- Benchmarks with fixed fixtures, bounded outputs, measured improvements and regression tests.
- Live Retail export verification with the user's reload; independently qualify Classic and Forever when available. Never label fixture success as live compatibility.

No computer use. No game-memory access, input automation, injection or protected-action bypass.
