"""Saved displayed-loot discovery never pools captures or credits owned stock."""
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


class LootDiscoveryTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_history_latest_kinds_ranges_and_owner_boundary(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        def sample(name,day,rows=None):
            value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['character']['name']=name;value['captured_at']=f'2025-01-{day:02}T00:00:00Z'
            value['sections']['bags']={'0':{'1':{'item_id':10,'count':2}}}
            if rows is not None:value['sections']['loot_window']={'coverage':'visible_loot_window_slots','slot_count':len(rows),
                'entries':{str(i):row for i,row in enumerate(rows,1)}}
            return value
        item=lambda count:{'status':'item','item_id':10,'quantity':count}
        currency={'status':'reported_currency','reported_currency_id':10,'reported_quantity':9}
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as other:
            store=SnapshotStore(Path(folder));other_store=SnapshotStore(Path(other));keys={}
            cases={'old_missing':sample('Alpha',1,[item(5)]),'new_missing':sample('Alpha',2),
                'old_short':sample('Bravo',1,[item(6)]),'new_short':sample('Bravo',2,[item(1)]),
                'currency':sample('Charlie',1,[currency]),'money':sample('Delta',1,[{'status':'money'}]),
                'empty':sample('Echo',1,[]),'zero':sample('Foxtrot',1,[item(0),{'status':'unreadable_or_non_item'}]),
                'mixed':sample('Gamma',1,[item(2),currency,item(3),{'status':'money'},{'status':'unreadable_or_non_item'}]),
                'split1':sample('Split1',1,[item(3)]),'split2':sample('Split2',1,[item(4)]),
                'maximum':sample('High',1,[item(4294967295)]*64)}
            cases['currency_maximum']=sample('CurrencyHigh',1,[{**currency,'reported_quantity':4294967295}]*64)
            cases['currency_zero']=sample('CurrencyZero',1,[{**currency,'reported_quantity':0}])
            cases['mixed']['sections']['professions']={'1':{'name':'Synthetic','rank':300,'max_rank':300,'skill_line':202}}
            for name,value in cases.items():keys[name]=store.ingest(value)
            other_key=other_store.ingest(sample('PRIVATE_OTHER_OWNER',1,[item(99)]))
            protected={path:path.read_bytes() for base in [Path(folder),Path(other)] for path in base.glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    reply=await session.call_tool('compare_snapshots',{'before_id':keys['old_short'],
                        'after_id':keys['new_short'],'loot_window_changes':True})
                    self.assertFalse(reply.isError,reply)
                    changes=json.loads(reply.content[0].text)['loot_window_changes']
                    self.assertEqual(changes['item_changes'],[
                        {'id':10,'before_observed_quantity':6,'after_observed_quantity':1}])
                    self.assertEqual(changes['reported_currency_changes'],[])
                    self.assertFalse(changes['source_identity_verified'])
                    reply=await session.call_tool('compare_snapshots',{'before_id':keys['old_missing'],
                        'after_id':keys['new_missing'],'loot_window_changes':True})
                    self.assertFalse(reply.isError,reply)
                    self.assertIsNone(json.loads(reply.content[0].text)['loot_window_changes'])
                    async def query(filter=None,error=False,**extra):
                        args={'family':'forever',**extra}
                        if filter is not None:args['loot_observation']=filter
                        reply=await session.call_tool('list_snapshots',args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE_OTHER_OWNER',str(reply));self.assertNotIn(other,str(reply))
                        self.assertNotIn(other_key,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    wanted={'kind':'item','item_id':10,'minimum_quantity':5}
                    history=await query(wanted)
                    self.assertEqual({row['snapshot_id'] for row in history['snapshots']},
                        {keys[name] for name in ['old_missing','old_short','mixed','maximum']})
                    latest=await query(wanted,latest_per_character=True)
                    self.assertEqual({row['snapshot_id'] for row in latest['snapshots']},{keys['mixed'],keys['maximum']})
                    composed=await query(wanted,profession_skill_line=202,profession_min_rank=300)
                    self.assertEqual((await query(wanted,inventory_item={'storage':'bags','item_id':10,
                        'minimum_quantity':3}))['snapshots'],[])
                    self.assertEqual([row['snapshot_id'] for row in composed['snapshots']],[keys['mixed']])
                    match=composed['snapshots'][0]['loot_observation_match']
                    self.assertEqual(match['observed_quantity'],5)
                    self.assertEqual(match['matching_slots'],[{'slot':1,'reported_quantity':2},{'slot':3,'reported_quantity':3}])
                    self.assertEqual(match['unreadable_slot_count'],1)
                    self.assertTrue(match['additional_matching_rows_unknown'])
                    for field in ['counts_toward_owned_inventory','counts_toward_owned_balances','ownership_verified',
                        'lootability_verified','currency_rewards_verified','money_amounts_verified','source_identity_verified','game_execution','live']:
                        self.assertFalse(match[field])
                    maximum=(await query(wanted,character={'name':'HIGH','realm':'test'}))['snapshots'][0]['loot_observation_match']
                    self.assertEqual(maximum['observed_quantity'],64*4294967295)
                    self.assertEqual([row['slot'] for row in maximum['matching_slots']],list(range(1,65)))
                    self.assertEqual((await query({**wanted,'minimum_quantity':18446744073709551615}))['snapshots'],[])
                    zero=await query({**wanted,'minimum_quantity':0,'maximum_quantity':0})
                    self.assertEqual([row['snapshot_id'] for row in zero['snapshots']],[keys['zero']])
                    currency_zero=await query({'kind':'reported_currency','reported_currency_id':10,
                        'minimum_quantity':0,'maximum_quantity':0})
                    self.assertEqual([row['snapshot_id'] for row in currency_zero['snapshots']],[keys['currency_zero']])
                    for filter,expected in [({'kind':'reported_currency','reported_currency_id':10},{keys['currency'],keys['mixed'],keys['currency_maximum']}),
                                            ({'kind':'money'},{keys['money'],keys['mixed']})]:
                        result=await query(filter)
                        self.assertEqual({row['snapshot_id'] for row in result['snapshots']},expected)
                        for row in result['snapshots']:
                            observation=row['loot_observation_match']
                            if filter['kind']=='money':
                                self.assertIsNone(observation['observed_quantity']);self.assertNotIn('maximum_quantity',observation)
                            else:
                                self.assertEqual(observation['observed_quantity'],
                                    64*4294967295 if row['snapshot_id']==keys['currency_maximum'] else 9)
                                self.assertEqual(observation['basis'],'raw_slot_info_before_container_conversion')
                    # Time bounds determine the latest capture before feature predicates.
                    dated=await query(wanted,latest_per_character=True,captured_through='2025-01-01T23:59:59Z')
                    self.assertEqual({row['snapshot_id'] for row in dated['snapshots']},
                        {keys[name] for name in ['old_missing','old_short','mixed','maximum']})
                    for offset in [0,1,4,18446744073709551615]:
                        page=await query(wanted,offset=offset,limit=1)
                        self.assertEqual(page['snapshots'],history['snapshots'][offset:offset+1])
                        self.assertEqual(page['has_more'],offset<3)
                    plain=await query()
                    self.assertTrue(all('loot_observation_match' not in row for row in plain['snapshots']))
                    for family in ['retail','classic']:await query(wanted,error=True,family=family)
                    invalid=[{}, {'kind':'unknown'}, {'kind':'money','minimum_quantity':0},
                        {'kind':'money','item_id':10},{'kind':'money','maximum_quantity':1},
                        {'kind':'item'}, {'kind':'item','item_id':0},{'kind':'item','item_id':True},
                        {'kind':'item','item_id':4294967296},{'kind':'item','item_id':10,'minimum_quantity':True},
                        {'kind':'item','item_id':10,'minimum_quantity':-1},{'kind':'item','item_id':10,'minimum_quantity':1.0},
                        {'kind':'item','item_id':10,'minimum_quantity':18446744073709551616},
                        {'kind':'item','item_id':10,'minimum_quantity':2,'maximum_quantity':1},
                        {'kind':'reported_currency','currency_id':10},
                        {'kind':'reported_currency','reported_currency_id':10,'maximum_quantity':0},
                        {**wanted,'private':other}]
                    for filter in invalid:await query(filter,error=True)
            for path,data in protected.items():self.assertEqual(path.read_bytes(),data)


if __name__=='__main__':unittest.main()
