"""Exact cached reference traversal: cycles, explicit gaps, bounds and one index check."""
from copy import deepcopy
from pathlib import Path
import sqlite3
import unittest
from unittest.mock import patch

from tools import forever_knowledge as knowledge


def instance(records):
    value = knowledge.Knowledge.__new__(knowledge.Knowledge)
    value.source_records = records
    value.index = Path('C:/Temp/codex/unused-knowledge.sqlite')
    value.rows = []
    for key, source in records.items():
        section = key.split(':', 1)[0]
        identity = source[knowledge.SOURCES[section][1]]
        value.rows.append((key, section, identity, source.get('name', key), 'text', 'a'*64))
    return value


def sample():
    return instance({
        'items:1': {'id': 1, 'name': 'Item', 'sources': [
            {'drop/one~': {'npcId': 10, 'zoneId': 1000}}, {'reputation': {'repFactionId': 2000}}],
            'randomSuffixOptions': [7, 8], 'effectIds': [9, 0]},
        'npcs:10': {'id': 10, 'name': 'NPC', 'zoneId': 1000},
        'zones:1000': {'id': 1000, 'name': 'Zone', 'zoneId': 1000},
        'factions:2000': {'id': 2000, 'name': 'Faction', 'sources': [{'drop': {'npcId': 10}}]},
        'random_suffixes:7': {'id': 7, 'name': 'Option', 'sources': [{'drop': {'npcId': 99}}]},
        'spell_effects:9': {'id': 9, 'name': 'Effect', 'sources': [{'drop': {'npcId': 11}}]},
        'npcs:11': {'id': 11, 'name': 'Second NPC', 'zoneId': 1001},
        'zones:1001': {'id': 1001, 'name': 'Second zone'},
    })


