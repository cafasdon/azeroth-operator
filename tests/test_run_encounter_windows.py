"""Shared budgets and complete-only weighted saved-window encounter comparisons."""
import asyncio
from contextlib import redirect_stdout, redirect_stderr
from copy import deepcopy
import hashlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.journal_forever_gear_windows import append_report
from tools.rank_forever_slot import collect, plan_report
from tools import run_forever_encounter_windows as runner


class InheritedEncounterDeadlineTests(unittest.TestCase):
    def test_inherited_deadline_validation_and_clamping(self):
        args = SimpleNamespace(encounter_lookup=[Path('encounter.json')], encounter_weight=None,
            max_new_runs=1, iteration_budget=3, time_budget_seconds=1)
        observed = []
        async def saved(options, *, deadline=None):
            observed.append(deadline)
            raise RuntimeError('captured inherited deadline')
        with patch.object(runner, 'monotonic', return_value=1.0), patch.object(runner.saved, 'execute', saved):
            for invalid in [1.0, 0, True, float('nan'), float('inf'), float('-inf'), '2']:
                with self.assertRaises(ValueError): asyncio.run(runner.execute(args, deadline=invalid))
            self.assertEqual(observed, [])
            for inherited in [1.2, 10.0]:
                with self.assertRaisesRegex(RuntimeError, 'captured inherited deadline'):
                    asyncio.run(runner.execute(args, deadline=inherited))
        self.assertEqual(observed, [1.2, 2.0])


