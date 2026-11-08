"""Retain numeric gear inputs without treating them as qualified simulation jobs."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from shared.snapshots import SnapshotStore
from test_merge_gear_windows import report
from test_simulator_level import inputs
from test_snapshots import fixture
from tools.journal_forever_gear_windows import append_report, journal_window
from tools.merge_forever_gear_windows import merge_reports
from tools.rank_forever_slot import plan_report


class RetainedGearInputTests(unittest.TestCase):
    def test_strict_numeric_inputs_hashes_and_legacy_absence(self):
        empty = {'items': [{} for _ in range(17)]}
        equipped = deepcopy(empty)
        equipped['items'][10] = {'id': 4549, 'enchant': 0, 'randomSuffix': -1}
        def sha(spec):
            return hashlib.sha256(json.dumps(spec, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        value = report(0, [sha(empty), sha(equipped)])
        for row, spec in zip(value['resolved_drafts'], [empty, equipped], strict=True):
            row['simulator_equipment_spec'] = spec
        result = merge_reports([value])
        self.assertTrue(result['equipment_specs_retained'])
        self.assertEqual(result['drafts'][0]['simulator_equipment_spec'], empty)
        self.assertEqual(result['drafts'][1]['simulator_equipment_spec'], equipped)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'PRIVATE_numeric.sqlite'
            append_report(path, value)
            before = path.read_bytes()
            self.assertNotIn('equipment_specs', journal_window(path, 0))
            exported = journal_window(path, 0, True)
            self.assertTrue(exported['equipment_specs_available'])
            self.assertEqual(exported['equipment_specs'][1]['simulator_equipment_spec'], equipped)
            self.assertNotIn('PRIVATE', json.dumps(exported))
            for mutate in ['hash', 'name', 'bool', 'null', 'missing', 'length', 'partial', 'large', 'suffix']:
                bad = deepcopy(value)
                spec = bad['resolved_drafts'][1]['simulator_equipment_spec']
                if mutate == 'hash': spec['items'][10]['id'] = 4550
                if mutate == 'name': spec['items'][10]['PRIVATE_NAME'] = 'PRIVATE'
                if mutate == 'bool': spec['items'][10]['id'] = True
                if mutate == 'null': bad['resolved_drafts'][1]['simulator_equipment_spec'] = None
                if mutate == 'missing': del bad['resolved_drafts'][1]['simulator_equipment_spec']
                if mutate == 'length': spec['items'].pop()
                if mutate == 'partial': del spec['items'][10]['enchant']
                if mutate == 'large': spec['items'][10]['id'] = 2**31
                if mutate == 'suffix': spec['items'][10]['randomSuffix'] = -(2**31) - 1
                with self.assertRaises(ValueError): append_report(path, bad)
                self.assertEqual(path.read_bytes(), before)
            legacy = Path(folder) / 'legacy.sqlite'
            append_report(legacy, report(0, ['b' * 64]))
            old = journal_window(legacy, 0, True)
            self.assertFalse(old['equipment_specs_available'])
            self.assertIsNone(old['equipment_specs'])
            with self.assertRaises(ValueError): merge_reports([value, report(4, [sha(empty)])])
        review = {'checks': {'simulator_equipment_spec': empty}}
        self.assertNotIn('simulator_equipment_spec', plan_report([([], review)], True)['resolved_drafts'][0])
        with self.assertRaises(ValueError): plan_report([([], review)], False, True)

    def test_actual_collector_retains_and_exports_explicit_inputs(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file(): self.skipTest('Requires pinned local catalogue; no download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            value = fixture()
            value['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            value['sections']['bags'] = {'0': {'1': {'item_id': 4549, 'count': 1, 'link': 'item:4549:0:0:0:0:0:0'}}}
            key = SnapshotStore(Path(folder)).ingest(value)
            template, _, _ = inputs()
            template_path = Path(folder) / 'PRIVATE_template.json'
            template_path.write_text(json.dumps(template))
            journal = Path(folder) / 'PRIVATE_retained.sqlite'
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            argv = [sys.executable, '-m', 'tools.collect_forever_gear_windows', '--mcp', str(binary),
                '--snapshot-id', key, '--slot', 'finger1', '--template', str(template_path),
                '--journal', str(journal), '--max-windows', '1', '--window-size', '1']
            def launch(command, code=0):
                process = subprocess.run(command, cwd=root, env=env, capture_output=True, text=True, timeout=60)
                self.assertEqual(process.returncode, code, process.stderr)
                self.assertNotIn('PRIVATE', process.stdout + process.stderr)
                return json.loads(process.stdout) if code == 0 else None
            launch([*argv, '--retain-equipment-specs'])
            before = journal.read_bytes()
            launch(argv, 1)
            self.assertEqual(journal.read_bytes(), before)
            complete = launch([*argv, '--retain-equipment-specs'])
            self.assertTrue(complete['progress']['reported_coordinate_coverage_complete'])
            self.assertTrue(complete['progress']['equipment_specs_retained'])
            export = launch([sys.executable, '-m', 'tools.journal_forever_gear_windows', '--journal', str(journal),
                '--window-offset', '1', '--include-equipment-specs'])
            self.assertTrue(export['equipment_specs_available'])
            specs = export['equipment_specs']
            self.assertEqual(specs[1]['simulator_equipment_spec']['items'][10]['id'], 4549)
            self.assertFalse(export['simulation_performed'])
            self.assertFalse(export['best_in_slot_verified'])
            before = journal.read_bytes()
            launch([sys.executable, '-m', 'tools.journal_forever_gear_windows', '--journal', str(journal),
                    '--include-equipment-specs'], 1)
            self.assertEqual(journal.read_bytes(), before)
