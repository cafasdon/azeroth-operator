# Saved talent-loadout metadata

## Deployment — 2026-11-04

Retail 0.14.0-dev and compatible importer/reader/companion are deployed. Public synthetic checks and all 27 installed hashes passed. Native testing remains deferred; the running client may not yet have loaded the files. See [deployment evidence](talent-metadata-deployment-2026-11-04.md). Earlier source-only checkpoints below are historical.

## Package 0.14.0-dev — 2026-11-04

The Retail package now includes `TalentMetadata.lua` and `TalentMetadataImport.lua` before shared dispatch: 27 files. Compiler output advertises player-applied support requiring addon `0.14.0-dev`, while `game_execution` and `live_qualified` remain false. This checkpoint does not change the installed addon, Windows companion or hosted services; compatible importer/reader deployment must precede installation.

Archive: `dist/talent-metadata-release/AzerothOperator-0.14.0-dev-retail-live.zip`, SHA-256 `10f9107c7164c4079e7a156286b95387b141ed67e27c664d55d4c2b7c114571c`. Actual staged Lua files passed rename/delete compiler→decoder→Review/Apply→executor→Core export→Rust receipt verification, including pre-write uncertainty and duplicate refusal. The package's existing equipment-set fixtures passed too. A staged fixture initially retained a talents-unavailable marker when supplying talents; the contradictory marker was removed and the package check passed.

Guarded installer and independent integrity verifier recognise the 27-file set while retaining legacy sets. Tests cover 25→27 upgrades with exact old-file backup, plus existing earlier-version, drift, identity and rollback cases. Full source gate passed Rust/Lua, Clippy and 36 Python tests; Clippy passed again after the staged-fixture correction. Importer deployment qualification now includes rename and delete receipts.

Jev advisory `jev-1.14.0`, request `ce2d642ce229bb7c82e24f097bda3a7f5199005fd19058973de56a881fbf5677`, 1042 input/134 output tokens, 0.289 s, returned investigate_supported_issue at reported confidence 0.10 without a concrete defect. Direct inspection confirms the installer still checks old hashes, exact file sets and identity, backs up before writing and publishes TOC last; offline tests do not establish native qualification or deployment. No AI review pass is claimed. Configured OCR/adversarial files remain absent.

## Shared review and receipts — 2026-11-04

Source Review/Apply now dispatches the strict metadata decoder and explicit rename/delete executors/verifiers. The shared caller retains the exact reviewed bytes and records `outcome_unknown` before the executor. Lua receipts, Rust snapshot validation and SavedVariables receipt parsing accept both metadata kinds. Normal unknown results permit observation-only verification for 60 seconds; exceptions, expiry and reload retain uncertainty. Repeated operation IDs remain refused.

Review shows target ID/name/spec, selected build, proposed name or deletion/recreation consequences, preservation requirements, and the absence of actual action-bar verification. Native Talents must be open and competing dialogs closed. No external reference repair is promised.

Actual Rust compiler → Lua decoder → real executor → Core export → Rust importer fixtures verify rename and delete, pre-write checkpoints, exact-review rejection, duplicate refusal, delayed completion, expiry, post-effect exceptions and reload. Widget fixtures verify consequences and sanitised names. Full source gate passed Rust/Lua tests, Clippy and 35 Python tests after correcting a fixture's expected sanitisation from doubled pipe to the existing slash replacement.

Source integration is not deployment: `execution_supported` stays false until packaging and compatible importer/reader rollout. Native qualification is deferred. Jev advisory `jev-1.14.0`, request `1624454cfd21459a3d0fbeed2c27d9416341398023701c35e20b862e76a06c25`, 951 input/135 output tokens, 0.297 s, returned insufficient evidence; no review pass is claimed. Direct source inspection and actual producer/consumer fixtures support the bounded offline claims. Configured OCR/adversarial files remain absent.

## Import transport — 2026-11-04

Metadata preparation now returns `AO-TALMETA-1` import data. `execution_supported` remains false: the source decoder is not packaged or connected to shared Review/Apply. Nothing deployed or applied in game.

Wire layout: magic, operation ID, snapshot ID, family, variant hex, build hex, interface, character hex, realm hex, kind, target config ID, rename-name hex (delete uses `-`), spec ID, working config ID, selected config ID (zero means absent), serialization version, active import hex, record count. Each saved record adds ID, name hex, shared-bars flag, serialization version and import hex. Exact arity is `18 + 5*N`, with 1–128 records and a 512 KiB total limit. Operation ID hashes the canonical wire with its ID field blank; this is binding/identity, not authentication.

