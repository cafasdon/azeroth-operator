"""Validate reported coordinate coverage without promoting it into full BiS."""
from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.merge_forever_gear_windows import merge_reports


def report(offset, keys, limit=4):
    total = 9
    count = min(limit, max(0, total - offset))
    return {'mode': 'plan_only', 'best_in_slot_verified': False, 'baseline_review': 1,
        'resolved_drafts': [{'review': i, 'simulator_gear_sha256': key,
            'selections': [] if i == 1 else [{'PRIVATE_SELECTION': True}]}
            for i, key in enumerate(keys, 1)],
        'PRIVATE_CHARACTER': 'PRIVATE_CHARACTER',
        'gear_search': {'snapshot_id': '1' * 64, 'requested_slots': ['finger1', 'finger2'],
            'candidate_locations_by_slot': {'finger1': 2, 'finger2': 2}, 'candidate_filters': {},
            'maximum_simultaneous_swaps': 2, 'inventory_evidence': None,
            'combinations_including_baseline': total, 'resolved_drafts_including_baseline': len(keys),
            'proposed_alternative_drafts': max(len(keys) - 1, 0), 'equivalent_simulator_drafts': 0,
            'distinct_simulator_gear_sets': len(set(keys)),
            'combination_window': {'version': 1, 'scope_sha256': '2' * 64, 'offset': offset,
                'limit': limit, 'considered': count, 'total_combinations_decimal': str(total),
                'next_offset': offset + count if offset + count < total else None,
                'baseline_reused': True, 'covers_requested_combinations': offset == 0 and count == total}}}


