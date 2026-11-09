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
                        '10': {'item_id':1,'count':3,'link':'PRIVATE_LINK','guid':'Item-Private',
                               'reported_stack_limit':20},
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
                    lookup=await session.call_tool('find_snapshot_item',{
                        'snapshot_id':store.ingest(stacks),'item_id':1})
                    self.assertFalse(lookup.isError,lookup)
                    self.assertNotIn('PRIVATE',lookup.content[0].text)
                    self.assertEqual([row['reported_stack_limit'] for row in
                        json.loads(lookup.content[0].text)['locations'] if 'reported_stack_limit' in row],[20])
                    for invalid in [None,True,0,2,2**32]:
                        malformed=deepcopy(stacks)
                        malformed['sections']['bags']['0']['10']['reported_stack_limit']=invalid
                        with self.assertRaises(ValueError): store.ingest(malformed)
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
                        projection = groups['groups'][0]['capacity_projection']
                        fields = (
                            'observed_counts_fit_assumption','minimum_stacks_under_assumption',
                            'maximum_slots_freed_under_assumption','maximum_disjoint_whole_stack_pairs')
                        self.assertEqual(tuple(projection[field] for field in fields),expected)
                        self.assertEqual(groups['capacity_projection_summary'],{
                            'estimated_group_count':int(expected[0]),
                            'inconsistent_group_count':int(not expected[0]),
                            'maximum_slots_freed_across_estimated_groups':expected[2] or 0,
                            'maximum_disjoint_whole_stack_pairs_across_estimated_groups':expected[3] or 0})
                        self.assertEqual(projection['whole_stack_pair_examples'],
                            None if capacity == 6 else [] if capacity == 7 else [{
                                'stacks':[{'bag':0,'slot':10,'count':3},{'bag':0,'slot':2,'count':7}],
                                'combined_count':10}])
                        self.assertEqual(projection['whole_stack_pair_examples_truncated'],
                            None if capacity == 6 else False)
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
                    for offset, count, more in [(0,20,True),(20,2,False),(22,0,False)]:
                        page = await session.call_tool('find_snapshot_item', {
                            'snapshot_id':store.ingest(many),'item_id':1,'bag_stack_groups':True,
                            'assumed_max_stack_size':2,'stack_group_offset':offset})
                        self.assertFalse(page.isError,page)
                        data = json.loads(page.content[0].text)['bag_stack_groups']
                        self.assertEqual((data['group_offset'],len(data['groups']),data['has_more_groups']),
                                         (offset,count,more))
                        self.assertEqual(data['capacity_projection_summary'],groups['capacity_projection_summary'])
                        if offset == 20:
                            self.assertEqual(data['groups'][0]['locations'][0]['slot'],41)
                            self.assertEqual(data['groups'][0]['capacity_projection']['whole_stack_pair_examples'][0]['combined_count'],2)
                    refused = await session.call_tool('find_snapshot_item', {
                        'snapshot_id':store.ingest(stacks),'item_id':1,'assumed_max_stack_size':10})
                    self.assertTrue(refused.isError)
                    banks = deepcopy(stacks)
                    banks['sections']['character_bank'] = {'6': {
                        '1':{'item_id':1,'count':2,'link':'PRIVATE_LINK'},
                        '2':{'item_id':1,'count':3,'link':'PRIVATE_LINK'}}}
                    banks['sections']['account_bank'] = {}
                    async def bank_review(snapshot, storage):
                        reply = await session.call_tool('find_snapshot_item', {
                            'snapshot_id':store.ingest(snapshot),'item_id':1,
                            'bag_stack_groups':True,'stack_group_storage':storage,'assumed_max_stack_size':5})
                        self.assertFalse(reply.isError,reply)
                        self.assertNotIn('PRIVATE',reply.content[0].text)
                        return json.loads(reply.content[0].text)['bag_stack_groups']
                    bank = await bank_review(banks, 'character_bank')
                    self.assertEqual(bank['storage'],'character_bank')
                    self.assertEqual(bank['groups'][0]['total_quantity'],5)
                    self.assertEqual(bank['groups'][0]['locations'],[
                        {'container':6,'slot':1,'count':2},{'container':6,'slot':2,'count':3}])
                    self.assertEqual((await bank_review(banks, 'account_bank'))['group_count'],0)
                    self.assertIsNone(await bank_review(stacks, 'account_bank'))
                    banks['sections']['character_bank']['7']={
                        '1':{'item_id':1,'count':2,'link':'PRIVATE_LINK'}}
                    banks['sections']['character_bank']['8']={}
                    scoped_query={'snapshot_id':store.ingest(banks),'item_id':1,
                                  'bag_stack_groups':True,'stack_group_storage':'character_bank',
                                  'assumed_max_stack_size':5}
                    for container,expected in [(6,1),(7,0),(8,0),(9,None)]:
                        reply=await session.call_tool('find_snapshot_item',{
                            **scoped_query,'stack_group_container_id':container})
                        self.assertFalse(reply.isError,reply)
                        self.assertNotIn('PRIVATE',reply.content[0].text)
                        scoped=json.loads(reply.content[0].text)
                        self.assertEqual(scoped['quantity_by_storage']['character_bank'],7)
                        groups=scoped['bag_stack_groups']
                        if expected is None:
                            self.assertIsNone(groups)
                        else:
                            self.assertEqual(groups['group_count'],expected)
                            self.assertEqual(groups['container_id'],container)
                            self.assertEqual(groups['capacity_projection_summary']
                                ['maximum_slots_freed_across_estimated_groups'],expected)
                    for options in [{'stack_group_container_id':65},
                                    {'stack_group_container_id':6,'bag_stack_groups':False}]:
                        reply=await session.call_tool('find_snapshot_item',{**scoped_query,**options})
                        self.assertTrue(reply.isError)
                    inspected = await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'bag_item_cooldowns'})
                    reported = deepcopy(stacks)
                    reported['sections']['bags']['0'] = {
                        str(index): {'item_id':1,'count':count,'link':link,
                            **({'reported_stack_limit':limit} if limit is not None else {})}
                        for index,(count,link,limit) in enumerate([
                            (2,'PRIVATE_A',10),(3,'PRIVATE_A',10),
                            (2,'PRIVATE_B',20),(3,'PRIVATE_B',None),
                            (2,'PRIVATE_C',20),(3,'PRIVATE_C',30)],1)
                    }
                    options={'snapshot_id':store.ingest(reported),'item_id':1,
                             'bag_stack_groups':True,'use_reported_stack_limits':True}
                    for offset in [0,1,99]:
                        estimated=await session.call_tool('find_snapshot_item',{
                            **options,'stack_group_offset':offset})
                        self.assertFalse(estimated.isError,estimated)
                        self.assertNotIn('PRIVATE',estimated.content[0].text)
                        result=json.loads(estimated.content[0].text)['bag_stack_groups']
                        self.assertEqual(result['reported_capacity_projection_summary'],{
                            'estimated_group_count':1,'unestimated_group_count':2,
                            'maximum_slots_freed_across_estimated_groups':1,
                            'maximum_disjoint_whole_stack_pairs_across_estimated_groups':1})
                        self.assertEqual([group['reported_stack_limit_status'] for group in result['groups']],
                            ['agreed','incomplete','conflicting'][offset:])
                        for group in result['groups']:
                            if group['reported_stack_limit_status']=='agreed':
                                self.assertEqual(group['reported_stack_limit'],10)
                                self.assertEqual(group['capacity_projection']['maximum_slots_freed_under_assumption'],1)
                            else:
                                self.assertIsNone(group['reported_stack_limit'])
                                self.assertNotIn('capacity_projection',group)
                    for invalid in [{'assumed_max_stack_size':10},{'bag_stack_groups':False}]:
                        rejected=await session.call_tool('find_snapshot_item',{**options,**invalid})
                        self.assertTrue(rejected.isError)
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
