"""Historical battlefield status discovery through the actual owner-local MCP."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class BattlefieldQueueFilterTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_latest_threshold_slot_composition_and_privacy(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        def capture(name,day,statuses):
            value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['character']['name']=name;value['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            value['sections']['bags']={'0':{'1':{'item_id':10,'count':2}}}
            if statuses is not None:value['sections']['battlefield_queue_slots']={
                'coverage':'reported_battlefield_queue_slots','reported_slot_count':len(statuses),
                'entries':{str(i):{'reported_status':status} for i,status in enumerate(statuses,1)}}
            return value
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as other:
            store=SnapshotStore(Path(folder))
            cases={'old_zero':capture('Alpha',1,['queued']),'new_zero':capture('Alpha',2,[]),
                'old_missing':capture('Bravo',1,['queued']),'new_missing':capture('Bravo',2,None),
                'mixed':capture('Gamma',1,['none','queued','confirm','active','locked','queued']),
                'all_none':capture('Delta',1,['none']*32),'all_queued':capture('Echo',1,['queued']*32)}
            keys={k:store.ingest(v) for k,v in cases.items()}
            private=SnapshotStore(Path(other)).ingest(capture('PRIVATE_OTHER_OWNER',1,['queued']*32))
            original={p:p.read_bytes() for d in [folder,other] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args);self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(other,str(reply));self.assertNotIn(private,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    async def search(filter,**extra):
                        return await call('list_snapshots',family='forever',battlefield_queue=filter,**extra)
                    history=await search({'reported_status':'queued'})
                    self.assertEqual({r['snapshot_id'] for r in history['snapshots']},
                        {keys[k] for k in ['old_zero','old_missing','mixed','all_queued']})
                    latest=await search({'reported_status':'queued'},latest_per_character=True)
                    self.assertEqual({r['snapshot_id'] for r in latest['snapshots']},{keys['mixed'],keys['all_queued']})
                    dated=await search({'reported_status':'queued'},latest_per_character=True,captured_through='2025-01-01T23:59:59Z')
                    self.assertEqual({r['snapshot_id'] for r in dated['snapshots']},{r['snapshot_id'] for r in history['snapshots']})
                    for token,expected in [('none',['mixed','all_none']),('confirm',['mixed']),('active',['mixed']),('locked',['mixed'])]:
                        found=await search({'reported_status':token})
                        self.assertEqual({r['snapshot_id'] for r in found['snapshots']},{keys[k] for k in expected})
                    for filter,expected in [({'reported_status':'queued','minimum_matching_slots':2},['mixed','all_queued']),
                            ({'reported_status':'queued','minimum_matching_slots':32},['all_queued']),
                            ({'reported_status':'queued','slot':2},['mixed','all_queued']),
                            ({'reported_status':'queued','slot':32},['all_queued']),
                            ({'reported_status':'queued','slot':2,'minimum_matching_slots':2},[])]:
                        result=await search(filter)
                        self.assertEqual({r['snapshot_id'] for r in result['snapshots']},{keys[k] for k in expected})
                    maximum=await search({'reported_status':'queued','minimum_matching_slots':32})
                    match=maximum['snapshots'][0]['battlefield_queue_match']
                    self.assertEqual(match['matching_slots'],list(range(1,33)))
                    self.assertEqual(match['matching_slot_count'],32);self.assertEqual(match['reported_slot_count'],32)
                    for flag in ['current_queue_membership_verified','complete_queue_coverage_verified','queue_identity_verified',
                        'acceptance_eligibility_verified','game_execution','live']:self.assertFalse(match[flag])
                    combined=await search({'reported_status':'queued'},section_capture={
                        'section':'battlefield_queue_slots','status':'not_recorded'})
                    self.assertEqual(combined['snapshots'],[])
                    self.assertEqual((await search({'reported_status':'queued'},inventory_item={
                        'item_id':10,'storage':'bags','minimum_quantity':3}))['snapshots'],[])
                    for offset in [0,1,2,18446744073709551615]:
                        paged=await search({'reported_status':'queued'},offset=offset,limit=1)
                        self.assertEqual(paged['snapshots'],history['snapshots'][offset:offset+1])
                    plain=await call('list_snapshots',family='forever')
                    self.assertTrue(all('battlefield_queue_match' not in r for r in plain['snapshots']))
                    self.assertEqual(await search(None),plain)
                    invalid=[{},None,True,1,[],{'reported_status':None},{'reported_status':True},
                        {'reported_status':'PRIVATE'},{'reported_status':'Queued'},{'reported_status':{'queued':None}},
                        {'reported_status':'queued','extra':'PRIVATE'}]
                    for field in ['slot','minimum_matching_slots']:
                        for value in [0,33,-1,1.0,True,'PRIVATE',{},[]]:
                            invalid.append({'reported_status':'queued',field:value})
                    for filter in invalid:
                        if filter is not None:await call('list_snapshots',True,family='forever',battlefield_queue=filter)
                    for family in ['retail','classic']:
                        await call('list_snapshots',True,family=family,battlefield_queue={'reported_status':'queued'})
            for path,content in original.items():self.assertEqual(path.read_bytes(),content)


if __name__=='__main__':unittest.main()
