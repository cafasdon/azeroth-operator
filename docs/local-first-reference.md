# Local reference from owner captures

`tools.forever_local` builds one private SQLite reference from one exact,
hash-validated Forever addon capture. It projects item labels and observed
variants, fixed recipe inputs/outputs, spell identities and skill names.
Recipe edges support traversal in both directions: recipe to ingredient/output
or item to recipes that consume/produce it. It makes no website or game call.

The single-capture scope is deliberate. Public reference knowledge and private
account inventory are separate stores. This index does not combine characters,
establish ownership quantities or claim a complete game database. Missing
recipes, unknown locations and unresolved requirements remain unknown.
Existing external world/catalogue caches remain useful for those gaps.

## Commands

Use an existing owner snapshot hash, not a path supplied through an MCP tool.

```powershell
./.venv/Scripts/python.exe -B -m tools.forever_local build --store "ABSOLUTE_OWNER_SNAPSHOT_STORE" --snapshot-id SNAPSHOT_SHA256 --database "ABSOLUTE_DATABASE_PATH"
./.venv/Scripts/python.exe -B -m tools.forever_local get --database "ABSOLUTE_DATABASE_PATH" --kind recipe --entity-id 1229432
./.venv/Scripts/python.exe -B -m tools.forever_local search --database "ABSOLUTE_DATABASE_PATH" --kind item --query "Leather"
./.venv/Scripts/python.exe -B -m tools.forever_local serve --database "ABSOLUTE_DATABASE_PATH"
```

The local Codex entry `azeroth-local` launches the last route with absolute
interpreter/database paths and an explicit project import path. It enables
only `search_local_forever_records` and `get_local_forever_record`. Both are
read-only; neither accepts a filesystem path or SQL. Existing app connections
can need a reconnect to discover these tools.

## Boundaries and retention

Only the configured owner's ordinary, validated addon snapshot can build the
index. Character/realm fields, inventory quantities, GUIDs, raw item links,
wallets, macros and unrelated sections are not projected. This is positive
field selection, not a general text privacy scrubber: a fabricated snapshot
could put arbitrary text in an otherwise allowed label or build field. The
database stays private and is not a public export or redistribution grant.

Different item suffix names remain `observed_names`, not one invented canonical
base name. Different observed variant characteristics stay separate. Neither
item-ID equality nor this projection proves interchangeable owned items.

The active database and staging database each have a 16 MiB ceiling. SQLite's
page quota applies before staging writes, and temporary SQLite data stays in
memory. Rebuilds atomically replace the active index after validation; there
is no version history. Active plus staging data therefore has a bounded
roughly 32 MiB file footprint, plus small transaction metadata. A failed build
keeps the active file and removes its staged files. Original owner snapshots
belong to their existing separate store and are not deleted by this builder.

## Installed evidence

The real build-70938 capture produced 153 records: 57 items, 20 recipes,
52 spells and 24 skills, with 81 recipe edges. The saved Codex configuration
launched successfully through an independent SDK client from a different
working directory. Both tools were discovered. Camp Tent's five-Light-Leather
edge, the reverse recipe link, item-name search and Leatherworking lookup passed.
The cold process plus four calls took 3.564 seconds; this is pilot evidence,
not a full-world performance benchmark.

Twelve focused tests passed. Independent caller review found a normal
random-suffix-name conflict; the fix and recheck passed. Independent privacy
review found no medium/high issue and identified the field-selection limit
stated above. A final storage check confirmed SQLite's actual page quota and
unchanged active bytes on an oversized insert. Its trivial test weakness was
removed and all 12 tests passed again. No pipeline ran.
