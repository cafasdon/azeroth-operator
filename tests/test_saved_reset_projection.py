"""Historical reset projection through the actual owner-local stdio MCP."""
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


class ResetProjectionTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_calendar_unknowns_scopes_and_privacy(self):
        root = Path(__file__).resolve().parents[1]
        binary = root / 'target/debug' / ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryDirectory() as foreign:
            store = SnapshotStore(Path(folder))
            capture = fixture()
            capture['client'] = {'family':'forever', 'variant':'live', 'interface':16001,
                                 'build':'70862'}
            capture['captured_at'] = '2025-01-01T23:59:00Z'
            capture['sections']['saved_world_bosses'] = {'coverage':'updated_cache',
                'update_age_seconds':999, 'entries':{
                    '1':{'name':'Fixture', 'reset_seconds':120},
                    '2':{'name':'Unknown', 'reset_seconds':-1},
                    '3':{'name':'Zero', 'reset_seconds':0}}}
            capture['sections']['saved_instances'] = {'coverage':'updated_cache',
                'update_age_seconds':5, 'entries':{'1':{'name':'Fixture raid',
                    'reset_seconds':3600, 'difficulty_id':1, 'locked':True,
                    'extended':True, 'is_raid':True, 'max_players':40}}}
            key = store.ingest(capture)
            empty = deepcopy(capture)
            for section in ['saved_instances', 'saved_world_bosses']:
                empty['sections'][section]['entries'] = {}
            empty_key = store.ingest(empty)
            missing = deepcopy(capture)
            for section in ['saved_instances', 'saved_world_bosses']:
                del missing['sections'][section]
            missing_key = store.ingest(missing)
            retail_key = store.ingest(fixture())
            private = deepcopy(capture)
            private['character']['name'] = 'PRIVATE_OTHER'
            foreign_key = SnapshotStore(Path(foreign)).ingest(private)
            before = {p:p.read_bytes() for base in [folder, foreign] for p in Path(base).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),
                    env={**os.environ, 'AZEROTH_DATA_DIR':folder})) as (reader, writer):
                async with ClientSession(reader, writer) as session:
                    await session.initialize()
                    async def inspect(snapshot_id=key, section='saved_world_bosses', flag=True, error=False, **extra):
                        args = {'snapshot_id':snapshot_id, 'saved_reset_projection':flag, **extra}
                        if section is not None:
                            args['section'] = section
                        reply = await session.call_tool('inspect_snapshot', args)
                        self.assertEqual(bool(reply.isError), error, reply)
                        self.assertNotIn('PRIVATE', str(reply))
                        self.assertNotIn(foreign, str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    result = await inspect()
                    self.assertEqual(result['data'], capture['sections']['saved_world_bosses'])
                    projected = result['saved_reset_projection']
                    self.assertEqual(projected['rows'][0]['projected_reset_at'], '2025-01-02T00:01:00Z')
                    self.assertEqual(projected['rows'][1]['projection_status'], 'negative_reported_duration')
                    self.assertIsNone(projected['rows'][1]['projected_reset_at'])
                    self.assertEqual(projected['rows'][2]['projected_reset_at'], capture['captured_at'])
                    self.assertEqual([projected[f] for f in ['recorded_count','projected_count','unknown_count']], [3,2,1])
                    self.assertEqual(projected['cache_update_age_seconds'], 999)
                    for field in ['complete_coverage_verified','current_lockout_verified',
                                  'current_availability_verified','extension_outcome_inferred','game_execution','live']:
                        self.assertIs(projected[field], False)
                    raid = (await inspect(section='saved_instances'))['saved_reset_projection']
                    self.assertEqual(raid['rows'][0]['projected_reset_at'], '2025-01-02T00:59:00Z')
                    self.assertIs(raid['rows'][0]['observation']['extended'], True)
                    self.assertEqual((await inspect(empty_key))['saved_reset_projection']['rows'], [])
                    self.assertIsNone((await inspect(missing_key))['saved_reset_projection']['rows'])
                    self.assertNotIn('saved_reset_projection', await inspect(flag=False))
                    for section in [None, 'quests', 'skill_lines']:
                        await inspect(section=section, error=True)
                    await inspect(retail_key, error=True)
                    await inspect(foreign_key, error=True)
                    for bad in ['PRIVATE_CANARY', 1, 0, None, {}, []]:
                        await inspect(flag=bad, error=True)
                    await inspect(skill_line_filter={'filter':{'minimum_rank':0}}, error=True)
            self.assertEqual(before, {p:p.read_bytes() for p in before})


if __name__ == '__main__':
    unittest.main()
