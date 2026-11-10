"""Real MCP stock-order witnesses compared with an independent reachable-state oracle."""
from collections import deque
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


def sample(seed=1):
    value=fixture();value['client'].update(family='forever',build='70862',interface=16001)
    value['sections'].update(bags={'0':{'1':{'item_id':30,'count':seed}}} if seed else {},
        character_bank={'6':{'1':{'item_id':30,'count':100}}},
        profession_recipe_materials={'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
            '100':{'status':'fixed_items','reagents':{'30':1},'output_item_id':10,'base_output_quantity':{'minimum':1,'maximum':1}},
            '200':{'status':'fixed_items','reagents':{'30':1},'output_item_id':20,'base_output_quantity':{'minimum':2,'maximum':3}},
            '300':{'status':'fixed_items','reagents':{'20':1},'output_item_id':30,'base_output_quantity':{'minimum':2,'maximum':2}}}})
    return value


class RecipeStockScheduleTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_backtracking_pages_budgets_and_reachable_state_reference(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder,tempfile.TemporaryDirectory(dir=root/'local-data') as foreign:
            store=SnapshotStore(Path(folder));key=store.ingest(sample())
            other=sample();other['character']['name']='PRIVATE_FOREIGN_OWNER'
            foreign_key=SnapshotStore(Path(foreign)).ingest(other)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    route=next(tool for tool in (await session.list_tools()).tools if tool.name=='plan_forever_recipe_restock')
                    def objects(value):
                        if isinstance(value,dict):
                            yield value
                            for child in value.values():yield from objects(child)
                        elif isinstance(value,list):
                            for child in value:yield from objects(child)
                    pages=[obj for obj in objects(route.inputSchema)
                        if {'state_budget','offset','limit'}<=set(obj.get('properties',{}))]
                    self.assertEqual(len(pages),1)
                    properties=pages[0]['properties']
                    for field,maximum,default in [('state_budget',50000,10000),('limit',50,20)]:
                        self.assertEqual(properties[field]['minimum'],1)
                        self.assertEqual(properties[field]['maximum'],maximum)
                        self.assertEqual(properties[field]['default'],default)
                    async def call(snapshot=key,page=None,error=False,**extra):
                        args={'snapshot_id':snapshot,'recipe_id':100,'target_bag_quantity':1,
                            'additional_targets':[{'recipe_id':200,'target_bag_quantity':1},{'recipe_id':300,'target_bag_quantity':1}],**extra}
                        if page is not None:args['stock_schedule']=page
                        reply=await session.call_tool('plan_forever_recipe_restock',args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE_FOREIGN_OWNER',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    plain=await call();self.assertNotIn('stock_schedule',plain)
                    self.assertEqual(await call(stock_schedule=None),plain)
                    report=await call(page={});schedule=report.pop('stock_schedule')
                    self.assertEqual(report.pop('same_recipe_intermediate_output_credit_scope'),'whole_recipe_material_ledger_only')
                    self.assertEqual(report,plain)
                    self.assertEqual(schedule['status'],'witness_found')
                    self.assertEqual([step['recipe_id'] for step in schedule['steps']],[200,300,100])
                    self.assertEqual(schedule['states_visited'],5)
                    for flag in ['native_craftability_verified','game_execution','live','current_inventory_verified',
                            'conditions_cooldowns_and_charges_checked','banks_mail_and_acquisitions_included']:
                        self.assertFalse(schedule[flag])
                    pages=[]
                    for offset in range(3):
                        piece=(await call(page={'offset':offset,'limit':1}))['stock_schedule']
                        self.assertEqual(piece['witness_sha256'],schedule['witness_sha256'])
                        pages.extend(piece['steps'])
                    self.assertEqual(pages,schedule['steps'])
                    self.assertEqual((await call(page={'offset':2**32-1}))['stock_schedule']['steps'],[])
                    unknown=(await call(page={'state_budget':2}))['stock_schedule']
                    self.assertEqual(unknown['status'],'state_budget_exhausted');self.assertFalse(unknown['decision_complete'])
                    self.assertEqual(unknown['steps'],[]);self.assertIsNone(unknown['witness_sha256'])
                    impossible=(await call(snapshot=store.ingest(sample(0)),page={}))['stock_schedule']
                    self.assertEqual(impossible['status'],'no_schedule_for_planned_counts')
                    self.assertEqual(impossible['states_visited'],1);self.assertTrue(impossible['decision_complete'])
                    for bad in [[],False,{'state_budget':0},{'state_budget':50001},{'state_budget':True},
                            {'offset':-1},{'offset':2**32},{'limit':0},{'limit':51},{'unknown':1}]:
                        await call(page=bad,error=True)
                    await call(snapshot=foreign_key,page={},error=True)
                    single=sample();single['sections']['bags']={'0':{'1':{'item_id':10,'count':1}}}
                    single['sections']['profession_recipe_materials']['entries']['100'].update(
                        reagents={'10':1},base_output_quantity={'minimum':2,'maximum':2})
                    single_key=store.ingest(single)
                    r=await call(snapshot=single_key,additional_targets=None,target_bag_quantity=3,page={})
                    self.assertEqual(r['recipe_review']['materials'][0]['shortfall_vs_bags'],1)
                    self.assertEqual(r['stock_schedule']['steps'],[{'step':1,'recipe_id':100,'crafts':2}])
                    self.assertTrue(r['stock_schedule']['per_craft_output_credit'])
                    empty=(await call(snapshot=single_key,additional_targets=None,page={'state_budget':1}))['stock_schedule']
                    self.assertTrue(empty['schedule_complete']);self.assertEqual(empty['steps'],[])
                    cases=0
                    for cross in range(3):
                        for owned_a in range(2):
                            for owned_b in range(2):
                                value=sample(0);value['sections']['character_bank']={}
                                bags={str(i):{'item_id':item,'count':count} for i,(item,count) in
                                    enumerate([(10,owned_a),(20,owned_b)],1) if count}
                                value['sections']['bags']={'0':bags}
                                entries=value['sections']['profession_recipe_materials']['entries']
                                entries['100'].update(reagents={'20':cross} if cross else {},base_output_quantity={'minimum':2,'maximum':2})
                                entries['200'].update(reagents={'10':1},base_output_quantity={'minimum':2,'maximum':2})
                                saved=store.ingest(value)
                                for target_a in range(1,3):
                                    for target_b in range(1,3):
                                        actual=await call(snapshot=saved,target_bag_quantity=target_a,
                                            additional_targets=[{'recipe_id':200,'target_bag_quantity':target_b}],page={'limit':50})
                                        counts=tuple(row['planned_crafts'] for row in actual['targets'])
                                        seen={(0,0)};queue=deque([(0,0)])
                                        while queue:
                                            done_a,done_b=queue.popleft()
                                            a=owned_a+2*done_a-done_b;b=owned_b+2*done_b-cross*done_a
                                            for enabled,next_state in [(done_a<counts[0] and b>=cross,(done_a+1,done_b)),
                                                    (done_b<counts[1] and a>=1,(done_a,done_b+1))]:
                                                if enabled and next_state not in seen:seen.add(next_state);queue.append(next_state)
                                        expected=counts in seen;out=actual['stock_schedule']
                                        self.assertEqual(out['schedule_complete'],expected)
                                        self.assertTrue(out['decision_complete'])
                                        if expected:
                                            a,b=owned_a,owned_b;used={100:0,200:0}
                                            for step in out['steps']:
                                                for _ in range(step['crafts']):
                                                    if step['recipe_id']==100:self.assertGreaterEqual(b,cross);b-=cross;a+=2
                                                    else:self.assertGreaterEqual(a,1);a-=1;b+=2
                                                    used[step['recipe_id']]+=1
                                            self.assertEqual((used[100],used[200]),counts)
                                            self.assertGreaterEqual(a,target_a);self.assertGreaterEqual(b,target_b)
                                        else:self.assertEqual(out['steps'],[])
                                        cases+=1
                    self.assertEqual(cases,48)
                    paths=[p for d in [folder,foreign] for p in Path(d).glob('*.json')]
                    before={p:p.read_bytes() for p in paths};await call(page={})
                    self.assertEqual(before,{p:p.read_bytes() for p in paths})
