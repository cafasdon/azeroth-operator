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
        longest=deepcopy(base)
        longest_rows={str(i):{'node_id':i,'name':'Node','state':('current' if i==1 else 'reachable' if i==65 else 'unreachable'),
                             'map_transition':False} for i in range(1,66)}
        longest_rows['65']['fare_copper']=100
        longest_rows['65']['observed_route_node_ids']={str(i):i for i in range(1,66)}
        longest['sections']['flight_map']['entries']=longest_rows
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));key=store.ingest(base);old=store.ingest(legacy);next_key=store.ingest(onward)
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
                    search=await call('inspect_snapshot',snapshot_id=key,section='flight_map',flight_route_search={'destination_node_id':20})
                    plan=search['flight_route_search']
                    for limit,status in [(99,'no_route_within_fare_ceiling'),(100,'found')]:
                        bounded=await call('inspect_snapshot',snapshot_id=key,section='flight_map',
                            flight_route_search={'destination_node_id':20,'maximum_total_fare_copper':limit})
                        self.assertEqual(bounded['flight_route_search']['status'],status)
                    self.assertEqual(len(plan['legs']),1)
                    self.assertEqual(plan['legs'][0]['observed_route_node_ids'],path)
                    self.assertEqual(plan['total_fare_copper'],100)
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
                    no_wallet = deepcopy(base)
                    no_wallet['sections'].pop('wallet')
                    no_fare = deepcopy(legacy)
                    no_fare['sections']['flight_map']['entries']['2'].pop('fare_copper')
                    poor = deepcopy(base)
                    poor['sections']['wallet']['copper'] = 99
                    for capture in [no_wallet, no_fare, poor]:
                        store.ingest(capture)
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
                    for bad in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'flight_map'})
                        self.assertTrue(response.isError)
                        self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
