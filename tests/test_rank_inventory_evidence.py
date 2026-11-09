"""Rank collection and CLI retain the saved inventory's evidence boundaries."""
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from mcp import ClientSession
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.rank_forever_slot import collect, main


class RankInventoryEvidenceTests(unittest.IsolatedAsyncioTestCase):
    async def test_collection_plan_cli_legacy_and_changed_evidence(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned local catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            sample = fixture()
            sample['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            sample['sections']['bags'] = {'0': {'1': {'item_id': 4549, 'count': 1, 'link': 'item:4549:0:0:0:0:0:0'},
                                               '2': {'item_id': 4294967295, 'count': 9},
                                               '3': {'item_id': 4549, 'count': 1, 'link': 'item:4549'}}}
            key = SnapshotStore(Path(folder)).ingest(sample)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            with patch.dict(os.environ, env), tempfile.TemporaryFile(mode='w+') as errlog:
                result = await collect(binary, key, ['finger1', 'finger2'], errlog,
                                       collapse_identical=True, item_ids=[4549], storage='bags', max_swaps=1)
                drafts, _, coverage, _, _, _, search = result
                self.assertEqual(len(result), 7)
                self.assertEqual(search['snapshot_id'], key)
                self.assertEqual(search['requested_slots'], ['finger1', 'finger2'])
                self.assertEqual(search['candidate_filters'], {'item_ids': [4549], 'storage': 'bags'})
                self.assertEqual(search['maximum_simultaneous_swaps'], 1)
                self.assertEqual(search['unresolved_candidate_locations_by_slot'], {'finger1': 1, 'finger2': 1})
                evidence = search['inventory_evidence']
                self.assertEqual(evidence['catalogue_unmatched_item_ids'], [4294967295])
                self.assertEqual(evidence['unavailable_storage_sections'], ['character_bank', 'account_bank'])
                self.assertFalse(evidence['best_in_slot_verified'])
                self.assertEqual(coverage['character_bank']['status'], 'unavailable')
                self.assertGreater(len(drafts), 1)

                original = ClientSession.call_tool
                async def legacy(session, name, *args, **kwargs):
                    reply = await original(session, name, *args, **kwargs)
                    if name == 'review_forever_owned_gear':
                        data = json.loads(reply.content[0].text)
                        data.pop('inventory_evidence', None)
                        reply.content[0].text = json.dumps(data)
                    return reply
                with patch('tools.rank_forever_slot.ClientSession.call_tool', legacy):
                    old = await collect(binary, key, ['finger1'], errlog)
                    self.assertIsNone(old[-1]['inventory_evidence'])
                    self.assertEqual(old[-1]['snapshot_id'], key)

                seen = 0
                async def changed(session, name, *args, **kwargs):
                    nonlocal seen
                    reply = await original(session, name, *args, **kwargs)
                    if name == 'review_forever_owned_gear':
                        seen += 1
                        if seen == 2:
                            data = json.loads(reply.content[0].text)
                            data['inventory_evidence']['catalogue_unmatched_item_locations'] += 1
                            reply.content[0].text = json.dumps(data)
                    return reply
                with patch('tools.rank_forever_slot.ClientSession.call_tool', changed):
                    try:
                        await collect(binary, key, ['finger1', 'finger2'], errlog)
                    except Exception as error:
                        while isinstance(error, ExceptionGroup) and len(error.exceptions) == 1:
                            error = error.exceptions[0]
                        self.assertIsInstance(error, ValueError)
                        self.assertEqual(str(error), 'Local MCP returned inconsistent inventory evidence')
                    else:
                        self.fail('Changed evidence must reject the collection')

            template, _, _ = inputs()
            template_path = Path(folder) / 'template.json'
            template_path.write_text(json.dumps(template))
            command = [sys.executable, '-m', 'tools.rank_forever_slot', '--mcp', str(binary),
                       '--snapshot-id', key, '--slot', 'finger1', '--template', str(template_path),
                       '--item-id', '4549', '--plan-only']
            run = subprocess.run(command, env=env, capture_output=True, text=True, timeout=20)
            self.assertEqual(run.returncode, 0, run.stderr)
            report = json.loads(run.stdout)
            self.assertEqual(report['mode'], 'plan_only')
            self.assertEqual(report['gear_search']['inventory_evidence'], evidence)
            self.assertEqual(report['gear_search']['snapshot_id'], key)
            self.assertEqual(report['gear_search']['unresolved_candidate_locations_by_slot'], {'finger1': 1})
            self.assertFalse(report['best_in_slot_verified'])

    def test_single_and_bulk_report_keep_evidence_and_deny_absolute_bis(self):
        template, review, encounter = inputs()
        search = {'snapshot_id': 'synthetic', 'inventory_evidence': None}
        async def collected(*args, **kwargs):
            return [([], review)], [], {}, {'level': 60}, None, None, search
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            binary = root / 'must-not-execute'
            binary.touch()
            for name, value in [('template', template), ('encounter', encounter)]:
                (root / f'{name}.json').write_text(json.dumps(value))
            argv = ['rank', '--mcp', str(binary), '--snapshot-id', 'synthetic', '--slot', 'head',
                    '--template', str(root / 'template.json'), '--engine', str(binary),
                    '--engine-sha256', 'a' * 64, '--encounter-lookup', str(root / 'encounter.json')]
            for extra in [[], ['--bulk-engine', str(binary), '--bulk-engine-sha256', 'b' * 64]]:
                with patch('sys.argv', argv + extra), patch('tools.rank_forever_slot.collect', collected), \
                        patch('tools.rank_forever_slot.rank_encounter', return_value={'best_in_slot_verified': True}) as run, \
                        redirect_stdout(io.StringIO()) as output:
                    main()
                run.assert_called_once()
                report = json.loads(output.getvalue())
                self.assertEqual(report['gear_search'], search)
                self.assertIsNone(report['gear_search']['inventory_evidence'])
                self.assertFalse(report['best_in_slot_verified'])


if __name__ == '__main__':
    unittest.main()
