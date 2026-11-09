"""Strict cross-reader bag cooldown observations through the actual local MCP."""
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


def sample():
    value = fixture()
    value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
    value['sections']['bags'] = {'0':{'1':{'item_id':1,'count':1},'2':{'item_id':2,'count':1}},
        '1':{},'2':{'1':{'item_id':3,'count':1}}}
    value['sections']['bag_item_cooldowns'] = {'coverage':'captured_carried_items_only',
        'captured_item_locations':3,'entries':{'0':{
            '1':{'item_id':1,'status':'reported','enabled':True,'remaining_seconds':0},
            '2':{'item_id':2,'status':'reported','enabled':False,'remaining_seconds':15}},
            '1':{},'2':{'1':{'item_id':3,'status':'unavailable'}}}}
    return value


class BagItemCooldownTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_summary_and_cross_reader_guards(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            store = SnapshotStore(base)
            value = sample()
            key = store.ingest(value)
            before = (base/f'{key}.json').read_bytes()
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    stacks = sample()
                    stacks['sections'].pop('bag_item_cooldowns')
                    stacks['sections']['bags'] = {'0': {
                        '10': {'item_id':1,'count':3,'link':'PRIVATE_LINK','guid':'Item-Private'},
                        '2': {'item_id':1,'count':7,'link':'PRIVATE_LINK'},
                        '1': {'item_id':1,'count':1,'link':'OTHER_LINK'},
                        '3': {'item_id':1,'count':1},
                        '4': {'item_id':2,'count':1,'link':'PRIVATE_LINK'}}}
                    reply = await session.call_tool('find_snapshot_item', {
                        'snapshot_id':store.ingest(stacks),'item_id':1,'limit':1,'offset':99,'bag_stack_groups':True})
                    self.assertFalse(reply.isError,reply)
                    self.assertNotIn('PRIVATE',reply.content[0].text)
                    report = json.loads(reply.content[0].text)
                    self.assertEqual(report['locations'],[])
                    self.assertEqual(report['bag_stack_groups'], {
                        'group_count':1,'groups_truncated':False,'locations_without_link':1,
                        'stack_compatibility_verified':False,'game_execution':False,
                        'groups':[{'stack_count':2,'total_quantity':10,'locations_truncated':False,
                                   'locations':[{'bag':0,'slot':2,'count':7},{'bag':0,'slot':10,'count':3}]}]})
                    for capacity, expected in [(6,(False,None,None,None)),(7,(True,2,0,0)),(10,(True,1,1,1))]:
                        projected = await session.call_tool('find_snapshot_item', {
                            'snapshot_id':store.ingest(stacks),'item_id':1,'bag_stack_groups':True,
                            'assumed_max_stack_size':capacity})
                        self.assertFalse(projected.isError,projected)
                        groups = json.loads(projected.content[0].text)['bag_stack_groups']
                        self.assertEqual(groups['assumed_max_stack_size'],capacity)
                        self.assertEqual(groups['groups'][0]['capacity_projection'],dict(zip((
                            'observed_counts_fit_assumption','minimum_stacks_under_assumption',
                            'maximum_slots_freed_under_assumption','maximum_disjoint_whole_stack_pairs'),expected)))
                        self.assertEqual(groups['capacity_projection_summary'],{
                            'estimated_group_count':int(expected[0]),
                            'inconsistent_group_count':int(not expected[0]),
                            'maximum_slots_freed_across_estimated_groups':expected[2] or 0,
                            'maximum_disjoint_whole_stack_pairs_across_estimated_groups':expected[3] or 0})
                    many = deepcopy(stacks)
                    many['sections']['bags'] = {'0': {str(i):{
                        'item_id':1,'count':1 if i <= 42 else 3,'link':f'group:{(i-1)//2}'}
                        for i in range(1,45)}}
                    projected = await session.call_tool('find_snapshot_item', {
                        'snapshot_id':store.ingest(many),'item_id':1,'bag_stack_groups':True,
                        'assumed_max_stack_size':2})
                    self.assertFalse(projected.isError,projected)
                    groups = json.loads(projected.content[0].text)['bag_stack_groups']
                    self.assertEqual(len(groups['groups']),20)
                    self.assertEqual(groups['capacity_projection_summary'],{
                        'estimated_group_count':21,'inconsistent_group_count':1,
                        'maximum_slots_freed_across_estimated_groups':21,
                        'maximum_disjoint_whole_stack_pairs_across_estimated_groups':21})
                    refused = await session.call_tool('find_snapshot_item', {
                        'snapshot_id':store.ingest(stacks),'item_id':1,'assumed_max_stack_size':10})
                    self.assertTrue(refused.isError)
                    inspected = await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'bag_item_cooldowns'})
                    self.assertFalse(inspected.isError,inspected)
                    self.assertEqual(json.loads(inspected.content[0].text)['data'],value['sections']['bag_item_cooldowns'])
                    reply = await session.call_tool('character_summary',{'snapshot_id':key})
                    self.assertFalse(reply.isError,reply)
                    observed = json.loads(reply.content[0].text)['bag_item_cooldown_observation']
                    self.assertEqual(observed,{'coverage':'captured_carried_items_only','captured_item_locations':3,
                        'reported_item_locations':2,'unavailable_item_locations':1,'disabled_item_locations':1,
                        'positive_timer_item_locations':1,'current_usability_verified':False,'game_execution':False})
                    mutations = []
                    for field,bad in [('coverage','wrong'),('captured_item_locations',True),('captured_item_locations',2),
                            ('captured_item_locations',4097),('entries',None),('extra','PRIVATE')]:
                        broken = deepcopy(value); broken['sections']['bag_item_cooldowns'][field]=bad; mutations.append(broken)
                    for field,bad in [('item_id',0),('item_id',True),('item_id',4),('item_id',4294967296),
                            ('status','ready'),('enabled',1),('enabled',None),('remaining_seconds',True),
                            ('remaining_seconds',-1),('remaining_seconds',.5),('remaining_seconds',4294967296),('startTime',100)]:
                        broken = deepcopy(value); broken['sections']['bag_item_cooldowns']['entries']['0']['1'][field]=bad; mutations.append(broken)
                    for family in ['retail','classic']:
                        broken=deepcopy(value); broken['client']['family']=family; mutations.append(broken)
                    broken=deepcopy(value); del broken['sections']['bags']; mutations.append(broken)
                    broken=deepcopy(value); del broken['sections']['bag_item_cooldowns']['entries']['1']; mutations.append(broken)
                    broken=deepcopy(value); del broken['sections']['bag_item_cooldowns']['entries']['0']['1']; mutations.append(broken)
                    broken=deepcopy(value); broken['sections']['bag_item_cooldowns']['entries']['2']['1']['enabled']=False; mutations.append(broken)
                    for index,broken in enumerate(mutations):
                        with self.subTest(index=index):
                            with self.assertRaises(ValueError): validate_snapshot(broken)
                            data=json.dumps(broken,sort_keys=True,separators=(',',':')).encode()
                            bad_key=hashlib.sha256(data).hexdigest(); (base/f'{bad_key}.json').write_bytes(data)
                            reply=await session.call_tool('inspect_snapshot',{'snapshot_id':bad_key})
                            self.assertTrue(reply.isError,index)
                            self.assertNotIn('PRIVATE',str(reply))
                    for count in [0,4096]:
                        boundary=sample(); bags={}; entries={}
                        for index in range(count):
                            bag,slot=str(index//1024),str(index%1024+1)
                            bags.setdefault(bag,{})[slot]={'item_id':1,'count':1}
                            entries.setdefault(bag,{})[slot]={'item_id':1,'status':'unavailable'}
                        boundary['sections']['bags']=bags
                        boundary['sections']['bag_item_cooldowns'].update(entries=entries,captured_item_locations=count)
                        boundary_key=store.ingest(boundary)
                        reply=await session.call_tool('character_summary',{'snapshot_id':boundary_key})
                        self.assertFalse(reply.isError,reply)
                        summary=json.loads(reply.content[0].text)['bag_item_cooldown_observation']
                        self.assertEqual(summary['unavailable_item_locations'],count)
                    legacy=deepcopy(value); del legacy['sections']['bag_item_cooldowns']
                    reply=await session.call_tool('character_summary',{'snapshot_id':store.ingest(legacy)})
                    self.assertIsNone(json.loads(reply.content[0].text)['bag_item_cooldown_observation'])
            self.assertEqual((base/f'{key}.json').read_bytes(),before)


if __name__ == '__main__':
    unittest.main()
