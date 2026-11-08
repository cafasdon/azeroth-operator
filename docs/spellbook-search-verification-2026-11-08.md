# Saved spellbook search — 8 November 2026

Source and offline verification only. No deployment, installation, native
capture, game interaction or new capture API.

`list_snapshots.spellbook_spell` is a strict nested Forever filter requiring
a positive unsigned-32-bit `spell_id`. It matches either the recorded base
ID or reported spell ID, consistent with existing spellbook inspection.
Optional `passive`, `off_spec`, `minimum_level_learned` and
`maximum_level_learned` apply to that same row. Level bounds are inclusive
unsigned-32-bit values and must be ordered. Missing flags cannot match false;
missing reported learn levels cannot match even a zero bound. Unrequested
metadata can remain unknown. Separate rows or captures cannot combine to
satisfy a conjunction. Empty/missing spellbooks yield no match.

Matches add `spellbook_spell_match` with `visible_player_spell_slots`
coverage, requested spell ID, matched-slot count, truncation flag and at
most 20 entries sorted by numeric slot. Each entry contains only slot,
base/reported IDs, passive/off-spec flags and reported learn level; unknowns
remain null. Names and unrelated rows are omitted. Count is observed slots,
not unique learned abilities. No match metadata is added without the filter.

Existing owner-root/hash/schema validation, character/time bounds, newest-
per-character selection, other feature conjunctions and pagination remain
in force. Newest selection happens before feature filtering: newer empty,
missing or metadata-unknown observations do not permit fallback to an older
matching capture. A time bound can explicitly select historical coverage.
The filter does not identify the character currently in play, prove a missing
spell unlearned, or establish current usability, full learned-spell coverage,
spell rank or static eligibility. Reported learn levels may be level-linked.

Author verification:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_spellbook_search test_spell_learned_level test_active_pet_search
```

Build passed; three actual Rust MCP stdio integration tests passed. New
cases cover base/reported IDs, cross-row predicate exclusion, unknown flags
and levels, known zero/max values, 25-slot truncation and numeric ordering,
name omission, absent/empty books, malformed stored records and filters,
other-feature conjunction without cross-capture mixing, pagination, newest
empty/unknown captures and historical cutoffs. Existing metadata comparisons
and active-pet search paths passed alongside the new filter.

`cargo test --locked --offline --lib
listing_preserves_validation_family_order_and_pages` passed the existing
store listing test. Format and whitespace checks passed.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, with no concrete finding. Callers re-ran the three MCP test modules,
verified actual strict/required-ID schema, constructed base/reported-ID alias
collisions with same-row predicates and inclusive ranges, and rejected five
noncanonical stored slot IDs before parsing. Failure review excluded 22
malformed/hash-invalid records, rejected five additional query forms and
confirmed null unknowns cannot satisfy false/zero, bounded metadata and name
omission. No compiled probes, cleanup actions or live operations occurred.

Jev advisory scope assessment supported offline feature completion while
leaving broad/native completion open: `jev-1.14.0`, request
`48c54f1a8994b988fa8b466ee28bc14f5e69009d893bc906db96cf33cfab3dde`,
717 input / 51 output tokens, 0.282 seconds. Tests and actual caller probes,
not this advisory result, establish the scoped verification.
