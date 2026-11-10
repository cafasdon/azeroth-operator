"""Conditional selected-stack sale arithmetic through the actual offline MCP."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


def sample():
    value = fixture(); value['client'].update(family='forever', build='70862', interface=16001)
    value['sections'].update(wallet={'copper': 7}, bags={'0': {
        '1': {'item_id': 10, 'count': 10, 'sell_price_copper': 3, 'link': 'PRIVATE_LINK'},
        '2': {'item_id': 10, 'count': 5, 'sell_price_copper': 9, 'link': 'PRIVATE_VARIANT'},
        '3': {'item_id': 20, 'count': 2},
        '4': {'item_id': 30, 'count': 1, 'sell_price_copper': 0, 'link': 'PRIVATE_FREE'}}})
    return value


def selection(slot=1, item=10, quantity=2, container=0):
    return {'container': container, 'slot': slot, 'item_id': item, 'quantity': quantity}


class BagSaleQuoteTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_prices_unknowns_overflows_guards_and_privacy(self):
        root = Path(__file__).resolve().parents[1]; base = sample()
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder, tempfile.TemporaryDirectory(dir=root/'local-data') as other:
            store = SnapshotStore(Path(folder))
            foreign = deepcopy(base); foreign['character']['name'] = 'PRIVATE_OTHER'
            foreign_key = SnapshotStore(Path(other)).ingest(foreign)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}; env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            binary = root/'target/debug'/('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    route = next(t for t in (await session.list_tools()).tools if t.name == 'inspect_snapshot')
                    def objects(value):
                        if isinstance(value, dict):
                            yield value
                            for child in value.values(): yield from objects(child)
                        elif isinstance(value, list):
                            for child in value: yield from objects(child)
                    schema = next(obj for obj in objects(route.inputSchema)
                                  if set(obj.get('properties', {})) == {'selections', 'reserve_copper'})
                    self.assertFalse(schema['additionalProperties'])
                    self.assertEqual(schema['properties']['selections']['minItems'], 1)
                    self.assertEqual(schema['properties']['selections']['maxItems'], 20)
                    self.assertEqual(schema['properties']['reserve_copper']['maximum'], 9999999999999)
                    async def call(value=base, quote=None, error=False, **extra):
                        args = {'snapshot_id': store.ingest(value), 'section': 'bags', **extra}
                        if quote is not None: args['bag_sale_quote'] = quote
                        reply = await session.call_tool('inspect_snapshot', args)
                        self.assertEqual(bool(reply.isError), error, reply)
                        if error:
                            self.assertNotIn('PRIVATE', str(reply))
                            self.assertNotIn(folder, str(reply)); self.assertNotIn(other, str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    plain = await call()
                    nullable = await call(bag_sale_quote=None)
                    plain.pop('age_seconds'); nullable.pop('age_seconds')
                    self.assertEqual(nullable, plain)
                    selected = [selection(2, quantity=3), selection()]
                    result = await call(quote={'selections': selected, 'reserve_copper': 50})
                    quote = result['data']
                    self.assertNotIn('PRIVATE', json.dumps(quote))
                    self.assertEqual([row['slot'] for row in quote['rows']], [1, 2])
                    self.assertEqual(quote['total_proceeds_copper'], 33)
                    self.assertEqual(quote['conditional_wallet_copper_after_sale'], 40)
                    self.assertEqual(quote['conditional_reserve_shortfall_copper'], 10)
                    self.assertEqual(quote['conditional_copper_above_reserve'], 0)
                    self.assertTrue(quote['wallet_projection_is_hypothetical'])
                    for flag in ['sale_recommendation', 'current_funds_credited', 'bank_or_mail_credit',
                                 'current_locations_verified', 'native_sale_eligibility_verified', 'game_execution', 'live']:
                        self.assertFalse(quote[flag])
                    mixed = (await call(quote={'selections': [selection(), selection(3, 20, 1)]}))['data']
                    self.assertEqual(mixed['known_proceeds_subtotal_copper'], 6)
                    self.assertIsNone(mixed['total_proceeds_copper']); self.assertIsNone(mixed['conditional_wallet_copper_after_sale'])
                    self.assertEqual(mixed['unpriced_selection_count'], 1)
                    free = (await call(quote={'selections': [selection(4, 30, 1)]}))['data']
                    self.assertEqual(free['total_proceeds_copper'], 0); self.assertTrue(free['price_quote_complete'])
                    missing_wallet = deepcopy(base); missing_wallet['sections'].pop('wallet')
                    self.assertIsNone((await call(missing_wallet, quote={'selections': [selection()]}))
                                      ['data']['conditional_wallet_copper_after_sale'])
                    # Independent small integer reference: selection quantity times its own location's price.
                    for first in range(1, 11):
                        for second in range(1, 6):
                            actual = (await call(quote={'selections': [selection(quantity=first),
                                selection(2, quantity=second)], 'reserve_copper': 20}))['data']
                            expected = 3*first + 9*second
                            self.assertEqual(actual['total_proceeds_copper'], expected)
                            self.assertEqual(actual['conditional_copper_above_reserve'], max(7+expected-20, 0))
                    huge = deepcopy(base); row = huge['sections']['bags']['0']['1']
                    row.update(count=4294967295, sell_price_copper=9999999999999)
                    overflow = (await call(huge, quote={'selections': [selection(quantity=4294967295)]}))['data']
                    self.assertEqual(overflow['rows'][0]['status'], 'arithmetic_overflow')
                    self.assertTrue(overflow['proceeds_arithmetic_overflow']); self.assertIsNone(overflow['total_proceeds_copper'])
                    huge['sections']['bags']['0']['2'] = deepcopy(row)
                    overflow = (await call(huge, quote={'selections': [selection(quantity=1000000), selection(2, quantity=1000000)]}))['data']
                    self.assertIsNone(overflow['known_proceeds_subtotal_copper']); self.assertTrue(overflow['proceeds_arithmetic_overflow'])
                    huge['sections']['wallet']['copper'] = 9999999999999
                    overflow = (await call(huge, quote={'selections': [selection(quantity=1844674)]}))['data']
                    self.assertTrue(overflow['price_quote_complete']); self.assertTrue(overflow['wallet_projection_arithmetic_overflow'])
                    self.assertIsNone(overflow['conditional_wallet_copper_after_sale'])
                    for bad in [[], True, 'PRIVATE_MARKER', {}, {'selections': []},
                                {'selections': [[0, 1, 10, 2]]},
                                {'selections': [selection()]*2}, {'selections': [selection()]*21},
                                {'selections': [selection(item=999)]}, {'selections': [selection(quantity=11)]},
                                {'selections': [selection(quantity=0)]}, {'selections': [selection(slot=0)]},
                                {'selections': [selection(slot=1025)]}, {'selections': [selection(container=65)]},
                                {'selections': [selection()], 'reserve_copper': -1},
                                {'selections': [selection()], 'reserve_copper': 10000000000000},
                                {'selections': [selection()], 'PRIVATE_MARKER': True}]:
                        await call(quote=bad, error=True)
                    for extra in [{'section': 'equipment'}, {'section': None}, {'coverage_sections': ['bags']},
                                  {'snapshot_id': foreign_key}, {'snapshot_id': '../PRIVATE_MARKER'}]:
                        await call(quote={'selections': [selection()]}, error=True, **extra)
                    for field in ['bags']:
                        missing = deepcopy(base); missing['sections'].pop(field)
                        await call(missing, quote={'selections': [selection()]}, error=True)
                    unknown = deepcopy(base); unknown['sections']['bags']['0']['1'].pop('count')
                    with self.assertRaises(ValueError): store.ingest(unknown)
                    raw = json.dumps(unknown, sort_keys=True, separators=(',', ':')).encode()
                    invalid_key = hashlib.sha256(raw).hexdigest()
                    (Path(folder)/f'{invalid_key}.json').write_bytes(raw)
                    await call(quote={'selections': [selection()]}, error=True, snapshot_id=invalid_key)
                    retail = deepcopy(base); retail['client']['family'] = 'retail'
                    retail['sections'].pop('wallet')
                    await call(retail, quote={'selections': [selection()]}, error=True)
                    files = {p: p.read_bytes() for p in Path(folder).glob('*.json')}
                    await call(quote={'selections': [selection()]})
                    self.assertEqual(files, {p: p.read_bytes() for p in files})
