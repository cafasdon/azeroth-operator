# Base inventory structure parity — 9 November 2026

## Finding and intended result

The offline route allowlist in src/main.rs remains restricted; no capability
or policy mode change is needed for this repair. During the audit, Python
validation accepted a bag item with an unsupported_private_field, while the
unchanged Rust Item contract uses deny_unknown_fields. It also skipped malformed
inventory maps and item identity checks. Baseline: 8a705da.

Align Python base inventory admission with the existing Rust reference:

- Inventory sections and slot groups are objects, with canonical ASCII numeric
  keys. Equipment slots are 1..19, container slots 1..1024 and containers 0..64.
- Item objects have only the eight existing fields: item_id, link, count, guid,
  required_level, sell_price_copper, equip_location and native_stats.
- Item IDs are positive u32 integers. Optional link/GUID nulls remain supported;
  nonnull links are strings of at most 4096 UTF-8 bytes. Nonnull GUIDs are
  nonempty strings of at most 128 UTF-8 bytes without Unicode control characters.
- Existing count, required-level, native-stat/category and price guards remain.
- Present bank sections require Forever and container IDs cannot repeat across
  carried bags and either bank section, including otherwise empty containers.

No coercion, guessed item/count, source rewrite or deletion. Ingestion rejects
before writing; reads refuse incompatible existing captures without altering
their bytes. This prevents unsupported item fields from being retained as
accepted captures; it is not a guarantee that all permitted strings are free of
private content or that every snapshot section has full reader equivalence.

## Author evidence

- Red: `python -m unittest test_inventory_item_shape_parity.InventoryItemShapeParityTests.test_unsupported_item_field_rejected_before_writes`:
  one failure before the fix, `AssertionError: ValueError not raised`.
- With `PYTHONPATH=.;tests`,
  `python -m unittest test_inventory_item_shape_parity test_item_stack_count_parity test_snapshots test_bag_item_cooldowns test_item_cooldown_lookup test_bag_cooldown_filter`:
  13 tests, 5.324s, OK, initially 229 structure cases.
- After adding Unicode byte boundaries, maximum item ID and existing-file read
  rejection: `python -m unittest test_inventory_item_shape_parity`:
  two tests, 2.482s, OK, 241 actual local MCP structure cases.
- `python -m unittest test_native_item_stats test_native_equip_location test_item_sell_price`:
  three existing metadata/MCP tests, 1.382s, OK.

The 241-case matrix spans all four storages, malformed section/group/item shapes,
missing/invalid IDs, canonical key bounds, unknown item fields, optional nulls,
ASCII/multibyte limits, control versus valid format characters, required-level
types, bank client restrictions and container collisions. It compares Python
admission against the existing Rust stdio reader; no Rust rebuild or schema
relaxation occurred. Rejected ingestion preserves existing accepted files; a
manually placed invalid test capture is refused on read with bytes unchanged.
Errors do not include supplied private marker values.

All captures are synthetic temporary owner-local fixtures. No game, private
character, external data, addon change, installation, deployment or data cleanup.
The supported input contract narrows to what the Rust MCP already accepts.

## Independent review

One clean round, gpt-6-astra high Callers and Failure/privacy. Callers ran 10
structure/count/snapshot tests (3.846s), three cooldown tests (.971s), and two
native-stat/price tests (.529s), all OK. Failure ran two structure tests (2.460s,
241 cases) and two count tests (1.231s), all OK. Both traced the unchanged Rust
contract, validation before writes and existing-read rejection. No concrete
fault found. Native/deployment lens omitted because no such operation occurs;
agent token usage unavailable.

## Plan advisory

Jev 1.14.0 request
`bb18f531e7039c086eebe15ab61be91eab0f1a1895fd5aba78fe34f6c7c775e3`:
proceed .85 / revise .10 / insufficient .05; 683 input, 47 output, .287s.
Advisory only; the red input and real reader comparisons establish this repair.

Final Jev 1.14.0 advisory request
`ac54a1e5e370e97499941149d4ff9225e467eb3ac7b42a902ecc99a3a8b10269`:
land .66 / fix .09 / insufficient .25; 626 input, 47 output, .302s.
Author disposition: land the focused inventory repair on the original red
input, actual Rust admission matrix and two clean independent reviews. The
advisory is retained unchanged. Full schema, native, ownership and broader MCP
coverage requirements remain open; live testing stays deferred.
