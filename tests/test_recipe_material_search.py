"""Discover fixed recipe material support; never infer learning or live crafting."""
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


class RecipeMaterialSearchTests(unittest.IsolatedAsyncioTestCase):
    async def test_maximum_bulk_rows_and_u64_material_totals(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        value = fixture()
        value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['profession_recipe_materials'] = {
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
                str(i):{'status':'fixed_items','reagents':{str(item):1 for item in range(10,14)}}
                for i in range(1,4097)}}
        for storage,container in [('bags','0'),('character_bank','6')]:
            value['sections'][storage] = {container:{str(i):{'item_id':item,'count':4294967295}
                for i,item in enumerate(range(10,14),1)}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            key = store.ingest(value)
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for offset,expected in [(0,list(range(1,51))),(4090,list(range(4091,4097)))]:
                        reply = await session.call_tool('search_forever_recipes',{'snapshot_id':key,
                            'material_filter':{'scope':'all_observed_storage','minimum_crafts':4294967295},
                            'offset':offset,'limit':50})
                        self.assertFalse(reply.isError,reply)
                        found = json.loads(reply.content[0].text)
                        self.assertEqual(found['matched_row_count'],4096)
                        self.assertEqual(found['material_support_unknown_count'],0)
                        self.assertEqual(found['material_support_insufficient_observed_count'],0)
                        self.assertEqual([row['recipe_id'] for row in found['records']],expected)
                        self.assertTrue(all(row['material_support']['supported_crafts']==8589934590 for row in found['records']))
                        self.assertEqual(found['next_offset'],50 if offset==0 else None)

    async def test_real_mcp_material_scopes_unknowns_paging_and_planner_reference(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        rows = {str(i):{'status':'fixed_items','reagents':reagents} for i,reagents in
            [(1,{'10':4}),(2,{'10':2,'11':2}),(3,{}),(7,{'99':1}),(8,{'10':4})]}
        rows.update({'4':{'status':'unavailable'},'5':{'status':'unsupported_required_slots'},'6':{'status':'budget_exhausted'}})
        base['sections']['profession_recipe_materials'] = {
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':rows}
        base['sections']['selected_recipe'] = {'recipe_id':8,'name':'PRIVATE','learned':False,
            'requirements':'fixed_items','reagents':{'10':4}}
        base['sections']['bags'] = {'0':{'1':{'item_id':10,'count':5},'2':{'item_id':11,'count':1}}}
        base['sections']['character_bank'] = {'6':{'1':{'item_id':10,'count':7}}}
        base['sections']['account_bank'] = {'7':{'1':{'item_id':10,'count':3},'2':{'item_id':11,'count':5}}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            key = store.ingest(base)
            variants = {}
            for name in ['missing_inventory','empty_bags','bags_only','missing_recipes','empty_recipes']:
                saved = deepcopy(base)
                if name in ['missing_inventory','empty_bags','bags_only']:
                    for field in ['bags','character_bank','account_bank']:
                        if name != 'bags_only' or field != 'bags': saved['sections'].pop(field,None)
                    if name == 'empty_bags': saved['sections']['bags'] = {}
                if name == 'missing_recipes': saved['sections'].pop('profession_recipe_materials')
                if name == 'empty_recipes': saved['sections']['profession_recipe_materials']['entries'] = {}
                variants[name] = store.ingest(saved)
            original = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def search(material=None,error=False,**options):
                        args = {'snapshot_id':key,**options}
                        if material is not None: args['material_filter'] = material
                        reply = await session.call_tool('search_forever_recipes',args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    default = await search()
                    self.assertEqual(default['matched_row_count'],8)
                    self.assertNotIn('material_filter',default['filters'])
                    self.assertNotIn('material_inventory_coverage',default)
                    ids = lambda result:[row['recipe_id'] for row in result['records']]
                    bags = await search({'scope':'bags','minimum_crafts':1})
                    self.assertEqual(ids(bags),[1,3,8])
                    self.assertEqual((bags['material_support_unknown_count'],bags['material_support_insufficient_observed_count']),(3,2))
                    self.assertEqual(bags['records'][0]['material_support']['supported_crafts'],1)
                    self.assertFalse(bags['craftability_verified'])
                    all_items = await search({'scope':'all_observed_storage','minimum_crafts':3})
                    self.assertEqual(ids(all_items),[1,2,3,8])
                    self.assertEqual(all_items['records'][1]['material_support']['supported_crafts'],3)
                    self.assertEqual(all_items['material_inventory_coverage']['unobserved_storage'],[])
                    self.assertFalse(all_items['material_inventory_coverage']['complete_ownership_verified'])
                    free = await search({'scope':'bags','minimum_crafts':4294967295})
                    self.assertEqual(ids(free),[3])
                    self.assertEqual(free['records'][0]['material_support'],{
                        'supported_crafts':None,'no_required_items':True,'covers_minimum_crafts':True})
                    unknown = await search({'scope':'bags','minimum_crafts':1},snapshot_id=variants['missing_inventory'])
                    self.assertEqual(ids(unknown),[3])
                    self.assertEqual((unknown['material_support_unknown_count'],unknown['material_support_insufficient_observed_count']),(7,0))
                    empty = await search({'scope':'bags','minimum_crafts':1},snapshot_id=variants['empty_bags'])
                    self.assertEqual((empty['material_support_unknown_count'],empty['material_support_insufficient_observed_count']),(3,4))
                    partial = await search({'scope':'all_observed_storage','minimum_crafts':1},snapshot_id=variants['bags_only'])
                    self.assertEqual(ids(partial),[1,3,8])
                    self.assertEqual(partial['material_inventory_coverage']['unobserved_storage'],['character_bank','account_bank'])
                    narrowed = await search({'scope':'bags','minimum_crafts':1},reagent_item_id=10,limit=1)
                    self.assertEqual(ids(narrowed),[1])
                    self.assertEqual(narrowed['matched_row_count'],2)
                    self.assertEqual(narrowed['next_offset'],1)
                    self.assertEqual(ids(await search({'scope':'bags','minimum_crafts':1},reagent_item_id=10,limit=1,offset=1)),[8])
                    self.assertEqual(narrowed['material_support_unknown_count'],0)
                    for name,expected in [('missing_recipes',None),('empty_recipes',0)]:
                        result = await search({'scope':'bags','minimum_crafts':1},snapshot_id=variants[name])
                        self.assertEqual(result['matched_row_count'],expected)
                        self.assertEqual(result['material_support_unknown_count'],expected)
                    for recipe in [1,2,3,7]:
                        reply = await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':recipe,'crafts':1})
                        self.assertFalse(reply.isError,reply)
                        plan = json.loads(reply.content[0].text)
                        for scope in ['bags','all_observed_storage']:
                            result = await search({'scope':scope,'minimum_crafts':1},recipe_id=recipe)
                            self.assertEqual(bool(result['records']),plan['captured_preparation'][
                                'bags_cover_fixed_reagents' if scope=='bags' else 'observed_storage_covers_fixed_reagents'])
                            if result['records']:
                                self.assertEqual(result['records'][0]['material_support']['supported_crafts'],
                                    (plan['material_capacity'][scope] or {}).get('supported_crafts'))
                    # Materials alone never override an explicit false learning observation.
                    conflict = await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':8,'crafts':1})
                    self.assertTrue(conflict.isError)
                    for bad in [{},{'scope':'bags'},{'minimum_crafts':1},{'scope':'equipment','minimum_crafts':1},
                        {'scope':'bags','minimum_crafts':0},{'scope':'bags','minimum_crafts':True},
                        {'scope':'bags','minimum_crafts':1.0},{'scope':'bags','minimum_crafts':'1'},
                        {'scope':'bags','minimum_crafts':4294967296},{'scope':'bags','minimum_crafts':1,'unexpected':'PRIVATE'}]:
                        await search(bad,error=True)
            self.assertEqual(original,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
