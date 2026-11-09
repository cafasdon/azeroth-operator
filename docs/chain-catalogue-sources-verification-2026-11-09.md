# Chain acquisition source references — 9 November 2026

Base source: `5ed780d`. ADR 024 connects complete chain external item/quantity
shortages to existing pinned public catalogue records in one opt-in review.
No new public dataset or network fetch was used. Local cache inspection found
6,043 item records; common reagents 2840 and 3470 are absent. This implementation
does not fill that world-data gap or infer absence of an acquisition source.

## Author evidence

```text
cargo test --locked --offline --lib chain_source_references_bound_and_validate_leaves
1 passed; 0 failed; 27 filtered out; test duration 0.00s
cargo build --locked --offline
Finished dev profile in 7.40s
```

The unit fixture exercises ten sources retained as eight, original source count,
truncation, reference indices and omitted ninth NPC name. It distinguishes empty
source metadata, missing source metadata and unknown items, preserves u64 maximum
shortage quantities and rejects zero/oversized/duplicate IDs, zero/Boolean
quantities and 513 leaves. Blocked chains return null.

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_chain_catalogue_sources test_recipe_reagent_chain test_reagent_chain_merchant test_reagent_chain_stock -v
```

Four tests passed in 1.643 seconds using the actual local Rust stdio MCP and
synthetic private captures. The new test uses the existing checksum-pinned local
public catalogue: vendors for 2505, quest for 744, drop for 872, crafted source
for 2870, absent source metadata for 720 and unmatched reagent 2840. It compares
source arrays directly with the cached records and verifies provenance, counts,
reference projection, false verification flags and private-link exclusion.

Observed stock reduces item 2505 demand from three to two exactly once. A later
compatible inventory with three units removes that leaf. Blocked/empty chains,
stock/merchant composition, omitted/false invariance, incompatible identity,
wrong mode, invalid Boolean and batch rejection are asserted. Missing catalogue
and a changed configured file reject; a successful earlier cache read does not
bypass subsequent checksum verification. Opt-out still works with the corrupt
cache. Capture bytes remain unchanged. The new test skips explicitly if no local
pinned dataset is installed; this recorded run used the installed dataset.

No game client, installation, deployment or native qualification occurred.
Current availability, eligibility, ownership, route optimality and full world
sources remain unproven. Live testing remains deferred.

## Planning advisory

Jev `jev-1.14.0`, request
`2c2e4c1a22e7b2a11282e403cb8dc3bda8f6f8f629a93ec1803b5872be7701e6`,
597 input / 47 output tokens, 0.280 seconds: proceed 0.92, revise 0.05,
insufficient-evidence 0.03, confidence 0.88. Advisory only, not proof or approval.

## Independent review

One round, Callers and Failure/security, using independent `gpt-6-astra` agents
at high effort. Both found no actionable defect; no correction round was needed.
Product source remained frozen during review.

Both ran the compiled Rust unit directly:

```powershell
& target/debug/deps/azeroth_operator-4fc27cedc19591d7.exe chain_source_references_bound_and_validate_leaves --nocapture
```

Each reported one pass, zero failures, 27 filtered. Callers ran
`python -m unittest discover -s tests -p test_chain_catalogue_sources.py -v`:
one pass in 0.963s. Failure/security ran
`python -m unittest tests/test_chain_catalogue_sources.py -v`: one pass in 0.859s.
Both used the actual local Rust stdio MCP; neither rebuilt or edited source.

Callers independently probed a two-step producer chain with two carried and
three banked units of item 2505. External leaves were 744:4, 872:2 and 2505:3;
produced intermediate items were absent. It asserted opt-out equality, public
source-array/provenance matches and one-based NPC/zone/faction reference joins.
Explicit null/string/object/array/integer flag values rejected. An initial
synthetic fixture was invalid; correcting the fixture passed, with no source
change or confirmed product fault. It independently confirmed 6,043 unique
cached items, absent 2840/3470 and cache SHA-256
`5ca68229623b8bc1f9a085ed3b5f564ab82ff66055503749e324d640ac04640e`.

Failure/security's bounded inline actual-MCP probe rejected realm/build/interface/
variant inventory mismatches, six non-Boolean flag values and three incompatible
mode combinations. Missing inventory produced blocked/null annotations. An
eight-step synthetic chain returned 505 unmatched external items, correct totals
and 201,635 response characters in 0.095s. This is one synthetic measurement,
not a native/general performance claim. After a valid load, deleted cache,
directory, empty/truncated/hash-modified and 8 MiB-plus-one files all rejected;
false opt-in remained usable, and restoring pinned bytes recovered identical
annotations. Provenance matched bytes; output excluded local paths and character
name. It also checked truncation references derive only from retained sources.

Both checked the updated docs and caller/unknown-data contracts. Neither used
HTTP, native game, installation, deployment, Git mutation or SQLite. Their
temporary synthetic probes were bounded and no live qualification was performed.

Final scoped Jev advisory: `jev-1.14.0`, request
`815c6ddd02775520d57cf398ae48371374e033ea325e4b69dcb5f37812c26213`,
797 input / 50 output tokens, 0.296 seconds; land-scoped 0.94, fix-first 0.04,
insufficient-evidence 0.02, confidence 0.91. Advisory only, not proof or approval.
