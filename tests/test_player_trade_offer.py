"""Own outgoing offers remain historical and separate from owned stock."""
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


class PlayerTradeOfferTests(unittest.IsolatedAsyncioTestCase):
    async def test_strict_reader_parity_real_mcp_and_stock_separation(self):
        binary=(Path(__file__).resolve().parents[1]/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base=fixture()
        base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['bags']={'0':{'1':{'item_id':10,'count':2}}}
        base['sections']['selected_recipe']={'recipe_id':101,'name':'Fixture','learned':True,
            'requirements':'fixed_items','reagents':{'10':3}}
        unknown={'coverage':'visible_player_trade_normal_slots',
            'entries':{str(i):{'status':'unreadable_or_empty'} for i in range(1,7)}}
        mixed=deepcopy(unknown)
        mixed['entries']['1']={'status':'item','item_id':10,'quantity':4}
        mixed['entries']['2']={'status':'item','item_id':10,'quantity':5}
        maximum=deepcopy(unknown)
        maximum['entries']={str(i):{'status':'item','item_id':4294967295,'quantity':4294967295} for i in range(1,7)}
        full = deepcopy(maximum)
        for row in full['entries'].values():
            row.update(item_id=10, quantity=1)
        invalid=[None,{},[],{**unknown,'coverage':'all_trade'},
            {**unknown,'partner':'PRIVATE'},{**unknown,'entries':{}},
            {**unknown,'entries':None}]
        for key in ['0','7','01','PRIVATE']:
            row=deepcopy(unknown);row['entries'][key]=row['entries'].pop('1');invalid.append(row)
        for row in [None,{},[],{'status':'empty'},{'status':'unreadable_or_empty','item_id':1},
                    {'status':'item','item_id':1}, {'status':'item','item_id':1,'quantity':1,'name':'PRIVATE'}]:
            section=deepcopy(unknown);section['entries']['1']=row;invalid.append(section)
        for key in ['item_id','quantity']:
            for value in [None,True,False,0,-1,1.0,'1',4294967296,[],{}]:
                section=deepcopy(mixed);section['entries']['1'][key]=value;invalid.append(section)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));valid=[];bad=[]
            for section in [None,unknown,mixed,maximum,full]:
                saved=deepcopy(base)
                if section is not None:saved['sections']['player_trade_offer']=section
                valid.append((store.ingest(saved),section))
            bad_snapshots=[]
            for section in invalid:
                saved=deepcopy(base);saved['sections']['player_trade_offer']=section;bad_snapshots.append(saved)
            for family,interface in [('retail',120000),('classic',11500)]:
                saved=deepcopy(base);saved['client']={'family':family,'variant':'live','build':'1','interface':interface}
                saved['sections'].pop('selected_recipe')
                saved['sections']['player_trade_offer']=unknown;bad_snapshots.append(saved)
            for saved in bad_snapshots:
                with self.assertRaises(ValueError):validate_snapshot(saved)
                raw=json.dumps(saved,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(raw).hexdigest();(Path(folder)/(key+'.json')).write_bytes(raw);bad.append(key)
            before={p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        result=await session.call_tool(tool,args)
                        self.assertEqual(bool(result.isError),error,result)
                        self.assertNotIn('PRIVATE',str(result))
                        return None if error else json.loads(result.content[0].text)
                    for key,section in valid:
                        result=await call('inspect_snapshot',snapshot_id=key,section='player_trade_offer')
                        summary=(await call('character_summary',snapshot_id=key))['player_trade_offer_observation']
                        if section is None:
                            self.assertFalse(result['available']);self.assertIsNone(summary)
                        else:
                            self.assertEqual(result['data'],section)
                            quantities={}
                            for row in section['entries'].values():
                                if row['status']=='item':quantities[str(row['item_id'])]=quantities.get(str(row['item_id']),0)+row['quantity']
                            self.assertEqual(summary['observed_offered_item_quantities'],quantities)
                            self.assertEqual(summary['normal_slots'],6)
                            self.assertEqual(summary['item_slots']+summary['unreadable_or_empty_slots'],6)
                            for flag in ['counts_toward_owned_inventory','trade_completion_verified','current_offer_verified','game_execution']:
                                self.assertIs(summary[flag],False)
                        found=await call('find_snapshot_item',snapshot_id=key,item_id=10)
                        self.assertEqual(found['quantity_by_storage']['bags'],2)
                        self.assertNotIn('player_trade_offer',found['quantity_by_storage'])
                        recipe=await call('review_forever_recipe',snapshot_id=key,crafts=1)
                        self.assertEqual(recipe['materials'][0]['quantity_observed'],2)
                        self.assertEqual(recipe['material_capacity']['bags']['supported_crafts'],0)
                    missing_id, unknown_id, mixed_id, maximum_id, full_id = [key for key,_ in valid]
                    selector = 'player_trade_offer_changes'
                    self.assertNotIn(selector, await call('compare_snapshots',before_id=mixed_id,after_id=maximum_id))
                    async def compare(a,b):
                        return (await call('compare_snapshots',before_id=a,after_id=b,**{selector:True}))[selector]
                    exact = await compare(maximum_id,full_id)
                    self.assertEqual(exact['items'], [
                        {'item_id':10,'before_observed_quantity':0,'after_observed_quantity':6,'quantity_delta':6},
                        {'item_id':4294967295,'before_observed_quantity':25769803770,'after_observed_quantity':0,'quantity_delta':-25769803770},
                    ])
                    for flag in ['same_trade_verified','trade_completion_verified','live']:
                        self.assertIs(exact[flag],False)
                    partial = await compare(mixed_id,full_id)
                    self.assertEqual(partial['items'], [{'item_id':10,'before_observed_quantity':9,
                        'after_observed_quantity':6,'quantity_delta':None}])
                    for key,slots in [(missing_id,None),(unknown_id,0)]:
                        uncertain = await compare(key,full_id)
                        self.assertEqual(uncertain['before_readable_item_slots'],slots)
                        self.assertIsNone(uncertain['items'][0]['before_observed_quantity'])
                        self.assertIsNone(uncertain['items'][0]['quantity_delta'])
                    for key in bad:await call('inspect_snapshot',error=True,snapshot_id=key,section='player_trade_offer')
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__':unittest.main()
