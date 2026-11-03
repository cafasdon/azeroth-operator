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

## Dedicated gateway profile

The source generator tools/gateway_profile.py creates only the standalone azeroth-operator profile: immutable image ID, five read tools, one fixed Rodrigo store mounted read-only. It does not export or modify homelab. The actual Docker MCP importer on CT290 rejected an absent version and version 2; version 1 was accepted. This illustrates why source fixtures are not deployment proof. Imported candidate pins the previously qualified c13e769 image, not the newer stdin-import source. No gateway process or public route was started. Store mounting/authentication are still unqualified.

Eleven Python tests pass, including immutable-reference rejection and profile allowlist/mount assertions. Infrastructure integration worktree is T:/Projects/Proxmox/.worktrees/azeroth-mcp, branch feat/azeroth-mcp from origin/main 1b01f436; unrelated active checkout preserved. No infrastructure source changes made there yet.

## Gateway launch qualification and loopback candidate

The imported profile was exercised through actual Docker MCP stdio. Initial launch refused the bind mount as outside allowed roots. Retest with MCP_GATEWAY_DOCKER_BIND_ALLOWED_PATHS set to the exact owner snapshot directory passed exact tool discovery, capabilities, three family listings and traversal denial. The mount remained read-only. No aggregate catalogue changes.

Added a loopback-only systemd candidate, dedicated 256-bit token file validation and first-install-only installer. Authentication unit tests reject missing, wrong, malformed and extra token configuration. Candidate is not yet installed or authenticated-HTTP-qualified. It does not enable the service on boot or publish a route. Existing installations are refused to avoid overwriting state.

## Authenticated loopback service deployed

Installed committed 4613fa4 service/checker on personal pylon CT290 using first-install-only source. Dedicated token was generated remotely into a root-owned mode-0600 file; no token was printed or copied locally. Authenticated HTTP probe verified missing token 401, wrong token 401, correct-token initialise, exact five-tool discovery and a successful empty Retail listing. Service active and enabled; listener remains 127.0.0.1:8823. Installed unit SHA256 106e14240e363b0f2c63e66cb5d31c7a7700c78d78df3f632bdb5c54880c18b7 and checker 4c0c6748c829783c7ec90c932c15fe21fbf958a36e4cf54c8d03d7a12d737317 match local committed files. No reboot test performed.

Source review found CT150 edge isolation covers ports 8765-8821, excluding new 8823. Before publishing, extend that isolation and qualify denial of relayed/host-originated connections, plus guest firewall proxy-only ingress and public TLS bearer checks. No proxy or firewall edits were performed. Mounted populated-data readback and current-source image refresh remain pending; running backend image remains the previously qualified c13e769 build.

## Public route verified

Infrastructure publication ea3d294b (cafasdon/Proxmox PR197) exposed the dedicated
scope at https://mcp.cafas.com.br/scopes/azeroth/mcp after extending CT150 isolation
and guest proxy-only ingress to port 8823. Actual public TLS probe from CT290
verified missing/wrong-token 401, authenticated initialise, exact five-tool
catalogue and empty Retail listing. No token copied to this workstation.
Direct port attempts from pylon and CT150 host failed; CT150 reject counter
increased. Explicit untrusted bridge and populated store probes remain pending.
The profile still pins the c13e769 image; newer stdin ingestion is not deployed.

## Local client and populated hosted readback

Configured Codex MCP entry azeroth with the public URL and bearer_token_env_var=AZEROTH_MCP_TOKEN. Dedicated token transferred without displaying it and stored in the Windows user environment. It is not in source or command arguments. Existing desktop processes may require restart to inherit it; current-chat tool discovery is not claimed. Parsed configuration verified URL/env reference.

Independent Python SDK on ICARUS successfully initialised the public TLS MCP. A synthetic GatewayFixture/Synthetic snapshot (source=fixture) was imported through the existing image CLI into the owner's store, hash ef93cc06b211e8e3c728f4d927ddada4133b6b78aae4ebe050a9a6968fa3006c. The local SDK read equipment item 123 and one equipped slot through public MCP. Cleanup checked the exact file hash and fixture identity before removing it; subsequent public probe verified the empty store and auth checks again. This is mounted-data readback, not live WoW evidence or an automated upload pipeline.

