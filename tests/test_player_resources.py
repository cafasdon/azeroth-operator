"""Player resource observations remain partial, historical and strictly typed."""
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


class PlayerResourcesTests(unittest.IsolatedAsyncioTestCase):
    async def test_summary_inspection_unknowns_and_reader_parity(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        resources = {'coverage': 'readable_player_resources', 'health': {'current': 25, 'maximum': 100},
                     'primary_power': {'current': 0, 'maximum': 200, 'power_type': 0, 'units': 'unmodified'}}
        cases = [None, resources, {'coverage': resources['coverage'], 'health': {'current': 0, 'maximum': 0}},
                 {'coverage': resources['coverage'], 'primary_power': {'current': 4294967295,
                  'maximum': 4294967295, 'power_type': 63, 'units': 'unmodified'}}]
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            valid = []
            for section in cases:
                sample = deepcopy(base)
                if section is not None:
                    sample['sections']['player_resources'] = section
                valid.append((store.ingest(sample), section))
            malformed = []
            for field in ['health', 'primary_power']:
                for component in [None, {}, {'current': 1}, {**resources[field], 'current': None},
                                  {**resources[field], 'maximum': True}, {**resources[field], 'current': 1.5},
                                  {**resources[field], 'current': -1}, {**resources[field], 'current': 201},
                                  {**resources[field], 'maximum': 4294967296},
                                  {**resources[field], 'private': 'PRIVATE_RESOURCE'}]:
                    bad = deepcopy(resources)
                    bad[field] = component
                    malformed.append(bad)
            for field, value in [('power_type', -1), ('power_type', 64), ('power_type', True),
                                 ('power_type', 0.0), ('units', 'display'), ('units', None)]:
                bad = deepcopy(resources)
                bad['primary_power'][field] = value
                malformed.append(bad)
            malformed += [None, {}, {'coverage': resources['coverage']}, {**resources, 'coverage': 'complete'},
                          {**resources, 'private': 'PRIVATE_RESOURCE'}]
            invalid_ids = []
            for section in malformed:
                sample = deepcopy(base)
                sample['sections']['player_resources'] = section
                with self.assertRaises(ValueError):
                    validate_snapshot(sample)
                raw = json.dumps(sample, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                invalid_ids.append(key)
            for family in ['retail', 'classic']:
                sample = deepcopy(base)
                sample['client']['family'] = family
                sample['sections']['player_resources'] = resources
                with self.assertRaises(ValueError):
                    validate_snapshot(sample)
                raw = json.dumps(sample, sort_keys=True, separators=(',', ':')).encode()
                key = hashlib.sha256(raw).hexdigest()
                (Path(folder) / f'{key}.json').write_bytes(raw)
                invalid_ids.append(key)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    for key, section in valid:
                        reply = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertFalse(reply.isError)
                        observation = json.loads(reply.content[0].text)['player_resource_observations']
                        if section is None:
                            self.assertIsNone(observation)
                            continue
                        self.assertFalse(observation['readiness_verified'])
                        for field in ['health', 'primary_power']:
                            row = section.get(field)
                            if row is None:
                                self.assertIsNone(observation[field])
                            else:
                                expected = {**row, 'missing': row['maximum'] - row['current'],
                                            'percent': row['current'] * 100.0 / row['maximum'] if row['maximum'] else None}
                                self.assertEqual(observation[field], expected)
                        inspected = await session.call_tool('inspect_snapshot', {'snapshot_id': key, 'section': 'player_resources'})
                        self.assertFalse(inspected.isError)
                        self.assertEqual(json.loads(inspected.content[0].text)['data'], section)
                    for key in invalid_ids:
                        reply = await session.call_tool('character_summary', {'snapshot_id': key})
                        self.assertTrue(reply.isError)
                        self.assertNotIn('PRIVATE_RESOURCE', str(reply))


if __name__ == '__main__':
    unittest.main()
