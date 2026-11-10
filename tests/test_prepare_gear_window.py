"""Revalidate saved plans before preparing requests; never invoke an engine."""
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
from tools.journal_forever_gear_windows import append_report
from tools.rank_forever_slot import collect, plan_report
from tools import prepare_forever_gear_window as preparer


class PrepareGearWindowTests(unittest.TestCase):
    def test_consumed_input_bytes_match_fingerprint(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'input.json'
            path.write_bytes(b'{"version":1}')
            expected = hashlib.sha256(path.read_bytes()).hexdigest()
            self.assertEqual(preparer.read_input(path, expected), {'version': 1})
            path.write_bytes(b'{"version":2}')
            with self.assertRaises(ValueError): preparer.read_input(path, expected)

    def test_actual_cli_revalidates_retained_inputs_and_preserves_guards(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file(): self.skipTest('Requires pinned local catalogue; no download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryFile(mode='w+') as errlog:
            saved = fixture()
            saved['character']['name'] = 'PRIVATE_CHARACTER'
            saved['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            saved['sections']['stats'] = {'level': 60, 'class': 'HUNTER', 'race': 'Orc', 'client_version': 'x'}
            saved['sections']['bags'] = {'0': {'1': {'item_id': 4549, 'count': 1, 'link': 'item:4549:0:0:0:0:0:0'}}}
            store = SnapshotStore(Path(folder))
            key = store.ingest(saved)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            with patch.dict(os.environ, env):
                drafts, skipped, coverage, _, _, _, search = asyncio.run(collect(binary, key, ['finger1'], errlog,
                    combination_limit=4, collapse_identical=True))
            artifact = plan_report(drafts, True, True)
            artifact.update(gear_search=search, skipped=skipped, storage_coverage=coverage, best_in_slot_verified=False)
            journal = Path(folder) / 'PRIVATE_journal.sqlite'
            append_report(journal, artifact)
            before = journal.read_bytes()
            template, _, encounter = inputs()
            template['raid']['parties'][0]['players'][0]['name'] = 'PRIVATE_TEMPLATE'
            template_path, encounter_path = Path(folder) / 'PRIVATE_template.json', Path(folder) / 'PRIVATE_encounter.json'
            template_path.write_text(json.dumps(template))
            encounter_path.write_text(json.dumps(encounter))
            argv = [sys.executable, '-m', 'tools.prepare_forever_gear_window', '--mcp', str(binary),
                '--journal', str(journal), '--window-offset', '0', '--template', str(template_path),
                '--encounter-lookup', str(encounter_path)]
            def launch(extra=(), code=0):
                process = subprocess.run([*argv, *extra], cwd=root, env=env, capture_output=True, text=True, timeout=60)
                self.assertEqual(process.returncode, code, process.stderr)
                self.assertNotIn('PRIVATE', process.stdout + process.stderr)
                self.assertEqual(journal.read_bytes(), before)
                return json.loads(process.stdout) if code == 0 else None
            result = launch()
            self.assertEqual(result['talent_identity_sha256'],
                hashlib.sha256(Path(preparer.talents.__file__).read_bytes()).hexdigest())
            self.assertEqual(result['talent_map_sha256'],preparer.talents.MAP_SHA256)
            self.assertEqual(len(result['requests']), len(drafts))
            self.assertEqual(result['snapshot_id'], key)
            for row, (_, draft) in zip(result['requests'], drafts, strict=True):
                request = row['request']
                player = request['raid']['parties'][0]['players'][0]
                self.assertEqual(player['name'], 'Player')
                self.assertEqual(player['equipment'], draft['checks']['simulator_equipment_spec'])
                self.assertEqual(row['request_sha256'], hashlib.sha256(preparer.encoded(request)).hexdigest())
            self.assertFalse(result['simulation_performed'])
            self.assertFalse(result['best_in_slot_verified'])
            for extra in [['--window-offset', '1'], ['--time-budget-seconds', '0'], ['--time-budget-seconds', '46']]:
                launch(extra, 1)
            original_template = template_path.read_bytes()
            template['raid']['parties'][0]['players'][0]['race'] = 'RaceHuman'
            template_path.write_text(json.dumps(template))
            launch(code=1)
            template_path.write_bytes(original_template)
            low = deepcopy(saved)
            low['sections']['stats']['level'] = 20
            low_key = store.ingest(low)
            low_report = deepcopy(artifact)
            low_report['gear_search']['snapshot_id'] = low_key
            low_journal = Path(folder) / 'low.sqlite'
            append_report(low_journal, low_report)
            launch(['--journal', str(low_journal)], 1)
            mismatched = deepcopy(artifact)
            mismatched['resolved_drafts'][1]['selections'][0]['expected_item_id'] = 4550
            wrong_journal = Path(folder) / 'wrong.sqlite'
            append_report(wrong_journal, mismatched)
            launch(['--journal', str(wrong_journal)], 1)
            projection = deepcopy(artifact)
            for row in projection['resolved_drafts']:
                del row['projected_set_changes']
            without_projection = Path(folder) / 'without-projection.sqlite'
            append_report(without_projection, projection)
            self.assertEqual(launch(['--journal', str(without_projection)])['requests'], result['requests'])
            projection['resolved_drafts'][0]['projected_set_changes'] = {
                **artifact['resolved_drafts'][0]['projected_set_changes'],
                'before_uncatalogued_equipped_slots': False,
            }
            wrong_projection = Path(folder) / 'wrong-projection.sqlite'
            append_report(wrong_projection, projection)
            launch(['--journal', str(wrong_projection)], 1)
            for index,cleared in enumerate([[17],[17.0],None]):
                wrong=deepcopy(artifact)
                wrong['resolved_drafts'][0]['cleared_equipment_slots']=cleared
                wrong_path=Path(folder)/f'cleared-{index}.sqlite'
                append_report(wrong_path,wrong)
                launch(['--journal',str(wrong_path)],1)
            legacy = deepcopy(artifact)
            for row in legacy['resolved_drafts']: del row['simulator_equipment_spec']
            legacy_journal = Path(folder) / 'legacy.sqlite'
            append_report(legacy_journal, legacy)
            launch(['--journal', str(legacy_journal)], 1)
            armed=deepcopy(saved)
            armed['sections']['equipment']={str(slot):{'item_id':754,'link':'item:754:0:0:0:0:0:0'} for slot in [16,17]}
            armed['sections']['bags']={'0':{'1':{'item_id':2042,'count':1,'link':'item:2042:0:0:0:0:0:0'}}}
            armed_key=store.ingest(armed)
            with patch.dict(os.environ,env):
                drafts,skipped,coverage,_,_,_,search=asyncio.run(collect(binary,armed_key,['main_hand'],errlog,
                    combination_limit=4,collapse_identical=True))
            clear_report=plan_report(drafts,True,True)
            clear_report.update(gear_search=search,skipped=skipped,storage_coverage=coverage,best_in_slot_verified=False)
            clearing=[row for row in clear_report['resolved_drafts'] if row['cleared_equipment_slots']]
            self.assertEqual(len(clearing),1)
            clear_path=Path(folder)/'clear-valid.sqlite'; append_report(clear_path,clear_report)
            launch(['--journal',str(clear_path)])
            clearing[0]['cleared_equipment_slots']=[]
            omitted_path=Path(folder)/'clear-omitted.sqlite'; append_report(omitted_path,clear_report)
            launch(['--journal',str(omitted_path)],1)

    def test_saved_selection_shapes_reject_extra_fields_and_booleans(self):
        valid = {'target_slot': 11, 'source': {'kind': 'bag', 'bag': 0, 'slot': 1}, 'expected_item_id': 4549}
        preparer.selections_for_review([valid])
        for row in [{**valid, 'PRIVATE': True}, {**valid, 'target_slot': True},
                    {**valid, 'expected_item_id': 0}, {**valid, 'source': {'kind': 'unknown'}},
                    {**valid, 'source': {'kind': 'bag', 'bag': 0, 'slot': 1, 'PRIVATE': True}}]:
            with self.assertRaises(ValueError): preparer.selections_for_review([row])
