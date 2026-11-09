"""Pinned pet option enums cannot silently turn unsupported values into no passive."""
import unittest
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import re
import zipfile
import subprocess
import sys
import tempfile
from tools import run_forever_sim as adapter
from test_simulator_pet_level import pet_inputs


class HunterPetOptionTests(unittest.TestCase):
    def test_real_module_rejects_private_speed_before_engine_or_journal(self):
        template,review,encounter=pet_inputs()
        template['raid']['parties'][0]['players'][0]['hunter']['options']['classOptions']['petAttackSpeed']='PRIVATE'
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder)
            for name,value in [('template',template),('review',review),('encounter',encounter)]:
                (base/(name+'.json')).write_text(json.dumps(value))
            result=subprocess.run([sys.executable,'-m','tools.run_forever_sim',
                '--engine',str(base/'missing.exe'),'--engine-sha256','a'*64,
                '--template',str(base/'template.json'),'--equipment-review',str(base/'review.json'),
                '--encounter-lookup',str(base/'encounter.json'),'--simulation-journal',str(base/'journal')],
                cwd=Path(__file__).resolve().parents[1],capture_output=True,text=True,timeout=15)
            self.assertEqual(result.returncode,1)
            self.assertEqual(result.stdout,'')
            self.assertIn('Forever simulation failed (ValueError)',result.stderr)
            self.assertNotIn('PRIVATE',result.stderr)
            self.assertNotIn(str(base),result.stderr)
            self.assertFalse((base/'journal').exists())

    def test_supported_aliases_and_defaults_are_template_only(self):
        template,review,encounter=pet_inputs()
        player=template['raid']['parties'][0]['players'][0]
        for index,name in enumerate(adapter.HUNTER_PET_ATTACK_SPEEDS):
            for selected in [index,name]:
                for key in ['petAttackSpeed','pet_attack_speed']:
                    player['hunter']['options']={'class_options':{'pet_type':'Wolf','pet_uptime':'1e0',key:selected}}
                    before=deepcopy(template)
                    adapter.request_from(template,review,encounter)
                    report=adapter.hunter_quiver_observation(player,None)['template_pet_reference']
                    self.assertEqual(report['attack_speed'],name)
                    self.assertEqual(report['pet_type'],'Wolf')
                    self.assertEqual(report['pet_uptime'],1)
                    self.assertTrue(all(value is False for key,value in report.items() if key.endswith('_verified')))
                    self.assertEqual(template,before)
        for options in [{},None,{'classOptions':None},{'classOptions':{'petAttackSpeed':None}}]:
            player['hunter']['options']=options
            self.assertEqual(adapter.hunter_quiver_observation(player,None)['template_pet_reference']['attack_speed'],'PetAttackSpeedNone')

    def test_bad_speed_rejects_even_without_active_pet(self):
        template,review,encounter=pet_inputs()
        player=template['raid']['parties'][0]['players'][0]
        for selected in [-1,10,999999,True,False,1.0,'1','PRIVATE',[],{}]:
            player['hunter']['options']={'classOptions':{'petAttackSpeed':selected}}
            for invoke in [lambda:adapter.request_from(template,review,encounter),
                           lambda:adapter.hunter_quiver_observation(player,None)]:
                with self.assertRaisesRegex(ValueError,'Invalid Hunter pet attack speed') as caught:invoke()
                self.assertNotIn('PRIVATE',str(caught.exception))
        player['hunter']['options']={'classOptions':{'petAttackSpeed':0,'pet_attack_speed':0}}
        with self.assertRaisesRegex(ValueError,'Duplicate Hunter pet option'):
            adapter.hunter_quiver_observation(player,None)

    def test_pinned_enum_inventory(self):
        source=os.environ.get('AZEROTH_TEST_SIM_SOURCE_ZIP')
        if not source:self.skipTest('Explicit pinned ZIP required; no download')
        self.assertEqual(hashlib.sha256(Path(source).read_bytes()).hexdigest(),'e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712')
        with zipfile.ZipFile(source) as archive:
            proto=archive.read(next(name for name in archive.namelist() if name.endswith('/proto/hunter.proto'))).decode()
        section=proto.split('message HunterOptions {',1)[1].split('\nmessage Hunter {',1)[0]
        enums={name:dict((key,int(value)) for key,value in re.findall(r'(\w+)\s*=\s*(\d+)',body))
               for name,body in re.findall(r'enum (\w+)\s*\{([^}]+)\}',section)}
        self.assertEqual(enums,{
            'PetType':dict(zip(adapter.HUNTER_PET_TYPES,range(len(adapter.HUNTER_PET_TYPES)))),
            'PetAttackSpeed':dict(zip(adapter.HUNTER_PET_ATTACK_SPEEDS,range(10))),
            'QuiverBonus':{name:item for name,(item,_) in adapter.HUNTER_QUIVERS.items()},
            'Ammo':{name:item for name,(item,_) in adapter.HUNTER_AMMO.items()}})

    def test_unknown_speed_rejects_before_request(self):
        template,review,encounter=pet_inputs()
        template['raid']['parties'][0]['players'][0]['hunter']['options']['classOptions']['petAttackSpeed']=999999
        with self.assertRaises(ValueError):adapter.request_from(template,review,encounter)


if __name__=='__main__':unittest.main()
