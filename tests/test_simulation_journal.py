"""Durable per-user numeric checkpoints through the actual fixed-run adapter."""
from contextlib import closing
from contextlib import redirect_stdout
from copy import deepcopy
import hashlib
from itertools import product
import json
import io
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from tools.run_forever_sim import run
from tools import run_forever_sim as adapter
from tools.simulation_journal import SimulationJournal
from test_simulator_level import inputs


def request(seed='10'):
    return {'raid': {'parties': [{'players': [{'name': 'PRIVATE_CHARACTER', 'equipment': {'items': []}}]}]},
            'encounter': {'duration': 90}, 'simOptions': {'iterations': 3, 'randomSeed': seed}}


def summary(engine, seed='10', paired=False, tank=False, healing=False):
    value = {'iterations': 3, 'seed': seed, 'engine_sha256': engine, 'mean_dps': 100, 'dps_standard_deviation': 2}
    if tank:
        value.update(mean_dtps=10, dtps_standard_deviation=1, mean_tps=-1, tps_standard_deviation=2,
            simulated_death_fraction=1/3, simulated_deaths=1, simulated_death_fraction_wilson_95={'lower': 0, 'upper': 1})
    if healing:
        value.update(mean_hps=20, hps_standard_deviation=1, mean_seconds_oom=1,
                     mean_iteration_duration_seconds=2, simulated_time_oom_fraction=0.5)
    if paired:
        for metric in ['dps'] + (['dtps', 'tps'] if tank else []) + (['hps'] if healing else []):
            value[f'_{metric}_samples'] = [1, 2, 3]
    return value


