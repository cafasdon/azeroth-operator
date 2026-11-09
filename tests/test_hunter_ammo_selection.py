"""Unsupported ammunition must not silently become no modeled ranged damage."""
import unittest
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import re
import zipfile
import tempfile
import io
from contextlib import redirect_stdout, redirect_stderr
from unittest.mock import patch
from tools import run_forever_sim as adapter
from test_simulator_level import inputs


class HunterAmmoSelectionTests(unittest.TestCase):
    def test_unknown_numeric_ammo_rejects_before_request_creation(self):
        template,review,encounter=inputs()
        template['raid']['parties'][0]['players'][0]['hunter']['options']={'classOptions':{'ammo':999999}}
        with self.assertRaises(ValueError):adapter.request_from(template,review,encounter)

    def test_supported_selections_and_null_remain_template_only(self):
        template,review,encounter=inputs();player=template['raid']['parties'][0]['players'][0]
        review['captured_equipped_bag_items']=[{'bag':1,'item_id':8217,'inventory_slot':20}]
        for name,(item,dps) in adapter.HUNTER_AMMO.items():
            for selected in [name,item]:
                for spelling in ['classOptions','class_options']:
                    player['hunter']['options']={spelling:{'ammo':selected}}
                    before=deepcopy((template,review,encounter))
                    report=adapter.hunter_quiver_observation(player,review['captured_equipped_bag_items'])
                    ammo=report['template_ammo_reference']
                    self.assertEqual((ammo['item_id'],ammo['bonus_ranged_dps']),(item,dps))
                    self.assertEqual(ammo['basis'],'pinned_template_selection_only')
                    for flag in ['identity_verified','availability_verified','native_effect_verified']:
                        self.assertIs(ammo[flag],False)
                    self.assertIs(report['ammo_identity_verified'],False)
                    request=adapter.request_from(template,review,encounter)
                    self.assertEqual(request['raid']['parties'][0]['players'][0]['hunter']['options'],player['hunter']['options'])
                    self.assertEqual((template,review,encounter),before)
        for options in [{}, {'classOptions':None}, {'classOptions':{}}, {'class_options':{'ammo':None}}]:
            player['hunter']['options']=options
            report=adapter.hunter_quiver_observation(player,None)
            self.assertEqual(report['template_ammo_reference']['item_id'],0)
            self.assertIs(report['ammo_identity_verified'],False)
            self.assertEqual(report['comparison'],'capture_unavailable')

    def test_malformed_private_selection_never_reaches_engine(self):
        template,review,encounter=inputs();player=template['raid']['parties'][0]['players'][0]
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder)
            for name,data in [('review',review),('encounter',encounter)]:
                (base/(name+'.json')).write_text(json.dumps(data))
            argv=['run_forever_sim','--engine',str(base/'must-not-launch.exe'),'--engine-sha256','a'*64,
                '--template',str(base/'template.json'),'--equipment-review',str(base/'review.json'),
                '--encounter-lookup',str(base/'encounter.json'),'--simulation-journal',str(base/'must-not-create')]
            for selected in [True,False,2512.0,'2512','PRIVATE',{},[],999999,-1,2**32]:
                player['hunter']['options']={'classOptions':{'ammo':selected}}
                with self.assertRaises(ValueError) as caught:adapter.request_from(template,review,encounter)
                self.assertNotIn('PRIVATE',str(caught.exception))
                (base/'template.json').write_text(json.dumps(template))
                output,error=io.StringIO(),io.StringIO()
                with patch('sys.argv',argv),patch.object(adapter,'run') as run,redirect_stdout(output),redirect_stderr(error):
                    with self.assertRaises(SystemExit) as stopped:adapter.main()
                    self.assertEqual(stopped.exception.code,1)
                    run.assert_not_called()
                self.assertEqual(output.getvalue(),'')
                self.assertNotIn('PRIVATE',error.getvalue())
                self.assertFalse((base/'must-not-create').exists())

    def test_pinned_source_and_optional_real_engine(self):
        source=os.environ.get('AZEROTH_TEST_SIM_SOURCE_ZIP')
        if not source:self.skipTest('Explicit pinned ZIP required; no download')
        self.assertEqual(hashlib.sha256(Path(source).read_bytes()).hexdigest(),'e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712')
        with zipfile.ZipFile(source) as z:
            data=z.read(next(name for name in z.namelist() if name.endswith('/sim/hunter/ammo_auto_gen.go'))).decode()
            proto=z.read(next(name for name in z.namelist() if name.endswith('/proto/hunter.proto'))).decode()
        found={name:(int(item),float(dps)) for name,item,dps in re.findall(r'"(\w+)":\s*\{Name:\s*"[^"]+",\s*ItemID:\s*(\d+),\s*DPS:\s*([\d.]+)\}',data)}
        self.assertEqual(found,{key:value for key,value in adapter.HUNTER_AMMO.items() if key!='AmmoNone'})
        enum=dict((name,int(item)) for name,item in re.findall(r'(\w+)\s*=\s*(\d+)',proto.split('enum Ammo {',1)[1].split('}',1)[0]))
        self.assertEqual(enum,{name:item for name,(item,_) in adapter.HUNTER_AMMO.items()})
        engine=os.environ.get('AZEROTH_TEST_REAL_ENGINE');public=os.environ.get('AZEROTH_TEST_PUBLIC_TEMPLATE')
        if not engine or not public:self.skipTest('Explicit qualified engine/template required')
        template=json.loads(Path(public).read_text());player=template['raid']['parties'][0]['players'][0]
        _,review,encounter=inputs();review['checks']['simulator_equipment_spec']=deepcopy(player['equipment'])
        encounter['simulation_input']['simOptions']={'iterations':200,'randomSeed':'20261109'}
        summaries=[]
        for selected in ['AmmoNone','ThoriumHeadedArrow']:
            player['hunter']['options']['classOptions']['ammo']=selected
            request=adapter.request_from(template,review,encounter)
            summaries.append(adapter.run(Path(engine),adapter.layout.ENGINE_SHA256,request))
        self.assertGreater(summaries[1]['mean_dps'],summaries[0]['mean_dps'])


if __name__=='__main__':unittest.main()
