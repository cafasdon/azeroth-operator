"""Character selection never substitutes newest saved data for live identity."""
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


class CharacterResolutionTests(unittest.IsolatedAsyncioTestCase):
    async def test_explicit_selection_unknown_default_and_owner_boundary(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryDirectory() as other:
            store = SnapshotStore(Path(folder))
            base = fixture()
            base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
            first = store.ingest(base)
            later = deepcopy(base)
            later['captured_at'] = '2025-01-02T00:00:00Z'
            expected = store.ingest(later)
            newest = deepcopy(later)
            newest['captured_at'] = '2025-01-03T00:00:00Z'
            newest['character']['name'] = 'Another'
            store.ingest(newest)
            foreign = deepcopy(newest)
            foreign['character']['name'] = 'PrivateOtherOwner'
            SnapshotStore(Path(other)).ingest(foreign)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def resolve(args):
                        reply = await session.call_tool('resolve_character', args)
                        self.assertFalse(reply.isError, reply)
                        return json.loads(reply.content[0].text)
                    default = await resolve({'family': 'forever'})
                    self.assertEqual(default, {'status': 'active_identity_unavailable', 'selection': None,
                                               'live': False, 'active_character_verified': False})
                    result = await resolve({'family': 'forever', 'character': {'name': 'example', 'realm': 'test'}})
                    self.assertEqual(result['status'], 'explicit_saved_character')
                    self.assertEqual(result['selection']['snapshot_id'], expected)
                    self.assertNotEqual(first, expected)
                    self.assertFalse(result['active_character_verified'])
                    self.assertFalse(result['live'])
                    for family, name in [('retail', 'Example'), ('classic', 'Example'),
                                         ('forever', 'Missing'), ('forever', 'PrivateOtherOwner')]:
                        result = await resolve({'family': family, 'character': {'name': name, 'realm': 'Test'}})
                        self.assertEqual(result['status'], 'no_matching_snapshot')
                        self.assertIsNone(result['selection'])
                        self.assertNotIn('PrivateOtherOwner', json.dumps(result))
                    for args in [ {'family': 'invalid'}, {'family': 'forever', 'active_character': newest['character']},
                                  {'family': 'forever', 'character': {'name': '', 'realm': 'Test'}},
                                  {'family': 'forever', 'character': {'name': 'Example'}},
                                  {'family': 'forever', 'character': {'name': 'Example', 'realm': 'Test', 'path': other}}]:
                        reply = await session.call_tool('resolve_character', args)
                        self.assertTrue(reply.isError)
                        self.assertNotIn(other, str(reply))
                    tools = {tool.name for tool in (await session.list_tools()).tools}
                    self.assertIn('resolve_character', tools)


if __name__ == '__main__':
    unittest.main()
