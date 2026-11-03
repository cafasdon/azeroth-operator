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


## Captured-loadout deployment, 20:05 UTC

Reconciled the interrupted deployment by reading the actual dedicated Docker MCP profile, importer image pin and active service. Both use `sha256:5a3467d27b7c238a00d9113e0273881106bcfab0d6e8c81e68f27767c02ce02c`, built from `c61d065712d8e05efad49a33c2cd7c88f6dd109f`; seven tools are allowlisted. No mutation was blindly replayed.

Uploaded synthetic snapshot `e979f4b91ba6ff84962f92da3e078d93cb3087cbc49a66deec9489755bf3dac8` through the restricted receiver. Public TLS smoke passed all seven-tool assertions including exact captured string `QUJDRA==`, unverified compatibility and no game execution. This is a synthetic base64 string, not a valid talent build. The first local fixture omitted required talent fields and was rejected before upload; adding config_id and nodes produced the qualified fixture. Exact-fixture cleanup succeeded; subsequent authenticated public discovery passed.

Installed Windows companion SHA256 `d79cd255c8d8b235cf2d0be1eb3e6c136a8f4822605b07d6c4bbb6c8b51dc05a` in the hash-addressed local runtime and updated sync configuration with a prior-config backup. Direct task trigger returned LastTaskResult=0; fresh 20:05:53 UTC status reports Retail and Forever waiting_for_export.

Only after all three schema consumers were upgraded, installed Retail addon 0.1.1-dev using the guarded updater. No Wow.exe process was observed. All four files verified; exact backup is local-data/addon-backups/addon-update-hy6nzoyk. ZIP SHA256 `b7dc6b3eadf6f0d3a667ae3b8aaa239f3dd11608c6c9715427bb1094409df59c`. No game launch, reload or input was performed. Forever installation unchanged; Classic absent.

Verification: 25 Python tests passed. Earlier source qualification passed 20 Windows and 21 Linux Rust tests and packaged Lua checks. Rollback test preserves concurrent edits. Required adversarial-review.md remains unavailable; no claim that its workflow ran.

Prior Jev plan advisory (jev-1.14.0, request f40a702dfc76faea3c586b4b1ffd20fa6130ff5af4a7f67dcd300447729b27d4, 1121 input/135 output tokens, 0.36 s) returned insufficient_evidence 0.57 and material_issue 0.79, with schema-order and optimisation-confusion concerns. It was not a pass. Ordered deployment and public readback now provide direct schema compatibility evidence; live API acceptance and optimisation remain explicitly unverified.


## Explicit installation metadata inventory

Added `python -m tools.inspect_client` with explicit absolute client path, family and variant. Reads only bounded regular metadata files, rejecting link/reparse ancestors. Family/product mismatch and malformed records fail closed; multiple distinct active versions are ambiguous. Variant is labelled caller-supplied, interface null, game_execution false, live_qualified false. This does not authorise writes or establish live API support.

Actual CLI calls identified Retail product wow/build 12.1.5.70688 and Forever product wowf/build 1.60.1.70755, matching the local metadata read independently. The explicit _classic_era_ directory reported not_installed; this is not a host-wide absence claim. Two new tests cover all three family boundaries, conflicting versions, inactive/other-product filtering, malformed metadata and byte limits. No hosted service or addon change was required.

Jev plan advisory: jev-1.14.0, request 3253aa3d3e27884bed4d2469d3b5b98a869bf8b416be5a18479a8f191d33ed22, 832 input/116 output tokens, 0.318s. Returned insufficient_evidence (choice confidence 0.28; probability 0.52), material_issue 0.70, evidence_sufficient 0.31 and overstatement concern 0.40. This is not approval. Actual metadata output and deterministic tests supply the implementation evidence; runtime compatibility remains explicitly unverified. Required bdr-ocr-review and adversarial-review files remain absent, so those workflows did not run.
`python -m unittest discover -s tests -q`: all 27 Python tests passed; git diff --check clean.


## Installed addon drift verification

Added tools/verify_installed_addon.py. Read-only, fixed four-name manifest allowlist, 16KiB manifest/1MiB file limits, link/reparse rejection, bounded directory enumeration, exact product-family check. Nonzero for absent or drifted installs; no automatic repairs. Caller-supplied manifest trust and live_qualified=false are explicit. Observation is not a lock against concurrent edits or a signed provenance claim.

