# Equipment-set creation and saving

## MCP preparation — source only, 2026-11-04

`prepare_equipment_set_operation` now accepts `operation: create` or `save` in source. Create requires a new name/icon and forbids `set_id`; save requires a captured ID, preserves its name and defaults to its icon unless explicitly replaced. All previous operation kinds still require an ID, including ID zero. Their existing wire formats remain unchanged.

Both new kinds require an explicit `ignored_slots` list (unique positions 1–19; an empty list is valid when all slots are occupied). Every uncaptured equipped slot additionally needs `expect_empty_slots`; conflicting, duplicate, out-of-range or nonignored-empty requests are refused. No absent slot is inferred empty. Captured item IDs are positive u32 values validated by the snapshot schema; GUIDs must be available and distinct. The executor must verify explicit empty preconditions against native reads later.

The bounded `equipment_set_save_draft_v1` result contains the complete catalogue, nineteen GUID/item-ID preimages, nineteen ignored flags, desired slot descriptions, spec association to preserve, and whether saved contents will be overwritten. Its deterministic operation ID binds the draft and snapshot. Creation accepts an empty catalogue, but native capacity/name checks remain necessary at Apply. ASCII case-insensitive captured name collisions are refused; this is not a substitute for native name lookup.

These are non-executable drafts: `import_code: null`, `addon_execution_enabled: false`, no required addon version, and `draft_transport_pending`. The hosted service and installed addon remain unchanged. Next work is strict transport/decoder, then player review/receipts and packaging; nonignored empty-slot semantics remain outstanding.

Verification: full `python -m tools.verify_source` passed format, locked build, all Rust/Lua tests, Clippy and 34 Python tests. The actual stdio MCP prepares both kinds, verifies disabled execution and exact GUID data, rejects incomplete requests and preserves snapshot-store bytes. Unit fixtures cover empty catalogue, ID zero, save metadata preservation, malformed lists, collision/rename refusal, missing identities and hashes changing with equipment or bystander catalogue edits. Existing metadata/use/spec transport fixtures remain green.

Advisory Jev review: `jev-1.14.0`, request `2e05eae3e49cf5f24b50e0f4991494a24dbae59600f1c620de94fda606645e20`, 3459 input/150 output tokens, 0.310 seconds, `no_material_issue` with reported confidence 0.33. Advisory only; direct source/protocol fixtures are the bounded evidence, not native qualification. The configured adversarial-review rule remains absent; the OCR skill was previously checked and is unavailable.

## Create/save executor — source only, 2026-11-04

`EquipmentSetSave.lua` adds source-only `create_equipment_set` and `save_equipment_set` executors. They remain excluded from transport, shared dispatch and packaging. Requests bind client/build/interface/character, full catalogue, nineteen equipped GUID/item-ID pairs and nineteen explicit ignored flags. Create requires an unused name and available native capacity. Save requires the captured set ID/name and preserves its spec association; renaming is a separate operation.

Before a save, the executor checks combat/casting/channel/cursor, equipped locks, full catalogue and repeated equipped identities, name lookup, and the native editor/popup state. It repeats these checks after temporary flag setup, immediately before exactly one `CreateEquipmentSet` or `SaveEquipmentSet` call. It does not equip items, retry or delete/repair a partial result. The outer caller must checkpoint uncertainty before entry. Exceptions propagate after the helper attempts flag restoration, so a future shared caller must retain unknown and must not enrol a thrown call for automatic reconciliation.

Readback requires exactly one new set for create (ID zero is valid), or the original set for save. All other catalogue entries must remain identical, with no extra/deleted bystanders. Name/icon/spec, ignored flags, every nonignored equipped item GUID/ID/location and aggregate item/ignored/lost counts must match; current equipment must still match all nineteen preimages. False native return values cannot override successful readback; true values cannot prove completion. Already-matching save is a no-op. A read-only verifier supports later observation of a normally returned delayed result.

Ignored empty equipment slots are supported. Nonignored empty slots are explicitly refused until native saved-empty semantics are established; no implicit ignore conversion or guessed sentinel is used. Set counts and observed locations are conservative verification requirements, not live-qualified semantics or a claim to expose internal persistent set GUIDs. Native event timing, complete UI exclusion and same-value edits by other addons remain unqualified. This is an implementation stage towards complete create/save support, not completion of that scope.

Native create/name collision/capacity behaviour was checked in [GearManagerPopupFrameMixin:OkayButton_OnClick](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_UIPanels_Game/Mainline/PaperDollFrame.lua); method signatures remain in the API reference below. No private-server implementation is used as evidence of Retail behaviour.

