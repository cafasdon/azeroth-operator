"""Refresh recipe material discovery without moving recipe evidence in time."""
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


class RecipeSearchInventoryTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_inventory_refresh_provenance_guards_and_unknowns(self):
        root=Path(__file__).resolve().parents[1]
        binary=(root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        original=fixture()
        original['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        original['character']['name']='PRIVATE_CHARACTER'
        original['sections']['profession_recipe_materials']={
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
                '1':{'status':'fixed_items','reagents':{'10':2}},
                '2':{'status':'fixed_items','reagents':{'20':1}},
                '3':{'status':'fixed_items','reagents':{}}}}
        original['sections']['profession_recipe_view']={
            'coverage':'filtered_local_recipe_list','profession_id':164,'entries':{
                '1':{'learned':True,'can_skill_up':True,'skill_ups_reported':1},
                '2':{'learned':False,'can_skill_up':True},'3':{'learned':True,'can_skill_up':False}}}
        original['sections']['bags']={'0':{'1':{'item_id':10,'count':1}}}
        original['sections']['character_bank']={'6':{'1':{'item_id':20,'count':5}}}
        newer=deepcopy(original); newer['captured_at']='2025-01-02T00:00:00Z'
        newer['sections']['bags']['0']['1']['count']=6
        newer['sections'].pop('character_bank')
        newer['sections']['profession_recipe_materials']['entries']={}
        newer['sections']['profession_recipe_view']['entries']['1']['learned']=False
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); store=SnapshotStore(base)
            recipe=store.ingest(original); inventory=store.ingest(newer)
            variants={}
            for name in ['older','character','realm','family','variant','build','interface','missing','empty','equal_time']:
                saved=deepcopy(newer)
                if name=='older': saved['captured_at']='2024-12-31T00:00:00Z'
                elif name=='character': saved['character']['name']='PRIVATE_OTHER'
                elif name=='realm': saved['character']['realm']='PRIVATE_OTHER'
                elif name=='family':
                    saved['client']['family']='retail'
                    for section in ['profession_recipe_materials','profession_recipe_view']:
                        saved['sections'].pop(section)
                elif name=='variant': saved['client']['variant']='ptr'
                elif name=='build': saved['client']['build']='70863'
                elif name=='interface': saved['client']['interface']=16002
                elif name=='missing': saved['sections'].pop('bags')
                elif name=='empty': saved['sections']['bags']={}
                elif name=='equal_time': saved['captured_at']=original['captured_at']
                variants[name]=store.ingest(saved)
            before={p.name:p.read_bytes() for p in base.glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def search(error=False,**extra):
                        reply=await session.call_tool('search_forever_recipes',{'snapshot_id':recipe,**extra})
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    material={'scope':'bags','minimum_crafts':3}
                    ids=lambda row:[r['recipe_id'] for r in row['records']]
                    legacy=await search(material_filter=material)
                    self.assertEqual(legacy,await search(material_filter=material,inventory_snapshot_id=None))
                    self.assertNotIn('inventory_snapshot_id',legacy)
                    self.assertEqual(ids(legacy),[3])
                    refreshed=await search(material_filter=material,inventory_snapshot_id=inventory)
                    self.assertEqual(ids(refreshed),[1,3])
                    self.assertEqual(refreshed['inventory_snapshot_id'],inventory)
                    self.assertEqual(refreshed['snapshot_id'],recipe)
                    self.assertEqual(refreshed['recipe_captured_at'],original['captured_at'])
                    self.assertEqual(refreshed['inventory_captured_at'],newer['captured_at'])
                    self.assertEqual(refreshed['captured_spell_count'],3)
                    flags=await search(material_filter=material,inventory_snapshot_id=inventory,
                        recipe_view_filter={'learned':True,'can_skill_up':True})
                    self.assertEqual(ids(flags),[1])
                    self.assertIs(flags['records'][0]['recipe_view_observation']['learned'],True)
                    page=await search(material_filter=material,inventory_snapshot_id=inventory,limit=1)
                    self.assertEqual((ids(page),page['next_offset']),([1],1))
                    self.assertEqual(ids(await search(material_filter=material,inventory_snapshot_id=inventory,limit=1,offset=1)),[3])
                    all_items=await search(material_filter={'scope':'all_observed_storage','minimum_crafts':1},
                        inventory_snapshot_id=inventory)
                    self.assertEqual(ids(all_items),[1,3])
                    self.assertEqual(all_items['material_inventory_coverage']['unobserved_storage'],['character_bank','account_bank'])
                    self.assertEqual(ids(await search(material_filter=material,inventory_snapshot_id=variants['equal_time'])),[1,3])
                    for name,unknown,insufficient in [('missing',2,0),('empty',0,2)]:
                        found=await search(material_filter=material,inventory_snapshot_id=variants[name])
                        self.assertEqual(ids(found),[3])
                        self.assertEqual((found['material_support_unknown_count'],found['material_support_insufficient_observed_count']),
                            (unknown,insufficient))
                    reply=await session.call_tool('review_forever_recipe',{'snapshot_id':recipe,'recipe_id':1,
                        'crafts':3,'inventory_snapshot_id':inventory})
                    self.assertFalse(reply.isError,reply)
                    plan=json.loads(reply.content[0].text)
                    self.assertEqual(plan['material_capacity']['bags']['supported_crafts'],
                        refreshed['records'][0]['material_support']['supported_crafts'])
                    for name in ['older','character','realm','family','variant','build','interface']:
                        await search(error=True,material_filter=material,inventory_snapshot_id=variants[name])
                    for bad in ['', '0'*64,True,1,{}]:
                        await search(error=True,material_filter=material,inventory_snapshot_id=bad)
                    await search(error=True,inventory_snapshot_id=inventory)
                    for flag in ['craftability_verified','complete_recipe_knowledge_verified','live','game_execution']:
                        self.assertFalse(refreshed[flag])
            self.assertEqual(before,{p.name:p.read_bytes() for p in base.glob('*.json')})


if __name__=='__main__': unittest.main()
