# Mount collection capture — 8 November 2026

The Forever collector uses `C_MountJournal.GetMountIDs` and
`C_MountJournal.GetMountInfoByID` as described in the
[mirrored generated API documentation](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/MountJournalDocumentation.lua),
read on 8 November 2026. This source contract is not live-client qualification.

It accepts a complete, stable, nonempty catalogue of at most 4,096 distinct
positive mount IDs. Every row must supply an explicit collection flag and a
matching returned ID. Only collected rows are exported, with name, spell ID,
favourite and recorded usability. No summon, favourite or journal-filter setter
is called. Restricted, missing or inconsistent data makes the section unavailable.

`inspect_snapshot` with `section: "mount_collection"` returns the saved data.
`character_summary.collected_mounts_observed` is null for unavailable data and
zero for an observed empty owned collection. `catalogue_count` describes the
enumerated catalogue, not the number owned. An ID absent from `entries` is not
proof of non-ownership outside that capture's catalogue. Usability describes
that capture only. Account/character sharing and native completeness remain
unqualified. No further owner or third-party identity fields are collected.

Update the reader and importer before using a package that exports this section.
No package was installed and no hosted service was changed in this task.

Offline evidence:

- `cargo test --locked --offline --test mount_collection`: one passed. Exercises
  Lua capture, serialisation and Rust decode, empty versus unavailable,
  duplicate/sparse/changing IDs, wrong identity and restricted data.
- `cargo test --locked --offline --test addon`: two passed.
- `PYTHONPATH=.;tests python -m unittest test_mcp test_snapshots test_mount_collection test_packaging`:
  13 passed, including real Rust MCP readback and malformed reader checks.
- `cargo build --locked --offline`, formatting and diff checks passed.
- Development-only package `AzerothOperator-0.0.0-mount-test-forever-live.zip`
  built under `local-data/mount-collection-qualification`; SHA-256
  `54ff91ee5a78e5f7c817a2b328cf1e590de369178376c93ffd6718c584403a9a`.

Independent capture/security and reader/caller reviews (gpt-6.1-sol, high effort)
found no concrete defects. Extra probes rejected 27 malformed/restricted Lua
cases, accepted the 4,096-ID boundary, and matched Python/Rust acceptance on 42
payload cases. Real MCP probes verified owner isolation and generic comparison.
No native game access was used.

Jev 1.14.0 scope advice returned supported, separately from verification:
request `02c6fa8a5364ee1f65cdaf2a24b7c8e85d700027858a31a4380a8224a4843ad0`,
508 input / 46 output tokens, 0.305 seconds. This does not establish live
API behaviour or grant authorisation for gameplay actions.

## Saved lookup and comparison

Inspection now accepts `mount_id` only with the mount collection section and
Forever family. Present rows report `observed_collected: true`; absent IDs and
missing collections return null instead of inferring non-ownership.

The optional `mount_collection_changes` comparison reports numerically ordered
IDs observed on one side, shared spell/favourite/usability changes, observed
count deltas and catalogue counts. Missing collections produce a null comparison.
Names are excluded so localisation alone does not appear as a shared-row change.
Existing owner, character, client/build and chronological checks still apply.

The extended real-MCP test covers lookup, unavailable versus absent rows,
numeric ordering, changing flags, ignored names, missing comparison input,
disabled output and invalid family/section combinations. The regression command
`PYTHONPATH=.;tests python -m unittest test_mcp test_snapshots test_mount_collection`
passed 11 tests; offline build, formatting and diff checks also passed.

Jev 1.14.0 scope advice for lookup/comparison returned supported, separate from
tests: request `f26dfdab8b13801118d76b10be6c7570aeb470fe769e4e3e6ee41ad1d2fbc415`,
472 input / 46 output tokens, 0.287 seconds.

Independent caller/failure review (gpt-6.1-sol, high effort) found no defects.
Additional real-stdio probes covered u32 boundaries and malformed selectors,
conflicts with spending/other selectors, empty/unavailable collections,
negative count changes, spell changes, name-only differences, default omission,
owner separation and rejected client/character or chronological mismatches.
