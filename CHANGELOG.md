# Changelog

## Unreleased

- Package Retail 0.11.0-dev native equipment-set use with twenty files, versioned preparation support, 19→20-file upgrade checks and staged Lua capture/compile/review/Apply/receipt verification. Candidate verified offline; not installed or live-qualified.

- Connect source equipment-set use to player review/Apply, exact per-slot review and persisted receipt validation. Offline end-to-end tests cover pre-write uncertainty, delayed completion, expiry and replay refusal across reload. Not packaged or deployed.

- Add source MCP equipment-set use preparation and strict 212-field decoder with all nineteen equipped preimages and explicit empty-slot preconditions. Execution remains disabled pending review/receipt/package integration; no deployment or game test.

- Add a source-only native equipment-set use executor with exact equipped GUID preimages/readback, ignored-slot preservation, unresolved-slot refusal and one native call. Offline failure fixtures pass; no transport, receipt registration, packaging or live qualification yet.

- Deploy Retail 0.10.0-dev equipment-set development support with compatible hosted importer/reader and Windows companion. Public fourteen-tool discovery/preparation checks and nineteen installed hashes pass. No live set mutation or in-game test occurred.

- Build nineteen-file Retail 0.10.0-dev equipment-set package with legacy upgrade support and actual staged Lua review/Apply/receipt verification. Require 0.10.0-dev for set operations; candidate verified offline, not installed.

- Expose source MCP equipment-set metadata preparation as the fourteenth read-only tool. Real protocol tests verify preparation/refusals and unchanged snapshot files; gateway allowlists and importer qualification fixtures are aligned. Not deployed.

- Connect equipment-set metadata operations to source player review/Apply, persisted receipt validation and bounded read-only completion checks. Prevent missing executors falling through and invalid results erasing uncertainty checkpoints. Not packaged or deployed.

- Add source-only equipment-set metadata compiler and strict 193-field Lua transport, preserving the complete captured set preimage and observed identities. Compiler-to-decoder-to-export roundtrip and executor fixtures pass; MCP/review/receipt integration remains pending.

- Add source-only exact-preimage equipment-set name/icon modification and deletion executors with native readback and failure fixtures. Not connected to MCP, review, receipts or packages.

- Refuse ambiguous action spell cursors when the native spell-ID return is absent; retain the cursor and stop before placement. Source regression verified; installed 0.9.0-dev is unchanged.

- Resolve optional equipment-set item observations to worn/carried locations, item GUIDs and lock flags, with repeated identity checks and strict validation. Unsupported or uncertain identities remain absent. Source only; native set swaps are not yet implemented.

- Add source-only Retail native equipment-set observations with raw slot values, missing-value preservation, bounded catalogue checks and Lua-to-Rust failure fixtures. No equipment-set mutations or deployment.

- Deploy Retail0.9.0-dev player-applied action-slot operations after importer/reader qualification and companion upgrade. Public13-tool endpoint and17installed files verified; no live action attempted.

- Prepare seventeen-file Retail0.9.0-dev action-operation package with legacy upgrade support, delayed-receipt/expiry tests and pre-write API availability checks. Candidate verified offline; not installed.

- Connect action-slot source decoder, player review, executor and receipts. End-to-end compiler-to-Lua-to-export/import tests pass, with replay refusal and macro review details. Installed addon remains unchanged.

- Deploy Retail0.8.0-dev partial action observations with matching hosted importer/reader and Windows companion. Public endpoint verifies13tools; action-bar execution remains disabled.

- Expose source MCP action-slot preparation with explicit execution-disabled output and real stdio tests. Align reference-store section recognition and importer qualification fixture. Thirteen-tool release not deployed.

- Add source-only action-slot compiler and strict Lua decoder, binding explicit slot and macro preimages with bounded transport. Roundtrip/refusal tests pass; MCP/UI integration remains pending.

- Add source-only action-slot move/swap executor and failure fixtures, including macro-content preimages. Not packaged, registered or MCP-enabled; cursor contracts and live protection behaviour remain unqualified.

- Add source-only, explicitly partial Retail action-button observations with occupied/empty slots and coverage counts. Actual Lua-to-Rust roundtrip and malformed-coverage tests pass; no action mutations or deployment.

