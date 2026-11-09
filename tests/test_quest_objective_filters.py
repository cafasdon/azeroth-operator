"""Search captured quest objectives with same-row and unknown-value semantics."""
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


class QuestObjectiveFilterTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_same_row_unknowns_and_existing_filters(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        def objective(kind='monster', finished=False, required=None, fulfilled=None):
            row = {'kind': kind, 'finished': finished}
            if required is not None:
                row.update(required=required, fulfilled=fulfilled)
            return row
        rows = [None, {}, {'1': objective(required=5, fulfilled=4)},
            {'1': objective(finished=True, required=10, fulfilled=10)},
            {'1': objective('item'), '2': objective(finished=True, required=1, fulfilled=1)},
            {'1': objective()}, {'1': objective(required=1, fulfilled=3)},
            {'1': objective('item', required=2, fulfilled=1)},
            {'1': objective(required=3, fulfilled=0), '2': objective()},
            {'1': objective(required=1, fulfilled=0), '2': objective()}]
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            missing = store.ingest(base)
            saved = deepcopy(base)
            saved['sections']['quests'] = {str(i): {'title': f'Quest {i}', 'level': 60,
                'ready_for_turn_in': i == 3, **({'objectives': row} if row is not None else {})}
                for i, row in enumerate(rows, 1)}
            saved['sections']['quests']['3']['item_rewards'] = {'1': {'item_id': 118, 'quantity': 2}}
            saved['sections']['quests']['7']['item_rewards'] = {'1': {'item_id': 858, 'quantity': 3}}
            saved['sections']['quests']['3']['timer'] = {'remaining_seconds':60,'total_seconds':120}
            key = store.ingest(saved)
            changed=deepcopy(saved)
            changed['sections']['quests']['9']['objectives']['2']=objective(required=1,fulfilled=2)
            changed_key=store.ingest(changed)
            retail = store.ingest(fixture())
            original = {p.name: p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def query(error=False, **options):
                        reply = await session.call_tool('inspect_snapshot', {'snapshot_id': key,
                            'section': 'quests', **options})
                        self.assertEqual(bool(reply.isError), error, reply)
                        return None if error else json.loads(reply.content[0].text)
                    options = {'quest_objective_kind': 'monster', 'quest_objective_finished': False,
                               'quest_objective_max_remaining': 1}
                    found = await query(**options)
                    self.assertEqual(set(found['data']), {'3', '7', '10'})
                    self.assertEqual(found['quest_objective_unknown_count'], 3)
                    self.assertEqual(found['quest_objective_filter'],
                                     {'kind': 'monster', 'finished': False, 'maximum_remaining': 1})
                    ready = await query(**options, quest_readiness='ready')
                    self.assertEqual(set(ready['data']), {'3'})
                    self.assertEqual(ready['quest_objective_unknown_count'], 0)
                    rewarded = await query(**options, quest_reward_item_id=118, quest_list_item_rewards=True)
                    self.assertEqual(set(rewarded['data']), {'3'})
                    self.assertEqual(rewarded['quest_list_item_rewards']['known_fixed_quantity_by_item_id'], {'118': 2})
                    self.assertEqual(set((await query(quest_objective_kind='monster',
                        quest_objective_finished=False))['data']), {'3', '6', '7', '9', '10'})
                    self.assertEqual(set((await query(quest_objective_max_remaining=0))['data']), {'4', '5', '7'})
                    bounded=await query(quest_objective_kind='monster',quest_objective_min_remaining=1,
                        quest_objective_max_remaining=1)
                    self.assertEqual(set(bounded['data']),{'3','10'})
                    self.assertEqual(bounded['quest_objective_filter']['minimum_remaining'],1)
                    self.assertEqual(set((await query(quest_objective_min_remaining=2))['data']),{'9'})
                    await query(error=True,quest_objective_min_remaining=2,quest_objective_max_remaining=1)
                    absent = await query(snapshot_id=missing, **options)
                    self.assertIsNone(absent['data'])
                    self.assertIsNone(absent['quest_objective_unknown_count'])
                    self.assertEqual((await query(quest_objective_kind='Monster'))['data'], {})
                    default = await query()
                    deadline=await query(quest_projected_expiry_by='2025-01-01T00:01:00Z')
                    self.assertEqual(set(deadline['data']),{'3'})
                    self.assertEqual(deadline['quest_timer_unknown_count'],9)
                    start='2025-01-01T00:01:00Z'
                    window=await query(quest_projected_expiry_from=start,quest_projected_expiry_by=start)
                    self.assertEqual(window['data'],deadline['data'])
                    self.assertEqual(window['quest_projected_expiry_from'],start)
                    start_only=await query(quest_projected_expiry_from=start)
                    self.assertEqual(start_only['quest_projected_expiry_by_quest_id'],{'3':start})
                    self.assertEqual((await query(quest_projected_expiry_from='2025-01-01T00:01:01Z'))['data'],{})
                    await query(error=True,quest_projected_expiry_from=start,quest_projected_expiry_by='2025-01-01T00:00:59Z')
                    self.assertEqual(deadline['quest_projected_expiry_by_quest_id'],{'3':'2025-01-01T00:01:00Z'})
                    self.assertNotIn('quest_projected_expiry_by_quest_id',default)
                    self.assertEqual(deadline['data']['3'],saved['sections']['quests']['3'])
                    filtered=await query(quest_projected_expiry_by='2025-01-01T00:01:00Z',
                        quest_reward_item_id=858)
                    self.assertEqual(filtered['quest_projected_expiry_by_quest_id'],{})
                    absent_deadline=await query(snapshot_id=missing,quest_projected_expiry_by='2025-01-01T00:01:00Z')
                    self.assertIsNone(absent_deadline['quest_projected_expiry_by_quest_id'])
                    self.assertEqual((await query(quest_projected_expiry_by='2025-01-01T00:00:59Z'))['data'],{})
                    self.assertEqual((await query(quest_projected_expiry_by='2025-01-01T00:01:00Z',
                        quest_max_remaining_seconds=59))['data'],{})
                    await query(error=True,quest_projected_expiry_by='2025-01-01')
                    await query(error=True,quest_projected_expiry_by='2025-01-01T00:01:00Z',section='equipment')
                    self.assertEqual(default['data'], saved['sections']['quests'])
                    self.assertNotIn('quest_objective_filter', default)
                    for bad in [{'quest_objective_kind': ''}, {'quest_objective_kind': 'x' * 65},
                        {'quest_objective_kind': 1}, {'quest_objective_finished': 0},
                        {'quest_objective_max_remaining': True}, {'quest_objective_max_remaining': -1},
                        {'quest_objective_max_remaining': 2**32}, {'quest_id': 3, **options},
                        {'section': 'equipment', **options}, {'section': None, **options},
                        {'snapshot_id': retail, **options}, {'snapshot_id': '../PRIVATE', **options}]:
                        await query(error=True, **bad)
                    reply=await session.call_tool('compare_snapshots',{'before_id':key,'after_id':changed_key,'quest_id':9})
                    self.assertFalse(reply.isError,reply)
                    quantities=json.loads(reply.content[0].text)['quest_change']['objective_quantities_by_kind']
                    self.assertEqual(quantities['before']['monster'],{'fulfilled':0,'required':3,'remaining':3,
                        'counted_objectives':1,'unknown_objectives':1})
                    self.assertEqual(quantities['after']['monster'],{'fulfilled':2,'required':4,'remaining':3,
                        'counted_objectives':2,'unknown_objectives':0})
                    self.assertFalse(quantities['objective_identity_verified'])
            self.assertEqual(original, {p.name: p.read_bytes() for p in Path(folder).glob('*.json')})
