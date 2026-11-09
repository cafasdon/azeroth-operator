"""Pinned quiver template validation and partial bag-identity evidence."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import re
import unittest
import zipfile
from tools import run_forever_sim as adapter
from test_simulator_level import inputs


class QuiverObservationTests(unittest.TestCase):
    def test_unknown_numeric_quiver_is_not_silent_none(self):
        template,review,encounter=inputs()
        template['raid']['parties'][0]['players'][0]['hunter']['options']={'classOptions':{'quiverBonus':999999}}
        with self.assertRaises(ValueError):adapter.request_from(template,review,encounter)

    def test_partial_observations_names_numeric_null_and_no_mutation(self):
        template,review,encounter=inputs();player=template['raid']['parties'][0]['players'][0]
        observed=[{'bag':1,'item_id':8217,'inventory_slot':20},{'bag':2,'item_id':100,'inventory_slot':21}]
        review['captured_equipped_bag_items']=observed
        before=deepcopy((template,review,encounter))
        for selected in ['QuickdrawQuiver',8217]:
            player['hunter']['options']={'classOptions':{'quiverBonus':selected}}
            original=deepcopy((player,observed))
            report=adapter.hunter_quiver_observation(player,observed)
            self.assertEqual(report['comparison'],'observed_item_match')
            self.assertEqual(report['template_pinned_haste_percent'],13)
            self.assertEqual(report['observed_pinned_quiver_item_ids'],[8217])
            self.assertEqual(report['unmapped_observed_item_count'],1)
            self.assertIs(report['template_haste_below_observed_reference'],False)
            self.assertEqual((player,observed),original)
            for flag in ['complete_equipped_bag_identity_verified','quiver_bonus_verified','ammo_identity_verified','simulator_input_modified','game_execution']:
                self.assertIs(report[flag],False)
        player['hunter']['options']={'class_options':{'quiver_bonus':0}}
        report=adapter.hunter_quiver_observation(player,observed)
        self.assertEqual(report['comparison'],'none_selection_unverified')
        self.assertIs(report['template_haste_below_observed_reference'],True)
        self.assertEqual(adapter.hunter_quiver_observation(player,None)['comparison'],'capture_unavailable')
        self.assertIsNone(adapter.hunter_quiver_observation(player,[])['highest_observed_pinned_haste_percent'])
        player['hunter']['options']={'classOptions':{'quiverBonus':'AncientSinewWrappedLamina'}}
        self.assertEqual(adapter.hunter_quiver_observation(player,observed)['comparison'],'not_observed_in_partial_capture')
        # Partial mismatch is diagnostic, not an inferred absence or request mutation.
        request=adapter.request_from(template,review,encounter)
        self.assertEqual(request['raid']['parties'][0]['players'][0]['hunter']['options'],player['hunter']['options'])
        self.assertEqual(review,before[1]);self.assertEqual(encounter,before[2])
        repeated=observed+[{'bag':3,'item_id':8217,'inventory_slot':22}]
        report=adapter.hunter_quiver_observation(player,repeated)
        self.assertEqual(report['highest_observed_pinned_haste_percent'],13)
        self.assertEqual(report['captured_identity_count'],3)

    def test_malformed_fragments_and_duplicate_aliases_reject(self):
        template,_,_=inputs();player=template['raid']['parties'][0]['players'][0]
        good={'bag':1,'item_id':8217,'inventory_slot':20}
        for observed in ['PRIVATE',{},[None],[{}],[{**good,'item_id':True}],
            [{**good,'inventory_slot':None}],[{**good,'bag':0}],[{**good,'bag':6}],
            [{**good,'item_id':2**32}],[{**good,'inventory_slot':256}],
            [{**good,'private':'PRIVATE'}],[good,good],
            [good,{**good,'bag':2}],[good,{**good,'inventory_slot':21}], [good]*6]:
            with self.assertRaises(ValueError) as caught:adapter.hunter_quiver_observation(player,observed)
            self.assertNotIn('PRIVATE',str(caught.exception))
        for declared in [True,8217.0,'8217','PRIVATE',{},[],999999,-1]:
            player['hunter']['options']={'classOptions':{'quiverBonus':declared}}
            with self.assertRaises(ValueError):adapter.hunter_quiver_observation(player,None)
        player['hunter']['options']={'classOptions':{'quiverBonus':8217,'quiver_bonus':8217}}
        with self.assertRaises(ValueError):adapter.hunter_quiver_observation(player,None)

    def test_pinned_reference_source_and_optional_real_engine(self):
        source=os.environ.get('AZEROTH_TEST_SIM_SOURCE_ZIP')
        if not source:self.skipTest('Explicit pinned source ZIP required; no download')
        archive=Path(source).read_bytes()
        self.assertEqual(hashlib.sha256(archive).hexdigest(),'e1309ddd725a89996a8806aac3aa3af7bfae7486feef972077639230e2ff2712')
        with zipfile.ZipFile(source) as z:
            name=next(name for name in z.namelist() if name.endswith('/sim/hunter/ammo_auto_gen.go'))
            text=z.read(name).decode()
        found={name:(int(item),int(haste)) for name,item,haste in re.findall(r'"(\w+)":\s*\{Name:\s*"[^"]+",\s*ItemID:\s*(\d+),\s*Haste:\s*(\d+)\}',text)}
        self.assertEqual(found,{key:value for key,value in adapter.HUNTER_QUIVERS.items() if key!='QuiverNone'})
        engine=os.environ.get('AZEROTH_TEST_REAL_ENGINE');public=os.environ.get('AZEROTH_TEST_PUBLIC_TEMPLATE')
        if not engine or not public:self.skipTest('Explicit qualified engine/template required for real run')
        template=json.loads(Path(public).read_text());player=template['raid']['parties'][0]['players'][0]
        _,review,encounter=inputs();review['checks']['simulator_equipment_spec']=deepcopy(player['equipment'])
        review['captured_equipped_bag_items']=[{'bag':1,'item_id':8217,'inventory_slot':20}]
        encounter['simulation_input']['simOptions']={'iterations':200,'randomSeed':'20261109'}
        runs=[]
        for selection in ['QuiverNone','QuickdrawQuiver']:
            player['hunter']['options']['classOptions']['quiverBonus']=selection
            request=adapter.request_from(template,review,encounter)
            runs.append(adapter.run(Path(engine),adapter.layout.ENGINE_SHA256,request))
        self.assertGreater(runs[1]['mean_dps'],runs[0]['mean_dps'])


if __name__=='__main__':unittest.main()
