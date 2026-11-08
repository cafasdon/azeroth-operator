# Species-only pet-journal capture — 8 November 2026

The [Forever pet-journal UI](https://raw.githubusercontent.com/Gethe/wow-ui-source/forever/Interface/AddOns/Blizzard_Collections/Shared/Blizzard_PetCollection.lua)
enumerates filtered journal rows with `GetNumPets`/`GetPetInfoByIndex` and obtains
public species names from `GetPetInfoBySpeciesID`. This is source evidence, not
native-build qualification. Individual pet handles are not required for the
species-level observations implemented here.

`pet_collection_view` requires a visible journal and reads at most 10,000
filtered rows. It aggregates ownership flags by species ID and public species
name, omitting the individual pet GUID, custom name and all other per-pet
details. The API's separate owned-total return is ignored. No filters, UI
selection, summoning, rename, release, cage or battle operation is invoked.
Two full reads compare the exported aggregate and displayed count; visibility
is checked around each read. Restricted or invalid required data, changing
aggregates/count or hidden UI make the section unavailable.

The Forever-only strict contract is:

- `coverage: filtered_journal_species` and `displayed_rows` from zero to 10,000.
- `entries` keyed by canonical positive unsigned 32-bit species IDs.
- Each row has public `name` (1–256 UTF-8 bytes), unsigned `owned_rows` and
  `unowned_rows`; their sum is positive and at most 10,000.
- Summed row counts must equal `displayed_rows`; unknown/private fields reject.

Counts describe journal rows within the filtered result, not unique individual
pet identities or complete account totals. Missing species cannot establish
that a pet is unowned. Valid empty views remain zero observations; unavailable
sections produce null summary counts. Active hunter-pet care remains a separate
section and is unchanged.

Offline verification:

- `cargo build --locked --offline` passed.
- `cargo test --locked --offline --test pet_collection_view` passed one
  Lua-export/Rust-import test for duplicate species aggregation, ignored private
  GUIDs/custom names and global totals, empty views, visibility/count/ownership
  changes, oversized counts, restricted fields and missing public names.
- With `PYTHONPATH=.;tests`, `python -m unittest test_pet_collection_view test_mcp test_snapshots test_packaging`
  passed 13 tests, including real stdio inspection/summary and Python/Rust
  rejection of wrong coverage/family, inconsistent counts, private fields,
  invalid types, overlength names and noncanonical species IDs.
- Formatting and diff checks passed.
- Development archive: `local-data/pet-view-qualification/AzerothOperator-0.0.0-pet-view-test-forever-live.zip`.
  SHA-256 `6b88e9d39350588c90799da7a5ce92f81e32d4bafe3d01f3c47e88e14144bdb9`.

The package was not installed. Live game testing remains deferred.

## Saved-species search

`list_snapshots.pet_species` requires positive-u32 `species_id`, with optional
unsigned-u32 `minimum_owned_rows` and `minimum_unowned_rows`. Both inclusive
thresholds apply to the same observed species row. With no thresholds, any
observed ownership state matches. Even zero requires a row; absent species and
missing/empty sections never match. These predicates describe the filtered
view and cannot prove full account possession or absence.

The compact `pet_species_match` includes coverage, displayed rows, species ID,
public name and the two recorded row counts. No individual identity is added.
Existing owner-root reads, character/family/date/newest and pagination semantics
remain in effect. Feature filters cannot resurrect an older matching capture
when the newest valid capture in the requested time window lacks the section.
Default listings do not include match metadata without the filter.

`cargo build --locked --offline` passed. With `PYTHONPATH=.;tests`,
`python -m unittest test_pet_collection_view test_mcp test_snapshots` passed
11 tests. The real-MCP test covers same-species conjunction, inclusive count
boundaries, observed zero versus absent zero, ownership state separation,
missing-newest suppression, time-window selection, exact metadata, malformed
IDs/counts/private fields, family restriction and unchanged defaults.
Formatting and diff checks passed. No game or installation actions occurred.

Independent capture/security review (gpt-6.1-sol, high effort) found no concrete
flaw. `cargo test --offline --test pet_collection_view -- --nocapture` passed
one test. Eight additional source invariants checked the getter-only path,
ignored private/global-total returns, absence of mutations and sanitised
unavailable markers. The reviewer independently verified tuple ordering and
filtered iteration against the Forever UI source. No source/game changes occurred.

Independent reader/failure review (gpt-6.1-sol, high effort) found no concrete
flaw. `python -m unittest discover -s tests -p test_pet_collection_view.py -v`
passed one test. A separate stdin Python harness ran 60 cases and 250 checks
through Python validation/store reads and the built Rust stdio MCP, with zero
failures. It covered empty/maximal views, canonical/u32 keys, UTF-8 boundaries,
counter types/ranges, private/unknown fields, family, sum consistency and
missing-versus-empty semantics. Hash-addressed invalid files were rejected by
Rust independently of Python ingestion. Temporary owner fixtures were removed.

Jev 1.14.0 advisory scope assessment returned supported (request
`9f4442f4ab0b79f2bd67a0e22aa297206f10e984e902546f0c31cc635ab04e88`,
571 input / 46 output tokens, 0.345 seconds), separately from test evidence.

Jev 1.14.0 advisory search-scope assessment returned supported (request
`16887583a3e45609713e45e4207e9eaaf98811233e1cbd7d01be365c4405251b`,
565 input / 46 output tokens, 0.307 seconds), separately from runtime evidence.

Independent search review ran two lenses (gpt-6.1-sol, high effort), with no
concrete findings:

- Callers/data boundaries: `python -m unittest discover -s tests -p test_pet_collection_view.py -v`
  passed one test in 0.363 seconds. A separate real stdio probe used five valid
  captures and one hash-valid private-field capture with an unavailable
  catalogue path. It passed conjunction, case-insensitive identity, newest per
  variant, pagination/order, absent-zero exclusion, default metadata omission,
  exact six-field metadata and invalid-private-data exclusion. No catalogue,
  native API or network path was used.
- Failure: the same scoped test passed. Real MCP probes checked counts at zero,
  10,000, 10,001 and u32 maximum; maximum species ID; same-row predicates;
  missing/empty/absent data at zero minima; newest/date order; five matches over
  three pages; 13 malformed filters; and six malformed stored files skipped
  with `invalid_count=6`, without a panic. Filter schema retained positive ID,
  nonnegative optional minima and strict unknown-field rejection.

Supplementary probes used temporary synthetic owner stores and made no source,
installation or game changes.
