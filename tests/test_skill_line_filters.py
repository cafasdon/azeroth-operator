"""Actual owner-local skill discovery and filtered inspection of base ranks."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class SkillFilterTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_predicates_unknowns_latest_paging_and_owner_scope(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        def row(rank,cap,category=None,**extra):
            result={'name':'Synthetic skill','rank':rank,'max_rank':cap,**extra}
            if category is not None:result['category_id']=category
            return result
        def capture(name,day,rows=None):
            value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['character']['name']=name;value['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            if rows is not None:value['sections']['skill_lines']=rows
            value['sections']['bags']={'0':{'1':{'item_id':11,'count':2}}}
            return value
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as other:
            store=SnapshotStore(Path(folder));keys={}
            cases={'old_empty':capture('Alpha',1,{'43':row(200,300,9)}),'new_empty':capture('Alpha',2,{}),
                'old_short':capture('Bravo',1,{'43':row(200,300,9)}),'new_short':capture('Bravo',2,{'43':row(100,300,9)}),
                'mixed':capture('Gamma',1,{'2':row(0,0,2),'10':row(200,300),
                    '43':row(250,300,9,modifier=100,temporary_points=50),'100':row(300,300,9),
                    '42':row(10,100,9,modifier=-5,temporary_points=0)}),
                'missing':capture('Delta',1),
                'maximum':capture('High',1,{str(i):row(0,4294967295,9) for i in range(1,513)})}
            cases['mixed']['sections']['professions']={'1':{'name':'Synthetic','rank':300,'max_rank':300,'skill_line':202}}
            for name,value in cases.items():keys[name]=store.ingest(value)
            foreign=SnapshotStore(Path(other)).ingest(capture('PRIVATE_OTHER_OWNER',1,{'43':row(300,300,9)}))
            retail=fixture();retail_key=store.ingest(retail)
            before={path:path.read_bytes() for base in [Path(folder),Path(other)] for path in base.glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(other,str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    async def listing(filter=None,**extra):
                        return await call('list_snapshots',family='forever',**({'skill_line':filter} if filter is not None else {}),**extra)
                    async def inspect(key,filter,**page):
                        return await call('inspect_snapshot',snapshot_id=key,section='skill_lines',
                            skill_line_filter={'filter':filter,**page})
                    selected={'skill_line_id':43,'minimum_rank':200}
                    history=await listing(selected)
                    self.assertEqual({r['snapshot_id'] for r in history['snapshots']},{keys[n] for n in ['old_empty','old_short','mixed']})
                    latest=await listing(selected,latest_per_character=True)
                    self.assertEqual([r['snapshot_id'] for r in latest['snapshots']],[keys['mixed']])
                    composed=await listing(selected,profession_skill_line=202,profession_min_rank=300)
                    self.assertEqual([r['snapshot_id'] for r in composed['snapshots']],[keys['mixed']])
                    self.assertEqual((await listing(selected,inventory_item={'item_id':11,'storage':'bags','minimum_quantity':3}))['snapshots'],[])
                    dated=await listing(selected,latest_per_character=True,captured_through='2025-01-01T23:59:59Z')
                    self.assertEqual({r['snapshot_id'] for r in dated['snapshots']},{keys[n] for n in ['old_empty','old_short','mixed']})
                    mixed=await inspect(keys['mixed'],{'category_id':9,'minimum_rank':100})
                    self.assertEqual([r['skill_line_id'] for r in mixed['data']['rows']],[43,100])
                    self.assertEqual([mixed['data'][f] for f in ['captured_count','matching_count','unknown_count','nonmatching_count']],[5,2,1,2])
                    self.assertEqual(mixed['data']['rows'][0]['points_to_observed_cap'],50)
                    self.assertEqual(mixed['data']['rows'][0]['observation']['modifier'],100)
                    self.assertNotIn('target_rank',mixed['data'])
                    self.assertNotIn('points_to_target',mixed['data']['rows'][0])
                    target=(await inspect(keys['mixed'],{'minimum_rank':0},target_rank=275))['data']
                    self.assertEqual(target['target_rank'],275)
                    target_rows={r['skill_line_id']:r for r in target['rows']}
                    self.assertEqual(target_rows[43]['points_to_target'],25)
                    self.assertFalse(target_rows[43]['observed_base_target_reached'])
                    self.assertFalse(target_rows[43]['target_exceeds_observed_cap'])
                    self.assertEqual(target_rows[100]['points_to_target'],0)
                    self.assertTrue(target_rows[100]['observed_base_target_reached'])
                    self.assertTrue(target_rows[42]['target_exceeds_observed_cap'])
                    maximum_target=(await inspect(keys['maximum'],{'minimum_rank':0},target_rank=4294967295,limit=1))['data']
                    self.assertEqual(maximum_target['rows'][0]['points_to_target'],4294967295)
                    for invalid_target in [True,1.0,'250',-1,4294967296]:
                        await call('inspect_snapshot',True,snapshot_id=keys['mixed'],section='skill_lines',
                            skill_line_filter={'filter':{'minimum_rank':0},'target_rank':invalid_target})
                    modifiers={'modifier':-5,'temporary_points':0}
                    matching=(await inspect(keys['mixed'],modifiers))['data']
                    self.assertEqual([r['skill_line_id'] for r in matching['rows']],[42])
                    self.assertEqual([matching[f] for f in ['matching_count','unknown_count','nonmatching_count']],[1,3,1])
                    contradicted=(await inspect(keys['mixed'],{'category_id':2,**modifiers}))['data']
                    self.assertEqual([contradicted[f] for f in ['matching_count','unknown_count','nonmatching_count']],[0,2,3])
                    self.assertEqual([r['snapshot_id'] for r in (await listing(modifiers))['snapshots']],[keys['mixed']])
                    under=await inspect(keys['mixed'],{'minimum_points_to_cap':1})
                    self.assertEqual([r['skill_line_id'] for r in under['data']['rows']],[10,42,43])
                    capped=await inspect(keys['mixed'],{'minimum_rank':300})
                    self.assertEqual([r['skill_line_id'] for r in capped['data']['rows']],[100])
                    zero=await listing({'skill_line_id':2,'minimum_rank':0,'maximum_cap':0,'maximum_points_to_cap':0})
                    self.assertEqual([r['snapshot_id'] for r in zero['snapshots']],[keys['mixed']])
                    missing=await inspect(keys['missing'],{'minimum_rank':0})
                    self.assertFalse(missing['available'])
                    for field in ['rows','captured_count','matching_count','unknown_count','nonmatching_count','has_more']:
                        self.assertIsNone(missing['data'][field])
                    empty=await inspect(keys['new_empty'],{'minimum_rank':0})
                    self.assertTrue(empty['available']);self.assertEqual(empty['data']['rows'],[])
                    self.assertEqual(empty['data']['matching_count'],0);self.assertFalse(empty['data']['has_more'])
                    maximum=(await listing({'minimum_points_to_cap':4294967295}))['snapshots'][0]['skill_line_match']
                    self.assertEqual(maximum['matching_skill_line_count'],512);self.assertTrue(maximum['matches_truncated'])
                    self.assertEqual([r['skill_line_id'] for r in maximum['matching_skill_lines']],list(range(1,21)))
                    tail=(await inspect(keys['maximum'],{'minimum_rank':0},offset=500,limit=50))['data']
                    self.assertEqual([r['skill_line_id'] for r in tail['rows']],list(range(501,513)));self.assertFalse(tail['has_more'])
                    huge=(await inspect(keys['maximum'],{'minimum_rank':0},offset=18446744073709551615,limit=50))['data']
                    self.assertEqual(huge['rows'],[]);self.assertEqual(huge['matching_count'],512);self.assertFalse(huge['has_more'])
                    for report in [maximum,mixed['data'],empty['data'],missing['data']]:
                        self.assertEqual(report['rank_basis'],'base_rank_only')
                        for flag in ['complete_skill_coverage_verified','current_training_eligibility_verified',
                            'effective_rank_inferred','simulator_input_modified','learning_state_inferred','game_execution','live']:
                            self.assertFalse(report[flag])
                    for offset in [0,1,3,18446744073709551615]:
                        page=await listing(selected,offset=offset,limit=1)
                        self.assertEqual(page['snapshots'],history['snapshots'][offset:offset+1])
                    plain=await listing()
                    self.assertTrue(all('skill_line_match' not in r for r in plain['snapshots']))
                    raw=await call('inspect_snapshot',snapshot_id=keys['mixed'],section='skill_lines')
                    self.assertEqual(raw['data'],cases['mixed']['sections']['skill_lines'])
                    await call('inspect_snapshot',True,snapshot_id=retail_key,section='skill_lines',skill_line_filter={'filter':{'minimum_rank':0}})
                    await call('inspect_snapshot',True,snapshot_id=keys['mixed'],section='professions',skill_line_filter={'filter':selected})
                    for family in ['retail','classic']:await call('list_snapshots',True,family=family,skill_line=selected)
                    invalid=[{}, {'minimum_rank':2,'maximum_rank':1},{'minimum_cap':2,'maximum_cap':1},
                        {'minimum_points_to_cap':2,'maximum_points_to_cap':1}, {'skill_line_id':0},{'category_id':0},
                        {'minimum_rank':True},{'minimum_rank':1.0},{'minimum_rank':'PRIVATE'},{'minimum_rank':-1},
                        {'maximum_cap':4294967296},{'skill_line_id':4294967296},{'extra':'PRIVATE'}]
                    for filter in invalid:
                        await call('list_snapshots',True,family='forever',skill_line=filter)
                        await call('inspect_snapshot',True,snapshot_id=keys['mixed'],section='skill_lines',skill_line_filter={'filter':filter})
                    for bad in [{'filter':selected,'limit':0},{'filter':selected,'limit':51},{'filter':selected,'offset':-1},
                        {'filter':selected,'limit':'PRIVATE'},{'minimum_rank':1},{'filter':selected,'extra':'PRIVATE'}]:
                        await call('inspect_snapshot',True,snapshot_id=keys['mixed'],section='skill_lines',skill_line_filter=bad)
                    changed=deepcopy(cases['mixed'])
                    changed['captured_at']='2025-01-02T00:00:00Z'
                    changed['sections']['skill_lines']['42']['modifier']=0
                    changed['sections']['skill_lines']['43'].pop('temporary_points')
                    changed['sections']['skill_lines']['100']['temporary_points']=0
                    changed_key=store.ingest(changed)
                    comparison=await call('compare_snapshots',before_id=keys['mixed'],after_id=changed_key)
                    self.assertEqual(comparison['skill_line_progress']['changes'],[])
                    self.assertEqual(comparison['skill_line_progress']['modifier_observation_changes'],[
                        {'skill_line_id':42,'fields':{'modifier':{'before':-5,'after':0,'delta':5}}},
                        {'skill_line_id':43,'fields':{'temporary_points':{'before':50,'after':None,'delta':None}}},
                        {'skill_line_id':100,'fields':{'temporary_points':{'before':None,'after':0,'delta':None}}}])
            for path,data in before.items():self.assertEqual(path.read_bytes(),data)


if __name__=='__main__':unittest.main()
