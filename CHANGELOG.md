# Changelog

## Unreleased

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
