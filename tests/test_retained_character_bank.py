"""Offline retained-bank provenance, ordering, limits and actual stdio callers."""
import asyncio
from contextlib import closing
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from tools.account_inventory import AccountInventory

ROOT = Path(__file__).resolve().parents[1]


def capture(day, *, bank=True, count=9, name='Alice'):
    value = {'schema_version': 1, 'source': 'addon',
        'captured_at': f'2025-01-{day:02d}T00:00:00Z',
        'client': {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001},
        'character': {'name': name, 'realm': 'Test'},
        'sections': {'equipment': {}, 'bags': {'0': {'1': {'item_id': 10, 'count': day}}}}}
    if bank:
        value['sections']['character_bank'] = {'6': {'1': {'item_id': 10, 'count': count}}}
    return value


class RetainedBankTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='retained-bank-', dir=ROOT/'local-data')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.store = SnapshotStore(self.root/'owner')
        self.db = self.root/'inventory.sqlite'
        self.inventory = AccountInventory(self.db)

    def ingest(self, value, account='owner'):
        key = self.store.ingest(value)
        result = self.inventory.ingest(self.store.root, account, key)
        return key, result

    def row(self, item=10):
        return self.inventory.query('owner', item)['characters'][0]

    def test_closed_bank_preserves_separate_provenance_and_unknown_current_counts(self):
        first, _ = self.ingest(capture(1))
        latest, result = self.ingest(capture(3, bank=False))
        row = self.row()
        self.assertEqual(result['character_bank_status'], 'unobserved')
        self.assertEqual(row['snapshot_id'], latest)
        self.assertEqual(row['storage']['bags']['quantity'], 3)
        self.assertIsNone(row['storage']['character_bank']['quantity'])
        bank = row['last_observed_character_bank']
        self.assertEqual(bank['snapshot_id'], first)
        self.assertEqual(bank['observed_quantity'], 9)
        self.assertEqual(bank['observed_matches'][0]['container'], '6')
        self.assertFalse(bank['current'])
        self.assertFalse(bank['available_stock'])
        self.assertFalse(bank['observed_in_latest_capture'])
        self.assertTrue(bank['matches_latest_client_metadata'])
        self.assertGreaterEqual(bank['age_seconds'], row['age_seconds'])
        self.assertNotIn('quantity', bank)

    def test_older_capture_backfills_bank_without_regressing_latest_character(self):
        latest, _ = self.ingest(capture(5, bank=False))
        older, result = self.ingest(capture(3))
        row = self.row()
        self.assertEqual(result['status'], 'updated')
        self.assertEqual(result['character_status'], 'ignored_older')
        self.assertEqual(result['character_bank_status'], 'updated')
        self.assertEqual(row['snapshot_id'], latest)
        self.assertEqual(row['storage']['bags']['quantity'], 5)
        self.assertIsNone(row['storage']['character_bank']['quantity'])
        self.assertEqual(row['last_observed_character_bank']['snapshot_id'], older)
        self.ingest(capture(2, count=99))
        self.assertEqual(self.row()['last_observed_character_bank']['observed_quantity'], 9)

    def test_observed_empty_replaces_old_contents_and_unknown_is_not_empty(self):
        self.ingest(capture(1))
        empty = capture(2)
        empty['sections']['character_bank'] = {}
        key, _ = self.ingest(empty)
        row = self.row()
        self.assertEqual(row['storage']['character_bank']['quantity'], 0)
        self.assertEqual(row['last_observed_character_bank']['observed_quantity'], 0)
        self.assertTrue(row['last_observed_character_bank']['observed_in_latest_capture'])
        self.ingest(capture(3, bank=False))
        row = self.row()
        self.assertIsNone(row['storage']['character_bank']['quantity'])
        self.assertEqual(row['last_observed_character_bank']['snapshot_id'], key)
        self.assertEqual(row['last_observed_character_bank']['observed_matches'], [])

    def test_idempotence_and_bank_timestamp_conflict_preserve_all_rows(self):
        value = capture(2)
        self.ingest(capture(5, bank=False))
        self.ingest(value)
        before = self.db.read_bytes()
        _, result = self.ingest(value)
        self.assertEqual(result['character_bank_status'], 'unchanged')
        with self.assertRaisesRegex(ValueError, 'Ambiguous'):
            self.ingest(capture(2, count=77))
        self.assertEqual(self.db.read_bytes(), before)
        self.assertEqual(self.row()['last_observed_character_bank']['observed_quantity'], 9)

    def test_shared_bank_conflict_rolls_back_new_character_and_bank_observation(self):
        self.ingest(capture(1))
        bob = capture(3, name='Bob')
        bob['sections']['account_bank'] = {}
        self.ingest(bob)
        before = self.db.read_bytes()
        conflict = capture(3, count=20)
        conflict['sections']['account_bank'] = {}
        with self.assertRaisesRegex(ValueError, 'Ambiguous'):
            self.ingest(conflict)
        self.assertEqual(self.db.read_bytes(), before)
        self.assertEqual(self.row()['last_observed_character_bank']['observed_quantity'], 9)

    def test_legacy_rows_derive_bank_without_a_write_and_preserve_it_on_next_import(self):
        key, _ = self.ingest(capture(1))
        with closing(sqlite3.connect(self.db)) as conn, conn:
            data = json.loads(conn.execute('SELECT data FROM characters').fetchone()[0])
            data.pop('last_observed_character_bank')
            conn.execute('UPDATE characters SET data=?', (json.dumps(data),))
        before = self.db.read_bytes()
        self.assertEqual(self.row()['last_observed_character_bank']['snapshot_id'], key)
        self.assertEqual(self.db.read_bytes(), before)
        self.ingest(capture(2, bank=False))
        self.assertEqual(self.row()['last_observed_character_bank']['snapshot_id'], key)
        with closing(sqlite3.connect(self.db)) as conn:
            self.assertEqual(conn.execute('PRAGMA user_version').fetchone()[0], 1)

    def test_never_observed_bank_stays_absent(self):
        self.ingest(capture(1, bank=False))
        self.assertIsNone(self.row()['last_observed_character_bank'])

    def test_account_variant_realm_and_character_keys_remain_separate(self):
        self.ingest(capture(1, count=11))
        self.ingest(capture(1, count=22), 'other')
        variant = capture(2, count=33)
        variant['client']['variant'] = 'test'
        self.ingest(variant)
        realm = capture(3, count=44)
        realm['character']['realm'] = 'Elsewhere'
        self.ingest(realm)
        self.ingest(capture(4, count=55, name='Bob'))
        latest = capture(5, bank=False, name='ALICE')
        latest['sections']['stats'] = {'faction': 'Horde'}
        self.ingest(latest)
        rows = self.inventory.query('owner', 10)['characters']
        self.assertEqual(sorted(r['last_observed_character_bank']['observed_quantity'] for r in rows), [11, 33, 44, 55])
        self.assertEqual(self.inventory.query('other', 10)['characters'][0]['last_observed_character_bank']['observed_quantity'], 22)
        self.assertEqual(self.inventory.query('absent')['character_count'], 0)

    def test_build_change_keeps_original_bank_build_and_marks_mismatch(self):
        self.ingest(capture(1))
        latest = capture(2, bank=False)
        latest['client']['build'] = '70863'
        self.ingest(latest)
        bank = self.row()['last_observed_character_bank']
        self.assertEqual(bank['client']['build'], '70862')
        self.assertFalse(bank['matches_latest_client_metadata'])
        self.assertFalse(bank['current'])

    def test_shared_bank_projection_does_not_persist_character_bank_history(self):
        first = capture(1)
        first['sections']['account_bank'] = {}
        self.ingest(first)
        self.ingest(capture(2, bank=False))
        with closing(sqlite3.connect(self.db)) as conn:
            shared = json.loads(conn.execute('SELECT data FROM shared_banks').fetchone()[0])
        self.assertNotIn('last_observed_character_bank', shared)
        self.assertEqual(set(shared['storage']), {'account_bank'})
        self.assertNotIn('last_observed_character_bank', self.inventory.query('owner')['shared_banks'][0])

    def test_item_filter_and_summary_do_not_pool_retained_and_current_counts(self):
        value = capture(1)
        value['sections']['character_bank']['6']['2'] = {'item_id': 20, 'count': 7}
        self.ingest(value)
        self.ingest(capture(2, bank=False))
        row = self.row(20)
        self.assertEqual(row['storage']['bags']['quantity'], 0)
        self.assertEqual(row['last_observed_character_bank']['observed_quantity'], 7)
        summary = self.inventory.query('owner')['characters'][0]['last_observed_character_bank']
        self.assertEqual(summary['observed_quantity'], 16)
        self.assertNotIn('observed_matches', summary)

    def test_replacements_keep_one_observation_and_existing_quota_rolls_back(self):
        self.inventory = AccountInventory(self.db, max_bytes=256*1024)
        for day in range(1, 20):
            self.ingest(capture(day, count=day))
        with closing(sqlite3.connect(self.db)) as conn:
            self.assertEqual(conn.execute('SELECT count(*) FROM characters').fetchone()[0], 1)
            data = json.loads(conn.execute('SELECT data FROM characters').fetchone()[0])
        self.assertNotIn('last_observed_character_bank', data['last_observed_character_bank'])
        before = self.db.read_bytes()
        large = capture(20)
        large['sections']['character_bank'] = {'6': {str(i): {'item_id': 10, 'count': 1,
            'link': 'item:10:' + ':'.join(['12345']*150)} for i in range(1, 101)}}
        with self.assertRaises((ValueError, sqlite3.DatabaseError)):
            self.ingest(large)
        self.assertEqual(self.db.read_bytes(), before)
        self.assertLessEqual(sum(p.stat().st_size for p in self.root.glob('inventory.sqlite*')), 256*1024)

    def test_large_retained_matches_fail_response_limit_without_truncation_or_writes(self):
        value = capture(1)
        value['sections']['character_bank'] = {'6': {str(i): {'item_id': 10, 'count': 1,
            'link': 'item:10:' + ':'.join(['12345']*250)} for i in range(1, 201)}}
        self.ingest(value)
        self.ingest(capture(2, bank=False))
        before = self.db.read_bytes()
        self.assertEqual(self.inventory.query('owner')['characters'][0]['last_observed_character_bank']['observed_quantity'], 200)
        with self.assertRaisesRegex(ValueError, 'Response exceeds limit'):
            self.inventory.query('owner', 10)
        self.assertEqual(self.db.read_bytes(), before)


class RetainedBankMCPTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_offline_stdio_reports_unknown_current_and_separate_retained_bank(self):
        with tempfile.TemporaryDirectory(prefix='retained-bank-mcp-', dir=ROOT/'local-data') as folder:
            base = Path(folder)
            store = SnapshotStore(base/'owner')
            inventory = AccountInventory(base/'inventory.sqlite')
            original = store.ingest(capture(1))
            inventory.ingest(store.root, 'synthetic', original)
            inventory.ingest(store.root, 'synthetic', store.ingest(capture(2, bank=False)))
            before = inventory.path.read_bytes()
            params = StdioServerParameters(command=str(ROOT/'.venv/Scripts/python.exe'),
                args=['-B', '-m', 'tools.account_inventory', 'serve', '--database', str(inventory.path)],
                cwd=str(base), env={'PYTHONPATH': str(ROOT)})
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    await asyncio.wait_for(session.initialize(), 15)
                    tools = (await session.list_tools()).tools
                    self.assertEqual(len(tools), 2)
                    for name in ('get_private_account_inventory', 'find_private_account_item'):
                        args = {'account_key': 'synthetic'}
                        if name.startswith('find_'):
                            args['item_id'] = 10
                        response = await asyncio.wait_for(session.call_tool(name, args), 10)
                        self.assertFalse(response.isError)
                        row = json.loads(response.content[0].text)['characters'][0]
                        self.assertIsNone(row['storage']['character_bank']['quantity'])
                        retained = row['last_observed_character_bank']
                        self.assertEqual(retained['snapshot_id'], original)
                        self.assertEqual(retained['observed_quantity'], 9)
                        self.assertFalse(retained['current'])
                        self.assertFalse(retained['available_stock'])
            self.assertEqual(inventory.path.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
