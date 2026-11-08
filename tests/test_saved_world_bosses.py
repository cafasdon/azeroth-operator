"""Owner-scoped saved world boss observations through the real MCP reader."""
import copy
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


class WorldBossTests(unittest.IsolatedAsyncioTestCase):
    async def test_capture_readback_and_rejection(self):
        binary = Path(__file__).resolve().parents[1] / 'target/debug' / (
            'azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY', binary)).resolve(strict=True)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            store = SnapshotStore(root)
            value = fixture()
            value['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            value['sections']['saved_world_bosses'] = {'coverage': 'updated_cache', 'update_age_seconds': 7,
                'entries': {'1': {'name': 'Boss', 'reset_seconds': 124}}}
            key = store.ingest(value)
            empty = copy.deepcopy(value)
            empty['sections']['saved_world_bosses']['entries'] = {}
            empty_key = store.ingest(empty)
            missing_key = store.ingest(fixture())
            bad_keys = []
            mutations = [
                lambda x: x['client'].update(family='retail'),
                lambda x: x['sections']['saved_world_bosses'].update(coverage='live'),
                lambda x: x['sections']['saved_world_bosses'].update(update_age_seconds=True),
                lambda x: x['sections']['saved_world_bosses']['entries']['1'].update(boss_id=123),
                lambda x: x['sections']['saved_world_bosses']['entries']['1'].update(reset_seconds=True),
                lambda x: x['sections']['saved_world_bosses']['entries']['1'].update(name='x'*257),
                lambda x: x['sections']['saved_world_bosses'].update(entries={'2': {'name':'Boss','reset_seconds':1}}),
            ]
            for mutation in mutations:
                bad = copy.deepcopy(value)
                mutation(bad)
                with self.assertRaises(ValueError): validate_snapshot(bad)
                raw = json.dumps(bad, sort_keys=True, separators=(',', ':')).encode()
                bad_key = hashlib.sha256(raw).hexdigest()
                (root / f'{bad_key}.json').write_bytes(raw)
                bad_keys.append(bad_key)
            env = {**os.environ, 'AZEROTH_DATA_DIR': directory}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    result = await session.call_tool('inspect_snapshot', {'snapshot_id':key,'section':'saved_world_bosses'})
                    self.assertFalse(result.isError)
                    self.assertEqual(json.loads(result.content[0].text)['data'], value['sections']['saved_world_bosses'])
                    for source, count in [(key,1),(empty_key,0),(missing_key,None)]:
                        result = await session.call_tool('character_summary', {'snapshot_id': source})
                        self.assertFalse(result.isError)
                        self.assertEqual(json.loads(result.content[0].text)['saved_world_bosses_observed'], count)
                    for source in bad_keys:
                        result = await session.call_tool('inspect_snapshot', {'snapshot_id':source,'section':'saved_world_bosses'})
                        self.assertTrue(result.isError)


if __name__ == '__main__':
    unittest.main()
