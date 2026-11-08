"""Plan confirmed consumable shortages with explicit bulk or selected recipes."""
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


class BulkConsumableRestockTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_bulk_mixed_legacy_and_rejections(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned local catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        base['sections']['bags'] = {'0': {'1': {'item_id': 2447, 'count': 999}}}
        base['sections']['character_bank'] = {}
        base['sections']['account_bank'] = {}
        rows = {'2330': {'status': 'fixed_items', 'reagents': {'2447': 2}, 'output_item_id': 118,
                         'base_output_quantity': {'minimum': 1, 'maximum': 2}},
                '2337': {'status': 'fixed_items', 'reagents': {'2447': 3}, 'output_item_id': 858,
                         'base_output_quantity': {'minimum': 2, 'maximum': 3}}}
        base['sections']['profession_recipe_materials'] = {
            'coverage': 'reported_known_spell_schematics', 'profession_id': 171, 'entries': rows}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            bulk_id = store.ingest(base)
            stock = deepcopy(base)
            stock['captured_at'] = '2025-01-02T00:00:00Z'
            stock['sections']['bags'] = {'0': {'1': {'item_id': 118, 'count': 1},
                                              '2': {'item_id': 2447, 'count': 10}}}
            stock_id = store.ingest(stock)
            legacy = deepcopy(base)
            legacy['sections'].pop('profession_recipe_materials')
            legacy['sections']['selected_recipe'] = {'recipe_id': 2330, 'name': 'PRIVATE_SELECTED',
                'learned': True, 'requirements': 'fixed_items', 'reagents': {'2447': 2}, 'output_item_id': 118,
                'base_output_quantity': {'minimum': 1, 'maximum': 2}, 'cooldown': {'remaining_seconds': 20}}
            legacy_id = store.ingest(legacy)
            bad_recipes = []
            for status in ['unavailable', 'budget_exhausted', 'unsupported_required_slots']:
                value = deepcopy(base)
                value['sections']['profession_recipe_materials']['entries']['2330'] = {'status': status}
                bad_recipes.append(store.ingest(value))
            for kind in ['unknown_output', 'wrong_output', 'false_learning', 'foreign', 'build', 'future', 'target_reagent']:
                value = deepcopy(base)
                row = value['sections']['profession_recipe_materials']['entries']['2330']
                if kind == 'unknown_output':
                    del row['output_item_id']
                    del row['base_output_quantity']
                elif kind == 'wrong_output':
                    row['output_item_id'] = 2455
                elif kind == 'false_learning':
                    value['sections']['tracked_recipes'] = {'coverage': 'player_tracked_recipes_only',
                        'crafts': {'2330': {'name': 'PRIVATE_TRACKED', 'learned': False}}, 'recrafts': {}}
                elif kind == 'foreign':
                    value['character']['name'] = 'PRIVATE_FOREIGN'
                elif kind == 'build':
                    value['client']['build'] = 'other'
                elif kind == 'future':
                    value['captured_at'] = '2025-01-03T00:00:00Z'
                elif kind == 'target_reagent':
                    row['reagents'] = {'858': 1}
                bad_recipes.append(store.ingest(value))
            incomplete = deepcopy(stock)
            del incomplete['sections']['account_bank']
            incomplete_id = store.ingest(incomplete)
            original = {path.name: path.read_bytes() for path in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            selectors = [{'snapshot_id': bulk_id, 'recipe_id': 2330}, {'snapshot_id': bulk_id, 'recipe_id': 2337}]
            targets = [{'item_id': 118, 'quantity': 5}, {'item_id': 858, 'quantity': 5}]
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def lookup(error=False, **options):
                        args = {'section': 'consumables', 'acquisition_snapshot_id': stock_id,
                                'consumable_targets': targets, 'restock_recipes': selectors, **options}
                        reply = await session.call_tool('lookup_forever_catalogue', args)
                        self.assertEqual(bool(reply.isError), error, reply)
                        self.assertNotIn('PRIVATE', str(reply))
                        return None if error else json.loads(reply.content[0].text)['acquisition_progress']
                    result = await lookup()
                    crafting = result['crafting_plan']
                    self.assertEqual([row['requested_crafts'] for row in crafting['recipes']], [4, 3])
                    self.assertEqual([row['target_output_quantity'] for row in crafting['recipes']], [4, 5])
                    self.assertEqual([row['recipe_id'] for row in crafting['recipes']], [2330, 2337])
                    self.assertTrue(all(row['recipe_observation_source'] == 'profession_recipe_materials' for row in crafting['recipes']))
                    self.assertTrue(all(row['recipe_profession_id'] == 171 for row in crafting['recipes']))
                    self.assertTrue(all(row['observed_cooldown'] is None and row['unmet_crafting_conditions'] is None
                                        for row in crafting['recipes']))
                    material = crafting['combined_materials'][0]
                    self.assertEqual(material['item_id'], 2447)
                    self.assertEqual(material['required'], 17)
                    self.assertEqual(material['quantity_observed'], 10)
                    self.assertEqual(material['shortfall_vs_observed'], 7)
                    self.assertFalse(crafting['craftability_verified'])
                    legacy_result = await lookup(restock_recipes=None, restock_recipe_snapshot_ids=[legacy_id],
                                                 consumable_targets=[targets[0]])
                    selected_result = await lookup(restock_recipes=[{'snapshot_id': legacy_id}],
                                                   consumable_targets=[targets[0]])
                    self.assertEqual(legacy_result, selected_result)
                    self.assertNotIn('recipe_observation_source', selected_result['crafting_plan']['recipes'][0])
                    mixed = await lookup(restock_recipes=[{'snapshot_id': legacy_id}, selectors[1]])
                    self.assertEqual(mixed['crafting_plan']['combined_materials'][0]['required'], 17)
                    for bad_id in bad_recipes:
                        await lookup(error=True, restock_recipes=[{'snapshot_id': bad_id, 'recipe_id': 2330}])
                    for bad in [[], selectors * 5, [selectors[0], selectors[0]],
                                [{'snapshot_id': bulk_id, 'recipe_id': 0}], [{'snapshot_id': bulk_id, 'recipe_id': True}],
                                [{'snapshot_id': bulk_id, 'recipe_id': '2330'}], [{'snapshot_id': bulk_id, 'recipe_id': 999}],
                                [{'snapshot_id': '../PRIVATE', 'recipe_id': 2330}],
                                [{'snapshot_id': bulk_id, 'recipe_id': 2330, 'private': 'PRIVATE'}]]:
                        await lookup(error=True, restock_recipes=bad)
                    await lookup(error=True, restock_recipe_snapshot_ids=[legacy_id])
                    await lookup(error=True, consumable_targets=None)
                    await lookup(error=True, acquisition_snapshot_id=incomplete_id)
                    await lookup(error=True, consumable_targets=[{'item_id': 118, 'quantity': 1}],
                                 restock_recipes=[selectors[0]])
            self.assertEqual(original, {path.name: path.read_bytes() for path in Path(folder).glob('*.json')})
