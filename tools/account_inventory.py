"""Private, bounded latest account inventory. CLI imports; MCP reads only."""
import argparse
from contextlib import closing
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import sqlite3
import tempfile

from shared.snapshots import SnapshotStore
from tools.sync_exports import absolute, ordinary

FORMAT = 0x415A4931
MAX_BYTES = 64 * 1024 * 1024
SCOPES = ('equipment', 'bags', 'character_bank', 'mail_attachments')


def encoded(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'), allow_nan=False)


def text(value, maximum=128):
    if (type(value) is not str or not value.strip() or len(value.encode()) > maximum
            or any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in value)):
        raise ValueError('Invalid identity')
    return value


def account(value):
    if type(value) is not str or not re.fullmatch(r'[A-Za-z0-9_-]{1,80}', value):
        raise ValueError('Explicit opaque local account key required')
    return value


def project(snapshot, snapshot_id):
    if snapshot['source'] != 'addon' or snapshot['client']['family'] != 'forever':
        raise ValueError('Only owner addon Forever captures are supported')
    client = {key: snapshot['client'][key] for key in ('family', 'variant', 'build', 'interface')}
    for key in ('family', 'variant', 'build'):
        text(client[key])
    character = {key: text(snapshot['character'][key]) for key in ('name', 'realm')}
    stamp = datetime.fromisoformat(snapshot['captured_at'].replace('Z', '+00:00')).astimezone(timezone.utc)
    result = dict(client=client, character=character, snapshot_id=snapshot_id,
                  captured_at=stamp.isoformat(), source='addon', storage={})
    faction = snapshot['sections'].get('stats', {}).get('faction')
    if faction in ('Alliance', 'Horde', 'Neutral'):
        result['stats'] = {'faction': faction}
    for scope in (*SCOPES, 'account_bank'):
        if scope not in snapshot['sections']:
            continue
        source = snapshot['sections'][scope]
        rows = []
        if scope == 'mail_attachments':
            result['storage'][scope] = dict(coverage=source['coverage'],
                loaded_messages=source['loaded_messages'], total_messages=source['total_messages'],
                unloaded_messages=source['total_messages'] - source['loaded_messages'], items=rows)
            for message, attachments in source['messages'].items():
                for slot, item in attachments.items():
                    rows.append(dict(message=message, slot=slot, **item))
            continue
        groups = {'equipment': source} if scope == 'equipment' else source
        for container, slots in groups.items():
            for slot, item in slots.items():
                row = dict(container=container, slot=slot, item_id=item['item_id'], count=item.get('count', 1))
                link = item.get('link', '')
                match = re.fullmatch(r'(?:\|c(?:[0-9a-fA-F]{8}|nIQ[0-9]+:))?\|H(item:[0-9:-]+)\|h[^\r\n]*\|h(?:\|r)?', link)
                payload = match[1] if match else link
                if re.fullmatch(r'item:[0-9]+(?::(?:-?[0-9]+)?)*', payload) and int(payload.split(':')[1]) == item['item_id']:
                    row['item_string'] = payload
                row['variant_status'] = 'exact_observed_item_string' if 'item_string' in row else 'unavailable'
                rows.append(row)
        result['storage'][scope] = {'items': rows}
    return result


def _identity(value):
    return tuple(value['client'][key].casefold() for key in ('family', 'variant')) + tuple(
        value['character'][key].casefold() for key in ('realm', 'name'))


def bank_observation(value):
    """Return an independent bank projection, including legacy current rows."""
    if 'character_bank' not in value['storage']:
        return None
    return {**{key: value[key] for key in
        ('client', 'character', 'snapshot_id', 'captured_at', 'source')},
        'storage': {'character_bank': value['storage']['character_bank']}}