class RunEncounterWindowTests(unittest.TestCase):
    def test_partial_encounter_leaders_survive_aggregate_top_limit_for_every_role(self):
        for metric,field in runner.roles.FIELDS.items():
            baseline = .5 if metric == 'death' else 100
            scale = .005 if metric == 'death' else 1
            direction = -1 if metric in ('dtps','death','oom') else 1
            profiles = []
            for gains in [(0,20,10),(0,-10,30)]:
                rows = []
                for review,gain in enumerate(gains,1):
                    value = baseline + direction*scale*gain
                    rows.append({'review':review,field:value,'mean_dps_delta_from_baseline':value-baseline})
                profiles.append({'results':rows})
            before = deepcopy(profiles)
            gear={i:str(i)*64 for i in range(1,5)}
            report=runner.completed_candidate_ranking(profiles,[.25,.75],metric,4,gear,
                top=1,include_encounter_leaders=True)
            self.assertEqual([row['review'] for row in report['results']],[3])
            self.assertTrue(report['results_truncated'])
            self.assertEqual([row['review'] for row in report['encounter_leader_candidates']],[2,3])
            self.assertEqual(report['omitted_candidate_count'],1)
            self.assertFalse(report['requested_results_complete'])
            filtered=runner.completed_candidate_ranking(profiles,[.25,.75],metric,4,gear,
                top=1,minimum_improvement=0,include_encounter_leaders=True)
            self.assertEqual([row['review'] for row in filtered['encounter_leader_candidates']],[3])
            self.assertEqual(profiles,before)

    def test_shared_finalization_rejects_late_weighted_result(self):
        args = SimpleNamespace(encounter_lookup=['fixture'], encounter_weight=[1], max_new_runs=1,
            iteration_budget=3, time_budget_seconds=1, checkpoint_only=False)
        clock = {'time': 0.0}
        async def completed(options, *, deadline):
            clock['time'] = 0.25
            return {'fingerprints': {field: 'a' * 64 for field in runner.FINGERPRINTS},
                'engine_sha256': 'b' * 64, 'simulation_profile_sha256': 'c' * 64,
                'search_coverage': {'snapshot_id': 'd' * 64}, 'requested_request_count': 2,
                'new_runs': 0, 'new_iterations': 0, 'checkpoint_hits': 2, 'requested_results_complete': True,
                'results': [{'review': 1, 'simulator_gear_sha256': 'e' * 64,
                    'mean_dps': 100, 'mean_dps_delta_from_baseline': 0},
                    {'review': 2, 'simulator_gear_sha256': 'f' * 64,
                    'mean_dps': 110, 'mean_dps_delta_from_baseline': 10}]}
        actual = runner.weighted_ranking
        def delayed(*values):
            result = actual(*values)
            clock['time'] += 6
            return result
        with patch.object(runner.saved, 'execute', completed), \
             patch.object(runner, 'monotonic', side_effect=lambda: clock['time']), \
             patch.object(runner, 'weighted_ranking', delayed):
            with self.assertRaises(ValueError): asyncio.run(runner.execute(args))

    def test_real_mcp_resume_shared_budget_weights_reversal_and_duplicate_profile(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file(): self.skipTest('Requires pinned local catalogue; no download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryFile(mode='w+') as errlog:
            base = Path(folder)
            saved = fixture()
            saved['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            saved['sections']['stats'] = {'level': 60, 'class': 'HUNTER', 'race': 'Orc', 'client_version': 'x'}
            saved['sections']['bags'] = {'0': {'1': {'item_id': 4549, 'count': 1, 'link': 'item:4549:0:0:0:0:0:0'}}}
            key = SnapshotStore(base).ingest(saved)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            with patch.dict(os.environ, env):
                drafts, skipped, coverage, _, _, _, search = asyncio.run(collect(binary, key, ['finger1'], errlog,
                    combination_offset=0, combination_limit=4))
            page = plan_report(drafts, True, True)
            page.update(gear_search=search, skipped=skipped, storage_coverage=coverage, best_in_slot_verified=False)
            journal = base / 'PRIVATE_gears.sqlite'
            append_report(journal, page)
            before = journal.read_bytes()
            template, _, encounter = inputs()
            encounter['simulation_input']['simOptions']['iterations'] = 3
            second = deepcopy(encounter)
            second['simulation_input']['encounter']['duration'] = 90
            for name, value in [('template', template), ('first', encounter), ('second', second)]:
                (base / f'{name}.json').write_text(json.dumps(value))
            engine = base / 'PRIVATE_engine.exe'
            engine.write_bytes(b'synthetic engine')
            sha = hashlib.sha256(engine.read_bytes()).hexdigest()
            checkpoints = base / 'PRIVATE_checkpoints'
            argv = ['--mcp', str(binary), '--journal', str(journal), '--window-offset', '0',
                '--template', str(base / 'template.json'), '--engine', str(engine), '--engine-sha256', sha,
                '--simulation-journal', str(checkpoints), '--paired', '--max-new-runs', '1',
                '--encounter-lookup', str(base / 'first.json'), '--encounter-lookup', str(base / 'second.json'),
                '--encounter-lookup', str(base / 'first.json'), '--encounter-weight', '1',
                '--encounter-weight', '3', '--encounter-weight', '1']
            def launch(extra=(), error=False):
                out, err = io.StringIO(), io.StringIO()
                with patch.dict(os.environ, env), redirect_stdout(out), redirect_stderr(err):
                    if error:
                        with self.assertRaises(SystemExit) as stopped: runner.main([*argv, *extra])
                        self.assertEqual(stopped.exception.code, 1)
                        self.assertEqual(out.getvalue(), '')
                    else: runner.main([*argv, *extra])
                self.assertNotIn('PRIVATE', out.getvalue() + err.getvalue())
                self.assertEqual(journal.read_bytes(), before)
                return None if error else json.loads(out.getvalue())
            def engine_stub(command, **kwargs):
                request = json.loads(Path(command[command.index('--infile') + 1]).read_text())
                ring = any(row.get('id') == 4549 for row in request['raid']['parties'][0]['players'][0]['equipment']['items'])
                mean = (110 if ring else 100) if request['encounter']['duration'] == 45 else (180 if ring else 200)
                Path(command[command.index('--outfile') + 1]).write_text(json.dumps({
                    'iterationsDone': 3, 'raidMetrics': {'dps': {'avg': mean, 'stdev': 1,
                        'allValues': [mean - 1, mean, mean + 1]}}}))
            with patch('tools.run_forever_sim.subprocess.run', side_effect=engine_stub) as execute:
                for invocation in range(4):
                    result = launch()
                    self.assertEqual(result['new_runs'], 1)
                    self.assertEqual(result['new_iterations'], 3)
                    if invocation < 3:
                        self.assertFalse(result['requested_results_complete'])
                        self.assertIsNone(result['weighted_ranking'])
                        partial = result['completed_candidate_ranking']
                        if invocation == 2:
                            self.assertIsNotNone(partial)
                            self.assertEqual(partial['completed_candidate_count'],1)
                            self.assertEqual(partial['omitted_candidate_count'],1)
                            self.assertEqual([row['review'] for row in partial['results']],[1])
                        else:
                            self.assertIsNone(partial)
                self.assertTrue(result['requested_results_complete'])
                self.assertEqual(execute.call_count, 4)
                weighted = result['weighted_ranking']['results']
                self.assertEqual([row['review'] for row in weighted], [1, 2])
                self.assertAlmostEqual(weighted[1]['weighted_dps_gain_pct'], -2)
                sensitivity = result['encounter_sensitivity']['results'][1]
                self.assertTrue(sensitivity['changes_sign_in_observed_encounters'])
                self.assertEqual(sensitivity['gain_pct_by_encounter'], [10, -10, 10])
                again = launch()
                self.assertEqual(again['new_runs'], 0)
                self.assertEqual(again['checkpoint_hits'], 6)
                self.assertFalse(again['simulation_performed'])
                self.assertEqual(execute.call_count, 4)
                limited = launch(['--simulation-journal', str(base / 'iteration_budget'), '--max-new-runs', '128', '--iteration-budget', '3'])
                self.assertEqual((limited['new_runs'], limited['new_iterations']), (1, 3))
                self.assertIsNone(limited['weighted_ranking'])
                observed = launch(['--simulation-journal', str(base / 'checkpoint_only'), '--checkpoint-only'])
                self.assertEqual(observed['new_runs'], 0)
                self.assertIsNone(observed['weighted_ranking'])
                self.assertEqual(execute.call_count, 5)
                launch(['--encounter-weight', '0'], error=True)
            for field in ['game_execution', 'best_in_slot_verified', 'feasible_set_coverage_verified']:
                self.assertFalse(result[field])

    def test_weight_normalisation_bounds(self):
        self.assertEqual(runner.normalise_weights(None, 2), [0.5, 0.5])
        self.assertEqual(runner.normalise_weights([1e308, 1e308], 2), [0.5, 0.5])
        for weights in [[0, 1], [-1, 1], [float('nan'), 1], [float('inf'), 1], [True, 1],
                        [1e-300, 1e300], [1]]:
            with self.assertRaises(ValueError): runner.normalise_weights(weights, 2)
