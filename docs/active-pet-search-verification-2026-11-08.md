# Saved active-pet search — 8 November 2026

Source and offline verification only. No deployment, installation, game
interaction or native capture. This adds analysis of existing observations,
not capture APIs or gameplay actions.

`list_snapshots.active_pet` is a strict nested Forever filter. It supports
recorded `present`, exact positive `family_id`, inclusive positive
`minimum_level`/`maximum_level`, and inclusive signed
`minimum_training_points_balance`/`maximum_training_points_balance`.
Balance means captured training total minus used, not eligibility to learn
a skill. Its range is ±4,294,967,295, derived from signed 32-bit counters.
The same checked signed-64-bit arithmetic helper is used by summaries.

At least one predicate is required. Empty filters, unknown keys, invalid
types/IDs, out-of-range or reversed bounds, and `present: false` combined
with detail predicates are rejected. Missing presence never matches; false
presence must be explicitly observed. Detail predicates imply a present
pet and require each requested value to be known. An unknown balance does
not match even a zero minimum or maximum. Other unrequested fields may
remain unknown. All predicates refer to the same captured pet observation.

Matching rows add only `active_pet_match` with presence, family ID, level and
signed balance; unknown unrequested fields remain null. No pet names, GUIDs,
loyalty labels or other private strings are added. Results without the
filter omit this metadata. Existing owner-root/hash/schema validation,
character/time filtering, newest-per-character selection, feature conjunction
and pagination remain in force. Newest selection runs before the new
feature filter, so a later unavailable capture does not expose an older
matching one. A time bound can intentionally restrict that newest selection.

This does not identify the character currently in play or establish present
pet ownership, stable roster, training eligibility or simulator compatibility.

Author verification:

```powershell
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
python -m unittest test_active_pet_search test_pet_training_points test_active_pet_details
```

Build passed; three real Rust MCP stdio integration tests passed. New search
cases cover family and combined ranges, exact levels, deficits, known zero,
missing fields/sections, present/absent distinction, signed extremes,
pagination after filtering, newest capture with missing data, date-bounded
newest selection, corrupt-row rejection, unsupported family and invalid
nested predicates. Existing training-point summaries and reader contracts
passed after sharing the balance helper.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort, with no concrete finding. Callers verified the real advertised
strict schema, eight additional malformed/contradictory inputs, conjunction,
identity/date/offset exclusion, unknown-zero handling and summary agreement.
Failure review rejected 49 invalid filters and passed 240 seeded independent
oracle queries (seed 87021) over ranges, unknowns, character/time/latest,
conjunction and pagination. Five malformed/private stored records were
excluded on every query, and match output retained its four-field allowlist.
Existing deserialisation errors can echo caller-supplied invalid strings;
these probes did not expose stored private data. No live operations occurred.

Jev advisory scope assessment supported offline feature completion while
leaving broader/native completion open: `jev-1.14.0`, request
`26fd09b41709984f5f9ad7ffb4a87115294b81879904f805fcc8d78954bd7216`,
694 input / 51 output tokens, 0.318 seconds. Tests and actual caller probes,
not this advisory result, establish the scoped verification.
