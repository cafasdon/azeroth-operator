# Carried bag swaps — source development contract

2026-11-04: `BagSwap.lua` implements source-only `swap_bag_items`, separate from existing move/split executors. No compiler/transport/review/receipt integration, packaging or deployment yet.

The operation binds exact Retail variant/build/interface/character/realm and two carried locations, each with item ID, count, GUID and link. Reject combat/casting/channel, occupied cursor, invalid locations, locked/changed items, same locations/GUIDs and identical item IDs. Same-ID swapping remains unfinished because it needs stack-merge compatibility evidence; it is not treated as a successful swap.

A successful fixture uses three pickup calls: source, target, source. After the first, verify source cursor identity, unchanged target and source either empty or retaining its exact original preimage. After the second, proceed only with the displaced target cursor, exact source identity at target and a genuinely empty source (both item info and GUID). If observations lag or placement is rejected, retain cursor and unknown; never place into an occupied/stale source. Final and delayed readback require both exact swapped identities/counts/links, unlocked states and empty cursor. No retry, ClearCursor, item use or protected-action bypass. Observers cannot finish writes. Cursor APIs expose item ID/link rather than a GUID; native identity/taint/timing and specialised bag compatibility remain qualification gaps.

Primary API reference read2026-11-04: [Blizzard generated ContainerDocumentation mirror](https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/ContainerDocumentation.lua), PickupContainerItem(containerIndex,slotIndex), GetContainerItemInfo and GetContainerNumSlots. This confirms API signatures, not swap semantics, addon permission or this implementation's native compatibility.

Offline fixtures exercise cross-bag success, exact swapped identities/counts, duplicate preimage refusal, combat, wrong GUID/count, locks, carried boundaries, same-ID refusal, race after pickup, rejected second/third placement, delayed read-only success, stale source observation and unavailable pickup API. Full source gate passed: format/build, Rust/Lua tests, Clippy and38Python tests. Existing bag move/split behaviour unchanged. No real item moved; previous Cutlass unknown operation was not retried.

Jev advisory request `78a90c03c2bbbadb8205899f5a88d7d3f634771263b12f5d6614308602326332`, resolved `jev-1.14.0`,1207input/154output tokens,0.289seconds: insufficient_evidence (probability0.87,confidence0.80), no concrete finding or pass. Configured adversarial-review file absent. Direct source inspection and deterministic fixtures support only the offline claims above.


## Preparation and transport — 2026-11-04

`prepare_bag_operation` now accepts optional `operation: "move" | "swap"`; omitted operation preserves existing move/split semantics and bytes. Swap refuses split amount or empty-target precondition and requires two captured item entries with GUID/count/link/ID. Both item IDs and GUIDs must differ. Client/character fields and item links reject control characters.

AO-BAGSWAP-1 has21colon-separated fields: common nine identity fields, then source and target groups of bag, slot, item ID, count, GUID hex, link hex. SHA256 ID binds all fields; it is not authentication. Decoder validates exact arity, canonical numeric ranges, nonempty bounded hex strings, Retail, distinct locations/identities and8192byte total. No Lua evaluation. Preparation reports execution disabled and no minimum addon version pending integration. No package/deployment change.

Full source gate passed including38Python tests and actual MCP preparation. Rust-to-Lua decoder tests cover delimiter-bearing links, both identities, malformed fields/arity, missing target/link, conflicting amount/empty flags and changed target count changing the operation ID. Existing move/split checks pass. The extra fixture item raised expected carried quantity from5to7; corrected the test expectation after the first full run caught it.

Jev advisory `837f480879c13a13c3229cdb6d28547268f83214b0f1e0c7b7454c8476b8f4f9`, model `jev-1.14.0`,1103input/154output tokens,0.321seconds: insufficient_evidence(probability0.71,confidence0.56), no concrete finding/pass. Configured adversarial-review file absent. Direct source/producer-consumer checks establish transport behaviour; compiler-to-executor shared lifecycle and native qualification remain pending.


## Review, Apply and receipts — 2026-11-04

Source shared dispatcher now routes AO-BAGSWAP-1 to the strict decoder and explicit ApplyBagSwap/VerifyBagSwap mappings. UI shows both complete item stacks with location, ID/count, GUID and link, plus uncertainty/no-retry instructions. Lua and Rust snapshot/checkpoint receipt allowlists accept swap_bag_items. Source preparation remains execution-disabled pending packaging; no hosted service or installed addon changed.

Actual Rust compiler → Lua decoder/review/Apply/executor → Core export → Rust SavedVariables import tests pass. They assert pre-write unknown checkpoint inside pickup, no-review/edited-byte refusal, immediate success, duplicate refusal, delayed read-only promotion, expiry preserving unknown, exceptions before/after effects and reload preserving unknown with replay refusal. Native APIs remain mocked. UI fixture verifies both target location and identity. Full source gate passed (format/build, Rust/Lua tests, Clippy and38Python tests).

Jev advisory `0f7b46b1a69cd17b1f4598a0f1fa9177f641555c483a8e31afcefa84dbecbb87`, resolved `jev-1.14.0`,1102input/154output tokens,0.353seconds: insufficient_evidence(probability0.78,confidence0.67), no concrete finding/pass. Configured frontend skill and adversarial-review file absent. Direct source and fixture checks establish integration only; packaging/deployment, same-ID stack semantics and native qualification remain incomplete.
