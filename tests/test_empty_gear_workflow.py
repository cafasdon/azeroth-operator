"""Removal choices survive collection, saved replay and numeric checkpoints."""
import asyncio
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools import evaluate_forever_owned_gear as workflow
from tools import positional_equipment as layout
from tools import rank_forever_slot as ranker


class EmptyGearWorkflowTests(unittest.TestCase):
    def test_optin_guard_precedes_mcp_launch(self):
        for value, positional in [(True,False),(1,True),(None,True)]:
            with self.assertRaises(ValueError):
                asyncio.run(ranker.collect(Path('must-not-launch'), 'a'*64,['finger1'],None,
                    include_empty_slots=value,positional_equipment=positional))

    def test_actual_mcp_saved_empty_choices_and_checkpoint_reuse(self):
        raw_engine = os.environ.get('AZEROTH_POSITIONAL_ENGINE')
        if not raw_engine: self.skipTest('Explicit qualified local engine required; no downloads')
        engine = Path(raw_engine).resolve(strict=True)
        root = Path(__file__).resolve().parents[1]
        catalogue = root/'local-data/forever/catalogue.json'
        if not catalogue.is_file(): self.skipTest('Pinned local catalogue required')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            value = fixture()
            value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['stats'] = {'class':'HUNTER','race':'Orc','level':60,'client_version':'synthetic'}
            value['sections']['bags'] = {}
            value['sections']['equipment'] = {str(slot):{'item_id':item,'link':f'item:{item}:0:0:0:0:0:0'}
                for slot,item in [(11,4549),(12,4550)]}
            key = SnapshotStore(base).ingest(value)
            snapshot = (base/f'{key}.json').read_bytes()
            template,_,encounter = inputs()
            encounter['simulation_input']['simOptions']['iterations'] = 3
            for name,data in [('template',template),('encounter',encounter)]:
                (base/f'{name}.json').write_text(json.dumps(data))
            argv = ['--mcp',str(binary),'--snapshot-id',key,'--template',str(base/'template.json'),
                '--engine',str(engine),'--engine-sha256',layout.ENGINE_SHA256,
                '--journal',str(base/'plans.sqlite'),'--simulation-journal',str(base/'results'),
                '--encounter-lookup',str(base/'encounter.json'),'--positional-equipment','--include-empty-slots',
                '--window-size','4','--max-windows','10','--max-new-runs','128','--top','20']
            actual_run = subprocess.run
            simulations = []
            def child(command,**kwargs):
                if command[1] != 'sim': return actual_run(command,**kwargs)
                request = json.loads(Path(command[command.index('--infile')+1]).read_text())
                equipment = request['raid']['parties'][0]['players'][0]['equipment']
                self.assertIs(equipment['preserveSlots'],True)
                simulations.append(equipment)
                dps = 200 - sum(bool(item) for item in equipment['items'])
                Path(command[command.index('--outfile')+1]).write_text(json.dumps({
                    'iterationsDone':3,'raidMetrics':{'dps':{'avg':dps,'stdev':0}}}))
            env = {'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            with patch.dict(os.environ,env),patch('tools.run_forever_sim.subprocess.run',side_effect=child):
                partial = asyncio.run(workflow.execute(workflow.parser().parse_args([*argv,'--max-windows','1'])))
                self.assertFalse(partial['reported_space_results_complete'])
                self.assertFalse(partial['reported_coordinate_coverage_complete'])
                cold = asyncio.run(workflow.execute(workflow.parser().parse_args(argv)))
                self.assertTrue(cold['reported_space_results_complete'])
                self.assertEqual(cold['collection']['progress']['total_combinations_decimal'],'9')
                self.assertEqual(cold['collection']['progress']['resolution_accounting']['unresolved_proposed_coordinates'],0)
                self.assertEqual(len(simulations),7)
                winner = cold['simulation']['ranked_gear_sets'][0]['saved_loadout']
                self.assertEqual(winner['selections'],[])
                self.assertEqual(winner['explicit_clear_equipment_slots'],[11,12])
                self.assertEqual(winner['cleared_equipment_slots'],[11,12])
                self.assertTrue(all(item == {} for item in winner['simulator_equipment_spec']['items']))
                journal_bytes = (base/'plans.sqlite').read_bytes()
                warm = asyncio.run(workflow.execute(workflow.parser().parse_args([*argv,'--checkpoint-only'])))
                self.assertEqual(warm['simulation']['new_runs'],0)
                self.assertEqual(warm['simulation']['ranked_gear_sets'],cold['simulation']['ranked_gear_sets'])
                self.assertEqual(len(simulations),7)
                with self.assertRaises(Exception):
                    asyncio.run(workflow.execute(workflow.parser().parse_args([arg for arg in argv if arg!='--include-empty-slots'])))
                self.assertEqual((base/'plans.sqlite').read_bytes(),journal_bytes)
                self.assertEqual((base/f'{key}.json').read_bytes(),snapshot)
                self.assertFalse(cold['best_in_slot_verified'])
                self.assertFalse(cold['game_execution'])


if __name__ == '__main__':
    unittest.main()
