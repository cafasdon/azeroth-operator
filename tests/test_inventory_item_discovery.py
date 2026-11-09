"""Saved-character stock discovery never aggregates characters or revives old stock."""
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


class InventoryItemDiscoveryTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_scopes_latest_unknowns_owner_and_paging(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        def sample(name,day,bags=None,family='forever'):
            value=fixture(); value['client']={'family':family,'variant':'live','build':'70862','interface':16001}
            value['character']['name']=name; value['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            if bags is not None: value['sections']['bags']=bags
            return value
        def bag(count): return {'0':{'1':{'item_id':100,'count':count}}}
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryDirectory() as other:
            base=Path(folder); store=SnapshotStore(base)
            a_old=store.ingest(sample('A',1,bag(5))); a_new=store.ingest(sample('a',2,{}))
            b_old=store.ingest(sample('B',1,bag(10))); b_new=store.ingest(sample('B',3))
            c=sample('C',4,bag(2)); c['sections']['bags']['0']['2']={'item_id':100,'count':4}
            c['sections']['bags']['0']['3']={'item_id':777,'count':7}
            c['sections']['bags']['1']={}
            c['sections']['equipment']={'11':{'item_id':100}}
            c['sections']['character_bank']={'6':{'1':{'item_id':100,'count':9}}}
            c['sections']['account_bank']={'7':{'1':{'item_id':100,'count':100}}}; c_key=store.ingest(c)
            d=sample('D',5,{}); d['sections']['equipment']={'11':{'item_id':100},'12':{'item_id':100}}; d_key=store.ingest(d)
            e=sample('E',6,bag(1)); e['sections']['account_bank']=deepcopy(c['sections']['account_bank']); e_key=store.ingest(e)
            retail_key=store.ingest(sample('Retail',7,bag(8),'retail'))
            classic_key=store.ingest(sample('Classic',7,bag(3),'classic'))
            SnapshotStore(Path(other)).ingest(sample('PRIVATE_OTHER_OWNER',8,bag(9999)))
            protected={path:path.read_bytes() for path in base.glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def query(filter=None,error=False,**extra):
                        arguments={'family':'forever',**extra}
                        if filter is not None: arguments['inventory_item']=filter
                        reply=await session.call_tool('list_snapshots',arguments)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE_OTHER_OWNER',str(reply)); self.assertNotIn(other,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    positive={'item_id':100,'storage':'bags','minimum_quantity':1}
                    history=await query(positive)
                    self.assertEqual({row['snapshot_id'] for row in history['snapshots']},{a_old,b_old,c_key,e_key})
                    latest=await query(positive,latest_per_character=True)
                    self.assertEqual([row['snapshot_id'] for row in latest['snapshots']],[e_key,c_key])
                    self.assertEqual([row['inventory_item_match']['observed_quantity'] for row in latest['snapshots']],[1,6])
                    for row in latest['snapshots']:
                        match=row['inventory_item_match']
                        self.assertEqual(match['storage'],'bags'); self.assertEqual(match['item_id'],100)
                        self.assertEqual(match['basis'],'same_snapshot_single_storage')
                        for field in ['complete_ownership_verified','current_access_verified','shared_storage_scope_verified',
                                'cross_character_aggregation_performed','game_execution']:
                            self.assertFalse(match[field])
                    self.assertEqual((await query(positive,character={'name':'a','realm':'TEST'}))['snapshots'][0]['snapshot_id'],a_old)
                    self.assertEqual((await query(positive,latest_per_character=True,character={'name':'a','realm':'TEST'}))['snapshots'],[])
                    bounded=await query(positive,latest_per_character=True,captured_through='2025-01-01T00:00:00Z')
                    self.assertEqual({row['snapshot_id'] for row in bounded['snapshots']},{a_old,b_old})
                    zero={'item_id':999,'storage':'bags','minimum_quantity':0,'maximum_quantity':0}
                    self.assertEqual({row['snapshot_id'] for row in (await query(zero,latest_per_character=True))['snapshots']},
                        {a_new,c_key,d_key,e_key})
                    self.assertNotIn(b_new,{row['snapshot_id'] for row in (await query(zero))['snapshots']})
                    scoped=await query({**positive,'container_id':0,'minimum_quantity':6},latest_per_character=True)
                    self.assertEqual([row['snapshot_id'] for row in scoped['snapshots']],[c_key])
                    match=scoped['snapshots'][0]['inventory_item_match']
                    self.assertEqual((match['basis'],match['container_id'],match['observed_quantity']),
                        ('same_snapshot_single_container',0,6))
                    empty=await query({**zero,'container_id':1},latest_per_character=True)
                    self.assertEqual([row['snapshot_id'] for row in empty['snapshots']],[c_key])
                    self.assertEqual((await query({**zero,'container_id':2}))['snapshots'],[])
                    equip={'item_id':100,'storage':'equipment','minimum_quantity':2}
                    self.assertEqual([row['snapshot_id'] for row in (await query(equip))['snapshots']],[d_key])
                    bank={'item_id':100,'storage':'character_bank','minimum_quantity':0}
                    self.assertEqual([row['snapshot_id'] for row in (await query(bank))['snapshots']],[c_key])
                    self.assertEqual([row['snapshot_id'] for row in
                        (await query({**bank,'container_id':6,'minimum_quantity':9}))['snapshots']],[c_key])
                    await query({**equip,'container_id':0},error=True)
                    await query({**positive,'container_id':65},error=True)
                    shared={'item_id':100,'storage':'account_bank','minimum_quantity':100}
                    shared_rows=(await query(shared))['snapshots']
                    self.assertEqual({row['snapshot_id'] for row in shared_rows},{c_key,e_key})
                    self.assertTrue(all(row['inventory_item_match']['observed_quantity']==100 for row in shared_rows))
                    self.assertEqual((await query({**shared,'minimum_quantity':101}))['snapshots'],[])
                    for family,key in [('retail',retail_key),('classic',classic_key)]:
                        self.assertEqual([row['snapshot_id'] for row in (await query(positive,family=family))['snapshots']],[key])
                        await query(bank,family=family,error=True)
                    for offset in [0,1,2,18446744073709551615]:
                        page=await query(positive,latest_per_character=True,offset=offset,limit=1)
                        self.assertEqual(page['snapshots'],latest['snapshots'][offset:offset+1])
                        self.assertEqual(page['has_more'],offset==0)
                    for invalid in [{},{'item_id':0,'storage':'bags','minimum_quantity':1},
                            {**positive,'storage':'all'},{**positive,'item_id':True},{**positive,'minimum_quantity':True},
                            {**positive,'minimum_quantity':-1},{**positive,'minimum_quantity':1.5},
                            {**positive,'maximum_quantity':0},{**positive,'maximum_quantity':18446744073709551616},
                            {**positive,'unsupported':other}]:
                        await query(invalid,error=True)
                    self.assertEqual((await query({**positive,'minimum_quantity':18446744073709551615}))['snapshots'],[])
                    self.assertTrue(all('inventory_item_match' not in row for row in (await query())['snapshots']))
            for path,data in protected.items(): self.assertEqual(path.read_bytes(),data)


if __name__=='__main__':
    unittest.main()
