# Public build experiment fixtures

`public-hunter-256.json` is a public level-60 hunter model case, not a saved
character capture. Its `public-model-fixture` variant is explicit. The current
character is unbound; its snapshot hash identifies public input, not a player.

The case follows `manifest_from_validated_pools` in
[`tests/benchmark_joint_build_windows.py`](../../../tests/benchmark_joint_build_windows.py).
It combines 128 generated gear variants with two generated talent allocations.
The gear pool keeps or removes each of seven armour slots. The talent pool uses
the two accepted allocations from the benchmark's explicit rank domains.
The base request comes from `fixture(20261115, 2)` in
[`tests/benchmark_joint_build_optimizer.py`](../../../tests/benchmark_joint_build_optimizer.py):
a fixed 45-second encounter, seed 20261115 and two trials per native request.
These trial counts test execution and accounting, not statistical superiority.

The underlying public hunter profiles are checked in under
`tests/fixtures/forever-hunter-{bm,mm}.json`. They derive from
[WoWSims Forever](https://github.com/wowsims/forever) revision
`9dc32ac8ca0d8d1b76337861d38b93c0f884af11`.
The [catalogue lock](../catalogue.lock.json) and
[schema provenance](../sim-request-descriptor.json) record the source pin.
The upstream [MIT licence](../third-party-WoWSims-LICENSE.txt) credits the
WoWSims team. The generated matrix and positional layout are project adaptations.

The fixture's LF-byte SHA-256 is
`99a6f97159121160cea7a1a119c34336432f11aea540cef736790bc6b24552d4`.
`.gitattributes` preserves LF line endings. See the
[activation record](../../../docs/mcp-build-activation-2026-11-15.md) for its
configured caller and the remaining qualification limits.
