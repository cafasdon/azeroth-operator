# Player-applied equipment operations

Development status, 2026-11-03: implemented in source, not deployed or live-qualified.

`prepare_equipment_operation` prepares one carried-bag item for an equipment slot. It binds the snapshot, client, character, source item GUID and destination item GUID. A missing destination requires explicit `expect_empty_target`; missing observations are not evidence of an empty slot. The player pastes the returned code into `/aoapply`, reviews it and clicks Apply.

The shared dispatcher records `outcome_unknown` before invoking the executor. It prevents replay of the same operation ID and requires a new review after input changes. SavedVariables persistence requires reload/logout. A different snapshot can produce a different ID; this is not a guarantee against semantic duplicates.

The executor checks combat, casting, locks, cursor ownership and current item identities. It returns the displaced item to the original bag slot. Two-handed items and multi-slot loadouts remain unsupported. Errors after pickup can leave an item on the cursor; automatic retries and cursor clearing are intentionally absent. Live asynchronous item updates remain unqualified.

Verification: source gate passed formatting, build, Rust tests, Clippy and 31 Python checks. The transport/shared dispatcher fixture additionally verifies pre-execution receipts, explicit review and duplicate refusal. The eleven-file Retail 0.4.0-dev package passed the staged Lua 5.1 export roundtrip and interface-change rejection. Archive SHA256: `1f2f078810df5745507c6626d8257d59cbd49e23a07f36da73ec73eb5ea41b2f`.

Next: review partial-effect cases and MCP deployment fixtures, upgrade hosted reader/importer and local companion for GUID fields, then install the addon and qualify a player-applied swap. Classic and Forever mutations remain unqualified.

Additional verification: explicit empty destination, equip refusal and failed displaced-item return tests pass. The shared dispatcher retains outcome_unknown on executor errors and refuses replay. The isolated importer deployment fixture now includes item GUIDs and an equip_bag_item receipt.

Advisory review: Jev 1.14.0, request 703d744a116fcbb73a5b00db6e8c5096d8568519ac65c823fdeabaee4f91ad32, 862 input/116 output tokens, 0.311 seconds. Result: insufficient_evidence (reported confidence 0.38), not a review pass. Source checks establish fixture behaviour only; live timing and deployment remain outstanding. The configured OCR skill and adversarial-review file were unavailable; those workflows were not run.
