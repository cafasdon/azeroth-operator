"""Bounded provider area-percent locations stay distinct from native map/phase proof."""
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
from test_chain_captured_sources import refs
from test_recipe_reagent_chain import fixed


def points(entries,total=None):
    count=len(entries) if total is None else total
    return {'reported_point_count':count,'truncated':count>16,'entries':{str(i):p for i,p in enumerate(entries,1)}}


class RecipeSourceLocationTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_parity_scoped_chain_locations_limits_and_provenance(self):
        binary=(Path(__file__).resolve().parents[1]/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['bags']={'0':{'1':{'item_id':10,'count':1}}}
        base['sections']['profession_recipe_materials']={'coverage':'reported_known_spell_schematics',
            'profession_id':164,'entries':{'101':fixed(100,{'10':3})}}
        coordinate={'kind':'coordinate','area_id':12,'x_percent':0,'y_percent':100,'provider_phase_id':1000}
        cache={'coordinate_system':'questiedb_area_percent','reported_npc_count':4,'reported_object_count':1,
            'npcs':{'1':{'status':'reported','spawns':points([coordinate])},
                '2':{'status':'not_in_provider'},'3':{'status':'unavailable'},'99':{'status':'reported'}},
            'objects':{'5':{'status':'reported','spawns':points([{'kind':'instance_presence','area_id':1581}])}}}
        source={'coverage':'first_reported_fixed_recipe_reagents','provider':'questiedb','provider_mode':'baked',
            'provider_contract_version':4,'profession_id':164,'reported_item_count':2,'entries':{
                '10':{'status':'reported','npc_drops':refs([1,2]),'vendors':refs([2,3]),'object_drops':refs([5])},
                '20':{'status':'reported','npc_drops':refs([99])}},'entity_locations':cache}
        base['sections']['recipe_item_sources']=source
        bad=[]
        for value in [None,{},[],{**cache,'coordinate_system':'ui_map_normalized'},
            {**cache,'reported_npc_count':3},{**cache,'reported_npc_count':4.0},
            {**cache,'npcs':{}},{**cache,'private':'PRIVATE'}]:
            saved=deepcopy(base);saved['sections']['recipe_item_sources']['entity_locations']=value;bad.append(saved)
        for row in [None,{}, {'status':'not_captured'},{'status':'unavailable','spawns':{}},
            {'status':'reported','spawns':None},{'status':'reported','private':'PRIVATE'}]:
            saved=deepcopy(base);saved['sections']['recipe_item_sources']['entity_locations']['npcs']['1']=row;bad.append(saved)
        badpoints=[{**coordinate,key:value} for key,value in [
            ('x_percent',-1),('x_percent',101),('x_percent',True),('y_percent',None),
            ('area_id',0),('area_id',1.0),('area_id',4294967296),
            ('provider_phase_id',0),('provider_phase_id',True),('provider_phase_id',None),
            ('provider_phase_id',4294967296),('kind','unknown'),('ui_map_id',37),('private','PRIVATE')]]
        badpoints += [{'kind':'instance_presence','area_id':12,'x_percent':-1},
            {'kind':'instance_presence','area_id':12,'provider_phase_id':1}]
        for point in badpoints:
            saved=deepcopy(base);saved['sections']['recipe_item_sources']['entity_locations']['npcs']['1']['spawns']=points([point]);bad.append(saved)
        for spawn in [{'reported_point_count':4097,'truncated':True,'entries':{}},
            {'reported_point_count':1,'truncated':True,'entries':{'1':coordinate}},
            {'reported_point_count':1,'truncated':False,'entries':{'01':coordinate}},
            {'reported_point_count':1,'truncated':False,'entries':{}},
            points([coordinate]*17,17)]:
            saved=deepcopy(base);saved['sections']['recipe_item_sources']['entity_locations']['npcs']['1']['spawns']=spawn;bad.append(saved)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));key=store.ingest(base)
            legacy=deepcopy(base);legacy['sections']['recipe_item_sources'].pop('entity_locations');legacy_key=store.ingest(legacy)
            cap=deepcopy(base);s=cap['sections']['recipe_item_sources'];s['reported_item_count']=64
            s['entries']={str(i):{'status':'reported','npc_drops':refs(range((i-1)*8+1,i*8+1)),
                'vendors':refs(range(512+(i-1)*8+1,512+i*8+1)),
                'object_drops':refs(range((i-1)*8+1,i*8+1))} for i in range(1,65)}
            maximum={'status':'reported','spawns':points([{**coordinate,'area_id':4294967295,
                'provider_phase_id':4294967295,'x_percent':i/2} for i in range(16)],4096)}
            s['entity_locations']={'coordinate_system':cache['coordinate_system'],'reported_npc_count':1024,
                'reported_object_count':512,'npcs':{str(i):deepcopy(maximum) for i in range(1,33)},
                'objects':{str(i):deepcopy(maximum) for i in range(1,33)}}
            cap_key=store.ingest(cap)
            clipped=deepcopy(base);s=clipped['sections']['recipe_item_sources'];s['reported_item_count']=5
            s['entries']={'10':{'status':'reported','vendors':refs(range(33,41))},
                **{str(20+i):{'status':'reported','npc_drops':refs(range(i*8+1,i*8+9))} for i in range(4)}}
            s['entity_locations']={'coordinate_system':cache['coordinate_system'],'reported_npc_count':40,
                'reported_object_count':0,'npcs':{str(i):{'status':'reported'} for i in range(1,33)},'objects':{}}
            clipped_key=store.ingest(clipped)
            older=deepcopy(base);older['captured_at']='2024-12-31T00:00:00Z'
            older['sections']['recipe_item_sources']['entity_locations']['npcs']['1']['spawns']['entries']['1']['x_percent']=25
            older_key=store.ingest(older)
            wrong=deepcopy(older);wrong['character']['name']='PRIVATE_CHARACTER';wrong_key=store.ingest(wrong)
            badkeys=[]
            for saved in bad:
                with self.assertRaises(ValueError):validate_snapshot(saved)
                raw=json.dumps(saved,sort_keys=True,separators=(',',':')).encode();id=hashlib.sha256(raw).hexdigest()
                (Path(folder)/f'{id}.json').write_bytes(raw);badkeys.append(id)
            before={p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args);self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));return None if error else json.loads(reply.content[0].text)
                    async def chain(id=key,**extra):
                        result=await call('review_forever_recipe',snapshot_id=id,recipe_id=101,crafts=1,
                            suggest_reagent_chain=True,reagent_chain_captured_sources=True,**extra)
                        return result['reagent_chain']['captured_sources']
                    for id,expected in [(key,source),(cap_key,cap['sections']['recipe_item_sources'])]:
                        inspected=await call('inspect_snapshot',snapshot_id=id,section='recipe_item_sources')
                        self.assertEqual(inspected['data'],expected)
                    result=await chain();location=result['entity_locations']
                    self.assertEqual(set(location['npcs']),{'1','2','3'});self.assertNotIn('99',location['npcs'])
                    self.assertEqual(location['npcs']['1'],cache['npcs']['1']);self.assertEqual(location['objects'],cache['objects'])
                    for field in ['coordinate_frame_verified','phase_visibility_verified','native_spawn_verified','routing_verified']:
                        self.assertFalse(location[field])
                    self.assertEqual(result['records'][0]['quantity'],2)
                    self.assertNotIn('entity_locations',await chain(legacy_key))
                    clipped_result=(await chain(clipped_key))['entity_locations']
                    self.assertEqual(set(clipped_result['npcs']),set(map(str,range(33,41))))
                    self.assertTrue(all(row=={'status':'not_captured'} for row in clipped_result['npcs'].values()))
                    self.assertEqual(clipped_result['reported_npc_count'],40)
                    self.assertEqual(clipped_result['retained_npcs_count'],32)
                    historical=await chain(reagent_source_snapshot_id=older_key)
                    self.assertEqual(historical['captured_at'],older['captured_at'])
                    self.assertEqual(historical['entity_locations']['npcs']['1']['spawns']['entries']['1']['x_percent'],25)
                    await call('review_forever_recipe',error=True,snapshot_id=key,recipe_id=101,crafts=1,
                        suggest_reagent_chain=True,reagent_chain_captured_sources=True,reagent_source_snapshot_id=wrong_key)
                    for id in badkeys:await call('inspect_snapshot',error=True,snapshot_id=id,section='recipe_item_sources')
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__':unittest.main()