Actual Retail 0.1.1-dev invocation returned matches_manifest, verified_files=4, problems=[] against dist/bigwigs-0.1.1/AzerothOperator-0.1.1-dev-retail-live.manifest.json. Test verifies edited content is preserved, missing/extra/oversized files reported and traversal manifest names refused. Full Python suite: 28 tests passed; git diff --check clean. No addon, game, sync or hosted deployment changes.

Jev claims advisory: jev-1.14.0, request 7d73ec87f26e36bc56928951fab702e15d1c914adbcc701183981240cc02a4e3, 933 input/112 output tokens, 0.309s. claim_supported=0.78, evidence_sufficient=0.73, no_material_issue choice confidence=0.73 (probability=0.82); concern_1=0.96 answered the positive question whether the result justifies only point-in-time manifest integrity, without authenticity/live compatibility. Advisory does not replace actual readback/tests. Required bdr-ocr-review and adversarial-review files remain absent; those workflows were not run.


## Primary collector failure isolation

Moved existing equipment, bags and stats reads into the same protected collector path as the other eight sections. A thrown API error or unserialisable result now omits that section and records unavailable; it does not publish an empty inventory or discard unrelated observations. Identity/build/interface and combat failures remain fatal and preserve the prior snapshot. No new API, schema field or game write.

Real Lua5.1 producer-to-Rust-decoder tests inject errors in each primary section and assert ten surviving sections, missing failed section, explicit unavailable reason and retained quest observation. Missing C_Container is covered separately. Full Windows suite: 20 Rust tests and 28 Python tests passed. Pinned network-isolated BigWigs Retail 0.1.2-dev package SHA256 7cbfdd58252fe33ce8d5c04237ff73e9153331eb61a9bdf56fad008f2b1c3461; actual packaged Lua roundtrip and interface-change refusal passed. Live compatibility remains unverified.

Jev advisory jev-1.14.0, request d6910fa5058cdc39268fcdcb4ce4bb4bc7d9fbb27e1017676531b18b864e555c, 895 input/112 output tokens, 0.568s: material_issue 0.23, evidence_sufficient 0.36, false-empty concern 0.20; next-step no_material_issue confidence 0.13 with probability tied at 0.42 against insufficient_evidence. Treat this as inconclusive, not approval; deterministic producer/decoder tests establish the narrow claim. Required diagnosing-bugs and adversarial-review files were absent; those workflows were not run.

Installed Retail 0.1.2-dev after observing no Wow.exe process. Guarded updater backed up prior files to local-data/addon-backups/addon-update-g6miv8xy and verified four replacements. Independent read-only installed verifier returned matches_manifest with no problems. Source commit 6160cdd. Hosted reader/importer and local companion unchanged because the schema is unchanged.


## Capture-local serialisation reuse

Successful section JSON was previously discarded and encoded again in the final envelope. Reuse those strings only within the synchronous capture; no encoding survives to the next capture. Current collectors own their returned tables and do not mutate earlier sections. New collectors must preserve that property. Secret-value checks still precede cache lookup; failed serialisation still produces unavailable, not empty data.

Fixed benchmark examples/benchmark_addon.rs runs four alternating pairs of separate Lua5.1 VMs, ten warmups then 100 captures per run, 180 bag and 19 equipped items. Each pair asserts exact byte identity and validates output with the Rust decoder. Evidence docs/benchmarks/addon-serialization-2026-11-03.json: median per-run mean 6.7497645ms before versus 5.164652ms after, 23.5% reduction. These are Windows fixture timings, not game measurements or a universal performance guarantee. Reproduce using the baseline Core.lua from f2fa14b and current Core.lua as the two positional paths to cargo run --release --locked --example benchmark_addon.

Producer regression test passes including reused-table mutation between captures and unsupported-value rejection. Clippy all-targets clean. Pinned BigWigs 0.1.3-dev package SHA256 98e0a34323ca5d88a49befce00d80a49daee068edd93728be078e136dc0422e6 passed packaged Lua roundtrip and interface guard. Schema unchanged.

Jev claims advisory jev-1.14.0, request d11b91d76045f03e55339e9eb05d0ff6d6c6e1224caefe3035a2a188a2ce670c, 1002 input/116 output tokens, 0.437s: insufficient_evidence confidence0.43/probability0.62, claim_supported0.42, evidence_sufficient0.26, positively phrased bounded-claim concern0.69. Not approval. The recorded samples and equality assertions support only the narrow fixture result. Required bdr-performance and adversarial-review files absent; those workflows did not run.

