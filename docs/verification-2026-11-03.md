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

Follow-up: final independent Python suite rerun against rebuilt Rust passed all nine tests before commit 58ffcae was pushed.

Nine Rust tests pass, including unavailable-section comparison and malformed-bag unknown counts. Clippy passes with warnings denied. Prior independent Python MCP/packaging suite passed nine tests; final analysis fixes were covered by Rust tests.

Warm-cache synthetic benchmark: 200 snapshots, 180 item entries each, 20 list runs and 200 inspect runs. Baseline list median/p95 96.8472/105.8761 ms; metadata-retention candidate 136.6885/144.3792 ms. Inspect also worsened from 0.296/0.5962 to 0.6698/1.0371 ms despite no inspect change. These unpaired runs do not establish a speedup or isolate a cause. Retention change avoids holding all section payloads while sorting; memory use was not measured. Paired performance qualification remains pending.

Jev advisory previously returned insufficient evidence (0.63), claim_supported 0.18; jev-1.14.0, request db9dbeed335c0d4fa25d313bd7ab3643ad7cfec255c8024a92c795ad1939e0c0, tokens 894/135, 0.488s. Not a pass. Required adversarial-review file was unavailable. Direct checks found and fixed omitted unavailable sections and misleading counts for malformed bags. Full nested schemas and live qualification remain open.

## Inventory validation and gateway discovery

Equipment and bag maps now reject array shapes, noncanonical/out-of-range indices, zero item IDs, missing/negative stack counts, oversized links and unknown item fields. Empty object means observed empty; missing remains unknown. Legacy array fixtures were migrated explicitly; existing such snapshots will be rejected rather than silently reinterpreted. Other nested sections remain pending. Ten Rust tests and nine Python tests pass, including the unchanged Lua producer roundtrip and actual MCP caller. Clippy passes.

Read-only SSH on 2026-11-03 verified host pylon, CT290 running with hostname mcp-gateway, and mcp-router active. Listener inventory collected without credentials. Existing source supports stdio-backed scope workers. No remote source, service, firewall, route or credentials were changed. Remote hosting and authenticated ingestion remain implementation work.

## Complete current section-shape validation

All eleven emitted section shapes now reject unsupported root shapes and unknown fields. Talents and currency entry maps are recursively checked; required fields, scalar types, coverage markers and string/entry bounds are enforced. Optional null is accepted. This validates the current wire contract, not complete semantic legality or actual game API compatibility. Runtime API differences may still require collector/schema changes together.

The Lua producer fixture imports successfully. Twenty-two root mutations (array and unknown field per section) and six nested numeric type mutations are rejected by the real decoder. Ten Rust tests, nine Python tests and warnings-denied Clippy pass. The Python MCP test invokes the rebuilt Rust binary.

Jev advisory: jev-1.14.0; request 33c47c319add1a9cfb727295f61a9d0145eb6d56f9452cfe6e0b362704a6f5a0; 899/135 tokens; 0.524s elapsed. Returned insufficient_evidence 0.49, claim_supported 0.62. Not a pass or live qualification. Disposition: commit directly tested contract checks; keep broader delivery and live verification open.

## Linux container qualification

Built on personal pylon CT290 with digest-pinned official Rust 1.98.1 and Debian bookworm bases. Ten Rust/Lua/CLI tests passed inside Linux before release compilation. Image ID sha256:d5d5135b1a8ffec089b58fd09030afe7f5b0b3f3167966dee37a6ecf97ba2501, size 80,666,033 bytes, configured user 10006:10006. Build archive SHA256 5c432ab2a28939826c1685be5214b619a00485a0892908a5f55f89f4d28d34bc; includes fc88b88 source plus the committed Docker packaging files.

Independent Python MCP client passed against the Windows executable and the remote container over SSH/lxc-attach. Remote run used --network none --read-only --cap-drop ALL --security-opt no-new-privileges. Exact five-tool discovery, read-only annotations, capabilities, empty listings for three families and traversal denial passed. Two initial remote launch attempts closed at initialise; a raw initialise probe succeeded, and the final runner preserving the local process environment and using one quoted remote command passed. Do not interpret those initial failures as a proven server defect or claim a single isolated cause.

No gateway profile, public route, service or credentials changed. The qualification image/build cache and source staging are on CT290; no persistent runtime container was launched. Public TLS/authentication, populated-store mount and Windows upload path remain unqualified. No Retail SavedVariables export was found at the known installation path during this increment.

## Bounded stdin ingestion primitive

Added import-stdin using the same bounded reader and schema validation as file import. The real CLI test sends a valid snapshot twice, observes identical receipts and exactly one stored file, then rejects empty/malformed/code-bearing and oversized inputs without a success receipt or extra stored files. Eleven Rust tests and warnings-denied Clippy pass. This is a local transport primitive only: no new public endpoint, authentication or per-owner routing is claimed. Slow sender deadlines belong to the transport wrapper and remain required for remote deployment.
