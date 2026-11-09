"""All material requirements must be covered within one historical storage."""
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


class InventoryRequirementsDiscoveryTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_all_materials_no_pooling_latest_and_professions(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        def sample(name,day,items=None,family='forever'):
            value=fixture(); value['client']={'family':family,'variant':'live','build':'70862','interface':16001}
            value['character']['name']=name; value['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            if items is not None:
                value['sections']['bags']={'0':{str(index):{'item_id':item,'count':quantity}
                    for index,(item,quantity) in enumerate(items,1)}}
            return value
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryDirectory() as other:
            base=Path(folder); store=SnapshotStore(base)
            store.ingest(sample('A',1,[(100,10)])); store.ingest(sample('B',2,[(200,5)]))
            complete=[(100,3),(100,7),(200,2),(200,3),(777,999)]
            c=sample('C',3,complete)
            c['sections']['professions']={'1':{'name':'Synthetic','rank':300,'max_rank':300,'skill_line':164}}
            shared={'7':{'1':{'item_id':100,'count':10},'2':{'item_id':200,'count':5}}}
            c['sections']['account_bank']=deepcopy(shared); c_key=store.ingest(c)
            d=sample('D',4,[(100,10)]); d['sections']['character_bank']={'6':{'1':{'item_id':200,'count':5}}}; store.ingest(d)
            e_old=store.ingest(sample('E',1,complete)); store.ingest(sample('E',5,[(100,10)]))
            f_old=store.ingest(sample('F',2,complete)); store.ingest(sample('F',6))
            g=sample('G',7,[]); g['sections']['equipment']={'11':{'item_id':100},'12':{'item_id':200}}; g_key=store.ingest(g)
            h=sample('H',7,[]); h['sections']['account_bank']=deepcopy(shared); h_key=store.ingest(h)
            boundary_key=store.ingest(sample('Boundary',8,[(item,1) for item in range(1,65)]))
            retail_key=store.ingest(sample('Retail',9,complete,'retail'))
            classic_key=store.ingest(sample('Classic',9,complete,'classic'))
            SnapshotStore(Path(other)).ingest(sample('PRIVATE_OTHER_OWNER',9,complete))
            protected={path:path.read_bytes() for path in base.glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            requirements={'storage':'bags','items':[{'item_id':200,'quantity':5},{'item_id':100,'quantity':10}]}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def query(filter=requirements,error=False,**extra):
                        arguments={'family':'forever',**extra}
                        if filter is not None: arguments['inventory_requirements']=filter
                        reply=await session.call_tool('list_snapshots',arguments)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE_OTHER_OWNER',str(reply)); self.assertNotIn(other,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    history=await query()
                    self.assertEqual({row['snapshot_id'] for row in history['snapshots']},{c_key,e_old,f_old})
                    latest=await query(latest_per_character=True)
                    self.assertEqual([row['snapshot_id'] for row in latest['snapshots']],[c_key])
                    match=latest['snapshots'][0]['inventory_requirements_match']
                    self.assertEqual(match['items'],[{'item_id':200,'required_quantity':5,'observed_quantity':5},
                        {'item_id':100,'required_quantity':10,'observed_quantity':10}])
                    self.assertTrue(match['all_requirements_covered_by_observed_quantities'])
                    self.assertEqual(match['basis'],'same_snapshot_single_storage')
                    for field in ['complete_ownership_verified','current_access_verified','shared_storage_scope_verified',
                            'cross_character_aggregation_performed','craftability_verified','transferability_verified','game_execution']:
                        self.assertFalse(match[field])
                    for storage in ['bags','character_bank']:
                        self.assertEqual((await query({**requirements,'storage':storage},character={'name':'D','realm':'Test'}))['snapshots'],[])
                    self.assertEqual([row['snapshot_id'] for row in (await query(profession_skill_line=164,profession_min_rank=300))['snapshots']],[c_key])
                    self.assertEqual((await query(profession_skill_line=164,profession_min_rank=301))['snapshots'],[])
                    older=await query(latest_per_character=True,captured_through='2025-01-02T00:00:00Z')
                    self.assertEqual({row['snapshot_id'] for row in older['snapshots']},{e_old,f_old})
                    self.assertEqual((await query(character={'name':'e','realm':'TEST'},latest_per_character=True))['snapshots'],[])
                    shared_result=await query({**requirements,'storage':'account_bank'})
                    self.assertEqual({row['snapshot_id'] for row in shared_result['snapshots']},{c_key,h_key})
                    self.assertTrue(all(row['inventory_requirements_match']['items']==match['items'] for row in shared_result['snapshots']))
                    short=deepcopy(requirements); short['items'][0]['quantity']=6
                    self.assertEqual((await query(short,latest_per_character=True))['snapshots'],[])
                    self.assertEqual((await query({**short,'storage':'account_bank'}))['snapshots'],[])
                    equip={'storage':'equipment','items':[{'item_id':200,'quantity':1},{'item_id':100,'quantity':1}]}
                    self.assertEqual([row['snapshot_id'] for row in (await query(equip))['snapshots']],[g_key])
                    boundary={'storage':'bags','items':[{'item_id':item,'quantity':1} for item in range(1,65)]}
                    self.assertEqual([row['snapshot_id'] for row in (await query(boundary))['snapshots']],[boundary_key])
                    for family,key in [('retail',retail_key),('classic',classic_key)]:
                        self.assertEqual([row['snapshot_id'] for row in (await query(family=family))['snapshots']],[key])
                        await query({**requirements,'storage':'account_bank'},family=family,error=True)
                    for offset in [0,1,3,18446744073709551615]:
                        page=await query(offset=offset,limit=1)
                        self.assertEqual(page['snapshots'],history['snapshots'][offset:offset+1])
                        self.assertEqual(page['has_more'],offset<2)
                    invalid=[{}, {'storage':'bags','items':[]}, {**requirements,'storage':'all'},
                        {'storage':'bags','items':[{'item_id':100,'quantity':1}]*2},
                        {'storage':'bags','items':[{'item_id':1,'quantity':0}]},
                        {'storage':'bags','items':[{'item_id':0,'quantity':1}]},
                        {'storage':'bags','items':[{'item_id':True,'quantity':1}]},
                        {'storage':'bags','items':[{'item_id':1,'quantity':True}]},
                        {'storage':'bags','items':[{'item_id':1,'quantity':1.5}]},
                        {'storage':'bags','items':[{'item_id':1,'quantity':18446744073709551616}]},
                        {'storage':'bags','items':[{'item_id':item,'quantity':1} for item in range(1,66)]},
                        {**requirements,'unsupported':other}]
                    for filter in invalid: await query(filter,error=True)
                    await query(inventory_item={'item_id':100,'storage':'bags','minimum_quantity':1},error=True)
                    maximum={'storage':'bags','items':[{'item_id':100,'quantity':18446744073709551615}]}
                    self.assertEqual((await query(maximum))['snapshots'],[])
                    ordinary=await query(None)
                    self.assertTrue(all('inventory_requirements_match' not in row for row in ordinary['snapshots']))
            for path,data in protected.items(): self.assertEqual(path.read_bytes(),data)


if __name__=='__main__':
    unittest.main()
