"""Owned review coverage describes the unfiltered capture, never complete BiS."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class OwnedInventoryEvidenceTests(unittest.IsolatedAsyncioTestCase):
    async def test_unfiltered_coverage_missing_empty_and_bounded_ids(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned local catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            base = fixture()
            base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            base['sections']['equipment'] = {'11': {'item_id': 4549}}
            base['sections']['bags'] = {'0': {'1': {'item_id': 4294967295, 'count': 7},
                                               '2': {'item_id': 4294967295, 'count': 1}}}
            partial = store.ingest(base)
            complete_sections = deepcopy(base)
            complete_sections['sections'].update(character_bank={}, account_bank={})
            all_sections = store.ingest(complete_sections)
            empty = deepcopy(complete_sections)
            empty['sections'].update(equipment={}, bags={})
            empty_id = store.ingest(empty)
            many = deepcopy(complete_sections)
            many['sections']['bags'] = {'0': {str(i + 1): {'item_id': 4294967200 + i, 'count': 1}
                                             for i in range(52)}}
            many_id = store.ingest(many)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder,
                   'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def review(key, **filters):
                        response = await session.call_tool('review_forever_owned_gear', {'snapshot_id': key, **filters})
                        self.assertFalse(response.isError, response)
                        return json.loads(response.content[0].text)
                    result = await review(partial)
                    evidence = result['inventory_evidence']
                    self.assertEqual(evidence['observed_storage_sections'], ['equipment', 'bags'])
                    self.assertEqual(evidence['unavailable_storage_sections'], ['character_bank', 'account_bank'])
                    self.assertFalse(evidence['all_storage_sections_observed'])
                    self.assertEqual(evidence['catalogue_unmatched_item_locations'], 2)
                    self.assertEqual(evidence['catalogue_unmatched_distinct_item_ids'], 1)
                    self.assertEqual(evidence['catalogue_unmatched_item_ids'], [4294967295])
                    self.assertFalse(evidence['catalogue_unmatched_item_ids_truncated'])
                    for filters in [{'item_id': 4549}, {'storage': 'equipment'}, {'simulator_slot': 'finger1'},
                                    {'offset': 99, 'limit': 1}]:
                        self.assertEqual((await review(partial, **filters))['inventory_evidence'], evidence)
                    for key in [all_sections, empty_id]:
                        data = (await review(key))['inventory_evidence']
                        self.assertTrue(data['all_storage_sections_observed'])
                        self.assertEqual(data['unavailable_storage_sections'], [])
                        for flag in ['complete_ownership_verified', 'complete_simulator_resolution_verified', 'best_in_slot_verified']:
                            self.assertFalse(data[flag])
                    self.assertEqual((await review(empty_id))['inventory_evidence']['catalogue_unmatched_item_ids'], [])
                    bounded = (await review(many_id, item_id=4549))['inventory_evidence']
                    self.assertEqual(bounded['catalogue_unmatched_distinct_item_ids'], 52)
                    self.assertEqual(bounded['catalogue_unmatched_item_locations'], 52)
                    self.assertEqual(bounded['catalogue_unmatched_item_ids'], list(range(4294967200, 4294967250)))
                    self.assertTrue(bounded['catalogue_unmatched_item_ids_truncated'])


if __name__ == '__main__':
    unittest.main()
