"""Budgeted saved-window execution with real MCP and a synthetic engine boundary."""
import asyncio
from contextlib import redirect_stdout, redirect_stderr
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
from types import SimpleNamespace

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.journal_forever_gear_windows import append_report
from tools.rank_forever_slot import collect, plan_report
from tools import run_forever_saved_windows as runner
from tools.run_forever_sim import comparison_from_summaries
from tools import run_forever_sim as adapter
from tools.simulation_journal import SimulationJournal


class RunSavedWindowTests(unittest.TestCase):
    def test_completed_checkpoint_crossing_work_deadline_returns_progress(self):
        template, review, encounter = inputs()
        request = adapter.request_from(template, review, encounter)
        request['simOptions']['iterations'] = 3
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            engine = base / 'engine.exe'
            engine.write_bytes(b'synthetic engine')
            sha = hashlib.sha256(engine.read_bytes()).hexdigest()
            jobs = [{'global_review': i, 'request': deepcopy(request), 'simulator_gear_sha256': str(i) * 64,
                     'request_sha256': str(i) * 64, 'references': []} for i in [1, 2]]
            jobs[1]['request']['raid']['parties'][0]['players'][0]['equipment']['items'][10] = {'id': 4549}
            work = {'requests': jobs, 'search_coverage': {'snapshot_id': '1' * 64, 'scope_sha256': '2' * 64}}
            args = SimpleNamespace(time_budget_seconds=1, max_new_runs=1, iteration_budget=100000,
                simulation_journal_mib=64, window_offset=[0], engine_sha256=sha, engine=engine,
                simulation_journal=base / 'journal', mcp=base / 'mcp.exe', journal=base / 'gears.sqlite',
                template=base / 'template.json', encounter_lookup=base / 'encounter.json', rank_by='dps', paired=False)
            clock = {'value': 0.0}
            async def prepared(options): return {}
            def cached(*args):
                clock['value'] = 1.001
                return {'iterations': 3, 'seed': '1', 'mean_dps': 100, 'dps_standard_deviation': 1, 'engine_sha256': sha}
            with patch.object(runner, 'monotonic', side_effect=lambda: clock['value']), \
                 patch('tools.run_forever_sim.time.monotonic', side_effect=lambda: clock['value']), \
                 patch.object(runner.preparer, 'prepare', prepared), \
                 patch.object(runner.preparer, 'file_digest', return_value=sha), \
                 patch.object(runner, 'merge_prepared', return_value=work), \
                 patch.object(SimulationJournal, 'get', side_effect=cached), \
                 patch.object(adapter, 'run', side_effect=AssertionError('No engine work after deadline')):
                result = asyncio.run(runner.execute(args))
            self.assertEqual(result['completed_request_count'], 1)
            self.assertEqual(result['uninspected_request_count'], 1)
            self.assertEqual(result['stop_reason'], 'time_budget')
            self.assertFalse(result['requested_results_complete'])

    def test_engine_change_is_rejected_before_checkpoint_commit(self):
        template, review, encounter = inputs()
        request = adapter.request_from(template, review, encounter)
        request['simOptions']['iterations'] = 3
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            engine = base / 'engine.exe'
            engine.write_bytes(b'expected engine')
            sha = hashlib.sha256(engine.read_bytes()).hexdigest()
            checkpoints = SimulationJournal(base / 'journal')
            key = checkpoints.key(sha, request, False, False, False)
            self.assertIsNone(checkpoints.get(key))
            path = checkpoints.root / 'checkpoints.sqlite3'
            before = path.read_bytes()
            def changed(command, **kwargs):
                engine.write_bytes(b'changed engine')
                Path(command[command.index('--outfile') + 1]).write_text(json.dumps({
                    'iterationsDone': 3, 'raidMetrics': {'dps': {'avg': 999, 'stdev': 1}}}))
            with patch('tools.run_forever_sim.subprocess.run', side_effect=changed):
                with self.assertRaises(ValueError): adapter.run(engine, sha, request, cache=checkpoints)
            self.assertIsNone(checkpoints.get(key))
            self.assertEqual(path.read_bytes(), before)

    def test_resume_budgets_real_mcp_guard_recheck_and_checkpoint_only_retry(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file(): self.skipTest('Requires pinned local catalogue; no download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryFile(mode='w+') as errlog:
            base = Path(folder)
            snapshot = fixture()
            snapshot['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            snapshot['sections']['stats'] = {'level': 60, 'class': 'HUNTER', 'race': 'Orc', 'client_version': 'x'}
            snapshot['sections']['bags'] = {'0': {'1': {'item_id': 4549, 'count': 1, 'link': 'item:4549:0:0:0:0:0:0'}}}
            store = SnapshotStore(base)
            key = store.ingest(snapshot)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            journal = base / 'PRIVATE_gears.sqlite'
            reports = []
            for offset in [0, 1]:
                with patch.dict(os.environ, env):
                    drafts, skipped, coverage, _, _, _, search = asyncio.run(collect(binary, key, ['finger1'], errlog,
                        combination_offset=offset, combination_limit=1))
                report = plan_report(drafts, True, True)
                report.update(gear_search=search, skipped=skipped, storage_coverage=coverage, best_in_slot_verified=False)
                reports.append(report)
                append_report(journal, report)
            before = journal.read_bytes()
            template, _, encounter = inputs()
            encounter['simulation_input']['simOptions']['iterations'] = 3
            for name, value in [('template', template), ('encounter', encounter)]:
                (base / f'{name}.json').write_text(json.dumps(value))
            engine = base / 'PRIVATE_engine.exe'
            engine.write_bytes(b'synthetic engine')
            engine_hash = hashlib.sha256(engine.read_bytes()).hexdigest()
            checkpoints = base / 'PRIVATE_checkpoints'
            args = ['--mcp', str(binary), '--journal', str(journal), '--window-offset', '0', '--window-offset', '1',
                '--template', str(base / 'template.json'), '--encounter-lookup', str(base / 'encounter.json'),
                '--engine', str(engine), '--engine-sha256', engine_hash, '--simulation-journal', str(checkpoints),
                '--paired', '--max-new-runs', '1']
            def engine_stub(command, **kwargs):
                request = json.loads(Path(command[command.index('--infile') + 1]).read_text())
                items = request['raid']['parties'][0]['players'][0]['equipment']['items']
                mean = 110 if any(item.get('id') == 4549 for item in items) else 100
                output = {'iterationsDone': 3, 'raidMetrics': {'dps': {'avg': mean, 'stdev': 2,
                    'allValues': [mean - 1, mean, mean + 1]}}}
                Path(command[command.index('--outfile') + 1]).write_text(json.dumps(output))
            def launch(extra=(), error=False):
                out, err = io.StringIO(), io.StringIO()
                with patch.dict(os.environ, env), redirect_stdout(out), redirect_stderr(err):
                    if error:
                        with self.assertRaises(SystemExit) as stopped: runner.main([*args, *extra])
                        self.assertEqual(stopped.exception.code, 1)
                        self.assertEqual(out.getvalue(), '')
                    else:
                        runner.main([*args, *extra])
                self.assertNotIn('PRIVATE', out.getvalue() + err.getvalue())
                self.assertEqual(journal.read_bytes(), before)
                return None if error else json.loads(out.getvalue())
            with patch('tools.run_forever_sim.subprocess.run', side_effect=engine_stub) as execute:
                first = launch()
                self.assertFalse(first['requested_results_complete'])
                self.assertEqual(first['new_runs'], 1)
                self.assertEqual(first['missing_reviews'], [2])
                second = launch()
                self.assertTrue(second['requested_results_complete'])
                self.assertEqual((second['new_runs'], second['checkpoint_hits']), (1, 1))
                self.assertEqual([row['review'] for row in second['results']], [2, 1])
                self.assertEqual(second['results'][0]['mean_dps_delta_from_baseline'], 10)
                self.assertEqual(second['observed_leading_dps_pair']['paired_dps_standard_error'], 0)
                third = launch()
                self.assertEqual((third['new_runs'], third['checkpoint_hits']), (0, 2))
                self.assertFalse(third['simulation_performed'])
                self.assertEqual(first['work_set_sha256'], third['work_set_sha256'])
                self.assertEqual(execute.call_count, 2)
                checkpoint_bytes = (checkpoints / 'checkpoints.sqlite3').read_bytes()
                low = deepcopy(snapshot)
                low['sections']['stats']['level'] = 20
                low_key = store.ingest(low)
                low_journal = base / 'low.sqlite'
                for report in deepcopy(reports):
                    report['gear_search']['snapshot_id'] = low_key
                    append_report(low_journal, report)
                launch(['--journal', str(low_journal)], error=True)
                self.assertEqual(execute.call_count, 2)
                self.assertEqual((checkpoints / 'checkpoints.sqlite3').read_bytes(), checkpoint_bytes)
                launch(['--iteration-budget', '0'], error=True)
            for flag in ['game_execution', 'best_in_slot_verified', 'feasible_set_coverage_verified']:
                self.assertFalse(third[flag])
        help_result = subprocess.run([sys.executable, '-m', 'tools.run_forever_saved_windows', '--help'],
            cwd=root, capture_output=True, text=True, timeout=30)
        self.assertEqual(help_result.returncode, 0, help_result.stderr)

    def test_completed_comparison_does_not_mutate_paired_checkpoints(self):
        template, review, encounter = inputs()
        from tools.run_forever_sim import request_from
        first = request_from(template, review, encounter)
        first['simOptions']['iterations'] = 3
        second = deepcopy(first)
        second['raid']['parties'][0]['players'][0]['equipment']['items'][10] = {'id': 4549}
        values = [{'iterations': 3, 'seed': '1', 'mean_dps': mean, 'dps_standard_deviation': 1,
                   'engine_sha256': 'a' * 64, '_dps_samples': [mean - 1, mean, mean + 1]} for mean in [100, 110]]
        before = deepcopy(values)
        result = comparison_from_summaries([first, second], values, paired=True)
        self.assertEqual(values, before)
        self.assertEqual(result['results'][1]['mean_dps_delta_from_baseline'], 10)
        self.assertNotIn('_dps_samples', result['results'][1])
