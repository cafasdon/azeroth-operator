"""Actual MCP comparisons distinguish slot observations from queue continuity."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class BattlefieldQueueCompareTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_positions_missing_capacity_status_and_gates(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        def snapshot(day,statuses,name='Fixture'):
            saved=fixture();saved['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            saved['character']['name']=name;saved['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            if statuses is not None:saved['sections']['battlefield_queue_slots']={
                'coverage':'reported_battlefield_queue_slots','reported_slot_count':len(statuses),
                'entries':{str(i):{'reported_status':status} for i,status in enumerate(statuses,1)}}
            return saved
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder))
            cases={'before':snapshot(1,['none','queued','confirm','active','locked']),
                'after':snapshot(2,['queued','queued','none']),
                'missing_old':snapshot(1,None),'missing_new':snapshot(2,None),
                'zero_old':snapshot(1,[]),'zero_new':snapshot(2,[]),
                'max_old':snapshot(1,['none']*32),'max_new':snapshot(2,['confirm']*32),
                'other_char':snapshot(2,[],name='Other')}
            keys={key:store.ingest(value) for key,value in cases.items()}
            private=SnapshotStore(Path(foreign)).ingest(snapshot(2,['queued'],name='PRIVATE_OTHER_OWNER'))
            retail=fixture();retail_after=deepcopy(retail);retail_after['captured_at']='2025-01-02T00:00:00Z'
            retail_keys=[store.ingest(value) for value in [retail,retail_after]]
            original={p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(before,after,flag=True,error=False,omit=False):
                        args={'before_id':before,'after_id':after}
                        if not omit:args['battlefield_queue_changes']=flag
                        reply=await session.call_tool('compare_snapshots',args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    result=(await call(keys['before'],keys['after']))['battlefield_queue_changes']
                    self.assertEqual(result['before_reported_slot_count'],5);self.assertEqual(result['after_reported_slot_count'],3)
                    self.assertTrue(result['reported_slot_capacity_changed'])
                    self.assertEqual(result['changed_slot_count'],4);self.assertEqual(result['unchanged_recorded_slot_count'],1)
                    self.assertEqual(result['slot_changes'],[
                        {'slot':1,'kind':'reported_status_changed','before_reported_status':'none','after_reported_status':'queued'},
                        {'slot':3,'kind':'reported_status_changed','before_reported_status':'confirm','after_reported_status':'none'},
                        {'slot':4,'kind':'observed_only_before','before_reported_status':'active','after_reported_status':None},
                        {'slot':5,'kind':'observed_only_before','before_reported_status':'locked','after_reported_status':None}])
                    for flag in ['slot_identity_verified','same_queue_continuity_verified','current_queue_membership_verified',
                        'complete_queue_coverage_verified','acceptance_eligibility_verified','join_or_leave_inferred','game_execution','live']:
                        self.assertFalse(result[flag])
                    for before,after in [('missing_old','missing_new'),('before','missing_new'),('missing_old','after')]:
                        result=(await call(keys[before],keys[after]))['battlefield_queue_changes']
                        for field in ['reported_slot_capacity_changed','changed_slot_count','unchanged_recorded_slot_count','slot_changes']:
                            self.assertIsNone(result[field])
                        self.assertEqual(result['before_reported_slot_count'] is None,before.startswith('missing'))
                        self.assertEqual(result['after_reported_slot_count'] is None,after.startswith('missing'))
                    empty=(await call(keys['zero_old'],keys['zero_new']))['battlefield_queue_changes']
                    self.assertEqual(empty['slot_changes'],[]);self.assertEqual(empty['changed_slot_count'],0)
                    self.assertEqual(empty['unchanged_recorded_slot_count'],0);self.assertFalse(empty['reported_slot_capacity_changed'])
                    added=(await call(keys['zero_old'],keys['max_new']))['battlefield_queue_changes']
                    self.assertEqual([row['slot'] for row in added['slot_changes']],list(range(1,33)))
                    self.assertTrue(all(row['kind']=='observed_only_after' and row['before_reported_status'] is None for row in added['slot_changes']))
                    full=(await call(keys['max_old'],keys['max_new']))['battlefield_queue_changes']
                    self.assertEqual(full['changed_slot_count'],32);self.assertEqual(len(full['slot_changes']),32)
                    self.assertFalse(full['reported_slot_capacity_changed'])
                    same=(await call(keys['before'],keys['before']))['battlefield_queue_changes']
                    self.assertEqual(same['slot_changes'],[]);self.assertEqual(same['unchanged_recorded_slot_count'],5)
                    plain=await call(keys['before'],keys['after'],omit=True)
                    self.assertNotIn('battlefield_queue_changes',plain)
                    self.assertEqual(await call(keys['before'],keys['after'],flag=False),plain)
                    for invalid in [None,'PRIVATE',0,1,1.0,{},[]]:
                        await call(keys['before'],keys['after'],flag=invalid,error=True)
                    await call(keys['after'],keys['before'],error=True)
                    await call(keys['before'],keys['other_char'],error=True)
                    await call(keys['before'],private,error=True)
                    await call(*retail_keys,error=True)
            for path,content in original.items():self.assertEqual(path.read_bytes(),content)


if __name__=='__main__':unittest.main()
