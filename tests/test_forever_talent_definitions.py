"""Native definition observations retain uncertainty without claiming model parity."""
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


def sample():
    value=fixture(); value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
    value['sections']['talents']={'config_id':7,'observed_tree_ids':{'10':True,'20':True},'observed_node_count':5,
        'staged_changes_observed':False,'nodes':{
            '101':{'entry_id':111,'rank':7,'tree_id':10,'definition_id':211,'purchased_rank':3,'max_rank':5},
            '102':{'entry_id':112,'rank':1,'tree_id':10,'definition_id':212,'purchased_rank':0,'max_rank':1},
            '103':{'entry_id':113,'rank':2,'tree_id':20}},
        'loadout_export':{'status':'unavailable','reason':'client_not_supported'}}
    return value


class ForeverTalentDefinitionTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_reader_summary_metadata_guards_and_legacy(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); store=SnapshotStore(base); value=sample(); key=store.ingest(value)
            before=(base/f'{key}.json').read_bytes()
            env={**os.environ,'AZEROTH_DATA_DIR':folder}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    reply=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'talents'})
                    self.assertFalse(reply.isError,reply)
                    self.assertEqual(json.loads(reply.content[0].text)['data'],value['sections']['talents'])
                    async def summary(observation):
                        snapshot=store.ingest(observation)
                        reply=await session.call_tool('character_summary',{'snapshot_id':snapshot})
                        self.assertFalse(reply.isError,reply)
                        return json.loads(reply.content[0].text)
                    observed=(await summary(value))['talent_definition_observation']
                    self.assertEqual(observed,{'basis':'reported_active_config_trait_observations','observed_tree_count':2,
                        'observed_node_count':5,'active_entry_nodes':3,'nodes_with_definition_observation':2,
                        'nodes_without_definition_observation':1,'staged_changes_observed':False,
                        'allocation_match_verified':False,'simulation_mapping_verified':False,'game_execution':False})
                    for stage in [True,None]:
                        changed=deepcopy(value)
                        if stage is None: del changed['sections']['talents']['staged_changes_observed']
                        else: changed['sections']['talents']['staged_changes_observed']=stage
                        self.assertIs((await summary(changed))['talent_definition_observation']['staged_changes_observed'],stage)
                    unknown=deepcopy(value)
                    for node in unknown['sections']['talents']['nodes'].values():
                        for field in ['definition_id','purchased_rank','max_rank']: node.pop(field,None)
                    self.assertEqual((await summary(unknown))['talent_definition_observation']['nodes_without_definition_observation'],3)
                    legacy=deepcopy(unknown)
                    for field in ['observed_tree_ids','observed_node_count','staged_changes_observed']:
                        legacy['sections']['talents'].pop(field,None)
                    self.assertNotIn('talent_definition_observation',await summary(legacy))
                    malformed=[]
                    for field,bad in [('observed_tree_ids',None),('observed_tree_ids',{}),('observed_tree_ids',{'0':True}),
                            ('observed_tree_ids',{'01':True}),('observed_tree_ids',{'10':False}),('observed_tree_ids',{'10':1}),
                            ('observed_node_count',True),('observed_node_count',2),('observed_node_count',4097),
                            ('observed_node_count',1.5),('staged_changes_observed',None),('staged_changes_observed',1),('extra','PRIVATE')]:
                        broken=deepcopy(value); broken['sections']['talents'][field]=bad; malformed.append(broken)
                    for field in ['observed_tree_ids','observed_node_count']:
                        broken=deepcopy(value); del broken['sections']['talents'][field]; malformed.append(broken)
                    for field,bad in [('definition_id',0),('definition_id',4294967296),('purchased_rank',-1),
                            ('purchased_rank',True),('purchased_rank',6),('max_rank',0),('max_rank',None),
                            ('rank',0),('tree_id',999),('definition_id',None),('extra','PRIVATE')]:
                        broken=deepcopy(value); broken['sections']['talents']['nodes']['101'][field]=bad; malformed.append(broken)
                    for field in ['definition_id','purchased_rank','max_rank']:
                        broken=deepcopy(value); del broken['sections']['talents']['nodes']['101'][field]; malformed.append(broken)
                    for family in ['retail','classic']:
                        broken=deepcopy(value); broken['client']['family']=family; malformed.append(broken)
                    broken=deepcopy(value); broken['sections']['talents']['observed_tree_ids']={str(i):True for i in range(1,18)}; malformed.append(broken)
                    for index,broken in enumerate(malformed):
                        with self.subTest(index=index):
                            with self.assertRaises(ValueError): validate_snapshot(broken)
                            data=json.dumps(broken,sort_keys=True,separators=(',',':')).encode()
                            bad_key=hashlib.sha256(data).hexdigest(); (base/f'{bad_key}.json').write_bytes(data)
                            reply=await session.call_tool('inspect_snapshot',{'snapshot_id':bad_key})
                            self.assertTrue(reply.isError,index); self.assertNotIn('PRIVATE',str(reply))
                    for count in [0,4096]:
                        boundary=deepcopy(value); talents=boundary['sections']['talents']
                        talents['observed_tree_ids']={'10':True}; talents['observed_node_count']=count
                        talents['nodes']={str(i):deepcopy(value['sections']['talents']['nodes']['101']) for i in range(1,count+1)}
                        result=(await summary(boundary))['talent_definition_observation']
                        self.assertEqual(result['active_entry_nodes'],count)
                        self.assertEqual(result['nodes_with_definition_observation'],count)
            self.assertEqual((base/f'{key}.json').read_bytes(),before)


if __name__=='__main__':
    unittest.main()
