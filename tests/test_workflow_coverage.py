"""Workflow input profiles describe one owner snapshot, never live readiness."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture

FLAGS=['current_state_verified','active_character_verified','workflow_execution_verified',
    'complete_ownership_verified','native_eligibility_verified','complete_game_coverage_verified','game_execution','live']


class WorkflowCoverageTests(unittest.IsolatedAsyncioTestCase):
    async def test_complete_input_report_preserves_identity_status_and_boundaries(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder))
            value=fixture();value['client'].update(family='forever',build='70862',interface=16001)
            value['sections'].update(bags={});value['unavailable']={'character_bank':'PRIVATE_REASON'}
            key=store.ingest(value)
            complete=deepcopy(value);complete['unavailable']={}
            complete['sections'].update(character_bank={},account_bank={});complete_key=store.ingest(complete)
            absent=deepcopy(value);absent['sections']={};absent['unavailable']={};absent_key=store.ingest(absent)
            newer=deepcopy(complete);newer['character']['name']='Other';newer['captured_at']='2025-01-02T00:00:00Z'
            newer_key=store.ingest(newer)
            retail_key=store.ingest(fixture())
            other=deepcopy(value);other['character']['name']='PRIVATE_FOREIGN_OWNER'
            other_key=SnapshotStore(Path(foreign)).ingest(other)
            original={p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')}
            server_env={**os.environ,'AZEROTH_DATA_DIR':folder}
            server_env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=server_env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize();tools=(await session.list_tools()).tools
                    names={tool.name for tool in tools}
                    async def call(snapshot=key,error=False,**extra):
                        reply=await session.call_tool('character_summary',{'snapshot_id':snapshot,**extra})
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE_FOREIGN_OWNER',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    plain=await call();self.assertNotIn('workflow_coverage',plain)
                    self.assertEqual(await call(include_workflow_coverage=False),plain)
                    result=await call(include_workflow_coverage=True)
                    coverage=result.pop('workflow_coverage');self.assertEqual(result,plain)
                    self.assertEqual(coverage['snapshot_id'],key)
                    self.assertEqual(coverage['target_basis'],'explicit_saved_snapshot_id')
                    self.assertEqual(coverage['known_workflow_count'],24)
                    self.assertEqual(len(coverage['workflows']),24)
                    self.assertNotIn('PRIVATE',json.dumps(coverage))
                    self.assertNotIn('available',coverage)
                    for flag in FLAGS:self.assertFalse(coverage[flag])
                    for row in coverage['workflows'].values():
                        entry=row['entrypoint']
                        if entry['catalogue_configuration_needed']:
                            self.assertEqual(entry['tool'],'review_forever_owned_gear')
                            self.assertNotIn(entry['tool'],names)
                        else:
                            self.assertIn(entry['tool'],names)
                        required=row['required_sections'];self.assertTrue(required)
                        expected={name:'observed' if name in value['sections'] else
                            'reported_unavailable' if name in value['unavailable'] else 'not_recorded'
                            for name in required}
                        self.assertEqual(required,expected)
                        expected_missing=sorted(name for name,status in expected.items() if status!='observed')
                        self.assertEqual(row['missing_required_sections'],expected_missing)
                        counts={status:sum(v==status for v in expected.values()) for status in
                            ['observed','reported_unavailable','not_recorded']}
                        self.assertEqual(row['required_status_counts'],counts)
                        self.assertEqual(row['all_required_inputs_observed'],not expected_missing)
                        state='all_observed' if not expected_missing else 'none_observed' if not counts['observed'] else 'some_observed'
                        self.assertEqual(row['input_status'],state)
                        for name,status in row['helpful_sections'].items():
                            self.assertEqual(status,'observed' if name in value['sections'] else
                                'reported_unavailable' if name in value['unavailable'] else 'not_recorded')
                        self.assertTrue(row['outside_snapshot_checks'])
                    self.assertEqual(coverage['workflows']['owned_gear_pool']['input_status'],'some_observed')
                    full=(await call(snapshot=complete_key,include_workflow_coverage=True))['workflow_coverage']
                    self.assertEqual(full['workflows']['owned_gear_pool']['input_status'],'all_observed')
                    self.assertFalse(full['complete_ownership_verified']);self.assertFalse(full['workflow_execution_verified'])
                    empty=(await call(snapshot=absent_key,include_workflow_coverage=True))['workflow_coverage']
                    self.assertTrue(all(row['input_status']=='none_observed' for row in empty['workflows'].values()))
                    second=(await call(snapshot=newer_key,include_workflow_coverage=True))['workflow_coverage']
                    self.assertEqual(second['snapshot_id'],newer_key)
                    self.assertEqual((await call(include_workflow_coverage=True))['workflow_coverage']['snapshot_id'],key)
                    active=await session.call_tool('resolve_character',{'family':'forever'})
                    self.assertFalse(active.isError,active)
                    self.assertEqual(json.loads(active.content[0].text)['status'],'active_identity_unavailable')
                    for bad in [None,0,1,'true',{},[],'PRIVATE_VALUE']:
                        await call(error=True,include_workflow_coverage=bad)
                    await call(snapshot=retail_key,error=True,include_workflow_coverage=True)
                    self.assertNotIn('workflow_coverage',await call(snapshot=retail_key))
                    await call(snapshot=other_key,error=True,include_workflow_coverage=True)
            self.assertEqual(original,{p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')})
