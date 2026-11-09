"""Search captured quest objectives with same-row and unknown-value semantics."""
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


class QuestObjectiveFilterTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_same_row_unknowns_and_existing_filters(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        def objective(kind='monster', finished=False, required=None, fulfilled=None):
            row = {'kind': kind, 'finished': finished}
            if required is not None:
                row.update(required=required, fulfilled=fulfilled)
            return row
        rows = [None, {}, {'1': objective(required=5, fulfilled=4)},
            {'1': objective(finished=True, required=10, fulfilled=10)},
            {'1': objective('item'), '2': objective(finished=True, required=1, fulfilled=1)},
            {'1': objective()}, {'1': objective(required=1, fulfilled=3)},
            {'1': objective('item', required=2, fulfilled=1)},
            {'1': objective(required=3, fulfilled=0), '2': objective()},
            {'1': objective(required=1, fulfilled=0), '2': objective()}]
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            missing = store.ingest(base)
            saved = deepcopy(base)
            saved['sections']['quests'] = {str(i): {'title': f'Quest {i}', 'level': 60,
                'ready_for_turn_in': i == 3, **({'objectives': row} if row is not None else {})}
                for i, row in enumerate(rows, 1)}
            saved['sections']['quests']['3']['item_rewards'] = {'1': {'item_id': 118, 'quantity': 2}}
            saved['sections']['quests']['7']['item_rewards'] = {'1': {'item_id': 858, 'quantity': 3}}
            saved['sections']['quests']['3']['timer'] = {'remaining_seconds':60,'total_seconds':120}
            saved['sections']['bags']={'0':{'1':{'item_id':118,'count':2},'2':{'item_id':118,'count':1}}}
            saved['sections']['wallet']={'copper':7}
            key = store.ingest(saved)
            bank_stock=deepcopy(saved)
            bank_stock['sections']['character_bank']={'6':{'1':{'item_id':118,'count':1}}}
            bank_stock['sections']['account_bank']={'7':{'1':{'item_id':118,'count':1}}}
            bank_stock_key=store.ingest(bank_stock)
            changed=deepcopy(saved)
            changed['sections']['quests']['9']['objectives']['2']=objective(required=1,fulfilled=2)
            changed['captured_at']='2025-01-01T00:00:30Z'
            changed['sections']['quests']['3']['timer']['remaining_seconds']=30
            changed['sections']['bags']['0']['1']['count']=4
            changed['sections']['wallet']['copper']=2
            changed_key=store.ingest(changed)
            merchant=deepcopy(base)
            merchant['sections']['merchant_stock']={
                '1':{'item_id':118,'bundle_size':3,'price_copper':7,'available':-1,'extended_cost':False},
                '2':{'item_id':858,'bundle_size':1,'price_copper':2,'available':-1,'extended_cost':False}}
            merchant['sections']['stats'] = {'class':'HUNTER','race':'Orc','level':60,
                                             'client_version':'fixture','map_id':1414,'map_name':'Market'}
            merchant_key=store.ingest(merchant)
            bundled_merchant = deepcopy(merchant)
            bundled_merchant['sections']['merchant_stock']['2']['bundle_size'] = 3
            for slot, available, extended in [(3,1,False),(4,0,True),(5,-1,True)]:
                bundled_merchant['sections']['merchant_stock'][str(slot)] = {
                    **bundled_merchant['sections']['merchant_stock']['2'],
                    'available':available,'extended_cost':extended}
            bundled_merchant_key = store.ingest(bundled_merchant)
            partial_merchant = deepcopy(merchant)
            partial_merchant['sections']['merchant_stock'].pop('2')
            partial_merchant_key = store.ingest(partial_merchant)
            other_merchant = deepcopy(merchant)
            other_merchant['sections']['merchant_stock'] = {'1':merchant['sections']['merchant_stock']['2']}
            other_merchant_key = store.ingest(other_merchant)
            expensive_merchant = deepcopy(merchant)
            expensive_merchant['sections']['merchant_stock']['1']['price_copper'] = 9
            expensive_merchant_key = store.ingest(expensive_merchant)
            retail = store.ingest(fixture())
            tied = deepcopy(changed)
            tied['character']['name'] = 'Tied'
            tied['sections']['quests']['10']['timer'] = {'remaining_seconds':30,'total_seconds':120}
            tied_key = store.ingest(tied)
            original = {p.name: p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def timed(error=False, **options):
                        reply = await session.call_tool('list_snapshots', {'family':'forever', **options})
                        self.assertEqual(bool(reply.isError),error,reply)
                        if not error:
                            return json.loads(reply.content[0].text)['snapshots']

                    result = await timed(quest_max_remaining_seconds=30, character=changed['character'])
                    self.assertEqual([row['snapshot_id'] for row in result], [changed_key])
                    self.assertEqual(result[0]['quest_match']['quest_count'], 1)
                    self.assertIs(result[0]['quest_match']['timer_filter']['current_expiry_verified'], False)
                    shortest = result[0]['quest_match']['timer_filter']['shortest_matching_timer']
                    self.assertEqual(shortest, {'quest_id':3,'remaining_seconds':30,
                                               'projected_expiry_at':'2025-01-01T00:01:00Z'})
                    tied_result = (await timed(quest_max_remaining_seconds=30, character=tied['character']))[0]
                    self.assertEqual(tied_result['snapshot_id'], tied_key)
                    self.assertEqual(tied_result['quest_match']['quest_count'], 2)
                    self.assertEqual(tied_result['quest_match']['timer_filter']['shortest_matching_timer'], shortest)
                    specific = (await timed(quest_max_remaining_seconds=30, quest_id=10))[0]
                    self.assertEqual(specific['quest_match']['timer_filter']['shortest_matching_timer']['quest_id'], 10)
                    self.assertEqual(await timed(quest_max_remaining_seconds=29), [])
                    self.assertEqual(await timed(quest_max_remaining_seconds=60, quest_id=7), [])
                    self.assertEqual(await timed(quest_max_remaining_seconds=60, quest_readiness='not_ready', character=changed['character']), [])
                    projected = await timed(quest_projected_expiry_by='2025-01-01T00:01:00Z', quest_id=3)
                    self.assertEqual({row['snapshot_id'] for row in projected}, {key, bank_stock_key, changed_key, tied_key})
                    self.assertEqual(await timed(quest_projected_expiry_by='2025-01-01T00:00:59Z'), [])
                    await timed(error=True, family='retail', quest_max_remaining_seconds=60)
                    await timed(error=True, quest_max_remaining_seconds=-1)
                    await timed(error=True, quest_projected_expiry_by='invalid')
                    async def query(error=False, **options):
                        reply = await session.call_tool('inspect_snapshot', {'snapshot_id': key,
                            'section': 'quests', **options})
                        self.assertEqual(bool(reply.isError), error, reply)
                        return None if error else json.loads(reply.content[0].text)
                    options = {'quest_objective_kind': 'monster', 'quest_objective_finished': False,
                               'quest_objective_max_remaining': 1}
                    found = await query(**options)
                    self.assertEqual(set(found['data']), {'3', '7', '10'})
                    self.assertEqual(found['quest_objective_unknown_count'], 3)
                    self.assertEqual(found['quest_objective_filter'],
                                     {'kind': 'monster', 'finished': False, 'maximum_remaining': 1})
                    ready = await query(**options, quest_readiness='ready')
                    self.assertEqual(set(ready['data']), {'3'})
                    self.assertEqual(ready['quest_objective_unknown_count'], 0)
                    rewarded = await query(**options, quest_reward_item_id=118, quest_list_item_rewards=True)
                    self.assertEqual(set(rewarded['data']), {'3'})
                    self.assertEqual(rewarded['quest_list_item_rewards']['known_fixed_quantity_by_item_id'], {'118': 2})
                    self.assertEqual(set((await query(quest_objective_kind='monster',
                        quest_objective_finished=False))['data']), {'3', '6', '7', '9', '10'})
                    self.assertEqual(set((await query(quest_objective_max_remaining=0))['data']), {'4', '5', '7'})
                    bounded=await query(quest_objective_kind='monster',quest_objective_min_remaining=1,
                        quest_objective_max_remaining=1)
                    self.assertEqual(set(bounded['data']),{'3','10'})
                    self.assertEqual(bounded['quest_objective_filter']['minimum_remaining'],1)
                    self.assertEqual(set((await query(quest_objective_min_remaining=2))['data']),{'9'})
                    await query(error=True,quest_objective_min_remaining=2,quest_objective_max_remaining=1)
                    absent = await query(snapshot_id=missing, **options)
                    self.assertIsNone(absent['data'])
                    self.assertIsNone(absent['quest_objective_unknown_count'])
                    self.assertEqual((await query(quest_objective_kind='Monster'))['data'], {})
                    default = await query()
                    summary_reply=await session.call_tool('character_summary',{'snapshot_id':key})
                    self.assertFalse(summary_reply.isError,summary_reply)
                    timed=json.loads(summary_reply.content[0].text)['timed_quest_checklist']
                    self.assertEqual(timed['recorded_timer_count'],1)
                    self.assertEqual(timed['quest_count_without_recorded_timer'],9)
                    self.assertEqual(timed['entries'][0]['quest_id'],3)
                    self.assertEqual(timed['entries'][0]['projected_expiry_at'],'2025-01-01T00:01:00Z')
                    self.assertEqual(timed['ready_for_turn_in_at_capture_count'],1)
                    self.assertEqual(timed['first_ready_for_turn_in_at_capture'],timed['entries'][0])
                    self.assertFalse(timed['current_status_verified'])
                    checklist=await query(quest_list_timed_checklist=True,quest_reward_item_id=118)
                    self.assertEqual(checklist['timed_quest_checklist']['entries'],timed['entries'])
                    self.assertEqual(checklist['timed_quest_checklist']['quest_count_without_recorded_timer'],0)
                    untimed=await query(quest_list_timed_checklist=True,quest_reward_item_id=858)
                    self.assertEqual(untimed['timed_quest_checklist']['entries'],[])
                    self.assertEqual(untimed['timed_quest_checklist']['ready_for_turn_in_at_capture_count'],0)
                    self.assertIsNone(untimed['timed_quest_checklist']['first_ready_for_turn_in_at_capture'])
                    self.assertEqual(untimed['timed_quest_checklist']['quest_count_without_recorded_timer'],1)
                    self.assertIsNone((await query(snapshot_id=missing,
                        quest_list_timed_checklist=True))['timed_quest_checklist'])
                    self.assertNotIn('timed_quest_checklist',default)
                    exact = await query(quest_id=3)
                    requirements={'storage':'bags','items':[{'item_id':118,'quantity':5},
                                                             {'item_id':858,'quantity':1}]}
                    reviewed=(await query(quest_id=3,quest_inventory_requirements=requirements))[
                        'quest_inventory_review']
                    self.assertEqual(reviewed['items'],[
                        {'item_id':118,'required_quantity':5,'observed_quantity':3,'shortfall_vs_observed':2},
                        {'item_id':858,'required_quantity':1,'observed_quantity':0,'shortfall_vs_observed':1}])
                    self.assertIs(reviewed['quest_item_binding_verified'],False)
                    self.assertEqual(reviewed['short_item_count'],2)
                    self.assertIs(reviewed['all_requirements_covered_by_observed_quantities'],False)
                    bank_plan=(await query(quest_id=3,quest_inventory_requirements=requirements,
                        quest_inventory_snapshot_id=bank_stock_key,
                        quest_inventory_bank_withdrawals=True))['quest_inventory_review']['bank_withdrawal_plan']
                    self.assertEqual(bank_plan['rows'],[
                        {'item_id':118,'bag_shortfall':2,'suggested_withdrawals':{'character_bank':1,'account_bank':1},
                         'remaining_after_observed_withdrawals':0},
                        {'item_id':858,'bag_shortfall':1,'suggested_withdrawals':{'character_bank':0,'account_bank':0},
                         'remaining_after_observed_withdrawals':1}])
                    self.assertEqual([row['storage'] for row in bank_plan['suggested_locations']['locations']],
                                     ['character_bank','account_bank'])
                    page=(await query(quest_id=3,quest_inventory_requirements=requirements,
                        quest_inventory_snapshot_id=bank_stock_key,quest_inventory_bank_withdrawals=True,
                        quest_inventory_bank_withdrawal_offset=1))['quest_inventory_review']['bank_withdrawal_plan']
                    self.assertEqual(page['rows'],bank_plan['rows'])
                    self.assertEqual(page['suggested_locations']['locations'],bank_plan['suggested_locations']['locations'][1:])
                    self.assertEqual(page['suggested_locations']['total_locations'],2)
                    await query(error=True,quest_id=3,quest_inventory_requirements=requirements,
                                quest_inventory_bank_withdrawal_offset=0)
                    await query(error=True,quest_id=3,quest_inventory_bank_withdrawals=True,
                                quest_inventory_requirements={**requirements,'storage':'character_bank'})
                    covered=(await query(quest_id=3,quest_inventory_requirements={
                        'storage':'bags','items':[{'item_id':118,'quantity':3}]}))['quest_inventory_review']
                    self.assertEqual(covered['short_item_count'],0)
                    self.assertIs(covered['all_requirements_covered_by_observed_quantities'],True)
                    for name, expected in [('market',True),('elsewhere',False)]:
                        market = await session.call_tool('list_snapshots',{
                            'family':'forever','merchant_item_id':858,'player_map_name':name})
                        self.assertFalse(market.isError,market)
                        self.assertEqual(bool(json.loads(market.content[0].text)['snapshots']),expected)
                    quote=(await query(quest_id=3,quest_inventory_requirements=requirements,
                        quest_inventory_merchant_snapshot_id=merchant_key))['quest_inventory_merchant_quote']
                    self.assertEqual(quote['merchant_snapshot_id'],merchant_key)
                    self.assertEqual(quote['copper_only_bundle_mix_total'],9)
                    self.assertIs(quote['copper_only_bundle_mix_affordable'],False)
                    bank_quote=(await query(quest_id=3,quest_inventory_requirements=requirements,
                        quest_inventory_snapshot_id=bank_stock_key,quest_inventory_bank_withdrawals=True,
                        quest_inventory_merchant_snapshot_id=merchant_key))['quest_inventory_merchant_quote']
                    self.assertEqual(bank_quote['copper_only_bundle_mix_total'],9)
                    residual=bank_quote['after_observed_bank_withdrawals']
                    self.assertEqual(residual['copper_only_bundle_mix_total'],2)
                    self.assertIs(residual['copper_only_bundle_mix_affordable'],True)
                    self.assertEqual([row['item_id'] for row in residual['reagents']],[858])
                    restock = {'section':'bags','snapshot_id':bank_stock_key,
                               'bag_restock_requirements':requirements['items'],
                               'bag_restock_merchant_snapshot_id':merchant_key}
                    bag_quote = (await query(**restock))['bag_restock_merchant_quote']
                    self.assertEqual(bag_quote['merchant_snapshot_id'],merchant_key)
                    self.assertEqual(bag_quote['copper_only_bundle_mix_total'],2)
                    self.assertIs(bag_quote['copper_only_bundle_mix_affordable'],True)
                    self.assertIs(bag_quote['purchase_eligibility_verified'],False)
                    self.assertEqual([{k:v for k,v in row.items() if k != 'captured_offer_coverage'}
                                      for row in bag_quote['reagents']],residual['reagents'])
                    compare_ids = [partial_merchant_key,bundled_merchant_key,key,merchant_key]
                    comparison = (await query(**restock,bag_restock_compare_merchant_snapshot_ids=compare_ids))[
                        'bag_restock_merchant_comparison']
                    self.assertEqual([visit['snapshot_id'] for visit in comparison['visits']],compare_ids)
                    self.assertEqual([visit['complete_quote_copper'] for visit in comparison['visits']],
                                     [None,2,None,2])
                    self.assertEqual(comparison['visits'][0]['unquoted_item_ids'],[858])
                    self.assertEqual(comparison['visits'][0]['captured_player_map_id'],1414)
                    self.assertIsNone(comparison['visits'][2]['captured_player_map_id'])
                    self.assertFalse(comparison['visits'][2]['quote_available'])
                    self.assertEqual(comparison['lowest_complete_quote_snapshot_ids'],
                                     [bundled_merchant_key,merchant_key])
                    self.assertFalse(comparison['offers_combined_across_visits'])
                    for ids in [[],[merchant_key,merchant_key],[retail],[merchant_key]*5]:
                        await query(error=True,**restock,bag_restock_compare_merchant_snapshot_ids=ids)
                    split_args = {'section':'bags','snapshot_id':key,
                        'bag_restock_requirements':requirements['items'],
                        'bag_restock_split_merchant_plan':True}
                    split = await query(**split_args,bag_restock_compare_merchant_snapshot_ids=[
                        partial_merchant_key,other_merchant_key])
                    self.assertIsNone(split['bag_restock_merchant_comparison']['lowest_complete_quote_copper'])
                    plan = split['bag_restock_split_merchant_plan']
                    self.assertEqual(plan['complete_quote_copper'],9)
                    self.assertEqual([row['merchant_snapshot_id'] for row in plan['items']],
                                     [partial_merchant_key,other_merchant_key])
                    self.assertIs(plan['affordable_from_inventory_wallet'],False)
                    partial_plan = (await query(**split_args,bag_restock_compare_merchant_snapshot_ids=[
                        other_merchant_key]))['bag_restock_split_merchant_plan']
                    self.assertEqual(partial_plan['unquoted_item_ids'],[118])
                    self.assertEqual(partial_plan['quoted_subtotal_copper'],2)
                    self.assertIsNone(partial_plan['complete_quote_copper'])
                    for visit_cost,count,purchases,total in [(0,2,9,9),(2,1,11,13),(3,1,11,14)]:
                        travel = await query(**split_args,bag_restock_compare_merchant_snapshot_ids=[
                            partial_merchant_key,other_merchant_key,expensive_merchant_key],
                            bag_restock_cost_per_merchant_visit_copper=visit_cost)
                        adjusted = travel['bag_restock_travel_adjusted_merchant_plan']
                        self.assertEqual(adjusted['visit_count'],count)
                        self.assertEqual(adjusted['purchase_copper'],purchases)
                        self.assertEqual(adjusted['complete_quote_copper'],total)
                        self.assertEqual(adjusted['assumed_visit_cost_copper'],visit_cost*count)
                        checklist = adjusted['merchant_visit_checklist']
                        self.assertEqual(len(checklist),count)
                        self.assertEqual(sum(visit['purchase_copper'] for visit in checklist),purchases)
                        self.assertEqual([visit['assumed_visit_cost_copper'] for visit in checklist],[visit_cost]*count)
                        running_cost = 0
                        for visit in checklist:
                            running_cost += visit['purchase_copper'] + visit['assumed_visit_cost_copper']
                            self.assertEqual(visit['captured_player_map_id'],1414)
                            self.assertEqual(visit['merchant_captured_at'],merchant['captured_at'])
                            self.assertEqual(visit['cumulative_cost_copper'],running_cost)
                            self.assertEqual(visit['inventory_wallet_remaining_copper'],
                                             7-running_cost if running_cost <= 7 else None)
                            self.assertEqual(visit['inventory_wallet_shortfall_copper'],max(0,running_cost-7))
                            self.assertEqual(visit['items'],[
                                {'item_id':row['item_id'],'bundles_by_slot':row['bundle_plan']['bundles_by_slot'],
                                 'quoted_units':row['bundle_plan']['units'],
                                 'purchase_copper':row['bundle_plan']['required_copper']}
                                for row in adjusted['items'] if row['merchant_snapshot_id']==visit['merchant_snapshot_id']])
                        self.assertEqual(adjusted['inventory_wallet_copper'],7)
                        self.assertIsNone(adjusted['remaining_inventory_wallet_copper'])
                        self.assertEqual(adjusted['inventory_wallet_shortfall_copper'],total-7)
                        self.assertIs(adjusted['travel_cost_verified'],False)
                        self.assertEqual(travel['bag_restock_split_merchant_plan'],plan)
                    await query(error=True,**split_args,bag_restock_compare_merchant_snapshot_ids=[merchant_key],
                                bag_restock_cost_per_merchant_visit_copper=10**13)
                    visit_ids = [partial_merchant_key,other_merchant_key,expensive_merchant_key]
                    for costs, selected, total in [
                        ([10,0,0],[expensive_merchant_key,expensive_merchant_key],11),
                        ([0,0,10],[partial_merchant_key,other_merchant_key],9),
                        ([0,0,1],[partial_merchant_key,other_merchant_key],9),
                    ]:
                        adjusted = (await query(**split_args,
                            bag_restock_compare_merchant_snapshot_ids=visit_ids,
                            bag_restock_merchant_visit_costs_copper=costs))[
                                'bag_restock_travel_adjusted_merchant_plan']
                        self.assertEqual(adjusted['assumed_visit_costs_copper'],costs)
                        self.assertEqual(adjusted['complete_quote_copper'],total)
                        self.assertEqual([row['merchant_snapshot_id'] for row in adjusted['items']],selected)
                        self.assertNotIn('assumed_copper_per_visit',adjusted)
                    for costs in [[],[0,0],[0,0,10**13]]:
                        await query(error=True,**split_args,bag_restock_compare_merchant_snapshot_ids=visit_ids,
                                    bag_restock_merchant_visit_costs_copper=costs)
                    await query(error=True,**split_args,bag_restock_compare_merchant_snapshot_ids=visit_ids,
                                bag_restock_merchant_visit_costs_copper=[0,0,0],
                                bag_restock_cost_per_merchant_visit_copper=0)
                    for cap, total in [(0,None),(1,11),(2,9),(4,9)]:
                        capped = await query(**split_args,bag_restock_compare_merchant_snapshot_ids=visit_ids,
                                             bag_restock_max_merchant_visits=cap)
                        self.assertEqual(capped['bag_restock_split_merchant_plan'],plan)
                        adjusted = capped['bag_restock_travel_adjusted_merchant_plan']
                        if total is None:
                            self.assertIsNone(adjusted)
                        else:
                            self.assertEqual(adjusted['complete_quote_copper'],total)
                            self.assertEqual(adjusted['maximum_merchant_visits'],cap)
                            self.assertLessEqual(adjusted['visit_count'],cap)
                    covered_plan = await query(**{**split_args,
                        'bag_restock_requirements':[{'item_id':118,'quantity':1}]},
                        bag_restock_compare_merchant_snapshot_ids=visit_ids,bag_restock_max_merchant_visits=0)
                    self.assertEqual(covered_plan['bag_restock_travel_adjusted_merchant_plan']['visit_count'],0)
                    covered_budget = covered_plan['bag_restock_travel_adjusted_merchant_plan']
                    self.assertEqual(covered_budget['merchant_visit_checklist'],[])
                    self.assertEqual(covered_budget['remaining_inventory_wallet_copper'],7)
                    self.assertEqual(covered_budget['inventory_wallet_shortfall_copper'],0)
                    await query(error=True,**split_args,bag_restock_compare_merchant_snapshot_ids=visit_ids,
                                bag_restock_max_merchant_visits=5)
                    for reserve, travel_cost, spendable, reserve_gap, plan_gap, affordable in [
                        (5,0,2,0,0,True),(6,0,1,0,1,False),(8,0,0,1,2,False),
                        (5,1,2,0,1,False),
                    ]:
                        travel_options = ({'bag_restock_cost_per_merchant_visit_copper':travel_cost}
                                          if travel_cost else {})
                        budgeted = await query(**{**split_args,'snapshot_id':bank_stock_key},
                            bag_restock_compare_merchant_snapshot_ids=[merchant_key],
                            bag_restock_wallet_reserve_copper=reserve,**travel_options)
                        self.assertEqual(budgeted['bag_restock_travel_adjusted_merchant_plan']['after_wallet_reserve'],{
                            'wallet_reserve_copper':reserve,'spendable_copper':spendable,
                            'reserve_shortfall_copper':reserve_gap,'plan_shortfall_vs_spendable_copper':plan_gap,
                            'affordable':affordable})
                    await query(error=True,**split_args)
                    with_limits = {**restock,
                        'bag_restock_stack_limits':[{'item_id':118,'maximum_stack':2},
                                                    {'item_id':858,'maximum_stack':2}]}
                    bundled = await query(**{**with_limits,
                        'bag_restock_merchant_snapshot_id':bundled_merchant_key})
                    coverage = bundled['bag_restock_merchant_quote']['reagents'][0]['captured_offer_coverage']
                    self.assertEqual(coverage, {'sold_out_slots':1,'in_stock_extended_cost_slots':1,
                        'unlimited_copper_only_slots':1,'finite_copper_only_slots':1,
                        'finite_stock_quantity_verified':False})
                    self.assertEqual(bundled['bag_restock_new_stack_space']['new_stack_slots'],2)
                    estimated = bundled['bag_restock_quoted_bundle_stack_space']
                    self.assertEqual(estimated['new_stack_slots'],3)
                    self.assertEqual([row['incoming_quantity_with_quoted_bundles']
                                      for row in estimated['items']],[2,3])
                    unquoted = await query(**{**with_limits,'bag_restock_merchant_snapshot_id':key})
                    self.assertIsNone(unquoted['bag_restock_quoted_bundle_stack_space']['new_stack_slots'])
                    self.assertEqual(unquoted['bag_restock_new_stack_space']['new_stack_slots'],2)
                    reserve_field = 'copper_only_bundle_mix_after_reserve'
                    self.assertNotIn(reserve_field,bag_quote)
                    for reserve, spendable, gap, shortfall, affordable in [
                        (0,7,0,0,True),(5,2,0,0,True),(6,1,0,1,False),(8,0,1,2,False),
                    ]:
                        reserved = (await query(**restock,bag_restock_wallet_reserve_copper=reserve))[
                            'bag_restock_merchant_quote']
                        self.assertEqual(reserved.pop(reserve_field), {
                            'wallet_reserve_copper':reserve,'spendable_copper':spendable,
                            'reserve_shortfall_copper':gap,'known_shortfall_copper':shortfall,
                            'affordable':affordable})
                        self.assertEqual(reserved,bag_quote)
                    await query(error=True,**restock,bag_restock_wallet_reserve_copper=10**13)
                    await query(error=True,section='bags',bag_restock_requirements=requirements['items'],
                                bag_restock_wallet_reserve_copper=0)
                    self.assertIsNone((await query(**{**restock,'snapshot_id':missing}))[
                        'bag_restock_merchant_quote'])
                    self.assertIsNone((await query(**{**restock,'bag_restock_merchant_snapshot_id':key}))[
                        'bag_restock_merchant_quote'])
                    await query(error=True,**{**restock,'bag_restock_merchant_snapshot_id':retail})
                    await query(error=True,section='bags',bag_restock_merchant_snapshot_id=merchant_key)
                    newer=await query(quest_id=3,quest_inventory_requirements=requirements,
                        quest_inventory_snapshot_id=changed_key,quest_inventory_merchant_snapshot_id=merchant_key)
                    self.assertEqual(newer['data'],exact['data'])
                    self.assertEqual(newer['quest_inventory_review']['inventory_snapshot_id'],changed_key)
                    self.assertEqual(newer['quest_inventory_review']['items'][0]['shortfall_vs_observed'],0)
                    self.assertEqual(newer['quest_inventory_merchant_quote']['copper_only_bundle_mix_total'],2)
                    self.assertIs(newer['quest_inventory_merchant_quote']['copper_only_bundle_mix_affordable'],True)
                    await query(error=True,snapshot_id=changed_key,quest_id=3,
                                quest_inventory_requirements=requirements,quest_inventory_snapshot_id=key)
                    await query(error=True,quest_id=3,quest_inventory_snapshot_id=changed_key)
                    await query(error=True,quest_id=3,quest_inventory_merchant_snapshot_id=merchant_key)
                    await query(error=True,quest_id=3,quest_inventory_requirements=requirements,
                                quest_inventory_merchant_snapshot_id=retail)
                    bank=(await query(quest_id=3,quest_inventory_requirements={
                        **requirements,'storage':'character_bank'}))['quest_inventory_review']
                    self.assertIs(bank['storage_observed'],False)
                    self.assertIsNone(bank['short_item_count'])
                    self.assertIsNone(bank['all_requirements_covered_by_observed_quantities'])
                    self.assertIsNone(bank['items'][0]['shortfall_vs_observed'])
                    self.assertIsNone((await query(quest_id=999,quest_inventory_requirements=requirements))[
                        'quest_inventory_review'])
                    await query(error=True,quest_inventory_requirements=requirements)
                    await query(error=True,quest_id=3,quest_inventory_requirements={
                        **requirements,'items':[requirements['items'][0]]*2})
                    self.assertEqual(list(exact['objective_quantities_by_kind'].values()), [
                        {'fulfilled':4,'required':5,'remaining':1,
                         'counted_objectives':1,'unknown_objectives':0}])
                    self.assertEqual((await query(quest_id=2))['objective_quantities_by_kind'],{})
                    self.assertIsNone((await query(quest_id=1))['objective_quantities_by_kind'])
                    await query(error=True,quest_list_timed_checklist=True,quest_id=3)
                    await query(error=True,quest_list_timed_checklist=True,snapshot_id=retail)
                    deadline=await query(quest_projected_expiry_by='2025-01-01T00:01:00Z')
                    self.assertEqual(set(deadline['data']),{'3'})
                    self.assertEqual(deadline['quest_timer_unknown_count'],9)
                    start='2025-01-01T00:01:00Z'
                    window=await query(quest_projected_expiry_from=start,quest_projected_expiry_by=start)
                    self.assertEqual(window['data'],deadline['data'])
                    self.assertEqual(window['quest_projected_expiry_from'],start)
                    start_only=await query(quest_projected_expiry_from=start)
                    self.assertEqual(start_only['quest_projected_expiry_by_quest_id'],{'3':start})
                    self.assertEqual((await query(quest_projected_expiry_from='2025-01-01T00:01:01Z'))['data'],{})
                    await query(error=True,quest_projected_expiry_from=start,quest_projected_expiry_by='2025-01-01T00:00:59Z')
                    self.assertEqual(deadline['quest_projected_expiry_by_quest_id'],{'3':'2025-01-01T00:01:00Z'})
                    self.assertNotIn('quest_projected_expiry_by_quest_id',default)
                    self.assertEqual(deadline['data']['3'],saved['sections']['quests']['3'])
                    filtered=await query(quest_projected_expiry_by='2025-01-01T00:01:00Z',
                        quest_reward_item_id=858)
                    self.assertEqual(filtered['quest_projected_expiry_by_quest_id'],{})
                    absent_deadline=await query(snapshot_id=missing,quest_projected_expiry_by='2025-01-01T00:01:00Z')
                    self.assertIsNone(absent_deadline['quest_projected_expiry_by_quest_id'])
                    self.assertEqual((await query(quest_projected_expiry_by='2025-01-01T00:00:59Z'))['data'],{})
                    self.assertEqual((await query(quest_projected_expiry_by='2025-01-01T00:01:00Z',
                        quest_max_remaining_seconds=59))['data'],{})
                    await query(error=True,quest_projected_expiry_by='2025-01-01')
                    await query(error=True,quest_projected_expiry_by='2025-01-01T00:01:00Z',section='equipment')
                    self.assertEqual(default['data'], saved['sections']['quests'])
                    self.assertNotIn('quest_objective_filter', default)
                    for bad in [{'quest_objective_kind': ''}, {'quest_objective_kind': 'x' * 65},
                        {'quest_objective_kind': 1}, {'quest_objective_finished': 0},
                        {'quest_objective_max_remaining': True}, {'quest_objective_max_remaining': -1},
                        {'quest_objective_max_remaining': 2**32}, {'quest_id': 3, **options},
                        {'section': 'equipment', **options}, {'section': None, **options},
                        {'snapshot_id': retail, **options}, {'snapshot_id': '../PRIVATE', **options}]:
                        await query(error=True, **bad)
                    reply=await session.call_tool('compare_snapshots',{'before_id':key,'after_id':changed_key,'quest_id':9})
                    self.assertFalse(reply.isError,reply)
                    quantities=json.loads(reply.content[0].text)['quest_change']['objective_quantities_by_kind']
                    self.assertEqual(quantities['before']['monster'],{'fulfilled':0,'required':3,'remaining':3,
                        'counted_objectives':1,'unknown_objectives':1})
                    self.assertEqual(quantities['after']['monster'],{'fulfilled':2,'required':4,'remaining':3,
                        'counted_objectives':2,'unknown_objectives':0})
                    self.assertFalse(quantities['objective_identity_verified'])
                    reply=await session.call_tool('compare_snapshots',{'before_id':key,'after_id':changed_key,'quest_id':3})
                    self.assertFalse(reply.isError,reply)
                    timer=json.loads(reply.content[0].text)['quest_change']['timer_change']
                    self.assertEqual(timer['remaining_seconds_delta'],-30)
                    self.assertEqual(timer['before_projected_expiry_at'],'2025-01-01T00:01:00Z')
                    self.assertEqual(timer['after_projected_expiry_at'],timer['before_projected_expiry_at'])
                    self.assertFalse(timer['projected_expiry_changed'])
            self.assertEqual(original, {p.name: p.read_bytes() for p in Path(folder).glob('*.json')})
