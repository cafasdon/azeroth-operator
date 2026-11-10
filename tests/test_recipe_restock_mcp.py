"""Actual offline MCP restock route: numeric targets never become craft actions."""
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
from tools.gateway_profile import TOOLS


class RecipeRestockTests(unittest.IsolatedAsyncioTestCase):
    async def test_saved_recipe_restock_targets_and_owner_boundaries(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder))
            value=fixture();value['client'].update(family='forever',build='70862',interface=16001)
            recipe={'recipe_id':100,'name':'Fixture','learned':True,'requirements':'fixed_items','reagents':{'1':3},
                'output_item_id':2,'base_output_quantity':{'minimum':3,'maximum':5}}
            value['sections'].update(selected_recipe=recipe,bags={'0':{'1':{'item_id':2,'count':4},
                '2':{'item_id':1,'count':9}}},character_bank={'6':{'1':{'item_id':2,'count':100}}})
            key=store.ingest(value)
            bulk=deepcopy(value)
            bulk['sections']['profession_recipe_materials']={'coverage':'reported_known_spell_schematics',
                'profession_id':164,'entries':{'100':{k:v for k,v in recipe.items()
                    if k not in ('recipe_id','learned','requirements','name')}}}
            bulk['sections']['profession_recipe_materials']['entries']['100']['status']='fixed_items'
            bulk_key=store.ingest(bulk)
            missing=deepcopy(value);del missing['sections']['bags'];missing_key=store.ingest(missing)
            empty=deepcopy(value);empty['sections']['bags']={};empty_key=store.ingest(empty)
            self_input=deepcopy(value);self_input['sections']['selected_recipe']['reagents']['2']=1
            self_key=store.ingest(self_input)
            zero_net=deepcopy(self_input);zero_net['sections']['selected_recipe']['reagents']['2']=3
            zero_key=store.ingest(zero_net)
            no_output=deepcopy(value);del no_output['sections']['selected_recipe']['base_output_quantity']
            no_output_key=store.ingest(no_output)
            unsupported=deepcopy(value);unsupported['sections']['selected_recipe']['requirements']='unsupported_required_slots'
            unsupported['sections']['selected_recipe']['reagents']={};unsupported_key=store.ingest(unsupported)
            conflict=deepcopy(bulk);conflict['sections']['selected_recipe']['output_item_id']=3
            conflict_key=store.ingest(conflict)
            partial_bulk=deepcopy(bulk)
            row=partial_bulk['sections']['profession_recipe_materials']['entries']['100']
            del row['output_item_id'];del row['base_output_quantity'];partial_key=store.ingest(partial_bulk)
            unavailable_bulk=deepcopy(bulk)
            unavailable_bulk['sections']['profession_recipe_materials']['entries']['100']={'status':'budget_exhausted'}
            unavailable_key=store.ingest(unavailable_bulk)
            retail=store.ingest(fixture())
            other=deepcopy(value);other['character']['name']='PRIVATE_FOREIGN_OWNER'
            foreign_key=SnapshotStore(Path(foreign)).ingest(other)
            before={p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize();routes=(await session.list_tools()).tools
                    self.assertEqual({t.name for t in routes},set(TOOLS))
                    tool=next(t for t in routes if t.name=='plan_forever_recipe_restock')
                    self.assertTrue(tool.annotations.readOnlyHint)
                    self.assertFalse(tool.annotations.destructiveHint);self.assertFalse(tool.annotations.openWorldHint)
                    async def call(snapshot=key,target=10,error=False,**extra):
                        reply=await session.call_tool('plan_forever_recipe_restock',
                            {'snapshot_id':snapshot,'target_bag_quantity':target,**extra})
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE_FOREIGN_OWNER',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    plan=await call()
                    self.assertEqual(plan['snapshot_id'],key)
                    self.assertEqual((plan['observed_bag_quantity'],plan['shortfall'],plan['planned_crafts']),(4,6,2))
                    self.assertEqual(plan['projected_bag_quantity_if_completed'],{'minimum':10,'maximum':14})
                    self.assertEqual(plan['recipe_review']['materials'][0]['required'],6)
                    for name in ['live','game_execution','craftability_verified','current_inventory_verified',
                            'native_bag_placement_verified','item_variant_equivalence_verified','active_character_verified']:
                        self.assertFalse(plan[name])
                    self.assertEqual((await call(snapshot=bulk_key,recipe_id=100))['planned_crafts'],2)
                    done=await call(target=3);self.assertEqual(done['planned_crafts'],0)
                    self.assertIsNone(done['recipe_review']);self.assertTrue(done['saved_target_already_met'])
                    self.assertEqual((await call(snapshot=empty_key))['planned_crafts'],4)
                    net=await call(snapshot=self_key);self.assertEqual(net['planned_crafts'],3)
                    self.assertEqual(net['projected_bag_quantity_if_completed'],{'minimum':10,'maximum':16})
                    self.assertEqual((await call(target=3004))['planned_crafts'],1000)
                    await call(target=3005,error=True)
                    for bad in [None,False,0,-1,1.5,'10',2**32]:await call(target=bad,error=True)
                    for bad in [missing_key,zero_key,no_output_key,unsupported_key,retail,foreign_key,'../PRIVATE_FOREIGN_OWNER']:
                        await call(snapshot=bad,error=True)
                    await call(snapshot=conflict_key,recipe_id=100,error=True)
                    await call(snapshot=conflict_key,error=True)
                    self.assertEqual((await call(snapshot=partial_key))['planned_crafts'],2)
                    self.assertEqual((await call(snapshot=unavailable_key))['planned_crafts'],2)
                    await call(snapshot=bulk_key,recipe_id=101,error=True)
            self.assertEqual(before,{p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')})
