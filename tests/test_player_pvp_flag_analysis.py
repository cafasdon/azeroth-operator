"""Actual MCP discovery, summary and comparison of independent player PvP flags."""
from copy import deepcopy
from itertools import product
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture

FIELDS=['is_pvp','is_free_for_all','is_sanctuary']
BOUNDARY=['current_pvp_state_verified','character_continuity_verified','combat_eligibility_verified',
    'zone_safety_verified','pvp_event_inferred','game_execution','live']


class PlayerPvpFlagAnalysisTests(unittest.IsolatedAsyncioTestCase):
    async def test_full_saved_flag_workflow_through_mcp(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        def flags(mask):return {field:bool(mask&(1<<i)) for i,field in enumerate(FIELDS)}
        def capture(name,day,mask):
            value=fixture();value['client'].update(family='forever',build='70862',interface=16001)
            value['character']={'name':name,'realm':'Test'};value['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            if mask is not None:value['sections']['player_pvp_flags']={'coverage':'reported_player_pvp_flags',**flags(mask)}
            return value
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder))
            before={mask:store.ingest(capture('Bits',mask+2,mask)) for mask in range(8)}
            after={mask:store.ingest(capture('Bits',mask+20,mask)) for mask in range(8)}
            missing_old=store.ingest(capture('Bits',1,None));missing_new=store.ingest(capture('Bits',28,None))
            store.ingest(capture('Clear',1,7));store.ingest(capture('Clear',2,0))
            stable=store.ingest(capture('Stable',1,7))
            private=SnapshotStore(Path(foreign)).ingest(capture('PRIVATE_OTHER_OWNER',29,7))
            retail=fixture();retail_after=deepcopy(retail);retail_after['captured_at']='2025-01-02T00:00:00Z'
            retail_ids=[store.ingest(v) for v in [retail,retail_after]]
            original={p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,
                    'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args);self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign,str(reply));self.assertNotIn(private,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    async def search(filter,**extra):
                        return await call('list_snapshots',family='forever',player_pvp_flags=filter,**extra)
                    for mask,key in before.items():
                        observed=(await call('character_summary',snapshot_id=key))['player_pvp_flag_observation']
                        self.assertEqual(observed['reported_flags'],flags(mask))
                        for flag in BOUNDARY:self.assertFalse(observed[flag])
                        inspected=await call('inspect_snapshot',snapshot_id=key,section='player_pvp_flags')
                        self.assertEqual(inspected['data'],{'coverage':'reported_player_pvp_flags',**flags(mask)})
                    self.assertIsNone((await call('character_summary',snapshot_id=missing_old))['player_pvp_flag_observation'])
                    for predicates in product([None,False,True],repeat=3):
                        selected={field:v for field,v in zip(FIELDS,predicates) if v is not None}
                        if not selected:continue
                        found=await search(selected,character={'name':'Bits','realm':'Test'},captured_through='2025-01-09T23:59:59Z')
                        expected={key for mask,key in before.items() if all(flags(mask)[field]==v for field,v in selected.items())}
                        self.assertEqual({r['snapshot_id'] for r in found['snapshots']},expected)
                    latest=await search({'is_pvp':True},latest_per_character=True)
                    self.assertEqual([r['snapshot_id'] for r in latest['snapshots']],[stable])
                    self.assertEqual(latest['snapshots'][0]['player_pvp_flags_match']['reported_flags'],flags(7))
                    history=await search({'is_pvp':False},character={'name':'Bits','realm':'Test'})
                    for offset in [0,1,20,18446744073709551615]:
                        page=await search({'is_pvp':False},character={'name':'Bits','realm':'Test'},offset=offset,limit=1)
                        self.assertEqual(page['snapshots'],history['snapshots'][offset:offset+1])
                    for old,new in product(range(8),repeat=2):
                        result=(await call('compare_snapshots',before_id=before[old],after_id=after[new],
                            player_pvp_flag_changes=True))['player_pvp_flag_changes']
                        expected={field:{'before':flags(old)[field],'after':flags(new)[field]}
                            for field in FIELDS if flags(old)[field]!=flags(new)[field]}
                        self.assertEqual(result['flag_changes'],expected)
                        self.assertEqual(result['changed_flag_count'],len(expected))
                        for flag in BOUNDARY:self.assertFalse(result[flag])
                    for old,new in [(missing_old,before[0]),(before[0],missing_new),(missing_old,missing_new)]:
                        result=(await call('compare_snapshots',before_id=old,after_id=new,
                            player_pvp_flag_changes=True))['player_pvp_flag_changes']
                        self.assertIsNone(result['flag_changes']);self.assertIsNone(result['changed_flag_count'])
                    plain=await call('list_snapshots',family='forever')
                    self.assertEqual(await search(None),plain)
                    self.assertTrue(all('player_pvp_flags_match' not in row for row in plain['snapshots']))
                    compared=await call('compare_snapshots',before_id=before[0],after_id=after[7])
                    self.assertNotIn('player_pvp_flag_changes',compared)
                    self.assertEqual(await call('compare_snapshots',before_id=before[0],after_id=after[7],
                        player_pvp_flag_changes=False),compared)
                    for bad in [{},True,1,[],{'is_pvp':None},{'is_pvp':False,'extra':'PRIVATE'},
                            {'is_pvp':0},{'is_free_for_all':'false'}]:
                        await search(bad,error=True)
                    for bad in [None,1,'true',{}]:
                        await call('compare_snapshots',error=True,before_id=before[0],after_id=after[7],player_pvp_flag_changes=bad)
                    await call('list_snapshots',error=True,family='retail',player_pvp_flags={'is_pvp':True})
                    await call('compare_snapshots',error=True,before_id=retail_ids[0],after_id=retail_ids[1],player_pvp_flag_changes=True)
                    await call('inspect_snapshot',error=True,snapshot_id=private,section='player_pvp_flags')
            self.assertEqual(original,{p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')})
