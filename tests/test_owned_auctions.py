"""Strict owner-local loaded auction observations; never inventory or wallet credit."""
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


class OwnedAuctionTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_schema_privacy_coverage_and_no_stock_credit(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['bags']={'0':{'1':{'item_id':10,'count':2}}}
        row={'auction_id':9007199254740991,'item_id':10,'quantity':4294967295,'reported_status':0}
        observed={'coverage':'loaded_player_owned_auction_cache','entries':{'1':row}}
        invalid=[None,{},[],{**observed,'coverage':'all_auctions'},{**observed,'bidder':'PRIVATE'},
            {**observed,'entries':[]},{**observed,'entries':{'2':row}},
            {**observed,'entries':{'01':row}},{**observed,'entries':{str(i):row for i in range(1,258)}},
            {**observed,'entries':{'1':row,'2':row}}]
        for field in row:
            values=[None,True,False,-1,1.0,'1',[],{},9007199254740992 if field=='auction_id' else 4294967296]
            if field!='reported_status':values.append(0)
            for value in values:
                changed=deepcopy(observed);changed['entries']['1'][field]=value;invalid.append(changed)
            changed=deepcopy(observed);del changed['entries']['1'][field];invalid.append(changed)
        changed=deepcopy(observed);changed['entries']['1']['bidder']='PRIVATE';invalid.append(changed)
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as other:
            store=SnapshotStore(Path(folder));keys=[]
            for day,section in enumerate([None,{**observed,'entries':{}},observed],1):
                saved=deepcopy(base);saved['captured_at']=f'2025-01-{day:02}T00:00:00Z'
                if section is not None:saved['sections']['owned_auctions']=section
                keys.append(store.ingest(saved))
            malformed=[]
            for section in invalid:
                saved=deepcopy(base);saved['sections']['owned_auctions']=section
                with self.assertRaises(ValueError):validate_snapshot(saved)
                malformed.append(saved)
            for family,interface in [('retail',120000),('classic',11500)]:
                saved=deepcopy(base);saved['sections'].pop('selected_recipe',None)
                saved['client']={'family':family,'variant':'live','build':'1','interface':interface}
                saved['sections']['owned_auctions']=observed
                with self.assertRaises(ValueError):validate_snapshot(saved)
                malformed.append(saved)
            bad=[]
            for saved in malformed:
                raw=json.dumps(saved,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(raw).hexdigest();(Path(folder)/(key+'.json')).write_bytes(raw);bad.append(key)
            foreign=deepcopy(base);foreign['character']['name']='PRIVATE_OTHER_OWNER'
            foreign['sections']['owned_auctions']=observed
            foreign_key=SnapshotStore(Path(other)).ingest(foreign)
            before={p:p.read_bytes() for d in [folder,other] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),
                    env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(other,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    for key,expected in zip(keys,[None,{**observed,'entries':{}},observed]):
                        result=await call('inspect_snapshot',snapshot_id=key,section='owned_auctions')
                        self.assertEqual(result['available'],expected is not None)
                        self.assertEqual(result['data'],expected)
                    listed=await call('list_snapshots',family='forever',
                        section_capture={'section':'owned_auctions','status':'observed'})
                    self.assertEqual({r['snapshot_id'] for r in listed['snapshots']},set(keys[1:]))
                    listed=await call('list_snapshots',family='forever',
                        inventory_item={'item_id':10,'storage':'bags','minimum_quantity':3})
                    self.assertEqual(listed['snapshots'],[])
                    for key in bad+[foreign_key]:
                        await call('inspect_snapshot',True,snapshot_id=key,section='owned_auctions')
            for path,content in before.items():self.assertEqual(path.read_bytes(),content)


if __name__=='__main__':unittest.main()