`TalentMetadataImport.lua` is data-only. It validates canonical integers, lower-case hex, bounded text, build alphabet, record count, booleans, uniform versions, unique IDs excluding the working config, target presence and rename/delete constraints. It reconstructs the executor's complete expected state, omitting absent selected IDs. Shared player review and receipt integration remain required before execution support.

Rust-to-Lua 5.1 rename/delete round trips and malformed field, duplicate, oversize, extra/truncated, selected-deletion and legacy-load regressions pass. Full source verification passed formatting, locked build, all Rust/Lua tests, Clippy and 35 Python tests. The initial test compilation exposed mlua errors lacking Send/Sync conversion into anyhow; fixture assertions were corrected and the complete gate rerun successfully.

Jev advisory: `jev-1.14.0`, request `f5a20e9cd95879c1ba523342f559f50b24b043f08f7ec7a231b1922d8663dc10`, 946 input/131 output tokens, 0.288 s; no_material_issue at reported confidence 0.28 with low evidence sufficiency. This is not independent proof. Direct decoder/source inspection and producer-consumer tests support the transport claims; no native qualification is claimed. Configured OCR/adversarial files remain absent.

## MCP preparation — 2026-11-04

`prepare_talent_operation` now accepts `operation: rename|delete` with a target ID and a required name for rename. Omitted operation still means load and retains the existing AO-TALENT-1 format. Metadata drafts bind client identity, snapshot, all captured current-spec saved entries, active contents, working config, selection and serialization version. Their deterministic hash changes when any bound bystander changes. Mixed versions, working-config catalogue entries, colliding/invalid names and selected deletion are refused.

These are source-only, non-executable drafts: `execution_supported: false`, `import_code: null`. No deployed service or addon changed. The catalogue remains a captured observation; the executor must independently compare the complete native state before writing. Transport, review, receipts and packaging remain next. Live testing is deferred at the user's request.

Verification: full source gate passed formatting, locked build, Rust/Lua fixtures, Clippy and 35 Python tests, including actual MCP requests and unchanged legacy load transport. Tests cover selection refusal, selected rename, catalogue hash binding, collision, mixed versions and invalid arguments. Jev advisory `jev-1.14.0`, request `4d3044b61035f9e71b8f04ed341b2ad5345300db666ac23c5a979a20c2a190f7`, 948 input/135 output tokens, 0.306 s, returned insufficient evidence; this is not a review pass. Direct source inspection and deterministic tests support the bounded preparation claims only. Configured OCR/adversarial files remain absent.

## Source executor — 2026-11-04

`addon/TalentMetadata.lua` adds source-only `rename_talent_config` and `delete_talent_config`. It is not in TOCs/packages, MCP preparation, import dispatch or receipt allowlists yet. No installed files or hosted services changed.

Primary reference: the [native edit dialog](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentLoadoutEditDialog.lua) calls `C_ClassTalents.RenameConfig` and `DeleteConfig` directly. The [native talents frame](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentsFrame.lua) separately tracks selected saved configurations and handles deletion events. API references establish a callable path, not live qualification or blanket approval.

Requests bind family/variant/build/interface/character, target ID and a full expected state: current spec, working config, selected saved config, serialization version, active import string, and every saved entry for the current spec (name, import string, serialization version and shared-action-bar flag). The executor reads the entire native state twice and compares both copies and the supplied preimage. Sparse, duplicate, restricted, changing or unavailable observations fail closed. Staged changes in working or saved configurations are refused. The active working config cannot be a saved target.

Apply requires the visible idle non-inspecting player Talents panel, matching working config, no cast/channel/combat/cursor, no competing create/edit/import dialog or deletion/shared-bars confirmation, and native permission to edit talents. Rename rejects invalid/colliding names and treats an identical name as a no-op. Exactly one native rename/delete call follows. No load, commit, spec switch, action-bar change, fallback, rollback or retry is requested.

Readback requires the complete expected state, changing only the target name or removing only the target entry. Active talent contents, selected ID and every captured bystander must match. Return values alone are not success evidence. Normal delayed outcomes remain unknown with an observation-only verifier available for later integration. Native exceptions propagate: the future shared caller must checkpoint unknown before entry and never automatically replay or reconcile thrown writes.

Renaming a selected saved build is supported by this source contract. Deleting the selected saved build is explicitly refused until selection side effects are resolved; this is unfinished scope, not a claim that the native API forbids it. Coverage is current-spec saved loadouts only. Other specs, actual action-bar contents, native timing/taint/name limits, and hidden UI transitions are not qualified. Talent creation/import/node editing remain separate outstanding work.

