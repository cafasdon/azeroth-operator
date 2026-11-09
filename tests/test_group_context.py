"""Group context is personal historical metadata, never a roster or social action."""
import asyncio
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


class GroupContextTests(unittest.IsolatedAsyncioTestCase):
    async def test_strict_reader_parity_inspection_summary_and_privacy(self):
        root=Path(__file__).resolve().parents[1]
        base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        row={'in_group':True,'in_raid':False,'reported_member_count':5,'player_is_leader':False}
        section={'coverage':'reported_player_group_context','categories':{'home':row}}
        both=deepcopy(section);both['categories']['instance']={**row,'in_raid':True,'reported_member_count':40,'player_is_leader':True}
        zero={'coverage':section['coverage'],'categories':{'instance':{'in_group':False,'in_raid':False,'reported_member_count':0}}}
        maximum=deepcopy(both);maximum['categories']['home']['reported_member_count']=4294967295
        inconsistent=deepcopy(section);inconsistent['categories']['home'].update(in_group=False,in_raid=True,reported_member_count=0)
        invalid=[None,{},[],{**section,'coverage':'complete_roster'},{**section,'categories':{}},
            {**section,'categories':None},{**section,'categories':[]},{**section,'name':'PRIVATE'}]
        for category in ['default','raid','HOME','PRIVATE']:
            invalid.append({**section,'categories':{category:row}})
        for bad in [None,{},[],{**row,'name':'PRIVATE'},{**row,'guid':'PRIVATE'},
                    {key:value for key,value in row.items() if key!='in_group'},
                    {key:value for key,value in row.items() if key!='in_raid'},
                    {key:value for key,value in row.items() if key!='reported_member_count'}]:
            invalid.append({**section,'categories':{'home':bad}})
        for field in ['in_group','in_raid','player_is_leader']:
            for value in [None,0,1,'true',[],{}]:invalid.append({**section,'categories':{'home':{**row,field:value}}})
        for value in [None,True,False,-1,1.0,'1',4294967296,{},[]]:
            invalid.append({**section,'categories':{'home':{**row,'reported_member_count':value}}})
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));valid=[];bad=[]
            for value in [None,section,both,zero,maximum,inconsistent]:
                saved=deepcopy(base)
                if value is inconsistent:saved['captured_at']='2025-01-02T00:00:00Z'
                if value is not None:saved['sections']['group_context']=value
                valid.append((store.ingest(saved),value))
            other = deepcopy(base)
            other['character']['name'] = 'Other'
            other_id = store.ingest(other)
            retail = deepcopy(base)
            retail['client'] = {'family':'retail','variant':'live','build':'1','interface':120000}
            retail_id = store.ingest(retail)
            snapshots=[]
            for value in invalid:
                saved=deepcopy(base);saved['sections']['group_context']=value;snapshots.append(saved)
            for family,interface in [('retail',120000),('classic',11500)]:
                saved=deepcopy(base);saved['client']={'family':family,'variant':'live','build':'1','interface':interface}
                saved['sections']['group_context']=section;snapshots.append(saved)
            for saved in snapshots:
                with self.assertRaises(ValueError):validate_snapshot(saved)
                raw=json.dumps(saved,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/(key+'.json')).write_bytes(raw);bad.append(key)
            before={path.name:path.read_bytes() for path in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(root/'target/debug/azeroth-operator.exe'),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        result=await asyncio.wait_for(session.call_tool(tool,args),timeout=10)
                        self.assertEqual(bool(result.isError),error,result);self.assertNotIn('PRIVATE',str(result))
                        return None if error else json.loads(result.content[0].text)
                    for key,value in valid:
                        inspection=await call('inspect_snapshot',snapshot_id=key,section='group_context')
                        summary=(await call('character_summary',snapshot_id=key))['group_context_observation']
                        if value is None:self.assertFalse(inspection['available']);self.assertIsNone(summary)
                        else:
                            self.assertEqual(inspection['data'],value);self.assertEqual(summary['categories'],value['categories'])
                            for flag in ['current_group_verified','group_identity_verified','complete_roster_verified','social_action_eligibility_verified','game_execution']:
                                self.assertIs(summary[flag],False)
                    earlier, later = valid[4][0], valid[5][0]
                    async def search(**fields):
                        return (await call('list_snapshots',family='forever',group_context=fields))['snapshots']
                    leader = await search(category='instance',in_raid=True,player_is_leader=True,minimum_reported_member_count=40)
                    self.assertEqual({row['snapshot_id'] for row in leader}, {valid[2][0],earlier})
                    self.assertEqual(leader[0]['group_context_match']['observation'], both['categories']['instance'])
                    self.assertIs(leader[0]['group_context_match']['current_group_verified'],False)
                    page = await call('list_snapshots',family='forever',limit=1,offset=1,
                        group_context={'category':'instance','in_raid':True})
                    self.assertEqual([row['snapshot_id'] for row in page['snapshots']], [leader[1]['snapshot_id']])
                    plain_list = await call('list_snapshots',family='forever',limit=1)
                    self.assertNotIn('group_context_match',plain_list['snapshots'][0])
                    solo = await search(category='instance',in_group=False,minimum_reported_member_count=0)
                    self.assertEqual([row['snapshot_id'] for row in solo], [valid[3][0]])
                    self.assertEqual(await search(category='instance',maximum_reported_member_count=0),solo)
                    self.assertEqual(await search(category='instance',minimum_reported_member_count=40,
                        maximum_reported_member_count=40),leader)
                    self.assertEqual(await search(category='instance',minimum_reported_member_count=1,
                        maximum_reported_member_count=39),[])
                    await call('list_snapshots',error=True,family='forever',group_context={
                        'category':'instance','minimum_reported_member_count':40,'maximum_reported_member_count':39})
                    self.assertEqual(await search(category='instance',in_group=False,player_is_leader=False),[])
                    self.assertEqual(await search(category='home',in_group=True,in_raid=True),[])
                    latest = await call('list_snapshots',family='forever',latest_per_character=True,group_context={'category':'instance'})
                    self.assertEqual(latest['snapshots'],[])
                    for query in [{}, {'category':'default'}, {'category':'home','player_is_leader':0}, {'category':'home','roster':True}]:
                        await call('list_snapshots',error=True,family='forever',group_context=query)
                    await call('list_snapshots',error=True,family='retail',group_context={'category':'home'})
                    plain = await call('compare_snapshots',before_id=earlier,after_id=later)
                    self.assertNotIn('group_context_changes',plain)
                    comparison = await call('compare_snapshots',before_id=earlier,after_id=later,group_context_changes=True)
                    group = comparison['group_context_changes']
                    self.assertEqual(group['categories']['home'], {
                        'before':maximum['categories']['home'], 'after':inconsistent['categories']['home'],
                        'reported_member_count_delta':-4294967295,
                        'reported_in_group_changed':True, 'reported_in_raid_changed':True,
                        'reported_player_is_leader_changed':False,
                    })
                    self.assertIsNone(group['categories']['instance']['after'])
                    self.assertIsNone(group['categories']['instance']['reported_member_count_delta'])
                    for field in ['in_group','in_raid','player_is_leader']:
                        self.assertIsNone(group['categories']['instance'][f'reported_{field}_changed'])
                    optional = await call('compare_snapshots',before_id=valid[3][0],after_id=earlier,
                        group_context_changes=True)
                    self.assertIsNone(optional['group_context_changes']['categories']['instance'][
                        'reported_player_is_leader_changed'])
                    for flag in ['group_identity_verified','membership_events_verified','live']:
                        self.assertIs(group[flag],False)
                    for a,b in [(later,earlier), (other_id,later), (retail_id,retail_id), (retail_id,later)]:
                        await call('compare_snapshots',error=True,before_id=a,after_id=b,group_context_changes=True)
                    for key in bad:await call('inspect_snapshot',error=True,snapshot_id=key,section='group_context')
            self.assertEqual(before,{path.name:path.read_bytes() for path in Path(folder).glob('*.json')})


if __name__=='__main__':unittest.main()
