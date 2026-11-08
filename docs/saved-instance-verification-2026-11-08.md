# Saved-instance cache capture — 8 November 2026

The [Forever raid-info UI source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_RaidFrame/Mainline/RaidInfoFrameSocialView.lua)
reads `GetNumSavedInstances` and the first ten return values of
`GetSavedInstanceInfo`, responding to `UPDATE_INSTANCE_INFO`. The collector
uses this source contract, with native qualification still pending.

A passive observer accepts successful instance-info update events and clears
readiness on `PLAYER_ENTERING_WORLD`. It never invokes `RequestRaidInfo` or
extension/acceptance APIs. Until an update is observed, capture is unavailable.
Updated data includes its age, at most 128 dense rows, public instance name,
difficulty ID/name, lock/extension and raid flags, instance size and the reported
reset timer rounded upward to seconds. Negative timers remain negative.

The unique lockout ID and high identifier component are discarded, as are group
details and chat links. Exact Python/Rust schemas reject unknown identity fields.
This is cached observation, not current access eligibility, a reset forecast
or a record of world-boss saves. No inspection advances timers.

Verification:

- `cargo test --locked --offline --test saved_instances --test addon --test mount_collection --test toy_collection`:
  five tests passed, including passive update readiness, world-entry invalidation,
  bounded failures and private-ID omission through Lua/Rust round trip.
- `PYTHONPATH=.;tests python -m unittest test_mcp test_snapshots test_mount_collection test_toy_collection test_saved_instances test_packaging`:
  15 tests passed, including real MCP readback and schema parity cases.
- Offline Rust build, formatting and diff checks passed.
- Development package `AzerothOperator-0.0.0-lockout-test-forever-live.zip` built
  under `local-data/saved-instance-qualification`, SHA-256
  `30245fda0d8cb1e392d514800a8cab39b906ff85f4887dcb6b88b493564e8861`.

Nothing was installed or deployed; no live game requests were sent. Update the
reader and importer before using an exporter with this section.

Independent capture/security and reader/caller reviews (gpt-6.1-sol, high effort)
reported no concrete flaws. Additional probes covered secret/malformed timer
and flag values, failed timestamp reads, clock rollback, private-ID omission,
53 Python/Rust schema cases and separate owner stores. Jev 1.14.0 scope advice
returned supported (request `b39ca73222c82d1df6e360eefbe4536908a32268bc39552e4ad5bfeb419854af`,
502 input / 46 output tokens, 0.364 seconds), separately from verification.

## Saved-character search

`list_snapshots.saved_instance` requires an exact case-insensitive name and
optionally matches difficulty, lock/extension and raid flags, nonnegative reset
seconds and the age of the cache update at capture. All predicates apply to
one entry. It cannot infer an unsaved state from a missing entry or section.

The result includes a matched count and at most 20 rows ordered by their
numeric captured slot, with truncation flagged. Standard character/date/newest
filters remain in effect. A maximum update age is not a live freshness check.

The real-MCP fixture checks conjunctive matching, explicit false lock flags,
inclusive reset/update-age boundaries, negative timer exclusion, missing-newest
suppression, family/name rejection and the 20-row result limit. The regression
command `PYTHONPATH=.;tests python -m unittest test_mcp test_snapshots test_saved_instances`
passed 11 tests. Offline build, formatting and diff checks passed.

Independent failure/caller review (gpt-6.1-sol, high effort) found no concrete
failure across 51 extra real-MCP checks: same-row predicates, false flags,
timer/update-age boundaries, date/newest order, pagination, 128-row truncation,
malformed arguments/snapshots, private-ID rejection and unchanged defaults.
Jev 1.14.0 scope advice returned supported (request
`7d2136864f3e95e1614eb103d061c3546a8c9f322896e633080f8bcd645e5eb0`,
471 input / 46 output tokens, 0.316 seconds), separately from runtime evidence.
