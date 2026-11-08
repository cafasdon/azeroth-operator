"""Pinned Hunter pets are level 60, independent of the active pet's actual level."""
from contextlib import redirect_stdout, redirect_stderr
from copy import deepcopy
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from test_simulator_level import inputs
from tools.rank_forever_slot import main
from tools.run_forever_sim import check_hunter_pet_level, request_from, UnsupportedSimulationPetLevel


def pet_inputs(snake=False):
    template,review,encounter=inputs()
    template['raid']['parties'][0]['players'][0]['hunter']={'options':
        {'class_options':{'pet_type':19,'pet_uptime':1}} if snake else
        {'classOptions':{'petType':'Wolf','petUptime':1}}}
    review['captured_active_pet']={'present':True,'level':60,'family_id':1}
    return template,review,encounter


class PetLevelTests(unittest.TestCase):
    def test_aliases_disabled_and_unknown_levels(self):
        for snake in [False,True]:
            template,review,_=pet_inputs(snake)
            player=template['raid']['parties'][0]['players'][0]
            result=check_hunter_pet_level(player,review['captured_active_pet'])
            self.assertEqual(result['level'],'matches')
            self.assertEqual(result['family_and_training'],'unverified')
            for observed in [None,{}, {'present':False,'level':60},
                             *({'present':True,'level':x} for x in [None,0,17,59,61,True,60.0,'60'])]:
                with self.subTest(observed=observed),self.assertRaises(UnsupportedSimulationPetLevel):
                    check_hunter_pet_level(player,observed)
                self.assertNotEqual(check_hunter_pet_level(player,observed,require_level=False)['level'],'matches')
        for options in [{},{'classOptions':{'petType':'PetNone','petUptime':1}},
                        {'classOptions':{'petType':'Wolf','petUptime':0}},
                        {'classOptions':{'petType':None,'petUptime':1}}]:
            self.assertEqual(check_hunter_pet_level({'hunter':{'options':options}},None)['level'],'not_applicable')
        self.assertEqual(check_hunter_pet_level({'mage':{}},None)['level'],'not_applicable')

    def test_quoted_protojson_uptime_numbers(self):
        for class_field,uptime_field in [('classOptions','petUptime'),('class_options','pet_uptime')]:
            for text,status in [('1','matches'),('1e0','matches'),('5e-1','matches'),
                                ('0','not_applicable'),('-0','not_applicable'),('1e-999','not_applicable')]:
                player={'hunter':{'options':{class_field:{'petType':'Wolf',uptime_field:text}}}}
                original=deepcopy(player)
                result=check_hunter_pet_level(player,{'present':True,'level':60})
                self.assertEqual(result['level'],status)
                self.assertEqual(player,original)
                if status=='matches':
                    with self.assertRaises(UnsupportedSimulationPetLevel):check_hunter_pet_level(player,None)
            for text in ['1e999','NaN','Infinity','-Infinity',' 1','+1','01','1junk','']:
                with self.subTest(text=text),self.assertRaises(ValueError):
                    check_hunter_pet_level({'hunter':{'options':{class_field:{'petType':'Wolf',uptime_field:text}}}},None)

    def test_invalid_options_and_duplicate_aliases(self):
        for value in [True,19.0,20,-1,'PRIVATE',[],{}]:
            with self.subTest(value=value),self.assertRaises(ValueError):
                check_hunter_pet_level({'hunter':{'options':{'classOptions':{'petType':value,'petUptime':1}}}},None)
        for value in [True,-1,2,float('nan'),float('inf'),10**400,'PRIVATE',[],{}]:
            with self.subTest(value=value),self.assertRaises(ValueError):
                check_hunter_pet_level({'hunter':{'options':{'classOptions':{'petType':19,'petUptime':value}}}},None)
        for options in [{'classOptions':{},'class_options':{}},
                        {'classOptions':{'petType':0,'pet_type':19}},
                        {'classOptions':{'petUptime':0,'pet_uptime':1}}]:
            with self.assertRaises(ValueError):check_hunter_pet_level({'hunter':{'options':options}},None)

    def test_request_construction_and_input_immutability(self):
        for snake in [False,True]:
            template,review,encounter=pet_inputs(snake)
            original=deepcopy((template,review,encounter))
            request_from(template,review,encounter)
            self.assertEqual((template,review,encounter),original)
            legacy=deepcopy(review);legacy.pop('captured_active_pet')
            with self.assertRaises(UnsupportedSimulationPetLevel):request_from(template,legacy,encounter)

    def test_actual_cli_rejects_second_review_and_ranker_keeps_plan_only(self):
        template,review,encounter=pet_inputs()
        bad=deepcopy(review);bad['captured_active_pet']['level']=17
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);binary=root/'never-run.exe';binary.touch()
            for name,data in [('template',template),('good',review),('bad',bad),('encounter',encounter)]:
                (root/f'{name}.json').write_text(json.dumps(data))
            result=subprocess.run([sys.executable,'-m','tools.run_forever_sim',
                '--engine',str(root/'absent.exe'),'--engine-sha256','a'*64,
                '--template',str(root/'template.json'),'--equipment-review',str(root/'good.json'),
                '--equipment-review',str(root/'bad.json'),'--encounter-lookup',str(root/'encounter.json')],
                capture_output=True,text=True,timeout=10)
            self.assertEqual(result.returncode,1)
            self.assertEqual(result.stdout,'')
            self.assertIn('saved active-pet level of 60',result.stderr)
            self.assertNotIn(str(root),result.stderr)
            base=['rank','--mcp',str(binary),'--snapshot-id','fixture','--slot','head',
                  '--template',str(root/'template.json')]
            async def collect(*args,**kwargs):return [([],bad)],[],{},{'level':60},None,None,{}
            with patch('tools.rank_forever_slot.collect',collect),patch('tools.rank_forever_slot.rank_encounter') as run:
                with patch('sys.argv',base+['--engine',str(binary),'--engine-sha256','a'*64,
                    '--encounter-lookup',str(root/'encounter.json')]),redirect_stderr(io.StringIO()) as errors:
                    with self.assertRaises(SystemExit):main()
                    self.assertIn('saved active-pet level of 60',errors.getvalue())
                run.assert_not_called()
                with patch('sys.argv',base+['--plan-only']),redirect_stdout(io.StringIO()) as output:main()
                status=json.loads(output.getvalue())['template_identity']['hunter_pet']
                self.assertEqual(status['level'],'unsupported_for_simulation')
                self.assertEqual(status['captured_pet_level'],17)
                run.assert_not_called()


if __name__=='__main__':unittest.main()
