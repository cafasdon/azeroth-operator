# Passive bag-item cooldowns — 9 November 2026

## Intended result

Expose historical public carried-item cooldown data without activating items,
inferring current usability or changing existing item records. Every captured
bag item retains either readable enabled/timer data or explicit unavailable
status. Both readers bind the section to exact source bag/slot/item IDs and
validate counts, status shape and bounds. Older captures remain valid.

Primary API contract: [pinned generated Container documentation](https://raw.githubusercontent.com/Gethe/wow-ui-source/9dc84f0d3bbae1768ac3405b3716000bd93eda06/Interface/AddOns/Blizzard_APIDocumentationGenerated/ContainerDocumentation.lua),
revision `9dc84f0d3bbae1768ac3405b3716000bd93eda06`, inspected 9 November 2026.
It declares the container-specific getter and numeric start/duration/enable
returns. No provider data is bundled and no external call occurs at capture.
See ADR 048 for the exact data and failure contract.

## Author checks

- `cargo fmt`: passed.
- `cargo test --locked --offline --test bag_item_cooldowns`: three Lua 5.1
  actual Core/export/decode tests passed, .37s after 13.21s compilation. Fixtures
  cover stable positive/zero/disabled timers; 44 unreadable/change/secret/error
  modes; original item/link/count identity; missing API; combat/flavour/scope;
  direct budget checks before reads and forbidden-action counters.
- `cargo build --locked --offline`: passed, 8.56s. Ordinary MCP rebuilt before
  actual stdio tests; no concurrent build/test binary replacement.
- `cargo build --locked --offline --example verify_addon`: passed, 1.56s.
- `PYTHONPATH=.;tests python -m unittest test_bag_item_cooldowns`: one actual
  Rust stdio MCP test passed, .505s. It checks inspection, exact summary counts,
  24 malformed captures rejected by Python and Rust, zero/4096-item boundaries,
  missing-section legacy behaviour and unchanged original snapshot bytes.

All observations are synthetic, in temporary owner-local stores. No game
process, real character, account data, item action, installation or deployment.

## Uninstalled package

Capture-only Forever/live/interface 16001, version 0.46.0-dev:

`dist/forever-bag-item-cooldowns-0.46.0-dev/AzerothOperator-0.46.0-dev-forever-live.zip`

SHA-256 `dda9256f46dee7e36beb6cdd0b246c21d33b73e1d29a58bfac0441bb142ea936`.

`python -m tools.verify_addon_packages --directory dist/forever-bag-item-cooldowns-0.46.0-dev --verifier target/debug/examples/verify_addon.exe`
passed source/rendered manifest verification and packaged Lua 5.1 roundtrip.
Forever capture fixture=true; changed-interface rejection=true;
live_qualified=false. No new action modules in the four-file capture package.
Use module invocation for the verifier so its package imports resolve.

## Independent review

One clean round, two gpt-6-astra high lenses: Callers and Failure/privacy.
Both ran the three prebuilt Rust tests (.42s/.41s) and actual MCP Python test
(.424s/.499s), all passed. Callers additionally replaced six nested object
positions with null/Boolean/number/string/array values: 30 malformed captures
rejected by both readers; the MCP stayed usable and listing retained only the
valid capture. Failure traced protected property lookup, public-value checks
before arithmetic, stable timer/context/identity, source linkage and counters.
No concrete faults found. Native/deployment review omitted because those gates
remain deferred. Agent token usage unavailable.

## Advisory plan

Jev 1.14.0 request
`59befaf8e14ec31f6da777752688284d5505d86526a508d162b5a47ee0811de9`:
proceed .70 / revise .21 / insufficient .09; 650 input, 47 output, .336s.
Advisory only; actual code/tests/package verification remain authoritative.

Final Jev 1.14.0 advisory request
`e2bab4740c8da6a1d526b68f28f7a845d9bc413c6f731a8c74e29c799dfcc648`:
land .56 / fix .04 / insufficient .40; 689 input, 47 output, .281s.
The advisory remains uncertain and is recorded unchanged. Author disposition:
land source/offline capture only on direct tests, two clean independent reviews
and package verification. Native behaviour is explicitly unverified and the
package remains uninstalled; no broad compliance or completion claim is made.
