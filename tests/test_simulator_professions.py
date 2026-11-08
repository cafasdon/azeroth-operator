"""Captured profession identities reach real MCP reviews and simulator checks."""
from copy import deepcopy
from contextlib import redirect_stdout, redirect_stderr
import io
import json
import os
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest
from unittest.mock import patch
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.run_forever_sim import check_profession_identity, request_from
from tools.rank_forever_slot import main


def profession(skill):
    return {'name':'Synthetic profession','rank':1,'max_rank':75,'skill_line':skill}


class ProfessionIdentityTests(unittest.TestCase):
    def test_equivalence_and_unknowns(self):
        observed={'2':profession(202),'5':profession(185)}
        for player in [{'profession1':'Engineering'}, {'profession2':4},
                       {'profession1':0,'profession2':'Engineering'}]:
            self.assertEqual(check_profession_identity(player,observed)['status'],'matches')
        self.assertEqual(check_profession_identity({}, {})['status'],'matches')
        for observation in [None, {'1':profession(999999)}]:
            self.assertEqual(check_profession_identity({'profession1':4},observation)['status'],'unverified')
        with self.assertRaises(ValueError):check_profession_identity({'profession1':4},{})
        with self.assertRaises(ValueError):check_profession_identity({},observed)
        mixed={'1':profession(202),'2':profession(999999)}
        with self.assertRaisesRegex(ValueError,'professions disagree'):
            check_profession_identity({'profession1':'Blacksmithing','profession2':'Tailoring'},mixed)
        self.assertEqual(check_profession_identity({'profession1':4},mixed)['status'],'unverified')
        self.assertEqual(check_profession_identity({},mixed,require_match=False)['status'],'mismatch')
        for value in [True,4.0,7,'Jewelcrafting','PRIVATE']:
            with self.subTest(value=value), self.assertRaises(ValueError):
                check_profession_identity({'profession1':value},observed)
        with self.assertRaises(ValueError):check_profession_identity({'profession1':4,'profession2':'Engineering'},observed)

    def test_direct_adapter_uses_review_before_simulation(self):
        template,review,encounter=inputs()
        player=template['raid']['parties'][0]['players'][0]
        player['profession1']='Engineering'
        review['captured_professions']={'2':profession(202)}
        original=deepcopy((template,review,encounter))
        self.assertEqual(request_from(template,review,encounter)['raid']['parties'][0]['players'][0]['profession1'],'Engineering')
        self.assertEqual((template,review,encounter),original)
        for captured in [{}, {'1':profession(164)}]:
            with self.assertRaisesRegex(ValueError,'professions disagree'):
                request_from(template,{**review,'captured_professions':captured},encounter)

    def test_cli_mismatch_is_actionable_and_plan_only_remains_available(self):
        template,review,encounter=inputs()
        template['raid']['parties'][0]['players'][0]['profession1']='Engineering'
        review['captured_professions']={}
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);binary=root/'never-run.exe';binary.touch()
            for name,value in [('template',template),('review',review),('encounter',encounter)]:
                (root/f'{name}.json').write_text(json.dumps(value))
            direct=subprocess.run([sys.executable,'-m','tools.run_forever_sim',
                '--engine',str(root/'absent.exe'),'--engine-sha256','a'*64,
                '--template',str(root/'template.json'),'--equipment-review',str(root/'review.json'),
                '--encounter-lookup',str(root/'encounter.json')],capture_output=True,text=True,timeout=10)
            self.assertEqual(direct.returncode,1)
            self.assertIn('Match the template',direct.stderr)
            self.assertEqual(direct.stdout,'')
            self.assertNotIn(str(root),direct.stderr)
            base=['rank','--mcp',str(binary),'--snapshot-id','fixture','--slot','head',
                  '--template',str(root/'template.json')]
            async def collect(*args,**kwargs):return [([],review)],[],{},{'level':60},None,None,{}
            with patch('tools.rank_forever_slot.collect',collect),patch('tools.rank_forever_slot.rank_encounter') as run:
                with patch('sys.argv',base+['--engine',str(binary),'--engine-sha256','a'*64,
                     '--encounter-lookup',str(root/'encounter.json')]),redirect_stderr(io.StringIO()) as errors:
                    with self.assertRaises(SystemExit):main()
                    self.assertIn('professions disagree',errors.getvalue())
                    self.assertNotIn(str(root),errors.getvalue())
                run.assert_not_called()
                with patch('sys.argv',base+['--plan-only']),redirect_stdout(io.StringIO()) as output:main()
                self.assertEqual(json.loads(output.getvalue())['template_identity']['professions']['status'],'mismatch')
                run.assert_not_called()


class ProfessionReviewCallerTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_carries_observation_and_unknown(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder)); cases=[]
            for observed in [None,{}, {'2':profession(202),'5':profession(185)}]:
                value=fixture()
                value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
                value['sections']['equipment']={'16':{'item_id':123}}
                if observed is not None:value['sections']['professions']=observed
                cases.append((store.ingest(value),observed))
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for key,observed in cases:
                        response=await session.call_tool('review_equipment_selection',{'snapshot_id':key,
                            'selections':[{'target_slot':16,'source':{'kind':'equipment','slot':16},'expected_item_id':123}]})
                        self.assertFalse(response.isError)
                        data=json.loads(response.content[0].text)
                        self.assertIn('captured_professions',data)
                        self.assertEqual(data['captured_professions'],observed)


if __name__=='__main__':unittest.main()
