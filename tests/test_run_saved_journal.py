"""Rank across stable saved pages; incomplete work never becomes full-journal proof."""
import asyncio
from contextlib import redirect_stdout, redirect_stderr, closing
from copy import deepcopy
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.journal_forever_gear_windows import append_report, journal_status
from tools.rank_forever_slot import collect, plan_report
from tools import run_forever_saved_journal as runner
from tools import journal_forever_gear_windows as journal_tools


class SavedJournalTests(unittest.TestCase):
    def test_real_module_empty_journal_and_invalid_options(self):
        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            journal = base/'PRIVATE_empty.sqlite'
            with closing(journal_tools.connect(journal,write=True)[0]) as connection:
                connection.commit()
            before = journal.read_bytes()
            args = [sys.executable,'-m','tools.run_forever_saved_journal','--journal',str(journal),
                '--mcp',str(base/'must-not-launch.exe'),'--engine',str(base/'must-not-launch.exe'),
                '--engine-sha256','a'*64,'--template',str(base/'missing.json'),
                '--encounter-lookup',str(base/'missing.json'),'--simulation-journal',str(base/'must-not-create')]
            proc = subprocess.run(args,cwd=root,capture_output=True,text=True,timeout=20)
            self.assertEqual(proc.returncode,0,proc.stderr)
            result = json.loads(proc.stdout)
            self.assertEqual(result['stop_reason'],'no_saved_work')
            self.assertFalse(result['all_saved_resolved_results_complete'])
            self.assertFalse(result['simulation_performed'])
            self.assertEqual(result['ranked_gear_sets'],[])
            self.assertFalse((base/'must-not-create').exists())
            self.assertEqual(journal.read_bytes(),before)
            for option,value in [('--top','0'),('--max-pages','0'),('--page-limit','65'),('--iteration-budget','0')]:
                invalid = subprocess.run([*args,option,value],cwd=root,capture_output=True,text=True,timeout=20)
                self.assertEqual(invalid.returncode,1)
                self.assertEqual(invalid.stdout,'')
                self.assertNotIn('PRIVATE',invalid.stderr)

    def test_real_mcp_multi_page_shared_budgets_weighted_ranking_and_revalidation(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = root / 'local-data/forever/catalogue.json'
        if not catalogue.is_file(): self.skipTest('Requires local pinned catalogue; no download')
        binary = (root / 'target/debug/azeroth-operator.exe').resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryFile(mode='w+') as errlog:
            base = Path(folder)
            value = fixture()
            value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['stats'] = {'level':60,'class':'HUNTER','race':'Orc','client_version':'x'}
            value['sections']['bags'] = {'0': {str(i): {'item_id':item,'count':1,
                'link':f'item:{item}:0:0:0:0:0:0'} for i,item in enumerate([4549,4550],1)}}
            key = SnapshotStore(base).ingest(value)
            env = {**os.environ,'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            journal = base / 'PRIVATE_gears.sqlite'
            for offset in range(3):
                with patch.dict(os.environ,env):
                    drafts,skipped,coverage,_,_,_,search = asyncio.run(collect(binary,key,['finger1'],errlog,
                        combination_offset=offset,combination_limit=1))
                page = plan_report(drafts,True,True)
                page.update(gear_search=search,skipped=skipped,storage_coverage=coverage,best_in_slot_verified=False)
                append_report(journal,page)
            before = journal.read_bytes()
            template,_,encounter = inputs()
            encounter['simulation_input']['simOptions']['iterations'] = 3
            second = deepcopy(encounter)
            second['simulation_input']['encounter']['duration'] = 90
            for name,data in [('template',template),('first',encounter),('second',second)]:
                (base/f'{name}.json').write_text(json.dumps(data))
            engine = base/'PRIVATE_engine.exe'
            engine.write_bytes(b'synthetic')
            argv = ['--mcp',str(binary),'--journal',str(journal),'--template',str(base/'template.json'),
                '--encounter-lookup',str(base/'first.json'),'--encounter-lookup',str(base/'second.json'),
                '--encounter-weight','1','--encounter-weight','3','--engine',str(engine),
                '--engine-sha256',hashlib.sha256(engine.read_bytes()).hexdigest(),
                '--simulation-journal',str(base/'results'),'--page-limit','2','--max-new-runs','1']
            def launch(extra=(),fails=False):
                out,err = io.StringIO(),io.StringIO()
                with patch.dict(os.environ,env),redirect_stdout(out),redirect_stderr(err):
                    if fails:
                        with self.assertRaises(SystemExit) as stopped: runner.main([*argv,*extra])
                        self.assertEqual(stopped.exception.code,1)
                        self.assertEqual(out.getvalue(),'')
                    else: runner.main([*argv,*extra])
                self.assertNotIn('PRIVATE',out.getvalue()+err.getvalue())
                self.assertEqual(journal.read_bytes(),before)
                return None if fails else json.loads(out.getvalue())
            def engine_stub(command,**kwargs):
                request = json.loads(Path(command[command.index('--infile')+1]).read_text())
                ids = {row.get('id') for row in request['raid']['parties'][0]['players'][0]['equipment']['items']}
                mean = 100
                if 4549 in ids: mean = 110 if request['encounter']['duration'] == 45 else 130
                if 4550 in ids: mean = 120 if request['encounter']['duration'] == 45 else 90
                Path(command[command.index('--outfile')+1]).write_text(json.dumps({
                    'iterationsDone':3,'raidMetrics':{'dps':{'avg':mean,'stdev':1}}}))
            with patch('tools.run_forever_sim.subprocess.run',side_effect=engine_stub) as calls:
                for index in range(6):
                    result = launch()
                    self.assertEqual((result['new_runs'],result['new_iterations']),(1,3))
                    self.assertEqual(result['all_saved_resolved_results_complete'],index == 5)
                self.assertEqual(calls.call_count,6)
                self.assertEqual(result['ranking_scope'],'all_saved_resolved_requests')
                self.assertEqual(result['completed_window_count'],3)
                self.assertEqual(len(result['ranked_gear_sets']),3)
                for row,expected in zip(result['ranked_gear_sets'],[25,0,-2.5],strict=True):
                    self.assertAlmostEqual(row['weighted_dps_gain_pct'],expected)
                self.assertEqual([row['source_window_offset_decimal'] for row in result['ranked_gear_sets']],['1','0','2'])
                self.assertEqual(len({row['simulator_gear_sha256'] for row in result['ranked_gear_sets']}),3)
                cached = launch(['--checkpoint-only'])
                self.assertEqual((cached['new_runs'],cached['checkpoint_hits']),(0,8))
                self.assertEqual(result['ranked_gear_sets'],cached['ranked_gear_sets'])
                narrow = launch(['--checkpoint-only','--page-limit','1','--top','2'])
                self.assertEqual(narrow['ranked_gear_sets'],cached['ranked_gear_sets'][:2])
                partial = launch(['--checkpoint-only','--max-pages','1'])
                self.assertFalse(partial['all_saved_resolved_results_complete'])
                self.assertEqual(partial['ranking_scope'],'completed_pages_only')
                self.assertEqual(partial['stop_reason'],'page_budget')
                self.assertEqual(calls.call_count,6)
                # Detect valid source changes even if aggregate coverage metadata stays unchanged.
                count = {'value':0}
                def changed_status(*args,**kwargs):
                    current = journal_status(*args,**kwargs)
                    count['value'] += 1
                    if count['value'] == 2: current['journal_content_sha256'] = '0'*64
                    return current
                with patch.object(runner.journal,'journal_status',side_effect=changed_status):
                    launch(['--checkpoint-only'],fails=True)
                original = runner.pages.execute
                async def changed_profile(options,*,deadline=None):
                    if options.after_window_offset is not None:
                        different = deepcopy(template)
                        different['raid']['parties'][0]['players'][0]['name'] = 'PRIVATE_CHANGED'
                        (base/'template.json').write_text(json.dumps(different))
                    return await original(options,deadline=deadline)
                with patch.object(runner.pages,'execute',changed_profile):
                    launch(['--checkpoint-only'],fails=True)
                self.assertEqual(calls.call_count,6)
            for flag in ['global_ranking_verified','best_in_slot_verified','feasible_set_coverage_verified','game_execution']:
                self.assertFalse(result[flag])
        proc = subprocess.run([sys.executable,'-m','tools.run_forever_saved_journal','--help'],cwd=root,
            capture_output=True,text=True,timeout=20)
        self.assertEqual(proc.returncode,0,proc.stderr)


if __name__ == '__main__': unittest.main()
