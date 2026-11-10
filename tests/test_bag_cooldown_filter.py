"""Historical timer predicates retain unknowns, numeric order and bounded pages."""
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


class BagCooldownFilterTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_predicates_unknowns_paging_and_legacy(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        value=fixture(); value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        rows=[(0,1,100,True,0),(0,2,101,False,60),(2,2,100,None,None),
            (2,12,102,True,30),(10,2,100,True,4294967295),(10,10,101,None,None)]
        bags={}; entries={}
        for bag,slot,item,enabled,remaining in rows:
            bags.setdefault(str(bag),{})[str(slot)]={'item_id':item,'count':1}
            row={'item_id':item,'status':'unavailable' if enabled is None else 'reported'}
            if enabled is not None: row.update(enabled=enabled,remaining_seconds=remaining)
            entries.setdefault(str(bag),{})[str(slot)]=row
        value['sections']['bags']=bags
        value['sections']['bag_item_cooldowns']={'coverage':'captured_carried_items_only','captured_item_locations':6,'entries':entries}
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); store=SnapshotStore(base); key=store.ingest(value)
            legacy=deepcopy(value); del legacy['sections']['bag_item_cooldowns']; legacy_key=store.ingest(legacy)
            empty=deepcopy(value); empty['sections']['bags']={}
            empty['sections']['bag_item_cooldowns'].update(entries={},captured_item_locations=0); empty_key=store.ingest(empty)
            retail=fixture(); retail_key=store.ingest(retail)
            large=deepcopy(empty)
            for index in range(4096):
                bag,slot=str(index//1024),str(index%1024+1)
                large['sections']['bags'].setdefault(bag,{})[slot]={'item_id':100,'count':1}
                large['sections']['bag_item_cooldowns']['entries'].setdefault(bag,{})[slot]={'item_id':100,'status':'unavailable'}
            large['sections']['bag_item_cooldowns']['captured_item_locations']=4096; large_key=store.ingest(large)
            protected={path:path.read_bytes() for path in base.glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def query(filter=None,snapshot=key,section='bag_item_cooldowns',error=False,**options):
                        arguments={'snapshot_id':snapshot,'section':section,**options}
                        if filter is not None: arguments['bag_cooldown_filter']=filter
                        reply=await session.call_tool('inspect_snapshot',arguments)
                        self.assertEqual(bool(reply.isError),error,reply)
                        return None if error else json.loads(reply.content[0].text)
                    raw=await query()
                    self.assertEqual(raw['data'],value['sections']['bag_item_cooldowns'])
                    all_rows=(await query({'limit':50}))['data']
                    reference='2025-01-01T00:00:30Z'
                    projected=(await query({'limit':4},bag_cooldown_reference_at=reference))['data']
                    self.assertEqual(projected['reference_at'],reference)
                    self.assertEqual([row['projected_cooldown_end_at'] for row in projected['rows']],
                                     ['2025-01-01T00:00:00Z',None,None,reference])
                    self.assertEqual([row['projected_end_at_or_before_reference'] for row in projected['rows']],
                                     [True,None,None,True])
                    self.assertEqual(projected['rows'][1]['remaining_seconds'],60)
                    self.assertFalse(projected['current_usability_verified'])
                    missing=(await query({'limit':1},snapshot=legacy_key,bag_cooldown_reference_at=reference))['data']
                    self.assertIsNone(missing['rows'])
                    await query(error=True,bag_cooldown_reference_at=reference)
                    self.assertEqual([(row['bag'],row['slot']) for row in all_rows['rows']],[(row[0],row[1]) for row in rows])
                    cases=[({'item_ids':[100]},[0,2,4],0,3),({'enabled':True},[0,3,4],2,1),
                        ({'maximum_remaining_seconds':0},[0],2,3),
                        ({'minimum_remaining_seconds':31,'maximum_remaining_seconds':100},[1],2,3),
                        ({'status':'unavailable'},[2,5],0,4),
                        ({'status':'unavailable','enabled':False},[],2,4),
                        ({'item_ids':[999],'enabled':True},[],0,6),
                        ({'item_ids':[100],'maximum_remaining_seconds':0},[0],1,4),
                        ({'status':'reported','maximum_remaining_seconds':0},[0],0,5),
                        ({'item_ids':list(range(1,64))+[100]},[0,2,4],0,3),
                        ({'minimum_remaining_seconds':4294967295},[4],2,3)]
                    for filter,indices,unknown,nonmatching in cases:
                        with self.subTest(filter=filter):
                            data=(await query(filter))['data']
                            self.assertEqual(data['rows'],[all_rows['rows'][index] for index in indices])
                            self.assertEqual(data['matched_item_locations'],len(indices))
                            self.assertEqual(data['unknown_filter_item_locations'],unknown)
                            self.assertEqual(data['nonmatching_item_locations'],nonmatching)
                            self.assertFalse(data['current_usability_verified']); self.assertFalse(data['game_execution'])
                    for offset in [0,1,2,4,6,7,18446744073709551615]:
                        data=(await query({'limit':2,'offset':offset}))['data']
                        self.assertEqual(data['rows'],all_rows['rows'][offset:offset+2])
                        self.assertEqual(data['matched_item_locations'],6)
                        self.assertEqual(data['has_more'],offset<4)
                    absent=(await query({'limit':1},legacy_key))
                    self.assertFalse(absent['available'])
                    for field in ['rows','captured_item_locations','matched_item_locations',
                            'unknown_filter_item_locations','nonmatching_item_locations','has_more']:
                        self.assertIsNone(absent['data'][field])
                    empty_result=await query({'limit':1},empty_key)
                    self.assertTrue(empty_result['available']); self.assertEqual(empty_result['data']['rows'],[])
                    self.assertEqual(empty_result['data']['matched_item_locations'],0)
                    large_result=(await query({'limit':50},large_key))['data']
                    self.assertEqual(large_result['matched_item_locations'],4096)
                    self.assertEqual(len(large_result['rows']),50)
                    self.assertEqual(large_result['rows'][-1]['slot'],50)
                    async def compare(after, item_id, before=key, error=False):
                        reply = await session.call_tool('compare_snapshots', {
                            'before_id':before,'after_id':after,'bag_cooldown_item_id':item_id})
                        self.assertEqual(bool(reply.isError),error,reply)
                        if not error:
                            return json.loads(reply.content[0].text)['bag_cooldown_item_observations']

                    compared = await compare(empty_key, 100)
                    self.assertEqual(compared['before'], {
                        'matching_item_locations':3,'reported_item_locations':2,
                        'unavailable_item_locations':1,'enabled_item_locations':2,
                        'zero_timer_enabled_item_locations':1,'minimum_enabled_remaining_seconds':0})
                    self.assertEqual(compared['after']['matching_item_locations'], 0)
                    self.assertIsNone(compared['after']['minimum_enabled_remaining_seconds'])
                    self.assertIs(compared['same_physical_item_verified'], False)
                    self.assertIs(compared['current_usability_verified'], False)
                    self.assertIsNone((await compare(legacy_key, 100))['after'])
                    self.assertIsNone((await compare(key, 101))['before']['minimum_enabled_remaining_seconds'])
                    self.assertEqual((await compare(large_key, 100))['after']['unavailable_item_locations'], 4096)
                    await compare(key, 0, error=True)
                    await compare(retail_key, 100, before=retail_key, error=True)
                    invalid=[{}, {'item_ids':[]},{'item_ids':[100,100]},{'item_ids':[0]},
                        {'item_ids':[True]},{'item_ids':list(range(1,66))},{'status':'ready'},
                        {'enabled':1},{'minimum_remaining_seconds':True},{'minimum_remaining_seconds':-1},
                        {'minimum_remaining_seconds':1.5},{'minimum_remaining_seconds':2,'maximum_remaining_seconds':1},
                        {'maximum_remaining_seconds':4294967296},{'limit':0},{'limit':51},{'limit':True},
                        {'offset':-1},{'offset':True},{'PRIVATE':1}]
                    for filter in invalid: await query(filter,error=True)
                    for section in [None,'bags','equipment']: await query({'limit':1},section=section,error=True)
                    await query({'limit':1},retail_key,error=True)
                    self.assertEqual((await query())['data'],raw['data'])
            for path,data in protected.items(): self.assertEqual(path.read_bytes(),data)


if __name__=='__main__':
    unittest.main()
