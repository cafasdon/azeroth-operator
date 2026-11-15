"""Private inventory separation, retention, transaction bounds and real MCP callers."""
from concurrent.futures import ThreadPoolExecutor
from contextlib import closing
from unittest.mock import patch
from copy import deepcopy
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import threading
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from tools.account_inventory import AccountInventory

ROOT = Path(__file__).resolve().parents[1]


def sample(name='Alice', day=1, count=2, faction='Alliance'):
    return {'schema_version': 1, 'source': 'addon', 'captured_at': f'2025-01-{day:02d}T00:00:00Z',
            'client': {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001},
            'character': {'name': name, 'realm': 'Test'},
            'sections': {'stats': {'faction': faction}, 'equipment': {},
                         'bags': {'0': {'1': {'item_id': 10, 'count': count,
                            'link': '|cffa335ee|Hitem:10:0:0:0:0:0:-17:42|h[PRIVATE NAME]|h|r'}}},
                         'character_bank': {'6': {'1': {'item_id': 10, 'count': 9}}},
                         'account_bank': {'13': {'1': {'item_id': 10, 'count': 100}}}}}


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=ROOT / 'local-data')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.store = SnapshotStore(self.root / 'owner')
        self.db = self.root / 'private.sqlite'
        self.inventory = AccountInventory(self.db)

    def ingest(self, value, account='accountA', inventory=None):
        key = self.store.ingest(value)
        return (inventory or self.inventory).ingest(self.store.root, account, key)

    def test_multiple_characters_factions_accounts_patch_preserves_others(self):
        self.ingest(sample())
        self.ingest(sample('Bob', 2, 7, 'Horde'))
        self.ingest(sample('Alice', 1, 88), 'accountB')
        update = sample('ALICE', 3, 3)
        update['client']['build'] = '70863'
        update['sections'].pop('account_bank')
        self.ingest(update)
        result = self.inventory.query('accountA', 10)
        self.assertEqual(result['character_count'], 2)
        alice, bob = result['characters']
        self.assertEqual(alice['storage']['bags']['quantity'], 3)
        self.assertEqual(bob['stats']['faction'], 'Horde')
        self.assertEqual(bob['storage']['bags']['quantity'], 7)
        self.assertEqual(self.inventory.query('accountB', 10)['characters'][0]['storage']['bags']['quantity'], 88)
        self.assertEqual(result['shared_banks'][0]['character']['name'], 'Bob')
        self.assertEqual(len(result['shared_banks']), 1)
        self.assertEqual(result['shared_banks'][0]['storage']['account_bank']['quantity'], 100)
        self.assertFalse(result['item_variant_equivalence_verified'])
        self.assertFalse(result['transfers_reconciled'])

    def test_shared_bank_missing_does_not_erase_and_observed_empty_does(self):
        self.ingest(sample())
        update = sample(day=2)
        update['sections'].pop('account_bank')
        update['sections'].pop('character_bank')
        update['sections']['bags'] = {}
        self.ingest(update)
        result = self.inventory.query('accountA', 10)
        row = result['characters'][0]
        self.assertIsNone(row['storage']['character_bank']['quantity'])
        self.assertEqual(row['storage']['bags']['quantity'], 0)
        bank = result['shared_banks'][0]
        self.assertIn('2025-01-01', bank['captured_at'])
        self.assertGreater(bank['age_seconds'], 0)
        empty = sample('Bob', 3)
        empty['sections']['account_bank'] = {}
        self.ingest(empty)
        bank = self.inventory.query('accountA', 10)['shared_banks'][0]
        self.assertEqual(bank['storage']['account_bank']['quantity'], 0)
        self.assertEqual(bank['storage']['account_bank']['ownership'], 'account_shared')
        self.assertEqual(self.inventory.query('accountA')['characters'][1]['storage']['character_bank']['ownership'], 'character')

    def test_stale_idempotence_and_tie_atomicity(self):
        current = sample(day=2)
        self.assertEqual(self.ingest(current)['status'], 'updated')
        self.assertEqual(self.ingest(current)['status'], 'unchanged')
        self.assertEqual(self.ingest(sample(day=1))['status'], 'ignored_older')
        before = self.inventory.query('accountA')
        with self.assertRaisesRegex(ValueError, 'Ambiguous'):
            self.ingest(sample(day=2, count=3))
        with self.assertRaisesRegex(ValueError, 'Ambiguous'):
            self.ingest(sample('Bob', day=2))
        self.assertEqual(self.inventory.query('accountA'), before)

    def test_mail_keeps_loaded_unknown_pending_separate_from_stock(self):
        value = sample()
        value['sections']['mail_attachments'] = {
            'coverage': 'loaded_inbox_attachments', 'loaded_messages': 1, 'total_messages': 3,
            'messages': {'1': {'1': {'kind': 'item', 'item_id': 10, 'count': 40},
                               '2': {'kind': 'unknown'}, '3': {'kind': 'currency', 'currency_id': 99, 'count': 4}}}}
        self.ingest(value)
        result = self.inventory.query('accountA', 10)
        storage = result['characters'][0]['storage']
        self.assertEqual(storage['bags']['quantity'], 2)
        self.assertEqual(storage['mail_attachments']['quantity'], 40)
        self.assertEqual(storage['mail_attachments']['unloaded_messages'], 2)
        self.assertEqual(storage['mail_attachments']['unknown_attachments'], 1)
        self.assertFalse(storage['mail_attachments']['available_stock'])
        self.assertEqual(storage['mail_attachments']['ownership'], 'recipient_character')
        self.assertFalse(result['mail_is_available_stock'])

    def test_numeric_variants_retained_private_fields_discarded(self):
        value = sample()
        value['sections']['wallet'] = {'copper': 123}
        value['sections']['macros'] = {'1': {'name': 'SECRET_MACRO', 'body': 'SECRET_BODY'}}
        self.ingest(value)
        match = self.inventory.query('accountA', 10)['characters'][0]['storage']['bags']['matches'][0]
        self.assertEqual(match['item_string'], 'item:10:0:0:0:0:0:-17:42')
        self.assertEqual(match['variant_status'], 'exact_observed_item_string')
        raw = self.db.read_bytes()
        for private in (b'PRIVATE NAME', b'SECRET_MACRO', b'SECRET_BODY', b'money_copper'):
            self.assertNotIn(private, raw)
        value = sample(day=2)
        value['sections']['bags']['0']['1']['link'] = '|Hitem:10:Player-1234-CREATOR|h[SECRET]|h'
        self.ingest(value)
        row = self.inventory.query('accountA', 10)['characters'][0]['storage']['bags']['matches'][0]
        self.assertNotIn('item_string', row)
        self.assertEqual(row['variant_status'], 'unavailable')
        self.assertNotIn(b'Player-1234', self.db.read_bytes())

    def test_variant_and_realm_partition_and_no_account_guess(self):
        self.ingest(sample())
        other = sample(day=2)
        other['client']['variant'] = 'test'
        self.ingest(other)
        other = sample(day=3)
        other['character']['realm'] = 'Other'
        self.ingest(other)
        self.assertEqual(self.inventory.query('accountA')['character_count'], 3)
        self.assertEqual(self.inventory.query('unknown')['character_count'], 0)
        for bad in (None, '', 'mail@example.com', '../account'):
            with self.assertRaises(ValueError):
                self.ingest(sample(), bad)
        for bad in (True, '10', 0, -1):
            with self.assertRaises(ValueError):
                self.inventory.query('accountA', bad)

    def test_size_failure_rolls_back_and_replacement_does_not_accumulate(self):
        small = AccountInventory(self.db, max_bytes=256 * 1024)
        self.ingest(sample(), inventory=small)
        before = small.query('accountA')
        large = sample(day=2)
        large['sections']['bags'] = {'0': {str(i): {'item_id': 10, 'count': 1, 'link': 'item:10:' + ':'.join(['12345'] * 150)} for i in range(1, 101)}}
        with self.assertRaises((sqlite3.DatabaseError, ValueError)):
            self.ingest(large, inventory=small)
        self.assertEqual(small.query('accountA'), before)
        self.assertLessEqual(sum(path.stat().st_size for path in self.root.glob('private.sqlite*')), 256 * 1024)
        for day in range(2, 25):
            self.ingest(sample(day=day), inventory=small)
        with closing(sqlite3.connect(self.db)) as conn:
            self.assertEqual(conn.execute('SELECT count(*) FROM characters').fetchone()[0], 1)
            self.assertEqual(conn.execute('SELECT count(*) FROM shared_banks').fetchone()[0], 1)
        self.assertLessEqual(self.db.stat().st_size, 32 * 1024)
        self.assertFalse(Path(str(self.db) + '-journal').exists())

    def test_unknown_database_bad_source_hash_and_path_rejected(self):
        self.db.write_bytes(b'NOT OUR DATABASE')
        journal = Path(str(self.db) + '-journal')
        journal.write_bytes(b'UNRELATED JOURNAL')
        with self.assertRaises((ValueError, sqlite3.DatabaseError)):
            self.ingest(sample())
        self.assertEqual(self.db.read_bytes(), b'NOT OUR DATABASE')
        self.assertEqual(journal.read_bytes(), b'UNRELATED JOURNAL')
        journal.unlink()
        self.db.unlink()
        wrong = sample(); wrong['source'] = 'fixture'
        with self.assertRaises(ValueError):
            self.ingest(wrong)
        self.assertFalse(self.db.exists())
        wrong = sample(); wrong['client']['family'] = 'retail'
        wrong['sections'].pop('character_bank'); wrong['sections'].pop('account_bank')
        with self.assertRaises(ValueError):
            self.ingest(wrong)
        key = self.store.ingest(sample())
        (self.store.root / (key + '.json')).write_text('{}')
        with self.assertRaises(ValueError):
            self.inventory.ingest(self.store.root, 'accountA', key)
        with self.assertRaises(ValueError):
            AccountInventory(Path('relative.sqlite'))
        with self.assertRaises(ValueError):
            self.inventory.ingest(self.store.root, 'accountA', '../escape')

    def test_first_create_race_preserves_competing_file(self):
        def competing_create():
            self.db.write_bytes(b'UNRELATED COMPETING FILE')
            raise FileExistsError('race')
        with patch.object(self.inventory, '_create', side_effect=competing_create):
            with self.assertRaises(FileExistsError):
                self.ingest(sample())
        self.assertEqual(self.db.read_bytes(), b'UNRELATED COMPETING FILE')

    def test_first_creator_failure_preserves_other_thread_commit(self):
        created = threading.Event()
        committed = threading.Event()
        first = AccountInventory(self.db, max_bytes=128 * 1024)
        original_create = first._create
        large = sample()
        large['sections']['bags'] = {'0': {str(i): {'item_id': 10, 'count': 1,
            'link': 'item:10:' + ':'.join(['12345'] * 150)} for i in range(1, 101)}}
        first_key = self.store.ingest(large)
        second_key = self.store.ingest(sample('Bob', day=2))

        def pause_after_create():
            original_create()
            created.set()
            if not committed.wait(10):
                raise TimeoutError('second writer did not commit')

        def second_writer():
            if not created.wait(10):
                raise TimeoutError('first writer did not create')
            try:
                return self.inventory.ingest(self.store.root, 'accountA', second_key)
            finally:
                committed.set()

        with patch.object(first, '_create', side_effect=pause_after_create):
            with ThreadPoolExecutor(max_workers=2) as pool:
                failing = pool.submit(first.ingest, self.store.root, 'accountA', first_key)
                winner = pool.submit(second_writer)
                self.assertEqual(winner.result(timeout=15)['status'], 'updated')
                with self.assertRaises((sqlite3.DatabaseError, ValueError)):
                    failing.result(timeout=15)
        result = self.inventory.query('accountA', 10)
        self.assertEqual(result['character_count'], 1)
        self.assertEqual(result['characters'][0]['character']['name'], 'Bob')
        self.assertEqual(result['characters'][0]['storage']['bags']['quantity'], 2)
        self.assertFalse(list(self.root.glob('.inventory-schema-*')))

    def test_bank_backfill_advances_without_character_regression(self):
        self.ingest(sample())
        latest = sample(day=3, count=30)
        latest['sections'].pop('account_bank')
        self.ingest(latest)
        backfill = sample(day=2, count=20)
        backfill['sections']['account_bank']['13']['1']['count'] = 200
        outcome = self.ingest(backfill)
        self.assertEqual(outcome['status'], 'updated')
        self.assertEqual(outcome['character_status'], 'ignored_older')
        self.assertEqual(outcome['account_bank_status'], 'updated')
        result = self.inventory.query('accountA', 10)
        self.assertEqual(result['characters'][0]['storage']['bags']['quantity'], 30)
        self.assertIn('2025-01-03', result['characters'][0]['captured_at'])
        self.assertEqual(result['shared_banks'][0]['storage']['account_bank']['quantity'], 200)
        self.assertIn('2025-01-02', result['shared_banks'][0]['captured_at'])
        repeat = self.ingest(backfill)
        self.assertEqual(repeat['account_bank_status'], 'unchanged')
        self.assertEqual(repeat['character_status'], 'ignored_older')

    def test_child_crash_journal_recovers_on_cli_import(self):
        self.ingest(sample())
        crash_script = """
import os, sqlite3, sys
connection = sqlite3.connect(sys.argv[1])
connection.execute('PRAGMA cache_size=1')
connection.execute('BEGIN IMMEDIATE')
connection.execute("UPDATE characters SET data=?", ('UNCOMMITTED-' * 5000,))
assert os.path.exists(sys.argv[1] + '-journal')
os._exit(23)
"""
        child = subprocess.run([sys.executable, '-c', crash_script, str(self.db)],
                               capture_output=True, text=True, timeout=20)
        self.assertEqual(child.returncode, 23, child.stderr)
        journal = Path(str(self.db) + '-journal')
        self.assertTrue(journal.exists())
        self.assertNotEqual(journal.read_bytes()[:8], b'\x00' * 8)
        before = self.db.read_bytes(), journal.read_bytes()
        with self.assertRaises(sqlite3.OperationalError):
            self.inventory.query('accountA')
        self.assertEqual((self.db.read_bytes(), journal.read_bytes()), before)
        key = self.store.ingest(sample('Bob', day=2))
        resumed = subprocess.run([sys.executable, '-m', 'tools.account_inventory', 'ingest',
            '--database', str(self.db), '--store', str(self.store.root), '--account-key',
            'accountA', '--snapshot-id', key], cwd=ROOT, capture_output=True, text=True, timeout=30)
        self.assertEqual(resumed.returncode, 0, resumed.stderr)
        self.assertFalse(journal.exists())
        result = self.inventory.query('accountA', 10)
        self.assertEqual(result['character_count'], 2)
        self.assertEqual(result['characters'][0]['character']['name'], 'Alice')
        self.assertEqual(result['characters'][0]['storage']['bags']['quantity'], 2)
        self.assertEqual(result['characters'][1]['character']['name'], 'Bob')

    def test_cold_crash_unchanged_import_remains_readable(self):
        capture = sample()
        self.ingest(capture)
        crash_script = """
import os, sqlite3, sys
connection = sqlite3.connect(sys.argv[1])
connection.execute('PRAGMA cache_spill=OFF')
connection.execute('BEGIN IMMEDIATE')
connection.execute("INSERT INTO characters SELECT account,family,variant,realm,'uncommitted',stamp,snapshot,data FROM characters")
with open(sys.argv[1] + '-journal', 'rb') as journal:
    assert journal.read(8) == bytes(8)
os._exit(24)
"""
        child = subprocess.run([sys.executable, '-c', crash_script, str(self.db)],
                               capture_output=True, text=True, timeout=20)
        self.assertEqual(child.returncode, 24, child.stderr)
        journal = Path(str(self.db) + '-journal')
        self.assertTrue(journal.exists())
        before = self.db.read_bytes(), journal.read_bytes()
        self.assertEqual(self.ingest(capture)['status'], 'unchanged')
        result = self.inventory.query('accountA', 10)
        self.assertEqual(result['character_count'], 1)
        self.assertEqual(result['characters'][0]['character']['name'], 'Alice')
        self.assertEqual(result['characters'][0]['storage']['bags']['quantity'], 2)
        self.assertEqual((self.db.read_bytes(), journal.read_bytes()), before)

    def test_readonly_query_with_active_writer_sees_only_committed_rows(self):
        self.ingest(sample())
        with closing(sqlite3.connect(self.db)) as writer:
            writer.execute('PRAGMA cache_spill=OFF')
            writer.execute('BEGIN IMMEDIATE')
            writer.execute("INSERT INTO characters SELECT account,family,variant,realm,'uncommitted',stamp,snapshot,data FROM characters")
            journal = Path(str(self.db) + '-journal')
            before = self.db.read_bytes(), journal.read_bytes()
            self.assertEqual(writer.execute('SELECT count(*) FROM characters').fetchone()[0], 2)
            result = self.inventory.query('accountA')
            self.assertEqual(result['character_count'], 1)
            self.assertEqual((self.db.read_bytes(), journal.read_bytes()), before)
            writer.rollback()
        self.assertEqual(self.inventory.query('accountA')['character_count'], 1)

    def test_child_crash_after_publication_leaves_single_link_usable_database(self):
        crash_script = """
import os, sys
from tools.account_inventory import AccountInventory
original_rename = os.rename
def crash_after_rename(source, target):
    original_rename(source, target)
    os._exit(25)
os.rename = crash_after_rename
AccountInventory(sys.argv[1])._create()
"""
        child = subprocess.run([sys.executable, '-c', crash_script, str(self.db)],
                               cwd=ROOT, capture_output=True, text=True, timeout=20)
        self.assertEqual(child.returncode, 25, child.stderr)
        self.assertEqual(self.db.stat().st_nlink, 1)
        self.assertFalse(list(self.root.glob('.inventory-schema-*')))
        self.assertEqual(self.inventory.query('accountA')['character_count'], 0)
        self.assertEqual(self.ingest(sample())['status'], 'updated')
        self.assertEqual(self.inventory.query('accountA')['character_count'], 1)

    def test_windows_publication_never_replaces_competing_database(self):
        original_rename = os.rename
        def publish_competing_file(source, target):
            self.db.write_bytes(b'COMPETING DATA')
            return original_rename(source, target)
        with patch('tools.account_inventory.os.rename', side_effect=publish_competing_file):
            self.inventory._create()
        self.assertEqual(self.db.read_bytes(), b'COMPETING DATA')
        self.assertFalse(list(self.root.glob('.inventory-schema-*')))
        with self.assertRaisesRegex(ValueError, 'Not a private'):
            self.ingest(sample())
        self.assertEqual(self.db.read_bytes(), b'COMPETING DATA')

    def test_new_database_creation_fails_closed_outside_windows(self):
        with patch('tools.account_inventory.os.name', 'posix'):
            with self.assertRaisesRegex(ValueError, 'Windows atomic no-replace'):
                self.inventory._create()
        self.assertFalse(self.db.exists())

    def test_cli_recovery_rejects_oversized_or_linked_journal(self):
        self.ingest(sample())
        key = self.store.ingest(sample(day=2))
        journal = Path(str(self.db) + '-journal')
        journal.write_bytes(b'x' * (128 * 1024))
        before = self.db.read_bytes()
        small = AccountInventory(self.db, max_bytes=128 * 1024)
        with self.assertRaisesRegex(ValueError, 'unrecovered'):
            small.ingest(self.store.root, 'accountA', key)
        self.assertEqual(self.db.read_bytes(), before)
        journal.unlink()
        target = self.root / 'unrelated-journal'
        target.write_bytes(b'UNCHANGED')
        try:
            journal.symlink_to(target)
        except OSError as exc:
            self.skipTest(str(exc))
        with self.assertRaisesRegex(ValueError, 'Linked paths'):
            self.inventory.ingest(self.store.root, 'accountA', key)
        self.assertEqual(self.db.read_bytes(), before)
        self.assertEqual(target.read_bytes(), b'UNCHANGED')

    def test_modern_item_colour_retains_numeric_variant_without_private_label(self):
        value = sample()
        value['sections']['bags']['0']['1']['link'] = '|cnIQ3:|Hitem:10:0:0:0:0:0:-17:42|h[PRIVATE MODERN NAME]|h|r'
        self.ingest(value)
        match = self.inventory.query('accountA', 10)['characters'][0]['storage']['bags']['matches'][0]
        self.assertEqual(match['item_string'], 'item:10:0:0:0:0:0:-17:42')
        self.assertEqual(match['variant_status'], 'exact_observed_item_string')
        self.assertNotIn(b'PRIVATE MODERN NAME', self.db.read_bytes())
        value = sample(day=2)
        value['sections']['bags']['0']['1']['link'] = '|cnIQ3:|Hitem:10:Player-123-CREATOR|h[PRIVATE]|h|r'
        self.ingest(value)
        match = self.inventory.query('accountA', 10)['characters'][0]['storage']['bags']['matches'][0]
        self.assertNotIn('item_string', match)
        self.assertEqual(match['variant_status'], 'unavailable')
        self.assertNotIn(b'Player-123', self.db.read_bytes())

    def test_shared_bank_older_other_character_does_not_replace_newer_bank(self):
        self.ingest(sample(day=3))
        old = sample('Bob', day=2)
        old['sections']['account_bank']['13']['1']['count'] = 999
        self.ingest(old)
        result = self.inventory.query('accountA', 10)
        self.assertEqual(result['character_count'], 2)
        self.assertEqual(result['shared_banks'][0]['storage']['account_bank']['quantity'], 100)
        self.assertEqual(result['shared_banks'][0]['character']['name'], 'Alice')

    def test_symlink_paths_rejected(self):
        self.ingest(sample())
        linked = self.root / 'link.sqlite'
        try:
            linked.symlink_to(self.db)
        except OSError as exc:
            self.skipTest(str(exc))
        with self.assertRaises(ValueError):
            AccountInventory(linked).query('accountA')
        key = self.store.ingest(sample(day=2))
        original = self.store.root / (key + '.json')
        moved = self.root / 'snapshot.json'
        original.rename(moved); original.symlink_to(moved)
        with self.assertRaises(ValueError):
            self.inventory.ingest(self.store.root, 'accountA', key)

    def test_threaded_read_connections(self):
        self.ingest(sample())
        with ThreadPoolExecutor(max_workers=4) as pool:
            values = list(pool.map(lambda _: self.inventory.query('accountA', 10), range(24)))
        self.assertTrue(all(value['characters'][0]['storage']['bags']['quantity'] == 2 for value in values))

    def test_real_cli_import_and_get(self):
        key = self.store.ingest(sample())
        base = [sys.executable, '-m', 'tools.account_inventory']
        result = subprocess.run(base + ['ingest', '--database', str(self.db), '--store', str(self.store.root), '--account-key', 'accountA', '--snapshot-id', key], cwd=ROOT, capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['status'], 'updated')
        result = subprocess.run(base + ['find', '--database', str(self.db), '--account-key', 'accountA', '--item-id', '10'], cwd=ROOT, capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['characters'][0]['storage']['bags']['quantity'], 2)


class MCPTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_sdk_two_strict_readonly_tools(self):
        with tempfile.TemporaryDirectory(dir=ROOT / 'local-data') as folder:
            root = Path(folder); store = SnapshotStore(root / 'owner')
            db = root / 'inventory.sqlite'; inventory = AccountInventory(db)
            inventory.ingest(store.root, 'accountA', store.ingest(sample()))
            before = db.read_bytes()
            params = StdioServerParameters(command=sys.executable, args=['-m', 'tools.account_inventory', 'serve', '--database', str(db)], cwd=str(ROOT))
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    tools = (await session.list_tools()).tools
                    self.assertEqual({t.name for t in tools}, {'get_private_account_inventory', 'find_private_account_item'})
                    for tool in tools:
                        self.assertTrue(tool.annotations.readOnlyHint)
                        self.assertFalse(tool.annotations.openWorldHint)
                        self.assertFalse(tool.inputSchema['additionalProperties'])
                        self.assertFalse({'database', 'store', 'sql'} & tool.inputSchema['properties'].keys())
                    good = await session.call_tool('find_private_account_item', {'account_key': 'accountA', 'item_id': 10})
                    self.assertFalse(good.isError)
                    self.assertEqual(json.loads(good.content[0].text)['characters'][0]['storage']['bags']['quantity'], 2)
                    for bad in (True, '10', 0, 10.0):
                        reply = await session.call_tool('find_private_account_item', {'account_key': 'accountA', 'item_id': bad})
                        self.assertTrue(reply.isError)
                    reply = await session.call_tool('get_private_account_inventory', {'account_key': 'accountA', 'limit': True})
                    self.assertTrue(reply.isError)
                    reply = await session.call_tool('get_private_account_inventory', {'account_key': 'accountA', 'database': 'other.sqlite'})
                    self.assertTrue(reply.isError)
                    reply = await session.call_tool('get_private_account_inventory', {'account_key': 'accountB'})
                    self.assertEqual(json.loads(reply.content[0].text)['character_count'], 0)
            self.assertEqual(db.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