PR197 checks: seven reported passing, gateway-scripts and Rust-router jobs still pending at this checkpoint. No CI success for the full suite claimed.

## Upload channel implementation candidate

Added Rust upload CLI with local validation, canonical JSON, fixed SSH alias and strict host-key/batch options. It accepts only the exact content-hash receipt and successful process exit; timeout/error remains unknown outcome. Duplex tests verify payload/receipt exchange, mismatch refusal and cancellation of a stalled peer. Thirteen Rust tests, fourteen Python tests and locked Clippy pass. Cargo.lock gained signal-hook-registry 1.4.8 for Tokio process support.

Receiver candidate rejects remote commands/TTY, pins CT290/store/image, bounds stdin and process execution, and cleans only its uniquely named import container. Unit tests verify fixed argv and reject mutable/injected image/container identifiers. These are source tests, not forced-key qualification. No key provisioned, no upload alias configured, no receiver installed, no updated image deployed. ADR004 records the required activation tests.

PR199 remains open with gateway scripts and Rust router checks pending; other checks passed at this checkpoint.

## Upload identity isolation and build cache candidate

Rust upload now requires an absolute AZEROTH_UPLOAD_SSH_CONFIG and invokes ssh -F, preventing normal administrator identities from becoming fallbacks. Generated a dedicated Ed25519 key in the ignored ACL-restricted _secrets directory and a dedicated config under the Codex state directory. ssh -G verifies exactly one IdentityFile, IdentityAgent none, IdentitiesOnly yes, strict host checking and forwarding cleared. Before registration, an actual authentication attempt was refused (publickey), confirming no broad identity fallback. The public key has not yet been authorised remotely.

Thirteen Rust tests and Clippy pass. Receiver tests now explicitly reject SSH commands and TTY before host access. First-time receiver provisioning source validates root-owned authorised_keys, appends only a restricted forced-command key under flock, and refuses existing receiver directories. Runtime grant and prohibited-operation probes remain pending.

Jev advisory jev-1.14.0, request 12d8d63769a8c502cff7c0a0546f39d1ae8492af24d347e31a717aa5a8e5d6cc, tokens984/135, elapsed1.113s: insufficient evidence0.74, live qualification concern0.92. Not a pass. Direct key-isolation evidence above narrows but does not complete qualification.

Docker build now uses persistent build-stage Cargo caches and a single release profile for tests/build. First Linux build is in progress; no speedup measured or claimed. The running public reader is unchanged.

PR199's first full CI passed, but concurrent PR198 also fixed the same static assertions. Merged origin/main into the isolated branch, retaining the extra 8824 behavioural check, and reran static tests successfully. Exact merged-head CI is rerunning before merge.

## Receiver location correction and Linux image

The Linux candidate built successfully with thirteen release-mode tests; image sha256:800b73ab19f052f983022bc479dc8e666fe45c0b6448d17ff728849e88d5831c. Build source was 779a2a9 plus the cached Dockerfile; the later dedicated-config client hardening is in the Windows client, not this image. Receiver uses only import-stdin. Build compile/test step took 290.2 seconds with an empty Cargo cache; no performance improvement claimed.

Inspection found pylon root authorised_keys is a cluster-shared symlink. Receiver provisioning now targets personal CT290 directly, where no root authorised_keys existed; no key was added to pylon or cluster shared storage. Fixed receiver command invokes local Docker, checks hostname mcp-gateway and preserves the same fixed owner store. Updated receiver tests pass. Provisioning and end-to-end checks still pending at this commit.

## Restricted upload channel qualified

Provisioned source d3068a5 inside personal CT290, not the cluster-shared pylon key file. The new public key has restrict plus the fixed timeout/receiver command. Private key remains in the local ignored ACL-restricted directory. Dedicated SSH config pins CT290's host key obtained through the existing trusted pylon administrative channel; normal identities/agent are excluded.

