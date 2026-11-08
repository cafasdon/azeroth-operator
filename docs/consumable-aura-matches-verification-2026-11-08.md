# Consumable reference and aura matches — 8 November 2026

`lookup_forever_catalogue` for `section: consumables` with an
`acquisition_snapshot_id` now adds
`acquisition_progress.captured_helpful_aura_matches`. The same validated owner-local
capture supplies both inventory counts and saved player aura observations. Without
an acquisition capture, no snapshot-derived result is added.

The join compares positive pinned catalogue `itemEffects[].buffId` and nested
`stackingAura.buffId` values with saved helpful `player_auras` spell IDs. Harmful
auras, names, application counts and raw item-use spell descriptions do not join.
Reference IDs are deduplicated. Each matched aura row counts once per item;
duplicate aura spell IDs retain a separate row count but one distinct-ID count.

The result reports `helpful_list_observed`, false `item_use_verified` and
`effectiveness_verified`, plus `by_item_id` for returned records. Each item has
catalogue effect-ID count, first 50 numeric IDs and truncation, and nullable
`observed_id_match`, matching aura row count, distinct spell-ID count, first
50 matching IDs and truncation. Computation uses the full sets before truncating.

Missing helpful observation or no recorded reference IDs makes the match unknown,
including absent/empty/malformed effect metadata in a synthetic direct-library
record. A known empty helpful list with reference IDs gives false exact-ID match
and zero matching rows. That means no matching ID was reported, not that the
consumable is inactive, expired, usable or needed. Some references identify
instant-use spells, procs or shared effects rather than persistent unique auras.
A positive match cannot establish which item or caster produced it, whether the
item is owned, current activity, uptime, strength or stacking eligibility.

Existing ownership quantities, withdrawal suggestions, restock shortages, crafting
inputs, budgets, cooldown groups and simulator templates are unchanged. Observed
effects do not add stock or cancel a restock shortage. No new native query,
gameplay action, external lookup, installation or deployment is introduced.

## Offline evidence

- Current `cargo build --locked --offline` passed.
- `cargo test --locked --offline --lib consumable_aura_matches` covers missing
  and malformed reference associations, direct/nested and duplicate reference
  IDs, duplicate aura rows, full counts and 50-ID bounds, and false verification
  flags using a validated synthetic snapshot.
- `PYTHONPATH=.;tests python -m unittest test_consumable_aura_matches test_player_auras`
  exercises the real stdio MCP with the existing pinned catalogue: missing and
  harmful-only observations, empty/nonmatching/matching helpful lists, shared
  effect IDs across two items, duplicate rows, zero/unknown applications,
  positive match without bag ownership and identical stock/restock calculations
  across all aura cases. No catalogue download is performed.

Independent Callers review (`gpt-6-astra`, high effort) found no actionable
finding, tracing same-snapshot store reads, filtered-record processing and all
stock/restock/crafting/simulator consumers. It ran the new MCP test against the
final rebuilt binary and checked that the cooldown helper remains unchanged.

Independent Failure/security review (`gpt-6-astra`, high effort) also found no
confirmed defect. It executed the MCP test and prebuilt Rust unit, and rejected
foreign-store IDs, traversal and four directly stored malformed aura payloads
without echoing private markers. It verified owner-store validation and catalogue
checksum remain on the caller path. Both lenses finished clean. Author's final
Rust unit passed and the two Python tests passed in 1.406 seconds.

Jev plan advisory: `jev-1.14.0`, request
`b99955780590e664672590ba2190e4c6fd952d2eb8a231cdcdf0f446a9792960`,
537 input / 47 output tokens, 0.361 seconds; proceed probability 0.93. Advisory
confidence is not evidence of item use or native effect semantics.

Post-verification advisory: `jev-1.14.0`, request
`9587bce4bc5851e483d73a2755f1751c3dcc86f2af541f2390648448c1262f35`,
609 input / 50 output tokens, 0.343 seconds; land-scoped probability 0.98.
Scoped tests and independent reviews provide the actual evidence.