- Install Retail 0.7.1-dev bag observation fixes after packaged Lua verification; fifteen installed files match the manifest. Refresh operational capability status; live qualification remains deferred.

- Handle retained source observations during bag pickup and delayed final move/split readback. Export-time reconciliation is bounded and read-only; unknown operations are never replayed. New failure/timing fixtures pass; live behaviour remains unqualified.

- Deploy saved-talent preparation and stack-split development support: hosted importer/reader, Windows companion and Retail addon 0.7.0-dev. Public discovery verifies 12 tools; installed addon matches all 15 files. In-game qualification is deferred by the player. Bag test persists outcome_unknown with the exact item still observed at its source; no replay.

- Add source-only exact-instance Retail equipment swap executor with GUID, cursor and slot checks, displaced-item readback and failure tests. Two-handed/multi-slot handling remains incomplete; no live gear changes.

- Add optional Retail item GUID capture to equipment/bag observations, preserving unknown identity when unavailable. Lua producer and Rust validation tested; undeployed groundwork for exact-instance equipment operations.
- Verify live binding no-op through player review, persisted already_selected receipt and public MCP readback; actual binding writes remain unqualified.

- Permit assigning uncaptured keys with an explicit unbound precondition, checked live before mutation and labelled separately from observed state. Compiler, Lua executor and protocol coverage added; no live key changes.

- Connect binding preparation to the local MCP, player review/apply dispatcher and persisted receipts; package binding modules and add `/aoapply`. Protocol and compiler-to-Lua execution tests pass. Not yet deployed or live-qualified.

- Add source-only binding compiler and strict Lua data decoder, with Rust-to-Lua tests for context preservation and uncaptured-key refusal. No binding transport is enabled in the deployed addon yet.

- Capture Retail binding contexts and active binding-set metadata for player-applied binding plans. Additive schema and Lua producer roundtrip verified; not yet deployed.

- Begin context-aware Retail keybinding execution: source-only single-key set/unbind with preimage, active-set, combat and registered-action checks. Lua fixture passed; MCP transport and deployment remain pending.
- Accept WoW's initial `AzerothOperatorOperations = nil` checkpoint without evaluating Lua. Regression checks reject invalid nil suffixes and duplicate assignments; updated companion imported the real SavedVariables file and uploaded successfully.
- Live macro creation qualified: player Apply, saved receipt and exact character macro contents verified through the public MCP. Other macro mutations remain pending live checks.

- Replace macro import transport pipes with colons to avoid WoW text-format escape handling observed during live paste. Decoder accepts both; canonical operation IDs remain unchanged. Full source gate passed. Retail 0.2.2-dev installed and the existing local test code re-encoded; hosted compiler image update still pending.

- Fix the macro paste field after the first live UI report: give the empty edit box an explicit height, mouse interaction, initial focus, dark background and paste instructions. Regression fixture checks its clickable dimensions and focus. Retail 0.2.1-dev installed with seven matching files; player confirmation pending.

- Export bounded macro operation receipts as `operation_receipts`, validated by Rust and readable with `inspect_snapshot`. Lua-to-snapshot and actual MCP protocol tests preserve unknown outcomes. Hosted reader/importer upgrades remain required before addon installation.

- Persist macro attempt receipts as bounded data strings and accept both SavedVariables assignments in the Rust companion without executing Lua. Reject duplicate IDs, unsupported statuses, extra assignments and executable input; retain legacy single-export support. Source verification passes; receipt inspection through hosted snapshots remains to implement.

- Package the three macro modules and add a hash-checked four-to-seven-file addon upgrade with rollback of newly added files. Development package verified in Lua5.1. Installation remains pending importer support for the additional receipt SavedVariable.

- Add local MCP `prepare_macro_operation`, strict data import and `/aomacros` review/apply UI source. Protocol tests cover all four macro operation types; Lua fixtures cover target checks, replay refusal and review invalidation. Not yet packaged, deployed or live-qualified.

- Begin executable macro management: source-only Lua executor for create/edit/delete/scope moves with exact-content checks and partial-move readback; not yet packaged or exposed by MCP.

