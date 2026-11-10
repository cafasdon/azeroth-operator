"""Reward/budget grouping stays deduplicated, potential and owner-local."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_quest_turn_in_review import sample


def rewarded_sample():
    value=sample();quests=value['sections']['quests']
    for key,xp,cost,copper,honor in [('1',100,10,20,2),('2',200,20,5,3),('3',50,0,0,0),('7',25,0,1,1)]:
        quests[key].update(required_copper=cost,reward_amounts={'xp':xp,'copper':copper,'honor':honor},
            item_rewards={},item_choices={},currency_rewards={},currency_choices={},renown_rewards={})
    for key,quantity in [('1',2),('2',1)]:
        quests[key]['item_rewards']={'1':{'item_id':100,'quantity':quantity}}
    quests['1']['item_choices']={'1':{'item_id':900,'quantity':99}}
    quests['1']['currency_choices']={'1':{'currency_id':900,'quantity':99}}
    quests['7']['item_rewards']={'1':{'item_id':101,'quantity':1}}
    for key,quantity,points in [('1',3,4),('2',2,6)]:
        quests[key]['currency_rewards']={'1':{'currency_id':200,'quantity':quantity}}
        quests[key]['renown_rewards']={'1':{'faction_id':300,'quantity':points}}
    quests['5']['reward_amounts']={'xp':10000,'copper':10000}
    value['sections']['wallet']={'copper':5}
    value['sections']['experience']={'status':'progress','current_xp':50,'level_threshold_xp':200,'xp_disabled':False}
    value['sections']['currency_balances']={'coverage':'visible_currency_ids_only','entries':{'200':{
        'name':'PRIVATE_CURRENCY_NAME','quantity':18,'max_quantity':20,'max_weekly_quantity':0,
        'earned_this_week':0,'total_earned':18,'cap_uses_total_earned':False,'account_wide':False,'inactive':False}}}
    value['sections']['reputation']={'coverage':'visible_faction_rows_only','entries':{'300':{
        'name':'PRIVATE_FACTION_NAME','kind':'renown','account_wide':False,'reaction':4,'standing':0,
        'threshold':0,'next_threshold':0,'renown':{'level':5,'earned_in_level':12,'level_threshold':20,'capped':False}}}}
    return value


class QuestTurnInRewardTests(unittest.IsolatedAsyncioTestCase):
    async def test_complete_reward_views_deduplicate_and_preserve_unknowns(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));value=rewarded_sample();key=store.ingest(value)
            missing=deepcopy(value)
            for section in ['wallet','experience','currency_balances','reputation']:missing['sections'].pop(section)
            missing_key=store.ingest(missing)
            disabled=deepcopy(value);disabled['sections']['experience']['xp_disabled']=True
            disabled_key=store.ingest(disabled)
            failed=deepcopy(value);failed['sections']['quests']['2']['failed']=True;failed_key=store.ingest(failed)
            no_refs=deepcopy(value);del no_refs['sections']['quest_giver_references'];no_refs_key=store.ingest(no_refs)
            original={p:p.read_bytes() for p in Path(folder).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,
                    'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(snapshot=key,**page):
                        reply=await session.call_tool('inspect_snapshot',{'snapshot_id':snapshot,
                            'section':'quest_giver_references','quest_turn_in_review':page})
                        self.assertFalse(reply.isError,reply);self.assertNotIn('PRIVATE',str(reply))
                        return json.loads(reply.content[0].text)['data']
                    baseline=await call()
                    self.assertEqual(baseline,await call(include_reward_review=False))
                    self.assertNotIn('unique_quest_reward_review',baseline)
                    all_views=await call(include_reward_review=True)
                    union=all_views['unique_quest_reward_review']
                    self.assertEqual(union['quest_ids'],[1,2,3,4,7]);self.assertEqual(union['quest_count'],5)
                    self.assertEqual(union['xp']['known_reward_xp'],375)
                    self.assertEqual(union['xp']['unknown_reward_count'],1)
                    self.assertEqual(union['wallet']['known_required_copper'],30)
                    self.assertEqual(union['wallet']['known_reward_copper'],26)
                    self.assertEqual(union['wallet']['unknown_cost_count'],1)
                    self.assertEqual(union['honor'],{'known_reward_honor':6,'unknown_reward_count':1})
                    self.assertEqual(union['fixed_items']['known_fixed_quantity_by_item_id'],{'100':3,'101':1})
                    self.assertNotIn('900',union['currencies']['known_fixed_rewards'])
                    self.assertEqual(union['currencies']['known_fixed_rewards']['200']['proposed_quantity'],5)
                    self.assertEqual(union['currencies']['known_fixed_rewards']['200']['quantity_above_recorded_headroom'],3)
                    self.assertEqual(union['renown']['known_rewards_by_faction']['300']['reward_points'],10)
                    self.assertFalse(union['choices_applied']);self.assertFalse(union['spell_rewards_projected'])
                    self.assertFalse(union['reward_delivery_verified']);self.assertFalse(all_views['group_reward_views_are_additive'])
                    self.assertEqual(union['currency_choices']['quests_with_observed_currency_choices'],[1])
                    for offset in [0,1,4294967295]:
                        paged=await call(include_reward_review=True,ready_for_turn_in=True,limit=1,offset=offset)
                        reward=paged['unique_quest_reward_review']
                        self.assertEqual(reward['quest_ids'],[1,2,7]);self.assertEqual(reward['xp']['known_reward_xp'],325)
                    ready=await call(include_reward_review=True,ready_for_turn_in=True)
                    self.assertGreater(sum(g['reward_review']['xp']['known_reward_xp'] for g in ready['groups']),325)
                    exact=await call(include_reward_review=True,ready_for_turn_in=True,giver_kind='npc',giver_id=10)
                    reward=exact['groups'][0]['reward_review']
                    self.assertEqual(reward['quest_ids'],[1,2])
                    self.assertEqual(reward['wallet']['ready_turn_in_order']['minimum_starting_copper'],10)
                    self.assertEqual(reward['wallet']['ready_turn_in_order']['shortfall_copper'],5)
                    self.assertEqual(reward['xp']['xp_to_next_level'],150)
                    self.assertTrue(reward['xp']['covers_recorded_xp_threshold'])
                    unknown=(await call(snapshot=missing_key,include_reward_review=True,ready_for_turn_in=True,
                        giver_kind='npc',giver_id=10))['unique_quest_reward_review']
                    self.assertFalse(unknown['wallet']['wallet_observed'])
                    self.assertIsNone(unknown['wallet']['ready_turn_in_order']['shortfall_copper'])
                    self.assertIsNone(unknown['xp']['xp_to_next_level'])
                    self.assertIsNone(unknown['currencies']['known_fixed_rewards']['200']['quantity_above_recorded_headroom'])
                    blocked=(await call(snapshot=failed_key,include_reward_review=True,ready_for_turn_in=True,
                        giver_kind='npc',giver_id=10))['unique_quest_reward_review']
                    self.assertIsNone(blocked['wallet']['ready_turn_in_order'])
                    off=(await call(snapshot=disabled_key,include_reward_review=True))['unique_quest_reward_review']
                    self.assertIsNone(off['xp']['ready_turn_in_plan'])
                    self.assertIsNone(off['xp']['covers_recorded_xp_threshold'])
                    self.assertIsNone((await call(snapshot=no_refs_key,include_reward_review=True))['unique_quest_reward_review'])
                    empty=(await call(include_reward_review=True,quest_ids=[999]))['unique_quest_reward_review']
                    self.assertEqual(empty['quest_count'],0);self.assertEqual(empty['xp']['known_reward_xp'],0)
                    for bad in [1,'true',None]:
                        reply=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'quest_giver_references',
                            'quest_turn_in_review':{'include_reward_review':bad}})
                        self.assertTrue(reply.isError,reply)
            self.assertEqual(original,{p:p.read_bytes() for p in Path(folder).glob('*.json')})
