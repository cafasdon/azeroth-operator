"""Search saved recipe rows through real MCP, without external reference data."""
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
from tools.gateway_profile import profile


class RecipeSearchTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_filters_paging_unknowns_and_planning(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        rows = {}
        statuses = ['fixed_items', 'unavailable', 'unsupported_required_slots', 'budget_exhausted']
        for i in range(1, 132):
            status = statuses[(i - 1) % 4]
            rows[str(i)] = {'status': status}
            if status == 'fixed_items':
                rows[str(i)].update(reagents={str(3000 + i % 5): i}, output_item_id=2000 + i % 3,
                                    base_output_quantity={'minimum': 2, 'maximum': 3})
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            missing_id = store.ingest(base)
            empty = deepcopy(base)
            empty['sections']['profession_recipe_materials'] = {
                'coverage': 'reported_known_spell_schematics', 'profession_id': 164, 'entries': {}}
            empty_id = store.ingest(empty)
            saved = deepcopy(empty)
            saved['sections']['profession_recipe_materials']['entries'] = rows
            saved['sections']['selected_recipe'] = {'recipe_id': 999, 'name': 'PRIVATE_SELECTED',
                'learned': True, 'requirements': 'fixed_items', 'reagents': {}}
            key = store.ingest(saved)
            retail_id = store.ingest(fixture())
            original = {path.name: path.read_bytes() for path in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    tools = {tool.name: tool for tool in (await session.list_tools()).tools}
                    self.assertTrue(tools['search_forever_recipes'].annotations.readOnlyHint)
                    self.assertFalse(tools['search_forever_recipes'].annotations.openWorldHint)
                    async def search(error=False, **options):
                        reply = await session.call_tool('search_forever_recipes', {'snapshot_id': key, **options})
                        self.assertEqual(bool(reply.isError), error, reply)
                        self.assertNotIn('PRIVATE', str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    missing = await search(snapshot_id=missing_id)
                    self.assertFalse(missing['section_observed'])
                    self.assertIsNone(missing['captured_spell_count'])
                    self.assertIsNone(missing['matched_row_count'])
                    empty_result = await search(snapshot_id=empty_id)
                    self.assertTrue(empty_result['section_observed'])
                    self.assertEqual(empty_result['matched_row_count'], 0)
                    self.assertEqual(empty_result['records'], [])
                    first = await search()
                    self.assertEqual(first['captured_spell_count'], 131)
                    self.assertEqual(first['matched_row_count'], 131)
                    self.assertEqual([row['recipe_id'] for row in first['records']], list(range(1, 21)))
                    self.assertEqual(first['next_offset'], 20)
                    self.assertEqual(first['snapshot_id'], key)
                    self.assertFalse(first['craftability_verified'])
                    self.assertFalse(first['complete_recipe_knowledge_verified'])
                    self.assertFalse(first['game_execution'])
                    for status in statuses:
                        records, offset = [], 0
                        while True:
                            page = await search(status=status, limit=7, offset=offset)
                            records.extend(page['records'])
                            if page['next_offset'] is None:
                                break
                            offset = page['next_offset']
                        expected = [i for i in range(1, 132) if rows[str(i)]['status'] == status]
                        self.assertEqual([row['recipe_id'] for row in records], expected)
                        self.assertEqual(page['matched_row_count'], len(expected))
                    combined = await search(output_item_id=2001, reagent_item_id=3001, limit=50)
                    expected = [i for i in range(1, 132) if rows[str(i)].get('output_item_id') == 2001
                                and '3001' in rows[str(i)].get('reagents', {})]
                    self.assertEqual([row['recipe_id'] for row in combined['records']], expected)
                    self.assertEqual((await search(recipe_id=1))['records'][0]['reagents'], {'3001': 1})
                    self.assertEqual((await search(recipe_id=1, output_item_id=2002))['matched_row_count'], 0)
                    self.assertEqual((await search(status='unavailable', reagent_item_id=3001))['matched_row_count'], 0)
                    self.assertEqual((await search(offset=4096))['records'], [])
                    self.assertIsNone((await search(offset=4096))['next_offset'])
                    chosen = combined['records'][0]['recipe_id']
                    reply = await session.call_tool('review_forever_recipe', {'snapshot_id': key,
                        'recipe_id': chosen, 'target_output_quantity': 3})
                    self.assertFalse(reply.isError, reply)
                    plan = json.loads(reply.content[0].text)
                    self.assertEqual(plan['recipe']['recipe_id'], chosen)
                    self.assertEqual(plan['requested_crafts'], 2)
                    self.assertEqual(plan['recipe_observation_source'], 'profession_recipe_materials')
                    for bad in [{'recipe_id': 0}, {'output_item_id': -1}, {'reagent_item_id': True},
                                {'recipe_id': '1'}, {'recipe_id': 4294967296}, {'status': 'learned'},
                                {'offset': -1}, {'limit': 0}, {'limit': 51}, {'limit': True},
                                {'unexpected': 'PRIVATE'}, {'snapshot_id': '../PRIVATE'},
                                {'snapshot_id': '0' * 64}, {'snapshot_id': retail_id}]:
                        await search(error=True, **bad)
            self.assertEqual(original, {path.name: path.read_bytes() for path in Path(folder).glob('*.json')})
        rendered = profile('sha256:' + 'a' * 64, store='/srv/fixture/snapshots')
        self.assertIn('search_forever_recipes', rendered['servers'][0]['tools'])
        self.assertTrue(rendered['servers'][0]['snapshot']['server']['volumes'][0].endswith(':ro'))
