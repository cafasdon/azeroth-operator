"""All saved workflow layers retain explicit role objectives and private checkpoints."""
import asyncio
from contextlib import redirect_stdout, redirect_stderr
import hashlib
import io
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools import evaluate_forever_owned_gear as owned
from tools import run_forever_saved_journal as journal
from tools import run_forever_journal_page as page
from tools import run_forever_encounter_windows as encounters
from tools import role_metrics as roles


class RoleGearWorkflowTests(unittest.TestCase):
    def test_bad_floor_rejects_before_other_owned_inputs(self):
        for execute in [owned.execute,journal.execute,encounters.execute]:
            for bad in [True,False,'PRIVATE',[],{},float('nan'),float('inf'),10**400]:
                with self.subTest(execute=execute.__module__),self.assertRaises(ValueError):
                    asyncio.run(execute(SimpleNamespace(minimum_encounter_improvement=bad)))

    def test_bad_metric_rejects_at_every_entry_before_other_inputs(self):
        for execute in [owned.execute,journal.execute,page.execute,encounters.execute]:
            for bad in [None,True,1,[],{},'PRIVATE_UNKNOWN']:
                with self.subTest(execute=execute.__module__,bad=bad),self.assertRaises(ValueError):
                    asyncio.run(execute(SimpleNamespace(rank_by=bad,ranking_objective='weighted_gain')))

    def test_actual_mcp_all_owned_roles_direction_zero_dps_partial_and_family_cache(self):
        root=Path(__file__).resolve().parents[1]
        catalogue=root/'local-data/forever/catalogue.json'
        if not catalogue.is_file(): self.skipTest('Pinned local catalogue required; no downloads')
        binary=(root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        value=fixture(); value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['stats']={'level':60,'class':'HUNTER','race':'Orc','client_version':'synthetic'}
        value['sections']['equipment']={str(slot):{'item_id':4549,'link':'item:4549:0:0:0:0:0:0'} for slot in [11,12]}
        value['sections']['bags']={'0':{'1':{'item_id':4550,'count':1,'link':'item:4550:0:0:0:0:0:0'}}}
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); key=SnapshotStore(base).ingest(value)
            snapshot=(base/f'{key}.json').read_bytes()
            template,_,encounter=inputs(); template['raid']['tanks']=[{'type':1,'index':0}]
            encounter['simulation_input']['simOptions']={'iterations':3,'randomSeed':'20261109'}
            first=base/'first.json'; first.write_text(json.dumps(encounter))
            encounter['simulation_input']['encounter']['duration']=90
            second=base/'second.json'; second.write_text(json.dumps(encounter))
            source=base/'template.json'; source.write_text(json.dumps(template))
            engine=base/'engine.exe'; engine.write_bytes(b'synthetic metric engine boundary')
            argv=['--mcp',str(binary),'--snapshot-id',key,'--template',str(source),
                '--journal',str(base/'plans.sqlite'),'--engine',str(engine),'--engine-sha256',hashlib.sha256(engine.read_bytes()).hexdigest(),
                '--simulation-journal',str(base/'results'),'--encounter-lookup',str(first),'--encounter-lookup',str(second),
                '--encounter-weight','9','--encounter-weight','1','--max-windows','64']
            env={'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            def launch(metric,extra=()):
                output,error=io.StringIO(),io.StringIO()
                with patch.dict(os.environ,env),redirect_stdout(output),redirect_stderr(error):
                    owned.main([*argv,'--rank-by',metric,*extra])
                self.assertNotIn('PRIVATE',output.getvalue()+error.getvalue())
                self.assertEqual((base/f'{key}.json').read_bytes(),snapshot)
                return json.loads(output.getvalue())
            def engine_stub(command,**kwargs):
                request=json.loads(Path(command[command.index('--infile')+1]).read_text())
                first=request['encounter']['duration']==45
                candidate=any(item.get('id')==4550 for item in request['raid']['parties'][0]['players'][0]['equipment']['items'])
                distribution=lambda number:{'avg':number,'stdev':0}
                player={'dps':distribution(0),
                    'dtps':distribution((8 if first else 11) if candidate else 10),
                    'threat':distribution((25 if first else 24) if candidate else 20),
                    'chanceOfDeath':(.2 if first else 0) if candidate else 0,
                    'hps':distribution((30 if first else 10) if candidate else 0),
                    'secondsOomAvg':(4 if first else 0) if candidate else 0}
                Path(command[command.index('--outfile')+1]).write_text(json.dumps({'iterationsDone':3,
                    'raidMetrics':{'dps':distribution(0),'parties':[{'players':[player]}]}}))
            with patch('tools.run_forever_sim.subprocess.run',side_effect=engine_stub) as child:
                partial=launch('dtps',['--max-new-runs','1'])
                self.assertFalse(partial['reported_space_results_complete'])
                self.assertEqual(partial['simulation']['new_runs'],1)
                for metric,expected_new,wants_candidate,expected_score in [
                    ('dtps',5,True,1.7),('tps',0,True,4.9),('death',0,False,0),
                    ('hps',6,True,28),('oom',0,False,0)]:
                    with self.subTest(metric=metric):
                        result=launch(metric); ranked=result['simulation']
                        self.assertTrue(result['reported_space_results_complete'])
                        self.assertEqual(ranked['rank_by'],metric)
                        self.assertEqual(ranked['ranking_score_unit'],roles.UNITS[metric])
                        self.assertEqual(ranked['new_runs'],expected_new)
                        winner=ranked['ranked_gear_sets'][0]
                        self.assertEqual(bool(winner['saved_loadout']['selections']),wants_candidate)
                        self.assertAlmostEqual(winner['ranking_score'],expected_score)
                        self.assertNotIn('weighted_dps_gain_pct',winner)
                        warm=launch(metric,['--checkpoint-only'])
                        self.assertEqual(warm['simulation']['new_runs'],0)
                        self.assertEqual(ranked['ranked_gear_sets'],warm['simulation']['ranked_gear_sets'])
                        robust=launch(metric,['--checkpoint-only','--ranking-objective','worst_encounter_gain'])
                        constrained=launch(metric,['--checkpoint-only','--minimum-encounter-improvement','0'])
                        self.assertEqual(constrained['minimum_encounter_improvement'],0)
                        self.assertEqual(constrained['simulation']['new_runs'],0)
                        self.assertTrue(all(row['minimum_improvement']>=0 for row in constrained['simulation']['ranked_gear_sets']))
                        impossible=launch(metric,['--checkpoint-only','--minimum-encounter-improvement','10000'])
                        self.assertEqual(impossible['simulation']['ranked_gear_sets'],[])
                        self.assertTrue(impossible['reported_space_results_complete'])
                        self.assertEqual(robust['simulation']['new_runs'],0)
                        self.assertEqual(bool(robust['simulation']['ranked_gear_sets'][0]['saved_loadout']['selections']),
                            metric in ['tps','hps'])
                        self.assertFalse(result['best_in_slot_verified']); self.assertFalse(result['game_execution'])
                self.assertEqual(child.call_count,12)


if __name__=='__main__': unittest.main()
