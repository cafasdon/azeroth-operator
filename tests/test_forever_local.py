"""Synthetic owner-capture projection, corruption guards and actual MCP stdio tests."""
import asyncio
from concurrent.futures import ThreadPoolExecutor
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from shared.snapshots import SnapshotStore
from tools import forever_local as local

ROOT = Path(__file__).resolve().parents[1]
SENTINEL = 'PRIVATE_OWNER_SENTINEL_289123'


def fixture():
    return {'schema_version': 1, 'source': 'addon', 'captured_at': '2025-01-01T00:00:00Z',
            'client': {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001},
            'character': {'name': SENTINEL, 'realm': SENTINEL},
            'sections': {
                'equipment': {'1': {'item_id': 42, 'guid': SENTINEL,
                    'link': '|cffffffff|Hitem:42:0:0:0:0:0:0:0|h[Test Ore]|h|r'}},
                'bags': {'0': {'1': {'item_id': 43, 'count': 5, 'guid': SENTINEL,
                    'link': '|Hitem:43:0|h[Test Bar]|h'}}},
                'character_bank': {'6': {'1': {'item_id': 44, 'count': 2, 'guid': SENTINEL}}},
                'account_bank': {'12': {'1': {'item_id': 45, 'count': 3, 'guid': SENTINEL}}},
                'profession_recipe_view': {'coverage': 'filtered_local_recipe_list',
                    'profession_id': 164, 'entries': {'100': {'name': 'Test Smelt', 'learned': True},
                        '101': {'name': 'Test Temper', 'learned': True}},
                    'rank_observation': {'rank': 123, 'max_rank': 300}},
                'profession_recipe_materials': {'coverage': 'reported_known_spell_schematics',
                    'profession_id': 164, 'entries': {
                        '100': {'status': 'fixed_items', 'reagents': {'42': 2},
                            'output_item_id': 43, 'base_output_quantity': {'minimum': 1, 'maximum': 2},
                            'reported_craftable_count': 13},
                        '101': {'status': 'fixed_items', 'reagents': {'43': 3}, 'output_item_id': 44},
                        '102': {'status': 'unsupported_required_slots'}}},
                'spellbook': {'coverage': 'visible_player_spell_slots', 'entries': {
                    '1': {'base_spell_id': 500, 'spell_id': 501, 'name': 'Test Spell',
                        'passive': False, 'off_spec': True, 'reported_cast_count': 19}}},
                'skill_lines': {'164': {'name': 'Blacksmithing', 'rank': 123, 'max_rank': 300, 'category_id': 11}},
            }}


class LocalReferenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        self.store = SnapshotStore(self.folder / 'owner')
        self.snapshot = fixture()
        self.snapshot_id = self.store.ingest(self.snapshot)
        self.database = self.folder / 'reference.sqlite'

    def build(self):
        return local.build(self.store.root, self.snapshot_id, self.database)

    def load(self):
        knowledge = local.LocalForever(self.database)
        self.addCleanup(knowledge.close)
        return knowledge

    def test_projection_exact_edges_privacy_and_offline(self):
        before = (self.store.root / (self.snapshot_id + '.json')).read_bytes()
        with patch.object(socket, 'socket', side_effect=AssertionError('No network')):
            receipt = self.build()
            knowledge = self.load()
            self.assertEqual(receipt['record_count'], 8)
            self.assertEqual(receipt['edge_count'], 6)
            self.assertEqual(knowledge.get('item', 42)['record']['fields'], {'observed_names': ['Test Ore']})
            self.assertEqual(knowledge.get('item', 44)['record']['fields'], {})
            recipe = knowledge.get('recipe', 100)
            self.assertEqual(recipe['record']['fields'], {'name': 'Test Smelt'})
            self.assertEqual(recipe['edges'], [
                {'direction': 'outgoing', 'relation': 'output', 'target_kind': 'item', 'target_id': 43, 'minimum': 1, 'maximum': 2},
                {'direction': 'outgoing', 'relation': 'profession', 'target_kind': 'skill', 'target_id': 164},
                {'direction': 'outgoing', 'relation': 'reagent', 'target_kind': 'item', 'target_id': 42, 'quantity': 2}])
            incoming = knowledge.get('item', 43)['edges']
            self.assertEqual([(x['relation'], x['target_id']) for x in incoming], [('output', 100), ('reagent', 101)])
            self.assertEqual(knowledge.get('spell', 501)['record']['fields'], {'name': 'Test Spell', 'base_spell_id': 500, 'passive': False})
            self.assertEqual(knowledge.get('skill', 164)['record']['fields'], {'name': 'Blacksmithing', 'category_id': 11})
            self.assertFalse(knowledge.get('recipe', 102)['found'])
        raw = self.database.read_bytes()
        self.assertNotIn(SENTINEL.encode(), raw)
        for private in (b'reported_craftable_count', b'guid', b'learned', b'count', b'rank_observation', b'off_spec', b'|Hitem:'):
            self.assertNotIn(private, raw)
        self.assertEqual(before, (self.store.root / (self.snapshot_id + '.json')).read_bytes())
        for result in (receipt, recipe, knowledge.search('item', 'test')):
            self.assertEqual(result['source'], 'owner_addon_capture')
            self.assertEqual(result['build'], '70862')
            self.assertEqual(result['captured_at'], self.snapshot['captured_at'])
            self.assertEqual(result['coverage'], 'incomplete_single_capture')
            self.assertFalse(result['live']); self.assertFalse(result['game_actions'])
            self.assertFalse(result['redistribution_rights_implied'])

    def test_literal_search_stable_paging_and_bounded_edges(self):
        self.build()
        knowledge = self.load()
        first = knowledge.search('item', ' TEST ', limit=1)
        second = knowledge.search('item', 'test', limit=1, offset=1)
        self.assertEqual(first['results'] + second['results'], knowledge.search('item', 'test')['results'])
        self.assertTrue(first['has_more']); self.assertFalse(second['has_more'])
        for query in ('%', '_', "' OR 1=1 --"):
            self.assertEqual(knowledge.search('item', query)['results'], [])
        edges = knowledge.get('recipe', 100, edge_limit=1)
        self.assertTrue(edges['has_more_edges'])
        self.assertEqual(edges['edges'] + knowledge.get('recipe', 100, edge_limit=2, edge_offset=1)['edges'], knowledge.get('recipe', 100)['edges'])
        self.assertEqual(knowledge.get('item', 43, edge_limit=1)['edges'] + knowledge.get('item', 43, edge_limit=1, edge_offset=1)['edges'], knowledge.get('item', 43)['edges'])
        self.assertFalse(knowledge.get('item', 999)['found'])
        for args in [('item', True), ('unknown', 42), ('item', 0), ('item', 2**32), ('item', 42, True), ('item', 42, 65), ('item', 42, 1, True)]:
            with self.subTest(args=args), self.assertRaises(ValueError): knowledge.get(*args)
        for args in [('item', ''), ('item', ' '), ('item', 'a' * 129), ('item', 'a\n'), ('item', 'test', True), ('item', 'test', 21), ('item', 'test', 1, True)]:
            with self.subTest(args=args), self.assertRaises(ValueError): knowledge.search(*args)

    def test_one_capture_no_wrong_profession_join_and_no_cross_capture_merge(self):
        self.snapshot['sections']['profession_recipe_view']['profession_id'] = 333
        self.snapshot_id = self.store.ingest(self.snapshot)
        self.build()
        knowledge = self.load()
        self.assertEqual(knowledge.get('recipe', 100)['record']['fields'], {})
        knowledge.close()
        self.snapshot = fixture()
        self.snapshot['sections'] = {'equipment': {}}
        self.snapshot_id = self.store.ingest(self.snapshot)
        self.build()
        empty = self.load()
        self.assertFalse(empty.get('item', 42)['found'])
        self.assertEqual(empty.manifest['snapshot_id'], self.snapshot_id)

    def test_malformed_hash_family_source_and_path_guards(self):
        for key in ('../x', 'a' * 63, 'A' * 64, True):
            with self.subTest(key=key), self.assertRaises(ValueError):
                local.build(self.store.root, key, self.database)
        path = self.store.root / (self.snapshot_id + '.json')
        raw = path.read_bytes()
        path.write_bytes(b'{}')
        with self.assertRaises(ValueError): self.build()
        path.write_bytes(raw)
        for source in ('fixture', 'official_api'):
            changed = fixture(); changed['source'] = source
            key = self.store.ingest(changed)
            with self.assertRaises(ValueError): local.build(self.store.root, key, self.database)
        changed = fixture(); changed['client']['family'] = 'retail'; changed['sections'] = {'equipment': {}}
        key = self.store.ingest(changed)
        with self.assertRaises(ValueError): local.build(self.store.root, key, self.database)
        changed = fixture(); changed['sections']['bags']['0']['1']['count'] = True
        raw = json.dumps(changed).encode(); key = hashlib.sha256(raw).hexdigest()
        (self.store.root / (key + '.json')).write_bytes(raw)
        with self.assertRaises(ValueError): local.build(self.store.root, key, self.database)
        for output in (Path('relative.sqlite'), self.store.root / 'reference.sqlite', path, self.folder):
            with self.subTest(output=output), self.assertRaises(ValueError):
                local.build(self.store.root, self.snapshot_id, output)
        with self.assertRaises(ValueError): local.build(Path('owner'), self.snapshot_id, self.database)

    def test_unrelated_files_untouched_tamper_detected_and_queries_frozen(self):
        self.database.write_bytes(b'UNRELATED FILE')
        with self.assertRaises(ValueError): self.build()
        self.assertEqual(self.database.read_bytes(), b'UNRELATED FILE')
        self.database.unlink(); self.build()
        knowledge = self.load()
        with closing(sqlite3.connect(self.database)) as connection, connection:
            connection.execute('UPDATE records SET value=? WHERE kind=? AND id=?', ('{"name":"Changed"}', 'item', 42))
        self.assertEqual(knowledge.get('item', 42)['record']['fields']['observed_names'], ['Test Ore'])
        with self.assertRaises(ValueError): local.LocalForever(self.database)
        with self.assertRaises(ValueError): self.build()

    def test_staging_has_a_sqlite_page_limit_and_preserves_active_database(self):
        self.build()
        before = self.database.read_bytes()
        real_connect = sqlite3.connect
        observed = []
        def constrained(*args, **kwargs):
            connection = real_connect(*args, **kwargs)
            if str(args[0]) != ':memory:':
                connection.set_trace_callback(observed.append)
            return connection
        # A tiny quota forces failure while creating the staging tables.
        with patch.object(local, 'MAX_DATABASE', 8192), patch.object(local.sqlite3, 'connect', constrained):
            with self.assertRaises((ValueError, sqlite3.DatabaseError)):
                self.build()
        self.assertEqual(self.database.read_bytes(), before)
        self.assertFalse(list(self.database.parent.glob('.local-reference-*')))
        # Existing-file validation can reject the tiny limit before any write.
        fresh = self.database.parent / 'fresh.sqlite'
        with patch.object(local, 'MAX_DATABASE', 8192), patch.object(local.sqlite3, 'connect', constrained):
            with self.assertRaises(sqlite3.DatabaseError):
                local.build(self.store.root, self.snapshot_id, fresh)
        self.assertFalse(fresh.exists())
        self.assertTrue(any('max_page_count=2' in statement for statement in observed))

    def test_schema_and_manifest_corruption(self):
        self.build(); good = self.database.read_bytes()
        for statement in ('CREATE TABLE other (secret TEXT)', 'DELETE FROM manifest', "UPDATE manifest SET value='{}'"):
            self.database.write_bytes(good)
            with closing(sqlite3.connect(self.database)) as connection, connection: connection.execute(statement)
            with self.subTest(statement=statement), self.assertRaises(ValueError): local.LocalForever(self.database)
        self.database.write_bytes(b'not sqlite')
        with self.assertRaises(ValueError): local.LocalForever(self.database)

    def test_missing_view_conflicting_known_recipe_and_modern_item_variants(self):
        del self.snapshot['sections']['profession_recipe_view']
        self.snapshot['sections']['equipment']['1']['link'] = '|cnIQ3:|Hitem:42:0|h[Test Ore]|h|r'
        self.snapshot['sections']['equipment']['1']['required_level'] = 5
        self.snapshot['sections']['bags']['0']['2'] = {'item_id': 42, 'count': 1,
            'link': '|cnIQ3:|Hitem:42:99|h[Test Ore]|h|r', 'required_level': 10,
            'native_stats': {'ITEM_MOD_STRENGTH_SHORT': 7}}
        self.snapshot_id = self.store.ingest(self.snapshot); self.build()
        knowledge = self.load()
        self.assertEqual(knowledge.get('recipe', 100)['record']['fields'], {})
        self.assertEqual(knowledge.get('item', 42)['record']['fields'], {'observed_names': ['Test Ore'],
            'observed_variants': [{'native_stats': {'ITEM_MOD_STRENGTH_SHORT': 7}, 'required_level': 10}, {'required_level': 5}]})
        self.snapshot = fixture()
        self.snapshot['sections']['profession_recipe_view']['entries']['100']['learned'] = False
        self.snapshot_id = self.store.ingest(self.snapshot)
        before = self.database.read_bytes()
        with self.assertRaisesRegex(ValueError, 'Conflicting'): self.build()
        self.assertEqual(self.database.read_bytes(), before)

    def test_same_base_item_distinct_suffix_names_remain_observations_and_searchable(self):
        self.snapshot['sections']['equipment']['1'].update(
            link='|cnIQ2:|Hitem:42:0:0:0:0:0:5|h[Example Sword of Strength]|h|r',
            native_stats={'ITEM_MOD_STRENGTH_SHORT': 7})
        self.snapshot['sections']['bags']['0']['2'] = {'item_id': 42, 'count': 1,
            'guid': SENTINEL, 'link': '|cnIQ2:|Hitem:42:0:0:0:0:0:6|h[Example Sword of Agility]|h|r',
            'native_stats': {'ITEM_MOD_AGILITY_SHORT': 9}}
        self.snapshot_id = self.store.ingest(self.snapshot)
        self.build(); knowledge = self.load()
        fields = knowledge.get('item', 42)['record']['fields']
        self.assertNotIn('name', fields)
        self.assertEqual(fields['observed_names'], ['Example Sword of Agility', 'Example Sword of Strength'])
        self.assertEqual(fields['observed_variants'], [
            {'native_stats': {'ITEM_MOD_AGILITY_SHORT': 9}},
            {'native_stats': {'ITEM_MOD_STRENGTH_SHORT': 7}}])
        for label in ('of Agility', 'of Strength'):
            self.assertEqual([row['id'] for row in knowledge.search('item', label)['results']], [42])
        self.assertNotIn(SENTINEL.encode(), self.database.read_bytes())
        self.assertNotIn(b'|Hitem:', self.database.read_bytes())

    def test_parallel_queries_from_worker_threads(self):
        self.build(); knowledge = self.load()
        def read(index):
            return knowledge.get('recipe', 100) if index % 2 else knowledge.search('item', 'Test')
        with ThreadPoolExecutor(max_workers=4) as workers:
            results = list(workers.map(read, range(32)))
        self.assertEqual(len(results), 32)
        self.assertTrue(all(result['source'] == 'owner_addon_capture' for result in results))

    def test_symlink_inputs_and_outputs_rejected(self):
        link = self.folder / 'link'
        try:
            link.symlink_to(self.store.root, target_is_directory=True)
        except OSError as error:
            self.skipTest(f'Host cannot create symbolic links: {error}')
        with self.assertRaises(ValueError): local.build(link, self.snapshot_id, self.database)
        link.unlink()
        path = self.store.root / (self.snapshot_id + '.json')
        saved = self.folder / 'saved.json'; saved.write_bytes(path.read_bytes())
        path.unlink(); path.symlink_to(saved)
        with self.assertRaises(ValueError): self.build()
        path.unlink(); path.write_bytes(saved.read_bytes())
        self.database.symlink_to(saved)
        with self.assertRaises(ValueError): self.build()
        with self.assertRaises(ValueError): local.LocalForever(self.database)
        self.assertEqual(saved.read_bytes(), path.read_bytes())

    def test_cli_build_get_search_and_stdio_sdk(self):
        environment = {**os.environ, 'PYTHONPATH': str(ROOT)}
        base = [sys.executable, '-m', 'tools.forever_local']
        for arguments in (['build', '--store', str(self.store.root), '--snapshot-id', self.snapshot_id],
                          ['get', '--kind', 'recipe', '--entity-id', '100'],
                          ['search', '--kind', 'item', '--query', 'Test']):
            result = subprocess.run([*base, *arguments, '--database', str(self.database)], cwd=ROOT,
                env=environment, capture_output=True, text=True, timeout=20, check=True)
            self.assertEqual(json.loads(result.stdout)['source'], 'owner_addon_capture')

        async def exercise():
            params = StdioServerParameters(command=sys.executable,
                args=['-m', 'tools.forever_local', 'serve', '--database', str(self.database)], cwd=str(ROOT), env=environment)
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    initial = await session.initialize()
                    self.assertIn('first-party saved observations', initial.instructions)
                    tools = (await session.list_tools()).tools
                    self.assertEqual({tool.name for tool in tools}, {'search_local_forever_records', 'get_local_forever_record', 'get_local_forever_recipe_dependencies', 'plan_local_forever_recipe_materials'})
                    for tool in tools:
                        self.assertTrue(tool.annotations.readOnlyHint)
                        self.assertFalse(tool.annotations.openWorldHint)
                        self.assertNotIn('database', tool.inputSchema['properties'])
                    result = await session.call_tool('get_local_forever_record', {'kind': 'recipe', 'entity_id': 100, 'edge_limit': 1})
                    self.assertFalse(result.isError)
                    data = json.loads(result.content[0].text)
                    self.assertTrue(data['has_more_edges']); self.assertEqual(len(data['edges']), 1)
                    result = await session.call_tool('search_local_forever_records', {'kind': 'item', 'query': 'Test', 'limit': 1, 'offset': 1})
                    self.assertFalse(result.isError)
                    self.assertEqual(json.loads(result.content[0].text)['results'][0]['id'], 43)
                    concurrent = await asyncio.gather(*(session.call_tool('get_local_forever_record', {'kind': 'recipe', 'entity_id': 100}) for _ in range(8)))
                    self.assertTrue(all(not result.isError for result in concurrent))
                    for arguments in ({'kind': 'item', 'entity_id': True}, {'kind': 'item', 'entity_id': 42, 'edge_limit': True}, {'kind': 'unknown', 'entity_id': 42}):
                        self.assertTrue((await session.call_tool('get_local_forever_record', arguments)).isError)
                    for arguments in ({'kind': 'item', 'query': 'test', 'limit': True}, {'kind': 'item', 'query': '%'}, {'kind': 'item', 'query': ''}):
                        result = await session.call_tool('search_local_forever_records', arguments)
                        self.assertEqual(result.isError, arguments['query'] != '%')
        asyncio.run(exercise())


if __name__ == '__main__':
    unittest.main()

