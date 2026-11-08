"""Price shared consumable reagent shortfalls against captured offers only."""
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


class RestockReagentQuoteTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_combined_quotes_wallet_provenance_and_unknowns(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned local catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        base['sections']['bags'] = {'0': {'1': {'item_id': 118, 'count': 1}, '2': {'item_id': 2447, 'count': 10}}}
        base['sections']['character_bank'] = {}
        base['sections']['account_bank'] = {}
        base['sections']['wallet'] = {'copper': 3}
        offer = {'item_id': 2447, 'bundle_size': 5, 'price_copper': 2, 'available': -1, 'extended_cost': False}
        base['sections']['merchant_stock'] = {'1': offer}
        base['sections']['profession_recipe_materials'] = {
            'coverage': 'reported_known_spell_schematics', 'profession_id': 171, 'entries': {
                '2330': {'status': 'fixed_items', 'reagents': {'2447': 2}, 'output_item_id': 118,
                         'base_output_quantity': {'minimum': 1, 'maximum': 2}},
                '2337': {'status': 'fixed_items', 'reagents': {'2447': 3}, 'output_item_id': 858,
                         'base_output_quantity': {'minimum': 2, 'maximum': 3}}}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            key = store.ingest(base)
            captures = {}
            for kind in ['offers', 'cheap', 'missing', 'unmatched', 'limited', 'extended', 'foreign', 'realm', 'build', 'no_wallet', 'covered']:
                value = deepcopy(base)
                value['captured_at'] = '2025-01-02T00:00:00Z'
                value['sections']['wallet']['copper'] = 999
                if kind == 'cheap':
                    value['sections']['merchant_stock']['1']['price_copper'] = 1
                elif kind == 'missing':
                    del value['sections']['merchant_stock']
                elif kind == 'unmatched':
                    value['sections']['merchant_stock']['1']['item_id'] = 2448
                elif kind == 'limited':
                    value['sections']['merchant_stock']['1']['available'] = 5
                elif kind == 'extended':
                    value['sections']['merchant_stock']['1'].update(extended_cost=True, price_copper=0)
                elif kind == 'foreign':
                    value['character']['name'] = 'PRIVATE_OTHER'
                elif kind == 'realm':
                    value['character']['realm'] = 'PRIVATE_REALM'
                elif kind == 'build':
                    value['client']['build'] = 'other'
                elif kind == 'no_wallet':
                    del value['sections']['wallet']
                elif kind == 'covered':
                    value['sections']['bags']['0']['2']['count'] = 20
                captures[kind] = store.ingest(value)
            original = {path.name: path.read_bytes() for path in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            selectors = [{'snapshot_id': key, 'recipe_id': 2330}, {'snapshot_id': key, 'recipe_id': 2337}]
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def lookup(error=False, **options):
                        reply = await session.call_tool('lookup_forever_catalogue', {'section': 'consumables',
                            'acquisition_snapshot_id': key, 'consumable_targets': [{'item_id': 118, 'quantity': 5},
                            {'item_id': 858, 'quantity': 5}], 'restock_recipes': selectors,
                            'restock_merchant_reagent_plan': True, **options})
                        self.assertEqual(bool(reply.isError), error, reply)
                        self.assertNotIn('PRIVATE', str(reply))
                        return None if error else json.loads(reply.content[0].text)['acquisition_progress']['crafting_plan']
                    quoted = await lookup()
                    quote = quoted['merchant_reagent_plan']
                    self.assertEqual(quoted['combined_materials'][0]['required'], 17)
                    self.assertEqual(quote['reagents'][0]['shortfall_vs_observed'], 7)
                    self.assertEqual(quote['reagents'][0]['offers_by_slot']['1']['bundles'], 2)
                    self.assertEqual(quote['reagents'][0]['offers_by_slot']['1']['units'], 10)
                    self.assertEqual(quote['unlimited_copper_only_total'], 4)
                    self.assertFalse(quote['unlimited_copper_only_affordable'])
                    self.assertEqual(quote['unlimited_copper_only_shortfall'], 1)
                    self.assertFalse(quote['game_execution'])
                    self.assertFalse(quote['purchase_eligibility_verified'])
                    external = await lookup(restock_merchant_snapshot_id=captures['offers'])
                    self.assertEqual(external['merchant_snapshot_id'], captures['offers'])
                    self.assertFalse(external['merchant_reagent_plan']['unlimited_copper_only_affordable'])
                    self.assertEqual(external['merchant_reagent_plan']['merchant_captured_at'], '2025-01-02T00:00:00Z')
                    cheap = (await lookup(restock_merchant_snapshot_id=captures['cheap']))['merchant_reagent_plan']
                    self.assertEqual(cheap['unlimited_copper_only_total'], 2)
                    self.assertTrue(cheap['unlimited_copper_only_affordable'])
                    self.assertIsNone((await lookup(restock_merchant_snapshot_id=captures['missing']))['merchant_reagent_plan'])
                    for kind in ['unmatched', 'limited', 'extended']:
                        unknown = (await lookup(restock_merchant_snapshot_id=captures[kind]))['merchant_reagent_plan']
                        self.assertIsNone(unknown['unlimited_copper_only_total'], kind)
                        self.assertIsNone(unknown['unlimited_copper_only_affordable'], kind)
                    unknown_wallet = (await lookup(acquisition_snapshot_id=captures['no_wallet']))['merchant_reagent_plan']
                    self.assertEqual(unknown_wallet['unlimited_copper_only_total'], 4)
                    self.assertIsNone(unknown_wallet['unlimited_copper_only_affordable'])
                    covered = (await lookup(acquisition_snapshot_id=captures['covered']))['merchant_reagent_plan']
                    self.assertEqual(covered['reagents'], [])
                    self.assertEqual(covered['unlimited_copper_only_total'], 0)
                    self.assertNotIn('merchant_reagent_plan', await lookup(restock_merchant_reagent_plan=False))
                    direct = await session.call_tool('review_forever_recipe', {'snapshot_id': key, 'recipe_id': 2330,
                        'crafts': 4, 'inventory_snapshot_id': key, 'additional_recipes': [
                            {'snapshot_id': key, 'recipe_id': 2337, 'crafts': 3}], 'merchant_reagent_plan': True})
                    self.assertFalse(direct.isError, direct)
                    direct_quote = json.loads(direct.content[0].text)['merchant_reagent_plan']
                    self.assertEqual(quote['reagents'], direct_quote['reagents'])
                    self.assertEqual(quote['unlimited_copper_only_total'], direct_quote['unlimited_copper_only_total'])
                    for kind in ['foreign', 'realm', 'build']:
                        await lookup(error=True, restock_merchant_snapshot_id=captures[kind])
                    await lookup(error=True, restock_merchant_snapshot_id='../PRIVATE')
                    await lookup(error=True, restock_merchant_snapshot_id=captures['offers'], restock_merchant_reagent_plan=False)
                    await lookup(error=True, restock_recipes=None)
                    await lookup(error=True, restock_merchant_reagent_plan='yes')
            self.assertEqual(original, {path.name: path.read_bytes() for path in Path(folder).glob('*.json')})
