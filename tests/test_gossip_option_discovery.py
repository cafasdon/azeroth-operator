"""Actual saved-dialogue discovery, unknowns, latest selection and owner scope."""
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


class GossipDiscoveryTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_same_row_unknowns_latest_and_private_arguments(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        def capture(name,day,rows=None):
            value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['character']['name']=name;value['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            if rows is not None:value['sections']['gossip_options']={'coverage':'visible_gossip_options','entries':rows}
            return value
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder));keys={}
            cases={'old':capture('Alpha',1,{'1':{'name':'Écho','option_id':42,'reported_status':0}}),
                'empty':capture('Alpha',2,{}), 'missing':capture('Delta',1),
                'mixed':capture('Gamma',1,{'1':{'name':'Écho','option_id':42,'reported_status':0,'reported_flags':0,'spell_id':100},
                    '2':{'name':'Écho','reported_status':0}, '3':{'name':'Écho','reported_status':1},
                    '4':{'name':'Other','option_id':42,'reported_status':0}}),
                'cross':capture('Bravo',1,{'1':{'name':'Other','option_id':42},'2':{'name':'Écho','option_id':43}}),
                'max':capture('High',1,{str(i):{'name':'Repeated','option_id':42,'reported_flags':0} for i in range(1,129)})}
            cases['mixed']['sections']['bags']={'0':{'1':{'item_id':11,'count':2}}}
            for name,value in cases.items():keys[name]=store.ingest(value)
            foreign_key=SnapshotStore(Path(foreign)).ingest(capture('PRIVATE_OTHER',1,{'1':{'name':'Écho','option_id':42}}))
            before={p:p.read_bytes() for base in [folder,foreign] for p in Path(base).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (reader,writer):
                async with ClientSession(reader,writer) as session:
                    await session.initialize()
                    async def listing(query=None,error=False,**extra):
                        args={'family':'forever',**extra}
                        if query is not None:args['gossip_option']=query
                        reply=await session.call_tool('list_snapshots',args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign_key,str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    query={'name':' éCHO ','option_id':42,'reported_status':0}
                    result=await listing(query)
                    self.assertEqual({row['snapshot_id'] for row in result['snapshots']},{keys['old'],keys['mixed']})
                    mixed=next(row['gossip_option_match'] for row in result['snapshots'] if row['snapshot_id']==keys['mixed'])
                    self.assertEqual([mixed[field] for field in ['captured_option_count','matching_option_count','unknown_option_count','nonmatching_option_count']],[4,1,1,2])
                    self.assertEqual(mixed['matching_options'][0]['slot'],1)
                    self.assertEqual(mixed['matching_options'][0]['observation']['name'],'Écho')
                    for field in ['npc_identity_verified','selection_eligibility_verified','complete_dialogue_coverage_verified','current_availability_verified','game_execution','live']:
                        self.assertIs(mixed[field],False)
                    latest=await listing(query,latest_per_character=True)
                    self.assertEqual([row['snapshot_id'] for row in latest['snapshots']],[keys['mixed']])
                    dated=await listing(query,latest_per_character=True,captured_through='2025-01-01T23:59:59Z')
                    self.assertEqual({row['snapshot_id'] for row in dated['snapshots']},{keys['mixed'],keys['old']})
                    self.assertEqual((await listing(query,character={'name':'Bravo','realm':'Test'}))['snapshots'],[])
                    self.assertEqual(len((await listing(query,inventory_item={'item_id':11,'storage':'bags','minimum_quantity':2}))['snapshots']),1)
                    self.assertEqual((await listing(query,inventory_item={'item_id':11,'storage':'bags','minimum_quantity':3}))['snapshots'],[])
                    spell=await listing({'spell_id':100,'reported_flags':0})
                    self.assertEqual([row['snapshot_id'] for row in spell['snapshots']],[keys['mixed']])
                    many=(await listing({'name':'Repeated','reported_flags':0}))['snapshots'][0]['gossip_option_match']
                    self.assertEqual(many['matching_option_count'],128);self.assertTrue(many['matches_truncated'])
                    self.assertEqual([row['slot'] for row in many['matching_options']],list(range(1,21)))
                    first=await listing({'option_id':42},limit=1);second=await listing({'option_id':42},offset=1,limit=1)
                    self.assertNotEqual(first['snapshots'][0]['snapshot_id'],second['snapshots'][0]['snapshot_id'])
                    raw=await listing();self.assertTrue(all('gossip_option_match' not in row for row in raw['snapshots']))
                    null=await session.call_tool('list_snapshots',{'family':'forever','gossip_option':None})
                    self.assertFalse(null.isError);self.assertEqual(json.loads(null.content[0].text),raw)
                    for bad in [{},{'name':' '},{'name':'é'*257},{'name':True},{'option_id':0},
                        {'option_id':'PRIVATE'},{'spell_id':-1},{'spell_id':4294967296},
                        {'reported_status':True},{'reported_flags':1.5},{'reported_flags':-1},
                        {'reported_status':'PRIVATE'},{'unexpected':'PRIVATE'},[],1,'PRIVATE']:
                        await listing(bad,True)
                    for family in ['retail','classic']:
                        await listing({'option_id':42},True,family=family)
                    self.assertEqual(before,{p:p.read_bytes() for p in before})
                    echo = {'name':'Écho','option_id':42,'reported_status':0}
                    other = {'name':'Other','option_id':42,'reported_order_index':-1}
                    updated = {**echo, 'reported_status':1}
                    compare_keys = [
                        store.ingest(capture('Compare', day, rows))
                        for day, rows in enumerate([
                            {'1':echo,'2':other,'3':echo},
                            {'1':other,'2':echo,'3':echo},
                            {'1':other,'2':echo,'3':updated},
                            {}, None,
                        ], 1)
                    ]
                    async def compare(after, *, before_id=compare_keys[0], error=False, enabled=True):
                        reply = await session.call_tool('compare_snapshots', {
                            'before_id':before_id,'after_id':after,'gossip_option_changes':enabled})
                        self.assertEqual(bool(reply.isError),error,reply)
                        return None if error else json.loads(reply.content[0].text)

                    reordered = (await compare(compare_keys[1]))['gossip_option_changes']
                    self.assertEqual(reordered['row_count_changes'], [])
                    self.assertEqual(reordered['before_observed_rows'], 3)
                    changed = (await compare(compare_keys[2]))['gossip_option_changes']
                    self.assertCountEqual(changed['row_count_changes'], [
                        {'observation':echo,'before_count':2,'after_count':1},
                        {'observation':updated,'before_count':0,'after_count':1},
                    ])
                    for flag in ['same_npc_verified','same_dialogue_verified',
                                 'complete_dialogue_coverage_verified','live']:
                        self.assertIs(changed[flag], False)
                    empty = (await compare(compare_keys[3]))['gossip_option_changes']
                    self.assertEqual(empty['after_observed_rows'], 0)
                    self.assertEqual(sum(row['before_count'] for row in empty['row_count_changes']), 3)
                    missing = (await compare(compare_keys[4]))['gossip_option_changes']
                    self.assertIsNone(missing['after_observed_rows'])
                    self.assertIsNone(missing['row_count_changes'])
                    self.assertNotIn('gossip_option_changes', await compare(compare_keys[2], enabled=False))
                    await compare(keys['mixed'], error=True)
                    await compare(foreign_key, error=True)
                    retail = fixture()
                    retail_key = store.ingest(retail)
                    await compare(retail_key, before_id=retail_key, error=True)
            self.assertEqual(before,{p:p.read_bytes() for p in before})


if __name__=='__main__':unittest.main()
