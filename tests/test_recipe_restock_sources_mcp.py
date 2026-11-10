"""Restock shortages join saved provider references without new external reads."""
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
from test_chain_captured_sources import refs


def with_sources():
    value = sample()
    value['sections']['character_bank']['6']['2'] = {'item_id': 1, 'count': 100}
    value['sections']['recipe_item_sources'] = {
        'coverage': 'first_reported_fixed_recipe_reagents', 'provider': 'questiedb',
        'provider_mode': 'baked', 'provider_contract_version': 4, 'profession_id': 164,
        'reported_item_count': 2, 'entries': {
            '1': {'status': 'reported', 'vendors': refs([5]), 'npc_drops': refs(list(range(10, 18)), 10),
                  'object_drops': refs([]), 'quest_rewards': refs([77])},
            '10': {'status': 'reported', 'vendors': refs([99])}},
        'entity_locations': {'coordinate_system': 'questiedb_area_percent',
            'reported_npc_count': 2, 'reported_object_count': 0,
            'npcs': {'5': {'status': 'reported', 'spawns': {
                'reported_point_count': 1, 'truncated': False, 'entries': {
                    '1': {'kind': 'coordinate', 'area_id': 12, 'x_percent': 25, 'y_percent': 50}}}},
                '99': {'status': 'not_in_provider'}}, 'objects': {}}}
    cache = value['sections']['recipe_item_sources']['entity_locations']
    cache['npcs'].update({str(i): {'status': 'unavailable'} for i in range(10, 18)})
    cache['reported_npc_count'] = 10
    return value


class RestockSourcesTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_shared_demand_sources_unknowns_and_composition(self):
        root = Path(__file__).resolve().parents[1]
        base = with_sources()
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder, tempfile.TemporaryDirectory(dir=root/'local-data') as other:
            store = SnapshotStore(Path(folder))
            foreign = deepcopy(base); foreign['character']['name'] = 'PRIVATE_FOREIGN'
            foreign_key = SnapshotStore(Path(other)).ingest(foreign)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            binary = root/'target/debug'/('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    route = next(t for t in (await session.list_tools()).tools if t.name == 'plan_forever_recipe_restock')
                    self.assertEqual(route.inputSchema['properties']['captured_sources']['type'], 'boolean')
                    self.assertFalse(route.inputSchema['properties']['captured_sources']['default'])
                    async def call(value=base, snapshot=None, error=False, **extra):
                        reply = await session.call_tool('plan_forever_recipe_restock', {
                            'snapshot_id': snapshot or store.ingest(value), 'recipe_id': 100,
                            'target_bag_quantity': 2, **extra})
                        self.assertEqual(bool(reply.isError), error, reply)
                        self.assertNotIn('PRIVATE_FOREIGN', str(reply)); self.assertNotIn(other, str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    baseline = await call()
                    self.assertNotIn('captured_sources', baseline)
                    self.assertEqual(await call(captured_sources=False), baseline)
                    targets = [{'recipe_id': 200, 'target_bag_quantity': 2}]
                    report = await call(captured_sources=True, additional_targets=targets)
                    source = report['captured_sources']
                    self.assertEqual(source['status'], 'saved_reference_review')
                    self.assertFalse(source['bank_credit']); self.assertFalse(source['acquisition_plan_complete'])
                    saved = source['source_review']
                    self.assertEqual(saved['external_item_count'], 1)
                    self.assertEqual([(r['item_id'], r['quantity']) for r in saved['records']], [(1, 3)])
                    row = saved['records'][0]
                    self.assertEqual(row['references']['vendors'], refs([5]))
                    self.assertEqual(row['references']['quest_rewards'], refs([77]))
                    self.assertEqual(row['truncated_reference_categories'], ['npc_drops'])
                    self.assertEqual(row['unknown_reference_categories'], ['container_drops'])
                    locations = saved['entity_locations']
                    self.assertNotIn('99', locations['npcs'])
                    self.assertEqual(locations['npcs']['5']['status'], 'reported')
                    self.assertFalse(locations['routing_verified'])
                    self.assertEqual(saved['source_capture_coverage']['provider_contract_version'], 4)
                    self.assertEqual(saved['captured_at'], base['captured_at'])
                    for flag in ['current_availability_verified', 'acquisition_eligibility_verified',
                                 'ownership_verified', 'complete_world_sources_verified', 'game_execution', 'live']:
                        self.assertFalse(saved[flag])
                    combined = await call(captured_sources=True, merchant_quote=True, stock_schedule={}, additional_targets=targets)
                    self.assertEqual(combined['captured_sources'], source)
                    self.assertIn('merchant_quote', combined); self.assertIn('stock_schedule', combined)
                    # Single target uses the same conservative gross bag basis, with no demand at target 1.
                    self.assertEqual((await call(captured_sources=True, target_bag_quantity=10))
                                     ['captured_sources']['source_review']['records'][0]['quantity'], 5)
                    self.assertEqual((await call(captured_sources=True, target_bag_quantity=1))
                                     ['captured_sources']['source_review']['records'], [])
                    for status in ['not_in_provider', 'unavailable']:
                        value = deepcopy(base); value['sections']['recipe_item_sources']['entries']['1'] = {'status': status}
                        value['sections']['recipe_item_sources'].pop('entity_locations')
                        observed = (await call(value, captured_sources=True, additional_targets=targets))['captured_sources']['source_review']
                        self.assertEqual(observed['records'][0]['status'], status)
                        self.assertIsNone(observed['records'][0]['references'])
                    value = deepcopy(base); del value['sections']['recipe_item_sources']['entries']['1']
                    value['sections']['recipe_item_sources']['reported_item_count'] = 1
                    value['sections']['recipe_item_sources'].pop('entity_locations')
                    observed = (await call(value, captured_sources=True, additional_targets=targets))['captured_sources']['source_review']
                    self.assertEqual(observed['records'][0]['status'], 'item_not_captured')
                    value = deepcopy(base); del value['sections']['recipe_item_sources']
                    observed = (await call(value, captured_sources=True, additional_targets=targets))['captured_sources']['source_review']
                    self.assertFalse(observed['section_available'])
                    self.assertEqual(observed['records'][0]['status'], 'capture_unavailable')
                    cycle = sample(0); entries = cycle['sections']['profession_recipe_materials']['entries']
                    entries['100']['reagents'] = {'20': 1}; entries['200']['reagents'] = {'10': 1}
                    entries['200']['base_output_quantity'] = {'minimum': 2, 'maximum': 2}
                    self.assertEqual((await call(cycle, captured_sources=True, additional_targets=targets))
                                     ['captured_sources']['status'], 'material_order_unavailable')
                    await call(snapshot=foreign_key, error=True, captured_sources=True)
                    for bad in [None, 'true', 1, [], {}]:
                        await call(error=True, captured_sources=bad)
                    files = {p: p.read_bytes() for p in Path(folder).glob('*.json')}
                    await call(captured_sources=True, additional_targets=targets)
                    self.assertEqual(files, {p: p.read_bytes() for p in files})