Installed Retail 0.1.3-dev after observing no Wow.exe process; backup local-data/addon-backups/addon-update-u82fltcv. Four files verified both by updater and independent read-only manifest check. Source e3eacc6. Hosted service and companion unchanged; no game input performed.


## Operation scope evidence record

Added docs/operation-policy.md to cover the user's explicit request for action boundaries before control. Read official Kaivax addon-policy opening post; excluded community replies. Official EULA indexed clauses retrieved, but direct en-US and en-GB page fetches returned403. Therefore no complete regional/account legal review or express MCP authorisation is claimed. Matrix distinguishes implemented observations, manual review, missing features and disabled/excluded game actions for all families. Source inspection confirms seven read tools and no addon command intake; no runtime boundary changed.

Jev claims advisory jev-1.14.0, request b5745a94df056b88ff97f5ef2fb1c5c855bd06ba6bded3efcad07913eba18e22, 873 input/116 output tokens, 0.556s: insufficient_evidence confidence0.70/probability0.80, claim_supported0.68, evidence_sufficient0.53, accidental-approval concern0.13. This does not approve the bridge; incomplete policy evidence remains an explicit outstanding requirement. Documentation-only update.
Authenticated public HTTPS smoke passed with seven tools. No fixture or live character readback claimed in this check.


## Hex encoding lookup optimisation

Replace per-payload-byte Lua callback/string.format with a 256-entry lookup table built once at addon load. JSON encoding, escaping, secret checks, wire format and4MiB limit unchanged. Regression test passes ASCII0..127 plus Portuguese, Chinese and emoji through actual Lua5.1 export and Rust decode. Existing failure-isolation/fatal-preservation tests pass; Clippy all-targets clean.

Fixed benchmark examples/benchmark_addon.rs, baseline Core.lua at888d162, current Core.lua: four alternating pairs,10warmups/100captures each,180bag/19equipment entries. Exact output equality asserted each pair. Samples and hashes in docs/benchmarks/addon-hex-2026-11-03.json. Median of per-run mean capture times4.934835ms before/2.0538005ms after,58.4% lower. Windows fixture only, not in-game latency/FPS. Packaged0.1.4-dev SHA256603019688b98a5282bb088897786aa668fbec86ca4029971f5efd6e0a6a73c61 passes actual packaged Lua checks.

Jev claims advisory jev-1.14.0 request4eaacc7e19f62ed247c4e4e9d4efc8e3183bf2d8beab44483b21194002a810f2,975input/112output tokens,0.297s: claim_supported0.80, evidence_sufficient0.64, unsupported-claim concern0.31, no_material_issue choice confidence0.33/probability0.55. Advisory only; measured samples and output equality supply the narrow evidence. Required bdr-performance and adversarial-review files absent; those workflows did not run.

Installed Retail0.1.4-dev after no Wow.exe process was observed. Backup local-data/addon-backups/addon-update-vn3f8zgq. All four installed files matched via updater and independent verifier; source96c8fc2. No companion/hosted upgrade needed; schema unchanged.


## Default protocol-test coverage corrected

The previous default Python suite selected the two-tool Python reference server unless AZEROTH_TEST_BINARY was set. Its 28-test count did not by itself establish Rust protocol coverage. Existing separately recorded Rust/public probes remain valid. Now distinct tests always exercise both servers: default Rust binary target/debug/azeroth-operator (platform suffix), explicit override permitted, missing binary fails with build instructions. Each session has a60-second timeout.

Built current Rust server with cargo build --locked. Independent Python SDK test exercises all seven Rust tools, exact capability inventory, read-only/non-destructive annotations, game_execution=false and empty live-qualified list. Added unknown equip_item refusal: actual server returns protocol McpError tool not found, not a successful call result; corrected the test to assert that observed contract. Full suite29 passed before timeout wrapper addition; targeted protocol tests rerun after it. No production/source runtime behaviour changed. Required adversarial-review file absent; its workflow not run. Deterministic checks only; no advisory model judgement used as proof.


## Unified source verification

