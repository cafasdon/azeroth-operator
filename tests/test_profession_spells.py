"""Reported profession spells stay bounded and distinct from complete recipes."""
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


class ProfessionSpellsTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_summary_catalogue_join_and_strict_readers(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root / 'local-data/forever/catalogue.json')
        if not catalogue.is_file():
            self.skipTest('Requires pinned catalogue; no automatic download')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        section = {'coverage': 'reported_known_profession_spells', 'profession_id': 164, 'entries': {'2675': {}}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            valid = []
            for entries in [None, {}, {'2675': {}}, {str(i): {} for i in range(1, 4097)}]:
                value = deepcopy(base)
                if entries is not None:
                    value['sections']['profession_spells'] = {**section, 'entries': entries}
                valid.append((store.ingest(value), entries))
            conflicts = []
            for kind in ['selected_false', 'list_false', 'output_mismatch']:
                value = deepcopy(base)
                value['sections']['profession_spells'] = section
                if kind == 'list_false':
                    value['sections']['profession_recipe_view'] = {'coverage': 'filtered_local_recipe_list',
                        'profession_id': 164, 'entries': {'2675': {'learned': False}}}
                else:
                    value['sections']['selected_recipe'] = {'recipe_id': 2675, 'name': 'PRIVATE_SCHEMATIC',
                        'learned': kind != 'selected_false', 'output_item_id': 3844 if kind == 'output_mismatch' else 2870,
                        'requirements': 'fixed_items', 'reagents': {}}
                conflicts.append(store.ingest(value))
            invalid = [None, {}, {**section, 'coverage': 'all_recipes'}, {**section, 'profession_id': True},
                       {**section, 'entries': {'0': {}}}, {**section, 'entries': {'01': {}}},
                       {**section, 'entries': {'4294967296': {}}}, {**section, 'entries': {'1': None}},
                       {**section, 'entries': {'1': {'learned': True}}},
                       {**section, 'entries': {str(i): {} for i in range(1, 4098)}}]
            bad_ids = []
            for bad in invalid:
                value = deepcopy(base)
                value['sections']['profession_spells'] = bad
                with self.assertRaises(ValueError):
                    validate_snapshot(value)
                raw = json.dumps(value, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            for family in ['retail', 'classic']:
                value = deepcopy(base)
                value['client']['family'] = family
                value['sections']['profession_spells'] = section
                with self.assertRaises(ValueError):
                    validate_snapshot(value)
                raw = json.dumps(value, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder, 'AZEROTH_FOREVER_CATALOGUE': str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def call(tool, **args):
                        reply = await session.call_tool(tool, args)
                        self.assertFalse(reply.isError, reply)
                        return json.loads(reply.content[0].text)
                    for key, entries in valid:
                        summary = (await call('character_summary', snapshot_id=key))['profession_spell_observations']
                        if entries is None:
                            self.assertIsNone(summary)
                            continue
                        self.assertEqual(summary['reported_spell_count'], len(entries))
                        self.assertEqual(summary['spell_ids'], sorted(map(int, entries))[:50])
                        self.assertEqual(summary['truncated'], len(entries) > 50)
                        self.assertFalse(summary['complete_recipe_knowledge_verified'])
                        self.assertFalse(summary['craftability_verified'])
                        self.assertEqual((await call('inspect_snapshot', snapshot_id=key, section='profession_spells'))['data']['entries'], entries)
                    known = valid[2][0]
                    query = {'section': 'items', 'key': '2870', 'acquisition_snapshot_id': known,
                             'only_observed_learned_recipe': True}
                    result = await call('lookup_forever_catalogue', **query)
                    self.assertEqual(result['records'][0]['id'], 2870)
                    source = result['acquisition_progress']['sources'][0]
                    self.assertTrue(source['recipe_learning']['profession_spell_reported'])
                    self.assertTrue(source['recipe_learning']['learned'])
                    self.assertIsNone(source['learned'])
                    self.assertIsNone(source['material_capacity'])
                    for key in conflicts:
                        result = await call('lookup_forever_catalogue', **{**query, 'acquisition_snapshot_id': key})
                        self.assertEqual(result['records'], [])
                        result = await call('lookup_forever_catalogue', **{**query, 'acquisition_snapshot_id': key,
                                            'only_observed_learned_recipe': False})
                        self.assertIsNone(result['acquisition_progress']['sources'][0]['recipe_learning']['learned'])
                        self.assertNotIn('PRIVATE_SCHEMATIC', json.dumps(result))
                    for key in bad_ids:
                        reply = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertTrue(reply.isError)


if __name__ == '__main__':
    unittest.main()