class MergeGearWindowTests(unittest.TestCase):
    def test_large_window_pool_requires_matching_explicit_scan_budget(self):
        first = report(0, ['b' * 64])
        search = first['gear_search']
        search['candidate_locations_by_slot']['finger1'] = 257
        search['combinations_including_baseline'] = 774
        search['combination_window']['total_combinations_decimal'] = '774'
        for budget in [None, True, 256, 4097]:
            bad = deepcopy(first)
            if budget is not None:
                bad['gear_search']['candidate_scan_limit'] = budget
            with self.assertRaises(ValueError):
                merge_reports([bad])
        search['candidate_scan_limit'] = 257
        second = deepcopy(first)
        second['gear_search']['candidate_scan_limit'] = 4096
        second['gear_search']['combination_window'].update(offset=4, next_offset=8)
        result = merge_reports([first, second])
        self.assertEqual(result['total_combinations_decimal'], '774')
        self.assertEqual(result['covered_coordinate_count'], 8)
        self.assertFalse(result['best_in_slot_verified'])
        from tools.journal_forever_gear_windows import append_report, journal_status
        with tempfile.TemporaryDirectory() as folder:
            journal = Path(folder) / 'plans.sqlite'
            append_report(journal, first)
            append_report(journal, second)
            progress = journal_status(journal)
            self.assertEqual(progress['covered_coordinate_count'], 8)
            self.assertEqual(progress['next_uncovered_offset_decimal'], '8')

    def test_module_rejects_numeric_overflow(self):
        root = Path(__file__).resolve().parents[1]
        valid = report(0, ['b' * 64], 64)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'PRIVATE_overflow.json'
            for number in ['1e999', '-1e999']:
                path.write_text(json.dumps(valid)[:-1] + ',"PRIVATE_EXTRA":' + number + '}')
                result = subprocess.run([sys.executable, '-m', 'tools.merge_forever_gear_windows', str(path)],
                    cwd=root, capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode, 1)
                self.assertEqual(result.stdout, '')
                self.assertNotIn('PRIVATE', result.stderr)
            path.write_text(json.dumps({**valid, 'PRIVATE_EXTRA': 1.25}))
            result = subprocess.run([sys.executable, '-m', 'tools.merge_forever_gear_windows', str(path)],
                cwd=root, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn('PRIVATE', result.stdout)

    def test_coverage_grouping_gaps_and_invalid_metadata(self):
        pages = [report(0, ['b' * 64, 'a' * 64]), report(4, ['b' * 64, 'a' * 64, 'c' * 64]),
                 report(8, ['b' * 64])]
        result = merge_reports(pages)
        self.assertTrue(result['reported_coordinate_coverage_complete'])
        self.assertEqual(result['covered_coordinate_count'], 9)
        self.assertIsNone(result['next_uncovered_offset_decimal'])
        self.assertEqual(result['distinct_simulator_gear_sets'], 3)
        self.assertEqual(len(result['drafts'][0]['references']), 3)
        self.assertEqual(result['drafts'][0]['simulator_gear_sha256'], 'b' * 64)
        self.assertFalse(result['best_in_slot_verified'])
        self.assertFalse(result['feasible_set_coverage_verified'])
        self.assertFalse(result['simulation_performed'])
        self.assertNotIn('PRIVATE', json.dumps(result))
        gap = merge_reports([pages[0], pages[2]])
        self.assertEqual(gap['uncovered_ranges'], [{'start_decimal': '4', 'end_decimal': '8'}])
        self.assertFalse(gap['reported_coordinate_coverage_complete'])
        self.assertEqual(merge_reports([pages[1]])['next_uncovered_offset_decimal'], '0')
        self.assertTrue(merge_reports(list(reversed(pages)))['reported_coordinate_coverage_complete'])
        for invalid in [[], pages * 22, [pages[0], pages[0]], [pages[0], report(3, ['b' * 64])]]:
            with self.assertRaises(ValueError):
                merge_reports(invalid)
        mutations = [(['mode'], 'paired'), (['best_in_slot_verified'], True), (['baseline_review'], True),
            (['gear_search', 'snapshot_id'], 'PRIVATE'),
            (['gear_search', 'requested_slots'], ['finger1', 'finger1']),
            (['gear_search', 'candidate_locations_by_slot', 'finger1'], 257),
            (['gear_search', 'maximum_simultaneous_swaps'], True),
            (['gear_search', 'candidate_filters'], []), (['gear_search', 'combinations_including_baseline'], 8),
            (['gear_search', 'proposed_alternative_drafts'], 4),
            (['gear_search', 'distinct_simulator_gear_sets'], 1),
            (['gear_search', 'combination_window', 'version'], True),
            (['gear_search', 'combination_window', 'scope_sha256'], 'A' * 64),
            (['gear_search', 'combination_window', 'offset'], -1),
            (['gear_search', 'combination_window', 'limit'], 0),
            (['gear_search', 'combination_window', 'considered'], True),
            (['gear_search', 'combination_window', 'total_combinations_decimal'], '09'),
            (['gear_search', 'combination_window', 'next_offset'], 5),
            (['gear_search', 'combination_window', 'covers_requested_combinations'], True),
            (['gear_search', 'combination_window', 'baseline_reused'], False),
            (['resolved_drafts', 0, 'simulator_gear_sha256'], None),
            (['resolved_drafts', 0, 'selections'], [{}]), (['resolved_drafts', 1, 'review'], 3)]
        for path, value in mutations:
            bad = deepcopy(pages[0])
            target = bad
            for key in path[:-1]:
                target = target[key]
            target[path[-1]] = value
            with self.assertRaises(ValueError, msg=path):
                merge_reports([bad])
        for field, value in [('scope_sha256', '3' * 64)]:
            bad = deepcopy(pages[1])
            bad['gear_search']['combination_window'][field] = value
            with self.assertRaises(ValueError):
                merge_reports([pages[0], bad])
        bad = deepcopy(pages[1])
        bad['resolved_drafts'][0]['simulator_gear_sha256'] = 'd' * 64
        with self.assertRaises(ValueError):
            merge_reports([pages[0], bad])

    def test_real_producer_and_merger_module(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            value = fixture()
            value['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            value['sections']['bags'] = {'0': {str(i): {'item_id': item, 'count': 1,
                'link': f'item:{item}:0:0:0:0:0:0'} for i, item in enumerate([4549, 4550], 1)}}
            key = SnapshotStore(Path(folder)).ingest(value)
            template, _, _ = inputs()
            template_path = Path(folder) / 'template.json'
            template_path.write_text(json.dumps(template))
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            reports, paths = [], []
            for offset, limit in [(0, 4), (4, 4), (8, 4), (0, 64)]:
                args = [sys.executable, '-m', 'tools.rank_forever_slot', '--mcp', str(binary), '--snapshot-id', key,
                    '--slot', 'finger1', '--slot', 'finger2', '--template', str(template_path), '--plan-only',
                    '--collapse-identical-items', '--combination-offset', str(offset), '--combination-limit', str(limit)]
                completed = subprocess.run(args, cwd=root, env=env, capture_output=True, text=True, timeout=60)
                self.assertEqual(completed.returncode, 0, completed.stderr)
                actual = json.loads(completed.stdout)
                if limit == 64:
                    whole = actual
                    break
                actual['PRIVATE_EXTRA'] = 'PRIVATE_EXTRA'
                path = Path(folder) / f'PRIVATE_report_{offset}.json'
                path.write_text(json.dumps(actual))
                reports.append(actual)
                paths.append(path)
            expected = merge_reports(reports)
            self.assertTrue(expected['reported_coordinate_coverage_complete'])
            self.assertEqual({row['simulator_gear_sha256'] for row in expected['drafts']},
                             {row['simulator_gear_sha256'] for row in whole['resolved_drafts']})
            before = [path.read_bytes() for path in paths]
            args = [sys.executable, '-m', 'tools.merge_forever_gear_windows', *map(str, paths)]
            completed = subprocess.run(args, cwd=root, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(json.loads(completed.stdout), expected)
            self.assertNotIn('PRIVATE', completed.stdout)
            self.assertEqual(before, [path.read_bytes() for path in paths])
            bad = Path(folder) / 'PRIVATE_bad.json'
            for contents in ['{"mode":"PRIVATE","mode":"plan_only"}', '{"PRIVATE":NaN}',
                             'PRIVATE', ' ' * (4 * 1024 * 1024 + 1)]:
                bad.write_text(contents)
                rejected = subprocess.run([sys.executable, '-m', 'tools.merge_forever_gear_windows', str(bad)],
                    cwd=root, env=env, capture_output=True, text=True, timeout=30)
                self.assertEqual(rejected.returncode, 1)
                self.assertEqual(rejected.stdout, '')
                self.assertNotIn('PRIVATE', rejected.stderr)
