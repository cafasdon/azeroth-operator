"""Same-capture buyback quote arithmetic never performs or proves a purchase."""
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


class BuybackCostReviewTests(unittest.IsolatedAsyncioTestCase):
    async def test_quotes_reserves_unknowns_validation_and_read_only_state(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        base['sections']['merchant_buyback'] = {'coverage': 'visible_merchant_buyback', 'entries': {
            '1': {'item_id': 4549, 'count': 3, 'buyback_price_copper': 10},
            '2': {'item_id': 118, 'count': 1, 'buyback_price_copper': 20},
            '3': {'item_id': 999, 'count': 5}, '4': {'item_id': 456, 'count': 1, 'buyback_price_copper': 0}}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            keys = {}
            for balance in [None, 0, 9, 10, 30, 35]:
                value = deepcopy(base)
                if balance is not None:
                    value['sections']['wallet'] = {'copper': balance}
                keys[balance] = store.ingest(value)
            missing = deepcopy(base)
            missing['sections'].pop('merchant_buyback')
            missing_id = store.ingest(missing)
            empty = deepcopy(base)
            empty['sections']['merchant_buyback']['entries'] = {}
            empty_id = store.ingest(empty)
            space_capture = deepcopy(base)
            space_capture['sections']['merchant_buyback']['entries']['3']['item_id'] = 4549
            space_capture['sections']['bags'] = {'0': {'1': {
                'item_id': 4549, 'count': 2, 'link': 'item:4549:0', 'reported_stack_limit': 5}}}
            space_capture['sections']['bag_capacity'] = {
                '0': {'total_slots': 16, 'free_slots': 1, 'family_mask': 0},
                **{str(bag): {'total_slots': 0, 'free_slots': 0} for bag in range(1, 5)}}
            space_id = store.ingest(space_capture)
            maximum = deepcopy(base)
            maximum['sections']['wallet'] = {'copper': 9999999999999}
            maximum['sections']['merchant_buyback']['entries'] = {str(i): {'item_id': 1, 'count': 4294967295,
                'buyback_price_copper': 9999999999999} for i in range(1, 13)}
            max_id = store.ingest(maximum)
            retail_id = store.ingest(fixture())
            before = {path.name: path.read_bytes() for path in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def review(balance, slots, reserve=0, snapshot=None, **options):
                        response = await session.call_tool('inspect_snapshot', {'snapshot_id': snapshot or keys[balance],
                            'section': 'merchant_buyback', 'buyback_slots': slots, 'buyback_wallet_reserve_copper': reserve,
                            **options})
                        self.assertFalse(response.isError, response)
                        return json.loads(response.content[0].text)['buyback_cost_review']
                    full = await review(30, [1, 2])
                    self.assertEqual(full['total_selected_quote_copper'], 30)
                    self.assertTrue(full['affordable_from_captured_wallet'])
                    self.assertFalse(full['buyback_eligibility_verified'])
                    self.assertFalse(full['game_action_performed'])
                    self.assertNotIn('bag_space_projection', full)
                    for slots, fits in [([1, 3], True), ([1, 3, 5], None), ([1, 2, 3], None)]:
                        result = await review(None, slots, snapshot=space_id, buyback_estimate_bag_space=True)
                        space = result['bag_space_projection']
                        item = next(row for row in space['items'] if row['item_id'] == 4549)
                        self.assertEqual(item['selected_quantity'], 8)
                        self.assertEqual(item['estimated_new_stack_slots'], 2)
                        self.assertIs(space['fits_recorded_general_slots_if_same_item_stacks_merge'], fits)
                        self.assertIsNone(result['total_selected_quote_copper'])
                    self.assertEqual((await review(10, [1]))['total_selected_quote_copper'], 10)
                    self.assertTrue((await review(35, [1, 2], 5))['affordable_from_captured_wallet'])
                    self.assertFalse((await review(35, [1, 2], 6))['affordable_from_captured_wallet'])
                    partial = await review(9, [1, 3])
                    self.assertIsNone(partial['total_selected_quote_copper'])
                    self.assertFalse(partial['affordable_from_captured_wallet'])
                    self.assertEqual(partial['slots_with_unknown_price'], [3])
                    self.assertEqual(partial['known_shortfall_vs_spendable_copper'], 1)
                    self.assertIsNone((await review(10, [1, 3]))['affordable_from_captured_wallet'])
                    self.assertTrue((await review(0, [4]))['affordable_from_captured_wallet'])
                    self.assertFalse((await review(0, [4], 1))['affordable_from_captured_wallet'])
                    self.assertIsNone((await review(None, [4]))['affordable_from_captured_wallet'])
                    unknown_slot = await review(35, [5])
                    self.assertEqual(unknown_slot['missing_captured_slots'], [5])
                    self.assertIsNone(unknown_slot['total_selected_quote_copper'])
                    self.assertIsNone(unknown_slot['affordable_from_captured_wallet'])
                    self.assertIsNone(await review(None, [1], snapshot=missing_id))
                    self.assertEqual((await review(None, [1], snapshot=empty_id))['missing_captured_slots'], [1])
                    large = await review(None, list(range(1, 13)), snapshot=max_id)
                    self.assertEqual(large['total_selected_quote_copper'], 119999999999988)
                    self.assertFalse(large['affordable_from_captured_wallet'])
                    ordinary = await session.call_tool('inspect_snapshot', {'snapshot_id': keys[35], 'section': 'merchant_buyback'})
                    self.assertFalse(ordinary.isError)
                    self.assertNotIn('buyback_cost_review', json.loads(ordinary.content[0].text))
                    invalid = [ {'buyback_estimate_bag_space': True}, {'buyback_slots': []}, {'buyback_slots': [1, 1]}, {'buyback_slots': [0]},
                                {'buyback_slots': [13]}, {'buyback_slots': [True]},
                                {'buyback_slots': [1], 'buyback_wallet_reserve_copper': 10000000000000},
                                {'buyback_wallet_reserve_copper': 1}, {'buyback_slots': [1], 'section': 'merchant_stock'},
                                {'buyback_slots': [1], 'section': None}, {'buyback_slots': [1], 'snapshot_id': retail_id}]
                    for args in invalid:
                        response = await session.call_tool('inspect_snapshot', {'snapshot_id': keys[35],
                            'section': 'merchant_buyback', **args})
                        self.assertTrue(response.isError)
            self.assertEqual({path.name: path.read_bytes() for path in Path(folder).glob('*.json')}, before)


if __name__ == '__main__':
    unittest.main()
