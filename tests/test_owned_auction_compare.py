"""Historical cache comparison through the actual owner-local MCP process."""
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


class OwnedAuctionCompareTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_missing_empty_changed_reordering_and_bounds(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        def row(id,item=10,quantity=2,status=0):
            return {'auction_id':id,'item_id':item,'quantity':quantity,'reported_status':status}
        def snapshot(day,rows,name='Fixture'):
            value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['character']['name']=name;value['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            if rows is not None:value['sections']['owned_auctions']={
                'coverage':'loaded_player_owned_auction_cache','entries':{str(i):r for i,r in enumerate(rows,1)}}
            return value
        before=[row(9007199254740991),row(2),row(10),row(20)]
        after=[row(10,20,4294967295,4294967295),row(3),row(2)]
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as other:
            store=SnapshotStore(Path(folder))
            cases={'missing_old':snapshot(1,None),'missing_new':snapshot(2,None),
                'empty_old':snapshot(1,[]),'empty_new':snapshot(2,[]),
                'before':snapshot(1,before),'after':snapshot(2,after),
                'reorder':snapshot(2,list(reversed(before))),
                'many_old':snapshot(1,[row(i,quantity=4294967295) for i in range(1,257)]),
                'many_after':snapshot(2,[row(i,quantity=1,status=1) for i in range(1,257)]),
                'many_other':snapshot(2,[row(i+256) for i in range(1,257)]),
                'foreign_character':snapshot(2,[],name='Other')}
            keys={k:store.ingest(v) for k,v in cases.items()}
            private=SnapshotStore(Path(other)).ingest(snapshot(2,after,name='PRIVATE_OTHER_OWNER'))
            retail_before=fixture();retail_after=deepcopy(retail_before);retail_after['captured_at']='2025-01-02T00:00:00Z'
            retail_keys=[store.ingest(v) for v in [retail_before,retail_after]]
            original={p:p.read_bytes() for d in [folder,other] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(old,new,flag=True,error=False):
                        args={'before_id':old,'after_id':new}
                        if flag is not None:args['owned_auction_changes']=flag
                        reply=await session.call_tool('compare_snapshots',args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(other,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    result=(await call(keys['before'],keys['after']))['owned_auction_changes']
                    self.assertEqual([r['auction_id'] for r in result['only_before']['rows']],[20,9007199254740991])
                    self.assertEqual([r['auction_id'] for r in result['only_after']['rows']],[3])
                    self.assertEqual(result['changed']['rows'],[{'auction_id':10,'fields':{
                        'item_id':{'before':10,'after':20},'quantity':{'before':2,'after':4294967295,'delta':4294967293},
                        'reported_status':{'before':0,'after':4294967295}}}])
                    self.assertEqual(result['unchanged_auction_id_count'],1)
                    self.assertEqual(result['reported_item_unit_delta'],4294967291)
                    for flag in ['numeric_auction_identity_verified','sale_or_delivery_inferred','current_ownership_verified',
                        'complete_auction_coverage_verified','counts_toward_inventory','counts_toward_wallet','game_execution','live']:
                        self.assertFalse(result[flag])
                    for old,new in [('missing_old','missing_new'),('before','missing_new'),('missing_old','after')]:
                        result=(await call(keys[old],keys[new]))['owned_auction_changes']
                        for field in ['only_before','only_after','changed','unchanged_auction_id_count','reported_item_unit_delta']:
                            self.assertIsNone(result[field])
                        self.assertEqual(result['before'] is None,old.startswith('missing'))
                        self.assertEqual(result['after'] is None,new.startswith('missing'))
                    empty=(await call(keys['empty_old'],keys['empty_new']))['owned_auction_changes']
                    for field in ['only_before','only_after','changed']:self.assertEqual(empty[field],{'count':0,'truncated':False,'rows':[]})
                    reordered=(await call(keys['before'],keys['reorder']))['owned_auction_changes']
                    self.assertEqual(reordered['unchanged_auction_id_count'],4);self.assertEqual(reordered['changed']['count'],0)
                    self.assertEqual(reordered['reported_item_unit_delta'],0)
                    changed=(await call(keys['many_old'],keys['many_after']))['owned_auction_changes']
                    self.assertEqual(changed['changed']['count'],256);self.assertTrue(changed['changed']['truncated'])
                    self.assertEqual([r['auction_id'] for r in changed['changed']['rows']],list(range(1,51)))
                    self.assertEqual(changed['reported_item_unit_delta'],256*(1-4294967295))
                    disjoint=(await call(keys['many_old'],keys['many_other']))['owned_auction_changes']
                    for field in ['only_before','only_after']:
                        self.assertEqual(disjoint[field]['count'],256);self.assertEqual(len(disjoint[field]['rows']),50)
                        self.assertTrue(disjoint[field]['truncated'])
                    plain=await call(keys['before'],keys['after'],flag=None)
                    self.assertNotIn('owned_auction_changes',plain)
                    self.assertEqual(await call(keys['before'],keys['after'],flag=False),plain)
                    for invalid in ['PRIVATE',1,0,1.0,{},[]]:await call(keys['before'],keys['after'],flag=invalid,error=True)
                    await call(*retail_keys,error=True)
                    await call(keys['before'],keys['foreign_character'],error=True)
                    await call(keys['before'],private,error=True)
            for path,content in original.items():self.assertEqual(path.read_bytes(),content)


if __name__=='__main__':unittest.main()
