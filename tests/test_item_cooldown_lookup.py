"""Item lookup keeps cooldown evidence with its original bag, character and time."""
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


class ItemCooldownLookupTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_exact_location_paging_and_multiple_captures(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); store=SnapshotStore(base); value=fixture()
            value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['equipment']={'11':{'item_id':100}}
            value['sections']['action_bar_observation']={
                'coverage':'loaded_builtin_buttons_only','observed_buttons':4,'missing_buttons':0,'slots':{
                    '1':{'empty':False,'kind':'item','id':100},'2':{'empty':False,'kind':'spell','id':101},
                    '3':{'empty':False,'kind':'item','id':100},'4':{'empty':False,'kind':'item','id':100}},
                'button_hotkeys':{
                    '1':{'button':'ActionButton1','slot':1,'key1':'1','key2':'SHIFT-1'},
                    '2':{'button':'ActionButton2','slot':2,'key1':'2'},
                    '3':{'button':'ActionButton3','slot':3}}}
            value['sections']['bags']={'0':{str(slot):{'item_id':100,'count':slot} for slot in [1,2,3]},
                '1':{'1':{'item_id':101,'count':99}}, '2':{}}
            value['sections']['character_bank']={'6':{'1':{
                'item_id':100,'count':20,'link':'item:100:0:0:0:0:0:0'}}}
            value['sections']['bag_item_cooldowns']={'coverage':'captured_carried_items_only','captured_item_locations':4,
                'entries':{'0':{
                    '1':{'item_id':100,'status':'reported','enabled':True,'remaining_seconds':0},
                    '2':{'item_id':100,'status':'reported','enabled':False,'remaining_seconds':50},
                    '3':{'item_id':100,'status':'unavailable'}},
                    '1':{'1':{'item_id':101,'status':'reported','enabled':False,'remaining_seconds':100}},
                    '2':{}}}
            key=store.ingest(value)
            legacy=deepcopy(value); del legacy['sections']['bag_item_cooldowns']; legacy_key=store.ingest(legacy)
            alt=deepcopy(value); alt['character']['name']='Another'; alt['captured_at']='2025-01-02T00:00:00Z'
            alt['sections']['bag_item_cooldowns']['entries']['0']['1']['remaining_seconds']=7
            alt['sections']['account_bank']={}
            alt_key=store.ingest(alt)
            target_capture=deepcopy(alt)
            timers=target_capture['sections']['bag_item_cooldowns']['entries']['0']
            timers['2'].update(enabled=True,remaining_seconds=7)
            timers['3'].update(status='reported',enabled=True,remaining_seconds=2)
            target_key=store.ingest(target_capture)
            changed=deepcopy(legacy)
            changed['captured_at']='2025-01-02T00:00:00Z'
            changed['sections']['bags']['0']['1']['count']=4
            del changed['sections']['bags']['2']
            changed['sections']['bags']['3']={}
            del changed['sections']['character_bank']
            changed['sections']['account_bank']={}
            changed_key=store.ingest(changed)
            retail_key=store.ingest(fixture())
            no_bags=deepcopy(legacy);del no_bags['sections']['bags']
            del no_bags['sections']['action_bar_observation']['button_hotkeys']
            no_bags_key=store.ingest(no_bags)
            location_captures = []
            for item, status in [
                ({'item_id':100,'count':1,'link':'item:100:0:0:0:0:0:0'}, 'same_item_id_and_link'),
                ({'item_id':100,'count':1,'link':'item:100:1:0:0:0:0:0'}, 'same_item_id_different_link'),
                ({'item_id':100,'count':1}, 'same_item_id_link_unknown'),
                ({'item_id':100,'count':1,'link':''}, 'same_item_id_link_unknown'),
                ({'item_id':101,'count':1}, 'different_item_id'),
                (None, 'not_observed'),
            ]:
                later = deepcopy(value)
                later['captured_at'] = '2025-01-03T00:00:00Z'
                later['sections']['character_bank']['6'] = {'1':item} if item else {}
                later['sections']['bags']['0']['1']['link'] = 'item:100:0:0:0:0:0:0'
                later['sections']['bags']['0']['2']['link'] = 'item:100:1:0:0:0:0:0'
                location_captures.append((store.ingest(later), status, item))
            protected={p:p.read_bytes() for p in base.glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    equipment_request = {'snapshot_id':key, 'selections':[{
                        'target_slot':11, 'expected_item_id':100,
                        'source':{'kind':'character_bank','container':6,'slot':1}}]}
                    for later_key, status, item in location_captures:
                        reply = await session.call_tool('review_equipment_selection', {
                            **equipment_request, 'compare_source_locations_snapshot_id':later_key})
                        self.assertFalse(reply.isError, reply)
                        comparison = json.loads(reply.content[0].text)['source_location_comparison']
                        self.assertEqual(comparison['snapshot_id'], later_key)
                        self.assertFalse(comparison['current_locations_verified'])
                        row, = comparison['selections']
                        candidates = row.pop('same_base_item_candidates', None)
                        self.assertEqual(row, {
                            **equipment_request['selections'][0], 'status':status,
                            'observed_item_id':item['item_id'] if item else None})
                        if status in ('different_item_id', 'not_observed', 'same_item_id_different_link'):
                            changed_link = status == 'same_item_id_different_link'
                            self.assertEqual(candidates['matched_locations'], 5 if changed_link else 4)
                            self.assertFalse(candidates['has_more'])
                            self.assertEqual(candidates['quantity_by_storage'],
                                             {'equipment':1,'bags':6,'character_bank':int(changed_link),
                                              'account_bank':None})
                            expected_locations = [
                                {'storage':'equipment','container':None,'slot':'11','count':1,
                                 'saved_link_matches_original':None},
                                *[{'storage':'bags','container':'0','slot':str(slot),'count':slot,
                                   'saved_link_matches_original':link_matches}
                                  for slot, link_matches in [(1,True),(2,False),(3,None)]],
                            ]
                            if changed_link:
                                expected_locations.append({
                                    'storage':'character_bank','container':'6','slot':'1','count':1,
                                    'saved_link_matches_original':False,
                                })
                            self.assertEqual(candidates['locations'], expected_locations)
                        else:
                            self.assertIsNone(candidates)
                    for invalid_key in [key, alt_key, retail_key]:
                        reply = await session.call_tool('review_equipment_selection', {
                            **equipment_request, 'compare_source_locations_snapshot_id':invalid_key})
                        self.assertTrue(reply.isError, reply)

                    audit_request = {
                        'section': 'action_bar_observation',
                        'action_bar_item_stock_audit': True,
                    }
                    for snapshot, target, expected in [
                        (key, 8, 2), (key, 4, 0), (no_bags_key, 8, None),
                    ]:
                        reply = await session.call_tool('inspect_snapshot', {
                            **audit_request, 'snapshot_id': snapshot,
                            'action_bar_item_stock_target_quantity': target,
                        })
                        self.assertFalse(reply.isError, reply)
                        audit = json.loads(reply.content[0].text)['action_bar_item_stock_audit']
                        self.assertEqual(list(audit), ['100'])
                        self.assertEqual(audit['100']['bag_target_quantity'], target)
                        self.assertEqual(audit['100']['bag_quantity_shortfall'], expected)
                        self.assertEqual(audit['100']['observed_hotkeys_by_slot'],
                                         None if snapshot == no_bags_key else
                                         {'1': ['1', 'SHIFT-1'], '3': [], '4': None})
                    reply = await session.call_tool('inspect_snapshot', {
                        **audit_request, 'snapshot_id': key,
                    })
                    self.assertFalse(reply.isError, reply)
                    self.assertEqual(json.loads(reply.content[0].text)['action_bar_item_stock_audit'],
                                     {'100': {'slots': [1, 3, 4], 'equipment_quantity': 1, 'bag_quantity': 6}})
                    for audit_enabled, target in [(False, 8), (True, 0)]:
                        reply = await session.call_tool('inspect_snapshot', {
                            **audit_request, 'snapshot_id': key,
                            'action_bar_item_stock_audit': audit_enabled,
                            'action_bar_item_stock_target_quantity': target,
                        })
                        self.assertTrue(reply.isError, reply)

                    async def find(snapshot=key,**extra):
                        reply=await session.call_tool('find_snapshot_item',{'snapshot_id':snapshot,'item_id':100,**extra})
                        self.assertFalse(reply.isError,reply)
                        return json.loads(reply.content[0].text)
                    result=await find()
                    self.assertEqual(result['matched_locations'],5)
                    self.assertNotIn('quantity_by_container',result)
                    self.assertEqual(result['quantity_by_storage'],{'equipment':1,'bags':6,'character_bank':20,'account_bank':None})
                    counts=result['bag_cooldown_observation']
                    self.assertEqual(counts,{'basis':'same_snapshot_exact_bag_slot_item',
                        'scope':'matching_bag_locations_before_pagination','reported_item_locations':2,
                        'unavailable_item_locations':1,'disabled_item_locations':1,'positive_timer_item_locations':1,
                        'current_usability_verified':False,'game_execution':False})
                    bag_rows=[row for row in result['locations'] if row['storage']=='bags']
                    self.assertEqual([row['captured_cooldown'] for row in bag_rows],
                        list(value['sections']['bag_item_cooldowns']['entries']['0'].values()))
                    self.assertTrue(all('captured_cooldown' not in row for row in result['locations'] if row['storage']!='bags'))
                    for offset in range(7):
                        page=await find(offset=offset,limit=1)
                        self.assertEqual(page['locations'],result['locations'][offset:offset+1])
                        self.assertEqual(page['bag_cooldown_observation'],counts)
                        self.assertEqual(page['matched_locations'],5)
                        self.assertEqual(page['has_more'],offset<4)
                    missing=await find(item_id=999)
                    self.assertEqual(missing['locations'],[])
                    self.assertEqual(missing['bag_cooldown_observation']['reported_item_locations'],0)
                    legacy_result=await find(legacy_key)
                    self.assertNotIn('bag_cooldown_observation',legacy_result)
                    expected=deepcopy(result); expected['snapshot_id']=legacy_key
                    del expected['bag_cooldown_observation']
                    for row in expected['locations']: row.pop('captured_cooldown',None)
                    self.assertEqual(legacy_result,expected)
                    container_quantities={'bags':{'0':6,'1':0,'2':0},'character_bank':{'6':20},'account_bank':None}
                    beyond=await find(include_container_quantities=True,offset=99,limit=1)
                    self.assertEqual(beyond['locations'],[])
                    self.assertEqual(beyond['quantity_by_container'],container_quantities)
                    combined=await find(additional_snapshot_ids=[alt_key,legacy_key],include_container_quantities=True)
                    other,old=combined['additional_snapshot_results']
                    self.assertEqual(combined['quantity_by_container'],container_quantities)
                    self.assertEqual(old['quantity_by_container'],container_quantities)
                    self.assertEqual(other['quantity_by_container'],{**container_quantities,'account_bank':{}})
                    self.assertEqual(other['captured_at'],alt['captured_at'])
                    self.assertEqual(next(row for row in other['locations'] if row['storage']=='bags')['captured_cooldown']['remaining_seconds'],7)
                    self.assertEqual(next(row for row in combined['locations'] if row['storage']=='bags')['captured_cooldown']['remaining_seconds'],0)
                    self.assertNotIn('bag_cooldown_observation',old)
                    self.assertNotIn('bag_cooldown_reference_at',combined)
                    reference='2025-01-02T00:00:06Z'
                    projected=await find(additional_snapshot_ids=[alt_key,legacy_key],bag_cooldown_reference_at=reference)
                    for capture_id,item,expected in [(key,100,(1,0,5)),(alt_key,100,(0,1,5)),
                                                      (legacy_key,100,(0,0,6)),(key,101,None)]:
                        reply=await session.call_tool('inspect_snapshot',{'snapshot_id':capture_id,
                            'section':'action_bar_observation','action_bar_item_id':item,
                            'action_bar_item_stock':True,'bag_cooldown_reference_at':reference})
                        self.assertFalse(reply.isError,reply)
                        stock=json.loads(reply.content[0].text)['action_bar_item_stock']
                        if expected is None:
                            self.assertIsNone(stock)
                        else:
                            self.assertEqual(stock['bag_quantity'],6)
                            self.assertEqual(stock['bag_cooldown_reference_at'],reference)
                            self.assertEqual(stock['bag_cooldown_quantity_projection'],{
                                'quantity_with_projected_end_at_or_before_reference':expected[0],
                                'quantity_with_projected_end_after_reference':expected[1],
                                'quantity_with_unknown_projected_end':expected[2],
                                'current_usability_verified':False})
                    for capture, timestamp, comparison, totals in [
                        (projected,'2025-01-01T00:00:00Z',True,(1,0,5)),
                        (projected['additional_snapshot_results'][0],'2025-01-02T00:00:07Z',False,(0,1,5)),
                        (projected['additional_snapshot_results'][1],None,None,(0,0,6)),
                    ]:
                        self.assertEqual(capture['bag_cooldown_reference_at'],reference)
                        self.assertEqual(capture['bag_cooldown_quantity_projection'],{
                            'quantity_with_projected_end_at_or_before_reference':totals[0],
                            'quantity_with_projected_end_after_reference':totals[1],
                            'quantity_with_unknown_projected_end':totals[2],
                            'current_usability_verified':False})
                        carried=[row for row in capture['locations'] if row['storage']=='bags']
                        self.assertEqual([row['projected_cooldown_end_at'] for row in carried],[timestamp,None,None])
                        self.assertEqual([row['projected_end_at_or_before_reference'] for row in carried],[comparison,None,None])
                        self.assertTrue(all('projected_cooldown_end_at' not in row for row in capture['locations']
                                            if row['storage']!='bags'))
                    page=await find(offset=1,limit=1,bag_cooldown_reference_at=value['captured_at'])
                    self.assertEqual(len(page['locations']),1)
                    self.assertTrue(page['locations'][0]['projected_end_at_or_before_reference'])
                    self.assertEqual(page['bag_cooldown_observation'],counts)
                    self.assertEqual(page['bag_cooldown_quantity_projection'],projected['bag_cooldown_quantity_projection'])
                    beyond=await find(offset=99,limit=1,bag_cooldown_reference_at=reference)
                    self.assertEqual(beyond['locations'],[])
                    self.assertEqual(beyond['bag_cooldown_quantity_projection'],projected['bag_cooldown_quantity_projection'])
                    absent_bags=await find(no_bags_key,bag_cooldown_reference_at=reference)
                    self.assertIsNone(absent_bags['bag_cooldown_quantity_projection'])
                    zero_bags=await find(item_id=999,bag_cooldown_reference_at=reference)
                    self.assertEqual(zero_bags['bag_cooldown_quantity_projection']['quantity_with_unknown_projected_end'],0)
                    for snapshot,quantity,covered,shortfall,earliest in [
                            (target_key,3,True,0,'2025-01-02T00:00:02Z'),
                            (target_key,4,False,1,'2025-01-02T00:00:07Z'),
                            (alt_key,1,None,1,'2025-01-02T00:00:07Z'),
                            (alt_key,6,False,6,None)]:
                        target=(await find(snapshot,offset=99,limit=1,bag_cooldown_reference_at=reference,
                                           bag_cooldown_target_quantity=quantity))['bag_cooldown_target_projection']
                        self.assertIs(target['projected_end_by_reference_covers_target'],covered)
                        self.assertEqual(target['shortfall_at_reference_from_known_timers'],shortfall)
                        self.assertEqual(target['earliest_known_projected_end_covering_target'],earliest)
                        reply=await session.call_tool('inspect_snapshot',{'snapshot_id':snapshot,
                            'section':'action_bar_observation','action_bar_item_id':100,'action_bar_item_stock':True,
                            'bag_cooldown_reference_at':reference,'bag_cooldown_target_quantity':quantity})
                        self.assertFalse(reply.isError,reply)
                        stock=json.loads(reply.content[0].text)['action_bar_item_stock']
                        self.assertEqual(stock['bag_cooldown_target_projection'],target)
                    for options in [{'action_bar_item_stock':True},{'bag_cooldown_reference_at':reference},
                                    {'action_bar_item_stock':True,'bag_cooldown_reference_at':reference,'bag_cooldown_target_quantity':0}]:
                        reply=await session.call_tool('inspect_snapshot',{'snapshot_id':key,
                            'section':'action_bar_observation','action_bar_item_id':100,'bag_cooldown_target_quantity':1,**options})
                        self.assertTrue(reply.isError,reply)
                    for options in [{'bag_cooldown_target_quantity':1},
                                    {'bag_cooldown_target_quantity':0,'bag_cooldown_reference_at':reference}]:
                        reply=await session.call_tool('find_snapshot_item',{'snapshot_id':key,'item_id':100,**options})
                        self.assertTrue(reply.isError,reply)
                    rejected=await session.call_tool('find_snapshot_item',{
                        'snapshot_id':retail_key,'item_id':100,'bag_cooldown_reference_at':reference})
                    self.assertTrue(rejected.isError,rejected)
                    comparison={'before_id':key,'after_id':changed_key,'item_id':100}
                    for options in [{}, {'item_container_changes':True}]:
                        reply=await session.call_tool('compare_snapshots',{**comparison,**options})
                        self.assertFalse(reply.isError,reply)
                        item=json.loads(reply.content[0].text)['item_quantity_change']
                        self.assertEqual('by_container' in item,bool(options))
                    self.assertEqual(item['by_container'],{
                        'bags':{'before_observed':True,'after_observed':True,'containers':{
                            '0':{'before':6,'after':9,'delta':3},
                            '1':{'before':0,'after':0,'delta':0},
                            '2':{'before':0,'after':None,'delta':None},
                            '3':{'before':None,'after':0,'delta':None}}},
                        'character_bank':{'before_observed':True,'after_observed':False,'containers':{
                            '6':{'before':20,'after':None,'delta':None}}},
                        'account_bank':{'before_observed':False,'after_observed':True,'containers':{}},
                    })
                    del comparison['item_id']
                    reply=await session.call_tool('compare_snapshots',{**comparison,'item_container_changes':True})
                    self.assertTrue(reply.isError)
            for path,data in protected.items(): self.assertEqual(path.read_bytes(),data)


if __name__=='__main__':
    unittest.main()
