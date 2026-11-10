"""Joint targets share one capture and one material ledger; all output stays conditional."""
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


def sample(owned=1):
    value=fixture();value['client'].update(family='forever',build='70862',interface=16001)
    bags={'1':{'item_id':1,'count':5}}
    if owned:bags['2']={'item_id':10,'count':owned}
    value['sections'].update(bags={'0':bags},character_bank={'6':{'1':{'item_id':10,'count':100}}},
        profession_recipe_materials={'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
            '100':{'status':'fixed_items','reagents':{'1':2},'output_item_id':10,'base_output_quantity':{'minimum':2,'maximum':3}},
            '200':{'status':'fixed_items','reagents':{'10':3},'output_item_id':20,'base_output_quantity':{'minimum':1,'maximum':1}}}})
    return value


class RecipeBatchRestockTests(unittest.IsolatedAsyncioTestCase):
    async def test_joint_targets_real_mcp_and_independent_integer_oracle(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder, tempfile.TemporaryDirectory(dir=root/'local-data') as foreign:
            store=SnapshotStore(Path(folder));key=store.ingest(sample())
            other=sample();other['character']['name']='PRIVATE_FOREIGN_OWNER'
            foreign_key=SnapshotStore(Path(foreign)).ingest(other)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(snapshot=key,primary=100,target=2,additional=None,error=False,**extra):
                        args={'snapshot_id':snapshot,'recipe_id':primary,'target_bag_quantity':target,**extra}
                        if additional is not None:args['additional_targets']=additional
                        reply=await session.call_tool('plan_forever_recipe_restock',args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE_FOREIGN_OWNER',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    targets=[{'recipe_id':200,'target_bag_quantity':2}]
                    result=await call(additional=targets)
                    self.assertEqual([r['planned_crafts'] for r in result['targets']],[4,2])
                    self.assertEqual(result['suggested_recipe_order'],[100,200])
                    self.assertEqual(result['combined_gross_materials'][0]['required'],8)
                    self.assertEqual(result['ordered_materials'][1]['required_before_batch'],0)
                    selected=sample();selected['sections']['selected_recipe']={
                        'recipe_id':100,'name':'Fixture','learned':True,'requirements':'fixed_items',
                        'reagents':{'1':2},'output_item_id':10,'base_output_quantity':{'minimum':2,'maximum':3}}
                    selected_key=store.ingest(selected)
                    self.assertEqual([r['planned_crafts'] for r in
                        (await call(snapshot=selected_key,primary=None,additional=targets))['targets']],[4,2])
                    selected['sections']['selected_recipe']['reagents']={'1':3}
                    await call(snapshot=store.ingest(selected),primary=None,additional=targets,error=True)
                    for flag in ['game_execution','live','craftability_verified','current_inventory_verified',
                            'active_character_verified','item_variant_equivalence_verified','acquisition_effects_included']:
                        self.assertFalse(result[flag])
                    single=await call();self.assertNotIn('targets',single)
                    self.assertEqual(single['planned_crafts'],1)
                    self.assertEqual(await call(additional_targets=None),single)
                    cases=0
                    for owned in range(3):
                        saved=store.ingest(sample(owned))
                        for first in range(1,5):
                            for second in range(1,5):
                                actual=await call(snapshot=saved,target=first,
                                    additional=[{'recipe_id':200,'target_bag_quantity':second}])
                                feasible=[(a,b) for a in range(21) for b in range(21)
                                    if owned+2*a-3*b>=first and b>=second]
                                expected=min(feasible,key=lambda row:(sum(row),row))
                                self.assertEqual(tuple(r['planned_crafts'] for r in actual['targets']),expected)
                                self.assertEqual(actual['targets'][0]['modelled_final_bag_quantity'],
                                    {'minimum':owned+2*expected[0]-3*expected[1],
                                     'maximum':owned+3*expected[0]-3*expected[1]})
                                cases+=1
                    self.assertEqual(cases,48)
                    eight=sample();entries=eight['sections']['profession_recipe_materials']['entries']
                    for number in range(3,9):entries[str(number*100)]={'status':'fixed_items','reagents':{'1':1},
                        'output_item_id':number*10,'base_output_quantity':{'minimum':1,'maximum':1}}
                    eight_key=store.ingest(eight)
                    all_targets=[{'recipe_id':i*100,'target_bag_quantity':1} for i in range(2,9)]
                    self.assertEqual(len((await call(snapshot=eight_key,target=1,additional=all_targets))['targets']),8)
                    for bad in [[],targets*2,[{'recipe_id':100,'target_bag_quantity':1}],
                            all_targets+[{'recipe_id':900,'target_bag_quantity':1}],
                            [{'recipe_id':0,'target_bag_quantity':1}], [{'recipe_id':200,'target_bag_quantity':0}],
                            [{'recipe_id':200,'target_bag_quantity':1,'snapshot_id':foreign_key}]]:
                        await call(additional=bad,error=True)
                    duplicate=sample();duplicate['sections']['profession_recipe_materials']['entries']['200']['output_item_id']=10
                    await call(snapshot=store.ingest(duplicate),additional=targets,error=True)
                    await call(additional=[{'recipe_id':200,'target_bag_quantity':1000}],error=True)
                    cycle=sample(0);rows=cycle['sections']['profession_recipe_materials']['entries']
                    rows['100']['reagents']={'20':1};rows['200']['reagents']={'10':1}
                    rows['200']['base_output_quantity']={'minimum':2,'maximum':2}
                    cyclic=await call(snapshot=store.ingest(cycle),target=1,
                        additional=[{'recipe_id':200,'target_bag_quantity':1}])
                    self.assertEqual(cyclic['dependency_order_status'],'cyclic_or_downstream')
                    self.assertIsNone(cyclic['suggested_recipe_order']);self.assertIsNone(cyclic['ordered_materials'])
                    rows['100']['base_output_quantity']={'minimum':1,'maximum':1}
                    rows['200']['base_output_quantity']={'minimum':1,'maximum':1}
                    await call(snapshot=store.ingest(cycle),target=1,
                        additional=[{'recipe_id':200,'target_bag_quantity':1}],error=True)
                    await call(snapshot=foreign_key,additional=targets,error=True)
                    missing=sample();del missing['sections']['bags']
                    await call(snapshot=store.ingest(missing),additional=targets,error=True)
                    paths=[p for d in [folder,foreign] for p in Path(d).glob('*.json')]
                    before={p:p.read_bytes() for p in paths}
                    await call(additional=targets)
                    self.assertEqual(before,{p:p.read_bytes() for p in paths})
