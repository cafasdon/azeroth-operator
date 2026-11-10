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
                    async def search(hand, **filters):
                        return (await call('list_snapshots',family='forever',
                            weapon_enchant={'hand':hand,**filters}))['snapshots']
                    covered=await call('list_snapshots',family='forever',required_sections=['weapon_enchants'])
                    self.assertEqual({row['snapshot_id'] for row in covered['snapshots']},
                        {keys[0],keys[2],keys[3]})
                    self.assertEqual((await call('list_snapshots',family='forever',
                        required_sections=['weapon_enchants','player_resources']))['snapshots'],[])
                    newest=deepcopy(absent)
                    newest['captured_at']='2025-01-02T00:00:00Z'
                    store.ingest(newest)
                    unavailable=deepcopy(newest)
                    unavailable['captured_at']='2025-01-03T00:00:00Z'
                    unavailable['unavailable']={'weapon_enchants':'PRIVATE_REASON'}
                    unavailable_id=store.ingest(unavailable)
                    for offset in (0,1,99):
                        coverage=await call('list_snapshots',family='forever',limit=1,offset=offset,
                            coverage_summary_sections=['weapon_enchants','player_resources'])
                        self.assertEqual(coverage['section_coverage_summary'],{
                            'matching_snapshot_count':6,'sections':{
                                'weapon_enchants':{'observed':3,'reported_unavailable':1,'not_recorded':2},
                                'player_resources':{'observed':0,'reported_unavailable':0,'not_recorded':6}}})
                        self.assertNotIn('PRIVATE_REASON',json.dumps(coverage))
                    latest_coverage=await call('list_snapshots',family='forever',latest_per_character=True,
                        coverage_summary_sections=['weapon_enchants'])
                    self.assertEqual(latest_coverage['section_coverage_summary'],{
                        'matching_snapshot_count':1,'sections':{
                            'weapon_enchants':{'observed':0,'reported_unavailable':1,'not_recorded':0}}})
                    view=await call('inspect_snapshot',snapshot_id=unavailable_id,
                        coverage_sections=['weapon_enchants','equipment','player_resources'])
                    self.assertEqual(view['section_coverage'],{
                        'weapon_enchants':'reported_unavailable','equipment':'observed',
                        'player_resources':'not_recorded'})
                    for omitted in ['data','unavailable','available_sections','unavailable_reason']:
                        self.assertNotIn(omitted,view)
                    self.assertNotIn('PRIVATE_REASON',json.dumps(view))
                    for options in [{'coverage_sections':[]},{'coverage_sections':['unknown']},
                                    {'coverage_sections':['equipment','equipment']},
                                    {'coverage_sections':['equipment'],'section':'equipment'}]:
                        invalid=await session.call_tool('inspect_snapshot',{'snapshot_id':unavailable_id,**options})
                        self.assertTrue(invalid.isError)
                    missing=await call('list_snapshots',family='forever',latest_per_character=True,
                        any_missing_sections=['weapon_enchants','player_resources'])
                    self.assertEqual([row['snapshot_id'] for row in missing['snapshots']],[unavailable_id])
                    self.assertEqual(missing['snapshots'][0]['missing_section_matches'],
                        {'weapon_enchants':'reported_unavailable','player_resources':'not_recorded'})
                    self.assertNotIn('PRIVATE_REASON',json.dumps(missing))
                    for before_id,status,available in [(keys[0],'observed',True),
                                                       (keys[1],'not_recorded',False)]:
                        comparison=await call('compare_snapshots',before_id=before_id,after_id=unavailable_id)
                        self.assertEqual(comparison['sections']['weapon_enchants'],{
                            'status':'not_comparable','before_available':available,'after_available':False,
                            'before_capture_status':status,'after_capture_status':'reported_unavailable'})
                        self.assertNotIn('PRIVATE_REASON',json.dumps(comparison))
                    mixed=await call('list_snapshots',family='forever',required_sections=['weapon_enchants'],
                        any_missing_sections=['weapon_enchants','player_resources'])
                    self.assertEqual({row['snapshot_id'] for row in mixed['snapshots']},{keys[0],keys[2],keys[3]})
                    self.assertTrue(all(row['missing_section_matches']=={'player_resources':'not_recorded'}
                                        for row in mixed['snapshots']))
                    self.assertEqual((await call('list_snapshots',family='forever',latest_per_character=True,
                        required_sections=['weapon_enchants']))['snapshots'],[])
                    for required in [[],['weapon_enchants','weapon_enchants'],['unknown']]:
                        invalid=await session.call_tool('list_snapshots',{
                            'family':'forever','required_sections':required})
                        self.assertTrue(invalid.isError)
                        invalid=await session.call_tool('list_snapshots',{
                            'family':'forever','any_missing_sections':required})
                        self.assertTrue(invalid.isError)
                    found = await search('main_hand',enchant_id=42,minimum_charges=0,
                                         maximum_remaining_milliseconds=0)
                    self.assertEqual([row['snapshot_id'] for row in found],[keys[0]])
                    self.assertEqual(found[0]['weapon_enchant_match'],
                        {'hand':'main_hand','matching_observed_rows':[1],'live':False})
                    self.assertEqual(await search('main_hand',enchant_id=42,minimum_charges=1),[])
                    self.assertEqual(await search('off_hand',enchant_id=42),[])
                    self.assertEqual(await search('main_hand',enchant_type='permanent',maximum_charges=0),found)
                    self.assertEqual(await search('main_hand',enchant_type='imbue',maximum_charges=0),[])
                    imbues=await search('main_hand',enchant_type='imbue')
                    self.assertEqual(imbues[0]['weapon_enchant_match']['matching_observed_rows'],[2])
                    self.assertEqual(await search('main_hand',enchant_type='temporary',enchant_id=42),[])
                    invalid_range=await session.call_tool('list_snapshots',{'family':'forever',
                        'weapon_enchant':{'hand':'main_hand','minimum_charges':1,'maximum_charges':0}})
                    self.assertTrue(invalid_range.isError)
                    unrestricted = await search('main_hand')
                    self.assertEqual(next(row for row in unrestricted if row['snapshot_id']==keys[0])
                        ['weapon_enchant_match']['matching_observed_rows'],[1,2,10])
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
                    reordered=deepcopy(value)
                    reordered['sections']['weapon_enchants']['hands']['main_hand']['entries']={
                        str(i):row for i,row in enumerate(reversed(list(rows.values())),1)}
                    changed=deepcopy(reordered)
                    changed['sections']['weapon_enchants']['hands']['main_hand']['entries']['10']['charges']=1
                    async def compare(after):
                        return (await call('compare_snapshots',before_id=keys[0],after_id=after,
                            weapon_enchant_changes=True))['weapon_enchant_changes']
                    same=await compare(store.ingest(reordered))
                    self.assertFalse(same['hands']['main_hand']['observed_rows_changed'])
                    self.assertEqual(same['hands']['off_hand'],
                        {'before_rows':[],'after_rows':[],'observed_rows_changed':False,'shared_temporary_enchant_expiries':[]})
                    self.assertEqual(same['hands']['ranged'],
                        {'before_rows':None,'after_rows':None,'observed_rows_changed':None,'shared_temporary_enchant_expiries':None})
                    difference=await compare(store.ingest(changed))
                    self.assertTrue(difference['hands']['main_hand']['observed_rows_changed'])
                    self.assertFalse(difference['enchant_events_verified'])
                    partial=await compare(keys[1])
                    self.assertEqual(len(partial['hands']['main_hand']['before_rows']),10)
                    self.assertIsNone(partial['hands']['main_hand']['observed_rows_changed'])
                    timed = deepcopy(value)
                    timed['character']['name'] = 'Timed'
                    timed['sections']['weapon_enchants']['hands']['main_hand']['entries']['10'][
                        'remaining_milliseconds'] = 1
                    timed['sections']['weapon_enchants']['hands']['main_hand']['entries']['3'] = {
                        'has_enchant':True,'enchant_type':'temporary','enchant_id':43}
                    timed_key = store.ingest(timed)
                    deadline = '2025-01-01T00:00:00.001Z'
                    expiring = await search('main_hand',projected_expiry_by=deadline)
                    self.assertEqual([row['snapshot_id'] for row in expiring], [timed_key])
                    self.assertEqual(expiring[0]['weapon_enchant_match']['matching_observed_rows'], [10])
                    self.assertEqual(await search('main_hand',projected_expiry_by='2025-01-01T00:00:00Z'), [])
                    self.assertEqual(await search('main_hand',projected_expiry_by=deadline,enchant_id=42), [])
                    self.assertEqual(await search('off_hand',projected_expiry_by=deadline), [])
                    for reference, expired in [('2025-01-01T00:00:00Z',False),(deadline,True)]:
                        result=await call('inspect_snapshot',snapshot_id=timed_key,section='weapon_enchants',
                                          weapon_enchant_reference_at=reference)
                        review=result['weapon_enchant_expiry_review']
                        self.assertEqual(review['hands']['off_hand'],{})
                        self.assertIsNone(review['hands']['ranged'])
                        self.assertEqual(set(review['hands']['main_hand']),{'3','10'})
                        self.assertEqual(review['hands']['main_hand']['10']['projected_expiry_at'],deadline)
                        self.assertIs(review['hands']['main_hand']['10']['projected_expiry_at_or_before_reference'],expired)
                        self.assertIsNone(review['hands']['main_hand']['3']['projected_expiry_at'])
                        self.assertFalse(review['current_enchant_verified'])
                    missing_review=await call('inspect_snapshot',snapshot_id=keys[1],section='weapon_enchants',
                                              weapon_enchant_reference_at=deadline)
                    self.assertIsNone(missing_review['weapon_enchant_expiry_review'])
                    rejected=await session.call_tool('inspect_snapshot',{
                        'snapshot_id':timed_key,'section':'equipment','weapon_enchant_reference_at':deadline})
                    self.assertTrue(rejected.isError)
                    comparable=deepcopy(timed)
                    comparable['sections']['weapon_enchants']['hands']['main_hand']['entries']['10']['enchant_id']=44
                    before_id=store.ingest(comparable)
                    comparable['captured_at']=deadline
                    for milliseconds, matches in [(0,True),(1,False),(None,None)]:
                        row=comparable['sections']['weapon_enchants']['hands']['main_hand']['entries']['10']
                        if milliseconds is None:
                            row.pop('remaining_milliseconds')
                        else:
                            row['remaining_milliseconds']=milliseconds
                        report=await call('compare_snapshots',before_id=before_id,after_id=store.ingest(comparable),
                                          weapon_enchant_changes=True)
                        hand=report['weapon_enchant_changes']['hands']['main_hand']
                        expiry=next(row for row in hand['shared_temporary_enchant_expiries'] if row['enchant_id']==44)
                        self.assertIs(hand['observed_rows_changed'],milliseconds != 1)
                        self.assertEqual(expiry['before_projected_expiries'],[deadline])
                        self.assertIs(expiry['projected_expiries_match'],matches)
                    invalid_deadline = await session.call_tool('list_snapshots', {'family':'forever',
                        'weapon_enchant':{'hand':'main_hand','projected_expiry_by':'invalid'}})
                    self.assertTrue(invalid_deadline.isError)
                    for bad in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'weapon_enchants'})
                        self.assertTrue(response.isError);self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
