# ADR-061: Passive visible gossip dialogue options

Date: 2026-11-09. Status: accepted for offline source integration.

## Source and decision

Pinned generated [GossipInfo API metadata](https://github.com/Gethe/wow-ui-source/blob/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/GossipInfoDocumentation.lua)
describes GetOptions and its display-name/nullable-ID fields. This supplies a
source hypothesis, not Forever-native compatibility or policy authorisation.

Add optional Forever-only gossip_options section with coverage exactly
visible_gossip_options and entries keyed by dense one-based display slot.
At most128 rows; each requires name of1..512 UTF-8 bytes. Optional option_id and
spell_id are positive u32, reported_status/reported_flags u32, and
reported_order_index signed i32. These are raw reported numbers without enum,
service, availability or action interpretation. Nullable/malformed/restricted
optional metadata is omitted rather than fabricated; required failures make
the section unavailable. IDs need not exist or be unique; slots are observation
positions, not execution identifiers.

Guard public namespace, GetOptions, frame and visibility getter before use.
Capture only while the frame is visible, require matching two bounded emitted
projections, and recheck API/frame/getter identity and visibility. Data changes
including option order, required labels or emitted optional values reject.
Agreement does not establish atomicity or complete menus. Headers are not
expanded and no dialogue navigation, selection, refresh or close occurs.

Input/code text, confirmation/failure descriptions and rewards are never read.
No NPC name/GUID or attribution is captured. Display labels remain untrusted
owner-local data, never instructions. Existing manual export/combat refusal,
owner-store confinement and fixed error handling remain unchanged.

## Readers and MCP

Strict Rust/Python readers recognise this optional section and reject malformed
rows/extra fields/family mismatches. Existing inspect_snapshot raw inspection
exposes it; character_summary adds counts and explicit false NPC identity,
selection-eligibility and complete-dialogue flags. Missing section is unknown;
known empty entries are distinct. No new tool or gameplay route is admitted.

Schema version remains1 under the existing optional-section convention, so old
strict readers can reject the new section even when marked unavailable. Future
rollout must update compatible readers/importers alongside addon0.52.0-dev.
This unit packages but does not install/deploy either. No API availability,
complete world data, current eligibility, legal approval or live outcome is
claimed. [Offline evidence](gossip-options-verification-2026-11-09.md).