Added python -m tools.verify_source: format check, locked build, locked Rust tests, Clippy all-targets warnings-as-errors, Python protocol/tool suite. Explicit CARGO_TARGET_DIR and AZEROTH_TEST_BINARY bind the protocol test to this checkout's freshly built target/debug executable. Each child has600-second deadline and reuses the existing cross-platform process-tree cleanup; no packaging/deployment. Test injects a build failure and stale environment overrides, proving no later checks run and both paths are replaced.

Actual gate passed all five stages:20 Rust tests,30 Python tests; binary SHA25634c435ec7e430b5314412be66f992ef856dd32e21b6212f1d70dca22aa8aa318. This is local source verification, not production qualification or a hermetic dependency environment. Deterministic verification only. Previous absent adversarial-review workflow remains unavailable.


## Deployed Linux image protocol refusal coverage

Inspected Dockerfile: Rust/Lua/CLI tests execute during build; Python protocol checks are external and must not be implied by Docker build success. Extended tools/smoke_mcp.py to verify exact implemented capability list, game_execution=false, no live-qualified clients, read-only/non-destructive annotations, all three family empty listings, and invalid/missing-snapshot refusal by inspect, summary, comparison, equipment review and captured loadout. Unknown equip_item must return tool-not-found protocol error.

Executed the independent Python SDK caller through SSH to personal pylon CT290 against exact deployed image sha256:5a3467d27b7c238a00d9113e0273881106bcfab0d6e8c81e68f27767c02ce02c in a disposable docker run --rm container, network none, root read-only, all capabilities dropped, no-new-privileges, no production store mounted. Exit0: seven tools, three families, five invalid-snapshot refusals, traversal and unknown mutation rejected. This verifies Linux image protocol boundaries with an empty store, not live export handling or public authentication (qualified separately earlier). Existing hosted deployment unchanged. Deterministic checks only; no model verdict used as proof.


## Sync discovery failure isolation

Previously a discover() OSError/ValueError aborted the entire pass before status publication. Now each family records failed_retry_next_pass and continues; normal main() status publication and nonzero failure result still apply. No retry loop, path disclosure or limit relaxation. Global configuration/aggregate-limit failures retain existing fatal behaviour.

Test injects permission failure for one root and a valid export for the other family, verifies one invocation with the correct family and uploaded hash, and checks the private error text is absent. All31 Python tests passed. Required adversarial-review file remains absent; no claim its workflow ran.

Updated only the existing Ready scheduled task action after matching its old runner hash e27eb99175a4e9647d9ad8d63b938517ecc64711db4b90ddfcd823e0334bb32d and pythonw executable. Prior XML retained at local-data/sync/task-before-discovery-fix.xml. New installed runner SHA256b6d38732a944fd44a8013d3bcfb894feb4e23d7d7f7ccfd584936b39de80fa85, copy/readback verified; existing task principal/settings/triggers/config preserved. Direct trigger completed LastTaskResult0, fresh20:25:27Z status waiting_for_export for both Retail and Forever. Real missing-export path verified; failure-path qualification is injected, not a production fault. No game input, upload or hosted change.


## Carried-item review source candidate

Added bounded aggregation to existing character_summary: observed unit totals, distinct/repeated item-ID counts, up to20 repeated groups and20 exact bag/slot locations per group with independent truncation flags. Full group/count totals preserved. Missing/malformed bag data is null; observed empty bags zero. Explicit item_id_only_not_item_identity, stack_compatibility unknown, free_slots not_captured and game_execution false. Different item links sharing an ID are not assumed fungible. This does not fulfil item movement or optimisation.

Source gate passed21 Rust tests,31 Python tests, formatting and Clippy. Rust coverage includes different variants, multi-bag counts, missing/empty/malformed data and462 observations across21 groups. Actual PythonSDK-to-Rust MCP call verifies the new summary field. Debugbinary SHA256bb287367687f6f9e9d284160298184c96c1562520e454069f8eddcd73153c85d. Hosted image/local installed companion unchanged; deployment pending.

Jev claims advisory jev-1.14.0 request0b26ab6afd2c3d970e004a150e9299d3ba360ad8676d9a4c8d07dd0cd89bd50f,873input/115output tokens,0.393s: investigate_supported_issue confidence0.31/probability0.54, claim_supported0.19, evidence_sufficient0.25, misleading-fungibility concern0.22. No concrete defect identified by this typed advisory; independently inspected fields and deterministic tests support only the stated observational contract. Not approval or live qualification. Required adversarial-review workflow remains unavailable as previously checked.
