# Changelog

## Unreleased

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