Actual Lua 5.1 executor/helper fixtures cover both operation kinds, new ID zero, no-op save, complete result verification, changed/missing/new bystanders, wrong GUID/spec/count/ignore flags, changed equipped state, cursor occupancy, delayed readback, native exceptions before/after effects, setup races, visible editor/dialog, capacity/name collisions, missing methods, stale/client/lock refusals and unresolved nonignored empties. Native game tests remain deferred; no installed package or service changes in this checkpoint.

Full source gate passed: `python -m tools.verify_source` (format, locked build, all Rust/Lua tests, Clippy with warnings denied, 34 Python tests). Advisory Jev review: `jev-1.14.0`, request `b9121dc2d935c9ff451cfbd2267ab57613d823b97ecfbe4243d239447a914ba6`, 3515 input/154 output tokens, 0.307 seconds; `insufficient_evidence`, reported confidence 0.19. No passing review or native qualification is inferred. The configured adversarial rule remains absent; the previously checked OCR skill is also unavailable. Direct source/fixture verification is the evidence for this bounded checkpoint.

## Temporary save flags — 2026-11-04

`addon/EquipmentSetSaveState.lua` is an internal source-only prerequisite for native create/save executors. It is excluded from TOCs, packaging and wire dispatch. No new game operation is enabled or deployed.

The native API exposes `IsSlotIgnoredForSave`, `IgnoreSlotForSave` and `UnignoreSlotForSave`. The native PaperDoll caller maintains these flags separately from its visual `button.ignored` fields. References inspected on 2026-11-04:

- [Equipment manager API declarations](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/EquipmentManagerDocumentation.lua).
- [PaperDollFrame_ClearIgnoredSlots and PaperDollFrame_IgnoreSlotsForSet](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_UIPanels_Game/Mainline/PaperDollFrame.lua).

The helper requires nineteen explicit boolean flags and Retail. It copies the requested map, checks all native methods before writes, reads the original flags twice, changes only differing slots and verifies the complete requested state before calling a synchronous save callback exactly once. Callback results are not completion evidence. A reentrancy guard prevents nested saves. No bulk clear, retry, equipment operation or automatic job is used.

Cleanup is attempted for every touched slot, including a setter that throws after changing state. It restores only those slots, leaves unrelated slots untouched, and verifies the complete original map afterwards. Unreadable state is not blindly overwritten. Failed setup, callback exceptions, changed state or incomplete restoration throw a bounded generic error; a future outer executor must already have persisted an unknown receipt. Successful return establishes only temporary-state restoration, not saved-set contents.

Boolean flags provide no ownership/version token: another addon writing the same value to a touched slot cannot be detected. The future executor must exclude a visible competing equipment-manager/popup editor and refuse reentrancy, while acknowledging other-addon callbacks are not fully isolatable. Native synchronous flag consumption, callback timing and visual-state behaviour still need game qualification. Do not present this as a transaction or guaranteed rollback.

Lua 5.1 fixtures exercise successful restoration despite a false callback return, no-change flags, nested calls, caller-map mutation, native exceptions before/after effects, bystander edits, restoration failure with continued cleanup, unreadable cleanup, incomplete/out-of-range flags, changing initial reads, restricted booleans, missing methods and unsupported family. They execute the actual module through Rust/mlua.

Remaining: create/save executors with exact catalogue and equipped GUID preimages; explicit ignored/empty-slot semantics; name/capacity collision checks; verified new set identification; whole-catalogue readback; compiler/transport; review and receipts; packaging and hosted compatibility; deferred native qualification. Do not infer set-empty sentinel meanings from inventory-cache constants. The unresolved Cutlass operation remains unknown and must not be replayed.

Verification: `python -m tools.verify_source` passed formatting, locked build, all Rust/Lua fixtures, Clippy with warnings denied and 34 Python tests. No installed file or service changed. The configured OCR skill and adversarial-review rule files were checked and remain absent; neither workflow is claimed.

Advisory Jev review: `jev-1.14.0`, request `24ea7a37472bcd5fd3f7bafbc8e4d2b39b47197c62b7650fc53bfaf17d8869a9`, 1801 input/153 output tokens, 0.309 seconds; `investigate_supported_issue` (reported confidence 0.26). This supplies no concrete defect and is not a passing review. Direct inspection confirms all operations after acquiring the busy guard are inside protected calls until guard release; subsequent fixtures successfully invoke the helper after each exception. Restoration and same-value ownership limitations are expressly documented above. Native qualification remains pending.
