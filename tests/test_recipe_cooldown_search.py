"""Discover saved cooldown evidence without refreshing time, learning or stock."""
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


class RecipeCooldownSearchTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_three_valued_matrix_composition_paging_and_provenance(self):
        root=Path(__file__).resolve().parents[1]
        binary=(root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base=fixture(); base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['character']['name']='PRIVATE_CHARACTER'
        cooldowns=[None,{'remaining_seconds':0},{'remaining_seconds':30,'daily':True},
            {'charges':0,'max_charges':2},{'charges':1,'max_charges':2,'remaining_seconds':90,'daily':False},
            {'charges':1,'max_charges':2},{'charges':0,'max_charges':2,'remaining_seconds':1},None,None,
            {'charges':4294967295,'max_charges':4294967295,'remaining_seconds':4294967295}]
        rows={str(i):{'status':'fixed_items','reagents':{'10':i%3+1},
            **({'cooldown':cooldown} if cooldown is not None else {})} for i,cooldown in enumerate(cooldowns,1)}
        rows['8']={'status':'unsupported_required_slots'}; rows['9']={'status':'unavailable'}
        base['sections']['profession_recipe_materials']={
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':rows}
        base['sections']['profession_recipe_view']={'coverage':'filtered_local_recipe_list','profession_id':164,
            'entries':{str(i):{'learned':True,'can_skill_up':i%2==0} for i in range(1,11)}}
        base['sections']['bags']={'0':{'1':{'item_id':10,'count':1}}}
        base['sections']['selected_recipe']={'recipe_id':1,'name':'Fixture','learned':True,
            'requirements':'fixed_items','reagents':{'10':2},'cooldown':{'charges':9,'max_charges':9}}
        def state(row):
            cooldown=row.get('cooldown',{})
            if 'charges' in cooldown: return 'available' if cooldown['charges']>0 else 'waiting'
            if 'remaining_seconds' in cooldown: return 'available' if cooldown['remaining_seconds']==0 else 'waiting'
            return 'unknown'
        def truth(row,criteria):
            tests=[]; cooldown=row.get('cooldown',{})
            if 'state' in criteria: tests.append(state(row)==criteria['state'])
            for criterion,key,compare in [('minimum_charges','charges',lambda x,y:x>=y),
                ('maximum_remaining_seconds','remaining_seconds',lambda x,y:x<=y)]:
                if criterion in criteria: tests.append(None if key not in cooldown else compare(cooldown[key],criteria[criterion]))
            return False if False in tests else None if None in tests else True
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder)); key=store.ingest(base)
            future=deepcopy(base); future['captured_at']='2025-01-02T00:00:00Z'
            future['sections']['bags']['0']['1']['count']=6
            future['sections']['profession_recipe_materials']['entries']['1']['cooldown']={'remaining_seconds':0}
            inventory=store.ingest(future)
            variants={}
            for name in ['missing','empty']:
                sample=deepcopy(base)
                if name=='missing': sample['sections'].pop('profession_recipe_materials')
                else: sample['sections']['profession_recipe_materials']['entries']={}
                variants[name]=store.ingest(sample)
            large=deepcopy(base); large['sections']['profession_recipe_materials']['entries']={str(i):{
                'status':'fixed_items','reagents':{},'cooldown':{'remaining_seconds':0 if i%2==0 else 30}}
                for i in range(1,4097)}
            large_key=store.ingest(large)
            before={p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def search(error=False,**extra):
                        reply=await session.call_tool('search_forever_recipes',{'snapshot_id':key,**extra})
                        self.assertEqual(bool(reply.isError),error,reply); self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    baseline=await search()
                    self.assertEqual(baseline,await search(cooldown_filter=None))
                    self.assertNotIn('cooldown_unknown_count',baseline)
                    for expected_state,minimum,maximum in product([None,'available','waiting','unknown'],
                        [None,0,1,2,4294967295],[None,0,30,4294967295]):
                        criteria={field:value for field,value in [('state',expected_state),('minimum_charges',minimum),
                            ('maximum_remaining_seconds',maximum)] if value is not None}
                        if not criteria: continue
                        wanted=[int(i) for i,row in rows.items() if truth(row,criteria) is True]
                        unknown=sum(truth(row,criteria) is None for row in rows.values())
                        nonmatching=sum(truth(row,criteria) is False for row in rows.values())
                        found=await search(cooldown_filter=criteria,limit=50)
                        self.assertEqual([row['recipe_id'] for row in found['records']],wanted)
                        self.assertEqual((found['cooldown_unknown_count'],found['cooldown_nonmatching_count']),(unknown,nonmatching))
                        self.assertEqual(found['cooldown_captured_at'],base['captured_at'])
                        for row in found['records']: self.assertEqual(row['captured_cooldown_state'],state(rows[str(row['recipe_id'])]))
                    ready={'state':'available'}
                    first=await search(cooldown_filter=ready,limit=2)
                    second=await search(cooldown_filter=ready,limit=2,offset=first['next_offset'])
                    self.assertEqual([row['recipe_id'] for row in first['records']+second['records']],[2,5,6,10])
                    flags={'learned':True,'can_skill_up':True}; materials={'scope':'bags','minimum_crafts':2}
                    combined=await search(cooldown_filter=ready,recipe_view_filter=flags,material_filter=materials,
                        inventory_snapshot_id=inventory)
                    self.assertEqual([row['recipe_id'] for row in combined['records']],[2,6,10])
                    self.assertEqual(combined['inventory_captured_at'],future['captured_at'])
                    self.assertEqual(combined['cooldown_captured_at'],base['captured_at'])
                    unknown=await search(recipe_id=1,cooldown_filter={'state':'unknown'})
                    self.assertEqual([row['recipe_id'] for row in unknown['records']],[1])
                    for i in [1,2,3,4,5,6,7,10]:
                        reply=await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':i,'crafts':1})
                        self.assertFalse(reply.isError,reply)
                        self.assertEqual(json.loads(reply.content[0].text)['captured_cooldown_state'],state(rows[str(i)]))
                    for name,expected in [('missing',None),('empty',0)]:
                        found=await search(snapshot_id=variants[name],cooldown_filter=ready)
                        self.assertEqual((found['matched_row_count'],found['cooldown_unknown_count']),(expected,expected))
                    found=await search(snapshot_id=large_key,cooldown_filter=ready,offset=2040,limit=50)
                    self.assertEqual(found['matched_row_count'],2048)
                    self.assertEqual([row['recipe_id'] for row in found['records']],list(range(4082,4097,2)))
                    self.assertIsNone(found['next_offset'])
                    for bad in [{},{'state':None},{'state':'wrong'},{'state':True},{'minimum_charges':True},
                        {'minimum_charges':1.0},{'minimum_charges':-1},{'minimum_charges':4294967296},
                        {'maximum_remaining_seconds':[]},{'maximum_remaining_seconds':1.0},{'unexpected':0}]:
                        await search(error=True,cooldown_filter=bad)
                    for flag in ['craftability_verified','complete_recipe_knowledge_verified','game_execution','live']:
                        self.assertFalse(combined[flag])
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
