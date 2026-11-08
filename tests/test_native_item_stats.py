"""Raw native stat tokens compare only when observed on both linked items."""
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


class NativeItemStatsTests(unittest.IsolatedAsyncioTestCase):
    async def test_readers_and_equipment_review(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['equipment']={'16':{'item_id':720,'link':'item:720:0:0:0:0:0:0','native_stats':{
            'ITEM_MOD_STRENGTH_SHORT':-2,'ITEM_MOD_DAMAGE_PER_SECOND_SHORT':8.86,'RESISTANCE0_NAME':20}}}
        value['sections']['bags']={'0':{'1':{'item_id':721,'count':1,'link':'item:721:0:0:0:0:0:0','native_stats':{
            'ITEM_MOD_STRENGTH_SHORT':3,'ITEM_MOD_DAMAGE_PER_SECOND_SHORT':9.5,'EMPTY_SOCKET_RED':0}}}}
        legacy=deepcopy(value);legacy['sections']['equipment']['16'].pop('native_stats')
        empty=deepcopy(value);empty['sections']['equipment']['16']['native_stats']={}
        for section,container in [('character_bank','6'),('account_bank','7')]:
            value['sections'][section]={container:{'1':deepcopy(value['sections']['bags']['0']['1'])}}
        invalid=[]
        for stats in [None,[],{'PRIVATE':1},{'ITEM_MOD_A_SHORT':True},{'ITEM_MOD_A_SHORT':'PRIVATE'},
                {'ITEM_MOD_A_SHORT':None},{'ITEM_MOD_A_SHORT':2**32},{'ITEM_MOD_A_SHORT':-(2**32)},
                {'RESISTANCE7_NAME':1},{'EMPTY_SOCKET_RED1':1},{'ITEM_MOD_é_SHORT':1},
                {'ITEM_MOD_'+('A'*90)+'_SHORT':1},{f'ITEM_MOD_A_{i}_SHORT':1 for i in range(129)}]:
            bad=deepcopy(value);bad['sections']['bags']['0']['1']['native_stats']=stats;invalid.append(bad)
        bad=deepcopy(value);bad['sections']['bags']['0']['1'].pop('link');invalid.append(bad)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));keys=[store.ingest(v) for v in [value,legacy,empty]]
            bad_keys=[]
            for bad in invalid:
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,**args):
                        response=await session.call_tool(tool,args);self.assertFalse(response.isError,response.content)
                        return json.loads(response.content[0].text)
                    for section in ['equipment','bags','character_bank','account_bank']:
                        self.assertEqual((await call('inspect_snapshot',snapshot_id=keys[0],section=section))['data'],value['sections'][section])
                    selection={'target_slot':16,'expected_item_id':721,'source':{'kind':'bag','bag':0,'slot':1}}
                    review=await call('review_equipment_selection',snapshot_id=keys[0],selections=[selection])
                    comparison=review['changes'][0]['native_stat_comparison']
                    self.assertEqual(comparison['shared_tokens']['ITEM_MOD_STRENGTH_SHORT']['delta'],5)
                    self.assertAlmostEqual(comparison['shared_tokens']['ITEM_MOD_DAMAGE_PER_SECOND_SHORT']['delta'],0.64)
                    self.assertEqual(comparison['observed_only_before'],{'RESISTANCE0_NAME':20})
                    self.assertEqual(comparison['observed_only_after'],{'EMPTY_SOCKET_RED':0})
                    self.assertEqual(comparison['effect_on_performance'],'unverified')
                    review=await call('review_equipment_selection',snapshot_id=keys[1],selections=[selection])
                    self.assertIsNone(review['changes'][0]['native_stat_comparison'])
                    review=await call('review_equipment_selection',snapshot_id=keys[2],selections=[selection])
                    self.assertEqual(review['changes'][0]['native_stat_comparison']['shared_tokens'],{})
                    for bad in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'bags'})
                        self.assertTrue(response.isError);self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
