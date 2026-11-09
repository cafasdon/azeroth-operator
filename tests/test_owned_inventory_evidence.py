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
            mixed = deepcopy(base)
            mixed['sections']['bags']['0']['3'] = {'item_id': 4549, 'count': 1, 'link': 'item:4549:0:0:0:0:0:0'}
            mixed_id = store.ingest(mixed)
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
                    mixed_evidence = (await review(mixed_id))['inventory_evidence']
                    resolved = await review(mixed_id, simulator_input_resolved=True, limit=1)
                    self.assertEqual(resolved['filtered_item_locations'], 1)
                    self.assertEqual(resolved['items'][0]['simulator_item_resolution'], 'resolved')
                    self.assertFalse(resolved['has_more'])
                    unresolved = await review(mixed_id, simulator_input_resolved=False, offset=1, limit=1)
                    self.assertEqual(unresolved['filtered_item_locations'], 3)
                    self.assertEqual(unresolved['items'][0]['simulator_item_resolution'], 'catalogue_item_unavailable')
                    self.assertTrue(unresolved['has_more'])
                    past_end = await review(mixed_id, simulator_input_resolved=False, offset=3, limit=1)
                    self.assertEqual(past_end['filtered_item_locations'], 3)
                    self.assertEqual(past_end['items'], [])
                    self.assertFalse(past_end['has_more'])
                    rings = await review(mixed_id, simulator_input_resolved=False, simulator_slot='finger1')
                    self.assertEqual(rings['filtered_item_locations'], 1)
                    self.assertEqual(rings['items'][0]['simulator_item_resolution'], 'item_link_unresolved')
                    for filtered in [resolved, unresolved, past_end, rings]:
                        self.assertEqual(filtered['inventory_evidence'], mixed_evidence)
                    unmatched = await review(mixed_id, catalogue_matched=False, limit=1)
                    self.assertEqual(unmatched['filtered_item_locations'], 2)
                    self.assertTrue(unmatched['has_more'])
                    self.assertEqual(unmatched['items'][0]['simulator_item_resolution'], 'catalogue_item_unavailable')
                    tail = await review(mixed_id, catalogue_matched=False, offset=1, limit=1, storage='bags')
                    self.assertEqual(tail['filtered_item_locations'], 2)
                    self.assertFalse(tail['has_more'])
                    self.assertNotEqual(unmatched['items'],tail['items'])
                    matched = await review(mixed_id, catalogue_matched=True, simulator_input_resolved=False)
                    self.assertEqual(matched['filtered_item_locations'], 1)
                    self.assertEqual(matched['items'][0]['simulator_item_resolution'], 'item_link_unresolved')
                    conflict = await review(mixed_id, catalogue_matched=False, simulator_slot='finger1')
                    self.assertEqual(conflict['items'], [])
                    for page in [unmatched, tail, matched, conflict]:
                        self.assertEqual(page['inventory_evidence'], mixed_evidence)
                    by_status = {
                        'catalogue_item_unavailable':2, 'item_link_unresolved':1, 'resolved':1,
                        'modifier_values_unavailable':0, 'suffix_reference_unresolved':0,
                        'enchant_reference_unresolved':0, 'simulator_item_unresolved':0,
                    }
                    for status, count in by_status.items():
                        page = await review(mixed_id, simulator_item_resolution=status)
                        self.assertEqual(page['filtered_item_locations'], count)
                        self.assertEqual([row['simulator_item_resolution'] for row in page['items']],
                                         [status]*count)
                        self.assertEqual(page['inventory_evidence'], mixed_evidence)
                    last = await review(mixed_id, simulator_item_resolution='catalogue_item_unavailable',
                                        catalogue_matched=False, storage='bags', offset=1, limit=1)
                    self.assertEqual(last['items'],tail['items'])
                    self.assertFalse(last['has_more'])
                    incompatible = await review(mixed_id, simulator_item_resolution='resolved',
                                                simulator_input_resolved=False)
                    self.assertEqual(incompatible['filtered_item_locations'],0)
                    self.assertTrue((await session.call_tool('review_forever_owned_gear', {
                        'snapshot_id':mixed_id,'simulator_item_resolution':'unknown'})).isError)
                    counts_field = 'simulator_item_resolution_counts'
                    self.assertNotIn(counts_field, await review(mixed_id))
                    expected_counts = {status:count for status,count in by_status.items() if count}
                    for offset in [0,1,4,99]:
                        counted = await review(mixed_id, include_simulator_item_resolution_counts=True,
                                               offset=offset, limit=1)
                        self.assertEqual(counted.pop(counts_field), expected_counts)
                        self.assertEqual(counted, await review(mixed_id, offset=offset, limit=1))
                    filtered_counts = await review(mixed_id, include_simulator_item_resolution_counts=True,
                                                   simulator_input_resolved=False, catalogue_matched=True)
                    self.assertEqual(filtered_counts[counts_field], {'item_link_unresolved':1})
                    self.assertEqual((await review(empty_id,
                        include_simulator_item_resolution_counts=True))[counts_field], {})
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
                    beyond_sample = await review(many_id, catalogue_matched=False, offset=50, limit=2)
                    self.assertEqual(beyond_sample['filtered_item_locations'],52)
                    self.assertEqual(len(beyond_sample['items']),2)
                    self.assertFalse(beyond_sample['has_more'])



if __name__ == '__main__':
    unittest.main()
