"""Public suffix possibilities never establish an owned rolled variant."""
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


class CatalogueOwnershipEvidenceTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_base_presence_and_suffix_boundaries(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        cache = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or
                     root/'local-data/forever/catalogue.json').resolve(strict=True)
        public = json.loads(cache.read_bytes())
        item = next(row for row in public['items'] if row['id'] == 14270)
        suffix = item['randomSuffixOptions'][0]
        sample = fixture()
        sample['client'].update(family='forever', build='70862', interface=16001)
        sample['sections']['bags'] = {'0': {'1': {
            'item_id': 14270, 'count': 1, 'link': 'PRIVATE_UNRESOLVED_VARIANT'}}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder)/'captures')
            owned = store.ingest(sample)
            absent = deepcopy(sample)
            absent['sections'].update(bags={}, character_bank={}, account_bank={})
            absent_id = store.ingest(absent)
            unknown = deepcopy(sample)
            unknown['sections']['bags'] = {}
            unknown_id = store.ingest(unknown)
            before = {p.name: p.read_bytes() for p in (Path(folder)/'captures').glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': str(Path(folder)/'captures'),
                   'AZEROTH_FOREVER_CATALOGUE': str(cache)}
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def call(snapshot=None, presence=None, with_suffix=True):
                        args = {'section': 'items', 'key': '14270'}
                        if with_suffix:
                            args['item_random_suffix_id'] = suffix
                        if snapshot is not None:
                            args.update(acquisition_snapshot_id=snapshot, observed_ownership=presence)
                        reply = await session.call_tool('lookup_forever_catalogue', args)
                        self.assertFalse(reply.isError, reply)
                        result = json.loads(reply.content[0].text)
                        self.assertNotIn('PRIVATE_', json.dumps(result))
                        return result
                    for key, presence, complete in ((owned, 'owned', False),
                            (absent_id, 'absent', True), (unknown_id, 'unknown', False)):
                        result = await call(key, presence)
                        self.assertEqual([r['id'] for r in result['records']], [14270])
                        self.assertEqual(result['ownership_filter_evidence'], {
                            'scope': 'saved_base_item_id_presence', 'requested_presence': presence,
                            'all_storage_sections_observed': complete, 'owned_variant_verified': False,
                            'current_ownership_verified': False, 'equip_legality_verified': False})
                        self.assertEqual(result['random_suffix_filter_evidence'], {
                            'suffix_id': suffix, 'relationship': 'catalogue_option_not_owned_variant',
                            'owned_variant_verified': False})
                    self.assertEqual((await call(unknown_id, 'absent'))['records'], [])
                    self.assertEqual((await call(absent_id, 'owned'))['records'], [])
                    self.assertNotIn('ownership_filter_evidence', await call())
                    self.assertNotIn('random_suffix_filter_evidence', await call(with_suffix=False))
                    refused = await session.call_tool('lookup_forever_catalogue', {
                        'section': 'items', 'key': '14270', 'observed_ownership': 'owned'})
                    self.assertTrue(refused.isError)
            self.assertEqual(before, {p.name: p.read_bytes()
                for p in (Path(folder)/'captures').glob('*.json')})
