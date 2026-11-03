# Delivery and verification ledger

## Implemented

- Separate Retail, Classic and Forever boundaries with explicit variant/build/interface.
- Bounded JSON snapshot store with SHA256 integrity and atomic publication.
- Listing optimisation deployed with unchanged per-file validation. Four paired synthetic Windows benchmark runs measured 24.4% lower listing median and 12.7% lower inspection median; hosted Linux speedup is not measured.
- Read-only MCP discovery and snapshot inspection; real stdio integration test.
- Rust CLI import of bounded JSON or strict hex-encoded SavedVariables; code-bearing files rejected.
- Family-filtered MCP snapshot listing with pagination and corrupt-file count.
- Initial Lua exporter for equipment, bags and basic stats; Lua 5.1 fixture roundtrip tested. Retail development package installed; not live-qualified.
- Expanded collectors and unavailable reasons tested with fixtures; local deterministic development ZIPs verified. Retail development addon installed with matching hashes; live qualification still pending.
- Pinned BigWigs packaging runs in isolated staging without network/upload access; packaged Retail/Forever/Classic-fixture Lua roundtrips verified and Retail rebuild byte identity confirmed. Manager and live-client qualification remain pending.
- Snapshot comparisons with client/character/capture-order gates and bounded differences; basic observed character summaries.
- Read-only equipment-selection JSON reviews with exact observed-location/item matching, duplicate rejection and swap/vacancy accounting. Deployed seven-tool public MCP and verified synthetic changed/no-change reviews and wrong-item refusal. These are unverified selection drafts, not optimal builds or game imports; see docs/equipment-review.md.
- Hosted read-only MCP and restricted authenticated upload on personal pylon CT290, verified with synthetic upload and public SDK readback. No live game snapshot yet.
- Incremental hash acknowledgements and bounded local sync runner; missing exports, retries, concurrency and Windows timeout cleanup tested. Current-user scheduled task installed; first real scheduled run completed with result 0 and waiting statuses for Retail/Forever.

## Required before completion

- All eleven current section shapes have nested validation and malformed-input coverage. Live API compatibility and semantic constraints remain unverified. Bag indices 0..64 and slots 1..1024 are ingestion bounds, not claims about available game slots.
- Maintain hosted MCP and authenticated local snapshot transfer; complete live-export qualification. ADRs 003–005 record the boundaries. Scheduled missing-export and manual synthetic runner paths are proven; actual new game-export delivery remains pending.
- Finish addon adapters, packaging and export coverage; data-only SavedVariables ingestion implemented.
- Explicit-root installed-client metadata identification and per-family fixture tests implemented; live capability inventory and independent variant/interface qualification remain pending.
- Character, equipment, bags, talents, stats, professions, quests, currencies, achievements, macros, bindings and action-bar coverage where supported; explicit unavailable reasons elsewhere.
- Deeper character/equipment analysis beyond observed counts; comparison and discovery implemented.
- Player-reviewed build/configuration proposals and export formats; no autonomous game writes.
- Policy evidence for every proposed action; unknown permission means disabled.
- Source distribution on cafasdon GitHub, isolated installation, reproducible dependency setup. Read-only installed-addon manifest verification implemented for Retail/Forever; Classic installation qualification remains pending.
- Benchmarks with fixed fixtures, bounded outputs, measured improvements and regression tests.
- Live Retail export verification with the user's reload; independently qualify Classic and Forever when available. Never label fixture success as live compatibility.

No computer use. No game-memory access, input automation, injection or protected-action bypass.

- Captured-loadout release deployed to reader, importer and local companion before installing Retail 0.1.1-dev. Public synthetic readback passed; real character and game-import acceptance remain unverified.
