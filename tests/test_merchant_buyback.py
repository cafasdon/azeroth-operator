"""Buyback observations never become owned stock or simulator candidates."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore, validate_snapshot
from test_snapshots import fixture


class MerchantBuybackTests(unittest.IsolatedAsyncioTestCase):
    async def test_quotes_unknowns_bounds_reader_parity_and_inventory_separation(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        catalogue = root / 'local-data/forever/catalogue.json'
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        base['sections'].update(bags={}, character_bank={}, account_bank={})
        cases = [None, {'1': {'item_id': 4549, 'count': 3, 'buyback_price_copper': 10},
                        '2': {'item_id': 118, 'count': 1}}, {},
                 {str(i): {'item_id': 4294967295, 'count': 4294967295, 'buyback_price_copper': 9999999999999}
                  for i in range(1, 13)}, {'1': {'item_id': 4549, 'count': 3, 'buyback_price_copper': 0}}]
        section = {'coverage': 'visible_merchant_buyback', 'entries': cases[1]}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            valid = []
            for rows in cases:
                value = deepcopy(base)
                if rows is not None:
                    value['sections']['merchant_buyback'] = {'coverage': section['coverage'], 'entries': rows}
                valid.append((store.ingest(value), rows))
            owned_copy = deepcopy(base)
            owned_copy['sections']['merchant_buyback'] = section
            owned_copy['sections']['bags'] = {'0': {'1': {'item_id': 4549, 'count': 1}}}
            owned_copy_key = store.ingest(owned_copy)
            invalid = [None, {}, {**section, 'coverage': 'owned_buyback'}, {**section, 'entries': None},
                       {**section, 'entries': {'0': {'item_id': 1, 'count': 1}}},
                       {**section, 'entries': {'01': {'item_id': 1, 'count': 1}}},
                       {**section, 'entries': {'2': {'item_id': 1, 'count': 1}}},
                       {**section, 'entries': {str(i): {'item_id': 1, 'count': 1} for i in range(1, 14)}}]
            for field, values in [('item_id', [None, 0, True, 1.0, 4294967296]),
                                  ('count', [None, 0, -1, True, 1.5, 4294967296]),
                                  ('buyback_price_copper', [None, -1, True, 1.5, '10', 10000000000000])]:
                for bad in values:
                    invalid.append({'coverage': section['coverage'], 'entries': {'1': {
                        **cases[1]['1'], field: bad}}})
            invalid.append({'coverage': section['coverage'], 'entries': {'1': {
                **cases[1]['1'], 'speaker': 'PRIVATE_BUYBACK'}}})
            bad_ids = []
            for value in invalid:
                sample = deepcopy(base)
                sample['sections']['merchant_buyback'] = value
                with self.assertRaises(ValueError):
                    validate_snapshot(sample)
                raw = json.dumps(sample, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            for family in ['retail', 'classic']:
                sample = deepcopy(base)
                sample['client']['family'] = family
                sample['sections']['merchant_buyback'] = section
                with self.assertRaises(ValueError):
                    validate_snapshot(sample)
                raw = json.dumps(sample, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            if catalogue.is_file():
                env['AZEROTH_FOREVER_CATALOGUE'] = str(catalogue.resolve())
            else:
                env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def call(tool, **args):
                        response = await session.call_tool(tool, args)
                        self.assertFalse(response.isError, response)
                        return json.loads(response.content[0].text)
                    async def search(item_id, **filters):
                        return (await call('list_snapshots',family='forever',
                            merchant_buyback={'item_id':item_id,**filters}))['snapshots']
                    zero = await search(4549,minimum_quantity=3,maximum_price_copper=0)
                    self.assertEqual([row['snapshot_id'] for row in zero],[valid[4][0]])
                    self.assertEqual(zero[0]['merchant_buyback_match'],{
                        'matches':[{'slot':1,'observation':cases[4]['1']}],
                        'included_in_owned_inventory':False,'buyback_eligibility_verified':False,'live':False})
                    self.assertEqual(await search(4549,minimum_quantity=4,maximum_price_copper=10),[])
                    self.assertEqual(await search(118,maximum_price_copper=9999999999999),[])
                    unknown = await search(118)
                    self.assertEqual({row['snapshot_id'] for row in unknown},{valid[1][0],owned_copy_key})
                    maximum = await search(4294967295,maximum_price_copper=9999999999999)
                    self.assertEqual([row['slot'] for row in maximum[0]['merchant_buyback_match']['matches']],
                                     list(range(1,13)))
                    funded = deepcopy(base)
                    funded['sections']['merchant_buyback'] = section
                    funded['sections']['wallet'] = {'copper': 15}
                    funded_key = store.ingest(funded)
                    affordable = await search(4549,minimum_remaining_wallet_copper=5)
                    self.assertEqual([row['snapshot_id'] for row in affordable],[funded_key])
                    self.assertNotIn('wallet',affordable[0]['merchant_buyback_match'])
                    self.assertEqual(await search(4549,minimum_remaining_wallet_copper=6),[])
                    self.assertEqual(await search(118,minimum_remaining_wallet_copper=0),[])
                    self.assertEqual(await search(4549,minimum_remaining_wallet_copper=5,maximum_price_copper=9),[])
                    for key, rows in valid:
                        summary = (await call('character_summary', snapshot_id=key))['merchant_buyback_observations']
                        if rows is None:
                            self.assertIsNone(summary)
                            continue
                        prices = [row['buyback_price_copper'] for row in rows.values() if 'buyback_price_copper' in row]
                        self.assertEqual(summary['listed_item_locations'], len(rows))
                        self.assertEqual(summary['listed_units'], sum(row['count'] for row in rows.values()))
                        self.assertEqual(summary['known_price_subtotal_copper'], sum(prices))
                        self.assertEqual(summary['total_listed_quote_copper'], sum(prices) if len(prices) == len(rows) else None)
                        self.assertFalse(summary['included_in_owned_inventory'])
                        self.assertFalse(summary['buyback_eligibility_verified'])
                        self.assertEqual((await call('inspect_snapshot', snapshot_id=key, section='merchant_buyback'))['data']['entries'], rows)
                        found = await call('find_snapshot_item', snapshot_id=key, item_id=4549)
                        self.assertTrue(all(amount == 0 for amount in found['quantity_by_storage'].values()))
                        if catalogue.is_file():
                            owned = await call('review_forever_owned_gear', snapshot_id=key)
                            self.assertEqual(owned['observed_item_locations'], 0)
                            self.assertEqual(owned['items'], [])
                    found = await call('find_snapshot_item', snapshot_id=owned_copy_key, item_id=4549)
                    self.assertEqual(found['quantity_by_storage']['bags'], 1)
                    if catalogue.is_file():
                        owned = await call('review_forever_owned_gear', snapshot_id=owned_copy_key)
                        self.assertEqual(owned['observed_item_locations'], 1)
                    for key in bad_ids:
                        response = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertTrue(response.isError)
                        self.assertNotIn('PRIVATE_BUYBACK', str(response))


if __name__ == '__main__':
    unittest.main()
