"""Reverse exact public references, independent of similarity retrieval."""
import sqlite3
import unittest
from unittest.mock import patch
from tools import forever_knowledge as knowledge
from test_knowledge_reference_walk import instance


class IncomingTests(unittest.TestCase):
    def test_cli_forwarding_and_early_rejection(self):
        base = ['forever_knowledge', 'get', '--catalogue', 'public.json', '--index', 'index.sqlite',
                '--model', 'model', '--record-key', 'npcs:7']
        with patch('sys.argv', base + ['--include-incoming-references', '--incoming-reference-offset', '32']), \
             patch.object(knowledge, 'Knowledge') as factory, patch('builtins.print'):
            factory.return_value.get_record.return_value = {'found': True}
            knowledge.main()
            factory.return_value.get_record.assert_called_once_with('npcs:7', False, 0,
                reference_depth=1, reference_node_budget=64, reference_edge_budget=256,
                include_incoming_references=True, incoming_reference_offset=32)
        for args in [base + ['--incoming-reference-offset', '1'],
                     base[:1] + ['search'] + base[2:] + ['--include-incoming-references']]:
            with patch('sys.argv', args), patch.object(knowledge, 'Knowledge') as factory, \
                 patch('sys.stderr'):
                with self.assertRaises(SystemExit): knowledge.main()
                factory.assert_not_called()

    def test_exact_namespace_duplicates_paging_hashes_and_copy(self):
        records = {'npcs:7': {'id': 7, 'name': 'NPC'}, 'zones:7': {'id': 7, 'name': 'Zone'}}
        records.update({f'items:{i}': {'id': i, 'name': str(i), 'sources': [
            {'drop/~': {'npcId': 7, 'zoneId': 7}}, {'drop': {'npcId': 7}}]}
            for i in range(1, 23)})
        records['items:0'] = {'id': 0, 'effectIds': [7], 'sources': [{'drop': {'npcId': True}}]}
        value = instance(records)
        first = value.incoming_references('npcs:7', 0)
        second = value.incoming_references('npcs:7', 32)
        self.assertEqual((first['reference_count'], first['returned_reference_count']), (44, 32))
        self.assertEqual(second['returned_reference_count'], 12)
        self.assertTrue(first['has_more_references']); self.assertFalse(second['has_more_references'])
        self.assertEqual(len({(e['source_record_key'], e['source_path']) for e in
                             first['references'] + second['references']}), 44)
        self.assertEqual(value.incoming_references('zones:7', 0)['reference_count'], 22)
        self.assertEqual(value.incoming_references('npcs:7', 100)['references'], [])
        for edge in first['references']:
            self.assertEqual(edge['catalogue_record_sha256'], knowledge.digest(
                knowledge.encoded(records[edge['source_record_key']]).encode()))
        self.assertTrue(any(e['source_path'] == '/sources/0/drop~1~0/npcId' for e in first['references']))
        first['references'].clear()
        self.assertEqual(value.incoming_references('npcs:7', 0)['returned_reference_count'], 32)

    def test_getter_validation_default_unknown_and_deadline(self):
        value = instance({'npcs:7': {'id': 7}, 'items:1': {'id': 1, 'sources': [{'drop': {'npcId': 7}}]}})
        with patch.object(knowledge, 'connect', side_effect=lambda _: sqlite3.connect(':memory:')), \
             patch.object(knowledge, 'checked_records', return_value=value.rows):
            self.assertNotIn('incoming_catalogue_references', value.get_record('npcs:7')['record'])
            result = value.get_record('npcs:7', include_incoming_references=True)
            self.assertEqual(result['record']['incoming_catalogue_references']['reference_count'], 1)
            self.assertFalse(value.get_record('npcs:999', include_incoming_references=True)['found'])
        with patch.object(knowledge, 'connect') as connect:
            for options in [{'include_incoming_references': 1}, {'incoming_reference_offset': 1},
                            {'include_incoming_references': True, 'incoming_reference_offset': True},
                            {'include_incoming_references': True, 'incoming_reference_offset': -1}]:
                with self.assertRaises(ValueError): value.get_record('npcs:7', **options)
            connect.assert_not_called()
        with patch.object(knowledge, 'monotonic', side_effect=[0, 6]):
            with self.assertRaisesRegex(ValueError, 'time limit'): value.incoming_references('npcs:7', 0)
