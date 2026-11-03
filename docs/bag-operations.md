# Player-applied bag operations

Development source, 2026-11-03. The executor moves a complete stack from one carried bag slot to an explicitly empty carried slot. Source GUID, item ID and count must match; both locations are checked before pickup and again before placement. Combat, casting, occupied cursor, locks, invalid carried locations and identical locations are refused. No automatic retry or cursor clearing occurs after a partial effect.

Verified Lua fixtures cover success, stale count/GUID, combat, occupied destination, replay against changed locations, failed placement and a destination becoming occupied between pickup and placement. Runtime asynchronous updates and specialised bag acceptance remain unqualified; a refused placement retains the cursor item and fails readback.

API reference checked 2026-11-03: https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/ContainerDocumentation.lua — PickupContainerItem takes bag and slot; SplitContainerItem additionally takes amount. This is API evidence, not blanket policy approval.

The Rust snapshot-bound compiler and AO-BAG-1 Lua decoder are implemented and roundtrip-tested. The format carries sixteen colon-separated fields: magic, operation ID, snapshot ID, family, variant hex, build hex, interface, character hex, realm hex, source bag, source slot, target bag, target slot, item ID, GUID hex and full-stack count. Destination emptiness is an explicit request precondition, rechecked at execution. Missing GUIDs and captured occupied destinations are refused.

Pending: shared player review/receipt and MCP integration, package/deployment and live tests. This module is not packaged or exposed yet. Swaps, merging, splitting, sorting and bank transfers remain subsequent work within the full operational goal.
