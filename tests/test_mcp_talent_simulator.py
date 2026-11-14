"""Actual stdio MCP talent-template review, cross-checked with the Python adapter."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_forever_talent_definitions import sample
from test_simulator_talent_identity import observation
from test_snapshots import fixture
from tools.forever_talent_identity import check_talent_identity, talent_map


class McpTalentSimulatorTests(unittest.IsolatedAsyncioTestCase):
    async def test_all_classes_definitions_positions_and_partial_states_match_python(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            env={**os.environ,'AZEROTH_DATA_DIR':folder}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    schema={tool.name:tool.inputSchema for tool in (await session.list_tools()).tools}
                    self.assertIn('simulator_talent_string',schema['captured_talent_loadout']['properties'])
                    self.assertNotIn('simulator_talent_string',schema['character_summary']['properties'])
                    async def compare(name,observed,text):
                        value=sample();value['sections']['stats']={'class':name.upper(),'level':60,'race':'Orc','client_version':'synthetic'}
                        if observed is None: value['sections'].pop('talents')
                        else: value['sections']['talents']=observed
                        key=store.ingest(value);before=(Path(folder)/f'{key}.json').read_bytes()
                        reply=await session.call_tool('captured_talent_loadout',
                            {'snapshot_id':key,'simulator_talent_string':text})
                        self.assertFalse(reply.isError,reply)
                        data=json.loads(reply.content[0].text)
                        expected=check_talent_identity({'talentsString':text},observed,name,require_match=False)
                        self.assertEqual(data['simulator_talent_observation'],expected,(name,text))
                        self.assertEqual(data['status'], 'unavailable' if observed is None else 'captured_allocation_for_review')
                        if observed is None:
                            self.assertEqual(data['reason'],'not_captured')
                        else:
                            self.assertEqual(data['loadout']['nodes'],observed['nodes'])
                            self.assertFalse(data['native_import_available'])
                            self.assertFalse(data['simulation_model_verified'])
                        self.assertFalse(data['game_execution']);self.assertFalse(data['live'])
                        self.assertNotIn('import_string',data)
                        self.assertEqual((Path(folder)/f'{key}.json').read_bytes(),before)
                    for name,tree in talent_map()['classes'].items():
                        # Synthetic rank fixture intentionally is not a legal build.
                        captured=observation();captured['nodes']={}
                        for index,row in enumerate(tree['talents'],1):
                            captured['nodes'][str(index)]={'entry_id':index,'tree_id':10,'rank':99,
                                'definition_id':row['definition_id'],'purchased_rank':row['max_rank'],
                                'max_rank':row['max_rank']}
                        captured['observed_node_count']=len(captured['nodes'])
                        text='-'.join('9'*size for size in tree['tree_sizes'])
                        await compare(name,captured,text)
                        offset=0
                        for tree_index,size in enumerate(tree['tree_sizes']):
                            for position in [0,size-1]:
                                row=tree['talents'][offset+position]
                                captured=observation(row['definition_id'],0,row['max_rank'])
                                parts=['']*3;parts[tree_index]='0'*position+'9'
                                await compare(name,captured,'-'.join(parts))
                            offset+=size
                    for captured in [None,observation(),observation(purchased=0),observation(999999),
                                     observation(maximum=4)]:
                        await compare('hunter',captured,'1')
                    for stage in [True,None]:
                        captured=observation(purchased=0)
                        if stage is None:captured.pop('staged_changes_observed')
                        else:captured['staged_changes_observed']=stage
                        await compare('hunter',captured,'1')
                    captured=observation();captured['nodes']['101']={'entry_id':111,'rank':9,'tree_id':10}
                    await compare('hunter',captured,'1')
                    captured=observation();captured['nodes']={}
                    await compare('hunter',captured,'--')
                    captured=observation();captured['observed_node_count']=4096
                    captured['nodes']={str(index):{'entry_id':index,'rank':1,'tree_id':10}
                                       for index in range(1,4097)}
                    await compare('hunter',captured,'')

    async def test_legacy_unknowns_retail_and_malformed_requests(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));value=sample()
            value['sections']['stats']={'class':'HUNTER','level':60,'race':'Orc','client_version':'synthetic'}
            value['sections']['talents']=observation();key=store.ingest(value)
            duplicate=deepcopy(value)
            duplicate['sections']['talents']['nodes']['102']=deepcopy(duplicate['sections']['talents']['nodes']['101'])
            duplicate_key=store.ingest(duplicate)
            unknown=deepcopy(value);unknown['sections'].pop('stats');unknown_key=store.ingest(unknown)
            legacy=deepcopy(value);legacy['sections']['talents']={'config_id':7,'nodes':{}}
            legacy_key=store.ingest(legacy)
            retail=fixture();retail['sections']['talents']={'config_id':7,'nodes':{},'loadout_export':{
                'status':'available','format':'blizzard_traits','import_string':'QUJDRA==','spec_id':263,'serialization_version':2,
                'staged_changes':False}}
            retail_key=store.ingest(retail)
            env={**os.environ,'AZEROTH_DATA_DIR':folder}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for snapshot in [key,legacy_key,retail_key]:
                        reply=await session.call_tool('captured_talent_loadout',{'snapshot_id':snapshot})
                        self.assertFalse(reply.isError,reply)
                        data=json.loads(reply.content[0].text)
                        self.assertNotIn('simulator_talent_observation',data)
                        if snapshot==retail_key:self.assertEqual(data['loadout']['import_string'],'QUJDRA==')
                    for snapshot in [unknown_key,legacy_key]:
                        reply=await session.call_tool('captured_talent_loadout',{'snapshot_id':snapshot,'simulator_talent_string':'1'})
                        self.assertFalse(reply.isError,reply)
                        report=json.loads(reply.content[0].text)['simulator_talent_observation']
                        self.assertEqual(report['status'],'unverified')
                        self.assertFalse(report['allocation_match_verified'])
                        if snapshot==unknown_key:self.assertEqual(report['reason'],'captured_class_unmapped')
                    bad=[{'snapshot_id':key,'simulator_talent_string':value}
                         for value in ['PRIVATE','１','1----','---','0'*17,'1a','0'*257,True,1,{}]]
                    bad += [{'snapshot_id':snapshot,'simulator_talent_string':'1'} for snapshot in [duplicate_key,retail_key,'../PRIVATE']]
                    bad += [{'snapshot_id':key,'simulator_talent_string':'1','class':'PRIVATE'}]
                    for args in bad:
                        reply=await session.call_tool('captured_talent_loadout',args)
                        self.assertTrue(reply.isError,args)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(folder,str(reply))


if __name__=='__main__':unittest.main()
