"""Plan exact bulk-observed recipes through the actual offline MCP route."""
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


class BulkRecipePlanningTests(unittest.IsolatedAsyncioTestCase):
    async def test_single_batch_unknowns_and_context(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        base['sections']['bags'] = {'0': {'1': {'item_id': 7, 'count': 2}}}
        base['sections']['character_bank'] = {}
        base['sections']['account_bank'] = {}
        base['sections']['wallet'] = {'copper': 10}
        base['sections']['merchant_stock'] = {'1': {'item_id': 7, 'bundle_size': 1,
            'price_copper': 4, 'available': -1, 'extended_cost': False}}
        rows = {}
        for recipe_id, output, reagent, quantity in [(101, 30, 20, 1), (102, 20, 10, 1), (103, 10, 7, 2)]:
            rows[str(recipe_id)] = {'status': 'fixed_items', 'reagents': {str(reagent): quantity},
                                   'output_item_id': output, 'base_output_quantity': {'minimum': 1, 'maximum': 2}}
        rows.update({'104': {'status': 'unsupported_required_slots'}, '105': {'status': 'unavailable'},
                     '106': {'status': 'budget_exhausted'}, '107': {'status': 'fixed_items', 'reagents': {}}})
        base['sections']['profession_recipe_materials'] = {
            'coverage': 'reported_known_spell_schematics', 'profession_id': 164, 'entries': rows}
        base['sections']['selected_recipe'] = {'recipe_id': 999, 'name': 'PRIVATE_SELECTED_NAME', 'learned': True,
                                              'requirements': 'fixed_items', 'reagents': {}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            key = store.ingest(base)
            missing = deepcopy(base)
            del missing['sections']['profession_recipe_materials']
            missing_id = store.ingest(missing)
            inventories = []
            for field, value in [('name', 'Other'), ('build', 'other'), ('captured_at', '2024-12-31T00:00:00Z')]:
                inventory = deepcopy(base)
                if field == 'name':
                    inventory['character'][field] = value
                elif field == 'build':
                    inventory['client'][field] = value
                else:
                    inventory[field] = value
                inventories.append(store.ingest(inventory))
            later = deepcopy(base)
            later['captured_at'] = '2025-01-02T00:00:00Z'
            later['sections']['bags']['0']['1']['count'] = 1
            later_id = store.ingest(later)
            conflicts = []
            selected = {'recipe_id': 103, 'name': 'PRIVATE_SELECTED_NAME', 'learned': True,
                        'requirements': 'fixed_items', 'reagents': {'7': 2}, 'output_item_id': 10,
                        'base_output_quantity': {'minimum': 1, 'maximum': 2}}
            for field, value in [('learned', False), ('reagents', {'7': 3}), ('output_item_id', 11),
                                 ('base_output_quantity', {'minimum': 2, 'maximum': 2}),
                                 ('profession', {'profession_id': 165, 'name': 'PRIVATE', 'rank': 1, 'max_rank': 2})]:
                capture = deepcopy(base)
                capture['sections']['selected_recipe'] = {**selected, field: value}
                conflicts.append(store.ingest(capture))
            capture = deepcopy(base)
            capture['sections']['profession_recipe_view'] = {'coverage': 'filtered_local_recipe_list',
                'profession_id': 164, 'entries': {'103': {'learned': False}}}
            conflicts.append(store.ingest(capture))
            capture = deepcopy(capture)
            capture['sections']['profession_recipe_view']['profession_id'] = 165
            conflicts.append(store.ingest(capture))
            for group in ['crafts', 'recrafts']:
                capture = deepcopy(base)
                capture['sections']['tracked_recipes'] = {'coverage': 'player_tracked_recipes_only',
                    'crafts': {}, 'recrafts': {}}
                capture['sections']['tracked_recipes'][group]['103'] = {'learned': False}
                conflicts.append(store.ingest(capture))
            compatible = []
            for learned in [None, True]:
                capture = deepcopy(base)
                capture['sections']['tracked_recipes'] = {'coverage': 'player_tracked_recipes_only',
                    'crafts': {'103': {} if learned is None else {'learned': learned}}, 'recrafts': {}}
                compatible.append(store.ingest(capture))
            capture = deepcopy(base)
            capture['sections']['profession_recipe_view'] = {'coverage': 'filtered_local_recipe_list',
                'profession_id': 165, 'entries': {'103': {'learned': True}, '999': {'learned': False}}}
            compatible.append(store.ingest(capture))
            capture = deepcopy(base)
            capture['sections']['profession_spells'] = {'coverage': 'reported_known_profession_spells',
                'profession_id': 165, 'entries': {'103': {}}}
            compatible.append(store.ingest(capture))
            original = {path.name: path.read_bytes() for path in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def call(error=False, **options):
                        args = {'snapshot_id': key, 'recipe_id': 103, 'crafts': 1, **options}
                        if args.get('crafts') is None:
                            del args['crafts']
                        reply = await session.call_tool('review_forever_recipe', args)
                        self.assertEqual(bool(reply.isError), error, reply)
                        if error:
                            self.assertNotIn('PRIVATE', str(reply))
                            return
                        return json.loads(reply.content[0].text)
                    result = await call()
                    self.assertEqual(result['recipe']['recipe_id'], 103)
                    self.assertEqual(result['recipe_observation_source'], 'profession_recipe_materials')
                    self.assertEqual(result['recipe_profession_id'], 164)
                    self.assertEqual(result['materials'][0]['required'], 2)
                    self.assertEqual(result['materials'][0]['shortfall_vs_observed'], 0)
                    self.assertEqual(result['material_capacity']['bags']['supported_crafts'], 1)
                    self.assertTrue(result['captured_preparation']['recipe_learned'])
                    self.assertIsNone(result['captured_preparation']['observed_conditions_met'])
                    self.assertEqual(result['captured_cooldown_state'], 'unknown')
                    self.assertIsNone(result['unmet_crafting_conditions'])
                    self.assertFalse(result['craftability_verified'])
                    self.assertFalse(result['game_execution'])
                    self.assertNotIn('PRIVATE', str(result))
                    target = await call(crafts=None, target_output_quantity=3)
                    self.assertEqual(target['requested_crafts'], 3)
                    self.assertEqual(target['base_output_if_completed'], {'item_id': 10, 'minimum': 3, 'maximum': 6})
                    self.assertEqual((await call(recipe_id=None))['recipe']['recipe_id'], 999)
                    self.assertNotIn('recipe_observation_source', await call(recipe_id=None))
                    newer = await call(inventory_snapshot_id=later_id)
                    self.assertEqual(newer['materials'][0]['shortfall_vs_observed'], 1)
                    self.assertIsNone((await call(recipe_id=104))['material_capacity']['bags'])
                    self.assertEqual((await call(recipe_id=107))['materials'], [])
                    batch = await call(recipe_id=101, additional_recipes=[
                        {'snapshot_id': key, 'recipe_id': 102, 'crafts': 1},
                        {'snapshot_id': key, 'recipe_id': 103, 'crafts': 1}], suggest_dependency_order=True)
                    self.assertEqual(batch['dependency_order']['suggested_recipe_numbers'], [3, 2, 1])
                    self.assertTrue(batch['dependency_order']['material_coverage']['bags'])
                    self.assertEqual([r['recipe']['recipe_id'] for r in batch['additional_recipe_reviews']], [102, 103])
                    quoted = await call(recipe_id=101, inventory_snapshot_id=later_id,
                        merchant_reagent_plan=True, suggest_dependency_order=True, additional_recipes=[
                            {'snapshot_id': key, 'recipe_id': 102, 'crafts': 1},
                            {'snapshot_id': key, 'recipe_id': 103, 'crafts': 1}])
                    self.assertEqual(quoted['dependency_order']['merchant_reagent_plan']['unlimited_copper_only_total'], 4)
                    mixed = await call(additional_recipes=[{'snapshot_id': key, 'crafts': 1}])
                    self.assertEqual(mixed['additional_recipe_reviews'][0]['recipe']['recipe_id'], 999)
                    for bad in [0, -1, 4294967296, True, '103', 105, 106, 108]:
                        await call(error=True, recipe_id=bad)
                    for bad in [0, 1001]:
                        await call(error=True, crafts=bad)
                    await call(error=True, target_output_quantity=2)
                    await call(error=True, recipe_id=107, crafts=None, target_output_quantity=2)
                    await call(error=True, snapshot_id=missing_id)
                    await call(error=True, additional_recipes=[{'snapshot_id': key, 'recipe_id': 103, 'crafts': 1}])
                    await call(error=True, additional_recipes=[{'snapshot_id': key, 'recipe_id': 0, 'crafts': 1}])
                    for inventory_id in inventories:
                        await call(error=True, inventory_snapshot_id=inventory_id)
                    for conflict_id in conflicts:
                        await call(error=True, snapshot_id=conflict_id)
                    for compatible_id in compatible:
                        self.assertTrue((await call(snapshot_id=compatible_id))['captured_preparation']['recipe_learned'])
            self.assertEqual(original, {path.name: path.read_bytes() for path in Path(folder).glob('*.json')})
