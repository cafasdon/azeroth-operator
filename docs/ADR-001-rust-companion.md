# ADR 001: Rust companion, Lua addon

Status: accepted, 2026-11-03.

Use Rust for the local MCP server, snapshot validation/storage and planning. Use Lua only inside the normal WoW addon sandbox, with Retail, Classic and Forever adapters. Python remains temporarily as the reference prototype and independent MCP test client.

Reasons: a standalone Windows executable, explicit typed boundaries and bounded data handling. No performance claim follows from the language choice; controlled benchmarks remain required.

The Rust implementation uses the official rmcp SDK over stdio. It contains no game execution endpoint or arbitrary Lua evaluator. Versioned data exports remain the integration boundary. No desktop automation.

Migration evidence: three Rust tests (including concurrent atomic publication) and eight Python tests pass with the MCP test targeting the Rust executable. The Python fixture store supplies the snapshot for that integration test, establishing cross-language reading compatibility. Canonical serialisation differs, so new Rust-ingested snapshots may have different hashes for semantically equal Python exports; existing stored file hashes remain valid.

Remaining: strict nested section contracts, addon/CLI ingestion, discovery/analysis/proposals, game-client qualification, repeatable benchmarks and retirement of the Python runtime after feature parity is checked.
