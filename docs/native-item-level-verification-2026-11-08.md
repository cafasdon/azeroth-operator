# Native item required-level evidence — 8 November 2026

The [Forever item API documentation](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/ItemDocumentation.lua)
identifies `GetItemInfo`'s fifth return as the minimum level and
`GetItemInfoInstant`'s first return as the base item ID. The documented usability
and equippable-item getters do not establish a general player-can-equip verdict.
This change uses only the narrower level observation.

The Forever exporter optionally adds `required_level` to equipment and the
shared bag/bank item rows. It queries the captured link only when readable,
nonempty and bounded, verifies the native link ID against the captured item ID,
then accepts only a readable integer from zero through u32 maximum. Missing
APIs, missing link, mismatched ID, uncached details, restricted values, invalid
types and getter errors leave the metadata absent. Zero is retained only when
explicitly returned. Ignored API names/other returns are never exported.
No equip, tooltip UI, explicit request-load or other game action is invoked.

The item schema accepts optional/null unsigned-u32 level metadata only when a
nonempty observed link is present for a known value. Both validators enforce
the new field. Capture is Forever-only; the generic item contract can preserve
this optional metadata without adding execution support for other clients.

For a changed equipment selection, the review compares the exact source
location's captured required level with a positive captured player level.
Known shortfalls populate `native_required_level_shortfalls` and withhold the
simulator equipment spec, independently of catalogue conflicts. Targeted
owned-gear reviews expose `native_required_level_shortfall`; the ranker skips
those candidates and recognises the same check in resolved draft batches.
Same-slot equipped selections and empty-selection baselines remain untouched.
Unknown levels do not add a native blocker, and existing catalogue checks still
apply. No general equip eligibility, proficiency, active usability, native
qualification or complete best-in-slot coverage is inferred.

Offline evidence:

- `cargo build --locked --offline` passed.
- `cargo test --locked --offline --test item_required_level` passed one Lua-
  export/import test for equipment and shared container capture, zero versus
  unknown, restricted/malformed/missing/error returns, ID mismatch, absent
  link, ignored private strings and no non-Forever capture.
- With `PYTHONPATH=.;tests`, `python -m unittest test_native_level_requirements test_gear_draft_dedup test_mcp test_snapshots`
  passed 12 tests. The new real-MCP test uses a native requirement above the
  catalogue's for a synthetic bag ring and checks direct review, targeted owned
  review and ranker exclusion, plus equality/zero/unknown acceptance under the
  remaining checks. It requires the pinned local catalogue; no test skipped.
- Formatting and diff checks passed.
- Development archive: `local-data/required-level-qualification/AzerothOperator-0.0.0-required-level-test-forever-live.zip`.
  SHA-256 `1b7c61b7bdc5aa3a85a07a105be394951be66d87176fa3dd8967e9f451b8797d`.

The package was not installed. Native game testing remains deferred.

Independent reviews used gpt-6.1-sol at high effort; neither found a concrete
flaw:

- Capture/security: `cargo test --offline --test item_required_level -- --nocapture`
  passed one test; `python -m unittest discover -s tests -p test_snapshots.py -q`
  passed six. A separate Python contract harness passed 48 assertions across
  storage kinds and numeric/link/UTF-8/null boundaries. An stdin-compiled mlua
  harness (`target/debug/required_level_capture_adversary.exe`, removed after
  use) passed 118 capture assertions across equipment, bags and both banks,
  covering missing/restricted/error returns, identity mismatches, zero/u32
  maximum and ignored private returns. Only getters entered the new path.
- Failure/callers: read-only PowerShell/Python here-string harnesses exercised
  the real MCP and ranker with temporary snapshots. Bags, both banks and moved
  equipment blocked known shortfalls; equality/zero/missing data, missing or
  nonpositive player level, same-slot selections and empty baselines retained
  their intended behavior. Batch and one-/two-slot ranker paths agreed. A
  catalogue conflict remained blocking with native zero; native 30 reported
  both independent conflicts. Wrong-slot/source/expected-ID and unresolved-link
  guards stayed effective. Legacy payloads remained readable.

Jev 1.14.0 advisory scope assessment returned supported (request
`e04917e46758fa4345852cdf15f5dd95b9fa2b942acdefa89975b8623e9a7c58`,
617 input / 46 output tokens, 0.281 seconds), separately from source and runtime
verification. Reviews performed no game, installation or source changes.
