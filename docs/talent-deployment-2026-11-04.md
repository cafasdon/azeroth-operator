# Talent deployment — 2026-11-04

Source: PR #5, merged main 5d17fd071ab849e070ea8287258a808ce1c30812; build source 0f98584 (identical tree).

Personal pylon / CT290 mcp-gateway identity checked before upgrade. Importer and reader now pin sha256:bf9899e1851b74b64d375f9d525ace1fda25cb8c6af5148d7441720768cbb452. Linux release tests/build passed. Importer legacy/extended fixture checks passed, including talent catalogue and uncertainty receipts. Reader profile upgrade, loopback auth/MCP probe and independent public TLS SDK discovery passed (12 tools). Reader rollback evidence: /var/tmp/azeroth-reader-upgrade-hi0ur3ff.

Windows release companion SHA256 d33fac2602d0faf93fc6abbcce14f083a9a4dcb2846cccfb26980d35a6a866f0 installed in the content-addressed runtime directory; sync configuration backed up as config-before-talents.json. Real saved export upload succeeded with the new companion and importer.

Retail addon 0.7.0-dev installed: 15 files independently matched package manifest. ZIP SHA256 213b95cba75ff101320bc3e88ecb57aae0738c71d4b15187cc409bf859b89f1d. Previous installation backed up under local-data/addon-backups/addon-update-er1bcj_d. Loading the new addon in-game and native talent switching remain unverified. No game input was generated.

## Bag test evidence and next investigation

Player screenshot reports outcome_unknown for operation d3c1b9f2df97c5b848d1a2bcc1ee08d7cb75d292d77cfd7b93331eb1fc0f3fb6. Subsequent saved export 184cd8a9ffd714eb7a5f33f4b661ba15e542c117fd483acef5b65e041b15f3c8 was uploaded and independently read through the public MCP. The exact source item remains recorded at backpack slot 1; slot 13 is absent; the operation receipt is move_bag_item/outcome_unknown. Private readback: local-data/live-qualification/bag-unknown.json. Cursor state was not captured. This does not establish the failure stage or a completed move. Do not replay the uncertain operation.

The player deferred further in-game qualification until tomorrow. Continue source development and offline checks without further game-test prompts. Predictions and fixture success must remain distinct from live qualification.

Code investigation: BagOperations currently requires source GUID absence immediately after pickup and synchronous final inventory state. Those assumptions need explicit retained-source and delayed-update fixtures. Blizzard's native EquipmentManager uses cursor checks and lock tracking, not source-slot disappearance between pickup and placement. C_Cursor.GetCursorItem exposes an ItemLocation, potentially suitable for exact cursor identity; actual availability/semantics require bounded checks. These are investigation leads, not a confirmed root cause.

Primary sources read 2026-11-04:
- https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_APIDocumentationGenerated/CursorDocumentation.lua
- https://raw.githubusercontent.com/Gethe/wow-ui-source/live/Interface/AddOns/Blizzard_FrameXML/Mainline/EquipmentManager.lua
