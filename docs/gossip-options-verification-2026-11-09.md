# Visible gossip options — 2026-11-09

Baseline e9eeea1. Previous goal turn made progress with completed-candidate
partial rankings. Inspection of their bounded implementation found no new
concrete issue. Current work adds a previously missing passive interaction
system: non-quest choices in an already visible gossip panel. Broad goal active;
full owned BiS, fresh active identity and native qualification remain incomplete.
No account data, game access, installation or deployment used.

## Primary API evidence

Gethe Blizzard UI source pin9dc84f0d3bbae1768ac3405b3716000bd93eda06,
Interface/AddOns/Blizzard_APIDocumentationGenerated/GossipInfoDocumentation.lua,
read directly from its public raw source. SHA-256:
22cd6a48eded91316753af2e40624e542a91c728c713636549b7d3a6ef7916a5.
GetOptions returns GossipOptionUIInfo rows with required name and nullable
option/spell IDs. Only a small raw field projection is captured. This generated
metadata does not prove availability in the user's Forever client or approve
the bridge. No selection, refresh/close, entered text or reward APIs are used.

## Direct author checks

```powershell
cargo test --locked --offline --test gossip_options_capture
cargo test --locked --offline --test addon forever_bank_export_distinguishes_observed_empty_and_unavailable
cargo build --locked --offline
$env:PYTHONPATH='.;tests'
$env:TEMP='C:/Temp'
$env:TMP='C:/Temp'
python -m unittest test_gossip_options test_policy_evidence
```

Three Lua5.1 capture tests passed (compile26.03s, runtime.01s). Core export fixture
first passed in.30s after1.92s compile. Ordinary MCP build12.04s. Expanded full
Core fixture explicitly captured/decoded/read the new section and confirmed
hidden-panel failure isolation:1 passed/1 filtered, .39s after3.30s compile.
Ordinary MCP rebuilt3.02s. Python actual-MCP and existing policy tests:2 passed,
.744s. Synthetic required-secret access and forbidden-action counters stay0.
Cases include nullable IDs, raw metadata, optional secrets, namespace/frame/
getter replacements, hidden/secret visibility, row changes/reorder, sparse lists,
known empty and128/129 rows, 512/513-byte names. Python/Rust parity rejects17
correctly hashed malformed captures (type/range/sparse/extra input text/family/
UTF-8 byte boundaries). Actual MCP proves raw inspection, summary counts/false
flags, missing versus empty and foreign owner rejection; stored bytes unchanged.

## Package evidence

```powershell
python -m tools.package_addon --family forever --variant live --interface 16001 --version 0.52.0-dev --output dist/forever-gossip-options-0.52.0-dev
cargo build --locked --offline --example verify_addon
python -m tools.verify_addon_packages --directory dist/forever-gossip-options-0.52.0-dev --verifier target/debug/examples/verify_addon.exe
```

The first invocation with an older prebuilt example correctly refused the new
unavailable section (Invalid unavailable section). Rebuilding that verifier with
the updated reader,1.95s, passed exact source rendering/checksums, generated
adapter, Lua5.1 roundtrip, Forever capture and changed-interface rejection.
This demonstrates why old readers must not accompany the new addon.
Package remains uninstalled:
dist/forever-gossip-options-0.52.0-dev/AzerothOperator-0.52.0-dev-forever-live.zip.
ArchiveSHA2569ba3db01408b8c847363c68aeb132834e19bde26c9f886d4d87fccc8e376d444.
Packaged LF CollectorsSHA256f122189f07519558d08a4fea96bc9a2966615ccd6fdc35e1dcde6b47fb0c44b8.
Packaged LF CoreSHA2560be2cdab8a33db1d1f83595ed2a87b2f655e1cf693226e05ade929b449f62157.
No full-suite, CI or live test was run.

## Advisory plan

Jev resolved jev-1.14.0, request
d497659d8b88e349fa9d429872cb5f670a496349fbf7d5033671846f526afd32:
proceed.90/revise.09/insufficient.01, confidence.85;586input/47output,.367s.
Advisory only; source and independent review decide integration.

## Adversarial finding and minimal correction

Both independent read-only reviewers (Callers and Failure/privacy, assigned
gpt-6-astra high; runtime IDs/usage unavailable) reproduced the same concrete
parity fault: present reported_order_index:null was accepted by Rust while Python
rejected it. Generic optional-field validation skips null and the new explicit
unsigned checks omitted this signed field. Author added the null case first:
actual-MCP test failed with Rust isError=false (one test,.317s). Added only a
local present signed-i32 check. Optional omission remains valid and no shared
validator or accepted bound was relaxed. Rebuilt ordinary MCP5.83s; expanded
actual-MCP/policy tests2passed,.583s. The malformed parity corpus now has18 cases.
Rebuilt package verifier5.45s and reverified the unchanged archive successfully.
Ordinary MCP SHA-256:
5750f667bb206788658ea2409ee6b1f5e76f7bab7adbdba9494e82644b803d2e.

Round1 also passed all3 primary Lua5.1 tests and scoped Core export fixture.
Failure/privacy added75 supplemental synthetic Lua5.3 cases covering optional
invalid/secret omissions and transitions, malformed labels/slots, ignored-field
access traps, hiding after fetch, replacing getters and wrong family. Restricted,
forbidden-field and action access counters stayed0. Lua5.3 supplements, rather
than replaces, the primary Lua5.1 fixtures. No further concrete fault found.

Finding triage Jev resolved jev-1.14.0, request
866d66b2d805261ba4861add618c7232a673cf4caeea1f14be438d8e0db8daa9:
fix.98/insufficient.01/not-material.01, confidence.97;443input/45output,.312s.
Advisory only; independent reproduction and author red/green establish the fault.

## Clean correction round

Callers replayed the original null failure plus omission, signed minimum/maximum,
zero, underflow/overflow, Boolean/float/string:10 cases agree across Python and
actual rebuilt MCP, with exact accepted values and no private-string echo.
Failure/privacy checked76 independent domain/Python/Rust-MCP cases: omission
plus each of five optional fields with15 null/type/range/private boundary inputs.
All agree; bytes unchanged. The expanded actual-MCP test passed,.779s. Original
null order now receives fixed Snapshot unavailable or invalid. No new fault.
Both reviewers inspected the bounded documentation and confirmed the intended
strict/passive/source-only contract, including old-reader incompatibility.
No builds/source writes/Git changes/network/game/deployment by reviewers. Live
drift was omitted because installation/native testing remain deferred.
Final format and whitespace checks passed. Broad objective remains active.

Final advisory Jev resolved jev-1.14.0, request
650569e2efc97c57211a9a14608a306b5aed1f89d7d7fe422a9d939ad46f1bce:
integrate.96/revise.02/insufficient.02, confidence.94;622input/52output,.388s.
Author accepts source integration on scoped fixtures, actual caller checks and
clean correction review. This grants no Blizzard authorisation or native proof.
