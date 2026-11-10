"""Reported skill-up metadata stays distinct from eligibility and guaranteed gains."""
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


class RecipeSkillUpTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_filters_unknowns_and_bounded_candidates(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        rows={str(i):{'learned':True,'can_skill_up':True,'skill_ups_reported':2,'relative_difficulty':'optimal'} for i in range(1,56)}
        rows['80']={'learned':True,'can_skill_up':False,'skill_ups_reported':0,'relative_difficulty':'trivial'}
        rows['81']={'learned':True};rows['82']={'learned':False,'can_skill_up':True};rows['83']={'can_skill_up':True}
        value['sections']['profession_recipe_view']={'coverage':'filtered_local_recipe_list','profession_id':164,'entries':rows}
        missing=deepcopy(value);missing['sections'].pop('profession_recipe_view')
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));key=store.ingest(value);missing_key=store.ingest(missing)
            malformed=[]
            for field,invalids in [('can_skill_up',[None,0,'PRIVATE']),('skill_ups_reported',[None,True,-1,1.5,2**32]),
                                   ('relative_difficulty',[None,0,'orange',{}])]:
                for invalid in invalids:
                    bad=deepcopy(value);bad['sections']['profession_recipe_view']['entries']['1'][field]=invalid
                    with self.assertRaises(ValueError):validate_snapshot(bad)
                    raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode();bad_key=hashlib.sha256(raw).hexdigest()
                    (Path(folder)/f'{bad_key}.json').write_bytes(raw);malformed.append(bad_key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,**args):
                        response=await session.call_tool(tool,args);self.assertFalse(response.isError,response.content)
                        return json.loads(response.content[0].text)
                    summary=(await call('character_summary',snapshot_id=key))['profession_skill_up_candidates']
                    self.assertEqual(summary['candidate_count'],55);self.assertTrue(summary['truncated'])
                    self.assertEqual([r['recipe_id'] for r in summary['recipes']],list(range(1,51)))
                    self.assertEqual(summary['learned_rows_with_unknown_skill_up_flag'],1)
                    self.assertFalse(summary['eligibility_verified'])
                    self.assertIsNone((await call('character_summary',snapshot_id=missing_key))['profession_skill_up_candidates'])
                    for recipe,options,expected in [(1,{'can_skill_up':True},1),(1,{'can_skill_up':False},0),
                            (80,{'can_skill_up':False,'minimum_skill_ups_reported':0},1),
                            (81,{'can_skill_up':False},0),(81,{'minimum_skill_ups_reported':0},0),
                            (1,{'minimum_skill_ups_reported':2},1),(1,{'minimum_skill_ups_reported':3},0),
                            (82,{'learning':'learned','can_skill_up':True},0),
                            (80,{'maximum_skill_ups_reported':0,'relative_difficulty':'trivial'},1),
                            (81,{'maximum_skill_ups_reported':0},0),
                            (1,{'minimum_skill_ups_reported':2,'maximum_skill_ups_reported':2},1),
                            (1,{'maximum_skill_ups_reported':1},0),
                            (1,{'relative_difficulty':'trivial'},0)]:
                        result=await call('list_snapshots',family='forever',profession_recipe={'recipe_id':recipe,**options})
                        self.assertEqual(len(result['snapshots']),expected,(recipe,options))
                        if expected:
                            match=result['snapshots'][0]['profession_recipe_match']
                            self.assertEqual(match['can_skill_up'],rows[str(recipe)]['can_skill_up'])
                    for bad in malformed:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'profession_recipe_view'})
                        self.assertTrue(response.isError);self.assertNotIn('PRIVATE',response.content[0].text)
                    for options in [{'minimum_skill_ups_reported':-1},{'minimum_skill_ups_reported':True},
                                    {'minimum_skill_ups_reported':2**32},{'can_skill_up':1},
                                    {'minimum_skill_ups_reported':2,'maximum_skill_ups_reported':1},
                                    {'relative_difficulty':'orange'}]:
                        response=await session.call_tool('list_snapshots',{'family':'forever','profession_recipe':{'recipe_id':1,**options}})
                        self.assertTrue(response.isError)


if __name__=='__main__':unittest.main()
