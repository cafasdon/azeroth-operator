"""Opt-in actual pinned simulator qualification; synthetic owner data, no game client."""
import asyncio
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools import evaluate_forever_owned_gear as workflow


class RealOwnedWorkflowTests(unittest.TestCase):
    def test_pinned_engine_all_slot_cold_and_checkpoint_only(self):
        if not all(os.environ.get(k) for k in ['AZEROTH_QUALIFY_ENGINE','AZEROTH_QUALIFY_TEMPLATE']):
            self.skipTest('Opt-in local pinned engine/template required; no downloads')
        root=Path(__file__).resolve().parents[1]
        engine=Path(os.environ['AZEROTH_QUALIFY_ENGINE']).resolve(strict=True)
        template=Path(os.environ['AZEROTH_QUALIFY_TEMPLATE']).resolve(strict=True)
        engine_sha='0e195f35c247d78f05c7d87c2c017e7052d7ca532500a84ae19faba133ae99a5'
        self.assertEqual(hashlib.sha256(engine.read_bytes()).hexdigest(),engine_sha)
        self.assertEqual(hashlib.sha256(template.read_bytes()).hexdigest(),
            '761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0')
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        catalogue=(root/'local-data/forever/catalogue.json').resolve(strict=True)
        with tempfile.TemporaryDirectory(prefix='real-owned-workflow-') as folder:
            base=Path(folder); value=fixture()
            value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['stats']={'class':'HUNTER','race':'Orc','level':60,'client_version':'synthetic'}
            value['sections']['professions']={str(i):{'name':'Synthetic','rank':300,'max_rank':300,'skill_line':skill}
                for i,skill in enumerate([202,164],1)}
            value['sections']['pet_care']={'present':True,'level':60}
            value['sections']['equipment']={str(slot):{'item_id':item,'link':f'item:{item}:0:0:0:0:0:0'}
                for slot,item in [(16,2042),(18,22347)]}
            value['sections']['bags']={'0':{'1':{'item_id':4549,'count':1,'link':'item:4549:0:0:0:0:0:0'}}}
            key=SnapshotStore(base).ingest(value)
            _,_,encounter=inputs()
            encounter['simulation_input']['simOptions']={'iterations':1000,'randomSeed':'20261109'}
            first=base/'first.json'; first.write_text(json.dumps(encounter))
            encounter['simulation_input']['encounter']['duration']=90
            second=base/'second.json'; second.write_text(json.dumps(encounter))
            journal=base/'plans.sqlite'
            argv=['--mcp',str(binary),'--snapshot-id',key,'--template',str(template),'--journal',str(journal),
                '--engine',str(engine),'--engine-sha256',engine_sha,'--simulation-journal',str(base/'results'),
                '--encounter-lookup',str(first),'--encounter-lookup',str(second),
                '--encounter-weight','1','--encounter-weight','3','--max-windows','64','--max-new-runs','16']
            env={'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue)}
            snapshot=(base/f'{key}.json').read_bytes()
            with patch.dict(os.environ,env):
                cold=asyncio.run(workflow.execute(workflow.parser().parse_args(argv)))
                plans=journal.read_bytes()
                warm=asyncio.run(workflow.execute(workflow.parser().parse_args([*argv,'--checkpoint-only'])))
            self.assertEqual(len(cold['requested_slots']),17)
            self.assertTrue(cold['reported_coordinate_coverage_complete'])
            self.assertTrue(cold['all_saved_resolved_results_complete'])
            self.assertGreater(cold['simulation']['new_runs'],0)
            self.assertLessEqual(cold['simulation']['new_runs'],16)
            self.assertFalse(cold['reported_space_results_complete'])
            self.assertEqual(cold['status'],'unresolved_plans')
            self.assertEqual(warm['collection']['windows_attempted'],0)
            self.assertEqual(warm['simulation']['new_runs'],0)
            self.assertEqual(warm['simulation']['ranked_gear_sets'],cold['simulation']['ranked_gear_sets'])
            self.assertEqual(journal.read_bytes(),plans)
            self.assertEqual((base/f'{key}.json').read_bytes(),snapshot)
            self.assertFalse(cold['best_in_slot_verified']); self.assertFalse(cold['game_execution'])
            print(json.dumps({'qualification':'actual_offline_owned_workflow','slots':len(cold['requested_slots']),
                'coordinates':cold['collection']['progress']['total_combinations_decimal'],
                'unresolved':cold['collection']['progress']['resolution_accounting']['unresolved_proposed_coordinates'],
                'cold_runs':cold['simulation']['new_runs'],'warm_runs':warm['simulation']['new_runs'],
                'top_weighted_gain_pct':cold['simulation']['ranked_gear_sets'][0]['weighted_dps_gain_pct']}))


if __name__=='__main__': unittest.main()
