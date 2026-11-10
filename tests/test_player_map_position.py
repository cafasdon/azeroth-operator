"""Private saved map diagram coordinates never become current position or a route."""
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


def sample(x=.25,y=.75):
    value=fixture();value['client'].update(family='forever',build='70862',interface=16001)
    value['sections']['player_map_position']={'coverage':'reported_player_map_position','map_id':1,'x':x,'y':y}
    return value


def invalid_samples():
    values=[]
    for field,bads in {'map_id':[None,False,0,-1,1.5,2**32,'1'],
            'x':[None,True,-.01,1.01,'PRIVATE',float('nan'),float('inf')],
            'y':[None,False,-1,2,'0'], 'coverage':[None,'PRIVATE']}.items():
        for bad in bads:
            value=sample();value['sections']['player_map_position'][field]=bad;values.append(value)
    for field in ['coverage','map_id','x','y']:
        value=sample();del value['sections']['player_map_position'][field];values.append(value)
    value=sample();value['sections']['player_map_position']['PRIVATE']=1;values.append(value)
    for family in ['retail','classic']:
        value=sample();value['client']['family']=family;values.append(value)
    return values


class PlayerMapReaderTests(unittest.TestCase):
    def test_strict_reader_preserves_endpoints_and_refuses_unknown_values(self):
        for x,y in [(0,0),(1,1),(.25,.75)]:
            value=sample(x,y);before=deepcopy(value)
            self.assertIs(validate_snapshot(value),value);self.assertEqual(value,before)
        for value in invalid_samples():
            with self.assertRaises(ValueError):validate_snapshot(value)


class PlayerMapMCPTests(unittest.IsolatedAsyncioTestCase):
    async def test_owner_local_raw_summary_and_explicit_target_preview(self):
        binary=Path(__file__).resolve().parents[1]/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder));key=store.ingest(sample())
            endpoint_key=store.ingest(sample(0,1))
            missing=sample();del missing['sections']['player_map_position'];missing_key=store.ingest(missing)
            unavailable=deepcopy(missing);unavailable['unavailable']={'player_map_position':'PRIVATE_REASON'}
            unavailable_key=store.ingest(unavailable)
            other=sample();other['character']['name']='PRIVATE_FOREIGN_OWNER'
            foreign_key=SnapshotStore(Path(foreign)).ingest(other)
            invalid_keys=[]
            for value in invalid_samples():
                raw=json.dumps(value,sort_keys=True).encode();bad=hashlib.sha256(raw).hexdigest()
                (Path(folder)/(bad+'.json')).write_bytes(raw);invalid_keys.append(bad)
            before={p:p.read_bytes() for root in [folder,foreign] for p in Path(root).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(snapshot=key,error=False,**extra):
                        reply=await session.call_tool('inspect_snapshot',
                            {'snapshot_id':snapshot,'section':'player_map_position',**extra})
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE_FOREIGN_OWNER',str(reply));self.assertNotIn(foreign,str(reply))
                        if error:self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    raw=await call();self.assertEqual(raw['data'],sample()['sections']['player_map_position'])
                    target={'map_id':1,'x':.55,'y':.35}
                    result=(await call(player_map_target=target))['data']
                    self.assertEqual(result['status'],'same_map_diagram_only')
                    self.assertAlmostEqual(result['diagram_delta']['x'],.3)
                    self.assertAlmostEqual(result['diagram_delta']['y'],-.4)
                    self.assertAlmostEqual(result['diagram_delta']['euclidean_separation'],.5)
                    self.assertEqual(result['diagram_delta']['units'],'normalised_map_axes_not_yards')
                    for flag in ['current_position_verified','active_character_verified','world_distance_verified',
                            'travel_route_verified','game_execution','live']:self.assertFalse(result[flag])
                    zero=(await call(player_map_target={'map_id':1,'x':.25,'y':.75}))['data']
                    self.assertEqual(zero['diagram_delta']['euclidean_separation'],0)
                    endpoint=(await call(snapshot=endpoint_key,player_map_target={'map_id':1,'x':1,'y':0}))['data']
                    self.assertAlmostEqual(endpoint['diagram_delta']['euclidean_separation'],2**.5)
                    different=(await call(player_map_target={**target,'map_id':2}))['data']
                    self.assertEqual(different['status'],'different_map');self.assertIsNone(different['diagram_delta'])
                    for saved,status in [(missing_key,'not_recorded'),(unavailable_key,'reported_unavailable')]:
                        unknown=(await call(snapshot=saved,player_map_target=target))['data']
                        self.assertNotIn('PRIVATE',json.dumps(unknown))
                        self.assertEqual(unknown['status'],status);self.assertIsNone(unknown['position'])
                        self.assertIsNone(unknown['diagram_delta'])
                    for malformed in [{**target,'map_id':0},{**target,'x':-1},{**target,'y':2},
                            {**target,'x':True},{**target,'x':'PRIVATE'}, {**target,'PRIVATE':1}, {}, []]:
                        await call(player_map_target=malformed,error=True)
                    await call(section='stats',player_map_target=target,error=True)
                    await call(coverage_sections=['player_map_position'],player_map_target=target,error=True)
                    for bad in [foreign_key,'../PRIVATE',*invalid_keys]:await call(snapshot=bad,error=True)
                    reply=await session.call_tool('character_summary',{'snapshot_id':key})
                    self.assertFalse(reply.isError)
                    summary=json.loads(reply.content[0].text)['player_map_position_observation']
                    self.assertEqual(summary['capture_status'],'observed')
                    self.assertEqual(summary['position'],raw['data']);self.assertFalse(summary['current_position_verified'])
            self.assertEqual(before,{p:p.read_bytes() for root in [folder,foreign] for p in Path(root).glob('*.json')})
