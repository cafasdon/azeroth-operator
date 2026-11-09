"""Observed material capacity ordering before paging, never a skill-gain forecast."""
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


class RecipeCapacityOrderTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_capacity_inventory_provenance_and_paging(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        capture=fixture();capture['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        rows={str(i):{'status':'fixed_items','reagents':reagents,'cooldown':{'remaining_seconds':0}}
              for i,reagents in [(1,{'10':4}),(2,{'20':1}),(3,{}),(5,{'30':1}),
                                 (6,{'10':1}),(10,{'10':2}),(11,{'20':1})]}
        rows['1']['cooldown']={'charges':1,'max_charges':1,'remaining_seconds':100}
        rows['4']={'status':'unavailable'}
        capture['sections']['profession_recipe_materials']={'coverage':'reported_known_spell_schematics','profession_id':164,'entries':rows}
        capture['sections']['profession_recipe_view']={'coverage':'filtered_local_recipe_list','profession_id':164,
            'entries':{str(i):{'learned':True,'can_skill_up':i!=6} for i in [1,2,3,4,5,6,10,11]}}
        capture['sections']['bags']={'0':{'1':{'item_id':10,'count':20},'2':{'item_id':20,'count':3}}}
        capture['sections']['character_bank']={'6':{'1':{'item_id':20,'count':100}}}
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder));key=store.ingest(capture)
            newer=deepcopy(capture);newer['captured_at']='2025-01-02T00:00:00Z'
            newer['sections']['bags']={'0':{'1':{'item_id':10,'count':4},'2':{'item_id':20,'count':30}}}
            newer_key=store.ingest(newer)
            missing=deepcopy(capture);del missing['sections']['bags'];missing_key=store.ingest(missing)
            absent=deepcopy(capture);del absent['sections']['profession_recipe_materials'];absent_key=store.ingest(absent)
            empty=deepcopy(capture);empty['sections']['profession_recipe_materials']['entries']={};empty_key=store.ingest(empty)
            retail_key=store.ingest(fixture())
            other=deepcopy(newer);other['character']['name']='PRIVATE_OTHER'
            foreign_key=SnapshotStore(Path(foreign)).ingest(other)
            before={p:p.read_bytes() for base in [folder,foreign] for p in Path(base).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (reader,writer):
                async with ClientSession(reader,writer) as session:
                    await session.initialize()
                    async def search(error=False,**extra):
                        reply=await session.call_tool('search_forever_recipes',{'snapshot_id':key,**extra})
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    material={'scope':'bags','minimum_crafts':1}
                    args={'material_filter':material,'recipe_view_filter':{'learned':True,'can_skill_up':True}}
                    raw=await search(**args)
                    self.assertEqual([r['recipe_id'] for r in raw['records']],[1,2,3,10,11])
                    self.assertEqual(await search(**args,order='recipe_id'),raw)
                    self.assertEqual(await search(**args,order=None),raw)
                    ordered=await search(**args,order='observed_item_capacity_desc')
                    self.assertEqual([r['recipe_id'] for r in ordered['records']],[10,1,2,11,3])
                    self.assertEqual([r['material_support']['supported_crafts'] for r in ordered['records']],[10,5,3,3,None])
                    self.assertEqual(ordered['matched_row_count'],5)
                    self.assertEqual(ordered['material_support_unknown_count'],1)
                    self.assertEqual(ordered['material_support_insufficient_observed_count'],1)
                    self.assertEqual(ordered['ordering_basis'],'independent_observed_required_item_capacity_only')
                    for flag in ['shared_material_allocation_performed','skill_gain_prediction_performed','craftability_verified','game_execution','live']:
                        self.assertIs(ordered[flag],False)
                    page=await search(**args,order='observed_item_capacity_desc',limit=1)
                    self.assertEqual(page['records'][0]['recipe_id'],10)
                    self.assertEqual(page['next_offset'],1)
                    self.assertEqual([r['recipe_id'] for r in (await search(**args,order='observed_item_capacity_desc',offset=1,limit=2))['records']],[1,2])
                    fresh=await search(**args,order='observed_item_capacity_desc',inventory_snapshot_id=newer_key,cooldown_filter={'state':'available'})
                    self.assertEqual([r['recipe_id'] for r in fresh['records']],[2,11,10,1,3])
                    self.assertEqual(fresh['recipe_captured_at'],capture['captured_at'])
                    self.assertEqual(fresh['inventory_captured_at'],newer['captured_at'])
                    self.assertEqual(fresh['cooldown_captured_at'],capture['captured_at'])
                    self.assertEqual(next(r for r in fresh['records'] if r['recipe_id']==1)['cooldown']['remaining_seconds'],100)
                    bank=await search(order='observed_item_capacity_desc',material_filter={'scope':'all_observed_storage','minimum_crafts':1})
                    self.assertEqual([r['recipe_id'] for r in bank['records'][:2]],[2,11])
                    self.assertEqual(bank['records'][0]['material_support']['supported_crafts'],103)
                    unknown=await search(snapshot_id=missing_key,order='observed_item_capacity_desc',**args)
                    self.assertEqual([r['recipe_id'] for r in unknown['records']],[3])
                    self.assertTrue(unknown['records'][0]['material_support']['no_required_items'])
                    for selected in [absent_key,empty_key]:
                        out=await search(snapshot_id=selected,order='observed_item_capacity_desc',**args)
                        self.assertEqual(out['records'],[])
                        self.assertEqual(out['matched_row_count'],None if selected==absent_key else 0)
                    await search(True,order='observed_item_capacity_desc')
                    await search(True,snapshot_id=retail_key,order='observed_item_capacity_desc',material_filter=material)
                    await search(True,order='observed_item_capacity_desc',material_filter=material,inventory_snapshot_id=foreign_key)
                    for bad in ['PRIVATE',True,1,[],{},'observed_item_capacity_asc']:
                        await search(True,order=bad,**args)
            self.assertEqual(before,{p:p.read_bytes() for p in before})


if __name__=='__main__':unittest.main()