Actual tests: command `id` was refused by the authenticated receiver; a remote forwarding request was denied by SSH; Windows Rust upload sent GatewayFixture and received the exact expected hash; an identical retry returned that same hash. The remote store contained exactly one snapshot. Independent local SDK read the equipment and summary through the public TLS MCP. Exact-hash cleanup removed the fixture and the public auth/empty-store probe passed again.

Importer image is sha256:800b73ab19f052f983022bc479dc8e666fe45c0b6448d17ff728849e88d5831c (779a2a9 plus cached Docker build recipe). Reader image remains c13e769-derived d5d5135b..., unchanged and compatible with the uploaded schema. This qualifies explicit CLI upload, not a watcher/scheduled sync or a live WoW export. Agent/X11 forwarding restrictions are configured by restrict but were not separately exercised.

PR199 merged-head gateway CI passed; Rust router CI still running at this checkpoint. Do not claim all checks passed for the merged head yet.

## Incremental runner and scheduled deployment (19:20Z)

Source b9dcf5d adds bounded explicit-root discovery, locking, atomic status, timeout tree cleanup and a current-user scheduled-task installer. Rust sync source7470ffe passed15 Rust tests and Clippy; full Python suite now21 passes. PowerShell installer parsed and ran successfully. Windows timeout fixture proved both the spawned parent and its child stopped. The prescribed OCR skill and adversarial-review rule file were unavailable; direct review covered tools/sync_exports.py, tests/test_sync_exports.py and deploy/install_sync_task.ps1. One test initially misplaced duplicate-family assertions; corrected before the passing run.

Actual runner uploaded the known GatewayFixture through the restricted SSH channel; independent SDK public MCP readback matched equipment/summary. First pass5.30s, acknowledged unchanged pass0.21s (one observation each, not a statistical benchmark). Exact-hash cleanup removed the synthetic remote object; subsequent public discovery passed and remote snapshot listing was empty.

Installed task AzerothOperator-ExportSync, current user Interactive/Limited, pythonw.exe, five-minute interval plus logon, IgnoreNew, ten-minute execution limit, WakeToRun=false. Stable Rust binary SHA256 104c544079fa7ab991e52f76ea3780f2d774b1ad92f5203439564bd417852701; deployed runner SHA256 e27eb99175a4e9647d9ad8d63b938517ecc64711db4b90ddfcd823e0334bb32d. Copies live under ignored local-data; task does not run source/build outputs directly. First scheduled run19:19:52Z completed result0; atomic status19:19:53Z reports waiting_for_export for Retail and Forever, live_game_access=false. Classic is not configured because no installed client was identified. No game snapshot or live API qualification is claimed.

Jev plan advisory: jev-1.14.0, request91782f7a8b5e05790061de6644cdcf34e503066f9a83bab95aaf0e15123e8a3f,965 input/135 output tokens,0.337s. Insufficient evidence0.69, material issue0.94; not a pass. Evidence supplied before actual scheduler and runner roundtrip qualification explicitly identified those as missing. The direct qualification above supplies those observations; it does not prove full game compatibility. An initial call used an unsupported focus string and was rejected, then corrected to plan.

Infra PR199 confirmed merged as8da910ae8b8268601ca7f455cee707aee9391f71. No new gateway image or route was changed by this local sync deployment. Full objective remains open: live client adapters/coverage, player-reviewed proposals, deeper analysis and remaining packaging/semantic/performance requirements in ROADMAP.md.

## Equipment-review MCP release

Source b008768 adds review_equipment_selection: exact snapshot ID, explicit target slot and source equipment/bag location, expected item ID, unique source/target checks, original-snapshot resolution and vacancy accounting. It returns a JSON draft with game_execution=false and all game compatibility/performance/current-ownership checks explicitly unknown. No game import, macro, execution or optimiser is introduced. docs/equipment-review.md states this distinction; the broader feature requirements remain open.

