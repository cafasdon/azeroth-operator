"""Strict owner-local bag identity observations without material or simulator credit."""
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


class EquippedBagTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_pair_parity_owner_and_no_stock_credit(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['equipment']={'16':{'item_id':123}}
        selection=[{'source':{'kind':'equipment','slot':16},'target_slot':16,'expected_item_id':123}]
        bags={str(i):{'total_slots':16 if i==0 else 8 if i==1 else 0,'free_slots':0} for i in range(5)}
        bags['1']['equipped_item']={'item_id':8217,'inventory_slot':20}
        value['sections']['bag_capacity']=bags
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder));key=store.ingest(value)
            old=deepcopy(value);del old['sections']['bag_capacity']['1']['equipped_item'];old_key=store.ingest(old)
            missing=deepcopy(value);del missing['sections']['bag_capacity'];missing_key=store.ingest(missing)
            upper=deepcopy(value);upper['sections']['bag_capacity']['1']['equipped_item']={'item_id':4294967295,'inventory_slot':255}
            upper_key=store.ingest(upper)
            other=deepcopy(value);other['character']['name']='PRIVATE_OTHER'
            other_key=SnapshotStore(Path(foreign)).ingest(other)
            bad_keys=[]
            def invalid(change):
                bad=deepcopy(value);change(bad)
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,ensure_ascii=False,sort_keys=True,allow_nan=False).encode()
                digest=hashlib.sha256(raw).hexdigest();(Path(folder)/(digest+'.json')).write_bytes(raw);bad_keys.append(digest)
            for item in [None,[],{}, {'item_id':8217},{'inventory_slot':20},
                         {'item_id':0,'inventory_slot':20},{'item_id':4294967296,'inventory_slot':20},
                         {'item_id':True,'inventory_slot':20},{'item_id':None,'inventory_slot':20},
                         {'item_id':8217,'inventory_slot':None},{'item_id':8217,'inventory_slot':0},
                         {'item_id':8217,'inventory_slot':256},{'item_id':8217,'inventory_slot':1.5},
                         {'item_id':8217,'inventory_slot':20,'private':'PRIVATE'}]:
                invalid(lambda bad,item=item:bad['sections']['bag_capacity']['1'].update(equipped_item=item))
            invalid(lambda bad:bad['sections']['bag_capacity']['0'].update(equipped_item={'item_id':1,'inventory_slot':21}))
            invalid(lambda bad:bad['sections']['bag_capacity']['2'].update(equipped_item={'item_id':1,'inventory_slot':21}))
            invalid(lambda bad:bad['sections']['bag_capacity']['2'].update(total_slots=4,equipped_item={'item_id':2,'inventory_slot':20}))
            before={p:p.read_bytes() for base in [folder,foreign] for p in Path(base).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (reader,writer):
                async with ClientSession(reader,writer) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    raw=await call('inspect_snapshot',snapshot_id=key,section='bag_capacity')
                    self.assertEqual(raw['data'],bags)
                    summary=await call('character_summary',snapshot_id=key)
                    observed=summary['equipped_bag_item_observations']
                    self.assertEqual(observed['entries'],[{'bag':1,'item_id':8217,'inventory_slot':20}])
                    self.assertEqual(observed['observed_identity_count'],1)
                    review=await call('review_equipment_selection',snapshot_id=key,selections=selection)
                    self.assertEqual(review['captured_equipped_bag_items'],observed['entries'])
                    for flag in ['complete_equipped_bag_identity_verified','quiver_bonus_verified','credited_as_material_stock','game_execution']:
                        self.assertIs(observed[flag],False)
                    found=await call('find_snapshot_item',snapshot_id=key,item_id=8217)
                    self.assertEqual(found['matched_locations'],0)
                    legacy=await call('inspect_snapshot',snapshot_id=old_key,section='bag_capacity')
                    self.assertNotIn('equipped_item',legacy['data']['1'])
                    self.assertEqual((await call('review_equipment_selection',snapshot_id=old_key,selections=selection))['captured_equipped_bag_items'],[])
                    self.assertIsNone((await call('review_equipment_selection',snapshot_id=missing_key,selections=selection))['captured_equipped_bag_items'])
                    self.assertEqual((await call('character_summary',snapshot_id=old_key))['equipped_bag_item_observations']['entries'],[])
                    self.assertIsNone((await call('character_summary',snapshot_id=missing_key))['equipped_bag_item_observations'])
                    self.assertEqual((await call('inspect_snapshot',snapshot_id=upper_key,section='bag_capacity'))['data']['1']['equipped_item']['inventory_slot'],255)
                    for bad in [*bad_keys,other_key]:
                        await call('inspect_snapshot',True,snapshot_id=bad,section='bag_capacity')
            self.assertEqual(before,{p:p.read_bytes() for p in before})


if __name__=='__main__':unittest.main()
