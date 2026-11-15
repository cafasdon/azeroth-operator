# ADR 110: Gear combinations over declared physical variants

Date: 15 November 2026. Status: accepted for offline reference constraints.

The current gear reviewer detects observed conflicts but can retain unknown
requirements. A generator needs a stricter accepted/unresolved/rejected contract
before it can supply candidates for a larger joint search.

Bundle class/race/proficiency/slot rules from the authenticated simulator source.
Use the exact pinned public item catalogue, explicit level-60 actor context and
declared inventory counts by complete item variant. Validate slots, class,
profession presence, race/faction, weapon/armour eligibility, unique groups,
hand pairing and physical-copy limits before accepting a coordinate. Do not
substitute a total item-ID count for copies of a particular enchanted/suffixed
variant.

For modified equipment, require exact preservation through the existing pinned
native equipment probe. Verify the executable hash before and after each run.
A failed, changed, timed-out or non-preserving probe makes the coordinate
unresolved, never valid. Unmodified equipment uses the reference checks without
a probe. Skill ranks and weapon training are not represented fully, and declared
counts are not observed ownership. Keep those/native-legality flags false.

Expose fixed-source CLI/MCP tools and bounded paging. Export accepted equipment
as the existing joint reference shape. Do not alter captured-character matching,
inventory storage, game operations or the native simulator. Larger joint
evaluation follows only after this feature's offline tests and review.
