"""Real offline MCP caller for combined saved restock quotes."""
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


class RestockQuoteTests(unittest.IsolatedAsyncioTestCase):
    async def test_saved_quotes(self):
        root = Path(__file__).resolve().parents[1]
        base = sample()
        base['sections'].update(wallet={'copper': 6}, merchant_stock={
            '1': {'item_id': 1, 'bundle_size': 2, 'price_copper': 5,
                  'available': -1, 'extended_cost': False}})
        # Bank holdings must not erase the three-unit bag shortage.
        base['sections']['character_bank']['6']['2'] = {'item_id': 1, 'count': 100}
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder:
            store = SnapshotStore(Path(folder))
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            binary = root/'target/debug'/('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def call(value=base, **extra):
                        reply = await session.call_tool('plan_forever_recipe_restock', {
                            'snapshot_id': store.ingest(value), 'recipe_id': 100,
                            'target_bag_quantity': 2, **extra})
                        self.assertFalse(reply.isError, reply)
                        return json.loads(reply.content[0].text)
                    self.assertNotIn('merchant_quote', await call())
                    targets = [{'recipe_id': 200, 'target_bag_quantity': 2}]
                    report = await call(merchant_quote=True, additional_targets=targets)
                    quote = report['merchant_quote']
                    self.assertEqual(quote['status'], 'saved_quote')
                    budget = quote['budget']
                    self.assertEqual([r['item_id'] for r in budget['reagents']], [1])
                    self.assertEqual(budget['reagents'][0]['shortfall_vs_observed'], 3)
                    self.assertEqual(budget['copper_only_bundle_mix_total'], 10)
                    self.assertFalse(budget['copper_only_bundle_mix_affordable'])
                    self.assertEqual(budget['copper_only_bundle_mix_shortfall'], 4)
                    self.assertFalse(quote['bank_credit'])
                    self.assertFalse(quote['game_execution'])
                    single = (await call(merchant_quote=True))['merchant_quote']
                    self.assertEqual(single['budget']['copper_only_bundle_mix_total'], 0)
                    zero = (await call(merchant_quote=True, target_bag_quantity=1))['merchant_quote']
                    self.assertEqual(zero['budget']['reagents'], [])
                    missing = deepcopy(base); missing['sections'].pop('merchant_stock')
                    self.assertEqual((await call(missing, merchant_quote=True))['merchant_quote']['status'],
                                     'merchant_observation_unavailable')
                    unknown = deepcopy(base); unknown['sections'].pop('wallet')
                    self.assertIsNone((await call(unknown, merchant_quote=True, additional_targets=targets))
                                      ['merchant_quote']['budget']['copper_only_bundle_mix_affordable'])
                    unsupported = deepcopy(base)
                    unsupported['sections']['merchant_stock']['1']['extended_cost'] = True
                    self.assertIsNone((await call(unsupported, merchant_quote=True, additional_targets=targets))
                                      ['merchant_quote']['budget']['copper_only_bundle_mix_total'])
                    cycle = sample(0)
                    entries = cycle['sections']['profession_recipe_materials']['entries']
                    entries['100']['reagents'] = {'20': 1}
                    entries['200']['reagents'] = {'10': 1}
                    entries['200']['base_output_quantity'] = {'minimum': 2, 'maximum': 2}
                    self.assertEqual((await call(cycle, merchant_quote=True, additional_targets=targets))
                                     ['merchant_quote']['status'], 'material_order_unavailable')
