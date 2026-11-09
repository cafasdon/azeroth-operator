"""Visible loot is historical observation, never owned or automatically taken."""
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


class LootWindowTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_reader_parity_summary_and_no_owned_inventory(self):
        root = Path(__file__).resolve().parents[1]
        binary = (root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base = fixture()
        base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['bags'] = {'0':{'1':{'item_id':10,'count':2}}}
        base['sections']['selected_recipe'] = {'recipe_id':101,'name':'Fixture','learned':True,
            'requirements':'fixed_items','reagents':{'10':3}}
        section = {'coverage':'visible_loot_window_slots','slot_count':3,'entries':{
            '1':{'status':'item','item_id':10,'quantity':4},
            '2':{'status':'unreadable_or_non_item'},
            '3':{'status':'item','item_id':10,'quantity':0}}}
        maximum = {'coverage':section['coverage'],'slot_count':64,'entries':{
            str(i):{'status':'item','item_id':4294967295,'quantity':4294967295} for i in range(1,65)}}
        empty = {'coverage':section['coverage'],'slot_count':0,'entries':{}}
        invalid = [None,{},[],{**section,'coverage':'complete_loot'},
            {**section,'slot_count':True},{**section,'slot_count':3.0},
            {**section,'slot_count':65},{**section,'slot_count':-1},
            {**section,'slot_count':None},{**section,'private':'PRIVATE'},
            {**section,'entries':None},{**section,'entries':{}},
            {**empty,'entries':{'1':{'status':'unreadable_or_non_item'}}},
            {**empty,'entries':{'01':{'status':'unreadable_or_non_item'}}}]
        for row in [None,{},[],{'status':'unknown'},
            {'status':'unreadable_or_non_item','item_id':1},
            {'status':'item','item_id':0,'quantity':1},
            {'status':'item','item_id':1,'quantity':-1},
            {'status':'item','item_id':True,'quantity':1},
            {'status':'item','item_id':1,'quantity':True},
            {'status':'item','item_id':1.0,'quantity':1},
            {'status':'item','item_id':1,'quantity':1.0},
            {'status':'item','item_id':4294967296,'quantity':1},
            {'status':'item','item_id':1,'quantity':4294967296},
            {'status':'item','item_id':1,'quantity':None},
            {'status':'item','item_id':1,'quantity':1,'link':'PRIVATE'}]:
            invalid.append({'coverage':section['coverage'],'slot_count':1,'entries':{'1':row}})
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder)); valid = []
            for value in [None,empty,section,maximum]:
                saved = deepcopy(base)
                if value is not None: saved['sections']['loot_window'] = value
                valid.append((store.ingest(saved),value))
            bad = []
            for value in invalid:
                saved = deepcopy(base); saved['sections']['loot_window'] = value
                with self.assertRaises(ValueError): validate_snapshot(saved)
                raw = json.dumps(saved,sort_keys=True,separators=(',',':')).encode()
                key = hashlib.sha256(raw).hexdigest(); (Path(folder)/f'{key}.json').write_bytes(raw); bad.append(key)
            for family,build,interface in [('retail','1',120000),('classic','1',11500)]:
                saved = deepcopy(base); saved['client'] = {'family':family,'variant':'live','build':build,'interface':interface}
                saved['sections'].pop('selected_recipe')
                saved['sections']['loot_window'] = empty
                with self.assertRaises(ValueError): validate_snapshot(saved)
                raw = json.dumps(saved,sort_keys=True,separators=(',',':')).encode()
                key = hashlib.sha256(raw).hexdigest(); (Path(folder)/f'{key}.json').write_bytes(raw); bad.append(key)
            before = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply = await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    for key,value in valid:
                        observed = (await call('character_summary',snapshot_id=key))['loot_window_observation']
                        inspected = await call('inspect_snapshot',snapshot_id=key,section='loot_window')
                        if value is None:
                            self.assertIsNone(observed); self.assertFalse(inspected['available'])
                        else:
                            self.assertEqual(inspected['data'],value)
                            quantities = {}; rows = list(value['entries'].values())
                            for row in rows:
                                if row['status']=='item': quantities[str(row['item_id'])] = quantities.get(str(row['item_id']),0)+row['quantity']
                            self.assertEqual(observed['observed_item_quantities'],quantities)
                            self.assertEqual(observed['item_slots'],sum(row['status']=='item' for row in rows))
                            self.assertEqual(observed['unreadable_or_non_item_slots'],sum(row['status']!='item' for row in rows))
                            self.assertEqual(observed['slot_count'],value['slot_count'])
                            for flag in ['counts_toward_owned_inventory','ownership_verified','lootability_verified','game_execution']:
                                self.assertFalse(observed[flag])
                        found = await call('find_snapshot_item',snapshot_id=key,item_id=10)
                        self.assertEqual(found['quantity_by_storage']['bags'],2)
                        self.assertNotIn('loot_window',found['quantity_by_storage'])
                        recipe = await call('review_forever_recipe',snapshot_id=key,crafts=1)
                        self.assertEqual(recipe['materials'][0]['quantity_observed'],2)
                        self.assertEqual(recipe['material_capacity']['bags']['supported_crafts'],0)
                    for key in bad: await call('inspect_snapshot',error=True,snapshot_id=key,section='loot_window')
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