18 Rust tests pass on Windows and in the Linux image,22 Python tests pass against the Rust server, and Clippy is clean. Independent SDK stdio smoke verifies all6 read-only annotations, all3 family listings and traversal refusal. Initial Rust map-key borrows and a test clone lint were corrected before these passing runs. The required OCR skill/adversarial-review files remain absent; direct review covered planning.rs, main.rs integration, upgrade/probe helpers and associated tests.

Linux build source b008768febf79ece1f1917b7a18a1fae0d6959e8 produced reader image sha256:823a4a9111ab8814febf8b3646768398f110dfe004ae5bfe9f58937e4a87a163. Build test/build step216.5s; dependencies were recompiled, so no build-cache speedup is claimed. Deployment helper source573ba954b4a14e0eb08a6c68f82f2f39f5926e46 adds bounded socket readiness and permits existing snapshots in its health probe. Upgrade accepted exact previous d5d5135b image, fixed read-only mount/environment and imported6-tool allowlist. Before/after profile evidence: personal pylon CT290 /var/tmp/azeroth-reader-upgrade-_0u40ock. Only the dedicated reader profile/service changed; importer image, auth token, owner store and proxy route were preserved.

Actual public SDK readback after fixture upload verified equipment/summary and the new review: same-slot already_selected, source16 to target17 requested_change with vacated slot16 and compatibility unknown, wrong item ID rejected. Exact-hash cleanup removed the fixture. Post-cleanup public auth probe: missing401, wrong401, authenticatedtrue, tools6, returned Retail snapshots0. This is synthetic integration evidence, not live client compatibility or gear advice. Scheduled sync independently completed another real run with result0 and waiting_for_export for Retail/Forever.

Jev code advisory jev-1.14.0, request101339c13f94942120498e67c89fcb92bfcf318d7f71353b2928a16f2f58b25e,1049 input/131 output tokens,0.295s. Highest next-step probability no_material_issue0.44, insufficient_evidence0.27; weak advisory, not proof. Legality-implication concern0.63 prompted retaining explicit draft/unverified status, unknown checks and documentation distinguishing this from an optimiser/importer; actual public output verified those fields. Duplicate-source concern0.15 is independently covered by rejection tests. No unsupported claim of legal or optimal equipment is made.

## Measured listing optimisation and deployed verification

Source8785b905fcddfc27f4b1e3db52875c9255537012 borrows item link fields during validation instead of cloning each item Value, and canonicalises the store root once per listing. Every file still gets ID syntax validation, canonical-path containment, bounded read, SHA256 verification and full snapshot/schema validation. No validity or metadata cache was introduced. Regression tests prove family filtering, timestamp-tie ordering, pagination, extreme offsets, corrupt/schema-invalid counts and detection of tampering between calls. Linux additionally proves refusal of a hash-valid, schema-valid symlink outside the store.

Windows19 Rust tests and22 Python tests passed; Clippy clean. Linux20 Rust tests passed, including the Unix-only symlink case. Baseline release benchmark from46bcf0b and candidate runtime changes matching8785b90 ran in four alternating pairs, each with200 synthetic snapshots,180 item slots each,20 warm-cache listings and200 inspections. All per-run samples, binary SHA256s and order are in docs/benchmarks/listing-2026-11-03.json. Median of run medians: listing91.6027ms ->69.2235ms (24.4307% reduction); inspection0.4641ms ->0.4050ms (12.7343%). Each invocation creates a separate temporary store and warms it. These observations describe synthetic Windows/local-filesystem behaviour only; hosted Linux latency was not benchmarked.

Jev claims advisory jev-1.14.0, requestf967d2f203363ef423b86a3cc777ec18c74e95c03b71ebf5a97fd8400fa86d4e,1081 input/131 output tokens,0.279s. Claim-supported0.39; highest next-step no_material_issue0.37, investigate0.33, insufficient0.30. This is inconclusive, not a pass. Hosted-latency-overstatement concern0.86 is addressed by limiting every performance claim to the measured Windows workload; containment concern0.10 is independently covered by the Linux test. Required performance/OCR skill and adversarial-review paths remain unavailable, so those workflows were not loaded; direct source/test review was used.

