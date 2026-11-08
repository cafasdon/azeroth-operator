"""Saved recipe lists discover cached crafting outputs without proving craftability."""
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


class CatalogueRecipeLearningTests(unittest.IsolatedAsyncioTestCase):
    async def test_list_learning_items_enchants_conflicts_and_selected_compatibility(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            base = fixture()
            base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            keys = {}
            for listed in [None, {}, {'learned': None}, {'learned': False}, {'learned': True}]:
                for selected in [None, False, True]:
                    sample = deepcopy(base)
                    if listed is not None:
                        sample['sections']['profession_recipe_view'] = {
                            'coverage': 'filtered_local_recipe_list', 'profession_id': 164,
                            'entries': {'2675': {**listed, 'name': 'PRIVATE_RECIPE_LIST_NAME'}, '3508': {'learned': True}}}
                    if selected is not None:
                        sample['sections']['selected_recipe'] = {'recipe_id': 2675, 'name': 'Synthetic',
                            'learned': selected, 'output_item_id': 2870, 'requirements': 'fixed_items', 'reagents': {}}
                    keys[(json.dumps(listed), selected)] = store.ingest(sample)
            mismatch = deepcopy(sample)
            mismatch['sections']['selected_recipe']['output_item_id'] = 3844
            mismatch_key = store.ingest(mismatch)
            enchant = deepcopy(base)
            enchant['sections']['profession_recipe_view'] = {'coverage': 'filtered_local_recipe_list',
                'profession_id': 165, 'entries': {'2152': {'learned': True}}}
            enchant_key = store.ingest(enchant)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def lookup(snapshot_id, **query):
                        response = await session.call_tool('lookup_forever_catalogue',
                            {'section': 'items', 'key': '2870', 'acquisition_snapshot_id': snapshot_id, **query})
                        self.assertFalse(response.isError, response)
                        return json.loads(response.content[0].text)
                    for (listed_json, selected), key in keys.items():
                        listed = json.loads(listed_json)
                        flag = listed.get('learned') if listed is not None else None
                        conflict = flag is not None and selected is not None and flag != selected
                        expected = None if conflict else selected if selected is not None else flag
                        result = await lookup(key)
                        self.assertNotIn('PRIVATE_RECIPE_LIST_NAME', json.dumps(result))
                        source = next(row for row in result['acquisition_progress']['sources'] if row.get('recipe_spell_id') == 2675)
                        learning = source['recipe_learning']
                        self.assertEqual(learning['learned'], expected)
                        self.assertEqual(learning['learning_conflict'], conflict)
                        self.assertEqual(learning['selected_recipe_observed'], selected is not None)
                        self.assertEqual(learning['recipe_list_observed'], listed is not None)
                        self.assertEqual(source['learned'], selected)
                        if selected is None:
                            self.assertIsNone(source['material_capacity'])
                        filtered = await lookup(key, only_observed_learned_recipe=True)
                        self.assertEqual(bool(filtered['records']), expected is True)
                    result = await lookup(mismatch_key)
                    learning = result['acquisition_progress']['sources'][0]['recipe_learning']
                    self.assertFalse(learning['selected_output_matches_catalogue'])
                    self.assertIsNone(learning['learned'])
                    self.assertEqual((await lookup(mismatch_key, only_observed_learned_recipe=True))['records'], [])
                    list_key = keys[(json.dumps({'learned': True}), None)]
                    batch = await lookup(list_key, key=None, crafted_profession_id=2, only_observed_learned_recipe=True)
                    self.assertEqual({row['id'] for row in batch['records']}, {2870, 3844})
                    self.assertEqual((await lookup(list_key, crafted_profession_id=3,
                        only_observed_learned_recipe=True))['records'], [])
                    self.assertEqual((await lookup(list_key, recipe_spell_id=3508,
                        only_observed_learned_recipe=True))['records'], [])
                    effects = await lookup(enchant_key, section='enchants', key=None, recipe_spell_id=2152,
                                           only_observed_learned_recipe=True)
                    self.assertTrue(effects['records'])
                    self.assertTrue(all(row['itemId'] == 2304 for row in effects['records']))
                    self.assertEqual(effects['recipe_observation']['snapshot_id'], enchant_key)


if __name__ == '__main__':
    unittest.main()
