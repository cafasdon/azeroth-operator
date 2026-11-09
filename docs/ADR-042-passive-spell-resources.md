# ADR 042: public spell resource observations with exact-ID references

Date: 9 November 2026. Status: source/offline; no installation.

Spellbook observations preserve identity, passive/off-spec flags, learned level
and API usability but omit charges, cast counts and cooldown flags. Add independent
optional primitive groups from the reported spell ID's public native getters:
`charge_count`, `reported_cast_count` and `cooldown_state`.

Place getter lookup, call and projection inside protected reads. Secret-screen
containers and selected fields before operations; require strict bounded count/
Boolean shapes and agreement across two reads. Ignore every timer, duration,
mod-rate, duration-object and GCD field. Recheck exact spellbook slot type/base/
reported IDs before attaching any resource data. Optional failure never discards
existing slot metadata; failure of one group does not discard other stable groups.

Both readers reject null/partial/unknown/malformed resource objects. Summary counts
rows, not unique abilities or independent pools. Action-bar references retain
numeric book slots and query IDs, joining only exact reported IDs, never base-ID
aliases alone. Preserve duplicate rows without merging conflicts. Omit the new
action-resource map for legacy books without new evidence. Existing identity/flag
matching remains unchanged. No request parameter or MCP action route is added.

The initial plan advisory preferred revision with low confidence. Author tightened
the planned action join to exact reported IDs, avoiding alias-based resource
attribution. The original advisory remains recorded, without a favourable reroll.
Concrete tests and independent review determine the source-only disposition.

Observations are historical, not castability or learning guarantees. No elapsed
recharge, target/range/power eligibility, restricted-data recovery, GCD inference,
casting/targeting, event hook or external lookup. Native availability, secret
semantics, responsiveness and complete class/spell coverage remain unqualified.
