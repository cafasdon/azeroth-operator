# Private account inventory

`tools.account_inventory` keeps the latest owner addon capture for each Forever
character in a local SQLite database. The installed pilot uses
`local-data/owner-account-inventory/inventory.sqlite`. This directory is ignored
by Git. Names, realms and item locations stay in this private database, separate
from public reference caches. Do not publish it or expose its MCP through a
shared service.

An explicit opaque account label groups characters. It is not a verified game
account identity. Character identity includes account label, family, variant,
realm and name. Observed faction remains character metadata; it does not create
a second account. Queries never infer account membership from a realm or faction.

## Import and read

Imports use the CLI with explicit absolute database and snapshot-store paths:

```powershell
python -m tools.account_inventory ingest --database 'T:\Projects\Azeroth Operator\local-data\owner-account-inventory\inventory.sqlite' --store 'T:\Projects\Azeroth Operator\local-data\first-party-reference\owner-snapshots' --account-key pilot --snapshot-id <validated-snapshot-sha256>
```

The installed `azeroth-account` stdio MCP exposes only:

- `get_private_account_inventory(account_key, offset, limit)`
- `find_private_account_item(account_key, item_id, offset, limit)`

These tools read saved data. They accept no SQL, database paths or import
commands. A client may need to reconnect before new tools appear. Account labels
select records within the owner's private process; they are not authentication
or a boundary between mutually untrusted users.

## Ownership and uncertainty

Equipment, bags and character banks belong to their character. Mail attachments
belong to the recipient and never count as available stock. Missing storage is
unknown, not empty. A later character capture replaces that character's stored
view, including storage that is now unknown. It does not preserve an older bank
view as current inventory.

The latest observed account bank has separate provenance and appears once per
account, family and variant. Character pages repeat this shared context: callers
must not add it across pages. Captures at different times can describe the same
transferred item twice. Transfer and delivery reconciliation remain unavailable.
Exact observed numeric item strings are retained where available. A shared base
item ID does not prove equivalent variants, usable gear or best-in-slot status.

## Limits and recovery

The default database and transaction budget is 64 MiB. There are at most 1,024
characters and 256 KiB per response. Imports replace current rows rather than
retain version history. Source snapshot retention is separate and is not covered
by this budget. The importer uses SQLite page limits, secure deletion and full
auto-vacuum. New database creation is supported on Windows only and fails closed
on other platforms. Existing databases use ordinary SQLite DELETE journals.
Never delete a journal manually. A hot journal needs a normal write import for
SQLite recovery; a cold journal does not prevent a read.

## Acceptance, 15 November 2026

The focused suite contains 24 tests. Three bounded review rounds covered callers,
failure recovery, privacy and data loss. Fixes closed one high-severity creation
race and medium-severity shared-bank ordering, journal recovery, publication
recovery and modern item-link parsing faults. The final checks left no open
medium or high finding. No infrastructure deployment changed.

The actual private pilot imported one Alliance character. An independent-cwd
stdio SDK caller found both strict read-only tools, one pilot character and zero
characters under another account label. Item 4253 appeared once in equipment;
the unobserved character bank remained unknown. Mail remained unavailable stock.
Multiple accounts, factions and shared-bank deduplication have synthetic test
coverage; native alternate-character captures remain untested. No game action
or external data download was necessary.
