"""Real MCP saved auction search, same-row joins, paging and owner boundaries."""
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


class OwnedAuctionFilterTests(unittest.IsolatedAsyncioTestCase):
    async def test_search_latest_same_row_paging_and_private_failures(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        def capture(name,day,rows):
            value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['character']['name']=name;value['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            value['sections']['bags']={'0':{'1':{'item_id':10,'count':2}}}
            if rows is not None:value['sections']['owned_auctions']={
                'coverage':'loaded_player_owned_auction_cache','entries':rows}
            return value
        rows={str(i):{'auction_id':9007199254740992-i,'item_id':10 if i%3==0 else 20,
            'quantity':4294967295,'reported_status':i%2} for i in range(1,257)}
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder))
            cases={'old':capture('Alpha',1,{'1':{'auction_id':1,'item_id':10,'quantity':3,'reported_status':0}}),
                'new':capture('Alpha',2,{}),'many':capture('Beta',1,rows),'missing':capture('Charlie',1,None)}
            keys={name:store.ingest(value) for name,value in cases.items()}
            private=SnapshotStore(Path(foreign)).ingest(capture('PRIVATE_OTHER_OWNER',1,rows))
            retail=fixture();retail_key=store.ingest(retail)
            before={p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),
                    env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    async def inspect(key,filter,**page):
                        return await call('inspect_snapshot',snapshot_id=key,section='owned_auctions',
                            owned_auction_filter={'filter':filter,**page})
                    history=await call('list_snapshots',family='forever',owned_auction={'item_id':10})
                    self.assertEqual({r['snapshot_id'] for r in history['snapshots']},{keys['old'],keys['many']})
                    latest=await call('list_snapshots',family='forever',latest_per_character=True,
                        owned_auction={'item_id':10})
                    self.assertEqual([r['snapshot_id'] for r in latest['snapshots']],[keys['many']])
                    matching=latest['snapshots'][0]['owned_auction_match']
                    self.assertEqual(matching['matching_auction_count'],85)
                    self.assertEqual(matching['matching_reported_item_units'],85*4294967295)
                    self.assertTrue(matching['matches_truncated']);self.assertEqual(len(matching['matching_auctions']),20)
                    combined=await call('list_snapshots',family='forever',owned_auction={'item_id':10},
                        inventory_item={'item_id':10,'storage':'bags','minimum_quantity':3})
                    self.assertEqual(combined['snapshots'],[])
                    for filter in [{'item_id':10},{'reported_status':0},{'reported_status':1},{'item_id':20,'reported_status':1},
                            {'auction_id':9007199254740991},{'item_id':10,'auction_id':9007199254740991},
                            {'item_id':4294967295},{'reported_status':4294967295}]:
                        expected=[{'slot':int(slot),'observation':row} for slot,row in sorted(rows.items(),key=lambda pair:int(pair[0]))
                            if all(row[key]==value for key,value in filter.items())]
                        for offset in [0,1,20,256,18446744073709551615]:
                            data=(await inspect(keys['many'],filter,offset=offset,limit=13))['data']
                            self.assertEqual(data['rows'],expected[offset:offset+13])
                            self.assertEqual(data['matching_auction_count'],len(expected))
                            self.assertEqual(data['captured_auction_count'],256)
                            self.assertEqual(data['matching_reported_item_units'],sum(r['observation']['quantity'] for r in expected))
                            self.assertEqual(data['has_more'],offset+13<len(expected))
                            for flag in ['complete_auction_coverage_verified','current_ownership_verified','sale_or_delivery_inferred',
                                    'counts_toward_inventory','counts_toward_wallet','game_execution','live']:
                                self.assertFalse(data[flag])
                    missing=(await inspect(keys['missing'],{'reported_status':0}))['data']
                    self.assertIsNone(missing['rows']);self.assertIsNone(missing['matching_reported_item_units'])
                    empty=(await inspect(keys['new'],{'reported_status':0}))['data']
                    self.assertEqual(empty['rows'],[]);self.assertEqual(empty['matching_reported_item_units'],0)
                    plain=await call('inspect_snapshot',snapshot_id=keys['many'],section='owned_auctions')
                    self.assertEqual(plain['data'],cases['many']['sections']['owned_auctions'])
                    for filter in [{},{'item_id':0},{'auction_id':0},{'auction_id':9007199254740992},
                            {'reported_status':4294967296},{'reported_status':-1},{'item_id':True},
                            {'auction_id':1.0},{'reported_status':'PRIVATE'},{'extra':'PRIVATE'}]:
                        await call('list_snapshots',True,family='forever',owned_auction=filter)
                        await call('inspect_snapshot',True,snapshot_id=keys['many'],section='owned_auctions',
                            owned_auction_filter={'filter':filter})
                    for page in [{'limit':0},{'limit':51},{'offset':-1},{'offset':1.0},{'limit':True},{'extra':'PRIVATE'}]:
                        await call('inspect_snapshot',True,snapshot_id=keys['many'],section='owned_auctions',
                            owned_auction_filter={'filter':{'item_id':10},**page})
                    for family in ['retail','classic']:
                        await call('list_snapshots',True,family=family,owned_auction={'item_id':10})
                    for key in [private,retail_key]:
                        await call('inspect_snapshot',True,snapshot_id=key,section='owned_auctions',
                            owned_auction_filter={'filter':{'item_id':10}})
                    await call('inspect_snapshot',True,snapshot_id=keys['many'],section='bags',
                        owned_auction_filter={'filter':{'item_id':10}})
            for path,content in before.items():self.assertEqual(path.read_bytes(),content)


if __name__=='__main__':unittest.main()
