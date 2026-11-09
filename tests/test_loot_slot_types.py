"""Typed non-item loot stays raw display evidence, outside owned stock/balances."""
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


class LootSlotTypesTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_typed_rows_bounds_unknowns_and_stock_isolation(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['bags']={'0':{'1':{'item_id':10,'count':2}}}
        base['sections']['selected_recipe']={'recipe_id':101,'name':'Synthetic','learned':True,
            'requirements':'fixed_items','reagents':{'10':3}}
        def section(rows):
            return {'coverage':'visible_loot_window_slots','slot_count':len(rows),
                    'entries':{str(i):row for i,row in enumerate(rows,1)}}
        currency={'status':'reported_currency','reported_currency_id':10,'reported_quantity':4294967295}
        cases=[section([currency,deepcopy(currency),{'status':'money'}]),
               section([currency,{'status':'money'},{'status':'unreadable_or_non_item'},
                        {'status':'item','item_id':10,'quantity':3}]),
               section([currency]*64),section([{'status':'money'}]*64),
               section([{'status':'reported_currency','reported_currency_id':4294967295,'reported_quantity':0}]),
               section([{'status':'item','item_id':10,'quantity':1}]),section([])]
        malformed=[]
        for field,values in [('reported_currency_id',[None,0,-1,True,1.0,'1',4294967296]),
                             ('reported_quantity',[None,-1,True,1.0,'1',4294967296])]:
            for value in values:
                bad=deepcopy(currency);bad[field]=value;malformed.append(bad)
            bad=deepcopy(currency);bad.pop(field);malformed.append(bad)
        malformed += [{'status':'money','quantity':1},{'status':'money','amount_copper':1},
                      {'status':'money','name':'PRIVATE'}, {'status':'reported_currency','currency_id':10,'quantity':1},
                      {**currency,'name':'PRIVATE'},{**currency,'item_id':10},{**currency,'currency_rewards_verified':True}]
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));valid=[];invalid=[]
            for loot in cases:
                value=deepcopy(base);value['sections']['loot_window']=loot;valid.append((store.ingest(value),loot))
            for row in malformed:
                value=deepcopy(base);value['sections']['loot_window']=section([row])
                with self.assertRaises(ValueError):validate_snapshot(value)
                raw=json.dumps(value,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(raw).hexdigest();(Path(folder)/f'{key}.json').write_bytes(raw);invalid.append(key)
            for family in ['retail','classic']:
                value=deepcopy(base);value['sections'].pop('selected_recipe')
                value['client']['family']=family;value['sections']['loot_window']=cases[0]
                with self.assertRaises(ValueError):validate_snapshot(value)
                raw=json.dumps(value,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(raw).hexdigest();(Path(folder)/f'{key}.json').write_bytes(raw);invalid.append(key)
            before={path.name:path.read_bytes() for path in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(folder,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    for key,loot in valid:
                        inspected=await call('inspect_snapshot',snapshot_id=key,section='loot_window',
                            loot_requirements=[{'item_id':10,'quantity':5}])
                        self.assertEqual(inspected['data'],loot)
                        rows=list(loot['entries'].values());currencies={}
                        for row in rows:
                            if row['status']=='reported_currency':
                                identifier=str(row['reported_currency_id'])
                                currencies[identifier]=currencies.get(identifier,0)+row['reported_quantity']
                        summary=(await call('character_summary',snapshot_id=key))['loot_window_observation']
                        money=sum(row['status']=='money' for row in rows)
                        unknown=sum(row['status']=='unreadable_or_non_item' for row in rows)
                        typed_count=sum(row['status']=='reported_currency' for row in rows)+money
                        if typed_count:
                            typed=summary['typed_non_item_observation']
                            self.assertEqual(typed['reported_currency_quantities'],currencies)
                            self.assertEqual(typed['money_slots'],money);self.assertEqual(typed['unreadable_slots'],unknown)
                            self.assertEqual(typed['currency_basis'],'raw_slot_info_before_container_conversion')
                            for flag in ['currency_rewards_verified','money_amounts_verified','counts_toward_owned_balances']:
                                self.assertFalse(typed[flag])
                        else:self.assertNotIn('typed_non_item_observation',summary)
                        review=inspected['loot_requirement_review']
                        self.assertEqual(review['additional_matching_quantities_unknown'],unknown>0)
                        if typed_count:
                            self.assertEqual(review['known_non_item_slots'],typed_count)
                            self.assertEqual(review['unreadable_slots'],unknown)
                        self.assertEqual(review['items'][0]['readable_quantity'],
                            sum(row['quantity'] for row in rows if row['status']=='item' and row['item_id']==10))
                        found=await call('find_snapshot_item',snapshot_id=key,item_id=10)
                        self.assertEqual(found['quantity_by_storage']['bags'],2)
                        self.assertNotIn('loot_window',found['quantity_by_storage'])
                        recipe=await call('review_forever_recipe',snapshot_id=key,crafts=1)
                        self.assertEqual(recipe['materials'][0]['quantity_observed'],2)
                        self.assertFalse(summary['ownership_verified']);self.assertFalse(summary['game_execution'])
                    for key in invalid:await call('inspect_snapshot',True,snapshot_id=key,section='loot_window')
            self.assertEqual(before,{path.name:path.read_bytes() for path in Path(folder).glob('*.json')})


if __name__=='__main__':unittest.main()
