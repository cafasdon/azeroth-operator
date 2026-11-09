"""Pinned mapping, real MCP projection and pre-engine purchased-rank conflicts."""
from copy import deepcopy
from contextlib import redirect_stdout, redirect_stderr
import io
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_forever_talent_definitions import sample
from test_simulator_level import inputs
from tools.forever_talent_identity import (check_talent_identity, talent_map,
    ConflictingTalentObservation, MAP_PATH, MAP_SHA256)
from tools.run_forever_sim import request_from
from tools import run_forever_sim as adapter
from tools import positional_equipment as layout
from tools.rank_forever_slot import main as rank_main


def observation(definition=134422, purchased=1, maximum=5):
    return {'config_id':7,'observed_tree_ids':{'10':True},'observed_node_count':4,
        'staged_changes_observed':False,'nodes':{'101':{'entry_id':111,'rank':9,'tree_id':10,
            'definition_id':definition,'purchased_rank':purchased,'max_rank':maximum}}}


class TalentIdentityTests(unittest.TestCase):
    def test_opt_in_actual_engine_keeps_matching_request_and_metrics(self):
        if not all(os.environ.get(key) for key in ['AZEROTH_POSITIONAL_ENGINE','AZEROTH_QUALIFY_TEMPLATE']):
            self.skipTest('Explicit pinned local engine and public template required; no downloads')
        engine=Path(os.environ['AZEROTH_POSITIONAL_ENGINE']).resolve(strict=True)
        path=Path(os.environ['AZEROTH_QUALIFY_TEMPLATE']).resolve(strict=True)
        self.assertEqual(hashlib.sha256(engine.read_bytes()).hexdigest(),layout.ENGINE_SHA256)
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),
            '761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0')
        template=json.loads(path.read_text());player=adapter.single_player(template['raid'])
        _,review,encounter=inputs()
        review['captured_active_pet']={'present':True,'level':60}
        review['checks']['simulator_equipment_spec']=deepcopy(player['equipment'])
        encounter['simulation_input']['simOptions']={'iterations':200,'randomSeed':'20261109'}
        baseline=request_from(template,review,encounter)
        first=talent_map()['classes']['hunter']['talents'][0]
        part=player['talentsString'].split('-')[0]
        rank=min(int(part[0]) if part else 0,first['max_rank'])
        review['captured_talent_observation']=observation(first['definition_id'],rank,first['max_rank'])
        matched=request_from(template,review,encounter)
        self.assertEqual(matched,baseline)
        reference=adapter.run(engine,layout.ENGINE_SHA256,baseline)
        self.assertEqual(adapter.run(engine,layout.ENGINE_SHA256,matched),reference)
        review['captured_talent_observation']['nodes']['101']['purchased_rank']=(rank+1)%(first['max_rank']+1)
        with self.assertRaises(ConflictingTalentObservation):request_from(template,review,encounter)

    def test_all_classes_tree_boundaries_effective_ranks_and_no_mutation(self):
        self.assertEqual(hashlib.sha256(MAP_PATH.read_bytes()).hexdigest(),MAP_SHA256)
        for name, tree in talent_map()['classes'].items():
            offset=0
            for index,size in enumerate(tree['tree_sizes']):
                for position in [0,size-1]:
                    row=tree['talents'][offset+position]
                    parts=['']*3;parts[index]='0'*position+'9'
                    player={'talentsString':'-'.join(parts)}
                    captured=observation(row['definition_id'],row['max_rank'],row['max_rank'])
                    original=deepcopy((player,captured))
                    report=check_talent_identity(player,captured,name)
                    self.assertEqual(report['status'],'observed_ranks_match',(name,index,position))
                    self.assertFalse(report['allocation_match_verified'])
                    self.assertEqual(report['compared_definition_count'],1)
                    self.assertEqual((player,captured),original)
                    captured['nodes']['101']['purchased_rank']=0
                    with self.assertRaises(ConflictingTalentObservation):check_talent_identity(player,captured,name)
                offset+=size
        # The upstream third Priest size does not affect three-tree offsets.
        priest=talent_map()['classes']['priest']
        self.assertEqual(priest['tree_sizes'],[18,17,17])
        self.assertEqual(priest['engine_tree_sizes'],[18,17,18])

    def test_unknowns_staged_and_ambiguity(self):
        player={'talentsString':'1'}
        for captured in [None, observation(999999), observation(maximum=4)]:
            self.assertEqual(check_talent_identity(player,captured,'hunter')['status'],'unverified')
        captured=observation();captured['nodes']['101']={'entry_id':111,'rank':1,'tree_id':10}
        self.assertEqual(check_talent_identity(player,captured,'hunter')['unknown_node_count'],1)
        captured=observation(purchased=0);del captured['staged_changes_observed']
        report=check_talent_identity(player,captured,'hunter')
        self.assertEqual(report['status'],'unverified');self.assertEqual(len(report['conflicts']),1)
        captured['staged_changes_observed']=True
        self.assertEqual(check_talent_identity(player,captured,'hunter',False)['status'],'staged_changes')
        with self.assertRaises(ConflictingTalentObservation):check_talent_identity(player,captured,'hunter')
        captured=observation();captured['nodes']['102']=deepcopy(captured['nodes']['101'])
        with self.assertRaises(ValueError):check_talent_identity(player,captured,'hunter')
        for value in [True,1,None,'PRIVATE','１','1----','0'*17,'---','1a']:
            with self.subTest(value=value),self.assertRaises(ValueError):
                check_talent_identity({'talentsString':value},observation(),'hunter')
        with self.assertRaises(ValueError):
            check_talent_identity({'talentsString':'1','talents_string':'0'},observation(),'hunter')
        for change in [lambda o:o.update(staged_changes_observed=None),
                       lambda o:o.update(observed_tree_ids={1:True}),
                       lambda o:o['nodes']['101'].update(purchased_rank=True),
                       lambda o:o['nodes']['101'].update(name='PRIVATE'),
                       lambda o:o['nodes']['101'].pop('max_rank')]:
            captured=observation();change(captured)
            with self.assertRaises(ValueError):check_talent_identity(player,captured,'hunter')

    def test_request_and_real_cli_reject_before_engine(self):
        template,review,encounter=inputs(); review['captured_talent_observation']=observation()
        request_from(template,review,encounter)
        review['captured_talent_observation']['nodes']['101']['purchased_rank']=0
        with self.assertRaises(ConflictingTalentObservation):request_from(template,review,encounter)
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            for name,value in [('template',template),('review',review),('encounter',encounter)]:
                (root/f'{name}.json').write_text(json.dumps(value))
            result=subprocess.run([sys.executable,'-m','tools.run_forever_sim','--engine',str(root/'never-run.exe'),
                '--engine-sha256','a'*64,'--template',str(root/'template.json'),
                '--equipment-review',str(root/'review.json'),'--encounter-lookup',str(root/'encounter.json')],
                capture_output=True,text=True,timeout=10)
            self.assertEqual(result.returncode,1);self.assertEqual(result.stdout,'')
            self.assertIn('purchased ranks',result.stderr);self.assertNotIn(str(root),result.stderr)

    def test_ranker_reports_plan_conflicts_and_refuses_simulation(self):
        template,review,encounter=inputs()
        review['captured_talent_observation']=observation(purchased=0)
        async def collect(*args,**kwargs):return [([],review)],[],{},{'level':60},None,None,{}
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder); binary=root/'never-run.exe';binary.touch()
            (root/'template.json').write_text(json.dumps(template))
            (root/'encounter.json').write_text(json.dumps(encounter))
            base=['rank','--mcp',str(binary),'--snapshot-id','fixture','--slot','head',
                  '--template',str(root/'template.json')]
            with patch('tools.rank_forever_slot.collect',collect),patch('tools.rank_forever_slot.rank_encounter') as run:
                with patch('sys.argv',base+['--engine',str(binary),'--engine-sha256','a'*64,
                     '--encounter-lookup',str(root/'encounter.json')]),redirect_stderr(io.StringIO()) as errors:
                    with self.assertRaises(SystemExit):rank_main()
                    self.assertIn('purchased ranks',errors.getvalue())
                    self.assertNotIn(str(root),errors.getvalue())
                run.assert_not_called()
                with patch('sys.argv',base+['--plan-only']),redirect_stdout(io.StringIO()) as output:rank_main()
                report=json.loads(output.getvalue())['template_identity']['talents']
                self.assertEqual(report['status'],'mismatch')
                self.assertEqual(report['conflicts'][0]['captured_purchased_rank'],0)
                self.assertFalse(report['allocation_match_verified'])
                run.assert_not_called()


class TalentReviewTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_projection_and_legacy(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder)); value=sample()
            value['sections']['equipment']={'16':{'item_id':123}}
            value['sections']['talents'].update(observation())
            value['sections']['talents']['name']='PRIVATE CONFIG'
            captured=store.ingest(value)
            legacy=deepcopy(value)
            for field in ['observed_tree_ids','observed_node_count','staged_changes_observed']:
                legacy['sections']['talents'].pop(field)
            for node in legacy['sections']['talents']['nodes'].values():
                for field in ['definition_id','purchased_rank','max_rank']:node.pop(field)
            old=store.ingest(legacy)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for key,expected in [(captured,observation()),(old,None)]:
                        reply=await session.call_tool('review_equipment_selection',{'snapshot_id':key,
                            'selections':[{'target_slot':16,'source':{'kind':'equipment','slot':16},'expected_item_id':123}]})
                        self.assertFalse(reply.isError,reply)
                        data=json.loads(reply.content[0].text)
                        self.assertEqual(data['captured_talent_observation'],expected)
                        self.assertNotIn('PRIVATE CONFIG',reply.content[0].text)
                        template,_,_=inputs()
                        self.assertEqual(check_talent_identity(template['raid']['parties'][0]['players'][0],
                            data['captured_talent_observation'],'hunter')['status'],
                            'observed_ranks_match' if expected else 'unverified')


if __name__=='__main__':unittest.main()
