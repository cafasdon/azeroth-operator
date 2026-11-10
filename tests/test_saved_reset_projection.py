"""Historical reset projection through the actual owner-local stdio MCP."""
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


class ResetProjectionTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_calendar_unknowns_scopes_and_privacy(self):
        root = Path(__file__).resolve().parents[1]
        binary = root / 'target/debug' / ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryDirectory() as foreign:
            store = SnapshotStore(Path(folder))
            capture = fixture()
            capture['client'] = {'family':'forever', 'variant':'live', 'interface':16001,
                                 'build':'70862'}
            capture['captured_at'] = '2025-01-01T23:59:00Z'
            capture['sections']['saved_world_bosses'] = {'coverage':'updated_cache',
                'update_age_seconds':999, 'entries':{
                    '1':{'name':'Fixture', 'reset_seconds':120},
                    '2':{'name':'Unknown', 'reset_seconds':-1},
                    '3':{'name':'Zero', 'reset_seconds':0}}}
            capture['sections']['saved_instances'] = {'coverage':'updated_cache',
                'update_age_seconds':5, 'entries':{'1':{'name':'Fixture raid',
                    'reset_seconds':3600, 'difficulty_id':1, 'locked':True,
                    'extended':True, 'is_raid':True, 'max_players':40}}}
            key = store.ingest(capture)
            encounter_capture=deepcopy(capture)
            encounter_capture['sections']={}
            encounter_capture['sections']['instance_context']={
                'kind':'raid','name':'Fixture raid','difficulty_id':1,'lock_warning':{
                    'remaining_seconds':60,'encounters_total':3,'encounters_completed':1,'encounters':{
                        '1':{'name':'Same name','killed':False},'2':{'name':'Defeated','killed':True},
                        '3':{'name':'Same name','killed':False}}}}
            encounters_id=store.ingest(encounter_capture)
            warning=encounter_capture['sections']['instance_context']['lock_warning']
            del warning['encounters']
            counts_only_id=store.ingest(encounter_capture)
            empty = deepcopy(capture)
            for section in ['saved_instances', 'saved_world_bosses']:
                empty['sections'][section]['entries'] = {}
            empty_key = store.ingest(empty)
            missing = deepcopy(capture)
            for section in ['saved_instances', 'saved_world_bosses']:
                del missing['sections'][section]
            missing_key = store.ingest(missing)
            retail_key = store.ingest(fixture())
            private = deepcopy(capture)
            private['character']['name'] = 'PRIVATE_OTHER'
            foreign_key = SnapshotStore(Path(foreign)).ingest(private)
            before = {p:p.read_bytes() for base in [folder, foreign] for p in Path(base).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),
                    env={**os.environ, 'AZEROTH_DATA_DIR':folder})) as (reader, writer):
                async with ClientSession(reader, writer) as session:
                    await session.initialize()
                    for warning_filter, expected in [
                        ({},{encounters_id,counts_only_id}),
                        ({'minimum_completed_encounters':1,'minimum_remaining_encounters':2,
                          'maximum_warning_seconds':60,'has_encounter_details':True},{encounters_id}),
                        ({'has_encounter_details':False},{counts_only_id}),
                        ({'minimum_completed_encounters':1,'maximum_completed_encounters':1,
                          'minimum_remaining_encounters':2,'maximum_remaining_encounters':2},
                         {encounters_id,counts_only_id}),
                        ({'maximum_completed_encounters':0},set()),
                        ({'maximum_remaining_encounters':1},set()),
                        ({'minimum_remaining_encounters':3},set()),({'maximum_warning_seconds':59},set()),
                    ]:
                        reply=await session.call_tool('list_snapshots',{
                            'family':'forever','instance_lock_warning':warning_filter})
                        self.assertFalse(reply.isError,reply)
                        rows=json.loads(reply.content[0].text)['snapshots']
                        self.assertEqual({row['snapshot_id'] for row in rows},expected)
                        for row in rows:
                            match=row['instance_lock_warning_match']
                            self.assertEqual(match['encounters_remaining'],2)
                            self.assertEqual(match['reported_warning_remaining_seconds'],60)
                            self.assertNotIn('Same name',json.dumps(row))
                            self.assertFalse(match['current_warning_verified'])
                    self.assertTrue((await session.call_tool('list_snapshots',{
                        'family':'retail','instance_lock_warning':{}})).isError)
                    for selector, count in [
                        ({'encounter_name':'  sAME NAME ','encounter_defeated':False},2),
                        ({'encounter_name':'Defeated','encounter_defeated':True},1),
                        ({'encounter_name':'Same name','encounter_defeated':True},0),
                        ({'encounter_name':'Same'},0),
                    ]:
                        reply=await session.call_tool('list_snapshots',{
                            'family':'forever','instance_lock_warning':selector})
                        self.assertFalse(reply.isError,reply)
                        matches=json.loads(reply.content[0].text)['snapshots']
                        self.assertEqual([row['snapshot_id'] for row in matches],[encounters_id] if count else [])
                        if count:
                            self.assertEqual(matches[0]['instance_lock_warning_match']['matching_encounter_count'],count)
                            self.assertNotIn('Same name',json.dumps(matches))
                    for invalid in [{'encounter_defeated':False},{'encounter_name':' '},{'encounter_name':'é'*129},
                                    {'minimum_completed_encounters':2,'maximum_completed_encounters':1},
                                    {'minimum_remaining_encounters':3,'maximum_remaining_encounters':2},
                                    {'maximum_remaining_encounters':-1},
                                    {'maximum_completed_encounters':True}]:
                        reply=await session.call_tool('list_snapshots',{
                            'family':'forever','instance_lock_warning':invalid})
                        self.assertTrue(reply.isError,reply)
                    for capture_id, details in [(encounters_id,{
                            'completed':[{'slot':2,'name':'Defeated'}],
                            'pending':[{'slot':1,'name':'Same name'},{'slot':3,'name':'Same name'}]}),
                            (counts_only_id,None),(key,None)]:
                        reply=await session.call_tool('inspect_snapshot',{
                            'snapshot_id':capture_id,'section':'instance_context','instance_lock_encounter_review':True})
                        self.assertFalse(reply.isError,reply)
                        review=json.loads(reply.content[0].text)['instance_lock_encounter_review']
                        if capture_id==key:
                            self.assertIsNone(review)
                        else:
                            self.assertEqual(review['encounter_details'],details)
                            self.assertEqual(review['encounters_remaining'],2)
                            self.assertEqual(review['projected_warning_expiry_at'],'2025-01-02T00:00:00Z')
                            self.assertFalse(review['saved_lockout_identity_verified'])
                            self.assertNotIn('warning_reference_at',review)
                    for timestamp, elapsed in [('2025-01-01T23:59:59.999Z',False),
                                               ('2025-01-02T01:00:00+01:00',True),
                                               ('2025-01-02T00:00:01Z',True)]:
                        reply=await session.call_tool('inspect_snapshot',{
                            'snapshot_id':encounters_id,'section':'instance_context',
                            'instance_lock_encounter_review':True,'instance_lock_warning_reference_at':timestamp})
                        self.assertFalse(reply.isError,reply)
                        review=json.loads(reply.content[0].text)['instance_lock_encounter_review']
                        self.assertIs(review['warning_expiry_at_or_before_reference'],elapsed)
                        self.assertFalse(review['current_warning_verified'])
                    self.assertTrue((await session.call_tool('inspect_snapshot',{
                        'snapshot_id':encounters_id,'section':'instance_context',
                        'instance_lock_warning_reference_at':'2025-01-02T00:00:00Z'})).isError)
                    for capture_id, section in [(key,'saved_instances'),(retail_key,'instance_context')]:
                        reply=await session.call_tool('inspect_snapshot',{
                            'snapshot_id':capture_id,'section':section,'instance_lock_encounter_review':True})
                        self.assertTrue(reply.isError,reply)
                    for selector,name,reset in [
                        ('saved_world_boss','Fixture','2025-01-02T00:01:00Z'),
                        ('saved_instance','Fixture raid','2025-01-02T00:59:00Z'),
                    ]:
                        for start,end,expected in [
                            (reset,None,[key]),(reset,reset,[key]),
                            ('2025-01-02T01:00:00Z',None,[]),
                            ('2025-01-02T01:00:00Z',reset,None),
                        ]:
                            reply=await session.call_tool('list_snapshots',{'family':'forever',
                                selector:{'name':name,'projected_reset_from':start,'projected_reset_by':end}})
                            self.assertEqual(bool(reply.isError),expected is None,reply)
                            if expected is not None:
                                matches=json.loads(reply.content[0].text)['snapshots']
                                self.assertEqual([r['snapshot_id'] for r in matches],expected)
                                if expected:
                                    self.assertEqual(matches[0][f'{selector}_match']['entries'][0]['projected_reset_at'],reset)
                        plain=await session.call_tool('list_snapshots',{'family':'forever',selector:{'name':name}})
                        self.assertFalse(plain.isError,plain)
                        entry=json.loads(plain.content[0].text)['snapshots'][0][f'{selector}_match']['entries'][0]
                        self.assertNotIn('projected_reset_at',entry)
                    for selector,name,deadline in [
                        ('saved_world_boss','Fixture','2025-01-02T00:01:00Z'),
                        ('saved_world_boss','Unknown','2025-01-03T00:00:00Z'),
                        ('saved_world_boss','Zero','2025-01-01T23:59:00Z'),
                        ('saved_instance','Fixture raid','2025-01-02T00:58:59Z'),
                        ('saved_instance','Fixture raid','2025-01-01T19:59:00-05:00'),
                    ]:
                        reply=await session.call_tool('list_snapshots',{'family':'forever',
                            selector:{'name':name,'projected_reset_by':deadline}})
                        self.assertFalse(reply.isError,reply)
                        self.assertEqual([r['snapshot_id'] for r in json.loads(reply.content[0].text)['snapshots']],
                            [] if name=='Unknown' or deadline.endswith('58:59Z') else [key])
                    async def inspect(snapshot_id=key, section='saved_world_bosses', flag=True, error=False, **extra):
                        args = {'snapshot_id':snapshot_id, 'saved_reset_projection':flag, **extra}
                        if section is not None:
                            args['section'] = section
                        reply = await session.call_tool('inspect_snapshot', args)
                        self.assertEqual(bool(reply.isError), error, reply)
                        self.assertNotIn('PRIVATE', str(reply))
                        self.assertNotIn(foreign, str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    result = await inspect()
                    self.assertEqual(result['data'], capture['sections']['saved_world_bosses'])
                    projected = result['saved_reset_projection']
                    self.assertEqual(projected['rows'][0]['projected_reset_at'], '2025-01-02T00:01:00Z')
                    self.assertEqual(projected['rows'][1]['projection_status'], 'negative_reported_duration')
                    self.assertIsNone(projected['rows'][1]['projected_reset_at'])
                    self.assertEqual(projected['rows'][2]['projected_reset_at'], capture['captured_at'])
                    self.assertEqual([projected[f] for f in ['recorded_count','projected_count','unknown_count']], [3,2,1])
                    self.assertEqual(projected['cache_update_age_seconds'], 999)
                    combined = await session.call_tool('character_summary', {
                        'snapshot_id':key,'saved_reset_reference_at':'2025-01-01T23:59:00Z'})
                    self.assertFalse(combined.isError,combined)
                    schedule = json.loads(combined.content[0].text)['saved_reset_schedule']
                    self.assertNotIn('cache_age_filter_matches',schedule)
                    self.assertEqual(schedule['upcoming_projected_resets'], [
                        {'at':'2025-01-02T00:01:00Z','entries':[
                            {'section':'saved_world_bosses','slot':1,'name':'Fixture'}]},
                        {'at':'2025-01-02T00:59:00Z','entries':[
                            {'section':'saved_instances','slot':1,'name':'Fixture raid',
                             'saved_instance_details':{'difficulty_id':1,'locked':True,
                                'extended':True,'is_raid':True,'max_players':40}}]}])
                    self.assertEqual(schedule['coverage']['saved_world_bosses']['unknown_count'],1)
                    self.assertEqual(schedule['matching_reset_window_counts']['saved_world_bosses'],{
                        'at_or_before_reference':1,'within_window':1,'after_window':0,'unknown_reset':1})
                    for limit, instances, bosses, expected in [
                            (4,False,False,[]),(5,True,False,schedule['upcoming_projected_resets'][1:]),
                            (999,True,True,schedule['upcoming_projected_resets'])]:
                        reply=await session.call_tool('character_summary',{
                            'snapshot_id':key,'saved_reset_reference_at':'2025-01-01T23:59:00Z',
                            'saved_reset_maximum_update_age_seconds':limit})
                        self.assertFalse(reply.isError,reply)
                        fresh=json.loads(reply.content[0].text)['saved_reset_schedule']
                        self.assertEqual(fresh['cache_age_filter_matches'],{
                            'saved_instances':instances,'saved_world_bosses':bosses})
                        self.assertEqual(fresh['upcoming_projected_resets'],expected)
                        self.assertEqual(fresh['coverage'],schedule['coverage'])
                        if not bosses:
                            self.assertEqual(fresh['matching_reset_window_counts']['saved_world_bosses']['unknown_reset'],0)
                    self.assertTrue((await session.call_tool('character_summary',{
                        'snapshot_id':key,'saved_reset_maximum_update_age_seconds':0})).isError)
                    for kind, expected in [('world_boss',schedule['upcoming_projected_resets'][:1]),
                                           ('raid',schedule['upcoming_projected_resets'][1:]),('dungeon',[])]:
                        reply = await session.call_tool('character_summary', {
                            'snapshot_id':key,'saved_reset_reference_at':'2025-01-01T23:59:00Z',
                            'saved_reset_kind':kind})
                        self.assertFalse(reply.isError,reply)
                        filtered = json.loads(reply.content[0].text)['saved_reset_schedule']
                        self.assertEqual(filtered['upcoming_projected_resets'],expected)
                        self.assertEqual(filtered['next_projected_reset'],expected[0] if expected else None)
                        self.assertEqual(filtered['coverage'],schedule['coverage'])
                        self.assertEqual(filtered['kind'],kind)
                        if kind == 'raid':
                            self.assertEqual(filtered['matching_reset_window_counts']['saved_world_bosses'],{
                                'at_or_before_reference':0,'within_window':0,'after_window':0,'unknown_reset':0})
                    self.assertIs(schedule['current_lockout_verified'],False)
                    for filters, expected in [
                        ({},schedule['upcoming_projected_resets'][1:]),
                        ({'difficulty_id':1,'locked':True,'extended':True,'max_players':40},schedule['upcoming_projected_resets'][1:]),
                        ({'difficulty_id':2},[]),({'locked':False},[]),({'extended':False},[]),({'max_players':10},[]),
                    ]:
                        reply=await session.call_tool('character_summary',{
                            'snapshot_id':key,'saved_reset_reference_at':'2025-01-01T23:59:00Z',
                            'saved_reset_instance_filter':filters})
                        self.assertFalse(reply.isError,reply)
                        filtered=json.loads(reply.content[0].text)['saved_reset_schedule']
                        self.assertEqual(filtered['upcoming_projected_resets'],expected)
                        self.assertEqual(filtered['coverage'],schedule['coverage'])
                    for end, expected in [('2025-01-02T00:58:59Z',[]),
                                          ('2025-01-02T00:59:00Z',schedule['upcoming_projected_resets'][1:])]:
                        reply=await session.call_tool('character_summary',{
                            'snapshot_id':key,'saved_reset_reference_at':'2025-01-01T23:59:00Z',
                            'saved_reset_through':end,'saved_reset_kind':'raid',
                            'saved_reset_instance_filter':{'locked':True}})
                        self.assertFalse(reply.isError,reply)
                        self.assertEqual(json.loads(reply.content[0].text)['saved_reset_schedule'][
                            'upcoming_projected_resets'],expected)
                    for extras in [{},{'saved_reset_kind':'world_boss','saved_reset_reference_at':'2025-01-01T23:59:00Z'}]:
                        reply=await session.call_tool('character_summary',{
                            'snapshot_id':key,'saved_reset_instance_filter':{'locked':True},**extras})
                        self.assertTrue(reply.isError,reply)
                    window = await session.call_tool('character_summary', {
                        'snapshot_id':key,'saved_reset_reference_at':'2025-01-01T23:59:00Z',
                        'saved_reset_through':'2025-01-02T00:01:00Z'})
                    self.assertFalse(window.isError,window)
                    bounded = json.loads(window.content[0].text)['saved_reset_schedule']
                    self.assertEqual(bounded['upcoming_projected_resets'],schedule['upcoming_projected_resets'][:1])
                    self.assertEqual(bounded['coverage'],schedule['coverage'])
                    self.assertEqual(bounded['matching_reset_window_counts']['saved_instances'],{
                        'at_or_before_reference':0,'within_window':0,'after_window':1,'unknown_reset':0})
                    for capture_id, counts in [(missing_key,None),(empty_key,{
                            'at_or_before_reference':0,'within_window':0,'after_window':0,'unknown_reset':0})]:
                        reply=await session.call_tool('character_summary',{
                            'snapshot_id':capture_id,'saved_reset_reference_at':'2025-01-01T23:59:00Z',
                            'saved_reset_maximum_update_age_seconds':999})
                        self.assertFalse(reply.isError,reply)
                        observed=json.loads(reply.content[0].text)['saved_reset_schedule']
                        self.assertEqual(observed['matching_reset_window_counts'],{'saved_instances':counts,'saved_world_bosses':counts})
                        match=None if counts is None else True
                        self.assertEqual(observed['cache_age_filter_matches'],{'saved_instances':match,'saved_world_bosses':match})
                    self.assertEqual(bounded['through_at'],'2025-01-02T00:01:00Z')
                    reversed_window = await session.call_tool('character_summary', {
                        'snapshot_id':key,'saved_reset_reference_at':'2025-01-02T00:01:00Z',
                        'saved_reset_through':'2025-01-01T23:59:00Z'})
                    self.assertTrue(reversed_window.isError)
                    reference = {'saved_reset_reference_at':'2025-01-01T23:59:00Z'}
                    relative = (await inspect(**reference))['saved_reset_projection']
                    self.assertEqual(relative['relative_to_reference'], {
                        'at_or_before_slots':[3],'after_slots':[1],'unknown_slots':[2],
                        'next_projected_reset':{'at':'2025-01-02T00:01:00Z','slots':[1]},
                        'upcoming_projected_resets':[{'at':'2025-01-02T00:01:00Z','slots':[1]}]})
                    early = (await inspect(saved_reset_reference_at='2025-01-01T23:58:00Z'))[
                        'saved_reset_projection']['relative_to_reference']
                    self.assertEqual(early['upcoming_projected_resets'], [
                        {'at':'2025-01-01T23:59:00Z','slots':[3]},
                        {'at':'2025-01-02T00:01:00Z','slots':[1]}])
                    at_reset = (await inspect(saved_reset_reference_at='2025-01-02T00:01:00Z'))[
                        'saved_reset_projection']['relative_to_reference']
                    self.assertIsNone(at_reset['next_projected_reset'])
                    self.assertEqual(at_reset['upcoming_projected_resets'], [])
                    self.assertEqual(relative['rows'],projected['rows'])
                    self.assertIsNone((await inspect(missing_key,**reference))[
                        'saved_reset_projection']['relative_to_reference'])
                    await inspect(flag=False,error=True,**reference)
                    for field in ['complete_coverage_verified','current_lockout_verified',
                                  'current_availability_verified','extension_outcome_inferred','game_execution','live']:
                        self.assertIs(projected[field], False)
                    raid = (await inspect(section='saved_instances'))['saved_reset_projection']
                    self.assertEqual(raid['rows'][0]['projected_reset_at'], '2025-01-02T00:59:00Z')
                    self.assertIs(raid['rows'][0]['observation']['extended'], True)
                    self.assertEqual((await inspect(empty_key))['saved_reset_projection']['rows'], [])
                    self.assertIsNone((await inspect(missing_key))['saved_reset_projection']['rows'])
                    self.assertNotIn('saved_reset_projection', await inspect(flag=False))
                    for section in [None, 'quests', 'skill_lines']:
                        await inspect(section=section, error=True)
                    await inspect(retail_key, error=True)
                    await inspect(foreign_key, error=True)
                    for bad in ['PRIVATE_CANARY', 1, 0, None, {}, []]:
                        await inspect(flag=bad, error=True)
                    await inspect(skill_line_filter={'filter':{'minimum_rank':0}}, error=True)
                    updated=store.read(encounters_id)
                    updated['captured_at']='2025-01-02T00:00:00Z'
                    warning=updated['sections']['instance_context']['lock_warning']
                    warning['encounters']['3']['killed']=True
                    warning['encounters_completed']=2
                    updated_id=store.ingest(updated)
                    warning['encounters']['3']['name']='Different encounter'
                    changed_names_id=store.ingest(updated)
                    updated['sections']['instance_context']['difficulty_id']=2
                    different_difficulty_id=store.ingest(updated)
                    for after_id,delta,changes in [
                        (updated_id,1,[{'slot':3,'name':'Same name','before_defeated':False,'after_defeated':True}]),
                        (changed_names_id,1,None),(counts_only_id,0,None),(different_difficulty_id,None,None),
                        (key,None,None),
                    ]:
                        reply=await session.call_tool('compare_snapshots',{
                            'before_id':encounters_id,'after_id':after_id,'instance_lock_warning_changes':True})
                        self.assertFalse(reply.isError,reply)
                        report=json.loads(reply.content[0].text)['instance_lock_warning_changes']
                        if delta is None:
                            self.assertIsNone(report)
                        else:
                            self.assertEqual(report['completed_count_delta'],delta)
                            self.assertEqual(report['encounter_flag_changes'],changes)
                            self.assertFalse(report['saved_lockout_identity_verified'])
                    reply=await session.call_tool('compare_snapshots',{
                        'before_id':retail_key,'after_id':retail_key,'instance_lock_warning_changes':True})
                    self.assertTrue(reply.isError,reply)
            self.assertEqual(before, {p:p.read_bytes() for p in before})


if __name__ == '__main__':
    unittest.main()
