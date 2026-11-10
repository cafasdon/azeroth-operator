"""Saved visible gossip lists never imply acceptance or turn-in eligibility."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore, validate_snapshot
from test_snapshots import fixture


class GossipQuestTests(unittest.IsolatedAsyncioTestCase):
    async def test_mcp_summary_inspection_unknowns_bounds_and_reader_parity(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        section = {'coverage': 'visible_gossip_quest_lists', 'available': {'10': {'repeatable': False}, '2': {}},
                   'active': {'20': {'is_complete': True}, '21': {'is_complete': False}, '22': {}}}
        cases = [None, section, {'coverage': section['coverage'], 'available': {}},
                 {'coverage': section['coverage'], 'active': {str(i): {'is_complete': True} for i in range(1, 129)}}]
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            valid = []
            for value in cases:
                sample = deepcopy(base)
                if value is not None:
                    sample['sections']['gossip_quests'] = value
                valid.append((store.ingest(sample), value))
            invalid = [None, {}, {'coverage': section['coverage']}, {**section, 'coverage': 'all_quests'},
                       {**section, 'available': None}, {**section, 'available': []},
                       {**section, 'available': {'0': {}}}, {**section, 'available': {'01': {}}},
                       {**section, 'available': {'4294967296': {}}}, {**section, 'available': {'2': None}},
                       {**section, 'available': {str(i): {} for i in range(1, 130)}},
                       {**section, 'speaker': 'PRIVATE_GOSSIP'}]
            for field in ['is_complete', 'repeatable', 'is_trivial', 'is_ignored']:
                for bad in [None, 0, 'false', [], {}]:
                    invalid.append({**section, 'available': {'2': {field: bad}}})
            invalid.append({**section, 'available': {'2': {'title': 'PRIVATE_GOSSIP'}}})
            bad_ids = []
            for value in invalid:
                sample = deepcopy(base)
                sample['sections']['gossip_quests'] = value
                with self.assertRaises(ValueError):
                    validate_snapshot(sample)
                raw = json.dumps(sample, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            for family in ['retail', 'classic']:
                sample = deepcopy(base)
                sample['client']['family'] = family
                sample['sections']['gossip_quests'] = section
                with self.assertRaises(ValueError):
                    validate_snapshot(sample)
                raw = json.dumps(sample, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    followup = deepcopy(base)
                    followup['captured_at'] = '2025-01-02T00:00:00Z'
                    followup['sections']['gossip_quests'] = {
                        'coverage': section['coverage'],
                        'available': {'2': {'repeatable': True}, '3': {}},
                        'active': {'10': {}, '20': {'is_complete': False}, '22': {'is_complete': True}},
                    }
                    followup['sections']['quests'] = {
                        '20': {'title':'Fixture quest','level':10,'ready_for_turn_in':True},
                        '22': {'title':'Unknown quest','level':10},
                        '10': {'title':'Failed quest','level':10,'ready_for_turn_in':True,'failed':True},
                    }
                    followup_key = store.ingest(followup)
                    reply = await session.call_tool('character_summary', {'snapshot_id': followup_key})
                    self.assertFalse(reply.isError)
                    observation = json.loads(reply.content[0].text)['gossip_quest_observations']
                    self.assertEqual(observation['available']['quest_log_matches'], {
                        'matched_quest_count': 0, 'rows': [], 'truncated': False,
                    })
                    self.assertEqual(observation['active']['quest_log_matches'], {
                        'matched_quest_count': 3, 'truncated': False,
                        'rows': [
                            {'quest_id':10,'quest_log_readiness':'failed','panel_is_complete':None},
                            {'quest_id':20,'quest_log_readiness':'ready','panel_is_complete':False},
                            {'quest_id':22,'quest_log_readiness':'unknown','panel_is_complete':True},
                        ],
                    })
                    async def compare(after, enabled=True):
                        reply = await session.call_tool('compare_snapshots', {
                            'before_id': valid[1][0], 'after_id': after, 'gossip_quest_changes': enabled,
                        })
                        self.assertFalse(reply.isError)
                        return json.loads(reply.content[0].text)
                    changes = (await compare(followup_key))['gossip_quest_changes']
                    self.assertEqual(changes['available'], {
                        'observed_only_before': [10], 'observed_only_after': [3], 'flag_changes': [],
                    })
                    self.assertEqual(changes['active'], {
                        'observed_only_before': [21], 'observed_only_after': [10],
                        'flag_changes': [{'quest_id': 20, 'fields': {'is_complete': {'before': True, 'after': False}}}],
                    })
                    self.assertFalse(changes['same_npc_verified'])
                    self.assertFalse(changes['acceptance_eligibility_verified'])
                    self.assertFalse(changes['turn_in_eligibility_verified'])
                    partial = (await compare(valid[2][0]))['gossip_quest_changes']
                    self.assertEqual(partial['available']['observed_only_before'], [2, 10])
                    self.assertIsNone(partial['active'])
                    self.assertNotIn('gossip_quest_changes', await compare(followup_key, False))
                    for snapshot_id, quest_id, in_log, available, active in (
                        (followup_key,20,True,{'present_in_saved_list':False,'observation':None},
                         {'present_in_saved_list':True,'observation':{'is_complete':False}}),
                        (followup_key,3,False,{'present_in_saved_list':True,'observation':{}},
                         {'present_in_saved_list':False,'observation':None}),
                        (valid[2][0],2,None,{'present_in_saved_list':False,'observation':None},None),
                        (valid[0][0],2,None,None,None),
                    ):
                        reply=await session.call_tool('inspect_snapshot',{
                            'snapshot_id':snapshot_id,'section':'quests','quest_id':quest_id,
                            'quest_gossip_observation':True,
                        })
                        self.assertFalse(reply.isError)
                        result=json.loads(reply.content[0].text)
                        self.assertIs(result['quest_observed'],in_log)
                        observation=result['quest_gossip_observation']
                        if snapshot_id == valid[0][0]:
                            self.assertIsNone(observation)
                        else:
                            self.assertEqual(observation['available'],available)
                            self.assertEqual(observation['active'],active)
                            self.assertFalse(observation['turn_in_eligibility_verified'])
                    invalid=await session.call_tool('inspect_snapshot',{
                        'snapshot_id':followup_key,'section':'quests','quest_gossip_observation':True,
                    })
                    self.assertTrue(invalid.isError)
                    for query, expected in (
                        ({'quest_id':10,'list':'available','repeatable':False},[valid[1][0]]),
                        ({'quest_id':10,'list':'active'},[followup_key,valid[3][0]]),
                        ({'quest_id':10,'list':'active','repeatable':False},[]),
                        ({'quest_id':20,'list':'active','is_complete':False},[followup_key]),
                        ({'quest_id':22,'list':'active','is_complete':False},[]),
                        ({'quest_id':20,'list':'active','is_complete':False,'quest_log_readiness':'ready'},[followup_key]),
                        ({'quest_id':20,'list':'active','quest_log_readiness':'not_ready'},[]),
                        ({'quest_id':22,'list':'active','quest_log_readiness':'unknown'},[followup_key]),
                        ({'quest_id':10,'list':'active','quest_log_readiness':'failed'},[followup_key]),
                        ({'quest_id':2,'list':'available','quest_log_readiness':'unknown'},[]),
                    ):
                        reply = await session.call_tool('list_snapshots', {'family':'forever','gossip_quest':query})
                        self.assertFalse(reply.isError)
                        rows = json.loads(reply.content[0].text)['snapshots']
                        self.assertEqual([row['snapshot_id'] for row in rows],expected)
                        if rows:
                            match = rows[0]['gossip_quest_match']
                            self.assertEqual((match['quest_id'],match['list']),(query['quest_id'],query['list']))
                            self.assertFalse(match['turn_in_eligibility_verified'])
                            if 'quest_log_readiness' in query:
                                self.assertEqual(match['quest_log_readiness'],query['quest_log_readiness'])
                    for family, query in (
                        ('retail',{'quest_id':10,'list':'active'}),
                        ('forever',{'quest_id':0,'list':'active'}),
                        ('forever',{'quest_id':10,'list':'PRIVATE'}),
                        ('forever',{'quest_id':10,'list':'active','quest_log_readiness':'PRIVATE'}),
                        ('forever',{'quest_id':10,'list':'active','is_complete':0}),
                    ):
                        reply=await session.call_tool('list_snapshots',{'family':family,'gossip_quest':query})
                        self.assertTrue(reply.isError)
                        self.assertNotIn('PRIVATE',str(reply))
                    for key, value in valid:
                        response = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertFalse(response.isError)
                        summary = json.loads(response.content[0].text)['gossip_quest_observations']
                        if value is None:
                            self.assertIsNone(summary)
                            continue
                        self.assertFalse(summary['acceptance_eligibility_verified'])
                        self.assertFalse(summary['turn_in_eligibility_verified'])
                        for kind in ['available', 'active']:
                            rows = value.get(kind)
                            if rows is None:
                                self.assertIsNone(summary[kind])
                                continue
                            ids = sorted(map(int, rows))
                            complete = [i for i in ids if rows[str(i)].get('is_complete') is True]
                            self.assertEqual(summary[kind]['observed_quest_count'], len(ids))
                            self.assertNotIn('quest_log_matches', summary[kind])
                            self.assertEqual(summary[kind]['quest_ids'], ids[:50])
                            self.assertEqual(summary[kind]['quest_ids_truncated'], len(ids) > 50)
                            self.assertEqual(summary[kind]['reported_complete_quest_ids'], complete[:50])
                            self.assertEqual(summary[kind]['reported_complete_quest_count'], len(complete))
                            self.assertEqual(summary[kind]['reported_complete_truncated'], len(complete) > 50)
                            self.assertEqual(summary[kind]['quests_with_unknown_completion'], sum('is_complete' not in row for row in rows.values()))
                        inspected = await session.call_tool('inspect_snapshot', {'snapshot_id': key, 'section': 'gossip_quests'})
                        self.assertFalse(inspected.isError)
                        self.assertEqual(json.loads(inspected.content[0].text)['data'], value)
                    for key in bad_ids:
                        response = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertTrue(response.isError)
                        self.assertNotIn('PRIVATE_GOSSIP', str(response))


if __name__ == '__main__':
    unittest.main()
