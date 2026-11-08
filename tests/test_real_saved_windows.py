"""Opt-in offline qualification: actual MCP, pinned engine, synthetic owner data."""
import asyncio
from copy import deepcopy
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import statistics
import sys
import tempfile
import unittest

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.journal_forever_gear_windows import append_report
from tools.rank_forever_slot import collect, plan_report
from tools import prepare_forever_gear_window as preparer
from tools import run_forever_sim as adapter
from types import SimpleNamespace


class RealSavedWindowQualification(unittest.TestCase):
    def test_real_engine_resume_and_weighted_profiles(self):
        names = ['AZEROTH_QUALIFY_ENGINE', 'AZEROTH_QUALIFY_TEMPLATE']
        if not all(os.environ.get(name) for name in names):
            self.skipTest('Opt-in local engine and public template required; no downloads')
        root = Path(__file__).resolve().parents[1]
        engine, template_path = [Path(os.environ[name]).resolve(strict=True) for name in names]
        expected = '0e195f35c247d78f05c7d87c2c017e7052d7ca532500a84ae19faba133ae99a5'
        self.assertEqual(hashlib.sha256(engine.read_bytes()).hexdigest(), expected)
        self.assertEqual(hashlib.sha256(template_path.read_bytes()).hexdigest(),
            '761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0')
        binary = (root / 'target/debug/azeroth-operator.exe').resolve(strict=True)
        catalogue = (root / 'local-data/forever/catalogue.json').resolve(strict=True)
        template = json.loads(template_path.read_text())
        with tempfile.TemporaryDirectory(prefix='real-saved-window-') as folder, tempfile.TemporaryFile(mode='w+') as errlog:
            base = Path(folder)
            value = fixture()
            value['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            value['sections']['stats'] = {'level': 60, 'class': 'HUNTER', 'race': 'Orc', 'client_version': 'synthetic'}
            # Deliberately synthetic capture, not the public template's gear or a native export.
            value['sections']['equipment'] = {str(slot): {'item_id': item, 'link': f'item:{item}:0:0:0:0:0:0'}
                for slot, item in [(16, 2042), (18, 22347)]}
            value['sections']['professions'] = {str(i): {'name': 'Synthetic', 'rank': 300,
                'max_rank': 300, 'skill_line': skill} for i, skill in enumerate([202, 164], 1)}
            value['sections']['pet_care'] = {'present': True, 'level': 60}
            value['sections']['bags'] = {'0': {'1': {'item_id': 4549, 'count': 1, 'link': 'item:4549:0:0:0:0:0:0'}}}
            store = SnapshotStore(base)
            key = store.ingest(value)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue)}
            journal = base / 'gears.sqlite'
            from unittest.mock import patch
            from mcp import ClientSession, StdioServerParameters
            from mcp.client.stdio import stdio_client
            async def inspect():
                async with stdio_client(StdioServerParameters(command=str(binary), env=env), errlog=errlog) as (read, write):
                    async with ClientSession(read, write) as session:
                        await session.initialize()
                        response = await session.call_tool('inspect_snapshot', {'snapshot_id': key, 'section': 'stats'})
                        self.assertFalse(response.isError, response.content)
            asyncio.run(inspect())
            with patch.dict(os.environ, env):
                drafts, skipped, coverage, _, _, _, search = asyncio.run(collect(binary, key,
                    ['finger1'], errlog, combination_offset=0, combination_limit=4))
            report = plan_report(drafts, True, True)
            report.update(gear_search=search, skipped=skipped, storage_coverage=coverage, best_in_slot_verified=False)
            self.assertEqual(len(report['resolved_drafts']), 2)
            append_report(journal, report)
            gear_bytes = journal.read_bytes()
            _, _, encounter = inputs()
            encounter['simulation_input']['simOptions'] = {'iterations': 1000, 'randomSeed': '20261109'}
            second = deepcopy(encounter)
            second['simulation_input']['encounter']['duration'] = 90
            paths = [base / 'first.json', base / 'second.json']
            for path, profile in zip(paths, [encounter, second]): path.write_text(json.dumps(profile))
            args = [sys.executable, '-m', 'tools.run_forever_encounter_windows', '--mcp', str(binary),
                '--journal', str(journal), '--window-offset', '0', '--template', str(template_path),
                '--engine', str(engine), '--engine-sha256', expected, '--simulation-journal', str(base / 'results'),
                '--paired', '--max-new-runs', '1', '--encounter-lookup', str(paths[0]),
                '--encounter-lookup', str(paths[1]), '--encounter-lookup', str(paths[0]),
                '--encounter-weight', '1', '--encounter-weight', '3', '--encounter-weight', '1']
            def launch(extra=(), fails=False, command=None):
                proc = subprocess.run([*(args if command is None else command), *extra], cwd=root,
                    env=env, capture_output=True, text=True, timeout=65)
                self.assertEqual(proc.returncode, 1 if fails else 0, proc.stderr)
                self.assertNotIn(folder, proc.stdout + proc.stderr)
                self.assertEqual(journal.read_bytes(), gear_bytes)
                if fails:
                    self.assertEqual(proc.stdout, '')
                    return
                return json.loads(proc.stdout)
            for index in range(4):
                result = launch()
                self.assertEqual((result['new_runs'], result['new_iterations']), (1, 1000))
                self.assertEqual(result['requested_results_complete'], index == 3)
                if index < 3: self.assertIsNone(result['weighted_ranking'])
            cached = launch(['--checkpoint-only'])
            self.assertEqual((cached['new_runs'], cached['checkpoint_hits']), (0, 6))
            self.assertEqual(result['weighted_ranking'], cached['weighted_ranking'])
            # Uncached direct engine runs check aggregate metrics and paired uncertainty.
            for profile, path in zip(cached['profiles'][:2], paths):
                options = SimpleNamespace(window_offset=0, time_budget_seconds=45, mcp=binary,
                    template=template_path, encounter_lookup=path, journal=journal)
                with patch.dict(os.environ, env): prepared = asyncio.run(preparer.prepare(options))
                actual = {row['review']: row for row in profile['result']['results']}
                fresh_rows = {}
                for row in prepared['requests']:
                    fresh = adapter.run(engine, expected, row['request'], paired=True)
                    fresh_rows[row['review']] = fresh
                    for field in ['mean_dps', 'dps_standard_deviation']:
                        self.assertEqual(actual[row['review']][field], fresh[field])
                self.assertGreater(actual[1]['mean_dps'], 0)
                baseline = fresh_rows[1]['_dps_samples']
                for review, fresh in fresh_rows.items():
                    self.assertEqual(len(fresh['_dps_samples']), 1000)
                    error = statistics.stdev(a-b for a,b in zip(fresh['_dps_samples'], baseline, strict=True)) / math.sqrt(1000)
                    self.assertEqual(actual[review]['paired_dps_standard_error'], error)
                    if review != 1:
                        delta = fresh['mean_dps'] - fresh_rows[1]['mean_dps']
                        lower, upper = delta - 1.96 * error, delta + 1.96 * error
                        self.assertEqual(actual[review]['paired_dps_delta_interval_approx_95'],
                            {'lower': lower, 'upper': upper, 'includes_zero': lower <= 0 <= upper})
                leader, runner_up = sorted(fresh_rows, key=lambda review: (-fresh_rows[review]['mean_dps'], review))
                error = statistics.stdev(a-b for a,b in zip(fresh_rows[leader]['_dps_samples'],
                    fresh_rows[runner_up]['_dps_samples'], strict=True)) / math.sqrt(1000)
                self.assertEqual(profile['result']['observed_leading_dps_pair'], {
                    'leading_review': leader, 'runner_up_review': runner_up,
                    'mean_dps_gap': fresh_rows[leader]['mean_dps'] - fresh_rows[runner_up]['mean_dps'],
                    'paired_dps_standard_error': error})
            gains = []
            for profile in cached['profiles']:
                rows = {row['review']: row for row in profile['result']['results']}
                gains.append((rows[2]['mean_dps'] / rows[1]['mean_dps'] - 1) * 100)
            weighted = {row['review']: row for row in cached['weighted_ranking']['results']}
            self.assertAlmostEqual(weighted[2]['weighted_dps_gain_pct'], sum(a*b for a,b in zip(gains, [.2,.6,.2])))
            # Split the same gear work across saved windows, then exercise the
            # combined runner with both existing checkpoints and a cold journal.
            paged_journal = base / 'paged.sqlite'
            for offset in [0, 1]:
                with patch.dict(os.environ, env):
                    drafts, skipped, coverage, _, _, _, search = asyncio.run(collect(binary, key,
                        ['finger1'], errlog, combination_offset=offset, combination_limit=1))
                page_report = plan_report(drafts, True, True)
                page_report.update(gear_search=search, skipped=skipped, storage_coverage=coverage,
                    best_in_slot_verified=False)
                append_report(paged_journal, page_report)
            paged_bytes = paged_journal.read_bytes()
            journal_command = list(args)
            journal_command[2] = 'tools.run_forever_saved_journal'
            offset_flag = journal_command.index('--window-offset')
            del journal_command[offset_flag:offset_flag + 2]
            journal_command.extend(['--journal', str(paged_journal), '--page-limit', '1'])
            expected_gears = {row['simulator_gear_sha256']: row for row in cached['weighted_ranking']['results']}
            def check_combined(result):
                self.assertTrue(result['all_saved_resolved_results_complete'])
                self.assertEqual((result['visited_page_count'], result['completed_window_count']), (2, 2))
                self.assertEqual(result['ranking_scope'], 'all_saved_resolved_requests')
                self.assertEqual(len(result['ranked_gear_sets']), len(expected_gears))
                self.assertEqual([row['simulator_gear_sha256'] for row in result['ranked_gear_sets']],
                    sorted(expected_gears, key=lambda gear: (-expected_gears[gear]['weighted_dps_gain_pct'], gear)))
                actual = {row['simulator_gear_sha256']: row for row in result['ranked_gear_sets']}
                self.assertEqual(set(actual), set(expected_gears))
                for gear, row in actual.items():
                    self.assertEqual(row['weighted_dps_gain_pct'], expected_gears[gear]['weighted_dps_gain_pct'])
                    review = expected_gears[gear]['review']
                    means = [next(entry['mean_dps'] for entry in profile['result']['results'] if entry['review'] == review)
                        for profile in cached['profiles']]
                    self.assertEqual(row['mean_dps_by_encounter'], means)
                    self.assertEqual(row['source_window_offset_decimal'], '0' if review == 1 else '1')
                    self.assertEqual(row['source_window_review'], review)
                self.assertEqual(paged_journal.read_bytes(), paged_bytes)
                for flag in ['game_execution', 'best_in_slot_verified', 'feasible_set_coverage_verified', 'global_ranking_verified']:
                    self.assertFalse(result[flag])
            combined = launch(['--checkpoint-only'], command=journal_command)
            self.assertEqual((combined['new_runs'], combined['checkpoint_hits']), (0, 9))
            check_combined(combined)
            cold_root = base / 'combined-results'
            for index in range(4):
                combined = launch(['--simulation-journal', str(cold_root)], command=journal_command)
                self.assertEqual((combined['new_runs'], combined['new_iterations']), (1, 1000))
                self.assertEqual(combined['all_saved_resolved_results_complete'], index == 3)
                self.assertEqual(paged_journal.read_bytes(), paged_bytes)
            check_combined(combined)
            rebuilt = launch(['--simulation-journal', str(cold_root), '--checkpoint-only'], command=journal_command)
            self.assertEqual((rebuilt['new_runs'], rebuilt['checkpoint_hits']), (0, 9))
            self.assertEqual(rebuilt['ranked_gear_sets'], combined['ranked_gear_sets'])
            checkpoint = base / 'results/checkpoints.sqlite3'
            before = checkpoint.read_bytes()
            combined_checkpoint = cold_root / 'checkpoints.sqlite3'
            combined_before = combined_checkpoint.read_bytes()
            changed = deepcopy(value)
            changed['sections']['stats']['level'] = 20
            low_key = store.ingest(changed)
            low = deepcopy(report)
            low['gear_search']['snapshot_id'] = low_key
            append_report(base / 'low.sqlite', low)
            launch(['--journal', str(base / 'low.sqlite')], fails=True)
            launch(['--journal', str(base / 'low.sqlite'), '--simulation-journal', str(cold_root)],
                fails=True, command=journal_command)
            self.assertEqual(checkpoint.read_bytes(), before)
            self.assertEqual(combined_checkpoint.read_bytes(), combined_before)
            for flag in ['game_execution', 'best_in_slot_verified', 'feasible_set_coverage_verified']:
                self.assertFalse(cached[flag])


if __name__ == '__main__':
    unittest.main()
