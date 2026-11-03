# Changelog

## Unreleased

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
