"""Page saved work without mistaking a page or cursor for a global ranking."""
import asyncio
from contextlib import redirect_stdout, redirect_stderr, closing
from copy import deepcopy
import hashlib
import io
import json
import os
from pathlib import Path
import sqlite3
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
from test_merge_gear_windows import report
from tools.journal_forever_gear_windows import append_report
from tools.rank_forever_slot import collect, plan_report
from tools import run_forever_journal_page as runner


class JournalPageTests(unittest.TestCase):
    def test_selection_cannot_renew_subsecond_work_budget(self):
        selected = {'after_window_offset_decimal': None, 'selected_window_offsets_decimal': ['0'],
            'selected_window_count': 1, 'page_limit': 1, 'selection_sha256': '1'*64,
            'next_saved_window_offset_decimal': None}
        args = SimpleNamespace(time_budget_seconds=1, max_new_runs=1, iteration_budget=3,
            simulation_journal_mib=64, encounter_lookup=[Path('encounter.json')], encounter_weight=None,
            engine_sha256='1'*64, journal=Path('journal.sqlite'), after_window_offset=None, page_limit=1)
        clock = {'now': 0.0}
        def select(*args):
            clock['now'] = .8
            return deepcopy(selected)
        async def delegated(options, *, deadline=None):
            self.assertEqual(deadline, 1.0, 'Selection must not renew the original work budget')
            return {'requested_results_complete': True, 'simulation_performed': False}
        with patch.object(runner, 'monotonic', side_effect=lambda: clock['now']), \
             patch.object(runner, 'select_page', side_effect=select), \
             patch.object(runner.encounters, 'execute', delegated):
            result = asyncio.run(runner.execute(args))
        self.assertEqual(result['page_status'], 'complete')

    def test_indexed_pages_beyond_64_and_invalid_cursor_or_row(self):
        spec = {'items': [{} for _ in range(17)]}
        sha = hashlib.sha256(json.dumps(spec, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'private.sqlite'
            for offset in range(81):
                value = report(0, [sha], 1)
                value['resolved_drafts'][0]['simulator_equipment_spec'] = spec
                search = value['gear_search']
                search.update(candidate_locations_by_slot={'finger1': 8, 'finger2': 8}, combinations_including_baseline=81)
                search['combination_window'].update(offset=offset, total_combinations_decimal='81',
                    next_offset=offset + 1 if offset < 80 else None)
                append_report(path, value)
            before = path.read_bytes()
            first = runner.select_page(path, None, 64, monotonic() + 30)
            self.assertEqual(first['selected_window_offsets_decimal'], [str(i) for i in range(64)])
            self.assertEqual(first['next_saved_window_offset_decimal'], '64')
            second = runner.select_page(path, 63, 64, monotonic() + 30)
            self.assertEqual(second['selected_window_offsets_decimal'], [str(i) for i in range(64,81)])
            self.assertIsNone(second['next_saved_window_offset_decimal'])
            self.assertEqual(path.read_bytes(), before)
            for after, limit in [(-1, 1), (True, 1), (None, 0), (None, 65)]:
                with self.assertRaises(ValueError): runner.select_page(path, after, limit, monotonic() + 30)
            with closing(sqlite3.connect(path)) as connection:
                connection.execute('UPDATE windows SET report_sha=? WHERE start_key=?', ('0'*64, str(64).zfill(64)))
                connection.commit()
            with self.assertRaises(ValueError): runner.select_page(path, 63, 64, monotonic() + 30)

    def test_real_mcp_resume_cursor_next_page_and_empty_page(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = root / 'local-data/forever/catalogue.json'
        if not catalogue.is_file(): self.skipTest('Requires local pinned catalogue; no download')
        binary = (root / 'target/debug/azeroth-operator.exe').resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryFile(mode='w+') as errlog:
            base = Path(folder)
            value = fixture()
            value['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            value['sections']['stats'] = {'level': 60, 'class': 'HUNTER', 'race': 'Orc', 'client_version': 'x'}
            value['sections']['bags'] = {'0': {str(i): {'item_id': item, 'count': 1,
                'link': f'item:{item}:0:0:0:0:0:0'} for i,item in enumerate([4549,4550],1)}}
            key = SnapshotStore(base).ingest(value)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            journal = base / 'PRIVATE_gears.sqlite'
            for offset in range(3):
                with patch.dict(os.environ, env):
                    drafts, skipped, coverage, _, _, _, search = asyncio.run(collect(binary, key, ['finger1'], errlog,
                        combination_offset=offset, combination_limit=1))
                page = plan_report(drafts, True, True)
                page.update(gear_search=search, skipped=skipped, storage_coverage=coverage, best_in_slot_verified=False)
                append_report(journal, page)
            before = journal.read_bytes()
            template, _, encounter = inputs()
            encounter['simulation_input']['simOptions']['iterations'] = 3
            for name, data in [('template',template), ('encounter',encounter)]: (base / f'{name}.json').write_text(json.dumps(data))
            engine = base / 'PRIVATE_engine.exe'
            engine.write_bytes(b'synthetic')
            argv = ['--mcp',str(binary),'--journal',str(journal),'--template',str(base/'template.json'),
                '--encounter-lookup',str(base/'encounter.json'),'--engine',str(engine),
                '--engine-sha256',hashlib.sha256(engine.read_bytes()).hexdigest(),
                '--simulation-journal',str(base/'results'),'--page-limit','2','--max-new-runs','1']
            def launch(extra=()):
                out, err = io.StringIO(), io.StringIO()
                with patch.dict(os.environ, env), redirect_stdout(out), redirect_stderr(err): runner.main([*argv,*extra])
                self.assertNotIn('PRIVATE', out.getvalue()+err.getvalue())
                self.assertEqual(journal.read_bytes(), before)
                return json.loads(out.getvalue())
            def engine_stub(command, **kwargs):
                request = json.loads(Path(command[command.index('--infile')+1]).read_text())
                items = request['raid']['parties'][0]['players'][0]['equipment']['items']
                mean = 100 + sum(bool(item) for item in items)
                Path(command[command.index('--outfile')+1]).write_text(json.dumps({
                    'iterationsDone':3,'raidMetrics':{'dps':{'avg':mean,'stdev':1}}}))
            with patch('tools.run_forever_sim.subprocess.run', side_effect=engine_stub) as calls:
                partial = launch()
                self.assertEqual(partial['page_status'],'incomplete')
                self.assertTrue(partial['journal_page']['resume_same_page'])
                self.assertIsNone(partial['journal_page']['resume_after_window_offset_decimal'])
                done = launch()
                self.assertEqual(done['page_status'],'complete')
                self.assertEqual(done['journal_page']['resume_after_window_offset_decimal'],'1')
                last = launch(['--after-window-offset','1'])
                self.assertEqual(last['page_status'],'complete')
                self.assertEqual(last['journal_page']['selected_window_offsets_decimal'],['2'])
                self.assertIsNone(last['journal_page']['resume_after_window_offset_decimal'])
                self.assertEqual(calls.call_count,3)
                empty = launch(['--after-window-offset','2'])
                self.assertEqual(empty['page_status'],'no_saved_windows_after_cursor')
                self.assertIsNone(empty['encounter_comparison'])
                self.assertEqual(calls.call_count,3)
            for flag in ['global_ranking_verified','best_in_slot_verified','feasible_set_coverage_verified','game_execution']:
                self.assertFalse(last[flag])
            self.assertFalse(last['journal_page']['prior_pages_verified'])
        proc = subprocess.run([sys.executable,'-m','tools.run_forever_journal_page','--help'],cwd=root,
            capture_output=True,text=True,timeout=20)
        self.assertEqual(proc.returncode,0,proc.stderr)


if __name__ == '__main__': unittest.main()