Linux image sha256:834f91f19b2af009232c04dc54ca64757c01f6eb544ed70ead05cc37a4272a22 passed independent SDK stdio qualification (six read-only tools,3families,traversal refusal), then replaced exact previous823a4a91 image through the guarded updater. Personal pylon CT290 before/after evidence: /var/tmp/azeroth-reader-upgrade-95gf9532. Public SDK fixture readback verified the optimised listing includes the uploaded ID plus inspection, summary, equipment drafts and mismatched-item refusal. Exact-hash fixture cleanup passed; post-cleanup public probe missing/wrong token401, authenticatedtrue,6tools,0Retail snapshots. Importer image and local scheduled binary were unchanged; this was a reader release.

Build test/build step187.1s, but dependencies were recompiled. No cache-speed improvement is claimed. Docker cache inventory during the build showed only the two active exec.cachemount records; cause of absent reusable mounts is not established and no daemon-wide cache settings were changed. Scheduled sync continued with result0 and waiting_for_export for Retail/Forever. Full live-client/analysis/import requirements in ROADMAP.md remain unfinished.

## Pinned BigWigs packaging and actual packaged Lua qualification

Local .flavor.info checks reconfirmed Retail wow and Forever wowf. Installation metadata remains Retail12.1.5.70688 and Forever1.60.1.70755; no Classic installation was found. No account data is included in this evidence.

Consulted upstream BigWigs README and pinned release.sh. Lock tools/packager.lock.json pins commit0f24d2169de3ceb3b79a981311b2ebab4c6160c7, scriptSHA256 b528d51b954e14152e4f7c2612fa514dc494213e335ba9d5345ae7d6a75891a4. WSL Debian already provided Bash/git/zip/curl/Python and successfully created user/network namespaces; no tool installation was required. Builds used a fresh staging Git repository with only generated addon files plus ignored metadata/changelog, cleared environment/empty HOME, disabled global/system Git config and upload/external/localisation flags. Packager execution occurred inside the disabled network namespace. Network access is used only for the initial hash-verified upstream script fetch outside that namespace.

Strict archive checks require exactly4expected payload paths and byte-identical source, rejecting duplicates, traversal, symlinks, oversized entries and changed payloads. Normalised ZIP metadata yields repeatable output: two independent Retail runs had SHA25663fdb37545ebdaa34b0e3df9503efcb9cb5128667f4d8c0681ce92928948a2bd despite different upstream ZIP metadata hashes. Forever beta/interface16001 ZIP e828da74182c896e1598ceded0876f15db51574bd7e243c31c5a26e11c516eed; Classic era-fixture/interface11500 ZIP d879f368be8d8950a84b7275f59b4ffb77e74f0803c887bff077b5646d259e65. Classic is explicitly a synthetic package, not a detected/supported installed build. Outputs/manifests remain in ignored dist/bigwigs.

The new Rust verify_addon example loaded each actual packaged Adapter/Collectors/Core under Lua5.1, captured/decoded an equipment fixture and verified family/variant/interface. All3 rejected a changed mock interface and preserved the previous export. All reported live_qualified=false. The4installed Retail addon files match the verified BigWigs package hashes; no game files were changed by this work. Full Python suite24passes; Clippyall-targetsclean. Existing Rust runtime code was unchanged; the new example was built and invoked against all3packages.

Jev code advisory jev-1.14.0, request92910364933dc99bebf0e1e762a1fbbbfe66507045c95f24fef3cd16183e60a0,1180input/131output tokens,0.334s; next-step no_material_issue0.60, insufficient_evidence0.33. Advisory only; actual output verification and packaged runtime tests above provide the evidence. OCR skill/adversarial-review paths remain absent; direct review covered the packaging/refactor/verification helpers and tests. All outputs retain development_only; no manager or live-client qualification, publication, remote gateway change or Classic installation is claimed. ADR002 and addon-packaging.md describe remaining gates.
