# Talent creation and import

## Creation preparation and transport — 2026-11-04

Source `prepare_talent_operation` accepts `operation: create`, `name` and required boolean `expected_new_shared_action_bars`; it forbids `target_config_id`. Existing load/rename/delete requests still require their positive target ID, reject creation-only fields and retain their prior wire formats. Creation binds the full captured current-spec catalogue and active contents, rejects collisions, mixed versions, a working config listed as saved, and catalogues already containing 128 entries. Empty captured catalogues are supported.

`AO-TALCREATE-1` uses the metadata header layout with creation kind and `-` target, then a new shared-bars expectation field. Exact field count is `19 + 5*N` for 0–127 saved records; total limit is 512 KiB. Each record carries config ID, name hex, shared-bars flag, serialization version and import hex. The operation ID hashes canonical wire with the ID field blank; it is identity binding, not authentication. `TalentCreateImport.lua` is a data-only strict decoder that reconstructs the creation executor's preimage.

Output explicitly reports `selects_new_build: true`, `execution_supported: false`, `addon_execution_enabled: false`, no required addon version yet, and no verification of actual action-bar contents. Shared player Review/Apply and receipt integration remain pending, so this transport is not packaged or deployed.

Actual Rust compiler→Lua decoder→creation executor fixtures passed, along with empty catalogue, malformed fields, duplicate IDs, wrong versions, target refusal, missing shared-bars expectation, name collision and capacity refusal. The expected shared-bars value changes the operation hash. Actual MCP calls and existing talent operation regressions passed. Full source gate: Rust/Lua, Clippy and 36 Python tests. Only the tool-description prose changed after that gate.

Jev advisory `jev-1.14.0`, request `2fe1e33b7cf2a2efd285df44c4265dd39a50d3e1626f753b20960e5d65431e34`, 1045 input/135 output tokens, 0.297 s: insufficient evidence. No AI review pass is claimed. Direct compiler/decoder inspection and producer-consumer tests support the data-only contract; native qualification remains absent. Configured OCR/adversarial files remain absent.

## Current-build creation executor — 2026-11-04

`TalentCreate.lua` adds source-only `create_talent_config`. It requires exact client identity, full captured current-spec catalogue/active contents, a non-colliding name and explicit `expected_new_shared_action_bars`. The latter is a readback expectation, not a setting request: the executor never changes shared-bars settings and does not assume the native default.

Apply requires native capacity/edit permission, no combat/cast/channel/cursor or staged changes, visible idle player Talents, no competing dialogs/popups and no pending native creation/population/commit indicators. It calls `RequestNewConfig` once and, only on a true acceptance return, invokes `OnTraitConfigCreateStarted` with the captured purchased-rank flag, matching native `NewEntryCallback`. Blizzard's own events own population and selection. There is no direct commit, guessed ID, fallback, rollback or retry.

The observation-only verifier requires exactly one added ID, the requested name, original active import contents/version, expected shared-bars flag and selection of that new ID. All prior catalogue entries, working config, active contents and spec must remain identical. A successful native return is insufficient; false return with complete readback can still be confirmed. Exceptions propagate for the future caller to retain its pre-write unknown checkpoint. Actual action-bar contents are not verified, and UI/server timing remains native-unqualified.

Read-only stable catalogue/comparison primitives are shared from `TalentMetadata.lua`; existing metadata behaviour is unchanged. Lua fixtures cover delayed completion, false-return-with-effect, wrong contents/settings, multiple added IDs, changed active/bystander data, exceptions before/after effect and in native bookkeeping, duplicate preimage refusal, capacity/staged/client/UI/pending-operation guards and restricted observations. Full source gate passed Rust/Lua, Clippy and 36 Python tests; the targeted creation suite passed again after pending-operation guards were added.

Jev advisory `jev-1.14.0`, request `a779c4ffa9375df889850ee3dc367acbc12382600e08d8afb195a90a86ec69b2`, 1066 input/135 output tokens, 0.328 s: insufficient evidence. No AI review pass is claimed. Native-source comparison and deterministic fixture results support only the stated source contract. Configured OCR/adversarial files remain absent. Compiler/transport/player review/receipts, packaging and deployment are pending; arbitrary import execution is still unfinished.

## Entry conversion inspection — 2026-11-04

`TalentImportEntries.lua` adds source-only `PlanTalentImport`, composing the bounded inspector with selected node/entry observations and native `ConvertToImportLoadoutEntryInfo`. It independently derives expected single-node, choice/subtree, granted and tiered entry records from copied metadata, compares the native conversion exactly, and refuses missing/extra/sparse records, wrong choices/ranks, or discarded tiered ranks. Node descriptions and the full inspected plan are read again before returning. The helper calls no mutation APIs.

