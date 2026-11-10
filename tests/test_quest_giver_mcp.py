"""Quest references survive owner-local storage and actual offline MCP callers."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_quest_giver_validation import fixture


class QuestGiverMcpTests(unittest.IsolatedAsyncioTestCase):
    async def test_inspection_summary_discovery_and_owner_boundary(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder))
            value=fixture();key=store.ingest(value)
            missing=deepcopy(value);missing['sections']={};missing['captured_at']='2025-01-02T00:00:00Z'
            missing_key=store.ingest(missing)
            empty=deepcopy(value);empty['character']['name']='Empty'
            section=empty['sections']['quest_giver_references']
            section['reported_quest_count']=0;section['entries']={}
            empty['sections']['quests']={}
            empty_key=store.ingest(empty)
            other=deepcopy(value);other['character']['name']='PRIVATE_OTHER_OWNER'
            other_key=SnapshotStore(Path(foreign)).ingest(other)
            original={p:p.read_bytes() for directory in [folder,foreign] for p in Path(directory).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,
                    'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    inspected=await call('inspect_snapshot',snapshot_id=key,section='quest_giver_references')
                    self.assertTrue(inspected['available'])
                    self.assertEqual(inspected['data'],value['sections']['quest_giver_references'])
                    summary=(await call('character_summary',snapshot_id=key))['quest_giver_reference_observation']
                    self.assertEqual(summary['reported_quest_count'],3)
                    self.assertEqual(summary['captured_quest_count'],3)
                    self.assertEqual(summary['status_counts'],{'reported':1,'unavailable':1,'not_in_provider':1})
                    for flag in ['current_availability_verified','native_identity_verified',
                            'complete_world_data_verified','game_execution']:
                        self.assertFalse(summary[flag])
                    self.assertIsNone((await call('character_summary',snapshot_id=missing_key))['quest_giver_reference_observation'])
                    missing_view=await call('inspect_snapshot',snapshot_id=missing_key,section='quest_giver_references')
                    self.assertFalse(missing_view['available']);self.assertIsNone(missing_view['data'])
                    empty_summary=(await call('character_summary',snapshot_id=empty_key))['quest_giver_reference_observation']
                    self.assertEqual(empty_summary['captured_quest_count'],0)
                    found=await call('list_snapshots',family='forever',required_sections=['quest_giver_references'])
                    self.assertEqual({r['snapshot_id'] for r in found['snapshots']},{key,empty_key})
                    latest=await call('list_snapshots',family='forever',required_sections=['quest_giver_references'],
                        latest_per_character=True)
                    self.assertEqual([r['snapshot_id'] for r in latest['snapshots']],[empty_key])
                    await call('inspect_snapshot',error=True,snapshot_id=other_key,section='quest_giver_references')
            self.assertEqual(original,{p:p.read_bytes() for directory in [folder,foreign]
                for p in Path(directory).glob('*.json')})