class AccountInventory:
    """One owner-private database, explicit accounts; no shared service authority."""
    def __init__(self, database, max_bytes=MAX_BYTES):
        self.path = ordinary(absolute(database))
        if type(max_bytes) is not int or not 128 * 1024 <= max_bytes <= MAX_BYTES:
            raise ValueError('Database budget must be 128 KiB to 64 MiB')
        self.max_bytes = max_bytes

    def _paths(self):
        ordinary(self.path)
        size = self.path.stat().st_size if self.path.exists() else 0
        if self.path.exists() and (not self.path.is_file() or self.path.stat().st_nlink != 1
                                  or size > self.max_bytes):
            raise ValueError('Invalid private database file')
        for suffix in ('-journal', '-wal', '-shm'):
            companion = ordinary(Path(str(self.path) + suffix))
            if companion.exists() and (suffix != '-journal'
                    or not companion.is_file() or companion.stat().st_nlink != 1
                    or companion.stat().st_size + size > self.max_bytes):
                raise ValueError('Database has an active or unrecovered transaction')

    def _check(self, connection):
        if (connection.execute('PRAGMA application_id').fetchone()[0] != FORMAT
                or connection.execute('PRAGMA user_version').fetchone()[0] != 1
                or connection.execute('PRAGMA auto_vacuum').fetchone()[0] != 1
                or connection.execute('PRAGMA journal_mode').fetchone()[0] != 'delete'
                or connection.execute("SELECT name,type FROM sqlite_master ORDER BY name").fetchall()
                != [('characters', 'table'), ('shared_banks', 'table')]):
            raise ValueError('Not a private account inventory database')
        for table, columns in [('characters', ['account', 'family', 'variant', 'realm', 'name', 'stamp', 'snapshot', 'data']),
                               ('shared_banks', ['account', 'family', 'variant', 'stamp', 'snapshot', 'data'])]:
            if [row[1] for row in connection.execute('PRAGMA table_info(' + table + ')')] != columns:
                raise ValueError('Unexpected private inventory schema')

    def _connect(self, write=False):
        self._paths()
        # Check ownership before SQLite can recover a journal or modify any pages.
        with self.path.open('rb') as stream:
            header = stream.read(100)
        if (header[:16] != b'SQLite format 3\0' or len(header) != 100
                or int.from_bytes(header[68:72], 'big') != FORMAT
                or int.from_bytes(header[60:64], 'big') != 1):
            raise ValueError('Not a private account inventory database')
        mode = 'rw' if write else 'ro'
        conn = sqlite3.connect(self.path.as_uri() + '?mode=' + mode, uri=True, timeout=5)
        try:
            conn.execute('PRAGMA trusted_schema=OFF')
            conn.execute('PRAGMA temp_store=MEMORY')
            if write:
                conn.execute('PRAGMA secure_delete=ON')
            self._check(conn)
            if not write:
                conn.execute('PRAGMA query_only=ON')
            return conn
        except Exception:
            conn.close()
            raise

    def _create(self):
        if os.name != 'nt':
            raise ValueError('New private databases require Windows atomic no-replace rename')
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._paths()
        descriptor, temporary = tempfile.mkstemp(prefix='.inventory-schema-', dir=self.path.parent)
        os.close(descriptor)
        try:
            with closing(sqlite3.connect(temporary)) as conn:
                conn.execute('PRAGMA temp_store=MEMORY')
                conn.execute('PRAGMA secure_delete=ON')
                conn.execute('PRAGMA auto_vacuum=FULL')
                conn.execute('PRAGMA journal_mode=DELETE')
                conn.execute('PRAGMA application_id=' + str(FORMAT))
                conn.execute('PRAGMA user_version=1')
                conn.execute('CREATE TABLE characters(account TEXT,family TEXT,variant TEXT,realm TEXT,name TEXT,stamp TEXT,snapshot TEXT,data TEXT,PRIMARY KEY(account,family,variant,realm,name)) WITHOUT ROWID')
                conn.execute('CREATE TABLE shared_banks(account TEXT,family TEXT,variant TEXT,stamp TEXT,snapshot TEXT,data TEXT,PRIMARY KEY(account,family,variant)) WITHOUT ROWID')
                conn.commit()
            try:
                os.rename(temporary, self.path)  # Windows atomically moves without replacing a destination.
            except FileExistsError:
                pass  # A competing creator won; _connect checks that final database.
        finally:
            Path(temporary).unlink(missing_ok=True)
            Path(temporary + '-journal').unlink(missing_ok=True)

    def ingest(self, store, account_key, snapshot_id):
        account(account_key)
        source = ordinary(absolute(store))
        if not re.fullmatch(r'[a-f0-9]{64}', snapshot_id):
            raise ValueError('Invalid snapshot ID')
        ordinary(source / (snapshot_id + '.json'))
        value = project(SnapshotStore(source).read(snapshot_id), snapshot_id)
        identity = (account_key, *_identity(value))
        shared = value['storage'].pop('account_bank', None)
        if not self.path.exists():
            self._create()
        with closing(self._connect(write=True)) as conn:
            page = conn.execute('PRAGMA page_size').fetchone()[0]
            # Three file budgets plus 64 KiB headroom bound DB and rollback journal peak.
            pages = (self.max_bytes - 65536) // (3 * page)
            if conn.execute('PRAGMA page_count').fetchone()[0] > pages:
                raise ValueError('Database exceeds transaction budget')
            conn.execute('PRAGMA max_page_count=' + str(pages))
            with conn:
                conn.execute('BEGIN IMMEDIATE')
                old = conn.execute('SELECT stamp,snapshot,data FROM characters WHERE account=? AND family=? AND variant=? AND realm=? AND name=?', identity).fetchone()
                character_status = self._order(old[:2] if old else None, value)
                previous = json.loads(old[2]) if old else None
                retained = (previous.get('last_observed_character_bank') or bank_observation(previous)) if previous else None
                observed = bank_observation(value)
                character_bank_status = 'unobserved'
                if observed is not None:
                    prior = (retained['captured_at'], retained['snapshot_id']) if retained else None
                    character_bank_status = self._order(prior, observed)
                    if character_bank_status == 'updated':
                        retained = observed
                if 'updated' in (character_status, character_bank_status):
                    if old is None and conn.execute('SELECT count(*) FROM characters').fetchone()[0] >= 1024:
                        raise ValueError('Character capacity reached')
                    # Keep retained provenance out of the incoming shared-bank projection.
                    saved = dict(value if character_status == 'updated' else previous)
                    if retained is not None:
                        saved['last_observed_character_bank'] = retained
                    conn.execute('INSERT OR REPLACE INTO characters VALUES(?,?,?,?,?,?,?,?)',
                                 (*identity, saved['captured_at'], saved['snapshot_id'], encoded(saved)))
                bank_status = 'unobserved'
                if shared is not None:
                    bank = dict(value, storage={'account_bank': shared})
                    prior = conn.execute('SELECT stamp,snapshot FROM shared_banks WHERE account=? AND family=? AND variant=?', identity[:3]).fetchone()
                    bank_status = self._order(prior, bank)
                    if bank_status == 'updated':
                        conn.execute('INSERT OR REPLACE INTO shared_banks VALUES(?,?,?,?,?,?)',
                                     (*identity[:3], value['captured_at'], snapshot_id, encoded(bank)))
            return {'status': 'updated' if 'updated' in (character_status, character_bank_status, bank_status) else character_status,
                    'character_status': character_status, 'character_bank_status': character_bank_status,
                    'account_bank_status': bank_status, 'snapshot_id': snapshot_id}

    @staticmethod
    def _order(old, value):
        if old is None:
            return 'updated'
        if old[1] == value['snapshot_id']:
            return 'unchanged'
        if value['captured_at'] == old[0]:
            raise ValueError('Ambiguous equal capture timestamp')
        return 'ignored_older' if value['captured_at'] < old[0] else 'updated'

    def query(self, account_key, item_id=None, offset=0, limit=50):
        account(account_key)
        if (item_id is not None and (type(item_id) is not int or not 1 <= item_id <= 4294967295)
                or type(offset) is not int or not 0 <= offset <= 1024
                or type(limit) is not int or not 1 <= limit <= 100):
            raise ValueError('Invalid private inventory query')
        with closing(self._connect()) as conn:
            # One transaction gives both tables the same read snapshot.
            conn.execute('BEGIN')
            characters = [json.loads(row[0]) for row in conn.execute(
                'SELECT data FROM characters WHERE account=? ORDER BY family,variant,realm,name', (account_key,))]
            banks = [json.loads(row[0]) for row in conn.execute(
                'SELECT data FROM shared_banks WHERE account=? ORDER BY family,variant', (account_key,))]
        result = dict(account_key=account_key, live=False, account_identity_verified=False,
            all_characters_covered=False, item_variant_equivalence_verified=False, full_bis_claim=False, transfers_reconciled=False,
            mail_is_available_stock=False, character_count=len(characters), offset=offset,
            has_more=offset + limit < len(characters),
            characters=[self._view(row, item_id, SCOPES) for row in characters[offset:offset + limit]],
            shared_banks=[self._view(row, item_id, ('account_bank',)) for row in banks])
        if len(encoded(result).encode()) > 256 * 1024:
            raise ValueError('Response exceeds limit; use a smaller character page')
        return result

    @staticmethod
    def _view(value, item_id, scopes):
        result = {key: value[key] for key in ('client', 'character', 'snapshot_id', 'captured_at', 'source')}
        if 'stats' in value:
            result['stats'] = value['stats']
        result['age_seconds'] = max(0, round((datetime.now(timezone.utc) - datetime.fromisoformat(value['captured_at'])).total_seconds()))
        if 'character_bank' in scopes:
            bank = value.get('last_observed_character_bank') or bank_observation(value)
            result['last_observed_character_bank'] = None
            if bank is not None:
                items = [item for item in bank['storage']['character_bank']['items']
                         if item_id is None or item.get('item_id') == item_id]
                result['last_observed_character_bank'] = {
                    **{key: bank[key] for key in ('client', 'character', 'snapshot_id', 'captured_at', 'source')},
                    'age_seconds': max(0, round((datetime.now(timezone.utc) - datetime.fromisoformat(bank['captured_at'])).total_seconds())),
                    'current': False, 'available_stock': False,
                    'observed_in_latest_capture': bank['snapshot_id'] == value['snapshot_id'],
                    'matches_latest_client_metadata': bank['client'] == value['client'],
                    'observed_quantity': sum(item['count'] for item in items),
                    'observed_item_locations': len(items),
                    **({'observed_matches': items} if item_id is not None else {})}
        result['storage'] = {}
        for scope in scopes:
            saved = value['storage'].get(scope)
            row = {'observed': saved is not None, 'ownership': 'account_shared' if scope == 'account_bank' else 'recipient_character' if scope == 'mail_attachments' else 'character'}
            result['storage'][scope] = row
            row['quantity'] = None
            if saved is not None:
                items = [item for item in saved['items'] if item_id is None or item.get('item_id') == item_id]
                row['quantity'] = sum(item['count'] for item in items if 'item_id' in item)
                row['item_locations'] = len(items)
                if item_id is not None:
                    row['matches'] = items
                if scope == 'mail_attachments':
                    row.update({key: val for key, val in saved.items() if key != 'items'})
                    row['unknown_attachments'] = sum(item.get('kind') == 'unknown' for item in saved['items'])
                    row['available_stock'] = False
        return result