Each selected node has bounded dense unique entry IDs; descriptors retain type, maximum ranks, entry capacities and any active-entry fallback. Return data includes copied entries/descriptors and `conversion_verified: true`, while retaining `semantic_legality_verified: false` and `game_execution: false`. Conversion fidelity is not eligibility, point-budget validity or successful native import. This module is not packaged or exposed through MCP.

The independent reference is the native [single/tiered conversion implementation](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentImportExport.lua). Lua 5.1 fixtures cover partial/full ranks, choices and hero choices, tiered distribution, grants, dropped/added records, wrong ranks/choices, invalid indices, missing/duplicate metadata, racing observations, converter input mutation and restricted values. The conversion dependency is mocked; native compatibility remains unqualified. Full source gate passed Rust/Lua tests, Clippy and 36 Python tests.

Jev advisory `jev-1.14.0`, request `7a0f573773debd8a186bf9d6a75ba23fd3ae217d14ab42dd8ed0ca928f428449`, 1061 input/135 output tokens, 0.308 s: insufficient evidence. No AI review pass is claimed. Direct source inspection and fixtures support the bounded read-only conversion contract. Configured OCR/adversarial files remain absent. Creation/import executors, legality/readback and asynchronous selection integration remain unfinished.

## Native flow and source inspection — 2026-11-04

Sources inspected: [create dialog](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentLoadoutCreateDialog.lua), [import dialog](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentLoadoutImportDialog.lua), [class talents frame](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentsFrame.lua), [import/export mixin](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentImportExport.lua), [ExportUtil](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_SharedXMLBase/ExportUtil.lua), and [generated class talents API](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/ClassTalentsDocumentation.lua). These are mirrored native sources at the live branch, not proof of behaviour on the installed build or policy authorisation.

The create callback requests a new configuration from current contents, starts native creation bookkeeping on success, and waits for server population. Completion selects the new configuration through the native UI. Import validates header/version/spec and any nonzero tree hash, converts nodes, calls native import and enters the same creation lifecycle. Therefore import can affect selected/active talents; a catalogue-only preservation contract would be incorrect. New configs may not be immediately populated. Creation success alone is insufficient completion evidence.

## Read-only validator

`addon/TalentImportValidation.lua` is source-only and not packaged, dispatched or exposed through MCP. `InspectTalentImport` uses native header/content parsing through a bounded stream wrapper. It checks the Retail family, visible idle player talents, unstaged working config, spec/version, dense sorted tree IDs, 16-byte tree hash, 16 KiB import limit, exact header consumption, body length and at most five zero padding bits. The native alphabet is unpadded six-bit encoding; `=` is rejected here. Omitted (zero-filled) hashes require an explicit boolean acknowledgement and are reported in the result.

The helper compares copied active/spec/tree/version/hash/node observations before and after parsing. It returns copied parsed content and explicit `semantic_legality_verified: false`, `game_execution: false`. It never calls import, view-loadout, reset, purchase, commit or selection APIs. Structural parsing does not establish point budgets, node eligibility, tiered-node mapping, canonical desired contents, or that the native import will succeed.

Rust-generated synthetic bitstreams and Lua 5.1 fixtures cover spec/version/hash mismatch, omitted-hash acknowledgement, truncation, excess data, padding, malformed/sparse/duplicate/racing tree observations, missing content, invalid partial ranks, staged state, restricted values and family refusal. Full source gate passed Rust/Lua tests, Clippy and 36 Python tests. The parser dependency is mocked in these fixtures; native source inspection is the independent format reference. Native parser compatibility remains unqualified.

Jev advisory `jev-1.14.0`, request `f161111d15eb2d1a1ee8ae6ffeaa1cb59e758ed12d91c46653d4b1d27927d508`, 1010 input/135 output tokens, 0.321 s: insufficient evidence. No AI review pass or native qualification is claimed. Configured OCR/adversarial files are absent; direct source inspection and bounded fixture results support only the stated helper contract.

Remaining: complete node/entry conversion and legality/readback contract; current-build duplication and arbitrary import executors with full catalogue preimages and native creation/selection lifecycle; explicit user review of active talent and action-bar consequences; completion/receipt integration, transport, packaging and deployment. No retry after uncertain creation and no guessing the server-assigned ID. Native tests remain deferred.

## Player review and receipt integration — 2026-11-04

