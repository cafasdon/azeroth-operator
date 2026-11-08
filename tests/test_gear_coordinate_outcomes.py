"""Exact saved coordinate diagnostics without leaking raw item selections."""
from copy import deepcopy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from test_merge_gear_windows import report
from tools.journal_forever_gear_windows import append_report, journal_window
from tools.merge_forever_gear_windows import merge_reports


def detailed_report():
    value = report(0, ['b' * 64, 'a' * 64])
    value['gear_search'].update(proposed_alternative_drafts=3, equivalent_simulator_drafts=1)
    value['gear_search']['combination_window']['coordinate_outcomes'] = [
        {'coordinate_decimal': '0', 'status': 'baseline', 'review': 1},
        {'coordinate_decimal': '1', 'status': 'resolved', 'review': 2},
        {'coordinate_decimal': '2', 'status': 'equivalent', 'review': 2},
        {'coordinate_decimal': '3', 'status': 'unresolved', 'reason': 'unresolved_simulator_draft'}]
    return value


class CoordinateOutcomeTests(unittest.TestCase):
    def test_exact_window_cli_legacy_unknown_and_invalid_rejection(self):
        value = detailed_report()
        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'PRIVATE_journal.sqlite'
            append_report(path, value)
            append_report(path, report(4, ['b' * 64]))
            before = path.read_bytes()
            result = journal_window(path, 0)
            self.assertTrue(result['coordinate_outcomes_available'])
            self.assertEqual(result['coordinate_outcomes'], value['gear_search']['combination_window']['coordinate_outcomes'])
            self.assertEqual(result['resolution_accounting']['unresolved_proposed_coordinates'], 1)
            self.assertFalse(result['best_in_slot_verified'])
            legacy = journal_window(path, 4)
            self.assertFalse(legacy['coordinate_outcomes_available'])
            self.assertIsNone(legacy['coordinate_outcomes'])
            for offset in [-1, True, 1, 99]:
                with self.assertRaises(ValueError):
                    journal_window(path, offset)
            for offset, code in [('0', 0), ('4', 0), ('1', 1), ('-1', 1)]:
                process = subprocess.run([sys.executable, '-m', 'tools.journal_forever_gear_windows',
                    '--journal', str(path), '--window-offset', offset], cwd=root, capture_output=True,
                    text=True, timeout=30)
                self.assertEqual(process.returncode, code, process.stderr)
                self.assertNotIn('PRIVATE', process.stdout + process.stderr)
                if code == 0:
                    self.assertEqual(json.loads(process.stdout)['mode'], 'gear_window_inspection')
            self.assertEqual(path.read_bytes(), before)

    def test_outcomes_match_counts_coordinates_and_prior_reviews(self):
        value = detailed_report()
        merge_reports([value])
        mutations = [(0, 'coordinate_decimal', 'PRIVATE'), (0, 'review', True),
            (0, 'status', 'resolved'), (1, 'review', 1), (1, 'review', 3),
            (1, 'status', 'equivalent'), (2, 'review', 3), (2, 'review', True),
            (3, 'reason', 'PRIVATE_ERROR'), (3, 'status', 'excluded'), (3, 'PRIVATE', True)]
        for index, field, replacement in mutations:
            bad = deepcopy(value)
            bad['gear_search']['combination_window']['coordinate_outcomes'][index][field] = replacement
            with self.assertRaises(ValueError):
                merge_reports([bad])
        for rows in [None, [], value['gear_search']['combination_window']['coordinate_outcomes'] * 2]:
            bad = deepcopy(value)
            bad['gear_search']['combination_window']['coordinate_outcomes'] = rows
            with self.assertRaises(ValueError):
                merge_reports([bad])
