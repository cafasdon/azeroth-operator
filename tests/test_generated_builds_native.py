"""Joint native execution of independently validated gear and talent references."""
from copy import deepcopy
import json
import os
from pathlib import Path
import unittest
from tests.benchmark_joint_build_optimizer import fixture
from tools import gear_builds, talent_builds, joint_build_optimizer as joint

ROOT=Path(__file__).resolve().parents[1]


class GeneratedBuildIntegrationTests(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('AZEROTH_POSITIONAL_ARTIFACTS'),'Explicit local native artifacts needed')
    def test_generated_gear_and_talents_execute_together(self):
        artifacts=Path(os.environ['AZEROTH_POSITIONAL_ARTIFACTS'])
        profile=json.loads((ROOT/'tests/fixtures/forever-hunter-bm.json').read_bytes())['player']
        equipment=deepcopy(profile['equipment']); equipment['preserveSlots']=True
        inventory=[dict(item=item,count=1) for item in equipment['items'] if item]
        context=dict(class_id=3,specialization='hunter',race=profile['race'],level=60,
                     professions=[profile['profession1'],profile['profession2']])
        gear=gear_builds.generate_gear_builds(context,equipment,inventory,
            [dict(slot=13,items=[equipment['items'][13],{}])],catalogue_path=ROOT/'local-data/forever/catalogue.json',
            probe_path=artifacts/'equipment-probe.exe')
        self.assertEqual(gear['accepted_count'],2)
        self.assertEqual(gear['unresolved_count'],0)
        talent=talent_builds.generate_talent_builds('hunter',60,profile['talentsString'],
            [dict(definition_id=134422,ranks=[4,5]),dict(definition_id=134438,ranks=[3,4])])
        self.assertEqual(talent['accepted_count'],2)
        manifest=fixture(20261115,2)
        manifest.update(axes=[gear['axis'],talent['axis']],references=gear['references']|talent['references'])
        manifest['case']['constraints_sha256']=joint.planner.digest(dict(gear_search=gear['search_id'],talent_search=talent['search_id']))
        result=joint.evaluate_joint_build_case(manifest,artifacts/'wowsimcli-positional.exe')
        self.assertEqual(result['evaluated_coordinates'],4)
        self.assertEqual(result['unique_requests'],4)
        self.assertTrue(all(row['mean_dps']>0 for row in result['results']))
        self.assertFalse(result['global_optimum_verified'])
        self.assertFalse(result['candidate_legality_verified'])


if __name__=='__main__':
    unittest.main()
