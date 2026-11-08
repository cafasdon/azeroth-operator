"""Catalogue effect-ID matches do not prove consumable use or change restocking."""
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


class ConsumableAuraMatchTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_exact_shared_ids_unknowns_and_stock_invariance(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        base['sections'].update(bags={'0': {'1': {'item_id': 118, 'count': 1}}}, character_bank={}, account_bank={})
        groups = [None, {'harmful': {'entries': {'1': {'spell_id': 439}}}},
                  {'helpful': {'entries': {}}}, {'helpful': {'entries': {'1': {'spell_id': 440}}}},
                  {'helpful': {'entries': {'1': {'spell_id': 439, 'applications': 0},
                                           '2': {'spell_id': 439}, '3': {'spell_id': 1248384}}}}]
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            cases = []
            for group in groups:
                value = deepcopy(base)
                if group is not None:
                    value['sections']['player_auras'] = {'coverage': 'readable_nonprivate_player_aura_lists', 'groups': group}
                cases.append(store.ingest(value))
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def lookup(key, **query):
                        response = await session.call_tool('lookup_forever_catalogue', {'section': 'consumables',
                            'acquisition_snapshot_id': key, **query})
                        self.assertFalse(response.isError, response)
                        return json.loads(response.content[0].text)
                    for index, key in enumerate(cases):
                        result = await lookup(key, item_ids=[118, 268881, 724])
                        evidence = result['acquisition_progress']['captured_helpful_aura_matches']
                        self.assertEqual(evidence['helpful_list_observed'], index >= 2)
                        self.assertFalse(evidence['item_use_verified'])
                        self.assertFalse(evidence['effectiveness_verified'])
                        self.assertEqual(set(evidence['by_item_id']), {'118', '268881', '724'})
                        for item in ['118', '268881']:
                            row = evidence['by_item_id'][item]
                            self.assertEqual(row['catalogue_effect_spell_ids'], [439])
                            if index < 2:
                                self.assertIsNone(row['observed_id_match'])
                                self.assertIsNone(row['matching_aura_spell_ids'])
                                self.assertIsNone(row['matching_aura_row_count'])
                            else:
                                self.assertEqual(row['observed_id_match'], index == 4)
                                self.assertEqual(row['matching_aura_spell_ids'], [439] if index == 4 else [])
                                self.assertEqual(row['matching_aura_row_count'], 2 if index == 4 else 0)
                                self.assertEqual(row['matching_aura_spell_id_count'], 1 if index == 4 else 0)
                        if index == 4:
                            self.assertTrue(evidence['by_item_id']['724']['observed_id_match'])
                            self.assertEqual(result['acquisition_progress']['owned_quantities_by_item_id']['268881']['bags'], 0)
                    plans = []
                    quantities = []
                    for key in cases:
                        result = await lookup(key, consumable_targets=[{'item_id': 118, 'quantity': 2}])
                        plans.append(result['acquisition_progress']['restock_plan'])
                        quantities.append(result['acquisition_progress']['owned_quantities_by_item_id'])
                    self.assertTrue(all(plan == plans[0] for plan in plans))
                    self.assertTrue(all(stock == quantities[0] for stock in quantities))
                    self.assertEqual(plans[0][0]['confirmed_shortfall'], 1)
                    no_capture = await session.call_tool('lookup_forever_catalogue', {'section': 'consumables', 'key': '118'})
                    self.assertFalse(no_capture.isError)
                    self.assertNotIn('acquisition_progress', json.loads(no_capture.content[0].text))


if __name__ == '__main__':
    unittest.main()