- Verify first Retail export through the hosted MCP; support namespaced Constants.MacroConsts.MAX_ACCOUNT_MACROS after a live macro-collector failure. Action-bar qualification remains open.

- Add bounded carried-item review to character summaries: observed quantities, distinct/repeated IDs and exact exported locations, with truncation and unknown stack compatibility. Source verified; hosted readback qualified with synthetic inventory.

- Isolate per-family export discovery errors, report retryable failure in fresh sync status and continue processing other families.

- Extend independent container protocol smoke to verify capability claims, missing-snapshot rejection across all read/review tools and rejection of unknown mutation tools.

- Add one bounded source-verification command binding protocol tests to the freshly built Rust target, with fail-fast checks and timeout cleanup.

- Make the seven-tool Rust MCP protocol test mandatory in the default Python suite; retain the two-tool Python reference test separately. Verify disabled execution, annotations, capability inventory and unknown mutation-tool rejection.

- Replace per-byte hex formatting with a fixed256-entry lookup. Four paired Lua fixtures measured58.4% lower capture median versus0.1.3-dev, with identical output; no in-game FPS claim.

- Record Retail/Classic/Forever operation scope with official policy sources, explicit evidence gaps and disabled execution boundaries. No newly enabled actions.

- Reuse validated section JSON within each synchronous capture, avoiding duplicate serialisation. Four paired synthetic Lua runs measured 23.5% lower median capture time with identical export bytes; no in-game performance claim.

- Isolate equipment, bags and stats collector failures so readable sections still export; failed sections are unavailable rather than empty. Preserve fatal combat/interface and identity guards.

- Add bounded read-only installed addon verification against explicit manifests, reporting drift without overwriting local changes.

- Add bounded read-only installed-client metadata inspection with product/family checks, ambiguous-build reporting and explicit unknown live compatibility. No interface or independently verified variant is inferred.

- Capture original Retail talent import strings for manual review when APIs permit and no staged changes exist. Preserve observed talent data on optional-export failure; add strict schema checks and read-only captured_talent_loadout MCP tool. Live/game-format acceptance remains unverified.

- Add checksum-pinned BigWigs packaging in isolated Linux/WSL staging, strict ZIP/source verification and actual packaged Lua5.1 fixture execution. Keep all packages development-only pending live-client/manager qualification.

- Reduce repeated item allocations and store-root resolution without caching snapshot validity. Four alternating Windows benchmark pairs measured 24.4% lower listing median and 12.7% lower inspection median on 200 synthetic snapshots; preserve hash/schema/containment and pagination checks.

- Add read-only equipment-selection review drafts bound to exact snapshot locations and expected item IDs. Reject duplicate targets/sources and unavailable observations; expose swaps/vacancies without claiming gameplay legality or performance gains.

- Add a bounded saved-export sync runner, Windows process-tree timeout cleanup, atomic status and an isolated current-user scheduled-task installer. Synthetic upload/readback succeeded; unchanged pass measured 0.21s versus 5.30s for upload.

- Add incremental sync with durable hash receipts, destination configuration binding, retry after failure and explicit missing-export status. Cached acknowledgements do not claim live remote state.

- Add bounded `import-stdin` for snapshot transfer, with subprocess tests for retry deduplication and empty, malformed, code-bearing and oversized rejection.

- Add digest-pinned Linux container build with Rust/Lua tests and non-root runtime, plus an independent stdio protocol smoke tool.

- Validate all eleven current export sections, including nested talents/currencies, required fields and partial-coverage markers. Exercise malformed variants of actual Lua fixture output through the importer.

- Validate equipment and carried-bag slot maps, positive item IDs/stack counts, bounded links and unknown item fields before storage and reads. Legacy array inventory fixtures are no longer accepted by Rust.

- Add bounded snapshot comparisons and character summaries, preserving unknown/missing data.
- Add repeatable synthetic release-mode storage benchmark.

- Expand read-only export collectors with explicit unavailable/partial coverage.
- Add deterministic development ZIPs and manifests; install initial Retail test package with hash verification.

- Add strict data-only SavedVariables/JSON import CLI and family-filtered MCP snapshot discovery.
- Add read-only Lua equipment/bag/basic-stat exporter and Lua 5.1 roundtrip harness.

