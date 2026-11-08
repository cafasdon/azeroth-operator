"""Persist and resume reported plan coverage without losing prior transactions."""
from copy import deepcopy
from contextlib import closing
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from test_merge_gear_windows import report
from tools.journal_forever_gear_windows import append_report, journal_status
import tools.journal_forever_gear_windows as journal


class GearWindowJournalTests(unittest.TestCase):
    def test_maximum_valid_report_fits_sqlite_row_limit(self):
        value = report(0, ['b' * 64])
        value['PRIVATE_PADDING'] = ''
        base_size = len(json.dumps(value, sort_keys=True, separators=(',', ':')).encode())
        value['PRIVATE_PADDING'] = 'x' * (4 * 1024 * 1024 - base_size)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'boundary.sqlite'
            self.assertTrue(append_report(path, value)['inserted'])
            status = journal_status(path)
            self.assertEqual(status['covered_coordinate_count'], 4)
            self.assertNotIn('PRIVATE', json.dumps(status))

    def test_oversized_database_values_reject_before_python_row_validation(self):
        with tempfile.TemporaryDirectory() as folder:
            for field in ['report', 'report_sha', 'start_key', 'end_key']:
                path = Path(folder) / f'{field}.sqlite'
                append_report(path, report(0, ['b' * 64]))
                with closing(sqlite3.connect(path)) as connection:
                    if field == 'report':
                        connection.execute('UPDATE windows SET report=zeroblob(?),payload_bytes=1', (8 * 1024 * 1024,))
                    else:
                        connection.execute(f'UPDATE windows SET {field}=?', ('X' * (8 * 1024 * 1024),))
                    connection.commit()
                before = path.read_bytes()
                for operation in [lambda: journal_status(path),
                                  lambda: append_report(path, report(0, ['b' * 64])),
                                  lambda: append_report(path, report(4, ['b' * 64]))]:
                    with patch('tools.journal_forever_gear_windows.checked_row', wraps=journal.checked_row) as checked:
                        with self.assertRaises((ValueError, sqlite3.DataError)):
                            operation()
                        self.assertEqual(checked.call_count, 0, field)
                    self.assertEqual(path.read_bytes(), before)

    def test_streaming_more_than_64_windows_and_gap_bounds(self):
        def page(offset, limit=1):
            value = report(0, ['b' * 64], 1)
            search = value['gear_search']
            search.update(requested_slots=['finger1'], candidate_locations_by_slot={'finger1': 256},
                          maximum_simultaneous_swaps=1, combinations_including_baseline=257)
            count = min(limit, 257 - offset)
            search['combination_window'].update(offset=offset, limit=limit, considered=count,
                total_combinations_decimal='257', next_offset=offset + count if offset + count < 257 else None)
            return value
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'stream.sqlite'
            for offset in range(65):
                append_report(path, page(offset))
            self.assertEqual(journal_status(path)['next_uncovered_offset_decimal'], '65')
            append_report(path, page(65, 192))
            status = journal_status(path)
            self.assertEqual(status['window_count'], 66)
            self.assertEqual(status['covered_coordinate_count'], 257)
            self.assertTrue(status['reported_coordinate_coverage_complete'])
            self.assertFalse(status['feasible_set_coverage_verified'])
            sparse = Path(folder) / 'sparse.sqlite'
            for offset in range(0, 130, 2):
                append_report(sparse, page(offset))
            gaps = journal_status(sparse)
            self.assertEqual(gaps['next_uncovered_offset_decimal'], '1')
            self.assertEqual(gaps['uncovered_range_count'], 65)
            self.assertEqual(len(gaps['uncovered_ranges']), 64)
            self.assertTrue(gaps['ranges_truncated'])
            before = path.read_bytes()
            with patch('tools.journal_forever_gear_windows.monotonic', side_effect=[0] + [100] * 1000):
                with self.assertRaises((ValueError, sqlite3.OperationalError)):
                    journal_status(path)
            self.assertEqual(path.read_bytes(), before)

    def test_resume_idempotency_budget_and_scope_rollback(self):
        pages = [report(0, ['b' * 64, 'a' * 64]), report(4, ['b' * 64, 'c' * 64]), report(8, ['b' * 64])]
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'PRIVATE_journal.sqlite'
            self.assertTrue(append_report(path, pages[1])['inserted'])
            progress = journal_status(path)
            self.assertEqual(progress['next_uncovered_offset_decimal'], '0')
            self.assertEqual(progress['uncovered_range_count'], 2)
            before = path.read_bytes()
            self.assertFalse(append_report(path, deepcopy(pages[1]))['inserted'])
            self.assertEqual(path.read_bytes(), before)
            self.assertTrue(append_report(path, pages[0])['inserted'])
            self.assertEqual(journal_status(path)['next_uncovered_offset_decimal'], '8')
            before = path.read_bytes()
            bad = deepcopy(pages[2])
            bad['gear_search']['combination_window']['scope_sha256'] = '3' * 64
            for value, quota in [(bad, 64 * 1024 * 1024), (pages[2], 1),
                                  (report(3, ['b' * 64]), 64 * 1024 * 1024)]:
                with self.assertRaises(ValueError):
                    append_report(path, value, quota)
                self.assertEqual(path.read_bytes(), before)
            conflict = deepcopy(pages[1])
            conflict['PRIVATE_CHANGED'] = 'PRIVATE'
            with self.assertRaises(ValueError):
                append_report(path, conflict)
            self.assertEqual(path.read_bytes(), before)
            self.assertTrue(append_report(path, pages[2])['inserted'])
            complete = journal_status(path)
            self.assertTrue(complete['reported_coordinate_coverage_complete'])
            self.assertEqual(complete['covered_coordinate_count'], 9)
            self.assertIsNone(complete['next_uncovered_offset_decimal'])
            self.assertFalse(complete['best_in_slot_verified'])
            self.assertFalse(complete['feasible_set_coverage_verified'])
            self.assertFalse(complete['simulation_performed'])
            self.assertNotIn('PRIVATE', json.dumps(complete))
            empty = Path(folder) / 'failed_initial.sqlite'
            with self.assertRaises(ValueError):
                append_report(empty, pages[0], 1)
            self.assertTrue(append_report(empty, pages[0])['inserted'])
            missing = Path(folder) / 'missing.sqlite'
            with self.assertRaises(ValueError):
                journal_status(missing)
            self.assertFalse(missing.exists())
            with self.assertRaises(ValueError):
                append_report(path, report(10, ['b' * 64]))

    def test_foreign_schema_corruption_and_busy_lock(self):
        page = report(0, ['b' * 64])
        with tempfile.TemporaryDirectory() as folder:
            foreign = Path(folder) / 'foreign.sqlite'
            with closing(sqlite3.connect(foreign)) as connection:
                connection.execute('CREATE TABLE private_data(value TEXT)')
                connection.execute("INSERT INTO private_data VALUES('PRIVATE')")
                connection.commit()
            foreign_before = foreign.read_bytes()
            for action in [lambda: append_report(foreign, page), lambda: journal_status(foreign)]:
                with self.assertRaises(ValueError):
                    action()
                self.assertEqual(foreign.read_bytes(), foreign_before)
            path = Path(folder) / 'own.sqlite'
            append_report(path, page)
            before = path.read_bytes()
            lock = sqlite3.connect(path, isolation_level=None)
            try:
                lock.execute('BEGIN IMMEDIATE')
                with self.assertRaises(sqlite3.OperationalError):
                    append_report(path, report(4, ['b' * 64]))
            finally:
                lock.close()
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(journal_status(path)['window_count'], 1)
            with closing(sqlite3.connect(path)) as connection:
                connection.execute('CREATE TABLE unexpected(value TEXT)')
                connection.commit()
            modified = path.read_bytes()
            with self.assertRaises(ValueError):
                append_report(path, report(4, ['b' * 64]))
            self.assertEqual(path.read_bytes(), modified)
            corrupt = Path(folder) / 'corrupt.sqlite'
            append_report(corrupt, page)
            with closing(sqlite3.connect(corrupt)) as connection:
                connection.execute('UPDATE windows SET report=?', (b'PRIVATE_CORRUPTION',))
                connection.commit()
            with self.assertRaises(ValueError):
                journal_status(corrupt)
            link = Path(folder) / 'link.sqlite'
            try:
                link.symlink_to(foreign)
            except OSError:
                pass  # Windows may deny symlink creation without developer mode.
            else:
                with self.assertRaises(ValueError):
                    append_report(link, page)
                self.assertEqual(foreign.read_bytes(), foreign_before)

    def test_real_module_restart_and_abandoned_transaction(self):
        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'PRIVATE_journal.sqlite'
            artifact = Path(folder) / 'PRIVATE_artifact.json'
            args = [sys.executable, '-m', 'tools.journal_forever_gear_windows', '--journal', str(path)]
            for offset in [0, 4, 8]:
                artifact.write_text(json.dumps(report(offset, ['b' * 64])))
                process = subprocess.run([*args, '--add', str(artifact)], cwd=root,
                    capture_output=True, text=True, timeout=20)
                self.assertEqual(process.returncode, 0, process.stderr)
                self.assertTrue(json.loads(process.stdout)['inserted'])
                self.assertNotIn('PRIVATE', process.stdout)
            before = path.read_bytes()
            repeated = subprocess.run([*args, '--add', str(artifact)], cwd=root,
                capture_output=True, text=True, timeout=20)
            self.assertEqual(repeated.returncode, 0, repeated.stderr)
            self.assertFalse(json.loads(repeated.stdout)['inserted'])
            self.assertEqual(path.read_bytes(), before)
            status = subprocess.run(args, cwd=root, capture_output=True, text=True, timeout=20)
            self.assertEqual(status.returncode, 0, status.stderr)
            self.assertTrue(json.loads(status.stdout)['reported_coordinate_coverage_complete'])
            self.assertNotIn('PRIVATE', status.stdout)
            artifact.write_text('{"PRIVATE":1e999}')
            rejected = subprocess.run([*args, '--add', str(artifact)], cwd=root,
                capture_output=True, text=True, timeout=20)
            self.assertEqual(rejected.returncode, 1)
            self.assertEqual(rejected.stdout, '')
            self.assertNotIn('PRIVATE', rejected.stderr)
            self.assertEqual(path.read_bytes(), before)
            interrupted = Path(folder) / 'interrupted.sqlite'
            append_report(interrupted, report(0, ['b' * 64]))
            child = "import os,sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('BEGIN IMMEDIATE'); c.execute('DELETE FROM windows'); os._exit(0)"
            process = subprocess.run([sys.executable, '-c', child, str(interrupted)], timeout=10)
            self.assertEqual(process.returncode, 0)
            append_report(interrupted, report(4, ['b' * 64]))
            self.assertEqual(journal_status(interrupted)['window_count'], 2)
