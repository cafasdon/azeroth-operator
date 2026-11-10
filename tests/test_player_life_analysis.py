"""Actual MCP summary/search/comparison of independent raw Boolean observations."""
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


FIELDS=['is_dead','is_ghost','is_dead_or_ghost','is_feigning_death']
FLAGS=['current_life_state_verified','character_continuity_verified','resurrection_eligibility_verified',
    'life_event_inferred','game_execution','live']


class PlayerLifeAnalysisTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_raw_summary_all_predicates_latest_and_comparison(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        def flags(mask):return {field:bool(mask&(1<<i)) for i,field in enumerate(FIELDS)}
        def capture(name,day,mask):
            value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['character']={'name':name,'realm':'Test'};value['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            if mask is not None:value['sections']['player_life_state']={'coverage':'reported_player_life_flags',**flags(mask)}
            return value
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder))
            bits={mask:store.ingest(capture('Bits',mask+2,mask)) for mask in range(16)}
            missing_old=store.ingest(capture('Bits',1,None));missing_new=store.ingest(capture('Bits',18,None))
            clear_old=store.ingest(capture('Clear',1,15));clear_new=store.ingest(capture('Clear',2,0))
            stable=store.ingest(capture('Stable',1,15))
            private=SnapshotStore(Path(foreign)).ingest(capture('PRIVATE_OTHER_OWNER',20,15))
            retail=fixture();retail_after=deepcopy(retail);retail_after['captured_at']='2025-01-02T00:00:00Z'
            retail_ids=[store.ingest(v) for v in [retail,retail_after]]
            original={p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args);self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign,str(reply));self.assertNotIn(private,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    async def search(filter,**extra):
                        return await call('list_snapshots',family='forever',player_life_state=filter,**extra)
                    for mask,key in bits.items():
                        observed=(await call('character_summary',snapshot_id=key))['player_life_state_observation']
                        self.assertEqual(observed['reported_flags'],flags(mask))
                        for flag in FLAGS:self.assertFalse(observed[flag])
                    self.assertIsNone((await call('character_summary',snapshot_id=missing_old))['player_life_state_observation'])
                    for predicates in product([None,False,True],repeat=4):
                        selected={field:v for field,v in zip(FIELDS,predicates) if v is not None}
                        if not selected:continue
                        found=await search(selected,character={'name':'Bits','realm':'Test'},captured_through='2025-01-17T23:59:59Z')
                        expected={key for mask,key in bits.items() if all(flags(mask)[field]==value for field,value in selected.items())}
                        self.assertEqual({r['snapshot_id'] for r in found['snapshots']},expected)
                    latest=await search({'is_dead':True},latest_per_character=True)
                    self.assertEqual([r['snapshot_id'] for r in latest['snapshots']],[stable])
                    self.assertEqual((await search({'is_dead':False},latest_per_character=True))['snapshots'][0]['snapshot_id'],clear_new)
                    match=latest['snapshots'][0]['player_life_state_match']
                    self.assertEqual(match['reported_flags'],flags(15))
                    for flag in FLAGS:self.assertFalse(match[flag])
                    history=await search({'is_dead':False})
                    for offset in [0,1,20,18446744073709551615]:
                        page=await search({'is_dead':False},offset=offset,limit=1)
                        self.assertEqual(page['snapshots'],history['snapshots'][offset:offset+1])
                    for old,new in [(0,15),(1,2),(8,9),(7,8),(3,10),(0,0),(15,15)]:
                        result=(await call('compare_snapshots',before_id=bits[old],after_id=bits[new],
                            player_life_state_changes=True))['player_life_state_changes']
                        expected={field:{'before':flags(old)[field],'after':flags(new)[field]}
                            for field in FIELDS if flags(old)[field]!=flags(new)[field]}
                        self.assertEqual(result['flag_changes'],expected);self.assertEqual(result['changed_flag_count'],len(expected))
                        self.assertEqual(result['before']['reported_flags'],flags(old));self.assertEqual(result['after']['reported_flags'],flags(new))
                        for flag in FLAGS:self.assertFalse(result[flag])
                    for old,new in [(missing_old,bits[0]),(bits[0],missing_new),(missing_old,missing_new)]:
                        result=(await call('compare_snapshots',before_id=old,after_id=new,player_life_state_changes=True))['player_life_state_changes']
                        self.assertIsNone(result['flag_changes']);self.assertIsNone(result['changed_flag_count'])
                        self.assertEqual(result['before'] is None,old==missing_old);self.assertEqual(result['after'] is None,new==missing_new)
                    plain=await call('list_snapshots',family='forever')
                    self.assertEqual(await search(None),plain)
                    self.assertTrue(all('player_life_state_match' not in r for r in plain['snapshots']))
                    compared=await call('compare_snapshots',before_id=bits[0],after_id=bits[15])
                    self.assertNotIn('player_life_state_changes',compared)
                    self.assertEqual(await call('compare_snapshots',before_id=bits[0],after_id=bits[15],player_life_state_changes=False),compared)
                    invalid=[{},True,1,[],{'is_dead':None},{'is_dead':False,'extra':'PRIVATE'}]
                    for field in FIELDS:
                        for value in [0,1,1.0,'PRIVATE',{},[]]:invalid.append({field:value})
                    for value in invalid:await call('list_snapshots',True,family='forever',player_life_state=value)
                    for value in [None,'PRIVATE',1,0,1.0,{},[]]:
                        await call('compare_snapshots',True,before_id=bits[0],after_id=bits[15],player_life_state_changes=value)
                    for family in ['retail','classic']:await call('list_snapshots',True,family=family,player_life_state={'is_dead':False})
                    for target in [private,clear_new]:
                        await call('compare_snapshots',True,before_id=bits[0],after_id=target,player_life_state_changes=True)
                    await call('character_summary',True,snapshot_id=private)
                    await call('compare_snapshots',True,before_id=retail_ids[0],after_id=retail_ids[1],player_life_state_changes=True)
                    await call('compare_snapshots',True,before_id=bits[15],after_id=bits[0],player_life_state_changes=True)
            for path,content in original.items():self.assertEqual(path.read_bytes(),content)


if __name__=='__main__':unittest.main()