Creation now uses the shared exact-byte Review/Apply dispatcher, explicit executor/verifier maps and `create_talent_config` receipt kind. The review explains copying committed contents, automatic selection of the new build, checked (not set) shared-bars expectations and unverified action-bar effects. SavedVariables and Rust snapshot/receipt parsing accept the new kind. Compiler execution remains disabled pending packaging; none of this source change is installed or deployed.

The compiler-to-decoder-to-Apply fixture asserts the unknown checkpoint exists inside the mocked native call, then exports through Core and imports through Rust. It covers review mismatch, duplicate attempts, delayed observation-only completion, 60-second expiry, exceptions before/after effect and in native bookkeeping, and reload with an outstanding unknown receipt. No path retries the native call. The persisted receipt prevents replay of the same operation ID; it is not semantic exactly-once protection against newly generated IDs. SavedVariables become durable only on reload/logout.

Full local gate passed format, build, Rust/Lua tests, Clippy and 36 Python tests. Native calls remain mocked and no in-game qualification is claimed. Configured OCR/adversarial review files are absent; direct inspection and the actual local compiler/export/import path are the evidence.

Jev advisory `jev-1.14.0`, request `75ea70d0a4d73b6628106d94ba2c5d2cdbc048ed397109c58c2c85a51f9b4676`, 1025 input/131 output tokens, 0.275 s: `no_material_issue`, confidence 0.13 (probabilities 0.42 no material issue, 0.35 insufficient evidence, 0.23 investigate). This weak advisory is not a review pass or native qualification. Packaging, compatible rollout and arbitrary import execution remain outstanding. The uncertain real bag move remains untouched.

## Creation package — 2026-11-04

Retail 0.15.0-dev contains 29 files, adding TalentCreate.lua and TalentCreateImport.lua after their metadata helpers and before shared dispatch. Creation preparation now advertises execution support with minimum addon 0.15.0-dev; game_execution and live_qualified remain false. Existing metadata operations retain minimum 0.14.0-dev. Read-only arbitrary import inspection/planning helpers are still source-only.

Artifact: `dist/talent-create-release/AzerothOperator-0.15.0-dev-retail-live.zip`, SHA-256 `82f9c5bf1d248a15acad78685b8ae703038bafdc211a40e7f43d5ba7b44cd704`. The staged verifier loads actual packaged files, compiles creation data, exercises Review/Apply with synthetic native APIs, asserts the pre-write unknown receipt and replay refusal, exports through Core and verifies the imported applied receipt. Existing metadata/equipment fixtures also pass. The guarded installer and integrity verifier accept the new file set while preserving legacy sets; a 27→29 fixture verifies backup preservation. No live install or game action occurred.

Full source gate passed Rust/Lua tests, Clippy and 37 Python tests; nine packaging/installer tests passed again after the importer qualification fixture addition. The Linux importer fixture now includes an applied creation receipt, but actual container qualification is pending the compatible rollout. Current hosted services and installed addon remain the previous release.

Jev advisory `jev-1.14.0`, request `95183f5f71ed261171d4e4e382e84dd4d13a6a1d1d211a2b5eb905c3e8fcb101`, 942 input/116 output tokens, 0.309 s: insufficient evidence. No AI review pass is claimed. Deterministic staged and protocol checks support package readiness only; deployment and native compatibility remain unproven. Configured adversarial/OCR files remain unavailable.

## Parsed import readback comparison — 2026-11-04

Source-only `TalentImportContentsMatch` compares a retained inspection with a native export's bounded parse. It requires exact working config/spec/tree/node IDs/tree hash/version and all parsed node flags/ranks/choices, then re-inspects to reject changed observations. The request's text and hash-omitted marker are deliberately excluded: an explicitly acknowledged zero-hash request can match the same contents exported with the actual hash. The readback itself must include the actual tree hash. Reference: [native import/export format and writer](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentImportExport.lua).

This helper performs no mutation, is not packaged or MCP-exposed, and is not a success receipt. Future import execution must retain its original plan privately, bind client/catalogue preimages, require unique new-build identity and native selection, and verify both active and saved contents plus preserved bystanders. Matching contents alone does not prove legality, successful import or selection.

Fixtures cover omitted-request/full-native hash equivalence, mandatory readback hash, each parsed field mismatch, all observation identity fields, changed node/hash and a tree race between reads. Full source gate passed Rust/Lua tests, Clippy and 37 Python tests. Parser dependencies remain mocked; native compatibility is unqualified. No game action or deployment.

Jev advisory jev-1.14.0, request `88d7f7eaa07bc639138264780fbcb637765718777c4043b751c59f9268fdf3db`, 903 input/116 output tokens, 0.287 s: insufficient evidence. No AI review pass claimed. Configured adversarial review file remains absent. Deterministic fixtures support the bounded comparison contract only.
