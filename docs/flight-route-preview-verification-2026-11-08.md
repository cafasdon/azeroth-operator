# Flight-route previews — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broader in-game coverage remains unfinished.

The public [Forever flight-map provider](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_FlightMap/FM_FlightPathDataProvider.lua)
reads `GetNumRoutes(destinationSlot)` and source/destination slots from
`TaxiGetNodeSlot(destinationSlot, segment, source)` for preview geometry.
It skips unavailable map pins; this does not expose a complete world graph.

Manual export now optionally adds `observed_route_node_ids` on reachable,
non-transition destinations. It is a compact ordered object with string keys
`1` through `N`, consistent with the exporter's object-only contract. The
first ID is the unique current origin, the last is the destination, and
intermediate IDs resolve to unique non-transition rows on that captured map.
The collector bounds native segment counts to 1–64, verifies continuity,
rejects repeated nodes and compares two complete path observations. Restricted
counts/slots are screened before use; missing APIs, errors, invalid bounds,
unknown slot references, transitions, ambiguity or changing previews omit
only the optional path. Native error text and vector/slot objects are not
exported. Map identity and visibility are rechecked after preview collection.

Both readers accept 2–65 unique positive u32 node IDs, exact compact sequence
keys, matching endpoints and referenced map rows. Null, malformed and
contextually invalid paths are rejected. Reader validation checks referenced
IDs for uniqueness; collector ambiguity anywhere on the panel conservatively
suppresses preview capture. An intermediate node's recorded availability
does not independently establish a selectable flight to it.

The shared saved-route helper conditionally copies this annotation into
`list_snapshots.flight_match`, search results and explicit itineraries. Each
leg retains its existing `map_id` and capture context. One native preview
still represents one quoted flight. Fare totals, player itinerary leg limits,
route objectives and budget calculations are unchanged. No segment fare,
travel-time, movement or current-availability claim is introduced. Legacy
captures remain valid and do not gain null annotations. New exports require
updated readers.

Verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test flight_route_nodes --test flight_map_positions --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_flight_route_nodes test_flight_map_positions
cargo fmt --check
git diff --check
```

Build passed; four Rust tests and two real MCP Python tests passed. Cases
include direct and multi-segment capture; missing APIs, restricted values,
errors, over-limit/zero/changing counts, unknown slots, discontinuity, cycles,
wrong endpoints, duplicate IDs, transitions, changing maps and panel closure;
section/fare preservation; strict reader agreement on malformed and contextual
inputs; conditional legacy output; all three saved-route callers; and
unchanged fare totals and itinerary leg counts. The Python reader/server
fixture also accepts the maximum 65-node path.

Development package, not installed:
`local-data/flight-preview-qualification/AzerothOperator-0.0.0-flight-preview-test-forever-live.zip`.
SHA-256: `1002449e89cb7def4cb52f79c0cafa231dc44d821745517affa73000d09e169c`.

Jev plan advisory selected proceeding with offline implementation, while
leaving native coverage unproven: `jev-1.14.0`, request
`bebc494677bc5fb7ceab95f8cec17ec076e1b16e66fc2d99e8d10102d0a96772`,
615 input / 51 output tokens, 0.279 seconds. The planned sequence was encoded
as a compact indexed object rather than an array to preserve the existing
export contract. Advisory probabilities were 0.73 proceed, 0.21 revise and
0.06 insufficient evidence; source review and actual tests remain necessary.

Independent callers and failure/security reviews used `gpt-6-astra`, high
effort. Neither found a concrete defect. Both ran the real MCP integration
and existing compiled Lua/Rust fixture; callers also ran the position
regression. Additional in-memory probes accepted a maximum-length preview
with unknown fare and rejected negative, floating, missing and extra-key
values. Failure review accepted the 512-row/65-node-per-path maximum Python
fixture in 0.893 seconds and rejected malformed transit rows without leaking
arbitrary text. Both traced native API arguments against the public UI and
confirmed that preview segments do not enter fare or itinerary leg accounting.
No review edits, compilation, cleanup actions or live operations occurred.

Post-review Jev advisory supported landing source/offline only:
`jev-1.14.0`, request
`af756c91417063a956191fecda3f8cbbc9209bd8c23e267f98a2b8b9467f0384`,
777 input / 57 output tokens, 0.395 seconds. Deterministic tests and independent
source/caller review establish the scoped evidence; the advisory is not
native qualification or proof of full coverage.
