# Saved talent-loadout metadata

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
