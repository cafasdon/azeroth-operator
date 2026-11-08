# Saved world-boss capture — 8 November 2026

The [Forever raid-info UI source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_RaidFrame/Mainline/RaidInfoFrameSocialView.lua)
enumerates `GetNumSavedWorldBosses` and reads name and reset from
`GetSavedWorldBossInfo`. This is a separate list from saved dungeon/raid
instances. Source API presence is not native-build qualification.

The `saved_world_bosses` section waits for the existing passive
`UPDATE_INSTANCE_INFO` observer. Entering the world invalidates readiness.
It exports only names and reset seconds, plus update age and `updated_cache`
coverage. It never requests a refresh or exports the API's second return.
The section is Forever-only and owner-scoped through the normal snapshot store.

The count is bounded at 128. Dense numbered rows contain only a nonempty UTF-8
name up to 256 bytes and signed 32-bit reset seconds. Fractional timers round
up; negative values remain observations. Restricted or invalid returns,
changed count, invalid clock and changed observer readiness make the section
unavailable. Zero rows remain a valid empty capture. Summary counts use null
for unavailable observations. No current loot eligibility is inferred.

Offline verification:

- `cargo build --locked --offline` passed.
- `cargo test --locked --offline --test saved_world_bosses --test saved_instances`
  passed two Lua-to-export-to-Rust-import tests.
- With `PYTHONPATH=.;tests`, `python -m unittest test_saved_world_bosses test_saved_instances test_mcp test_snapshots test_packaging`
  passed 14 tests, including real MCP stdio readback, schema rejection and
  owner-scoped summary semantics.
- Formatting and diff checks passed.
- Development archive: `local-data/world-boss-qualification/AzerothOperator-0.0.0-world-boss-test-forever-live.zip`.
  SHA-256 `b91a6727d1d3eb2ab4441e8b7fc62ee93ad89e4cd1781028ef8e922e65896abc`.

The package was not installed. Native game testing remains deferred.

## Saved-character search

`list_snapshots.saved_world_boss` requires a bounded nonblank name and Forever
family. Name matching is exact after lowercasing, without whitespace trimming.
Optional reset and update-age ceilings are inclusive captured values. Negative
reset sentinels are excluded when a reset ceiling is requested. Missing rows or
cache sections never match. Output includes at most 20 matching rows in numeric
slot order, plus complete matched count and a truncation flag.

Existing owner-root reads, character/time bounds, newest-capture semantics and
pagination are retained. A newest capture without the requested observations
suppresses the older match. The time window is applied before newest selection.

`cargo build --locked --offline` passed. With `PYTHONPATH=.;tests`,
`python -m unittest test_saved_world_bosses test_saved_instances test_mcp test_snapshots`
passed 12 tests. The real stdio test covers exact case-insensitive name matching,
timer/age boundaries, negative timers, missing-newest suppression, time-window
selection, numeric truncation, invalid arguments, family restriction and
unchanged default listings.

Jev 1.14.0 advisory scope assessment returned supported, separately from
runtime verification (request
`494788a3842f0eb7798a6ac47cb25e976eac16ecdfa38171177b777e289a8624`,
496 input / 46 output tokens, 0.294 seconds).

Independent review completed with no concrete findings:

- Capture/security lens (gpt-6.1-sol, high): `cargo test --test saved_world_bosses -- --nocapture`
  passed one test. The reviewer checked the Forever UI return tuple and confirmed
  the collector has no refresh/extension calls or identifier output. Restricted
  names, ignored restricted IDs, NaN, changing counts and world-entry invalidation
  were exercised offline.
- Reader/caller lens (gpt-6.1-sol, high): `PYTHONPATH=.;tests python -m unittest discover -s tests -p test_saved_world_bosses.py -v`
  passed one test in 0.227 seconds. A separate transient 51-case matrix agreed
  across Python validation, real Rust stdio MCP inspection and `import-stdin`.
  It covered numeric endpoints/overflow, malformed types, UTF-8 byte boundaries,
  0/128/129 rows, sparse/noncanonical keys, disallowed identifiers, field/coverage
  errors and non-Forever families. Three additional real MCP cases confirmed
  missing and unavailable sections give null summary counts while empty gives zero.
  The transient matrix is review evidence; the committed test above is the
  retained reproducible regression check.

Jev 1.14.0 advisory scope assessment for search returned supported, separately
from runtime evidence (request
`3446d47db4c7f5f176ee1be813ca738b2eb8c0419ad7d5b70b6490c976c9ea10`,
491 input / 46 output tokens, 0.323 seconds).

Independent failure/caller review (gpt-6.1-sol, high effort) found no concrete
flaw. `python -m unittest discover -s tests -p test_saved_world_bosses.py -v`
passed one test in 0.344 seconds. Separate bounded real-MCP probes checked
pagination/has-more, same-row name/reset matching, inclusive zero limits,
newest-before-feature and date bounds, missing/empty exclusion, exact names,
19 malformed payloads, invalid-slot rejection and external-store isolation.
The probe reported all checks passing. No live or installation actions occurred.
