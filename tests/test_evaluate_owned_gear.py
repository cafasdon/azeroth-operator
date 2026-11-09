"""One all-slot workflow preserves complete-space gaps and shared offline budgets."""
import asyncio
from contextlib import redirect_stdout, redirect_stderr
from copy import copy, deepcopy
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from time import monotonic
import unittest
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools import evaluate_forever_owned_gear as workflow
from tools import journal_forever_gear_windows as plans


class OwnedGearEvaluationTests(unittest.TestCase):
    def test_collection_ceiling_hands_committed_prefix_to_simulation(self):
        root=Path(__file__).resolve().parents[1]
        catalogue=root/'local-data/forever/catalogue.json'
        if not catalogue.is_file(): self.skipTest('Pinned local cache required; no download')
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); value=fixture()
            value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['stats']={'class':'HUNTER','race':'Orc','level':60,'client_version':'synthetic'}
            value['sections']['bags']={'0':{str(i):{'item_id':item,'count':1,'link':f'item:{item}:0:0:0:0:0:0'}
                for i,item in enumerate([4549,4550,4549,4550],1)}}
            key=SnapshotStore(base).ingest(value); template,_,encounter=inputs()
            encounter['simulation_input']['simOptions']['iterations']=3
            (base/'t.json').write_text(json.dumps(template)); (base/'e.json').write_text(json.dumps(encounter))
            engine=base/'engine.exe'; engine.write_bytes(b'synthetic')
            argv=['--mcp',str(binary),'--snapshot-id',key,'--template',str(base/'t.json'),
                '--journal',str(base/'plans.sqlite'),'--engine',str(engine),'--engine-sha256',hashlib.sha256(b'synthetic').hexdigest(),
                '--simulation-journal',str(base/'results'),'--encounter-lookup',str(base/'e.json'),
                '--time-budget-seconds','20','--collection-seconds','2','--window-size','1','--max-windows','1024','--max-new-runs','1']
            def engine_stub(command,**kwargs):
                Path(command[command.index('--outfile')+1]).write_text(json.dumps({'iterationsDone':3,
                    'raidMetrics':{'dps':{'avg':100,'stdev':1}}}))
            env={'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            with patch.dict(os.environ,env),patch('tools.run_forever_sim.subprocess.run',side_effect=engine_stub) as runs:
                report=asyncio.run(workflow.execute(workflow.parser().parse_args(argv)))
                self.assertEqual(report['collection']['stop_reason'],'time_budget')
                self.assertGreater(report['collection']['progress']['window_count'],0)
                self.assertEqual(report['status'],'collection_incomplete')
                self.assertIsNotNone(report['simulation'])
                self.assertEqual(report['simulation']['new_runs'],1)
                self.assertEqual(runs.call_count,1)
                # Early source failures and explicit task cancellation must not be recovered.
                for failure in [RuntimeError('PRIVATE'),asyncio.CancelledError()]:
                    async def broken(*args,**kwargs): raise failure
                    with patch.object(workflow.collection,'collect_windows',side_effect=broken):
                        with self.assertRaises((ValueError,asyncio.CancelledError)):
                            asyncio.run(workflow.execute(workflow.parser().parse_args(argv)))
                    self.assertEqual(runs.call_count,1)

    def test_sqlite_sidecar_inputs_reject_before_collection(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); key=SnapshotStore(base).ingest(fixture())
            template,_,encounter=inputs()
            (base/'template.json').write_text(json.dumps(template)); (base/'encounter.json').write_text(json.dumps(encounter))
            journal=base/'plans.sqlite'
            for suffix in ['-journal','-wal','-shm']:
                protected=Path(str(journal)+suffix); protected.write_bytes(b'synthetic')
                argv=['--mcp',str(binary),'--snapshot-id',key,'--template',str(base/'template.json'),
                    '--journal',str(journal),'--engine',str(protected),'--engine-sha256',hashlib.sha256(b'synthetic').hexdigest(),
                    '--simulation-journal',str(base/'results'),'--encounter-lookup',str(base/'encounter.json')]
                with patch.dict(os.environ,{'AZEROTH_DATA_DIR':folder}):
                    with self.assertRaises(Exception): asyncio.run(workflow.execute(workflow.parser().parse_args(argv)))
                self.assertTrue(protected.exists(),suffix)
                self.assertEqual(protected.read_bytes(),b'synthetic')
                self.assertFalse(journal.exists())

    def test_actual_mcp_all_slots_resume_budgets_checkpoint_reuse_and_guards(self):
        root=Path(__file__).resolve().parents[1]
        catalogue=root/'local-data/forever/catalogue.json'
        if not catalogue.is_file(): self.skipTest('Pinned local cache required; no download')
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder)
            value=fixture()
            value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['stats']={'class':'HUNTER','race':'Orc','level':60,'client_version':'synthetic'}
            value['sections']['bags']={'0':{'1':{'item_id':4549,'count':1,'link':'item:4549:0:0:0:0:0:0'}}}
            key=SnapshotStore(base).ingest(value)
            template,_,encounter=inputs(); encounter['simulation_input']['simOptions']['iterations']=3
            second=deepcopy(encounter); second['simulation_input']['encounter']['duration']=90
            for name,data in [('PRIVATE_template',template),('PRIVATE_first',encounter),('PRIVATE_second',second)]:
                (base/f'{name}.json').write_text(json.dumps(data))
            engine=base/'PRIVATE_engine.exe'; engine.write_bytes(b'synthetic')
            journal=base/'PRIVATE_plans.sqlite'
            argv=['--mcp',str(binary),'--snapshot-id',key,'--template',str(base/'PRIVATE_template.json'),
                '--journal',str(journal),'--engine',str(engine),'--engine-sha256',hashlib.sha256(engine.read_bytes()).hexdigest(),
                '--simulation-journal',str(base/'PRIVATE_results'),'--encounter-lookup',str(base/'PRIVATE_first.json'),
                '--encounter-lookup',str(base/'PRIVATE_second.json'),'--encounter-weight','1','--encounter-weight','3',
                '--window-size','2','--max-windows','1','--max-new-runs','1']
            env={**os.environ,'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            protected={p:p.read_bytes() for p in [base/f'{key}.json',engine,base/'PRIVATE_template.json',
                base/'PRIVATE_first.json',base/'PRIVATE_second.json']}
            captured=[]; original_collect=workflow.collection.collect_windows; original_sim=workflow.simulation.execute
            async def collecting(options,*,deadline=None):
                captured.append(('collection',deadline,copy(options)))
                return await original_collect(options,deadline=deadline)
            async def simulating(options,*,deadline=None):
                captured.append(('simulation',deadline,copy(options)))
                return await original_sim(options,deadline=deadline)
            def launch(extra=(),fails=False):
                out,err=io.StringIO(),io.StringIO()
                with patch.dict(os.environ,env),redirect_stdout(out),redirect_stderr(err):
                    if fails:
                        with self.assertRaises(SystemExit) as stopped: workflow.main([*argv,*extra])
                        self.assertEqual(stopped.exception.code,1)
                        self.assertEqual(out.getvalue(),'')
                    else: workflow.main([*argv,*extra])
                self.assertNotIn('PRIVATE',out.getvalue()+err.getvalue())
                for path,data in protected.items(): self.assertEqual(path.read_bytes(),data)
                return None if fails else json.loads(out.getvalue())
            def engine_stub(command,**kwargs):
                request=json.loads(Path(command[command.index('--infile')+1]).read_text())
                ids={row.get('id') for row in request['raid']['parties'][0]['players'][0]['equipment']['items']}
                mean=100 if 4549 not in ids else (110 if request['encounter']['duration']==45 else 130)
                Path(command[command.index('--outfile')+1]).write_text(json.dumps({
                    'iterationsDone':3,'raidMetrics':{'dps':{'avg':mean,'stdev':1}}}))
            with patch('tools.run_forever_sim.subprocess.run',side_effect=engine_stub) as runs, \
                    patch.object(workflow.collection,'collect_windows',side_effect=collecting), \
                    patch.object(workflow.simulation,'execute',side_effect=simulating):
                reports=[]
                for _ in range(10):
                    report=launch(); reports.append(report)
                    self.assertLessEqual(report['simulation']['new_runs'],1)
                    self.assertLessEqual(report['simulation']['new_iterations'],3)
                    self.assertEqual(report['requested_slots'],list(workflow.collection.ranker.SIMULATOR_SLOTS))
                    self.assertEqual(len(report['requested_slots']),17)
                    self.assertEqual(report['candidate_filters'],{})
                    if report['reported_coordinate_coverage_complete'] and report['all_saved_resolved_results_complete']: break
                self.assertFalse(report['reported_space_results_complete'],report)
                self.assertEqual(report['status'],'unresolved_plans')
                self.assertEqual(report['collection']['progress']['total_combinations_decimal'],'4')
                self.assertEqual(report['collection']['progress']['covered_coordinate_count'],4)
                self.assertEqual(report['collection']['progress']['resolution_accounting']['unresolved_proposed_coordinates'],1)
                self.assertFalse(reports[0]['reported_coordinate_coverage_complete'])
                self.assertEqual(reports[0]['status'],'collection_incomplete')
                self.assertEqual(runs.call_count,4)
                gains=[r['weighted_dps_gain_pct'] for r in report['simulation']['ranked_gear_sets']]
                contexts = report['simulation']['encounter_contexts']
                self.assertEqual([context['encounter_settings']['duration_seconds'] for context in contexts], [45,90])
                self.assertEqual([context['baseline']['mean_dps'] for context in contexts], [100,100])
                self.assertAlmostEqual(gains[0],25); self.assertEqual(gains[1],0)
                before=journal.read_bytes()
                calls=runs.call_count
                cached=launch(['--checkpoint-only'])
                self.assertEqual(cached['collection']['windows_attempted'],0)
                self.assertEqual(cached['simulation']['new_runs'],0)
                self.assertEqual(cached['simulation']['ranked_gear_sets'],report['simulation']['ranked_gear_sets'])
                self.assertEqual(cached['simulation']['encounter_contexts'],contexts)
                self.assertEqual(runs.call_count,calls)
                self.assertEqual(journal.read_bytes(),before)
                original_argv = argv[:]
                index = argv.index('--snapshot-id')
                argv[index:index+2] = ['--character-name',value['character']['name'],
                    '--character-realm',value['character']['realm']]
                named = launch(['--checkpoint-only'])
                self.assertEqual(named['selection'],'explicit_saved_character')
                self.assertEqual(named['snapshot_id'],key)
                self.assertEqual(named['simulation']['ranked_gear_sets'],cached['simulation']['ranked_gear_sets'])
                self.assertEqual(runs.call_count,calls)
                changed_build = deepcopy(value)
                changed_build['captured_at'] = '2025-02-01T00:00:00Z'
                changed_build['client']['build'] = '70863'
                changed_build['sections'].pop('equipment')
                SnapshotStore(base).ingest(changed_build)
                exact = launch(['--checkpoint-only','--character-client','live','70862','16001'])
                self.assertEqual(exact['snapshot_id'],key)
                self.assertEqual(exact['simulation']['ranked_gear_sets'],cached['simulation']['ranked_gear_sets'])
                self.assertEqual(runs.call_count,calls)
                bounded = launch(['--checkpoint-only','--captured-from',value['captured_at'],
                                  '--captured-through',value['captured_at']])
                self.assertEqual(bounded['snapshot_id'],key)
                self.assertEqual(bounded['simulation']['ranked_gear_sets'],cached['simulation']['ranked_gear_sets'])
                self.assertEqual(runs.call_count,calls)
                strict = ['--require-unique-latest','--require-latest-section','equipment',
                          '--require-latest-section','bags']
                launch(['--checkpoint-only','--journal',str(base/'must-not-create.sqlite'),*strict],fails=True)
                self.assertFalse((base/'must-not-create.sqlite').exists())
                safe = launch(['--checkpoint-only','--character-client','live','70862','16001',*strict])
                self.assertEqual(safe['snapshot_id'],key)
                tied = deepcopy(changed_build)
                tied['client']['build'] = '70864'
                tied['sections']['equipment'] = value['sections']['equipment']
                SnapshotStore(base).ingest(tied)
                launch(['--checkpoint-only','--journal',str(base/'must-not-create.sqlite'),*strict],fails=True)
                self.assertFalse((base/'must-not-create.sqlite').exists())
                self.assertEqual(runs.call_count,calls)
                argv[:] = original_argv
                for first,second_stage in zip(captured[::2],captured[1::2],strict=True):
                    self.assertEqual((first[0],second_stage[0]),('collection','simulation'))
                    self.assertGreater(second_stage[1],first[1])
                    self.assertLessEqual(second_stage[1]-first[1],135)
                for flag in ['active_character_verified','complete_ownership_verified','global_ranking_verified',
                    'best_in_slot_verified','feasible_set_coverage_verified','game_execution']:
                    self.assertFalse(report[flag])
                # All invalid/alias inputs stop before creating a new plan journal.
                for extra in [['--max-pages','0'],['--collection-seconds','181'],['--engine-sha256','0'*64],
                    ['--encounter-weight','0'],['--journal',str(engine)],['--simulation-journal',str(base)],
                    ['--simulation-journal',str(journal)],['--captured-through',value['captured_at']],
                    ['--require-unique-latest'],['--require-latest-section','equipment']]:
                    launch(['--journal',str(base/'must-not-create.sqlite'),*extra],fails=True)
                    self.assertFalse((base/'must-not-create.sqlite').exists())
                    self.assertEqual(runs.call_count,calls)
                # Bound helpers reject expired/nonfinite/Boolean inherited deadlines.
                for invalid in [True,float('nan'),float('inf'),monotonic()-1]:
                    with self.assertRaises(ValueError): asyncio.run(original_collect(copy(captured[0][2]),deadline=invalid))
                    with self.assertRaises(ValueError): asyncio.run(original_sim(copy(captured[1][2]),deadline=invalid))
                # Fill both paired baseline slots: the all-slot search can now fully resolve.
                filled=deepcopy(value)
                filled['sections']['equipment']={str(slot):{'item_id':item,'link':f'item:{item}:0:0:0:0:0:0'}
                    for slot,item in [(11,4549),(12,4550)]}
                filled['sections']['bags']['0']['999'] = {'item_id': 4549, 'count': 1, 'link': 'item:4549'}
                filled_key=SnapshotStore(base).ingest(filled)
                protected[base/f'{filled_key}.json']=(base/f'{filled_key}.json').read_bytes()
                complete=launch(['--snapshot-id',filled_key,'--journal',str(base/'complete.sqlite'),
                    '--simulation-journal',str(base/'complete-results'),'--window-size','64','--max-windows','64',
                    '--max-new-runs','128'])
                self.assertEqual(complete['status'],'reported_space_complete')
                self.assertTrue(complete['reported_space_results_complete'])
                self.assertEqual(complete['unresolved_candidate_locations_by_slot']['finger1'], 1)
                self.assertFalse(complete['best_in_slot_verified'])
                self.assertEqual(len(complete['requested_slots']),17)
            self.assertTrue(plans.journal_status(journal)['reported_coordinate_coverage_complete'])
        proc=subprocess.run([sys.executable,'-m','tools.evaluate_forever_owned_gear','--help'],cwd=root,
            capture_output=True,text=True,timeout=20)
        self.assertEqual(proc.returncode,0,proc.stderr)
        self.assertNotIn('--slot',proc.stdout)


if __name__=='__main__': unittest.main()
