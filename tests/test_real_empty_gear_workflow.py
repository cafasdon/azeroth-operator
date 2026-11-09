"""Opt-in actual engine qualification for saved removal choices, never game data."""
import asyncio
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from time import monotonic
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools import collect_forever_gear_windows as collection
from tools import prepare_forever_gear_window as preparer
from tools import run_forever_sim as adapter
from tools import positional_equipment as layout


class RealEmptyGearWorkflowTests(unittest.TestCase):
    def test_real_paired_engine_saved_removals_and_uncached_reference(self):
        if not all(os.environ.get(key) for key in ['AZEROTH_POSITIONAL_ENGINE','AZEROTH_QUALIFY_TEMPLATE']):
            self.skipTest('Explicit local engine and pinned public template required; no downloads')
        engine = Path(os.environ['AZEROTH_POSITIONAL_ENGINE']).resolve(strict=True)
        template = Path(os.environ['AZEROTH_QUALIFY_TEMPLATE']).resolve(strict=True)
        self.assertEqual(hashlib.sha256(engine.read_bytes()).hexdigest(),layout.ENGINE_SHA256)
        self.assertEqual(hashlib.sha256(template.read_bytes()).hexdigest(),
            '761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0')
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        catalogue = (root/'local-data/forever/catalogue.json').resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            value = fixture()
            value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['stats'] = {'class':'HUNTER','race':'Orc','level':60,'client_version':'synthetic'}
            value['sections']['equipment'] = {str(slot):{'item_id':item,'link':f'item:{item}:0:0:0:0:0:0'}
                for slot,item in [(11,4549),(12,4550),(16,2042),(18,22347)]}
            value['sections']['bags'] = {}
            value['sections']['professions'] = {str(index):{'name':'Synthetic','rank':300,'max_rank':300,'skill_line':skill}
                for index,skill in enumerate([202,164],1)}
            value['sections']['pet_care'] = {'present':True,'level':60}
            key = SnapshotStore(base).ingest(value)
            snapshot = (base/f'{key}.json').read_bytes()
            _,_,encounter = inputs()
            encounter['simulation_input']['simOptions'] = {'iterations':200,'randomSeed':'20261109'}
            encounter_path = base/'encounter.json'
            encounter_path.write_text(json.dumps(encounter))
            journal = base/'plans.sqlite'
            args = collection.parser().parse_args(['--mcp',str(binary),'--snapshot-id',key,
                '--template',str(template),'--journal',str(journal),'--slot','finger1','--slot','finger2',
                '--retain-equipment-specs','--collapse-identical-items','--positional-equipment',
                '--include-empty-slots','--window-size','4','--max-windows','3'])
            env = {**os.environ,'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue)}
            with patch.dict(os.environ,env):
                collected = asyncio.run(collection.collect_windows(args))
                self.assertTrue(collected['progress']['reported_coordinate_coverage_complete'])
                self.assertEqual(collected['progress']['total_combinations_decimal'],'9')
                self.assertEqual(collected['progress']['resolution_accounting']['unresolved_proposed_coordinates'],0)
                before = journal.read_bytes()
                command = [sys.executable,'-m','tools.run_forever_saved_journal','--mcp',str(binary),
                    '--journal',str(journal),'--template',str(template),'--engine',str(engine),
                    '--engine-sha256',layout.ENGINE_SHA256,'--simulation-journal',str(base/'results'),
                    '--encounter-lookup',str(encounter_path),'--paired','--top','20']
                def launch(extra=()):
                    process = subprocess.run([*command,*extra],cwd=root,env=env,capture_output=True,text=True,timeout=90)
                    self.assertEqual(process.returncode,0,process.stderr)
                    self.assertNotIn(folder,process.stdout+process.stderr)
                    self.assertEqual(journal.read_bytes(),before)
                    return json.loads(process.stdout)
                cold = launch()
                self.assertTrue(cold['all_saved_resolved_results_complete'])
                self.assertEqual(cold['new_runs'],7)
                self.assertEqual(cold['new_iterations'],1400)
                self.assertEqual(len(cold['ranked_gear_sets']),7)
                self.assertEqual([row['mean_dps_by_encounter'][0] for row in cold['ranked_gear_sets']],
                    sorted((row['mean_dps_by_encounter'][0] for row in cold['ranked_gear_sets']),reverse=True))
                warm = launch(['--checkpoint-only'])
                self.assertEqual(warm['new_runs'],0)
                self.assertEqual(warm['ranked_gear_sets'],cold['ranked_gear_sets'])
                requests = {}
                baseline = None
                for offset in [0,4,8]:
                    prepared = asyncio.run(preparer.prepare(SimpleNamespace(mcp=binary,journal=journal,
                        window_offset=offset,template=template,encounter_lookup=encounter_path,time_budget_seconds=45)))
                    baseline = baseline or prepared['requests'][0]
                    for row in prepared['requests']:
                        requests.setdefault(row['simulator_gear_sha256'],row)
                self.assertEqual(len(requests),7)
                deadline = monotonic()+90
                summaries = {gear:adapter.run(engine,layout.ENGINE_SHA256,row['request'],deadline=deadline,paired=True)
                    for gear,row in requests.items()}
                baseline_summary = summaries[baseline['simulator_gear_sha256']]
                for leader in cold['ranked_gear_sets']:
                    gear = leader['simulator_gear_sha256']
                    request = requests[gear]
                    observed = leader['simulation_evidence_by_encounter'][0]
                    comparison = adapter.comparison_from_summaries([baseline['request'],request['request']],
                        [deepcopy(baseline_summary),deepcopy(summaries[gear])],paired=True)['results'][1]
                    for field in ['mean_dps','dps_standard_deviation','paired_dps_standard_error']:
                        self.assertEqual(observed[field],comparison[field])
                    self.assertEqual(observed.get('paired_dps_delta_interval_approx_95'),
                        comparison.get('paired_dps_delta_interval_approx_95'))
                    self.assertEqual(observed['request_sha256'],request['request_sha256'])
                    self.assertEqual(leader['mean_dps_by_encounter'],[summaries[gear]['mean_dps']])
                self.assertTrue(any(row['saved_loadout'].get('explicit_clear_equipment_slots') == [11,12]
                    for row in cold['ranked_gear_sets']))
                self.assertFalse(cold['best_in_slot_verified'])
                self.assertFalse(cold['game_execution'])
                self.assertEqual(journal.read_bytes(),before)
                self.assertEqual((base/f'{key}.json').read_bytes(),snapshot)
                print(json.dumps({'qualification':'real_empty_gear_saved_workflow','coordinates':9,
                    'distinct_sets':7,'cold_runs':7,'warm_runs':0,'uncached_reference_runs':7,
                    'iterations_per_run':200,'seed':'20261109','encounter_seconds':45,
                    'mean_dps_by_rank':[row['mean_dps_by_encounter'][0] for row in cold['ranked_gear_sets']],
                    'game_execution':False}))


if __name__ == '__main__':
    unittest.main()
