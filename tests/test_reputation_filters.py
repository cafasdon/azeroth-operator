"""Search visible saved reputation without mixing rank units or filling gaps."""
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


class ReputationFilterTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_kinds_progress_rewards_unknowns_and_guards(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        def row(kind, **detail):
            return {'name': 'Faction', 'kind': kind, 'account_wide': False, 'reaction': 4,
                    'standing': 90, 'threshold': 0, 'next_threshold': 100, **detail}
        rows = [row('standard', account_wide=True), row('standard', reaction=8),
            row('standard', threshold=10, next_threshold=5, standing=7),
            row('renown', account_wide=True, renown={'level': 2, 'earned_in_level': 9, 'level_threshold': 10, 'capped': False}),
            row('renown', renown={'level': 10, 'earned_in_level': 0, 'level_threshold': 0, 'capped': True}),
            row('renown'), row('friendship', account_wide=True, friendship={'standing': 9, 'threshold': 0, 'next_threshold': 10}),
            row('friendship', friendship={'standing': 10, 'threshold': 10}), row('friendship'),
            row('paragon', account_wide=True, paragon={'current_value': 10, 'reward_threshold': 10, 'reward_pending': True, 'level_too_low': False}),
            row('paragon', paragon={'current_value': 10, 'reward_threshold': 10, 'reward_pending': True, 'level_too_low': True}),
            row('paragon', paragon={'current_value': 0, 'reward_threshold': 10, 'reward_pending': False, 'level_too_low': True}),
            row('paragon'), row('unknown'), row('standard', reaction=1, standing=-5, threshold=-10, next_threshold=0)]
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            missing = store.ingest(base)
            empty = deepcopy(base)
            empty['sections']['reputation'] = {'coverage': 'visible_faction_rows_only', 'entries': {}}
            empty_id = store.ingest(empty)
            saved = deepcopy(empty)
            saved['sections']['reputation']['entries'] = {str(i): value for i, value in enumerate(rows, 1)}
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
                            'section': 'reputation', **options})
                        self.assertEqual(bool(reply.isError), error, reply)
                        return None if error else json.loads(reply.content[0].text)
                    cases = [({'kind': 'standard', 'maximum_remaining_to_next_rank': 10}, {'1', '15'}, 1),
                        ({'kind': 'standard', 'capped': True}, {'2'}, 1),
                        ({'kind': 'renown', 'maximum_remaining_to_next_rank': 1}, {'4'}, 1),
                        ({'kind': 'friendship', 'maximum_remaining_to_next_rank': 1}, {'7'}, 1),
                        ({'kind': 'friendship', 'capped': True}, {'8'}, 1),
                        ({'kind': 'paragon', 'paragon_reward_status': 'pending'}, {'10'}, 1),
                        ({'kind': 'paragon', 'paragon_reward_status': 'pending_level_restricted'}, {'11'}, 1),
                        ({'kind': 'paragon', 'paragon_reward_status': 'no_pending_reward'}, {'12'}, 1),
                        ({'kind': 'paragon', 'paragon_reward_status': 'pending', 'account_wide': True}, {'10'}, 0),
                        ({'kind': 'unknown'}, {'14'}, 0), ({'account_wide': True}, {'1', '4', '7', '10'}, 0)]
                    for filters, ids, unknown in cases:
                        found = await query(reputation_filter=filters)
                        self.assertEqual(set(found['data']['entries']), ids, filters)
                        self.assertEqual(found['reputation_filter_unknown_count'], unknown, filters)
                        self.assertEqual(found['data']['coverage'], 'visible_faction_rows_only')
                        for field in ['standard_reputation_progress', 'special_reputation_progress']:
                            self.assertTrue(set(found[field]).issubset(ids))
                    capped = await query(reputation_filter={'kind': 'renown', 'capped': True,
                                                           'maximum_remaining_to_next_rank': 0})
                    self.assertEqual(capped['data']['entries'], {})
                    absent = await query(snapshot_id=missing, reputation_filter={'kind': 'standard'})
                    self.assertIsNone(absent['data'])
                    self.assertIsNone(absent['reputation_filter_unknown_count'])
                    observed_empty = await query(snapshot_id=empty_id, reputation_filter={'kind': 'standard'})
                    self.assertEqual(observed_empty['data']['entries'], {})
                    self.assertEqual(observed_empty['reputation_filter_unknown_count'], 0)
                    self.assertEqual((await query())['data'], saved['sections']['reputation'])
                    self.assertTrue((await query(faction_id=1))['faction_observed'])
                    for bad in [{}, {'kind': None}, {'kind': 'other'}, {'capped': True},
                        {'maximum_remaining_to_next_rank': 1}, {'kind': 'paragon', 'capped': False},
                        {'paragon_reward_status': 'pending'}, {'kind': 'renown', 'paragon_reward_status': 'pending'},
                        {'kind': 'standard', 'maximum_remaining_to_next_rank': -1},
                        {'kind': 'standard', 'maximum_remaining_to_next_rank': True},
                        {'kind': 'standard', 'maximum_remaining_to_next_rank': 2**64},
                        {'account_wide': 1}, {'PRIVATE': True}]:
                        await query(error=True, reputation_filter=bad)
                    for bad in [{'faction_id': 1}, {'section': 'equipment'}, {'section': None},
                                {'snapshot_id': retail}, {'snapshot_id': '../PRIVATE'}]:
                        await query(error=True, reputation_filter={'kind': 'standard'}, **bad)
            self.assertEqual(original, {p.name: p.read_bytes() for p in Path(folder).glob('*.json')})
