"""Filtered local profession observations through the owner-scoped MCP reader."""
import copy
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


class ProfessionRecipeViewTests(unittest.IsolatedAsyncioTestCase):
    async def test_readback_unknowns_and_schema_parity(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            value=fixture()
            value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            view={'coverage':'filtered_local_recipe_list','profession_id':164,
                  'rank_observation':{'rank':100,'max_rank':150,'skill_modifier':-5},
                  'entries':{'7':{'name':'Recipe','learned':True},'8':{'learned':False},'9':{}}}
            value['sections']['profession_recipe_view']=view
            key=store.ingest(value)
            empty=copy.deepcopy(value)
            empty['character']['name']='Empty'
            empty['sections']['profession_recipe_view']['entries']={}
            empty_key=store.ingest(empty)
            missing=copy.deepcopy(value)
            missing['sections'].pop('profession_recipe_view')
            missing['captured_at']='2025-01-02T00:00:00Z'
            missing_key=store.ingest(missing)
            later=copy.deepcopy(missing)
            later['captured_at']='2025-01-03T00:00:00Z'
            store.ingest(later)
            nullable=copy.deepcopy(value)
            nullable['character']['name']='Null'
            nullable['sections']['profession_recipe_view'].pop('rank_observation')
            nullable['sections']['profession_recipe_view']['entries']['9']['learned']=None
            nullable_key=store.ingest(nullable)
            bad_keys=[]
            for change in [
                lambda x:x['client'].update(family='retail'),
                lambda x:x['sections']['profession_recipe_view'].update(coverage='all_known_recipes'),
                lambda x:x['sections']['profession_recipe_view'].update(profession_id=True),
                lambda x:x['sections']['profession_recipe_view'].update(rank_observation=None),
                lambda x:x['sections']['profession_recipe_view'].update(rank_observation={'rank':151,'max_rank':150}),
                lambda x:x['sections']['profession_recipe_view'].update(rank_observation={'rank':0,'max_rank':0}),
                lambda x:x['sections']['profession_recipe_view']['rank_observation'].update(skill_modifier=None),
                lambda x:x['sections']['profession_recipe_view']['rank_observation'].update(skill_modifier=True),
                lambda x:x['sections']['profession_recipe_view']['rank_observation'].update(skill_modifier=2**31),
                lambda x:x['sections']['profession_recipe_view']['entries']['7'].update(crafter='private'),
                lambda x:x['sections']['profession_recipe_view']['entries']['7'].update(learned=1),
                lambda x:x['sections']['profession_recipe_view']['entries']['7'].update(name='é'*129),
                lambda x:x['sections']['profession_recipe_view']['entries'].update({'07':{}}),
            ]:
                bad=copy.deepcopy(value)
                change(bad)
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
                bad_key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/f'{bad_key}.json').write_bytes(raw)
                bad_keys.append(bad_key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    summary=await session.call_tool('character_summary',{'snapshot_id':key})
                    self.assertFalse(summary.isError)
                    self.assertEqual(json.loads(summary.content[0].text)['profession_skill_up_candidates']['rank_observation'],
                                     view['rank_observation'])
                    async def search(**arguments):
                        response=await session.call_tool('list_snapshots',{'family':'forever',**arguments})
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)
                    for recipe,state,expected in [(7,'learned',1),(7,'unlearned',0),(7,'unknown',0),
                        (8,'unlearned',1),(8,'learned',0),(9,'unknown',1),(9,'unlearned',0),(10,'unknown',0)]:
                        args={'character':value['character'],'profession_recipe':{
                            'recipe_id':recipe,'profession_id':164,'learning':state}}
                        rows=(await search(**args))['snapshots']
                        self.assertEqual(len(rows),expected)
                        if rows:
                            self.assertEqual(rows[0]['snapshot_id'],key)
                            self.assertEqual(rows[0]['profession_recipe_match']['coverage'],'filtered_local_recipe_list')
                            self.assertEqual(rows[0]['profession_recipe_match']['recipe_id'],recipe)
                    args={'character':value['character'],'profession_recipe':{'recipe_id':7}}
                    self.assertEqual(len((await search(**args))['snapshots']),1)
                    for maximum, expected in ((0,0),(49,0),(50,1)):
                        rows=(await search(profession_recipe={
                            'recipe_id':7,'maximum_rank_points_to_cap':maximum}))['snapshots']
                        self.assertEqual(len(rows),expected)
                        if rows:
                            self.assertEqual(rows[0]['snapshot_id'],key)
                            match=rows[0]['profession_recipe_match']
                            self.assertEqual(match['rank_observation'],view['rank_observation'])
                            self.assertEqual(match['rank_points_to_observed_cap'],50)
                    self.assertEqual((await search(**args,latest_per_character=True))['snapshots'],[])
                    self.assertEqual(len((await search(**args,latest_per_character=True,
                        captured_through='2025-01-01T00:00:00Z'))['snapshots']),1)
                    self.assertEqual((await search(character=value['character'],profession_recipe={
                        'recipe_id':7,'profession_id':165}))['snapshots'],[])
                    rows=(await search(character=nullable['character'],profession_recipe={
                        'recipe_id':9,'learning':'unknown'}))['snapshots']
                    self.assertEqual(rows[0]['snapshot_id'],nullable_key)
                    self.assertIsNone(rows[0]['profession_recipe_match']['learned'])
                    self.assertTrue(all('profession_recipe_match' not in row for row in (await search())['snapshots']))
                    for bad in [{'recipe_id':0},{'recipe_id':True},{'recipe_id':2**32},{'recipe_id':7,'profession_id':0},
                                {'recipe_id':7,'learning':'missing'},{'recipe_id':7,'learned':False},{}]:
                        response=await session.call_tool('list_snapshots',{'family':'forever','profession_recipe':bad})
                        self.assertTrue(response.isError)
                    response=await session.call_tool('list_snapshots',{'family':'retail','profession_recipe':{'recipe_id':7}})
                    self.assertTrue(response.isError)
                    changed=copy.deepcopy(value)
                    changed['captured_at']='2025-01-02T00:00:00Z'
                    changed_view=changed['sections']['profession_recipe_view']
                    changed_view['rank_observation']={'rank':110,'max_rank':175}
                    changed_view['entries']['7']['learned']=False
                    changed_view['entries']['9']['learned']=True
                    changed_view['entries']['9'].update(can_skill_up=False,skill_ups_reported=0)
                    changed_view['entries'].pop('8')
                    changed_view['entries']['10']={'learned':False}
                    changed_key=store.ingest(changed)
                    async def compare(left,right,**arguments):
                        response=await session.call_tool('compare_snapshots',{'before_id':left,'after_id':right,**arguments})
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)
                    delta=(await compare(key,changed_key,profession_recipe_changes=True))['profession_recipe_changes']
                    self.assertTrue(delta['same_profession'])
                    self.assertEqual(delta['rank_observation_change'],{
                        'before':{'rank':100,'max_rank':150,'rank_points_to_observed_cap':50},
                        'after':{'rank':110,'max_rank':175,'rank_points_to_observed_cap':65},
                        'rank_delta':10,'max_rank_delta':25,'rank_points_to_observed_cap_delta':15,
                    })
                    self.assertEqual(delta['learning_observation_changes'],[
                        {'recipe_id':7,'before_learned':True,'after_learned':False},
                        {'recipe_id':9,'before_learned':None,'after_learned':True}])
                    self.assertEqual(delta['observed_only_before'],[8])
                    self.assertEqual(delta['observed_only_after'],[10])
                    self.assertEqual(delta['skill_up_observation_changes'],[
                        {'recipe_id':9,'fields':{'can_skill_up':{'before':None,'after':False},
                            'skill_ups_reported':{'before':None,'after':0}}}])
                    other_profession=copy.deepcopy(changed)
                    other_profession['sections']['profession_recipe_view']['profession_id']=165
                    other_key=store.ingest(other_profession)
                    delta=(await compare(key,other_key,profession_recipe_changes=True))['profession_recipe_changes']
                    self.assertFalse(delta['same_profession'])
                    for field in ['observed_only_before','observed_only_after','learning_observation_changes',
                                  'skill_up_observation_changes','rank_observation_change']:
                        self.assertIsNone(delta[field])
                    renamed=copy.deepcopy(value)
                    renamed['sections']['profession_recipe_view']['entries']['7']['name']='Translated'
                    renamed['sections']['profession_recipe_view']['entries']['9']['learned']=None
                    renamed_key=store.ingest(renamed)
                    delta=(await compare(key,renamed_key,profession_recipe_changes=True))['profession_recipe_changes']
                    self.assertEqual(delta['learning_observation_changes'],[])
                    self.assertEqual(delta['skill_up_observation_changes'],[])
                    self.assertEqual(delta['rank_observation_change']['rank_delta'],0)
                    legacy=copy.deepcopy(changed)
                    legacy['sections']['profession_recipe_view'].pop('rank_observation')
                    delta=(await compare(key,store.ingest(legacy),profession_recipe_changes=True))['profession_recipe_changes']
                    self.assertIsNone(delta['rank_observation_change'])
                    self.assertIsNone((await compare(key,missing_key,profession_recipe_changes=True))['profession_recipe_changes'])
                    self.assertNotIn('profession_recipe_changes',await compare(key,changed_key))
                    retail_key=store.ingest(fixture())
                    response=await session.call_tool('compare_snapshots',{'before_id':retail_key,'after_id':retail_key,
                                                                         'profession_recipe_changes':True})
                    self.assertTrue(response.isError)
                    response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'profession_recipe_view'})
                    self.assertFalse(response.isError)
                    self.assertEqual(json.loads(response.content[0].text)['data'],view)
                    for source,expected in [(key,{'observed':3,'learned':1,'unlearned':1,'learning_unknown':1}),
                        (empty_key,{'observed':0,'learned':0,'unlearned':0,'learning_unknown':0}),(missing_key,None)]:
                        response=await session.call_tool('character_summary',{'snapshot_id':source})
                        self.assertFalse(response.isError)
                        self.assertEqual(json.loads(response.content[0].text)['profession_recipe_view_counts'],expected)
                    for source in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':source,'section':'profession_recipe_view'})
                        self.assertTrue(response.isError)


if __name__=='__main__':
    unittest.main()
