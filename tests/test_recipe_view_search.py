"""Search joined captured recipe flags without inventing learning or skill gains."""
from copy import deepcopy
from itertools import product
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class RecipeViewSearchTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_three_valued_filters_materials_profession_and_projection(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        observations = {}
        for i,(learned,skill,points) in enumerate(product([None,False,True],[None,False,True],[None,0,2]),1):
            row = {'name':'PRIVATE_RECIPE','relative_difficulty':'optimal'}
            for field,value in [('learned',learned),('can_skill_up',skill),('skill_ups_reported',points)]:
                if value is not None: row[field] = value
            observations[str(i)] = row
        observations['28'] = {'learned':True,'can_skill_up':True,'skill_ups_reported':2}
        bulk = {str(i):{'status':'fixed_items','reagents':{'10':i%3+1}} for i in range(1,28)}
        bulk['28'] = {'status':'unsupported_required_slots'}
        base['sections']['profession_recipe_view'] = {
            'coverage':'filtered_local_recipe_list','profession_id':164,'entries':observations}
        base['sections']['profession_recipe_materials'] = {
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':bulk}
        base['sections']['bags'] = {'0':{'1':{'item_id':10,'count':4}}}
        base['sections']['selected_recipe'] = {'recipe_id':27,'name':'PRIVATE_SELECTED',
            'learned':False,'requirements':'fixed_items','reagents':{'10':1}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            key = store.ingest(base)
            variants = {}
            for name in ['missing_view','empty_view','different_profession','partial_view','missing_bulk','empty_bulk']:
                saved = deepcopy(base)
                view = saved['sections']['profession_recipe_view']
                if name == 'missing_view': saved['sections'].pop('profession_recipe_view')
                if name == 'empty_view': view['entries'] = {}
                if name == 'different_profession': view['profession_id'] = 202
                if name == 'partial_view': view['entries'].pop('27')
                if name == 'missing_bulk': saved['sections'].pop('profession_recipe_materials')
                if name == 'empty_bulk': saved['sections']['profession_recipe_materials']['entries'] = {}
                variants[name] = store.ingest(saved)
            original = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def search(error=False,**options):
                        reply = await session.call_tool('search_forever_recipes',{'snapshot_id':key,**options})
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    default = await search()
                    self.assertEqual(default,await search(recipe_view_filter=None))
                    self.assertNotIn('recipe_view_coverage',default)
                    def truth(row,criteria):
                        checks = []
                        for field,expected in criteria.items():
                            source = 'skill_ups_reported' if field in (
                                'minimum_skill_ups_reported','maximum_skill_ups_reported') else field
                            checks.append(None if source not in row else row[source]>=expected
                                if field=='minimum_skill_ups_reported' else row[source]<=expected
                                if field=='maximum_skill_ups_reported' else row[source]==expected)
                        return False if False in checks else None if None in checks else True
                    for learned,skill,points in product([None,False,True],[None,False,True],[None,0,1,3]):
                        criteria = {field:value for field,value in [('learned',learned),('can_skill_up',skill),
                            ('minimum_skill_ups_reported',points)] if value is not None}
                        if not criteria: continue
                        expected = [int(i) for i,row in observations.items() if truth(row,criteria) is True]
                        unknown = sum(truth(row,criteria) is None for row in observations.values())
                        nonmatching = sum(truth(row,criteria) is False for row in observations.values())
                        for materials in [False,True]:
                            extra = {'material_filter':{'scope':'bags','minimum_crafts':3}} if materials else {}
                            wanted = [i for i in expected if i!=28 and i%3==0] if materials else expected
                            collected,offset = [],0
                            while True:
                                found = await search(recipe_view_filter=criteria,limit=7,offset=offset,**extra)
                                self.assertEqual(found['recipe_view_unknown_count'],unknown)
                                self.assertEqual(found['recipe_view_nonmatching_count'],nonmatching)
                                self.assertEqual(found['matched_row_count'],len(wanted))
                                collected.extend(found['records'])
                                if found['next_offset'] is None: break
                                offset = found['next_offset']
                            self.assertEqual([row['recipe_id'] for row in collected],wanted)
                            for row in collected:
                                view = row['recipe_view_observation']
                                self.assertEqual(view['source'],'profession_recipe_view')
                                for field in ['learned','can_skill_up','skill_ups_reported','relative_difficulty']:
                                    self.assertEqual(view[field],observations[str(row['recipe_id'])].get(field))
                            self.assertFalse(found['recipe_view_coverage']['learning_verified'])
                            self.assertFalse(found['recipe_view_coverage']['skill_gains_verified'])
                            self.assertFalse(found['craftability_verified'])
                            if materials:
                                self.assertEqual(found['material_support_unknown_count'],int(28 in expected))
                                self.assertEqual(found['material_support_insufficient_observed_count'],len(expected)-len(wanted)-int(28 in expected))
                    for name in ['missing_view','empty_view','different_profession']:
                        found = await search(snapshot_id=variants[name],recipe_view_filter={'learned':False})
                        self.assertEqual(found['records'],[])
                        self.assertEqual(found['recipe_view_unknown_count'],28)
                        self.assertEqual(found['recipe_view_nonmatching_count'],0)
                    mismatch = await search(snapshot_id=variants['different_profession'],recipe_view_filter={'learned':True})
                    self.assertFalse(mismatch['recipe_view_coverage']['profession_matches_bulk'])
                    for name,count in [('missing_bulk',None),('empty_bulk',0)]:
                        found = await search(snapshot_id=variants[name],recipe_view_filter={'learned':True})
                        self.assertEqual(found['recipe_view_unknown_count'],count)
                        self.assertEqual(found['matched_row_count'],count)
                    partial = await search(snapshot_id=variants['partial_view'],recipe_id=27,recipe_view_filter={'learned':True})
                    self.assertEqual(partial['recipe_view_unknown_count'],1)
                    for bounds in (
                        {'maximum_skill_ups_reported':0},
                        {'minimum_skill_ups_reported':2,'maximum_skill_ups_reported':2},
                        {'maximum_skill_ups_reported':2**32-1,'relative_difficulty':'optimal'},
                    ):
                        found=await search(recipe_view_filter=bounds,limit=50)
                        expected=[int(i) for i,row in observations.items() if truth(row,bounds) is True]
                        self.assertEqual([row['recipe_id'] for row in found['records']],expected)
                        self.assertEqual(found['recipe_view_unknown_count'],
                                         sum(truth(row,bounds) is None for row in observations.values()))
                    await search(error=True,recipe_view_filter={
                        'minimum_skill_ups_reported':2,'maximum_skill_ups_reported':1})
                    narrowed = await search(recipe_id=27,recipe_view_filter={'learned':True,'can_skill_up':True})
                    self.assertEqual([row['recipe_id'] for row in narrowed['records']],[27])
                    rejected = await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':27,'crafts':1})
                    self.assertTrue(rejected.isError)  # View discovery does not override conflicting selected learning.
                    for bad in [{},{'learned':None},{'learned':1},{'can_skill_up':'true'},
                        {'minimum_skill_ups_reported':-1},{'minimum_skill_ups_reported':True},
                        {'minimum_skill_ups_reported':4294967296},{'can_skill_up':True,'unexpected':'PRIVATE'}]:
                        await search(error=True,recipe_view_filter=bad)
            self.assertEqual(original,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
