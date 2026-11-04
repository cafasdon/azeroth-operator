# Delivery and verification ledger

## Current operational checkpoint — 2026-11-04

- Public MCP: fourteen tools; importer and reader support macro, binding, equipment, bag and saved-talent operation data. Preparation is external; execution requires player review and Apply in the addon.
- Installed Retail addon: 0.12.0-dev, twenty-two files verified. Native equipment-set observations and player-applied name/icon/delete/use/spec-association operations are development-supported but unqualified. This includes full-stack moves, splits, saved-talent switches, observation-only completion handling and explicitly partial action-bar capture. Action-slot preparation and player-applied execution are deployed as development support; no live operation qualified. Installation is not proof the running client loaded this version. [Latest deployment evidence](docs/equipment-set-spec-deployment-2026-11-04.md).
- Live-qualified mutations: blank test macro creation only. Binding no-op verified. Item identity capture verified. Bag move receipt is outcome_unknown with the original item still observed at source; no replay. Equipment swaps, actual binding changes, other macro mutations, splits and talent switches remain unqualified.
- User deferred further in-game tests until tomorrow. Continue offline development; no additional game-test prompts today.
- Next implementation work: native equipment-set empty-slot semantics/create/save; talent creation/import/editing; further action-bar operations; inventory swap/merge/sort; remaining permitted operations in the policy matrix. Independently qualify family-specific adapters rather than enabling Retail paths on Classic/Forever.
- Create/save prerequisite: source-only temporary ignored-slot helper and failure fixtures implemented; no new packaged operation. [Contract and remaining work](docs/equipment-set-save-operations.md).
- Create/save executors now have source-only preimage, single-call and readback fixtures. Compiler/transport/review integration and nonignored empty-slot support remain pending; no additional operation is deployed.
- Source MCP preparation now produces create/save drafts with explicit ignored/empty preconditions and exact equipped identities. Strict import transport, review/receipts, packaging and native empty-slot qualification remain outstanding.

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
- Hosted read-only MCP and restricted authenticated upload on personal pylon CT290, verified with synthetic upload and public SDK readback. First live Retail snapshot uploaded and read back; ten sections available after the macro fix; action bars remain unavailable.
- Incremental hash acknowledgements and bounded local sync runner; missing exports, retries, concurrency and Windows timeout cleanup tested. Current-user scheduled task installed; first real scheduled run completed with result 0 and waiting statuses for Retail/Forever.

## Required before completion

- All eleven current section shapes have nested validation and malformed-input coverage. Live API compatibility and semantic constraints remain unverified. Bag indices 0..64 and slots 1..1024 are ingestion bounds, not claims about available game slots.
- Maintain hosted MCP and authenticated local snapshot transfer; complete live-export qualification. ADRs 003–005 record the boundaries. Scheduled missing-export and manual synthetic runner paths are proven; actual Retail game-export delivery verified on build70688/interface120105; broader qualification remains pending.
- Finish addon adapters, packaging and export coverage; data-only SavedVariables ingestion implemented. Retail action-bar slot enumeration remains unresolved; see docs/action-bar-qualification.md. The synthetic MAX_ACTION_BUTTONS fixture is not a live API contract.
- Explicit-root installed-client metadata identification and per-family fixture tests implemented; live capability inventory and independent variant/interface qualification remain pending.
- Character, equipment, bags, talents, stats, professions, quests, currencies, achievements, macros, bindings and action-bar coverage where supported; explicit unavailable reasons elsewhere.
- Deeper character/equipment analysis beyond observed counts; comparison and discovery implemented. Carried-item review deployed; all-observed-equipment no-change review and self-comparison verified using the first real Retail export. Optimisation remains unfinished.
- Player-reviewed build/configuration proposals and export formats; no autonomous game writes. Saved graphics/hardware baseline recorded in docs/graphics-baseline-2026-11-03.md; runtime display/FPS qualification remains pending.
- Operation scope/evidence matrix recorded in docs/operation-policy.md. Full current applicable terms and operation-specific authorisation remain unverified; unknown permission means disabled.
- Source distribution on cafasdon GitHub, isolated installation, reproducible dependency setup. Read-only installed-addon manifest verification implemented for Retail/Forever; Classic installation qualification remains pending.
- Benchmarks with fixed fixtures, bounded outputs, measured improvements and regression tests.
- Live Retail export verification with the user's reload; independently qualify Classic and Forever when available. Never label fixture success as live compatibility.

No computer use. No game-memory access, input automation, injection or protected-action bypass.

- Captured-loadout release deployed to reader, importer and local companion before installing Retail 0.1.1-dev. Public synthetic readback passed; real character and game-import acceptance remain unverified.

## Expanded operational goal (2026-11-03)

The active goal is maximum supported game operation within the established policy boundaries. Read-only coverage is a foundation, not the end state. ADR006 defines player-applied operations. Macro create/edit/delete/scope-move executor is source-tested; transport, review/apply, receipts, deployment and live qualification remain required. Gear, talents, inventory organisation, bindings/action bars, settings and other permitted families remain in scope; unavailable/unknown APIs must be reported explicitly.
