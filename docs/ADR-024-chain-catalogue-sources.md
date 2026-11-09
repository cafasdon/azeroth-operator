# ADR 024: cached public sources for recipe-chain external materials

Date: 9 November 2026. Status: source/offline implementation.

Recipe-chain drafts already calculate shared-stock net external shortages, but
callers otherwise need a separate local catalogue lookup for each leaf. Add
optional `reagent_chain_catalogue_sources` to existing exact standalone chain
review. It requires chain mode and a configured checksum-valid catalogue.
Omitted/false leaves output unchanged and does not load the cache. A failed
configured pin or missing catalogue rejects the opt-in rather than masquerading
as a successful lookup with no matches. A valid cache is required even for a
blocked chain, whose annotations are null.

Reuse the existing per-request verified catalogue loader and shared item index.
Read only the chain's complete external leaf list; validate at most 512 unique
positive u32 IDs and positive u64 quantities. Eight steps with at most 64 fixed
reagent kinds bound the leaf universe. Preserve quantities as already net of
shared stock; do not subtract stock or expand production again. Return items in
numeric order with cached names, first eight public sources, original source
counts and explicit truncation. Reuse public NPC/zone/faction reference matching
only on retained sources. Record embedded dataset provenance.

Distinguish absent items, missing source metadata and observed empty source
arrays. Aggregate match/source counters measure only this cache; even all
matched does not prove complete world sources. The existing simulator-derived
cache notably omits common reagents. No new dataset, licence grant, network
refresh or coverage claim is introduced. Further world/reference datasets need
their own version, provenance and licence evaluation.

Keep these public references separate from observed player stock, merchant
quotes and availability. Do not pass private character snapshots to the helper.
Current availability, acquisition eligibility, ownership and complete source
coverage stay unverified; no route optimisation, purchase, craft or other game
execution occurs. Full progression, world acquisition and native qualification
remain unfinished.