class SimulationJournalTests(unittest.TestCase):
    def test_posix_plaintext_payload_rejects_request_key_substitution(self):
        with tempfile.TemporaryDirectory() as folder, patch('tools.simulation_journal.user_protected',
                side_effect=lambda data, key, decrypt=False: data):
            journal = SimulationJournal(Path(folder) / 'journal')
            key = journal.key('a' * 64, request(), False, False, False)
            changed = request()
            changed['encounter']['duration'] = 91
            other = journal.key('a' * 64, changed, False, False, False)
            self.assertNotEqual(key, other)
            journal.put(key, summary('a' * 64))
            path = journal.root / 'checkpoints.sqlite3'
            with closing(sqlite3.connect(path)) as connection:
                connection.execute('INSERT INTO results SELECT ?,body,digest,plain_bytes FROM results WHERE key=?', (other, key))
                connection.commit()
            before = path.read_bytes()
            with self.assertRaises(ValueError): journal.get(other)
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(journal.get(key), summary('a' * 64))

    def test_simulation_cli_flag_reuses_and_import_entrypoints(self):
        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            engine = base / 'engine.exe'
            engine.write_bytes(b'synthetic engine')
            sha = hashlib.sha256(engine.read_bytes()).hexdigest()
            template, review, encounter = inputs()
            encounter['simulation_input']['simOptions']['iterations'] = 3
            for name, value in [('template', template), ('review', review), ('encounter', encounter)]:
                (base / f'{name}.json').write_text(json.dumps(value))
            argv = ['simulate', '--engine', str(engine), '--engine-sha256', sha,
                '--template', str(base / 'template.json'), '--equipment-review', str(base / 'review.json'),
                '--encounter-lookup', str(base / 'encounter.json'), '--simulation-journal', str(base / 'journal')]
            def launch(command, **kwargs):
                output = {'iterationsDone': 3, 'raidMetrics': {'dps': {'avg': 100, 'stdev': 2}}}
                Path(command[command.index('--outfile') + 1]).write_text(json.dumps(output))
            with patch('tools.run_forever_sim.subprocess.run', side_effect=launch) as execute:
                for status in ['miss', 'hit']:
                    output = io.StringIO()
                    with patch('sys.argv', argv), redirect_stdout(output): adapter.main()
                    self.assertEqual(json.loads(output.getvalue())['cache_status'], status)
                self.assertEqual(execute.call_count, 1)
            for command in [[sys.executable, '-m', 'tools.run_forever_sim', '--help'],
                            [sys.executable, 'tools/run_forever_sim.py', '--help'],
                            [sys.executable, '-m', 'tools.rank_forever_slot', '--help']]:
                result = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('--simulation-journal', result.stdout)
            result = subprocess.run([sys.executable, '-m', 'tools.run_forever_sim', *argv[1:], '--cache'],
                cwd=root, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 2)
            self.assertIn('not allowed with argument', result.stderr)

    def test_actual_adapter_reuses_checkpoint_across_instances_and_verifies_engine(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            engine = root / 'engine.exe'
            engine.write_bytes(b'synthetic engine')
            sha = hashlib.sha256(engine.read_bytes()).hexdigest()
            journal = SimulationJournal(root / 'journal')
            def launch(argv, **kwargs):
                output = {'iterationsDone': 3, 'raidMetrics': {'dps': {'avg': 100, 'stdev': 2, 'allValues': [99, 100, 101]}}}
                Path(argv[argv.index('--outfile') + 1]).write_text(json.dumps(output))
            with patch('tools.run_forever_sim.subprocess.run', side_effect=launch) as execute:
                first = run(engine, sha, request(), paired=True, cache=journal)
                second = run(engine, sha, request(), paired=True, cache=SimulationJournal(journal.root))
                self.assertEqual(execute.call_count, 1)
                self.assertEqual(first.pop('cache_status'), 'miss')
                self.assertEqual(second.pop('cache_status'), 'hit')
                self.assertEqual(first, second)
                engine.write_bytes(b'changed')
                with self.assertRaises(ValueError): run(engine, sha, request(), paired=True, cache=journal)
                self.assertEqual(execute.call_count, 1)
            data = (journal.root / 'checkpoints.sqlite3').read_bytes()
            self.assertNotIn(b'PRIVATE_CHARACTER', data)
            if os.name == 'nt': self.assertNotIn(b'mean_dps', data)

    def test_more_than_cache_limit_no_eviction_and_all_metric_modes(self):
        with tempfile.TemporaryDirectory() as folder:
            journal = SimulationJournal(Path(folder) / 'journal')
            for i in range(130):
                seed = str(i + 1)
                key = journal.key('a' * 64, request(seed), False, False, False)
                journal.put(key, summary('a' * 64, seed))
            restarted = SimulationJournal(journal.root)
            for i in range(130):
                seed = str(i + 1)
                key = restarted.key('a' * 64, request(seed), False, False, False)
                self.assertEqual(restarted.get(key), summary('a' * 64, seed))
            for paired, tank, healing in product([False, True], repeat=3):
                key = journal.key('b' * 64, request(), paired, tank, healing)
                value = summary('b' * 64, paired=paired, tank=tank, healing=healing)
                journal.put(key, value)
                self.assertEqual(journal.get(key), value)

    def test_quota_conflict_corruption_and_foreign_database_reject_without_eviction(self):
        with tempfile.TemporaryDirectory() as folder:
            journal = SimulationJournal(Path(folder) / 'journal', max_entries=1)
            key = journal.key('a' * 64, request(), False, False, False)
            value = summary('a' * 64)
            journal.put(key, value)
            path = journal.root / 'checkpoints.sqlite3'
            before = path.read_bytes()
            journal.put(key, value)
            self.assertEqual(path.read_bytes(), before)
            other = journal.key('a' * 64, request('11'), False, False, False)
            with self.assertRaises(ValueError): journal.put(other, summary('a' * 64, '11'))
            with self.assertRaises(ValueError): journal.put(key, {**value, 'mean_dps': 101})
            limited = SimulationJournal(journal.root, max_payload_bytes=1)
            next_key = limited.key('a' * 64, request('12'), False, False, False)
            with self.assertRaises(ValueError): limited.put(next_key, summary('a' * 64, '12'))
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(journal.get(key), value)
            with closing(sqlite3.connect(path)) as connection:
                connection.execute('UPDATE results SET body=zeroblob(?),plain_bytes=1', (1024 * 1024,))
                connection.commit()
            corrupt = path.read_bytes()
            with patch.object(journal, 'read_row', wraps=journal.read_row) as read:
                with self.assertRaises(sqlite3.DataError): journal.get(key)
                self.assertEqual(read.call_count, 0)
            self.assertEqual(path.read_bytes(), corrupt)
            foreign = SimulationJournal(Path(folder) / 'foreign')
            foreign.root.mkdir(mode=0o700)
            foreign_path = foreign.root / 'checkpoints.sqlite3'
            foreign_path.write_bytes(b'PRIVATE foreign file')
            foreign_key = foreign.key('a' * 64, request(), False, False, False)
            with self.assertRaises(ValueError): foreign.get(foreign_key)
            self.assertEqual(foreign_path.read_bytes(), b'PRIVATE foreign file')

    def test_summary_whitelist_and_key_substitution(self):
        with tempfile.TemporaryDirectory() as folder:
            journal = SimulationJournal(Path(folder) / 'journal')
            key = journal.key('a' * 64, request(), True, True, True)
            value = summary('a' * 64, paired=True, tank=True, healing=True)
            journal.put(key, value)
            before = (journal.root / 'checkpoints.sqlite3').read_bytes()
            bad_values = [{**value, 'PRIVATE': 'PRIVATE'}, {**value, 'iterations': True},
                {**value, 'seed': '11'}, {**value, 'engine_sha256': 'b' * 64},
                {**value, 'mean_dps': float('nan')}, {**value, '_dps_samples': [1, True, 3]},
                {**value, '_hps_samples': [1, 2]}, {**value, 'simulated_deaths': 4},
                {**value, 'simulated_death_fraction_wilson_95': {'lower': False, 'upper': 1}}]
            for bad in bad_values:
                with self.assertRaises(ValueError): journal.put(key, bad)
                self.assertEqual((journal.root / 'checkpoints.sqlite3').read_bytes(), before)
            self.assertEqual(journal.get(key), value)
            if os.name == 'nt':
                other_request = deepcopy(request())
                other_request['encounter']['duration'] = 91
                other = journal.key('a' * 64, other_request, True, True, True)
                path = journal.root / 'checkpoints.sqlite3'
                with closing(sqlite3.connect(path)) as connection:
                    connection.execute('INSERT INTO results SELECT ?,body,digest,plain_bytes FROM results WHERE key=?', (other, key))
                    connection.commit()
                with self.assertRaises(OSError): journal.get(other)
