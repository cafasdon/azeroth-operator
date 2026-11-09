# Optional local recipe item references — 9 November 2026

Base source: `94ca54c`. This source-only integration reads public reference IDs
from a separately installed compatible provider during manual Forever export.
It neither downloads nor bundles the provider, its data or its code.

## Primary API research

Inspected QuestieDB revision `c1b50fefb19c064edc436429bef73435d9ff4254`:

- [Public API](https://raw.githubusercontent.com/Questie/QuestieDB/c1b50fefb19c064edc436429bef73435d9ff4254/docs/api.md):
  entity existence/field reads, contract-range compatibility and mode status.
- [API implementation](https://raw.githubusercontent.com/Questie/QuestieDB/c1b50fefb19c064edc436429bef73435d9ff4254/src/api.lua):
  public namespace publication and `RequireContract(1)` semantics.
- [Mode status](https://raw.githubusercontent.com/Questie/QuestieDB/c1b50fefb19c064edc436429bef73435d9ff4254/src/ui/modeIndicator.lua):
  status reads report mode, expansion and contract version. The collector
  requires an explicit Forever expansion; unavailable status is not guessed.
- [Item schema](https://raw.githubusercontent.com/Questie/QuestieDB/c1b50fefb19c064edc436429bef73435d9ff4254/src/meta/itemMeta.lua):
  drop/container/vendor/quest source ID arrays. Empty arrays are normalised to
  nil, so missing fields are not evidence of no sources.
- [Licensing notice](https://raw.githubusercontent.com/Questie/QuestieDB/c1b50fefb19c064edc436429bef73435d9ff4254/LICENSE.md):
  historical contribution rights have explicit caveats. Do not describe all
  provider content as unconditionally GPL-covered or redistribute a bundled
  dataset on that assumption. No redistribution was added here.

This pins the inspected API research, not the installed provider's dataset.
Captures record reported API contract and mode only; dataset revision, accuracy
and completeness remain unverified. Runtime corrections can affect those reads.

## Author evidence

```text
cargo build --locked --offline
Finished dev profile in 7.24s
cargo test --locked --offline --test recipe_item_sources --test profession_recipe_materials
profession_recipe_materials: 1 passed; 0 failed; 0.33s
recipe_item_sources: 1 passed; 0 failed; 0.07s
```

MLua executes the actual collectors and Core export, then imports the hex export
with the strict Rust decoder. The fixture provider is an API double, not an
installed/native qualification. It asserts unique sorted IDs, explicit empty
versus nil data, partial secret/error/sparse/oversized field omission, unstable
and failing existence handling, ten-to-eight truncation, contract/flavour/mode/
context failures, missing provider/materials, empty materials and a 65-item input
with exactly the numerically first 64 queried. Existing bulk material capture
remains covered. Private markers stay absent from serialised captures.

```powershell
$env:PYTHONPATH='.;tests'; $env:TEMP='C:/Temp'; $env:TMP='C:/Temp'
C:/Python314/python.exe -m unittest test_recipe_item_sources test_profession_recipe_materials test_reported_craftable_count -v
```

Three tests passed in 1.746s using the actual local Rust stdio MCP. The new
test accepts absent/empty/normal/maximum-count sections, all three item statuses,
source/baked modes, u32 maximum source ID and 4,096-to-eight reference bounds.
Malformed metadata, unknown/private fields, null/list/status errors, noncanonical
IDs, zero IDs, non-Boolean flags, inconsistent counts/truncation, 65 retained
items and wrong client families reject in both strict Python and hash-valid
Rust readers. Inspector readback equals the accepted section. Capture files
remain byte-identical and private error markers are absent from responses.

No public dataset was fetched, no game client was accessed and no installation
or deployment occurred. Public API research did use the network. Native provider
availability, loading/order, actual data, secret-value engine behaviour, capture
latency and policy qualification remain untested. No source is treated as
currently available or as an automatic acquisition action. Chain joins and
world routing using this new owner-local section remain separate unfinished work.

## Planning advisory

Jev `jev-1.14.0`, request
`0f7892a2db62c8cba46e6448fd326cad6816c60afa8f984d128c23ffc5ef884c`,
722 input / 47 output tokens, 0.291s: proceed 0.81, revise 0.18,
insufficient-evidence 0.01, confidence 0.72. Advisory only, not proof or approval.

## Independent review

One round with two independent `gpt-6-astra` agents at high effort: Callers and
Failure/security. Both found no concrete defect; no correction round was needed.
Source stayed frozen during review. Both directly ran the compiled provider and
bulk-material MLua test executables: one pass, zero failures each.

Callers ran:

```text
PYTHONPATH=.;tests python -m unittest test_recipe_item_sources -v
Ran 1 test in 0.411s — OK
```

Its independent actual-Rust-stdio mutation probe covered 11 metadata/row/category
fields across 12 bad/boundary types plus a valid base: 133 cases, two accepted,
zero Python/Rust acceptance mismatches and zero unexpected Python exceptions.
It traced manual export, successful Core section handoff, hex decoding,
whole-snapshot validation and hash-bound owner-store inspection. Old snapshots
remain supported; the new section stays Forever-only.

Failure/security ran:

```text
python -m unittest discover -s tests -p test_recipe_item_sources.py -v
Ran 1 test — OK
```

Its additional actual-MCP probe produced:

```json
{"cases":321,"accepted":20,"rejected":301,"python_exceptions":{"ValueError":301},"parity_mismatches":[],"private_marker_leaks":[],"path_or_unknown_id_probes_rejected":6}
```

Those mutations included all new field types and bounds, malformed/Unicode/
oversized IDs, partial rows and reference structures. It also inspected public
numeric-only capture, controlled failure labels, context failure omission and
the absence of a new external-request path. Both checked the completed docs
against source; no source edits, builds, installation or native game calls ran.

For a direct API-double comparison, the author cached four immutable upstream
API text/source files locally without executing them. Callers independently
verified their SHA-256 values and read the originals:

| Upstream path | SHA-256 |
| --- | --- |
| `docs/api.md` | `8df9170a999c5b9368a19980a665967335bdfb28b2df0c9aa7ce8977b51ae00b` |
| `src/api.lua` | `4208f7e2a8cc490bfc1e73ff794fc736d807fde0fb3fee29971bfe9b49f16923` |
| `src/meta/itemMeta.lua` | `a11209ef102e9cc0d1d199de4e7d662a515964a36f3013aafce9b6a14bae509f` |
| `src/ui/modeIndicator.lua` | `e076aaf10a2777279bbd205747c2ac64edc422d75bb73e486715181cedd3bc2f` |

It found no material API-double mismatch. Potentially nil expansion correctly
rejects; explicit empty-array fixtures exercise defensive handling rather than
normal provider output. This original-file check is separate from native
acceptance and does not prove the user's installed provider matches the pin.

Final scoped Jev advisory: `jev-1.14.0`, request
`03707c7b9fcf3dc55c491dd0eb0acd2f18850d757f834e6d6a8dc30ac75f1a05`,
873 input / 49 output tokens, 0.334s: land-scoped 1.0, fix-first 0.0,
insufficient-evidence 0.0, confidence 1.0. These uncalibrated advisory values are
not proof, permission, native qualification or a policy-compliance guarantee.
