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
            empty['character']['name'] = 'Empty'
            empty['sections']['saved_world_bosses']['entries'] = {}
            empty_key = store.ingest(empty)
            missing_key = store.ingest(fixture())
            negative = copy.deepcopy(value)
            negative['character']['name'] = 'Negative'
            negative['sections']['saved_world_bosses']['entries']['1']['reset_seconds'] = -1
            negative_key = store.ingest(negative)
            newest = copy.deepcopy(value)
            newest['captured_at'] = '2025-01-02T00:00:00Z'
            newest['sections'].pop('saved_world_bosses')
            newest_key = store.ingest(newest)
            many = copy.deepcopy(value)
            many['character']['name'] = 'Many'
            many['sections']['saved_world_bosses']['entries'] = {
                str(i): {'name': 'Boss', 'reset_seconds': i} for i in range(1, 26)}
            many_key = store.ingest(many)
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
                    async def search(**arguments):
                        result = await session.call_tool('list_snapshots', {'family':'forever', **arguments})
                        self.assertFalse(result.isError)
                        return json.loads(result.content[0].text)
                    query = {'character':value['character'], 'saved_world_boss':{'name':'bOsS'}}
                    rows = (await search(**query))['snapshots']
                    self.assertEqual([row['snapshot_id'] for row in rows], [key])
                    self.assertEqual(rows[0]['saved_world_boss_match'], {
                        'update_age_seconds':7,'matched_count':1,'truncated':False,
                        'entries':[{'slot':1,'data':{'name':'Boss','reset_seconds':124}}]})
                    for reset, age, expected in [(124,7,1),(123,7,0),(124,6,0),(0,7,0)]:
                        q = {**query,'saved_world_boss':{'name':'Boss',
                            'maximum_reset_seconds':reset,'maximum_update_age_seconds':age}}
                        self.assertEqual(len((await search(**q))['snapshots']), expected)
                    self.assertEqual((await search(**query,latest_per_character=True))['snapshots'], [])
                    self.assertEqual(len((await search(**query,latest_per_character=True,
                        captured_through='2025-01-01T00:00:00Z'))['snapshots']),1)
                    q = {'character':negative['character'],'saved_world_boss':{'name':'Boss'}}
                    self.assertEqual((await search(**q))['snapshots'][0]['snapshot_id'],negative_key)
                    q['saved_world_boss']['maximum_reset_seconds'] = 4_294_967_295
                    self.assertEqual((await search(**q))['snapshots'], [])
                    result = (await search(character=many['character'],saved_world_boss={'name':'Boss'}))['snapshots'][0]
                    self.assertEqual(result['snapshot_id'],many_key)
                    self.assertEqual(result['saved_world_boss_match']['matched_count'],25)
                    self.assertTrue(result['saved_world_boss_match']['truncated'])
                    self.assertEqual([row['slot'] for row in result['saved_world_boss_match']['entries']],list(range(1,21)))
                    for bad in [{'name':''},{'name':'  '},{'name':'x'*257},
                                {'name':'Boss','maximum_reset_seconds':-1},
                                {'name':'Boss','maximum_update_age_seconds':True},
                                {'name':'Boss','boss_id':1}]:
                        result = await session.call_tool('list_snapshots',{'family':'forever','saved_world_boss':bad})
                        self.assertTrue(result.isError)
                    result = await session.call_tool('list_snapshots',{'family':'retail','saved_world_boss':{'name':'Boss'}})
                    self.assertTrue(result.isError)
                    self.assertTrue(all('saved_world_boss_match' not in row for row in (await search())['snapshots']))
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
