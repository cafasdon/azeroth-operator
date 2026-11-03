# ADR 003: hosted MCP and local export bridge

Date: 2026-11-03. Status: accepted direction; implementation and deployment pending.

User authorised hosting on mcp.cafas.com.br and a separate repository if needed.
Keep one repository initially, with independently packaged Lua addon and Rust
companion/server. Their shared snapshot contract benefits from atomic changes.

Target the personal pylon CT290 gateway with a dedicated Azeroth scope. Proposed
route: /scopes/azeroth/mcp. It is not provisioned or verified. Inspect the live
configuration and owning source before allocating ports or modifying routing.
Gateway runbooks describe scope workers and principal policies, but are not
evidence of current runtime state.

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
