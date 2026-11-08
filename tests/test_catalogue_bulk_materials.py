"""Catalogue crafted sources expose separately qualified bulk material reviews."""
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


class CatalogueBulkMaterialsTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_bulk_material_reviews_and_unknowns(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned local catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        base['sections']['bags'] = {'0': {'1': {'item_id': 2840, 'count': 7}, '2': {'item_id': 3470, 'count': 1}}}
        base['sections']['character_bank'] = {'6': {'1': {'item_id': 2840, 'count': 8}}}
        fixed = {'status': 'fixed_items', 'reagents': {'2840': 5, '3470': 2}, 'output_item_id': 2870,
                 'base_output_quantity': {'minimum': 1, 'maximum': 2}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            keys = {}
            for kind in ['missing', 'fixed', 'unsupported_required_slots', 'unavailable', 'budget_exhausted',
                         'wrong_output', 'unknown_output', 'learning_conflict', 'material_conflict', 'missing_inventory']:
                value = deepcopy(base)
                if kind != 'missing':
                    row = deepcopy(fixed) if kind not in ['unsupported_required_slots', 'unavailable', 'budget_exhausted'] else {'status': kind}
                    if kind == 'wrong_output':
                        row['output_item_id'] = 3844
                    if kind == 'unknown_output':
                        del row['output_item_id']
                        del row['base_output_quantity']
                    value['sections']['profession_recipe_materials'] = {
                        'coverage': 'reported_known_spell_schematics', 'profession_id': 164, 'entries': {'2675': row}}
                if kind == 'learning_conflict':
                    value['sections']['tracked_recipes'] = {'coverage': 'player_tracked_recipes_only',
                        'crafts': {'2675': {'name': 'PRIVATE_TRACKED', 'learned': False}}, 'recrafts': {}}
                else:
                    value['sections']['selected_recipe'] = {'recipe_id': 999, 'name': 'PRIVATE_SELECTED',
                        'learned': True, 'requirements': 'fixed_items', 'reagents': {}}
                if kind == 'material_conflict':
                    value['sections']['selected_recipe'] = {'recipe_id': 2675, 'name': 'PRIVATE_SELECTED',
                        'learned': True, 'requirements': 'fixed_items', 'reagents': {'2840': 6}, 'output_item_id': 2870}
                if kind == 'missing_inventory':
                    del value['sections']['bags']
                    del value['sections']['character_bank']
                keys[kind] = store.ingest(value)
            original = {path.name: path.read_bytes() for path in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def call(tool, **args):
                        reply = await session.call_tool(tool, args)
                        self.assertFalse(reply.isError, reply)
                        result = json.loads(reply.content[0].text)
                        self.assertNotIn('PRIVATE', json.dumps(result))
                        return result
                    for kind, key in keys.items():
                        result = await call('lookup_forever_catalogue', section='items', key='2870', acquisition_snapshot_id=key)
                        source = next(row for row in result['acquisition_progress']['sources'] if row.get('recipe_spell_id') == 2675)
                        bulk = source['bulk_recipe_review']
                        if kind == 'missing':
                            self.assertIsNone(bulk)
                            continue
                        expected = {'wrong_output': 'catalogue_output_mismatch', 'learning_conflict': 'learning_conflict',
                                    'material_conflict': 'review_unavailable', 'unavailable': 'unavailable',
                                    'budget_exhausted': 'budget_exhausted'}.get(kind, 'reviewed')
                        self.assertEqual(bulk['status'], expected, kind)
                        if expected != 'reviewed':
                            self.assertIsNone(bulk['review'])
                            continue
                        direct = await call('review_forever_recipe', snapshot_id=key, recipe_id=2675, crafts=1)
                        self.assertEqual(bulk['review'], direct)
                        self.assertFalse(direct['craftability_verified'])
                        self.assertFalse(direct['game_execution'])
                        self.assertIsNone(direct['unmet_crafting_conditions'])
                        self.assertEqual(direct['captured_cooldown_state'], 'unknown')
                        self.assertIsNone(source['material_capacity'])
                        if kind == 'fixed':
                            materials = {row['item_id']: row for row in direct['materials']}
                            self.assertEqual(materials[2840]['quantity_observed'], 15)
                            self.assertEqual(materials[3470]['shortfall_vs_observed'], 1)
                            self.assertIsNone(materials[2840]['quantity_by_storage']['account_bank'])
                        if kind == 'unsupported_required_slots':
                            self.assertEqual(direct['recipe']['requirements'], 'unsupported_required_slots')
                            self.assertIsNone(direct['material_capacity']['bags'])
                        if kind == 'missing_inventory':
                            self.assertIsNone(direct['material_capacity']['all_observed_storage'])
                        if kind == 'unknown_output':
                            self.assertIsNone(direct['base_output_if_completed'])
            self.assertEqual(original, {path.name: path.read_bytes() for path in Path(folder).glob('*.json')})
