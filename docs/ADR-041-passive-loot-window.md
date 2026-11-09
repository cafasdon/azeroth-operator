# ADR 041: passive visible loot-window observations

Date: 9 November 2026. Status: source/offline; no installation.

Existing snapshots cover inventory and many service panels but have no loot-window
observations. Add a Forever-only manual-export section for an already shown native
LootFrame. This is inspection of displayed rows, never a loot or roll action.

Use only native read getters: slot count, slot hyperlink and the third slot-info
return (quantity). Bound the count at 64. Require shown context and stable count
before/after two full reads. Parse a bounded public item hyperlink into a positive
u32 ID; never retain its text, name, source GUID or other tuple fields. Item ID and
u32 quantity must agree between reads. Unreadable, restricted, unstable or non-item
rows become status-only `unreadable_or_non_item`; other stable rows survive.
Unavailable APIs, hidden/closed context or invalid/changing count omit the section.

Strict readers require exact coverage/count/entries fields and contiguous canonical
slot keys. Item rows contain only status/ID/quantity; unknown rows only status.
A shown empty window is observed-empty, distinct from missing data. Summary totals
are bounded by 64 u32 quantities, retain unknown-slot counts and explicitly deny
ownership/lootability verification. Existing inspection exposes saved rows.

Never add this section to inventory/storage enumeration, reagent stock or owned
gear. Different loot sources may expose identical rows; no source identity or
continuity is inferred. Money/currency, allocation/roll eligibility and current
availability remain unknown. No events, panel mutation, gameplay commands or new
MCP action route. Native API/secret behaviour and installation remain deferred.

Review also exposed a generic importer mismatch: Python had not validated optional
top-level `unavailable` metadata. Match existing Rust rules now: map of known
section names absent from captured sections, nonblank string reasons of at most
256 UTF-8 bytes. Preserve Rust Unicode whitespace semantics, including its
difference from Python for U+001C..U+001F. No Rust acceptance rule is broadened.
