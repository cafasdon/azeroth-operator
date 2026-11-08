"""Distinguish covered coordinates from unresolved proposed equipment plans."""
from copy import deepcopy
import json
from pathlib import Path
import random
import subprocess
import sys
import tempfile
import unittest

from test_merge_gear_windows import report
from tools.journal_forever_gear_windows import append_report, journal_status
from tools.merge_forever_gear_windows import merge_reports


class ResolutionAccountingTests(unittest.TestCase):
    def test_complete_coverage_can_retain_unresolved_proposals(self):
        first = report(0, ['b' * 64, 'a' * 64])
        first['gear_search'].update(proposed_alternative_drafts=3, equivalent_simulator_drafts=1)
        second = report(4, ['b' * 64, 'a' * 64])
        second['gear_search']['proposed_alternative_drafts'] = 3
        last = report(8, ['b' * 64])
        pages = [first, second, last]
        expected = {'basis': 'validated_local_plan_report_metadata', 'baseline_coordinates': 1,
            'pre_review_excluded_coordinates': 2, 'proposed_alternative_coordinates': 6,
            'resolved_alternative_draft_instances': 2, 'equivalent_alternative_coordinates': 1,
            'unresolved_proposed_coordinates': 3, 'baseline_review_instances': 3}
        merged = merge_reports(pages)
        self.assertTrue(merged['reported_coordinate_coverage_complete'])
        self.assertEqual(merged['resolution_accounting'], expected)
        self.assertEqual(merged['distinct_simulator_gear_sets'], 2)
        self.assertFalse(merged['feasible_set_coverage_verified'])
        self.assertEqual(merge_reports(list(reversed(pages)))['resolution_accounting'], expected)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'PRIVATE_progress.sqlite'
            for page in [last, first, second]:
                append_report(path, page)
            before = path.read_bytes()
            append_report(path, first)
            status = journal_status(path)
            self.assertEqual(status['resolution_accounting'], expected)
            self.assertEqual(path.read_bytes(), before)
            root = Path(__file__).resolve().parents[1]
            process = subprocess.run([sys.executable, '-m', 'tools.journal_forever_gear_windows',
                '--journal', str(path)], cwd=root, capture_output=True, text=True, timeout=30)
            self.assertEqual(process.returncode, 0, process.stderr)
            self.assertEqual(json.loads(process.stdout)['resolution_accounting'], expected)
            self.assertNotIn('PRIVATE', process.stdout + process.stderr)
            self.assertEqual(path.read_bytes(), before)
            invalid = deepcopy(first)
            invalid['gear_search']['equivalent_simulator_drafts'] = 3
            with self.assertRaises(ValueError):
                append_report(path, invalid)
            self.assertEqual(path.read_bytes(), before)

    def test_seeded_coordinate_partition_oracle(self):
        rng = random.Random(20261108)
        for _ in range(100):
            offset = rng.choice([0, 4, 8])
            count = min(4, 9 - offset)
            baseline = int(offset == 0)
            proposed = rng.randrange(count - baseline + 1)
            resolved = rng.randrange(proposed + 1)
            equivalent = rng.randrange(proposed - resolved + 1)
            keys = ['b' * 64] + [f'{i:064x}' for i in range(resolved)]
            page = report(offset, keys)
            page['gear_search'].update(proposed_alternative_drafts=proposed,
                                      equivalent_simulator_drafts=equivalent)
            summary = merge_reports([page])
            counts = summary['resolution_accounting']
            self.assertEqual(counts['baseline_coordinates'], baseline)
            self.assertEqual(counts['pre_review_excluded_coordinates'], count - baseline - proposed)
            self.assertEqual(counts['proposed_alternative_coordinates'], proposed)
            self.assertEqual(counts['resolved_alternative_draft_instances'], resolved)
            self.assertEqual(counts['equivalent_alternative_coordinates'], equivalent)
            self.assertEqual(counts['unresolved_proposed_coordinates'], proposed - resolved - equivalent)
            self.assertEqual(counts['baseline_review_instances'], 1)
            partition = sum(counts[field] for field in ['baseline_coordinates',
                'pre_review_excluded_coordinates', 'resolved_alternative_draft_instances',
                'equivalent_alternative_coordinates', 'unresolved_proposed_coordinates'])
            self.assertEqual(partition, summary['covered_coordinate_count'])
