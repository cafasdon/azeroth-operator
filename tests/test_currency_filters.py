"""Find saved currency cap pressure without inventing earning eligibility."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class CurrencyFilterTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_flags_caps_basis_unknowns_and_guards(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        def row(**extra):
            return {'name': 'Currency', 'quantity': 90, 'max_quantity': 100, 'max_weekly_quantity': 10,
                'earned_this_week': 9, 'total_earned': 1, 'account_wide': False, 'inactive': False,
                'cap_uses_total_earned': False, 'can_earn_per_week': True, **extra}
        rows = [row(), row(quantity=1, total_earned=95, cap_uses_total_earned=True, account_wide=True),
            row(max_quantity=0, max_weekly_quantity=0, can_earn_per_week=False),
            row(max_weekly_quantity=0, quantity=99), row(quantity=99),
            row(can_earn_per_week=False, inactive=True, quantity=101, earned_this_week=10),
            row(max_quantity=0, earned_this_week=20), row(max_weekly_quantity=20, earned_this_week=10)]
        for index in [3, 4]:
            del rows[index]['can_earn_per_week']
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            missing = store.ingest(base)
            empty = deepcopy(base)
            empty['sections']['currency_balances'] = {'coverage': 'visible_currency_ids_only', 'entries': {}}
            empty_id = store.ingest(empty)
            saved = deepcopy(empty)
            saved['sections']['currency_balances']['entries'] = {str(i): value for i, value in enumerate(rows, 1)}
            key = store.ingest(saved)
            retail = store.ingest(fixture())
            original = {p.name: p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def query(error=False, **options):
                        reply = await session.call_tool('inspect_snapshot', {'snapshot_id': key,
                            'section': 'currency_balances', **options})
                        self.assertEqual(bool(reply.isError), error, reply)
                        return None if error else json.loads(reply.content[0].text)
                    cases = [({'maximum_weekly_remaining': 1, 'can_earn_per_week': True}, {'1', '2', '7'}, 1),
                        ({'maximum_total_remaining': 5, 'can_earn_per_week': True}, {'2'}, 2),
                        ({'maximum_weekly_remaining': 1, 'maximum_total_remaining': 5, 'can_earn_per_week': True}, {'2'}, 1),
                        ({'has_weekly_cap': False, 'has_total_cap': False}, {'3'}, 0),
                        ({'has_weekly_cap': False, 'can_earn_per_week': True}, set(), 1),
                        ({'inactive': True}, {'6'}, 0), ({'account_wide': True}, {'2'}, 0),
                        ({'maximum_weekly_remaining': 0}, {'6', '7'}, 0),
                        ({'maximum_total_remaining': 0}, {'6'}, 0),
                        ({'has_weekly_cap': False, 'maximum_weekly_remaining': 0}, set(), 0)]
                    for filters, ids, unknown in cases:
                        found = await query(currency_filter=filters)
                        self.assertEqual(set(found['data']['entries']), ids, filters)
                        self.assertEqual(found['currency_filter_unknown_count'], unknown, filters)
                        self.assertTrue(set(found['currency_cap_headroom']).issubset(ids))
                        self.assertEqual(found['data']['coverage'], 'visible_currency_ids_only')
                    total = await query(currency_filter={'account_wide': True})
                    self.assertEqual(total['currency_cap_headroom']['2']['maximum_basis'], 'total_earned')
                    self.assertEqual(total['currency_cap_headroom']['2']['maximum_remaining'], 5)
                    absent = await query(snapshot_id=missing, currency_filter={'inactive': False})
                    self.assertIsNone(absent['data'])
                    self.assertIsNone(absent['currency_filter_unknown_count'])
                    observed_empty = await query(snapshot_id=empty_id, currency_filter={'inactive': False})
                    self.assertEqual(observed_empty['data']['entries'], {})
                    self.assertEqual(observed_empty['currency_filter_unknown_count'], 0)
                    default = await query()
                    self.assertEqual(default['data'], saved['sections']['currency_balances'])
                    self.assertNotIn('currency_filter_unknown_count', default)
                    self.assertTrue((await query(currency_id=2))['currency_observed'])
                    for bad in [{}, {'inactive': None}, {'inactive': 0}, {'can_earn_per_week': 'true'},
                        {'maximum_total_remaining': -1}, {'maximum_weekly_remaining': True},
                        {'maximum_total_remaining': 2**64}, {'PRIVATE': True}]:
                        await query(error=True, currency_filter=bad)
                    for bad in [{'currency_id': 2}, {'currency_reward_quantity': 1}, {'section': 'equipment'},
                                {'section': None}, {'snapshot_id': retail}, {'snapshot_id': '../PRIVATE'}]:
                        await query(error=True, currency_filter={'inactive': False}, **bad)
            self.assertEqual(original, {p.name: p.read_bytes() for p in Path(folder).glob('*.json')})
