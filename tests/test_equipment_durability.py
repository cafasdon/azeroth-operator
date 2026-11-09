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
            budgets=[]
            for copper in [None,0,10,13]:
                saved=deepcopy(value)
                saved['sections'].pop('equipment_durability')
                saved['sections']['repair_quote']={'copper':10}
                if copper is not None:saved['sections']['wallet']={'copper':copper}
                budgets.append(store.ingest(saved))
            bad_keys=[]
            for bad in malformed:
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    inspect_tool=next(tool for tool in (await session.list_tools()).tools
                                      if tool.name=='inspect_snapshot')
                    self.assertEqual(inspect_tool.inputSchema['properties'][
                        'equipment_durability_max_percent']['maximum'],100)
                    async def call(tool,**args):
                        response=await session.call_tool(tool,args);self.assertFalse(response.isError,response.content)
                        return json.loads(response.content[0].text)
                    captured=await call('inspect_snapshot',snapshot_id=keys[0],section='equipment_durability')
                    for key, covers, remaining, shortfall in zip(
                        budgets,[None,False,True,True],[None,None,0,3],[None,10,0,0],strict=True):
                        status=(await call('character_summary',snapshot_id=key))['repair_status']
                        self.assertIs(status['wallet_covers_saved_repair_quote'],covers)
                        self.assertEqual(status['wallet_remaining_after_saved_repair_quote_copper'],remaining)
                        self.assertEqual(status['wallet_shortfall_copper'],shortfall)
                    for reserve, expected in [(0,{budgets[2]:0,budgets[3]:3}),(3,{budgets[3]:3}),(4,{})]:
                        listed=await call('list_snapshots',family='forever',repair_min_remaining_copper=reserve)
                        self.assertEqual({row['snapshot_id']:row['repair_quote_match']['wallet_remaining_copper']
                                          for row in listed['snapshots']},expected)
                    self.assertEqual(captured['data'],value['sections']['equipment_durability'])
                    for key, maximum, expected in [
                        (keys[0],0,[1]),(keys[0],33,[1]),(keys[0],34,[1,16]),
                        (keys[0],100,[1,5,16]),(keys[1],100,[]),(keys[2],100,None),
                        (keys[3],99,[]),
                    ]:
                        shortlist=await call('inspect_snapshot',snapshot_id=key,section='equipment_durability',
                                             equipment_durability_max_percent=maximum)
                        self.assertEqual(shortlist['equipment_durability_matching_slots'],expected)
                    self.assertNotIn('equipment_durability_matching_slots',captured)
                    listing=await call('list_snapshots',family='forever',
                                       equipment_durability_max_percent=33)
                    self.assertEqual({row['snapshot_id']:row['equipment_durability_matching_slots']
                                      for row in listing['snapshots']},
                                     {keys[0]:[1],keys[5]:[2]})
                    latest=await call('list_snapshots',family='forever',latest_per_character=True,
                                      equipment_durability_max_percent=33)
                    self.assertEqual([row['snapshot_id'] for row in latest['snapshots']],[keys[5]])
                    for section, maximum in [('equipment',50),('equipment_durability',101)]:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':keys[0],
                            'section':section,'equipment_durability_max_percent':maximum})
                        self.assertTrue(response.isError)
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
