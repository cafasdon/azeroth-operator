# ADR 003: hosted MCP and local export bridge

Date: 2026-11-03. Status: deployed and synthetic roundtrip verified; live game export pending.

User authorised hosting on mcp.cafas.com.br and a separate repository if needed.
Keep one repository initially, with independently packaged Lua addon and Rust
companion/server. Their shared snapshot contract benefits from atomic changes.

The personal pylon CT290 gateway hosts the dedicated /scopes/azeroth/mcp route.
Public TLS discovery and fixture reads, missing/wrong-token refusal and network
isolation were verified; the dated verification log records the probes and image
identities. Inspect current live state and owning source before changing routing.

The server cannot read Windows WoW files directly. A local bridge must upload
explicitly exported snapshots through authenticated transport. Separate upload
permission from read-only MCP permission. Bound request sizes, validate before
publication, make duplicate uploads idempotent by content hash, and isolate data
by authenticated owner. Never trust a payload owner field. No remote arbitrary
file import or code execution tools.

Qualification must cover unauthenticated and wrong-scope denial, owner isolation,
duplicate uploads, invalid and oversized input, and actual MCP discovery and
snapshot reads over public TLS. Preserve local stdio for development. No game
input automation or live game-control channel is introduced.
