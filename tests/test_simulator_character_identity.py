"""Direct simulation binds class/race to the same saved character as the gear."""
from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.rank_forever_slot import check_template_identity
from tools.run_forever_sim import check_character_identity, request_from, ConflictingCharacterIdentity


class CharacterIdentityTests(unittest.TestCase):
    def test_shared_mapping_preserves_known_unknown_and_aliases(self):
        for declared in [{'class':'ClassHunter','race':'RaceOrc'}, {'class':3,'race':6}]:
            self.assertEqual(check_character_identity(declared,{'class':'HUNTER','race':'Orc'}),
                             {'class':'matches','race':'matches'})
        self.assertEqual(check_character_identity({'race':9},{'race':'Scourge'})['race'],'matches')
        for observed in [None,{}, {'class':None,'race':'UnmappedRace'}]:
            self.assertEqual(check_character_identity({},observed),{'class':'unverified','race':'unverified'})
        for observed in [[], {'class':True}, {'race':2}]:
            with self.assertRaises(ValueError):check_character_identity({},observed)
        for declared in [{'class':'ClassMage','race':'RaceOrc'}, {'class':3,'race':4},
                         {'class':True,'race':6}]:
            with self.assertRaises(ConflictingCharacterIdentity):
                check_character_identity(declared,{'class':'HUNTER','race':'Orc'})
        for observed in [{'class':'UNMAPPED','race':'Human'}, {'class':'MAGE','race':None}]:
            with self.assertRaises(ConflictingCharacterIdentity):
                check_character_identity({'class':3,'race':6},observed)

    def test_direct_review_check_and_ranker_share_identity_guard(self):
        template,review,encounter=inputs()
        captured={'class':'HUNTER','race':'Orc'}
        review['captured_character_identity']=captured
        original=deepcopy((template,review,encounter))
        request_from(template,review,encounter)
        self.assertEqual((template,review,encounter),original)
        for field,value in [('class','MAGE'),('race','Human')]:
            mismatch={**captured,field:value}
            with self.assertRaises(ConflictingCharacterIdentity):
                request_from(template,{**review,'captured_character_identity':mismatch},encounter)
            with self.assertRaises(ConflictingCharacterIdentity):
                check_template_identity(template,{**mismatch,'level':60},require_simulator_level=False)

    def test_module_cli_checks_every_review_before_engine_execution(self):
        template,review,encounter=inputs()
        good={**review,'captured_character_identity':{'class':'HUNTER','race':'Orc'}}
        for field,value in [('class','MAGE'),('race','Human')]:
            bad=deepcopy(good);bad['captured_character_identity'][field]=value
            with tempfile.TemporaryDirectory() as folder:
                root=Path(folder)
                for name,data in [('template',template),('good',good),('bad',bad),('encounter',encounter)]:
                    (root/f'{name}.json').write_text(json.dumps(data))
                result=subprocess.run([sys.executable,'-m','tools.run_forever_sim',
                    '--engine',str(root/'must-not-run.exe'),'--engine-sha256','a'*64,
                    '--template',str(root/'template.json'),'--equipment-review',str(root/'good.json'),
                    '--equipment-review',str(root/'bad.json'),'--encounter-lookup',str(root/'encounter.json')],
                    capture_output=True,text=True,timeout=10)
                self.assertEqual(result.returncode,1)
                self.assertEqual(result.stdout,'')
                self.assertIn('class or race disagrees',result.stderr)
                self.assertIn('Match the template',result.stderr)
                self.assertNotIn(str(root),result.stderr)


class CharacterIdentityMcpTests(unittest.IsolatedAsyncioTestCase):
    async def test_review_projects_only_class_and_race(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));cases=[]
            for stats in [None,{'level':60,'class':'HUNTER','race':'Orc','client_version':'1.60.1'}]:
                value=fixture()
                value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
                value['sections']['equipment']={'16':{'item_id':123}}
                if stats:value['sections']['stats']=stats
                cases.append((store.ingest(value),stats))
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for key,stats in cases:
                        response=await session.call_tool('review_equipment_selection',{'snapshot_id':key,
                            'selections':[{'target_slot':16,'source':{'kind':'equipment','slot':16},'expected_item_id':123}]})
                        self.assertFalse(response.isError)
                        data=json.loads(response.content[0].text)
                        self.assertEqual(data['captured_character_identity'],
                            {field:stats.get(field) if stats else None for field in ['class','race']})


if __name__=='__main__':unittest.main()
