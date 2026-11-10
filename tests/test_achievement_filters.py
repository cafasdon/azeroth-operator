"""Find tracked achievement progress without inferring absent observations."""
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


class AchievementFilterTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_same_criterion_credit_unknowns_and_work(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        def criterion(id=100, completed=False, quantity=None, required=None):
            row = {'completed': completed}
            if id is not None:
                row['criterion_id'] = id
            if quantity is not None:
                row.update(quantity=quantity, required=required)
            return row
        lists = [[], [criterion(quantity=4, required=5)],
            [criterion(completed=True, quantity=1, required=1)],
            [criterion(completed=True), criterion(200)],
            [criterion(None, quantity=1, required=2)], [criterion()],
            [criterion(quantity=3, required=1)], [criterion(None, quantity=0, required=5)],
            [criterion(200)], [criterion(None), criterion(quantity=0, required=1)],
            [criterion(quantity=0, required=10), criterion(None)]]
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            missing = store.ingest(base)
            empty = deepcopy(base)
            empty['sections']['tracked_achievements'] = {'coverage': 'tracked_player_achievements_only', 'entries': {}}
            empty_id = store.ingest(empty)
            saved = deepcopy(empty)
            saved['sections']['tracked_achievements']['entries'] = {str(i): {
                'name': f'Achievement {i}', 'points': 10, 'completed': i == 2,
                'earned_by_character': False, 'account_wide': i == 2,
                'criteria': {str(j): row for j, row in enumerate(rows, 1)}}
                for i, rows in enumerate(lists, 1)}
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
                            'section': 'tracked_achievements', **options})
                        self.assertEqual(bool(reply.isError), error, reply)
                        return None if error else json.loads(reply.content[0].text)
                    filters = {'criterion_id': 100, 'criterion_completed': False, 'criterion_max_remaining': 1}
                    found = await query(achievement_filter=filters)
                    self.assertEqual(set(found['data']['entries']), {'2', '7', '10'})
                    self.assertEqual(set(found['tracked_achievement_work']), {'2', '7', '10'})
                    self.assertEqual(found['tracked_achievement_work']['7']['1']['remaining_count'], 0)
                    self.assertEqual(found['achievement_filter_unknown_count'], 3)
                    incomplete = await query(achievement_filter={**filters, 'completed': False})
                    self.assertEqual(set(incomplete['data']['entries']), {'7', '10'})
                    for maximum, ids in [
                        (0, {'1','3'}),
                        (1, {'1','2','3','4','5','6','7','8','9'}),
                        (4294967295, {str(i) for i in range(1,12)}),
                    ]:
                        workload = await query(achievement_filter={'maximum_unfinished_criteria':maximum})
                        self.assertEqual(set(workload['data']['entries']),ids)
                    short = await query(achievement_filter={'maximum_unfinished_criteria':1,**filters})
                    self.assertEqual(set(short['data']['entries']),{'2','7'})
                    for maximum in [-1, True, 4294967296]:
                        await query(error=True,achievement_filter={'maximum_unfinished_criteria':maximum})
                    credited = await query(achievement_filter={'completed': True, 'earned_by_character': False,
                                                               'account_wide': True})
                    self.assertEqual(set(credited['data']['entries']), {'2'})
                    self.assertEqual(credited['achievement_filter_unknown_count'], 0)
                    self.assertEqual(credited['data']['coverage'], 'tracked_player_achievements_only')
                    absent = await query(snapshot_id=missing, achievement_filter=filters)
                    self.assertIsNone(absent['data'])
                    self.assertIsNone(absent['achievement_filter_unknown_count'])
                    observed_empty = await query(snapshot_id=empty_id, achievement_filter=filters)
                    self.assertEqual(observed_empty['data']['entries'], {})
                    self.assertEqual(observed_empty['achievement_filter_unknown_count'], 0)
                    default = await query()
                    self.assertEqual(default['data'], saved['sections']['tracked_achievements'])
                    self.assertNotIn('achievement_filter_unknown_count', default)
                    selected = await query(achievement_id=2)
                    self.assertTrue(selected['achievement_observed'])
                    self.assertFalse(selected['data']['earned_by_character'])
                    for bad in [{}, {'completed': None}, {'completed': 0}, {'criterion_completed': 'false'},
                        {'criterion_id': 0}, {'criterion_id': True}, {'criterion_id': 2**32},
                        {'criterion_max_remaining': -1}, {'criterion_max_remaining': True},
                        {'criterion_max_remaining': 2**32}, {'PRIVATE': True}]:
                        await query(error=True, achievement_filter=bad)
                    for bad in [{'achievement_id': 2}, {'section': 'quests'}, {'section': None},
                                {'snapshot_id': retail}, {'snapshot_id': '../PRIVATE'}]:
                        await query(error=True, achievement_filter=filters, **bad)
            self.assertEqual(original, {p.name: p.read_bytes() for p in Path(folder).glob('*.json')})
