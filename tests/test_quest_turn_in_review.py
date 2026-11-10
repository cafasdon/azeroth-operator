"""Saved turn-in grouping through the real MCP; no quest actions or world inference."""
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


def group(ids, count=None):
    count=len(ids) if count is None else count
    return {'entries':{str(i):True for i in ids},'distinct_id_count':count,'truncated':count>16}


def sample():
    value=fixture();value['client'].update(family='forever',build='70862',interface=16001)
    value['sections']['quests']={str(i):{'title':'PRIVATE_QUEST_TITLE','level':1} for i in range(1,8)}
    for i in [1,2,7]:value['sections']['quests'][str(i)]['ready_for_turn_in']=True
    for i in [3,6]:value['sections']['quests'][str(i)]['ready_for_turn_in']=False
    rows={str(i):{'status':'reported'} for i in range(1,8)}
    rows['1']['finished_by']={'npcs':group([2,10]),'objects':group([10])}
    rows['2']['finished_by']={'npcs':group([10])}
    rows['3']['finished_by']={'npcs':group([2])}
    rows['4']['finished_by']={'npcs':group([10])}
    rows['5']={'status':'unavailable'};rows['6']={'status':'not_in_provider'}
    rows['7']['finished_by']={'npcs':group(list(range(100,116)),17)}
    value['sections']['quest_giver_references']={'coverage':'first_saved_quest_ids','provider':'questiedb',
        'provider_mode':'source','provider_contract_version':1,'reported_quest_count':7,'entries':rows}
    return value


class QuestTurnInReviewTests(unittest.IsolatedAsyncioTestCase):
    async def test_grouping_filters_pages_unknowns_and_owner_boundary(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder));value=sample();key=store.ingest(value)
            missing=deepcopy(value);del missing['sections']['quest_giver_references'];missing_key=store.ingest(missing)
            neither=deepcopy(missing);del neither['sections']['quests'];neither_key=store.ingest(neither)
            empty=deepcopy(value);empty['sections']['quests']={}
            empty['sections']['quest_giver_references'].update(reported_quest_count=0,entries={})
            empty_key=store.ingest(empty)
            many=deepcopy(value);many['sections']['quests']={str(i):{'title':'Fixture','level':1} for i in range(1,67)}
            many['sections']['quest_giver_references'].update(reported_quest_count=66,
                entries={str(i):{'status':'reported','finished_by':{}} for i in range(1,65)})
            many_key=store.ingest(many)
            retail_key=store.ingest(fixture())
            other=deepcopy(value);other['character']['name']='PRIVATE_OTHER_OWNER'
            other_key=SnapshotStore(Path(foreign)).ingest(other)
            original={p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,
                    'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(snapshot=key,page=None,error=False,section='quest_giver_references',**extra):
                        args={'snapshot_id':snapshot,'section':section,**extra}
                        if page is not None:args['quest_turn_in_review']=page
                        reply=await session.call_tool('inspect_snapshot',args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    raw=await call();self.assertEqual(raw['data'],value['sections']['quest_giver_references'])
                    result=(await call(page={}))['data']
                    self.assertEqual(result['status'],'reviewed');self.assertEqual(result['selected_quest_count'],7)
                    self.assertEqual(result['ready_flag_unknown_quest_count'],2)
                    self.assertEqual(result['reference_status_counts'],{'reported':5,'unavailable':1,'not_in_provider':1,'not_captured':0})
                    self.assertEqual(result['matching_group_count'],19)
                    self.assertEqual([(g['giver_kind'],g['giver_id']) for g in result['groups']],
                        [('npc',2),('npc',10)]+[('npc',i) for i in range(100,116)]+[('object',10)])
                    npc=result['groups'][1]
                    self.assertEqual(npc['quest_ids'],[1,2,4])
                    self.assertEqual(npc['ready_for_turn_in_quest_ids'],[1,2])
                    self.assertEqual(npc['not_ready_quest_ids'],[]);self.assertEqual(npc['unknown_ready_quest_ids'],[4])
                    self.assertEqual(result['ungrouped_quest_ids'],[5,6])
                    self.assertEqual(result['selected_reference_truncated_group_count'],1)
                    for flag in ['live','game_execution','native_identity_verified','current_readiness_verified',
                            'turn_in_eligibility_verified','optimal_route_verified','complete_finisher_coverage_verified']:
                        self.assertFalse(result[flag])
                    combined=[]
                    for offset in range(0,20,3):
                        page=(await call(page={'offset':offset,'limit':3}))['data']
                        combined.extend(page['groups']);self.assertEqual(page['has_more'],offset+3<19)
                    self.assertEqual(combined,result['groups'])
                    self.assertEqual((await call(page={'offset':4294967295}))['data']['groups'],[])
                    ready=(await call(page={'ready_for_turn_in':True,'giver_kind':'npc','giver_id':10}))['data']
                    self.assertEqual(ready['groups'][0]['quest_ids'],[1,2])
                    not_ready=(await call(page={'ready_for_turn_in':False}))['data']
                    self.assertEqual(not_ready['selected_quest_count'],2)
                    self.assertEqual(not_ready['groups'][0]['quest_ids'],[3])
                    self.assertEqual(not_ready['ungrouped_quest_ids'],[6])
                    objects=(await call(page={'giver_kind':'object','giver_id':10,'quest_ids':[1,2,4]}))['data']
                    self.assertEqual(objects['groups'][0]['quest_ids'],[1])
                    self.assertEqual((await call(page={'quest_ids':[999]}))['data']['selected_quest_count'],0)
                    self.assertEqual((await call(snapshot=missing_key,page={}))['data']['status'],'references_not_recorded')
                    absent=(await call(snapshot=neither_key,page={}))['data']
                    self.assertEqual(absent['status'],'source_quests_not_recorded');self.assertIsNone(absent['groups'])
                    zero=(await call(snapshot=empty_key,page={}))['data']
                    self.assertEqual(zero['groups'],[]);self.assertEqual(zero['matching_group_count'],0)
                    unqueried=(await call(snapshot=many_key,page={'quest_ids':[65,66]}))['data']
                    self.assertEqual(unqueried['reference_status_counts']['not_captured'],2)
                    self.assertEqual(unqueried['ungrouped_quest_ids'],[65,66])
                    full=(await call(snapshot=many_key,page={}))['data']
                    self.assertEqual(full['ungrouped_selected_quest_count'],66)
                    self.assertEqual(len(full['ungrouped_quest_ids']),64);self.assertTrue(full['ungrouped_quest_ids_truncated'])
                    for bad in [{'limit':0},{'limit':51},{'offset':-1},{'offset':True},
                            {'ready_for_turn_in':0},{'giver_id':10},{'giver_kind':'item'},
                            {'quest_ids':[]},{'quest_ids':[1,1]},{'quest_ids':[0]},
                            {'quest_ids':[True]},{'bogus':True}]:
                        await call(page=bad,error=True)
                    await call(page={},section='quests',error=True)
                    await call(page={},coverage_sections=['quests'],error=True)
                    await call(snapshot=retail_key,page={},error=True)
                    await call(snapshot=other_key,page={},error=True)
            self.assertEqual(original,{p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')})
