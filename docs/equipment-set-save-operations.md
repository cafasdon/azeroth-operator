# Equipment-set creation and saving

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
