"""Deduplicate prepared requests only against identical saved gear and context."""
import asyncio
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from test_merge_gear_windows import report
from tools.journal_forever_gear_windows import append_report, encoded as report_bytes
from tools.merge_forever_gear_windows import merge_reports
from tools.merge_forever_prepared_windows import FINGERPRINTS, merge_prepared
from tools import merge_forever_prepared_windows as merger
from tools.prepare_forever_gear_window import encoded
from tools.rank_forever_slot import collect, plan_report
from tools.run_forever_sim import request_from


class MergePreparedWindowTests(unittest.TestCase):
    def test_context_hash_source_row_and_count_rejections(self):
        self.assertIn('talent_identity_sha256', FINGERPRINTS)
        self.assertIn('talent_map_sha256', FINGERPRINTS)
        template, review, encounter = inputs()
        specs = [deepcopy(review['checks']['simulator_equipment_spec']) for _ in range(2)]
        specs[1]['items'][10] = {'id': 4549, 'enchant': 0, 'randomSuffix': 0}
        keys = [hashlib.sha256(encoded(spec)).hexdigest() for spec in specs]
        pages = [report(0, keys), report(4, keys), report(8, keys[:1])]
        artifacts = []
        with tempfile.TemporaryDirectory() as folder:
            journal = Path(folder) / 'PRIVATE.sqlite'
            for page in pages:
                page['PRIVATE_PADDING'] = 'x' * 10000
                for row, spec in zip(page['resolved_drafts'], specs): row['simulator_equipment_spec'] = spec
                append_report(journal, page)
                requests = []
                for row in page['resolved_drafts']:
                    request = request_from(template, {**review, 'checks': {'simulator_equipment_spec': row['simulator_equipment_spec']}}, encounter)
                    requests.append({'review': row['review'], 'simulator_gear_sha256': row['simulator_gear_sha256'],
                                     'request_sha256': hashlib.sha256(encoded(request)).hexdigest(), 'request': request})
                artifacts.append({'mode': 'prepared_gear_window_requests', 'snapshot_id': page['gear_search']['snapshot_id'],
                    'scope_sha256': page['gear_search']['combination_window']['scope_sha256'],
                    'source_report_sha256': hashlib.sha256(report_bytes(page)).hexdigest(),
                    'window_offset_decimal': str(page['gear_search']['combination_window']['offset']),
                    'resolution_accounting': merge_reports([page])['resolution_accounting'],
                    **{field: 'a' * 64 for field in FINGERPRINTS}, 'requests': requests,
                    **{field: False for field in ['simulation_performed', 'game_execution',
                        'best_in_slot_verified', 'feasible_set_coverage_verified']}})
            before = journal.read_bytes()
            for field in ['talent_identity_sha256', 'talent_map_sha256']:
                bad=deepcopy(artifacts);bad[1][field]='b'*64
                with self.assertRaises(ValueError):merge_prepared(journal,bad)
            merged = merge_prepared(journal, artifacts)
            self.assertEqual((merged['request_instances'], merged['distinct_requests'], merged['reused_request_instances']), (5, 2, 3))
            self.assertTrue(merged['search_coverage']['reported_coordinate_coverage_complete'])
            self.assertEqual([len(row['references']) for row in merged['requests']], [3, 2])
            self.assertEqual(merged['hunter_quiver_observation']['comparison'],'capture_unavailable')
            with_bags=deepcopy(artifacts)
            for artifact in with_bags:
                artifact['captured_equipped_bag_items']=[{'bag':1,'item_id':8217,'inventory_slot':20}]
            diagnosed=merge_prepared(journal,with_bags)
            self.assertEqual(diagnosed['hunter_quiver_observation']['observed_pinned_quiver_item_ids'],[8217])
            self.assertIs(diagnosed['hunter_quiver_observation']['template_haste_below_observed_reference'],True)
            self.assertEqual(diagnosed['requests'],merged['requests'])
            reverse = merge_prepared(journal, list(reversed(artifacts)))
            self.assertEqual([row['request_sha256'] for row in merged['requests']], [row['request_sha256'] for row in reverse['requests']])
            mutations = ['context', 'fingerprint', 'equipment', 'request_hash', 'source_hash', 'missing',
                         'review', 'accounting', 'offset', 'flag', 'bag_identity_type',
                         'bag_identity_mismatch', 'quiver_enum', 'ammo_enum']
            for mutation in mutations:
                bad = deepcopy(artifacts)
                target = bad[1]
                row = target['requests'][1]
                if mutation == 'context':
                    row['request']['encounter']['duration'] += 1
                    row['request_sha256'] = hashlib.sha256(encoded(row['request'])).hexdigest()
                if mutation == 'fingerprint': target['template_sha256'] = 'b' * 64
                if mutation == 'equipment':
                    row['request']['raid']['parties'][0]['players'][0]['equipment']['items'][10]['id'] = 4550
                    row['request_sha256'] = hashlib.sha256(encoded(row['request'])).hexdigest()
                if mutation == 'request_hash': row['request_sha256'] = '0' * 64
                if mutation == 'source_hash': target['source_report_sha256'] = '0' * 64
                if mutation == 'missing': target['requests'].pop()
                if mutation == 'review': row['review'] = True
                if mutation == 'accounting': target['resolution_accounting']['baseline_review_instances'] = True
                if mutation == 'offset': target['window_offset_decimal'] = '04'
                if mutation == 'flag': target['simulation_performed'] = True
                if mutation == 'bag_identity_type': target['captured_equipped_bag_items']='PRIVATE'
                if mutation == 'bag_identity_mismatch': target['captured_equipped_bag_items']=[{'bag':1,'item_id':8217,'inventory_slot':20}]
                if mutation == 'quiver_enum':
                    row['request']['raid']['parties'][0]['players'][0]['hunter']['options']={'classOptions':{'quiverBonus':999999}}
                    row['request_sha256']=hashlib.sha256(encoded(row['request'])).hexdigest()
                if mutation == 'ammo_enum':
                    row['request']['raid']['parties'][0]['players'][0]['hunter']['options']={'classOptions':{'ammo':999999}}
                    row['request_sha256']=hashlib.sha256(encoded(row['request'])).hexdigest()
                if mutation == 'ammo_enum':
                    with self.assertRaisesRegex(ValueError,'Unsupported Hunter ammunition selection'):
                        merge_prepared(journal,bad)
                else:
                    with self.assertRaises(ValueError, msg=mutation): merge_prepared(journal, bad)
                self.assertEqual(journal.read_bytes(), before)
            with self.assertRaises(ValueError): merge_prepared(journal, artifacts * 2)
            partial = merge_prepared(journal, artifacts[:1])
            self.assertFalse(partial['search_coverage']['reported_coordinate_coverage_complete'])
            self.assertFalse(merged['best_in_slot_verified'])
            self.assertEqual(journal.read_bytes(), before)
            self.assertNotIn('PRIVATE', json.dumps(merged))
            self.assertLess(sum(len(encoded(artifact)) for artifact in artifacts), 20000)
            self.assertGreater(sum(len(report_bytes(page)) for page in pages), 20000)
            with patch.object(merger, 'MAX_TOTAL_BYTES', 20000):
                with self.assertRaises(ValueError): merge_prepared(journal, artifacts)
            self.assertEqual(journal.read_bytes(), before)

    def test_actual_collection_preparation_and_merge_cli(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file(): self.skipTest('Requires pinned local catalogue; no download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryFile(mode='w+') as errlog:
            saved = fixture()
            saved['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            saved['sections']['stats'] = {'level': 60, 'class': 'HUNTER', 'race': 'Orc', 'client_version': 'x'}
            saved['sections']['bags'] = {'0': {str(i): {'item_id': 4549, 'count': 1,
                'link': 'item:4549:0:0:0:0:0:0'} for i in [1, 2]}}
            key = SnapshotStore(Path(folder)).ingest(saved)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            journal = Path(folder) / 'PRIVATE_progress.sqlite'
            template, _, encounter = inputs()
            template_path, encounter_path = Path(folder) / 'template.json', Path(folder) / 'encounter.json'
            template_path.write_text(json.dumps(template)); encounter_path.write_text(json.dumps(encounter))
            paths = []
            for offset in range(3):
                with patch.dict(os.environ, env):
                    drafts, skipped, coverage, _, _, _, search = asyncio.run(collect(binary, key, ['finger1'], errlog,
                        combination_offset=offset, combination_limit=1))
                page = plan_report(drafts, True, True)
                page.update(gear_search=search, skipped=skipped, storage_coverage=coverage, best_in_slot_verified=False)
                append_report(journal, page)
                process = subprocess.run([sys.executable, '-m', 'tools.prepare_forever_gear_window', '--mcp', str(binary),
                    '--journal', str(journal), '--window-offset', str(offset), '--template', str(template_path),
                    '--encounter-lookup', str(encounter_path)], cwd=root, env=env, capture_output=True, text=True, timeout=60)
                self.assertEqual(process.returncode, 0, process.stderr)
                path = Path(folder) / f'PRIVATE_prepared_{offset}.json'
                path.write_text(process.stdout)
                paths.append(str(path))
            before = journal.read_bytes()
            process = subprocess.run([sys.executable, '-m', 'tools.merge_forever_prepared_windows',
                '--journal', str(journal), *paths], cwd=root, env=env, capture_output=True, text=True, timeout=60)
            self.assertEqual(process.returncode, 0, process.stderr)
            result = json.loads(process.stdout)
            self.assertEqual((result['request_instances'], result['distinct_requests'], result['reused_request_instances']), (5, 2, 3))
            self.assertEqual(result['search_coverage']['covered_coordinate_count'], 3)
            self.assertTrue(result['search_coverage']['reported_coordinate_coverage_complete'])
            self.assertFalse(result['simulation_performed'])
            self.assertNotIn('PRIVATE', process.stdout + process.stderr)
            self.assertEqual(journal.read_bytes(), before)