class ReferenceWalkTests(unittest.TestCase):
    def test_cli_forwards_walk_options_and_rejects_before_model_load(self):
        base = ['forever_knowledge', 'get', '--catalogue', 'public.json',
                '--index', 'index.sqlite', '--model', 'model', '--record-key', 'items:1']
        with patch('sys.argv', base + ['--include-references', '--reference-depth', '3',
                                      '--reference-node-budget', '12', '--reference-edge-budget', '20']), \
             patch.object(knowledge, 'Knowledge') as factory, patch('builtins.print'):
            factory.return_value.get_record.return_value = {'found': True}
            knowledge.main()
            factory.return_value.get_record.assert_called_once_with('items:1', True, 0,
                reference_depth=3, reference_node_budget=12, reference_edge_budget=20)
        with patch('sys.argv', base + ['--reference-depth', '3']), \
             patch.object(knowledge, 'Knowledge') as factory:
            with self.assertRaises(ValueError): knowledge.main()
            factory.assert_not_called()

    def test_exact_links_cycles_missing_ids_depth_and_independent_source_hashes(self):
        value = sample(); before = deepcopy(value.source_records)
        two = value._reference_walk('items:1', 2, 64, 256)
        self.assertEqual(two['status'], 'complete_within_requested_depth')
        self.assertEqual((two['node_count'], two['edge_count'], two['unresolved_reference_count']), (7, 12, 3))
        self.assertEqual(two['depth_boundary_record_keys'], ['npcs:11'])
        self.assertEqual(two['unexpanded_discovered_record_keys'], [])
        self.assertEqual(two['edges'][0]['source_path'], '/sources/0/drop~1one~0/npcId')
        self.assertTrue(any(e['source_record_key'] == e['target_record_key'] for e in two['edges']))
        self.assertTrue(any(e['status'] == 'invalid_reference_id' and e['target_record_key'] is None for e in two['edges']))
        self.assertTrue(any(e['target_record_key'] == 'random_suffixes:8'
                            and e['status'] == 'not_in_pinned_catalogue' for e in two['edges']))
        for node in two['nodes']:
            self.assertEqual(node['catalogue_record_sha256'], knowledge.digest(
                knowledge.encoded(value.source_records[node['record_key']]).encode()))
            self.assertNotIn('catalogue_record', node)
        three = value._reference_walk('items:1', 3, 64, 256)
        self.assertEqual((three['node_count'], three['edge_count']), (8, 13))
        self.assertEqual(three['depth_boundary_record_keys'], ['zones:1001'])
        self.assertEqual(value.source_records, before)
        self.assertEqual(three, value._reference_walk('items:1', 3, 64, 256))
        for flag in ['complete_world_graph_verified', 'current_acquisition_availability_verified',
                     'native_world_identity_verified', 'owned_variant_verified', 'routing_verified', 'game_execution', 'live']:
            self.assertFalse(three[flag])

    def test_limits_exact_boundaries_and_reference_pages_beyond_32(self):
        value = sample()
        nodes = value._reference_walk('items:1', 3, 1, 256)
        self.assertEqual(nodes['status'], 'node_budget_exhausted')
        self.assertEqual((nodes['node_count'], nodes['edge_count']), (1, 1))
        self.assertFalse(nodes['edges'][0]['target_node_included'])
        self.assertEqual(nodes['unexpanded_discovered_record_keys'], ['items:1'])
        edges = value._reference_walk('items:1', 3, 64, 1)
        self.assertEqual(edges['status'], 'edge_budget_exhausted')
        self.assertEqual((edges['node_count'], edges['edge_count']), (2, 1))
        cycle = instance({'npcs:1': {'id': 1, 'name': 'A', 'zoneId': 2},
                          'zones:2': {'id': 2, 'name': 'B', 'zoneId': 2}})
        exact = cycle._reference_walk('npcs:1', 3, 2, 2)
        self.assertTrue(exact['traversal_complete_within_requested_depth'])
        records = {'items:1': {'id': 1, 'name': 'Many',
                   'sources': [{'drop': {'npcId': i}} for i in range(1, 41)]}}
        records.update({f'npcs:{i}': {'id': i, 'name': f'NPC {i}'} for i in range(1, 41)})
        wide = instance(records)._reference_walk('items:1', 2, 64, 256)
        self.assertEqual((wide['node_count'], wide['edge_count']), (41, 40))
        self.assertTrue(wide['traversal_complete_within_requested_depth'])

    def test_getter_validates_once_defaults_unknowns_options_deadline_and_byte_cap(self):
        value = sample()
        with patch.object(knowledge, 'connect', side_effect=lambda _: sqlite3.connect(':memory:')), \
             patch.object(knowledge, 'checked_records', return_value=value.rows) as validate:
            old = value.get_record('items:1', True)
            self.assertEqual(old, value.get_record('items:1', True, reference_depth=1))
            self.assertNotIn('catalogue_reference_walk', old['record'])
            validate.reset_mock()
            deep = value.get_record('items:1', True, reference_depth=3)
            self.assertEqual(validate.call_count, 1)
            self.assertNotIn('catalogue_references', deep['record'])
            self.assertTrue(deep['record']['catalogue_reference_walk']['traversal_complete_within_requested_depth'])
            self.assertFalse(value.get_record('items:999', True, reference_depth=3)['found'])
            validate.side_effect = ValueError('Corrupt index')
            with self.assertRaisesRegex(ValueError, 'Corrupt index'):
                value.get_record('items:1', True, reference_depth=3)
        with patch.object(knowledge, 'connect') as connect:
            for kwargs in [{'reference_depth': True}, {'reference_depth': 0}, {'reference_depth': 4},
                           {'reference_depth': 2}, {'include_references': True, 'reference_depth': 2, 'reference_offset': 1},
                           {'reference_node_budget': 0}, {'reference_edge_budget': 1025},
                           {'reference_depth': 2, 'include_references': True, 'reference_node_budget': True},
                           {'reference_depth': 2, 'include_references': True, 'reference_node_budget': 257}]:
                with self.assertRaises(ValueError): value.get_record('items:1', **kwargs)
            connect.assert_not_called()
        clock = [0]
        original = value.references
        def expired(*args):
            clock[0] = 6
            return original(*args)
        with patch.object(knowledge, 'monotonic', side_effect=lambda: clock[0]), \
             patch.object(value, 'references', side_effect=expired):
            timed = value._reference_walk('items:1', 3, 64, 256)
        self.assertEqual(timed['status'], 'analysis_time_budget_exhausted')
        self.assertFalse(timed['traversal_complete_within_requested_depth'])
        value.source_records['items:1']['name'] = 'x'*(256*1024)
        with patch.object(knowledge, 'connect', side_effect=lambda _: sqlite3.connect(':memory:')), \
             patch.object(knowledge, 'checked_records', return_value=value.rows):
            with self.assertRaisesRegex(ValueError, 'byte limit'):
                value.get_record('items:1', True, reference_depth=3)
