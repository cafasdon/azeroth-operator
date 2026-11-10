"""Historical character item index never pools account banks or invents active identity."""
from copy import deepcopy
import json
import os
from pathlib import Path
import random
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


def sample(name='Alice', day=1, quantity=2, realm='Test'):
    value = fixture(); value['client'].update(family='forever', build='70862', interface=16001)
    value['character'] = {'name': name, 'realm': realm}
    value['captured_at'] = f'2025-01-{day:02d}T00:00:00Z'
    value['sections'].update(bags={'0': {'1': {'item_id': 10, 'count': quantity, 'link': 'PRIVATE_LINK'}}},
        character_bank={'6': {'1': {'item_id': 10, 'count': 9}}},
        account_bank={'13': {'1': {'item_id': 10, 'count': 100}}})
    return value


class CharacterItemIndexTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_latest_ties_unknowns_order_and_owner_boundary(self):
        root = Path(__file__).resolve().parents[1]
        old = sample(); middle = sample(day=2, quantity=99); newest = sample(day=3, quantity=5)
        newest['sections'].pop('character_bank')
        bob = sample('Bob', 2, 7); tie = sample('Bob', 2, 8)
        carol = sample('Carol', 4); carol['sections']['bags'] = {}; carol['sections']['character_bank'] = {}
        other_realm = sample(realm='Other')
        dave = sample('Dave'); dave['sections'].pop('bags'); dave['sections'].pop('account_bank')
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder, tempfile.TemporaryDirectory(dir=root/'local-data') as foreign:
            store = SnapshotStore(Path(folder))
            captures = [old, middle, newest, bob, tie, carol, other_realm, dave]
            keys = [store.ingest(value) for value in captures]
            stranger = sample('PRIVATE_OTHER'); foreign_key = SnapshotStore(Path(foreign)).ingest(stranger)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}; env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            binary = root/'target/debug'/('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    route = next(t for t in (await session.list_tools()).tools if t.name == 'find_snapshot_item')
                    self.assertEqual(route.inputSchema['properties']['character_item_index']['type'], 'boolean')
                    self.assertFalse(route.inputSchema['properties']['character_item_index']['default'])
                    async def call(primary=keys[0], additional=None, error=False, **extra):
                        args = {'snapshot_id': primary, 'item_id': 10, 'limit': 1, **extra}
                        if additional is not None: args['additional_snapshot_ids'] = additional
                        reply = await session.call_tool('find_snapshot_item', args)
                        self.assertEqual(bool(reply.isError), error, reply)
                        if error:
                            self.assertNotIn('PRIVATE_OTHER', str(reply)); self.assertNotIn(foreign, str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    plain = await call(additional=keys[1:])
                    self.assertNotIn('character_item_index', plain)
                    self.assertEqual(await call(additional=keys[1:], character_item_index=False), plain)
                    report = await call(additional=keys[1:], character_item_index=True)
                    index = report.pop('character_item_index')
                    self.assertEqual(report, plain)
                    self.assertEqual(index['selected_capture_count'], 8)
                    self.assertEqual(index['character_count'], 5)
                    self.assertEqual(index['ambiguous_character_count'], 1)
                    self.assertNotIn('PRIVATE_LINK', json.dumps(index))
                    self.assertIsNone(index['cross_character_total'])
                    for flag in ['missing_storage_falls_back_to_older_capture', 'account_bank_quantities_are_additive',
                                 'account_identity_verified', 'simultaneous_ownership_verified', 'transfers_reconciled',
                                 'all_characters_covered', 'current_inventory_verified', 'active_character_verified',
                                 'item_variant_equivalence_verified', 'game_execution', 'live']:
                        self.assertFalse(index[flag])
                    rows = {(r['character']['name'], r['character']['realm']): r for r in index['characters']}
                    alice = rows['Alice', 'Test']
                    self.assertTrue(index['characters'][0]['primary_character'])
                    self.assertEqual(alice['latest_snapshot_id'], keys[2])
                    self.assertEqual(alice['older_selected_capture_count'], 2)
                    self.assertEqual(alice['quantity_by_character_storage'], {'equipment': 0, 'bags': 5, 'character_bank': None})
                    self.assertEqual(alice['unobserved_character_storage'], ['character_bank'])
                    self.assertEqual(alice['account_bank_quantity'], 100)
                    ambiguous = rows['Bob', 'Test']
                    self.assertEqual(ambiguous['latest_capture_status'], 'ambiguous_latest')
                    self.assertEqual(ambiguous['latest_candidate_snapshot_ids'], sorted(keys[3:5]))
                    self.assertIsNone(ambiguous['latest_snapshot_id']); self.assertIsNone(ambiguous['quantity_by_character_storage'])
                    self.assertIsNone(ambiguous['account_bank_quantity'])
                    self.assertEqual(rows['Carol', 'Test']['quantity_by_character_storage']['bags'], 0)
                    self.assertIsNone(rows['Dave', 'Test']['quantity_by_character_storage']['bags'])
                    self.assertIsNone(rows['Dave', 'Test']['account_bank_quantity'])
                    self.assertEqual(rows['Alice', 'Other']['quantity_by_character_storage']['bags'], 2)
                    # Page-independent counts; primary and additional input order never pick a tied capture.
                    wide = await call(additional=keys[1:], character_item_index=True, offset=999, limit=50)
                    self.assertEqual(wide['character_item_index'], index)
                    rng = random.Random(97)
                    for _ in range(24):
                        additional = keys[1:].copy(); rng.shuffle(additional)
                        self.assertEqual((await call(additional=additional, character_item_index=True))['character_item_index'], index)
                    moved = (await call(primary=keys[2], additional=[keys[0], *keys[1:2], *keys[3:]],
                                        character_item_index=True))['character_item_index']
                    self.assertEqual(moved, index)
                    single = (await call(character_item_index=True))['character_item_index']
                    self.assertEqual(single['character_count'], 1)
                    self.assertEqual(single['characters'][0]['quantity_by_character_storage']['bags'], 2)
                    for bad in [None, 1, [], {}, 'true']:
                        await call(error=True, character_item_index=bad)
                    for additional in [[keys[0]], [keys[1]]*2, [*keys[1:], foreign_key], [foreign_key]]:
                        await call(additional=additional, character_item_index=True, error=True)
                    wrong = sample('Wrong'); wrong['client']['build'] = '999'
                    await call(additional=[store.ingest(wrong)], character_item_index=True, error=True)
                    retail = fixture(); retail['sections']['bags'] = {}
                    await call(primary=store.ingest(retail), character_item_index=True, error=True)
                    files = {p: p.read_bytes() for p in Path(folder).glob('*.json')}
                    await call(additional=keys[1:], character_item_index=True)
                    self.assertEqual(files, {p: p.read_bytes() for p in files})
