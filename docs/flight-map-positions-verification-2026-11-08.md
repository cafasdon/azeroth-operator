# Flight-map positions — 8 November 2026

Source and offline verification only. No installation, deployment, game
interaction or native capture. Broader in-game coverage remains unfinished.

The public [Forever taxi API documentation](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_APIDocumentationGenerated/TaxiMapDocumentation.lua)
defines `TaxiNodeInfo.position` as a vector. The
[flight-map provider](https://github.com/Gethe/wow-ui-source/blob/forever/Interface/AddOns/Blizzard_FlightMap/FM_FlightPathDataProvider.lua)
uses `position:GetXY()` to place node pins. This is UI map-relative data for
the open flight map at the current flight master, not a complete world graph.

The manual collector optionally exports `map_position: {x,y}` after two
consistent readable coordinate observations. Both values must be numeric in
the inclusive range zero to one. Restricted values are screened before
comparison, and native vector objects are never serialised. Missing APIs or
vectors, errors, restrictions, invalid ranges and changing positions leave
the field absent while preserving valid node/fare data and other sections.
No additional game interaction or movement API is introduced.

Both readers require exactly two non-null numeric coordinates with the same
range. Boolean, string, partial and extra-key objects are rejected. Optional
route annotations `origin_map_position` and `destination_map_position` use
the existing leg's `map_id`; different maps have different coordinate frames.
Legacy routes gain no null keys. Fare totals, route selection and exclusions
of transition nodes are unchanged. Positions establish neither physical
distance, travel duration nor movement instructions.

Author verification:

```powershell
cargo build --locked --offline
cargo test --locked --offline --test flight_map_positions --test addon
$env:PYTHONPATH='.;tests'
python -m unittest test_flight_map_positions test_trainer_profession_slots
cargo fmt --check
git diff --check
```

Build passed; three Rust tests and two real MCP Python tests passed. Cases
exercise actual Lua export and Rust import, coordinate endpoints, missing,
restricted, throwing, changing and non-finite reads, section preservation,
both schema readers, malformed stored snapshots, legacy route equivalence,
cross-map itinerary annotations, unchanged total fares and transition-node
exclusion. The transition test initially expected an invented status; source
inspection confirmed the existing `no_fully_priced_route` result and the test
was corrected without changing product behaviour.

Development package, not installed:
`local-data/flight-position-qualification/AzerothOperator-0.0.0-flight-position-test-forever-live.zip`.
SHA-256: `e5fd322d405c2015e325921b30d7f0f193239c97f3dd777d49ac3fc0d9b9e739`.

Independent failure/security review (`gpt-6-astra`, high effort) found no
concrete flaw. It re-ran the real MCP integration (one passed), rejected 22
additional malformed Python-validator axis cases and accepted three boundary
positions plus legacy omission. Source inspection covered protected-value
ordering, two-read stability, map end guards and conditional route copying.
No live checks, edits, compilation or cleanup actions were performed.

Independent callers review (`gpt-6-astra`, high effort) also found no concrete
defect. Its real MCP integration passed (one test), and a separate
`list_snapshots` probe with `family: forever` and destination node 20 returned
two captures: a destination-only annotation was conditional, the missing
origin annotation stayed absent and legacy route/fare fields matched. It
also rejected NaN, both infinities and Booleans on the Python reader's y axis.
The initial probe omitted required family and was corrected as a harness
error. Review traced all three route-helper callers. Both reviews used the
existing debug binary; the author built that binary from current source and
ran the Lua/Rust tests separately. No native compatibility is inferred.

Jev's advisory assessment supported scoped offline semantics, explicitly
pending callers review and without a native/full-coverage claim:
`jev-1.14.0`, request
`82a6ca0dcbc5a9098991da39c70043ea2dca875fa2442a74f2b920ef07d9fb53`,
616 input / 47 output tokens, 0.292 seconds. Deterministic tests and source
review, rather than advisory confidence, establish the recorded evidence.
