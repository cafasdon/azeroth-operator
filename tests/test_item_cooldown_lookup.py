"""Item lookup keeps cooldown evidence with its original bag, character and time."""
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


class ItemCooldownLookupTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_exact_location_paging_and_multiple_captures(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); store=SnapshotStore(base); value=fixture()
            value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['equipment']={'11':{'item_id':100}}
            value['sections']['bags']={'0':{str(slot):{'item_id':100,'count':slot} for slot in [1,2,3]},
                '1':{'1':{'item_id':101,'count':99}}, '2':{}}
            value['sections']['character_bank']={'6':{'1':{'item_id':100,'count':20}}}
            value['sections']['bag_item_cooldowns']={'coverage':'captured_carried_items_only','captured_item_locations':4,
                'entries':{'0':{
                    '1':{'item_id':100,'status':'reported','enabled':True,'remaining_seconds':0},
                    '2':{'item_id':100,'status':'reported','enabled':False,'remaining_seconds':50},
                    '3':{'item_id':100,'status':'unavailable'}},
                    '1':{'1':{'item_id':101,'status':'reported','enabled':False,'remaining_seconds':100}},
                    '2':{}}}
            key=store.ingest(value)
            legacy=deepcopy(value); del legacy['sections']['bag_item_cooldowns']; legacy_key=store.ingest(legacy)
            alt=deepcopy(value); alt['character']['name']='Another'; alt['captured_at']='2025-01-02T00:00:00Z'
            alt['sections']['bag_item_cooldowns']['entries']['0']['1']['remaining_seconds']=7
            alt['sections']['account_bank']={}
            alt_key=store.ingest(alt)
            protected={p:p.read_bytes() for p in base.glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def find(snapshot=key,**extra):
                        reply=await session.call_tool('find_snapshot_item',{'snapshot_id':snapshot,'item_id':100,**extra})
                        self.assertFalse(reply.isError,reply)
                        return json.loads(reply.content[0].text)
                    result=await find()
                    self.assertEqual(result['matched_locations'],5)
                    self.assertNotIn('quantity_by_container',result)
                    self.assertEqual(result['quantity_by_storage'],{'equipment':1,'bags':6,'character_bank':20,'account_bank':None})
                    counts=result['bag_cooldown_observation']
                    self.assertEqual(counts,{'basis':'same_snapshot_exact_bag_slot_item',
                        'scope':'matching_bag_locations_before_pagination','reported_item_locations':2,
                        'unavailable_item_locations':1,'disabled_item_locations':1,'positive_timer_item_locations':1,
                        'current_usability_verified':False,'game_execution':False})
                    bag_rows=[row for row in result['locations'] if row['storage']=='bags']
                    self.assertEqual([row['captured_cooldown'] for row in bag_rows],
                        list(value['sections']['bag_item_cooldowns']['entries']['0'].values()))
                    self.assertTrue(all('captured_cooldown' not in row for row in result['locations'] if row['storage']!='bags'))
                    for offset in range(7):
                        page=await find(offset=offset,limit=1)
                        self.assertEqual(page['locations'],result['locations'][offset:offset+1])
                        self.assertEqual(page['bag_cooldown_observation'],counts)
                        self.assertEqual(page['matched_locations'],5)
                        self.assertEqual(page['has_more'],offset<4)
                    missing=await find(item_id=999)
                    self.assertEqual(missing['locations'],[])
                    self.assertEqual(missing['bag_cooldown_observation']['reported_item_locations'],0)
                    legacy_result=await find(legacy_key)
                    self.assertNotIn('bag_cooldown_observation',legacy_result)
                    expected=deepcopy(result); expected['snapshot_id']=legacy_key
                    del expected['bag_cooldown_observation']
                    for row in expected['locations']: row.pop('captured_cooldown',None)
                    self.assertEqual(legacy_result,expected)
                    container_quantities={'bags':{'0':6,'1':0,'2':0},'character_bank':{'6':20},'account_bank':None}
                    beyond=await find(include_container_quantities=True,offset=99,limit=1)
                    self.assertEqual(beyond['locations'],[])
                    self.assertEqual(beyond['quantity_by_container'],container_quantities)
                    combined=await find(additional_snapshot_ids=[alt_key,legacy_key],include_container_quantities=True)
                    other,old=combined['additional_snapshot_results']
                    self.assertEqual(combined['quantity_by_container'],container_quantities)
                    self.assertEqual(old['quantity_by_container'],container_quantities)
                    self.assertEqual(other['quantity_by_container'],{**container_quantities,'account_bank':{}})
                    self.assertEqual(other['captured_at'],alt['captured_at'])
                    self.assertEqual(next(row for row in other['locations'] if row['storage']=='bags')['captured_cooldown']['remaining_seconds'],7)
                    self.assertEqual(next(row for row in combined['locations'] if row['storage']=='bags')['captured_cooldown']['remaining_seconds'],0)
                    self.assertNotIn('bag_cooldown_observation',old)
            for path,data in protected.items(): self.assertEqual(path.read_bytes(),data)


if __name__=='__main__':
    unittest.main()