Actual Lua 5.1 fixtures cover rename/delete success despite false native return, true return without effect, no-op, selected rename, selected-delete refusal, delayed completion, exceptions before/after effect, active-content/selection/bystander/shared-flag changes, duplicate/sparse lists, missing APIs, staged changes, stale/spec/version changes, restricted reads, racing selection reads and UI/combat/cursor guards. Full `python -m tools.verify_source` passed formatting, locked build, all Rust/Lua fixtures, Clippy and 35 Python tests.

Advisory Jev review: `jev-1.14.0`, request `6f261e14e5b9709a83b4ed54f0678b9de545f37dc1caf2662dcb13858ae6ad5b`, 2868 input/153 output tokens, 0.301 seconds; `investigate_supported_issue` with reported confidence 0.30. No concrete defect was supplied. Direct reinspection confirms complete structured state comparison before/after the single call; fixtures reject changed active contents, bystanders and selections. This does not prove uncaptured effects or native behaviour. The configured OCR skill and adversarial rule files were checked and are absent; neither workflow is claimed.

Next: compiler and strict import transport, player review including deletion consequences, receipt integration, packaging and compatible importer rollout. Native testing remains deferred. Existing unknown game operations must not be replayed.

## Selected saved-build deletion — source integration, 2026-11-04

Selected deletion is now implemented in source, with compiler execution_supported/addon_execution_enabled false and no minimum addon version until packaging. Deployed 0.15.0-dev still refuses this case. Existing nonselected deletion and rename contracts remain unchanged.

Native reference: [ClassTalentsFrame](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentsFrame.lua), OnTraitConfigDeleted and ClearLastSelectedConfigID. On the selected build's deletion, the native handler clears its saved selection. Our executor calls DeleteConfig once and leaves all event/selection handling to Blizzard. It requires matching API preimage, native frame and dropdown selection, available native handlers and no pending creation/commit. No manual selection clear, load, commit, rollback or retry is introduced.

Desired readback removes exactly the selected target and requires nil API/frame/dropdown selection while preserving active working config, committed contents, spec/version and every other saved entry. UI review explains this consequence. Missing/delayed native events, changed active/bystander contents, or stale UI selection retain unknown. The current AO-TALMETA-1 data shape already binds the selected ID, so no extra wire field is required.

Tests cover producer/decoder and review wording, single-call execution, delayed API and independent frame/dropdown clearing, missing handlers, pending-operation/stale-UI refusal, and full compiler→shared Apply→Core export→Rust import. Shared expiry, reload, exception and replay tests run for selected deletion too. Full source gate passed Rust/Lua, Clippy and 37 Python tests; the affected metadata suite passed again after independent UI readback was added. Native APIs remain mocked; no game input or deployment.

Jev advisory jev-1.14.0, request `532792edb1c80e7db59f47fb99eea2f9d7e0573c005523e20cf0d1f3c1ca6192`, 1039 input/134 output tokens, 0.294 s: investigate_supported_issue (0.82 probability). It supplied no concrete finding. Follow-up source inspection identified that API-only selection readback could omit stale UI state; frame/dropdown readback and independently delayed tests were added. This is not a passing AI review; native event compatibility remains unresolved. Configured adversarial review file is absent. Packaging and native qualification remain pending.

## Selected deletion package — 2026-11-04

Retail 0.16.0-dev packages selected saved-build deletion in the existing 29-file layout. Compiler preparation enables this case with minimum addon 0.16.0-dev, while retaining rename/nonselected-delete minimum 0.14.0-dev and creation minimum 0.15.0-dev. The staged verifier runs selected deletion through actual packaged Lua, compiler data, shared Review/Apply, Core export and Rust import; the mocked native effect clears selection, the pre-write unknown receipt is asserted and duplicate attempts are refused. Existing metadata, creation and equipment fixtures also pass.

Artifact `dist/selected-delete-release/AzerothOperator-0.16.0-dev-retail-live.zip`, SHA-256 `c40af5251f618118fc6d7ba0f782f5243f103acec1d28a9ae99586cf4fb26c9c`. Full source gate passed Rust/Lua tests, Clippy and 37 Python tests. No installation, hosted rollout or game action occurred in this package checkpoint; native qualification remains deferred.

Jev advisory jev-1.14.0, request `8cc6ae4f0c1c01a9745e30b101719ffa70bf08783d92955cfbb902ba4c61dae8`, 957 input/116 output tokens, 0.299 s: insufficient evidence. No AI review pass claimed. Direct packaged execution and deterministic tests establish only the offline contract. Configured adversarial review file remains absent.
