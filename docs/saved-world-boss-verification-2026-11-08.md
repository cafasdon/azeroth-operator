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
