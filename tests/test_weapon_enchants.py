"""Weapon enchant observations keep timer/charge values separate from eligibility."""
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


class WeaponEnchantTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_partial_hands_and_strict_readers(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        rows={str(i):{'has_enchant':False,'enchant_type':'none'} for i in range(1,11)}
        rows['1']={'has_enchant':True,'enchant_type':'permanent','enchant_id':42,'charges':0,'remaining_milliseconds':0}
        rows['2']={'has_enchant':True,'enchant_type':'imbue'}
        rows['10']={'has_enchant':True,'enchant_type':'temporary','remaining_milliseconds':2**32-1}
        value['sections']['weapon_enchants']={'coverage':'public_weapon_enchant_lists','hands':{
            'main_hand':{'entries':rows},'off_hand':{'entries':{}}}}
        absent=deepcopy(value);absent['sections'].pop('weapon_enchants')
        unknown=deepcopy(value);unknown['sections']['weapon_enchants']['hands']={}
        maximum=deepcopy(value);maximum['sections']['weapon_enchants']['hands']={hand:{'entries':{
            str(i):{'has_enchant':True,'enchant_type':'temporary','enchant_id':2**32-1,'charges':2**32-1,
                    'remaining_milliseconds':2**32-1} for i in range(1,33)}} for hand in ['main_hand','off_hand','ranged']}
        malformed=[]
        for change in [lambda v:v['hands'].update({'PRIVATE':{'entries':{}}}),
                lambda v:v.update(coverage='all_effects'),lambda v:v['hands']['main_hand'].update(extra=1),
                lambda v:v['hands']['main_hand'].update(entries=[]),
                lambda v:v['hands']['main_hand']['entries'].pop('3'),
                lambda v:v['hands']['main_hand'].update(entries={str(i):{'has_enchant':False,'enchant_type':'none'} for i in range(1,34)})]:
            bad=deepcopy(value);change(bad['sections']['weapon_enchants']);malformed.append(bad)
        for row in [{}, {'has_enchant':0,'enchant_type':'none'}, {'has_enchant':True,'enchant_type':'PRIVATE'},
                {'has_enchant':False,'enchant_type':'none','charges':0},
                {'has_enchant':True,'enchant_type':'temporary','enchant_id':0},
                {'has_enchant':True,'enchant_type':'temporary','enchant_id':None},
                {'has_enchant':True,'enchant_type':'temporary','charges':True},
                {'has_enchant':True,'enchant_type':'temporary','charges':-1},
                {'has_enchant':True,'enchant_type':'temporary','remaining_milliseconds':1.5},
                {'has_enchant':True,'enchant_type':'temporary','remaining_milliseconds':2**32},
                {'has_enchant':True,'enchant_type':'temporary','guid':'PRIVATE'}]:
            bad=deepcopy(value);bad['sections']['weapon_enchants']['hands']['main_hand']['entries']['1']=row;malformed.append(bad)
        for family in ['retail','classic']:
            bad=deepcopy(value);bad['client']['family']=family;malformed.append(bad)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));keys=[store.ingest(v) for v in [value,absent,unknown,maximum]]
            bad_keys=[]
            for bad in malformed:
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
                    data=await call('inspect_snapshot',snapshot_id=keys[0],section='weapon_enchants')
                    self.assertEqual(data['data'],value['sections']['weapon_enchants'])
                    summary=(await call('character_summary',snapshot_id=keys[0]))['weapon_enchant_observations']
                    self.assertEqual(summary['unobserved_hands'],['ranged'])
                    active=summary['hands']['main_hand']['active_enchants']
                    self.assertEqual([r['row'] for r in active],[1,2,10])
                    self.assertEqual(active[0]['charges'],0);self.assertEqual(active[0]['remaining_milliseconds'],0)
                    self.assertIsNone(active[1]['charges']);self.assertEqual(active[1]['enchant_type'],'imbue')
                    self.assertEqual(summary['hands']['off_hand'],{'observed_rows':0,'active_enchants':[]})
                    self.assertEqual(summary['effect_on_performance'],'unverified')
                    self.assertIsNone((await call('character_summary',snapshot_id=keys[1]))['weapon_enchant_observations'])
                    summary=(await call('character_summary',snapshot_id=keys[2]))['weapon_enchant_observations']
                    self.assertEqual(summary['unobserved_hands'],['main_hand','off_hand','ranged'])
                    summary=(await call('character_summary',snapshot_id=keys[3]))['weapon_enchant_observations']
                    self.assertEqual(summary['unobserved_hands'],[])
                    self.assertEqual(sum(len(hand['active_enchants']) for hand in summary['hands'].values()),96)
                    for bad in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'weapon_enchants'})
                        self.assertTrue(response.isError);self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
