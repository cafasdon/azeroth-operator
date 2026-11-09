# ADR 026: historical captured sources for recipe-chain external needs

Date: 9 November 2026. Status: source/offline implementation.

ADR 025 added owner-local provider reference capture, but chain callers still
needed to inspect the raw section and manually match material IDs. Add optional
`reagent_chain_captured_sources` to existing exact standalone chain review, with
optional `reagent_source_snapshot_id` requiring that flag. Default to the recipe
capture, never the inventory or newest saved capture. No public catalogue,
running provider, network access or game API is needed to review saved data.

The explicit reference snapshot must match exact character name/realm and
client family/variant/build/interface. Check identity before returning null for
blocked drafts. Older references are allowed and retain their own timestamp;
different observed professions are allowed because references join exact item
IDs, not recipe knowledge. Existing inventory freshness/comparability rules do
not change. Do not merge these references into catalogue annotations.

For complete drafts, validate at most 512 unique positive u32 external IDs with
positive u64 quantities. Use already-net shortages without subtracting stock or
expanding production again. Return numeric item order. Project only the five
validated numeric reference categories, count/truncation and fixed metadata,
not private identity, links, names or unrelated saved sections. Preserve selected
reference snapshot ID/time and original capture coverage separately from chain
recipe/inventory provenance.

Distinguish unavailable capture, item not captured, provider miss, unavailable
read and reported row. Reported rows may have zero categories; missing categories
remain unknown and retained truncated categories stay explicit. Count captured
rows, provider-reported rows and rows with any retained source IDs separately.
Capture counters describe its bounded original reagent subset, not full provider
or world coverage. An absent item can be unqueried, not unobtainable. Complete
empty demand yields empty records; blocked drafts yield null.

Captured references are independent of stock, producer choices, merchant quotes
and public catalogue references. They do not prove dataset revision, current
availability, ownership, acquisition eligibility, optimality or native policy
qualification. Source/entity/coordinate resolution, full acquisition routing,
complete world data and native acceptance remain unfinished. Live testing,
installation and deployment remain deferred.