- Port snapshot foundation and MCP tools to Rust using the official rmcp SDK.
- Verify Rust protocol interoperability with the independent Python SDK client.
- Add typed envelope validation and concurrent publication regression test.

- Add bounded content-addressed JSON snapshots, explicit age/source and atomic publication.
- Add read-only stdio MCP and real client integration test; eight tests pass.

- Establish Retail, Classic and Forever subdivisions with shared MCP and contracts.
- Require explicit client family, variant, build and interface; reject cross-client imports.
- Record conservative policy boundary; runtime features remain unqualified.

- Add standalone gateway-profile generation with immutable image pin, explicit read-tool allowlist and read-only owner store; validate format using CT290 Docker MCP importer.

- Qualify dedicated Docker MCP profile with the exact read-only bind allowance; add loopback service and dedicated token validation candidates.

- Verify hosted populated-snapshot readback using the independent SDK, and add exact-hash synthetic fixture cleanup tooling.

- Add Rust upload command with bounded fixed SSH transport and exact hash receipts, plus a restricted receiver candidate. Provisioning and live upload qualification remain pending.

- Qualify dedicated CT290 upload key, command/remote-forwarding refusal, hash-matched duplicate uploads and public MCP readback. Private key and token stay out of source.

## Equipment integration — development, 2026-11-03

- Added snapshot-bound equipment preparation to MCP and shared review/receipt dispatch.
- Packaged both equipment modules; maintained legacy installation manifest support.
- Verified eleven-file package and shared dispatcher replay refusal. Not deployed or live-qualified.

Equipment development deployment: hosted importer/reader and local companion upgraded; Retail 0.4.0-dev installed with eleven-file integrity verification. Public MCP exposes ten tools. Live GUID export and gear swap qualification remain pending. See docs/equipment-deployment-2026-11-03.md.

## Bag movement executor — development

- Added guarded full-stack carried-slot movement and partial-effect fixtures. Transport, UI and deployment remain pending.
- Updated the operation matrix to distinguish live macro creation, binding no-op and deployed equipment development support.

- Added snapshot-bound bag-move compiler and AO-BAG-1 decoder; Rust-to-Lua tests preserve GUID/count and reject missing identity, occupied destination and implicit emptiness.

- Connected bag movement to MCP preparation, shared review/receipts and the thirteen-file 0.5.0-dev package. Source/protocol/package checks pass; deployment and live qualification remain pending.

- Bag development release deployed to hosted MCP/importer, Windows companion and Retail addon 0.5.0-dev. Public discovery and thirteen-file integrity passed; player move test pending.

- Added source-only stack-split executor checks and quantity-conservation/failure fixtures. Split transport and deployment are pending; the installed bag-move test build is unchanged.

- Split preparation, review/receipt integration and 0.6.0-dev package verified in fixtures. Review displays both split amount and source remainder. Not deployed.

- Apply diagnostics now show a bounded plain-text refusal/error reason while retaining outcome_unknown for executor errors. Tests verify the reason survives dispatch and WoW markup/control characters cannot alter its display. This source change is not installed; the previously recorded 0.6.0-dev ZIP predates it and must be rebuilt before release.

- Investigated saved talent-loadout switching: active working config differs from saved selection, and LoadInProgress requires completion observation. Recorded catalogue capture and asynchronous receipt requirements in docs/talent-operations.md before implementing talent writes.

- Added source-only saved talent-loadout catalogue capture and nested validation, with real Lua producer roundtrip fixtures. Not deployed.

- Added snapshot-bound saved-talent operation preparation in source, including current/desired build contents and action-bar setting. Addon transport/execution and deployment remain pending.

- Added source-only talent executor and bounded observation-only receipt reconciliation. Completion requires committed contents and matching saved selection; expiry remains uncertain. Targeted failure/dispatch tests pass; not deployed.

- Talent application now delegates saved-loadout selection/commit bookkeeping to the visible native Talents panel, with config/inspection/busy checks and no direct-API fallback. Source tests pass; live UI qualification pending.

- Connected talent preparation to MCP and included the decoder/executor in the fifteen-file 0.7.0-dev package. Protocol and package checks pass; not deployed or live-qualified.
