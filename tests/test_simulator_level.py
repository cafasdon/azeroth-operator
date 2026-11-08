"""The fixed level-60 engine must not rank a different captured character level."""
from contextlib import redirect_stdout, redirect_stderr
from copy import deepcopy
import io
import json
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest
from unittest.mock import patch
from tools.rank_forever_slot import check_template_identity, main
from tools.run_forever_sim import request_from, UnsupportedSimulationLevel


def inputs():
    template={'raid':{'parties':[{'players':[{'class':'ClassHunter','race':'RaceOrc','hunter':{},
        'talentsString':'1','rotation':{'type':'TypeAPL','priorityList':[{'action':{'castSpell':{}}}]}}]}]}}
    review={'client':{'family':'forever'},'captured_character_level':60,
            'checks':{'simulator_equipment_spec':{'items':[{}]*17}}}
    encounter={'family':'forever','section':'encounters','simulation_input':{
        'encounter':{'duration':45,'durationVariation':0,'executeProportion20':0,'executeProportion35':0,'targets':[{}]},
        'simOptions':{'iterations':1000,'randomSeed':'1'}}}
    return template,review,encounter


class SimulatorLevelTests(unittest.TestCase):
    def test_identity_known_mismatch_and_unknown(self):
        template,_,_=inputs()
        stats={'class':'HUNTER','race':'Orc','level':60}
        self.assertEqual(check_template_identity(template,stats)['level'],'matches')
        for level in [None,0,-1,True,60.0,'60',59,61]:
            with self.subTest(level=level):
                with self.assertRaises(UnsupportedSimulationLevel):check_template_identity(template,{**stats,'level':level})
                report=check_template_identity(template,{**stats,'level':level},require_simulator_level=False)
                self.assertEqual(report['simulator_character_level'],60)
                self.assertNotEqual(report['level'],'matches')

    def test_direct_adapter_requires_review_level_before_mapping(self):
        template,review,encounter=inputs()
        original=deepcopy((template,review,encounter))
        self.assertEqual(len(request_from(template,review,encounter)['raid']['parties'][0]['players'][0]['equipment']['items']),17)
        self.assertEqual((template,review,encounter),original)
        for level in [None,0,20,61,True,60.0,'60']:
            with self.subTest(level=level),self.assertRaises(UnsupportedSimulationLevel):
                request_from(template,{**review,'captured_character_level':level},encounter)
        old=deepcopy(review);old.pop('captured_character_level')
        with self.assertRaises(UnsupportedSimulationLevel):request_from(template,old,encounter)

    def test_cli_rejects_before_simulation_but_keeps_plan_only(self):
        template,review,encounter=inputs()
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);binary=root/'mcp.exe';binary.touch()
            template_path=root/'template.json';template_path.write_text(json.dumps(template))
            encounter_path=root/'encounter.json';encounter_path.write_text(json.dumps(encounter))
            base=['rank','--mcp',str(binary),'--snapshot-id','fixture','--slot','head','--template',str(template_path)]
            async def collect(*args,**kwargs):
                return [([],review)],[],{},{'level':20},None,None,{}
            sim=base+['--engine',str(binary),'--engine-sha256','a'*64,'--encounter-lookup',str(encounter_path)]
            with patch('tools.rank_forever_slot.collect',collect),patch('tools.rank_forever_slot.rank_encounter') as run:
                with patch('sys.argv',sim),redirect_stderr(io.StringIO()) as errors:
                    with self.assertRaises(SystemExit) as stopped:main()
                    self.assertEqual(stopped.exception.code,1)
                    self.assertIn('captured level of 60',errors.getvalue())
                run.assert_not_called()

                with patch('sys.argv',base+['--plan-only']),redirect_stdout(io.StringIO()) as output:
                    main()
                result=json.loads(output.getvalue())
                self.assertEqual(result['mode'],'plan_only')
                self.assertEqual(result['template_identity']['level'],'unsupported_for_simulation')
                run.assert_not_called()

    def test_direct_module_cli_explains_legacy_review_remedy(self):
        template,review,encounter=inputs()
        review.pop('captured_character_level')
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            for name,data in [('template',template),('review',review),('encounter',encounter)]:
                (root/f'{name}.json').write_text(json.dumps(data))
            result=subprocess.run([sys.executable,'-m','tools.run_forever_sim',
                '--engine',str(root/'must-not-run.exe'),'--engine-sha256','a'*64,
                '--template',str(root/'template.json'),'--equipment-review',str(root/'review.json'),
                '--encounter-lookup',str(root/'encounter.json')],capture_output=True,text=True,timeout=10)
            self.assertEqual(result.returncode,1)
            self.assertEqual(result.stdout,'')
            self.assertIn('captured_character_level=60',result.stderr)
            self.assertIn('Re-export the saved capture',result.stderr)
            self.assertNotIn(str(root),result.stderr)


if __name__=='__main__':unittest.main()
