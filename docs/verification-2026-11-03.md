# Foundation verification — 2026-11-03

Eight unittest cases passed, including an actual SDK stdio client/server session. Tests verify tool discovery, snapshot inspection, invalid ID rejection, hash tampering, explicit identity boundaries, freshness metadata and basic ingestion rejection. They do not prove live game compatibility or complete schema validation.

Review found non-atomic publication; replaced direct writes with same-directory temporary file, flush/fsync and atomic replacement. Concurrent publication still requires a dedicated regression test.

Jev advisory: resolved jev-1.14.0; request eab70f0a806d20d38bfe60cbd01399a0ef5b6e8b9de52f1cfcc6b7682f5fe406; input/output tokens 867/131; elapsed 0.434s. Evidence sufficiency 0.39, material issue 0.21, full completion unsupported 0.93. Disposition: foundation increment only; no completion/release claim. Initial review request had an invalid focus enum and was corrected.

Required adversarial-review file was absent at the supplied path. Performance skill path was unavailable. No benchmark or optimisation claim made.

## Rust port

Three Rust tests passed, including eight-thread concurrent publication. `cargo clippy --all-targets -- -D warnings` passed. Eight Python tests passed with `AZEROTH_TEST_BINARY` targeting the compiled Rust executable; the independent MCP client verified discovery, snapshot reading and error semantics.

Jev claims advisory: jev-1.14.0; request 8ab54f316d923e489a77fc96cfe166d2631d35eb2cb7696331629939fa2b1cc0; tokens 863/131; elapsed 0.36s. Limited foundation claim supported 0.85; performance/full completion unsupported concern 0.85. Disposition: commit verified port; leave overall goal active. This is advisory, not policy approval or independent source review.

## Export/import increment

Six Rust tests pass: four library tests, Lua 5.1 fixture export/import (including escaped strings and combat refusal preserving prior export), and an actual CLI subprocess importing a file into the store, family discovery and code-bearing input rejection. Eight Python tests pass, including Rust MCP discovery and family filtering. Clippy passes with warnings denied.

Jev advisory requested before the final CLI test: jev-1.14.0; hash b7a8409b65724527c521db5fed958b365a2029774334e17a87cad554400ffd8b; tokens 909/135; elapsed 0.431s. Returned insufficient evidence (0.65), claim supported 0.29. This is not recorded as a pass. Added the real CLI test and verified it directly. Evidence remains fixture-only; addon not packaged, installed or live-qualified. No speed claim. Listing currently scans payloads (bounded 10,000 files); benchmark and indexing remain open.

## Collectors and development packaging

Six Rust tests and nine Python tests pass; Clippy clean. Lua fixture asserts all eleven section values/coverage, missing-API reasons, combat and interface mismatch preservation. Packaging test checks deterministic bytes, four-file allowlist, TOC references and separate family archives. Independent Python MCP test targets Rust.

Retail development archive SHA256 2b4b05512cb56723a40403b5d3bb835a8f97391d7ef9c311cdf8d46cfe1dcbd6 installed while game was closed into previously absent addon directory. All four installed hashes match manifest. Forever beta archive built, not installed. No Classic client found in the named installation. No live game export yet.

Jev advisory jev-1.14.0, hash ff6d47574e23a99d8a683de4fc765c07b725943e61837db6cba882083b2708af, tokens 900/135, 0.456s: insufficient evidence, live/project completion unproven 0.96. Not a pass. Disposition: development artifact only, retain outstanding live tests and performance work. Local packager is a deterministic staging helper; release publishing integration is still pending.

## Snapshot analysis increment

Nine Rust tests pass, including unavailable-section comparison and malformed-bag unknown counts. Clippy passes with warnings denied. Prior independent Python MCP/packaging suite passed nine tests; final analysis fixes were covered by Rust tests.

Warm-cache synthetic benchmark: 200 snapshots, 180 item entries each, 20 list runs and 200 inspect runs. Baseline list median/p95 96.8472/105.8761 ms; metadata-retention candidate 136.6885/144.3792 ms. Inspect also worsened from 0.296/0.5962 to 0.6698/1.0371 ms despite no inspect change. These unpaired runs do not establish a speedup or isolate a cause. Retention change avoids holding all section payloads while sorting; memory use was not measured. Paired performance qualification remains pending.

Jev advisory previously returned insufficient evidence (0.63), claim_supported 0.18; jev-1.14.0, request db9dbeed335c0d4fa25d313bd7ab3643ad7cfec255c8024a92c795ad1939e0c0, tokens 894/135, 0.488s. Not a pass. Required adversarial-review file was unavailable. Direct checks found and fixed omitted unavailable sections and misleading counts for malformed bags. Full nested schemas and live qualification remain open.
