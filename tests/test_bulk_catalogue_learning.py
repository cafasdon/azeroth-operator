"""Use bulk/tracked exact recipe observations in local catalogue learning joins."""
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


class BulkCatalogueLearningTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_catalogue_bulk_tracking_output_and_conflicts(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            cases = []
            for kind in ['missing', 'unavailable', 'budget_exhausted', 'unsupported_required_slots',
                         'fixed_items', 'wrong_output', 'unknown_output']:
                for craft, recraft in [(None, None), (False, None), (True, None),
                                       (None, False), (None, True), (True, False)]:
                    value = deepcopy(base)
                    status = 'fixed_items' if kind in ['wrong_output', 'unknown_output'] else kind
                    if kind != 'missing':
                        row = {'status': status}
                        if status == 'fixed_items':
                            row['reagents'] = {'2840': 5}
                            if kind != 'unknown_output':
                                row['output_item_id'] = 3844 if kind == 'wrong_output' else 2870
                        value['sections']['profession_recipe_materials'] = {
                            'coverage': 'reported_known_spell_schematics', 'profession_id': 164,
                            'entries': {'2675': row}}
                    if craft is not None or recraft is not None:
                        value['sections']['tracked_recipes'] = {'coverage': 'player_tracked_recipes_only',
                            'crafts': {} if craft is None else {'2675': {'name': 'PRIVATE_TRACKED', 'learned': craft}},
                            'recrafts': {} if recraft is None else {'2675': {'learned': recraft}}}
                    positive = status in ['fixed_items', 'unsupported_required_slots']
                    flags = ([True] if positive else []) + [flag for flag in [craft, recraft] if flag is not None]
                    conflict = True in flags and False in flags
                    learned = None if conflict or kind == 'wrong_output' else flags[0] if flags else None
                    cases.append((store.ingest(value), kind, craft, recraft, conflict, learned))
            other_conflicts = []
            for section in ['selected_recipe', 'profession_recipe_view', 'profession_spells']:
                value = deepcopy(base)
                value['sections']['profession_recipe_materials'] = {
                    'coverage': 'reported_known_spell_schematics', 'profession_id': 164,
                    'entries': {'2675': {'status': 'fixed_items', 'reagents': {}, 'output_item_id': 2870}}}
                if section == 'selected_recipe':
                    value['sections'][section] = {'recipe_id': 2675, 'name': 'PRIVATE_SELECTED', 'learned': False,
                        'requirements': 'fixed_items', 'reagents': {}, 'output_item_id': 2870}
                elif section == 'profession_recipe_view':
                    value['sections'][section] = {'coverage': 'filtered_local_recipe_list', 'profession_id': 165,
                        'entries': {'2675': {'learned': False}}}
                else:
                    value['sections'][section] = {'coverage': 'reported_known_profession_spells',
                        'profession_id': 165, 'entries': {'2675': {}}}
                    value['sections']['tracked_recipes'] = {'coverage': 'player_tracked_recipes_only',
                        'crafts': {'2675': {'learned': False}}, 'recrafts': {}}
                other_conflicts.append(store.ingest(value))
            enchant_keys = []
            for output in [2304, 3844, None]:
                value = deepcopy(base)
                row = {'status': 'fixed_items', 'reagents': {}}
                if output is not None:
                    row['output_item_id'] = output
                value['sections']['profession_recipe_materials'] = {
                    'coverage': 'reported_known_spell_schematics', 'profession_id': 165,
                    'entries': {'2152': row}}
                enchant_keys.append(store.ingest(value))
            original = {path.name: path.read_bytes() for path in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def lookup(snapshot_id, **options):
                        reply = await session.call_tool('lookup_forever_catalogue', {'section': 'items',
                            'key': '2870', 'acquisition_snapshot_id': snapshot_id, **options})
                        self.assertFalse(reply.isError, reply)
                        result = json.loads(reply.content[0].text)
                        self.assertNotIn('PRIVATE', json.dumps(result))
                        return result
                    for key, kind, craft, recraft, conflict, expected in cases:
                        result = await lookup(key)
                        source = next(row for row in result['acquisition_progress']['sources'] if row.get('recipe_spell_id') == 2675)
                        learning = source['recipe_learning']
                        self.assertEqual(learning['bulk_recipe_observed'], kind != 'missing')
                        self.assertEqual(learning['tracked_craft_learned'], craft)
                        self.assertEqual(learning['tracked_recraft_learned'], recraft)
                        self.assertEqual(learning['learning_conflict'], conflict, (kind, craft, recraft))
                        self.assertEqual(learning['learned'], expected, (kind, craft, recraft))
                        self.assertEqual(learning['bulk_output_matches_catalogue'],
                                         False if kind == 'wrong_output' else True if kind == 'fixed_items' else None)
                        self.assertIsNone(source['material_capacity'])
                        self.assertIsNone(source['learned'])
                        filtered = await lookup(key, only_observed_learned_recipe=True)
                        self.assertEqual(bool(filtered['records']), expected is True)
                    for key in other_conflicts:
                        self.assertIsNone((await lookup(key))['acquisition_progress']['sources'][0]['recipe_learning']['learned'])
                        self.assertEqual((await lookup(key, only_observed_learned_recipe=True))['records'], [])
                    for key, expected in zip(enchant_keys, [True, False, True]):
                        result = await lookup(key, section='enchants', key=None, recipe_spell_id=2152,
                                              only_observed_learned_recipe=True)
                        self.assertEqual(bool(result['records']), expected)
                        if expected:
                            self.assertTrue(all(row['itemId'] == 2304 for row in result['records']))
            self.assertEqual(original, {path.name: path.read_bytes() for path in Path(folder).glob('*.json')})
