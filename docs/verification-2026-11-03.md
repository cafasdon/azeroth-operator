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
