# Filtered toy view — 8 November 2026

The [Forever toy-box UI source](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_Collections/Mainline/Blizzard_ToyBox.lua)
uses `GetNumFilteredToys`, `GetToyFromIndex`, `GetToyInfo` and `PlayerHasToy`
to render the filtered list. These are the read calls used by this collector.
The source is a mirrored contract reference, not native-client verification.

The new Forever-only `toy_collection_view` section requires the panel to be
visible and records at most 4,096 filtered rows. It validates IDs, names,
explicit collected/favourite flags, and repeats the count/ID/visibility reads
before accepting the view. Invalid, changing, restricted or unloaded data
makes the whole section unavailable. The collector does not call panel, search,
filter, favourite, fanfare or toy-use mutations.

Coverage is always `filtered_journal`. It is never a full account collection
claim. No search text or third-party account information is exported. Empty
results remain empty; closed or failed panels are unavailable. Rust and Python
validate exact cardinality and types. Owner-scoped inspection exposes the view;
summary fields explicitly refer to rows and collected toys within that view.

Offline verification:

- `cargo test --locked --offline --test toy_collection --test mount_collection --test addon`:
  four tests passed, covering Lua export and Rust decode as well as regressions.
- `PYTHONPATH=.;tests python -m unittest test_mcp test_snapshots test_mount_collection test_toy_collection test_packaging`:
  14 tests passed, including real Rust MCP and both reader validators.
- Offline build, formatting and diff checks passed.
- Development-only package `AzerothOperator-0.0.0-toy-test-forever-live.zip` built
  in `local-data/toy-collection-qualification`, SHA-256
  `0d14e490a1424b26348edbeeffa4e0d49ac146c8c66ff01e2987213e19ad60a8`.

The package was not installed. No hosted services were updated and no native
game testing was performed. Readers must be updated before using the exporter.

Independent capture/security and reader/caller reviews (gpt-6.1-sol, high effort)
found no concrete defects. Extra probes passed 21 Lua restricted/malformed/boundary
cases and 49 Python/Rust validation-parity cases, including owner isolation.
Jev 1.14.0 scope advice returned supported, separately from verification:
request `476230270b07062f3831cc01924c8fbb3183581b7b957e600293b40419288882`,
506 input / 46 output tokens, 0.323 seconds.
