"""Saved public player aura lists are bounded and never imply full effectiveness."""
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


class PlayerAuraTests(unittest.IsolatedAsyncioTestCase):
    async def test_reader_parity_partial_empty_duplicates_and_bounded_summary(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        section = {'coverage': 'readable_nonprivate_player_aura_lists', 'groups': {
            'helpful': {'entries': {'1': {'spell_id': 100, 'applications': 0}, '2': {'spell_id': 100},
                                    '3': {'spell_id': 2, 'applications': 4}}}, 'harmful': {'entries': {}}}}
        cases = [None, section, {'coverage': section['coverage'], 'groups': {'harmful': {'entries': {}}}},
                 {'coverage': section['coverage'], 'groups': {'helpful': {'entries': {
                     str(i): {'spell_id': 4294967295 - i, 'applications': 4294967295} for i in range(1, 257)}}}}]
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            valid = []
            for value in cases:
                sample = deepcopy(base)
                if value is not None:
                    sample['sections']['player_auras'] = value
                valid.append((store.ingest(sample), value))
            invalid = [None, {}, {**section, 'coverage': 'all_auras'}, {**section, 'groups': {}},
                       {**section, 'groups': {'target': {'entries': {}}}},
                       {**section, 'groups': {'helpful': None}}, {**section, 'groups': {'helpful': {'entries': None}}}]
            for row in [None, {}, {'spell_id': 0}, {'spell_id': True}, {'spell_id': 1.0}, {'spell_id': 4294967296},
                        {'spell_id': 1, 'applications': None}, {'spell_id': 1, 'applications': -1},
                        {'spell_id': 1, 'applications': True}, {'spell_id': 1, 'sourceUnit': 'PRIVATE_AURA'}]:
                invalid.append({'coverage': section['coverage'], 'groups': {'helpful': {'entries': {'1': row}}}})
            for entries in [{'0': {'spell_id': 1}}, {'01': {'spell_id': 1}}, {'2': {'spell_id': 1}},
                            {str(i): {'spell_id': 1} for i in range(1, 258)}]:
                invalid.append({'coverage': section['coverage'], 'groups': {'helpful': {'entries': entries}}})
            bad_ids = []
            for value in invalid:
                sample = deepcopy(base)
                sample['sections']['player_auras'] = value
                with self.assertRaises(ValueError):
                    validate_snapshot(sample)
                raw = json.dumps(sample, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            for family in ['retail', 'classic']:
                sample = deepcopy(base)
                sample['client']['family'] = family
                sample['sections']['player_auras'] = section
                with self.assertRaises(ValueError):
                    validate_snapshot(sample)
                raw = json.dumps(sample, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                bad_ids.append(key)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def search(group, spell_id, family='forever', error=False, **bounds):
                        result = await session.call_tool('list_snapshots', {'family': family,
                            'player_aura': {'group': group, 'spell_id': spell_id, **bounds}})
                        self.assertEqual(bool(result.isError), error, result)
                        return None if error else json.loads(result.content[0].text)['snapshots']
                    matches = await search('helpful', 100)
                    self.assertEqual([row['snapshot_id'] for row in matches], [valid[1][0]])
                    self.assertEqual(matches[0]['player_aura_match'], {'group': 'helpful',
                        'spell_id': 100, 'matching_observed_rows': 2, 'effectiveness_verified': False})
                    self.assertEqual(await search('harmful', 100), [])
                    zero = await search('helpful', 100, maximum_applications=0)
                    self.assertEqual(zero[0]['player_aura_match']['matching_observed_rows'],1)
                    self.assertEqual(await search('helpful', 100, minimum_applications=1),[])
                    self.assertEqual(await search('helpful', 100, minimum_applications=0),zero)
                    self.assertEqual(len(await search('helpful', 4294967294, minimum_applications=4294967295)),1)
                    await search('helpful', 100, minimum_applications=2, maximum_applications=1, error=True)
                    self.assertEqual([row['snapshot_id'] for row in await search('helpful', 4294967294)],
                                     [valid[3][0]])
                    await search('helpful', 0, error=True)
                    await search('helpful', 100, family='retail', error=True)
                    for key, value in valid:
                        response = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertFalse(response.isError)
                        summary = json.loads(response.content[0].text)['player_aura_observations']
                        if value is None:
                            self.assertIsNone(summary)
                            continue
                        self.assertFalse(summary['effectiveness_verified'])
                        self.assertFalse(summary['cast_eligibility_verified'])
                        for kind in ['helpful', 'harmful']:
                            group = value['groups'].get(kind)
                            if group is None:
                                self.assertIsNone(summary[kind])
                                continue
                            rows = group['entries'].values()
                            ids = sorted({row['spell_id'] for row in rows})
                            self.assertEqual(summary[kind]['observed_rows'], len(group['entries']))
                            self.assertEqual(summary[kind]['distinct_spell_ids'], len(ids))
                            self.assertEqual(summary[kind]['spell_ids'], ids[:50])
                            self.assertEqual(summary[kind]['spell_ids_truncated'], len(ids) > 50)
                            self.assertEqual(summary[kind]['rows_with_unknown_applications'], sum('applications' not in row for row in rows))
                        inspected = await session.call_tool('inspect_snapshot', {'snapshot_id': key, 'section': 'player_auras'})
                        self.assertFalse(inspected.isError)
                        self.assertEqual(json.loads(inspected.content[0].text)['data'], value)
                    for key in bad_ids:
                        response = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertTrue(response.isError)
                        self.assertNotIn('PRIVATE_AURA', str(response))


if __name__ == '__main__':
    unittest.main()