def serve(inventory):
    import asyncio
    from mcp.server.lowlevel import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import Tool, ToolAnnotations
    server = Server('Azeroth Private Account Inventory')
    annotations = ToolAnnotations(readOnlyHint=True, destructiveHint=False, openWorldHint=False)
    properties = {'account_key': {'type': 'string', 'pattern': '^[A-Za-z0-9_-]{1,80}$'},
                  'offset': {'type': 'integer', 'minimum': 0, 'maximum': 1024, 'default': 0},
                  'limit': {'type': 'integer', 'minimum': 1, 'maximum': 100, 'default': 50}}
    descriptions = {
        'get_private_account_inventory': 'List latest character capture/storage counts and latest observed shared-bank provenance. Missing current storage is unknown. last_observed_character_bank is separate historical evidence, never current or available stock; never add it to current storage. Local account labels are not verified identity. Pages contain characters; shared banks repeat as context and must never be added across pages. No live, transfer or delivery claim.',
        'find_private_account_item': 'Find a base item ID on each character page and account bank with locations and exact observed numeric item strings. last_observed_character_bank has separate observed counts and provenance, never current or available stock; never add it to current storage. Keep owners and mail separate; variants are not equivalent. Shared banks repeat across pages; never sum them. Mail is not available stock or proof of delivery. No full gear/BiS claim.'}

    @server.list_tools()
    async def list_tools():
        return [Tool(name=name, description=description, annotations=annotations, inputSchema={
            'type': 'object', 'additionalProperties': False,
            'properties': {**properties, **({'item_id': {'type': 'integer', 'minimum': 1, 'maximum': 4294967295}} if name.startswith('find_') else {})},
            'required': ['account_key', 'item_id'] if name.startswith('find_') else ['account_key']})
            for name, description in descriptions.items()]

    @server.call_tool()
    async def call_tool(name, arguments):
        if name not in descriptions:
            raise ValueError('Unknown private inventory tool')
        try:
            return await asyncio.to_thread(inventory.query, **arguments)
        except (OSError, ValueError, sqlite3.Error, KeyError, TypeError):
            raise ValueError('Private account inventory unavailable or query invalid') from None

    async def run():
        async with stdio_server() as (read, write):
            await server.run(read, write, server.create_initialization_options())
    asyncio.run(run())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('ingest', 'get', 'find', 'serve'))
    parser.add_argument('--database', type=Path, required=True)
    parser.add_argument('--max-bytes', type=int, default=MAX_BYTES)
    parser.add_argument('--store', type=Path)
    parser.add_argument('--account-key')
    parser.add_argument('--snapshot-id')
    parser.add_argument('--item-id', type=int)
    parser.add_argument('--offset', type=int, default=0)
    parser.add_argument('--limit', type=int, default=50)
    args = parser.parse_args()
    inventory = AccountInventory(args.database, args.max_bytes)
    if args.action == 'serve':
        serve(inventory)
    elif args.action == 'ingest':
        if not args.store or not args.snapshot_id:
            parser.error('ingest requires --store and --snapshot-id')
        print(encoded(inventory.ingest(args.store, args.account_key, args.snapshot_id)))
    else:
        if args.action == 'find' and args.item_id is None:
            parser.error('find requires --item-id')
        print(encoded(inventory.query(args.account_key, args.item_id if args.action == 'find' else None, args.offset, args.limit)))


if __name__ == '__main__':
    main()
