"""Flight-map coordinates annotate saved routes without changing fares or reachability."""
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


def without_positions(value):
    if isinstance(value,dict):return {k:without_positions(v) for k,v in value.items()
        if k not in {'map_position','origin_map_position','destination_map_position','snapshot_id'}}
    if isinstance(value,list):return [without_positions(v) for v in value]
    return value


class FlightPositionTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_positions_and_route_context(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            base['sections']['wallet']={'copper':1000}
            base['sections']['flight_map']={'map_id':1,'entries':{
                '1':{'node_id':10,'name':'Origin','state':'current','map_transition':False,'map_position':{'x':0,'y':1}},
                '2':{'node_id':20,'name':'Destination','state':'reachable','map_transition':False,'fare_copper':100,
                     'map_position':{'x':0.25,'y':0.75}},
                '3':{'node_id':99,'name':'Layer','state':'reachable','map_transition':True,'map_position':{'x':0.5,'y':0.5}}}}
            positioned=store.ingest(base)
            legacy=deepcopy(base)
            for row in legacy['sections']['flight_map']['entries'].values():row.pop('map_position')
            legacy_id=store.ingest(legacy)
            onward=deepcopy(base);onward['sections']['flight_map']={'map_id':2,'entries':{
                '1':{'node_id':20,'name':'Destination','state':'current','map_transition':False,'map_position':{'x':0.8,'y':0.2}},
                '2':{'node_id':30,'name':'Further','state':'reachable','map_transition':False,'fare_copper':50,
                     'map_position':{'x':0.1,'y':0.9}}}}
            onward_id=store.ingest(onward)
            malformed=[None,{}, {'x':0}, {'x':0,'y':0,'guid':'PRIVATE'}]
            for invalid in [True,-0.1,1.1,'0',None]:malformed.append({'x':invalid,'y':0})
            bad_keys=[]
            for position in malformed:
                bad=deepcopy(base);bad['sections']['flight_map']['entries']['2']['map_position']=position
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(raw).hexdigest();(Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def inspect(key,**arguments):
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'flight_map',**arguments})
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)
                    data=(await inspect(positioned,flight_node_id=20))['data']['entries']
                    self.assertEqual(set(data),{'2'})
                    self.assertEqual(data['2']['map_position'],{'x':0.25,'y':0.75})
                    query={'flight_route_search':{'destination_node_id':20}}
                    plan=(await inspect(positioned,**query))['flight_route_search']
                    old_plan=(await inspect(legacy_id,**query))['flight_route_search']
                    self.assertEqual(without_positions(plan),without_positions(old_plan))
                    leg=plan['legs'][0]
                    self.assertEqual(leg['origin_map_position'],{'x':0,'y':1})
                    self.assertEqual(leg['destination_map_position'],{'x':0.25,'y':0.75})
                    self.assertNotIn('origin_map_position',old_plan['legs'][0])
                    result=await inspect(positioned,flight_node_id=20,
                        flight_onward_legs=[{'snapshot_id':onward_id,'destination_node_id':30}])
                    legs=result['flight_itinerary']['legs']
                    self.assertEqual([leg['map_id'] for leg in legs],[1,2])
                    self.assertEqual(legs[0]['destination_map_position'],{'x':0.25,'y':0.75})
                    self.assertEqual(legs[1]['origin_map_position'],{'x':0.8,'y':0.2})
                    self.assertEqual(result['flight_itinerary']['total_fare_copper'],150)
                    transition=(await inspect(positioned,flight_route_search={'destination_node_id':99}))['flight_route_search']
                    self.assertEqual(transition['status'],'no_fully_priced_route')
                    for key in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'flight_map'})
                        self.assertTrue(response.isError)
                        self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
