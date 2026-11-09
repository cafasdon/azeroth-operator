# ADR-051: Local simulator talent observation checks

Date: 2026-11-09. Status: accepted for offline source integration.

## Problem

An explicit simulation template can describe different purchased talents from
the saved character. Active ranks may include boosts and must not be substituted
for points spent. ADR-050 provides optional native definition/rank observations.

## Decision

Bundle `clients/forever/talent-map.json`, derived from WoWSims Forever revision
`9dc32ac8ca0d8d1b76337861d38b93c0f884af11`. The source ZIP SHA-256 is
`e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712`.
The bundled map SHA-256 is
`33dc07ad7ba838a4bb6f95d825df44235b0e1a00e92eb3dfdb9c68d8d96c18b3`.
It contains nine classes and 467 ordered definition IDs with rank limits.
No runtime network lookup is required.

`python -m tools.build_forever_talent_map --source-zip <local-pinned-zip> --check clients/forever/talent-map.json`
reproduces the map without extracting or executing archive contents or writing
the map. It cross-checks JSON definition IDs and tree order with proto field
numbers, names, spell IDs and maximum ranks, and the Go tree-size offsets.

Upstream Priest declares 18 entries for its third tree, while JSON/proto contain
17. `FillTalentsProto` uses only the first two tree sizes to locate three-tree
fields. Preserve both source sizes and actual counts; require the first two to
agree, and never permit an extra template digit past the last actual field.
The generator initially refused this mismatch; source inspection established
the offset rule before narrowing that condition. This is not a native-client fix.

Equipment reviews project optional Forever `captured_talent_observation` using
only config ID, nodes, tree IDs/count and optional staged flag. Native config
names, saved loadouts and import text are excluded. Legacy/other-client reviews
report null for this projection.

The adapter validates the projection, verifies the bundled checksum and parses
up to three bounded ASCII digit trees. Missing trailing digits mean zero;
one-point talents and over-limit digits use the pinned engine's effective rank.
Duplicate protobuf talent-string aliases, overlong trees, invalid characters,
partial rank tuples and duplicate observed definition IDs are rejected.

For definitions in the pinned class whose observed native maximum agrees,
compare **purchased_rank**, never active rank. With a captured false staged flag,
known differences reject simulation. A true staged flag also rejects simulation.
Ranker `--plan-only` and saved-window collection can retain these diagnostics.
Direct simulation and ranker results include bounded numeric observation reports.
Saved-window preparation calls the same request adapter against fresh MCP reviews.

Missing observations, unknown definition IDs, omitted optional tuples, native
maximum disagreement and unknown staged state remain unverified. A matching
partial report is named `observed_ranks_match`; it is not whole-allocation proof.
Reports keep allocation_match_verified, native_tree_coverage_verified and
simulation_model_verified false. Unknown state is not silently asserted committed.

## Limits and next work

No automatic talent-string generation, point spending, tree legality check,
spec choice, in-game execution or external personal-data transmission. The map
establishes pinned definition/position correspondence, not live server parity or
mechanical accuracy. Complete native tree coverage, unknown/staged resolution
and full simulator build identity remain open. Live testing remains deferred.

Preparation binds the talent checker source and pinned map bytes in its
unchanged-input checks; prepared-window merging requires matching hashes.
Legacy prepared artefacts require re-preparation, while saved journals remain
readable. Collection also binds the checker source. The checksummed public map
has a scoped LF Git attribute so Windows checkout conversion cannot invalidate
its checksum.
