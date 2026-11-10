"""Observed taxi preview segments annotate one quoted leg, not separate purchases."""
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


class FlightRouteNodesTests(unittest.IsolatedAsyncioTestCase):
    async def test_paths_validate_and_annotate_real_mcp_callers(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['wallet']={'copper':200}
        base['sections']['flight_map']={'map_id':1,'entries':{
            '1':{'node_id':10,'name':'Origin','state':'current','map_transition':False},
            '2':{'node_id':20,'name':'Target','state':'reachable','map_transition':False,'fare_copper':100,
                 'observed_route_node_ids':{'1':10,'2':30,'3':20}},
            '3':{'node_id':30,'name':'Transit','state':'unreachable','map_transition':False}}}
        legacy=deepcopy(base);legacy['sections']['flight_map']['entries']['2'].pop('observed_route_node_ids')
        malformed=[]
        for path in [None,[],{}, {'1':10}, {'1':10,'3':20}, {'01':10,'2':20},
                     {'1':30,'2':20}, {'1':10,'2':30}, {'1':10,'2':400},
                     {'1':10,'2':30,'3':30,'4':20}, {'1':10,'2':True,'3':20},
                     {'1':10,'2':'PRIVATE','3':20}, {'1':10,'2':20.0},
                     {str(i):i for i in range(1,67)}]:
            value=deepcopy(base);value['sections']['flight_map']['entries']['2']['observed_route_node_ids']=path
            malformed.append(value)
        for change in ['transition','duplicate','two_origins','no_origin','unreachable_target']:
            value=deepcopy(base);rows=value['sections']['flight_map']['entries']
            if change=='transition':rows['3']['map_transition']=True
            if change=='duplicate':rows['4']=deepcopy(rows['3'])
            if change=='two_origins':rows['3']['state']='current'
            if change=='no_origin':rows['1']['state']='unreachable'
            if change=='unreachable_target':rows['2']['state']='unreachable';rows['2'].pop('fare_copper')
            malformed.append(value)
        onward=deepcopy(base);onward['sections']['flight_map']={'map_id':2,'entries':{
            '1':{'node_id':20,'name':'Target','state':'current','map_transition':False},
            '2':{'node_id':40,'name':'Further','state':'reachable','map_transition':False,'fare_copper':50,
                 'observed_route_node_ids':{'1':20,'2':40}}}}
        onward['captured_at']='2025-01-01T00:00:10Z'
        fractional=deepcopy(onward)
        fractional['captured_at']='2025-01-01T00:00:10.001Z'
        longest=deepcopy(base)
        longest_rows={str(i):{'node_id':i,'name':'Node','state':('current' if i==1 else 'reachable' if i==65 else 'unreachable'),
                             'map_transition':False} for i in range(1,66)}
        longest_rows['65']['fare_copper']=100
        longest_rows['65']['observed_route_node_ids']={str(i):i for i in range(1,66)}
        longest['sections']['flight_map']['entries']=longest_rows
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));key=store.ingest(base);old=store.ingest(legacy);next_key=store.ingest(onward)
            fractional_key=store.ingest(fractional)
            longest_key=store.ingest(longest)
            bad_keys=[]
            for value in malformed:
                with self.assertRaises(ValueError):validate_snapshot(value)
                raw=json.dumps(value,sort_keys=True,separators=(',',':')).encode()
                bad=hashlib.sha256(raw).hexdigest();(Path(folder)/f'{bad}.json').write_bytes(raw);bad_keys.append(bad)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,**arguments):
                        response=await session.call_tool(tool,arguments)
                        self.assertFalse(response.isError,response.content)
                        return json.loads(response.content[0].text)
                    path={'1':10,'2':30,'3':20}
                    data=await call('inspect_snapshot',snapshot_id=key,section='flight_map',flight_node_id=20)
                    self.assertEqual(data['data']['entries']['2']['observed_route_node_ids'],path)
                    for reserve, affordable, gap, spendable in [(0,True,0,100),(100,True,0,0),(101,False,1,None)]:
                        result = await call('inspect_snapshot', snapshot_id=key, section='flight_map',
                            flight_wallet_projection=True, flight_wallet_reserve_copper=reserve)
                        projection = result['flight_wallet_projection']
                        self.assertEqual(projection['2'], {
                            'affordable':affordable, 'shortfall_copper':gap,
                            'remaining_wallet_after_fare_copper':100,
                            'spendable_after_fare_and_reserve_copper':spendable,
                        })
                        self.assertTrue(all(value is None for value in projection['1'].values()))
                    search=await call('inspect_snapshot',snapshot_id=key,section='flight_map',flight_route_search={'destination_node_id':20})
                    plan=search['flight_route_search']
                    for limit,status in [(99,'no_route_within_fare_ceiling'),(100,'found')]:
                        bounded=await call('inspect_snapshot',snapshot_id=key,section='flight_map',
                            flight_route_search={'destination_node_id':20,'maximum_total_fare_copper':limit})
                        self.assertEqual(bounded['flight_route_search']['status'],status)
                        if limit==99:
                            self.assertEqual(bounded['flight_route_search']['saved_path_evidence'],{
                                'reachable_path_observed':True,
                                'fully_priced_path_observed_without_budget':True,'maximum_legs':8,
                                'minimum_fully_priced_fare_copper':100,'effective_fare_ceiling_copper':99,
                                'shortfall_vs_effective_ceiling_copper':1})
                    self.assertEqual(len(plan['legs']),1)
                    self.assertEqual(plan['legs'][0]['observed_route_node_ids'],path)
                    self.assertEqual(plan['total_fare_copper'],100)
                    named_plan=(await call('inspect_snapshot',snapshot_id=key,section='flight_map',
                        flight_route_search={'destination_name':'  FuRtHeR  ','onward_snapshot_ids':[next_key]}))[
                            'flight_route_search']
                    self.assertEqual(named_plan['destination_node_id'],40)
                    self.assertEqual(named_plan['total_fare_copper'],150)
                    self.assertEqual(len(named_plan['legs']),2)
                    self.assertNotIn('selected_capture_window',named_plan)
                    bounded={'destination_node_id':40,'onward_snapshot_ids':[next_key],
                             'maximum_capture_skew_seconds':10}
                    window=(await call('inspect_snapshot',snapshot_id=key,section='flight_map',
                        flight_route_search=bounded))['flight_route_search']['selected_capture_window']
                    self.assertEqual(window,{'earliest_captured_at':base['captured_at'],
                        'latest_captured_at':onward['captured_at'],'maximum_skew_seconds':10})
                    single=(await call('inspect_snapshot',snapshot_id=key,section='flight_map',
                        flight_route_search={'destination_node_id':20,'maximum_capture_skew_seconds':0}
                        ))['flight_route_search']['selected_capture_window']
                    self.assertEqual(single['earliest_captured_at'],single['latest_captured_at'])
                    for rejected in [
                        {**bounded,'maximum_capture_skew_seconds':9},
                        {**bounded,'destination_node_id':20,'onward_snapshot_ids':[fractional_key]},
                        {**bounded,'maximum_capture_skew_seconds':-1},
                    ]:
                        self.assertTrue((await session.call_tool('inspect_snapshot',{
                            'snapshot_id':key,'section':'flight_map','flight_route_search':rejected,
                        })).isError)
                    for destination in [{},{'destination_name':' '},{'destination_name':'é'*129},
                                        {'destination_name':'Target','destination_node_id':30}]:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,
                            'section':'flight_map','flight_route_search':destination})
                        self.assertTrue(response.isError)
                    old_search=await call('inspect_snapshot',snapshot_id=old,section='flight_map',flight_route_search={'destination_node_id':20})
                    self.assertNotIn('observed_route_node_ids',old_search['flight_route_search']['legs'][0])
                    self.assertEqual(old_search['flight_route_search']['total_fare_copper'],100)
                    itinerary=(await call('inspect_snapshot',snapshot_id=key,section='flight_map',flight_node_id=20,
                        flight_onward_legs=[{'snapshot_id':next_key,'destination_node_id':40}]))['flight_itinerary']
                    self.assertEqual(len(itinerary['legs']),2)
                    self.assertEqual(itinerary['total_fare_copper'],150)
                    self.assertEqual(itinerary['legs'][1]['observed_route_node_ids'],{'1':20,'2':40})
                    listed=await call('list_snapshots',family='forever',flight_destination_node_id=20)
                    self.assertEqual(len(listed['snapshots']),2)
                    match=next(row for row in listed['snapshots'] if row['snapshot_id']==key)['flight_match']
                    self.assertEqual(match['observed_route_node_ids'],path)
                    named=await call('list_snapshots',family='forever',flight_destination_name='  tArGeT  ',
                        flight_origin_node_id=10,flight_wallet_reserve_copper=100)
                    self.assertEqual({row['snapshot_id'] for row in named['snapshots']},{key,old})
                    self.assertEqual(next(row for row in named['snapshots'] if row['snapshot_id']==key)
                        ['flight_match']['observed_route_node_ids'],path)
                    for name in ['Tar','Transit','Origin']:
                        rejected=await call('list_snapshots',family='forever',flight_destination_name=name)
                        self.assertEqual(rejected['snapshots'],[])
                    for family,name in [('retail','Target'),('forever',' '),('forever','é'*129)]:
                        response=await session.call_tool('list_snapshots',{
                            'family':family,'flight_destination_name':name})
                        self.assertTrue(response.isError)
                    no_wallet = deepcopy(base)
                    no_wallet['sections'].pop('wallet')
                    no_fare = deepcopy(legacy)
                    no_fare['sections']['flight_map']['entries']['2'].pop('fare_copper')
                    poor = deepcopy(base)
                    poor['sections']['wallet']['copper'] = 99
                    for capture in [no_wallet, no_fare, poor]:
                        store.ingest(capture)
                    unknown_fare=(await call('inspect_snapshot',snapshot_id=store.ingest(no_fare),
                        section='flight_map',flight_route_search={'destination_name':'Target'}))['flight_route_search']
                    self.assertEqual(unknown_fare['status'],'no_fully_priced_route')
                    self.assertEqual(unknown_fare['saved_path_evidence'],{
                        'reachable_path_observed':True,
                        'fully_priced_path_observed_without_budget':False,'maximum_legs':8,
                        'minimum_fully_priced_fare_copper':None,'effective_fare_ceiling_copper':None,
                        'shortfall_vs_effective_ceiling_copper':None})
                    witness=unknown_fare['saved_path_witness']
                    self.assertIsNone(witness['total_fare_copper'])
                    self.assertEqual([leg['destination_node_id'] for leg in witness['legs']],[20])
                    self.assertIsNone(witness['legs'][0]['fare_copper'])
                    over_budget=(await call('inspect_snapshot',snapshot_id=key,section='flight_map',
                        flight_route_search={'destination_node_id':20,'maximum_total_fare_copper':99}))['flight_route_search']
                    self.assertEqual(over_budget['status'],'no_route_within_fare_ceiling')
                    self.assertEqual(over_budget['saved_path_witness']['total_fare_copper'],100)
                    self.assertEqual(over_budget['saved_path_witness']['legs'][0]['snapshot_id'],key)
                    capped_path=(await call('inspect_snapshot',snapshot_id=key,section='flight_map',
                        flight_route_search={'destination_name':'Further','onward_snapshot_ids':[next_key],
                                             'max_legs':1}))['flight_route_search']
                    self.assertEqual(capped_path['saved_path_evidence'],{
                        'reachable_path_observed':False,
                        'fully_priced_path_observed_without_budget':False,'maximum_legs':1,
                        'minimum_fully_priced_fare_copper':None,'effective_fare_ceiling_copper':None,
                        'shortfall_vs_effective_ceiling_copper':None})
                    self.assertNotIn('saved_path_witness',capped_path)
                    for reserve, expected in [(0, {key,old}), (100, {key,old}), (101, set())]:
                        result = await call('list_snapshots', family='forever',
                            flight_destination_node_id=20, flight_wallet_reserve_copper=reserve)
                        self.assertEqual({row['snapshot_id'] for row in result['snapshots']}, expected)
                        for row in result['snapshots']:
                            self.assertEqual(row['flight_match']['remaining_wallet_copper'], 100)
                            self.assertEqual(row['flight_match']['required_wallet_reserve_copper'], reserve)
                    limited = await call('list_snapshots', family='forever', flight_destination_node_id=20,
                        flight_wallet_reserve_copper=100, flight_max_fare_copper=99)
                    self.assertEqual(limited['snapshots'], [])
                    for args in [
                        {'family':'forever','flight_wallet_reserve_copper':0},
                        {'family':'forever','flight_wallet_reserve_copper':10**13,'flight_destination_node_id':20},
                        {'family':'classic','flight_wallet_reserve_copper':0,'flight_destination_node_id':20},
                    ]:
                        self.assertTrue((await session.call_tool('list_snapshots', args)).isError)
                    longest_data=await call('inspect_snapshot',snapshot_id=longest_key,section='flight_map',flight_node_id=65)
                    self.assertEqual(len(longest_data['data']['entries']['65']['observed_route_node_ids']),65)
                    direct=deepcopy(base)
                    direct_path={'1':10,'2':20}
                    direct['sections']['flight_map']['entries']['2']['observed_route_node_ids']=direct_path
                    direct_key=store.ingest(direct)
                    other_map=deepcopy(direct)
                    other_map['sections']['flight_map']['map_id']=2
                    for target,expected_path,changed in [
                            (key,path,False),(direct_key,direct_path,True),
                            (old,None,None),(store.ingest(other_map),direct_path,None)]:
                        result=await call('compare_snapshots',before_id=key,after_id=target,
                            flight_destination_node_id=20)
                        route=result['flight_destination_comparison']
                        self.assertEqual(route['before']['observed_route_node_ids'],path)
                        self.assertEqual(route['after'].get('observed_route_node_ids'),expected_path)
                        self.assertIs(route['observed_route_changed'],changed)
                        self.assertEqual(route['before']['remaining_wallet_after_fare_copper'],100)
                        self.assertEqual(route['after']['wallet_shortfall_copper'],0)
                        for field in ['remaining_wallet_after_fare_copper_delta','wallet_shortfall_copper_delta']:
                            self.assertEqual(route[field],0 if route['same_map_and_origin'] else None)
                    no_routes=await call('compare_snapshots',before_id=old,after_id=old,
                        flight_destination_node_id=20)
                    self.assertNotIn('observed_route_changed',no_routes['flight_destination_comparison'])
                    self.assertNotIn('wallet_reserve_changes',no_routes['flight_destination_comparison'])
                    tight = deepcopy(base)
                    tight['sections']['wallet']['copper'] = 180
                    tight_key = store.ingest(tight)
                    for target, reserve, expected in [
                        (tight_key, 50, (False, 0, -20)),
                        (tight_key, 100, (True, 20, None)),
                        (store.ingest(other_map), 50, (None, None, None)),
                    ]:
                        result = await call('compare_snapshots', before_id=key, after_id=target,
                            flight_destination_node_id=20, flight_wallet_reserve_copper=reserve)
                        self.assertEqual(result['flight_destination_comparison']['wallet_reserve_changes'],
                            dict(zip(('affordability_changed','shortfall_copper_delta','spendable_copper_delta'),
                                     expected, strict=True)))
                    for reserve, affordable, shortfall, spendable in [(0,True,0,100),(100,True,0,0),(101,False,1,None)]:
                        result = await call('compare_snapshots', before_id=key, after_id=old,
                            flight_destination_node_id=20, flight_wallet_reserve_copper=reserve)
                        route = result['flight_destination_comparison']
                        self.assertEqual(route['required_wallet_reserve_copper'],reserve)
                        for side in ('before','after'):
                            self.assertTrue(route[side]['affordable'])
                            self.assertEqual(route[side]['wallet_reserve_projection'], {
                                'affordable':affordable,'shortfall_copper':shortfall,'spendable_copper':spendable,
                            })
                    for invalid in [{'flight_wallet_reserve_copper':0},
                                    {'flight_destination_node_id':20,'flight_wallet_reserve_copper':10**13}]:
                        reply = await session.call_tool('compare_snapshots', {
                            'before_id':key,'after_id':old,**invalid,
                        })
                        self.assertTrue(reply.isError,reply)
                    ambiguous=deepcopy(base)
                    ambiguous['character']['name']='Ambiguous'
                    ambiguous['sections']['flight_map']['entries']['4']={
                        'node_id':99,'name':'TARGET','state':'reachable','map_transition':False,'fare_copper':150}
                    ambiguous_key=store.ingest(ambiguous)
                    scope={'family':'forever','flight_destination_name':'Target',
                           'character':ambiguous['character'],'flight_max_fare_copper':100}
                    self.assertEqual((await call('list_snapshots',**scope))['snapshots'],[])
                    narrowed=await call('list_snapshots',**scope,flight_destination_node_id=20)
                    self.assertEqual([row['snapshot_id'] for row in narrowed['snapshots']],[ambiguous_key])
                    route_scope={'destination_name':'Target','maximum_total_fare_copper':100}
                    response=await session.call_tool('inspect_snapshot',{'snapshot_id':ambiguous_key,
                        'section':'flight_map','flight_route_search':route_scope})
                    self.assertTrue(response.isError)
                    disambiguated=await call('inspect_snapshot',snapshot_id=ambiguous_key,section='flight_map',
                        flight_route_search={**route_scope,'destination_node_id':20})
                    self.assertEqual(disambiguated['flight_route_search']['destination_node_id'],20)
                    expensive_direct=deepcopy(base)
                    expensive_direct['sections']['flight_map']['entries']['4']={
                        'node_id':40,'name':'Further','state':'reachable','map_transition':False,'fare_copper':180}
                    budgeted=await call('inspect_snapshot',snapshot_id=store.ingest(expensive_direct),
                        section='flight_map',flight_wallet_reserve_copper=70,
                        flight_route_search={'destination_name':'Further','onward_snapshot_ids':[next_key],
                            'objective':'fewest_legs','require_affordable':True})
                    evidence=budgeted['flight_route_search']['saved_path_evidence']
                    self.assertEqual(evidence['minimum_fully_priced_fare_copper'],150)
                    self.assertEqual(evidence['effective_fare_ceiling_copper'],130)
                    self.assertEqual(evidence['shortfall_vs_effective_ceiling_copper'],20)
                    avoid={'destination_name':'Further','onward_snapshot_ids':[next_key],
                           'excluded_landing_node_ids':[20]}
                    rerouted=(await call('inspect_snapshot',snapshot_id=store.ingest(expensive_direct),
                        section='flight_map',flight_route_search=avoid))['flight_route_search']
                    self.assertEqual(rerouted['excluded_landing_node_ids'],[20])
                    self.assertEqual((rerouted['total_fare_copper'],len(rerouted['legs'])),(180,1))
                    constrained=(await call('inspect_snapshot',snapshot_id=store.ingest(expensive_direct),
                        section='flight_map',flight_route_search={**avoid,'maximum_total_fare_copper':160}
                        ))['flight_route_search']
                    self.assertEqual(constrained['saved_path_witness']['total_fare_copper'],180)
                    blocked=(await call('inspect_snapshot',snapshot_id=key,section='flight_map',
                        flight_route_search=avoid))['flight_route_search']
                    self.assertFalse(blocked['saved_path_evidence']['reachable_path_observed'])
                    self.assertNotIn('saved_path_witness',blocked)
                    flyover=(await call('inspect_snapshot',snapshot_id=key,section='flight_map',
                        flight_route_search={'destination_node_id':20,'excluded_landing_node_ids':[30]}
                        ))['flight_route_search']
                    self.assertEqual(flyover['legs'][0]['observed_route_node_ids'],path)
                    preview_search={'destination_name':'Target','excluded_flyover_node_ids':[30]}
                    for capture in [key,old]:
                        avoided=(await call('inspect_snapshot',snapshot_id=capture,section='flight_map',
                            flight_route_search=preview_search))['flight_route_search']
                        self.assertEqual(avoided['excluded_flyover_node_ids'],[30])
                        self.assertFalse(avoided['saved_path_evidence']['reachable_path_observed'])
                        self.assertNotIn('saved_path_witness',avoided)
                    permitted=(await call('inspect_snapshot',snapshot_id=key,section='flight_map',
                        flight_route_search={**preview_search,'excluded_flyover_node_ids':[20,10],
                            'maximum_total_fare_copper':99}))['flight_route_search']
                    self.assertEqual(permitted['excluded_flyover_node_ids'],[10,20])
                    self.assertEqual(permitted['saved_path_witness']['total_fare_copper'],100)
                    for ids in [[30,30],[0],list(range(100,133))]:
                        self.assertTrue((await session.call_tool('inspect_snapshot',{
                            'snapshot_id':key,'section':'flight_map',
                            'flight_route_search':{**preview_search,'excluded_flyover_node_ids':ids},
                        })).isError)
                    for ids in [[10],[40],[20,20],[0],list(range(100,133))]:
                        self.assertTrue((await session.call_tool('inspect_snapshot',{
                            'snapshot_id':key,'section':'flight_map',
                            'flight_route_search':{**avoid,'excluded_landing_node_ids':ids},
                        })).isError)
                    for bad in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'flight_map'})
                        self.assertTrue(response.isError)
                        self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
