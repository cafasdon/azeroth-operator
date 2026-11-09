"""Exact durability describes only readable equipped slot observations."""
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


class EquipmentDurabilityTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_condition_and_strict_readers(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['equipment_durability']={'coverage':'readable_equipped_slots_only','entries':{
            '16':{'current':1,'maximum':3},'1':{'current':0,'maximum':100},'5':{'current':100,'maximum':100}}}
        empty=deepcopy(value);empty['sections']['equipment_durability']['entries']={}
        absent=deepcopy(value);absent['sections'].pop('equipment_durability')
        maximum=deepcopy(value);maximum['sections']['equipment_durability']['entries']={str(i):{'current':2**32-1,'maximum':2**32-1} for i in range(1,20)}
        fractional=deepcopy(value);fractional['sections']['equipment_durability']['entries']={'16':{'current':1,'maximum':3}}
        later=deepcopy(value);later['captured_at']='2025-01-02T00:00:00Z'
        later['sections']['equipment_durability']['entries']={
            '1':{'current':90,'maximum':120},'2':{'current':0,'maximum':5},
            '16':{'current':1,'maximum':3}}
        malformed=[]
        for row in [None,{}, {'current':1}, {'current':0,'maximum':0}, {'current':2,'maximum':1},
                    {'current':0,'maximum':1,'guid':'PRIVATE'}, {'current':True,'maximum':1},
                    {'current':-1,'maximum':1}, {'current':0.5,'maximum':1}, {'current':0,'maximum':2**32},
                    {'current':None,'maximum':1}, {'current':0,'maximum':None}]:
            bad=deepcopy(value);bad['sections']['equipment_durability']['entries']['1']=row;malformed.append(bad)
        for slot in ['0','20','01','-1','PRIVATE']:
            bad=deepcopy(value);bad['sections']['equipment_durability']['entries'][slot]={'current':0,'maximum':1};malformed.append(bad)
        for family in ['classic','retail']:
            bad=deepcopy(value);bad['client']['family']=family;malformed.append(bad)
        for field,invalid in [('coverage','all_equipment'),('entries',None)]:
            bad=deepcopy(value);bad['sections']['equipment_durability'][field]=invalid;malformed.append(bad)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));keys=[store.ingest(v) for v in [value,empty,absent,maximum,fractional,later]]
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
                    captured=await call('inspect_snapshot',snapshot_id=keys[0],section='equipment_durability')
                    self.assertEqual(captured['data'],value['sections']['equipment_durability'])
                    changes=(await call('compare_snapshots',before_id=keys[0],after_id=keys[5]))[
                        'equipment_durability_changes']
                    self.assertEqual(changes['changes'],[{'slot':1,'before':{'current':0,'maximum':100},
                        'after':{'current':90,'maximum':120},'current_delta':90,'maximum_delta':20}])
                    self.assertEqual(changes['observed_only_before_slots'],[5])
                    self.assertEqual(changes['observed_only_after_slots'],[2])
                    self.assertIs(changes['same_item_verified'],False)
                    self.assertIsNone((await call('compare_snapshots',before_id=keys[2],after_id=keys[5]))[
                        'equipment_durability_changes'])
                    condition=(await call('character_summary',snapshot_id=keys[0]))['equipment_condition']
                    self.assertEqual(condition['observed_slots'],3);self.assertEqual(condition['unobserved_slots'],16)
                    self.assertEqual(condition['damaged_slots'],[1,16]);self.assertEqual(condition['broken_slots'],[1])
                    self.assertEqual([r['slot'] for r in condition['slots']],[1,5,16])
                    self.assertAlmostEqual(condition['slots'][2]['percent'],100/3)
                    self.assertEqual(condition['minimum_observed_percent'],0)
                    condition=(await call('character_summary',snapshot_id=keys[1]))['equipment_condition']
                    self.assertIsNone(condition['minimum_observed_percent']);self.assertEqual(condition['unobserved_slots'],19)
                    self.assertEqual(condition['damaged_slots'],[])
                    self.assertIsNone((await call('character_summary',snapshot_id=keys[2]))['equipment_condition'])
                    condition=(await call('character_summary',snapshot_id=keys[3]))['equipment_condition']
                    self.assertEqual(condition['observed_slots'],19);self.assertEqual(condition['minimum_observed_percent'],100)
                    condition=(await call('character_summary',snapshot_id=keys[4]))['equipment_condition']
                    self.assertAlmostEqual(condition['minimum_observed_percent'],100/3)
                    self.assertEqual(condition['unobserved_slots'],18)
                    for bad in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'equipment_durability'})
                        self.assertTrue(response.isError);self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
