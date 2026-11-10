"""Bulk material/yield observation differences through the real offline MCP."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_recipe_restock_batch_mcp import sample


def fixed(item=10, count=2):
    return {'status': 'fixed_items', 'reagents': {'1': count},
            'output_item_id': item, 'base_output_quantity': {'minimum': 1, 'maximum': 2}}


class RecipeMaterialCompareTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_pages_unknowns_and_owner_boundaries(self):
        root = Path(__file__).resolve().parents[1]
        before = sample()
        before['sections']['profession_recipe_materials']['entries'] = {
            '100': fixed(), '200': {'status': 'budget_exhausted'},
            '300': {'status': 'unavailable'}, '400': {'status': 'unsupported_required_slots'},
            '500': {**fixed(), 'cooldown': {'remaining_seconds': 30}}, '600': fixed()}
        after = deepcopy(before); after['captured_at'] = '2025-01-02T00:00:00Z'
        entries = after['sections']['profession_recipe_materials']['entries']
        entries['100'] = fixed(count=3)
        entries['100']['base_output_quantity'] = {'minimum': 2, 'maximum': 3}
        entries['200'] = fixed(20)
        entries['400'] = {'status': 'unavailable'}
        entries['500']['cooldown']['remaining_seconds'] = 0
        entries['500']['reported_craftable_count'] = 5
        del entries['600']; entries['900'] = fixed(90)
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder:
            store = SnapshotStore(Path(folder)); a = store.ingest(before); b = store.ingest(after)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            binary = root/'target/debug'/('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    route = next(t for t in (await session.list_tools()).tools if t.name == 'compare_snapshots')
                    def objects(value):
                        if isinstance(value, dict):
                            yield value
                            for child in value.values(): yield from objects(child)
                        elif isinstance(value, list):
                            for child in value: yield from objects(child)
                    schemas = [obj for obj in objects(route.inputSchema)
                               if set(obj.get('properties', {})) == {'offset', 'limit'}]
                    self.assertEqual(len(schemas), 1)
                    self.assertFalse(schemas[0]['additionalProperties'])
                    properties = schemas[0]['properties']
                    self.assertEqual(properties['offset']['default'], 0)
                    self.assertEqual(properties['offset']['format'], 'uint32')
                    self.assertEqual(properties['limit']['minimum'], 1)
                    self.assertEqual(properties['limit']['maximum'], 50)
                    self.assertEqual(properties['limit']['default'], 20)
                    async def call(old=a, new=b, error=False, **extra):
                        reply = await session.call_tool('compare_snapshots', {
                            'before_id': old, 'after_id': new, **extra})
                        self.assertEqual(bool(reply.isError), error, reply)
                        self.assertNotIn('PRIVATE_MARKER', str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    baseline = await call()
                    self.assertNotIn('recipe_material_changes', baseline)
                    self.assertEqual(await call(recipe_material_changes=None), baseline)
                    async def compare(**page):
                        return (await call(recipe_material_changes=page))['recipe_material_changes']
                    report = await compare()
                    self.assertEqual(report['status'], 'comparable')
                    self.assertEqual(report['counts'], {'changed_shared_recipes': 3,
                        'unchanged_shared_recipes': 2, 'observed_only_before': 1, 'observed_only_after': 1})
                    self.assertEqual(report['total_changes'], 5)
                    self.assertEqual([r['recipe_id'] for r in report['rows']], [100, 200, 400, 600, 900])
                    self.assertEqual(report['rows'][0]['changed_fields'], ['reagents', 'base_output_quantity'])
                    self.assertIsNone(report['rows'][3]['after'])
                    self.assertIsNone(report['rows'][4]['before'])
                    for flag in ['absence_means_unlearned', 'complete_profession_coverage_verified',
                                 'recipe_definition_change_verified', 'craftability_verified', 'game_execution', 'live']:
                        self.assertFalse(report[flag])
                    pages = []; offset = 0
                    while True:
                        page = await compare(offset=offset, limit=2)
                        self.assertEqual(page['counts'], report['counts'])
                        pages.extend(page['rows'])
                        if not page['has_more']: break
                        offset = page['next_offset']
                    self.assertEqual(pages, report['rows'])
                    self.assertEqual((await compare(offset=4294967295))['rows'], [])
                    self.assertFalse((await compare(offset=4294967295))['has_more'])
                    same = (await call(new=a, recipe_material_changes={}))['recipe_material_changes']
                    self.assertEqual(same['total_changes'], 0)
                    self.assertEqual(same['counts']['unchanged_shared_recipes'], 6)
                    mismatch = deepcopy(after)
                    mismatch['sections']['profession_recipe_materials']['profession_id'] = 171
                    result = (await call(new=store.ingest(mismatch), recipe_material_changes={}))['recipe_material_changes']
                    self.assertEqual(result['status'], 'profession_mismatch'); self.assertIsNone(result['rows'])
                    missing = deepcopy(after); missing['sections'].pop('profession_recipe_materials')
                    result = (await call(new=store.ingest(missing), recipe_material_changes={}))['recipe_material_changes']
                    self.assertEqual(result['status'], 'section_not_recorded'); self.assertIsNone(result['counts'])
                    # Numeric recipe order must differ from lexicographic order and pages must reach beyond 50.
                    many = deepcopy(after)
                    many['sections']['profession_recipe_materials']['entries'] = {
                        str(i): fixed() for i in range(1, 64)}
                    empty = deepcopy(before); empty['sections']['profession_recipe_materials']['entries'] = {}
                    page = (await call(old=store.ingest(empty), new=store.ingest(many),
                                      recipe_material_changes={'offset': 50, 'limit': 50}))['recipe_material_changes']
                    self.assertEqual([r['recipe_id'] for r in page['rows']], list(range(51, 64)))
                    self.assertEqual(page['total_changes'], 63)
                    for bad in [[], True, 'PRIVATE_MARKER', {'limit': 0}, {'limit': 51},
                                {'offset': -1}, {'offset': 4294967296}, {'limit': True},
                                {'PRIVATE_MARKER': 1}, {'limit': 'PRIVATE_MARKER'}]:
                        await call(error=True, recipe_material_changes=bad)
                    await call(old=b, new=a, error=True, recipe_material_changes={})
                    for mutate in [lambda v: v['character'].update(name='PRIVATE_MARKER'),
                                   lambda v: v['client'].update(build='99999')]:
                        wrong = deepcopy(after); mutate(wrong)
                        await call(new=store.ingest(wrong), error=True, recipe_material_changes={})
                    files = {p: p.read_bytes() for p in Path(folder).glob('*.json')}
                    await compare(limit=1)
                    self.assertEqual(files, {p: p.read_bytes() for p in files})
