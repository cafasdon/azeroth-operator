"""Captured producer chains share stock and expose missing or ambiguous evidence."""
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


def fixed(output, reagents, minimum=1):
    return {'status':'fixed_items','output_item_id':output,'reagents':reagents,
        'base_output_quantity':{'minimum':minimum,'maximum':minimum+1}}


class RecipeReagentChainTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_shared_stock_yields_blockers_context_and_conservation(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        value = fixture()
        value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['bags'] = {'0':{'1':{'item_id':10,'count':5}}}
        value['sections']['profession_recipe_materials'] = {
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
                '101':fixed(100,{'20':3,'30':1}), '102':fixed(20,{'10':2},2), '103':fixed(30,{'10':3})}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            key = store.ingest(value)
            cases = {}
            for name in ['owned_intermediate','ambiguous','cycle','missing_yield','learning_conflict','root_learning_conflict',
                'missing_inventory','no_cost','craft_limit','repeated_producer','unsupported','eight_steps','nine_steps']:
                saved = deepcopy(value)
                rows = saved['sections']['profession_recipe_materials']['entries']
                if name == 'owned_intermediate': saved['sections']['bags']['0']['2'] = {'item_id':20,'count':1}
                if name == 'ambiguous': rows['104'] = fixed(20,{'10':1})
                if name == 'cycle': rows['102']['reagents'] = {'100':1}
                if name == 'missing_yield': rows['102'].pop('base_output_quantity')
                if name in ['learning_conflict','root_learning_conflict']:
                    selected = 102 if name=='learning_conflict' else 101
                    saved['sections']['selected_recipe'] = {'recipe_id':selected,'name':'PRIVATE','learned':False,
                        'requirements':'fixed_items','reagents':rows[str(selected)]['reagents']}
                if name in ['missing_inventory','no_cost']: saved['sections'].pop('bags')
                if name == 'no_cost': rows['101']['reagents'] = {}
                if name == 'craft_limit': rows['101']['reagents'] = {'20':4294967295}
                if name == 'unsupported': rows['101'] = {'status':'unsupported_required_slots'}
                if name == 'repeated_producer':
                    rows['101']['reagents'] = {'20':1,'30':1}
                    rows['102']['reagents'] = {'10':1}
                    rows['103']['reagents'] = {'20':3}
                if name in ['eight_steps','nine_steps']:
                    count = 8 if name=='eight_steps' else 9
                    saved['sections']['bags'] = {}
                    saved['sections']['profession_recipe_materials']['entries'] = {
                        str(101+i):fixed(100+i,{str(101+i if i<count-1 else 99999):1}) for i in range(count)}
                cases[name] = store.ingest(saved)
            inventory = deepcopy(value)
            inventory['captured_at'] = '2025-01-02T00:00:00Z'
            inventory['sections']['bags']['0']['2'] = {'item_id':20,'count':1}
            inventory_id = store.ingest(inventory)
            mismatch = deepcopy(inventory)
            mismatch['character']['name'] = 'PRIVATE_OTHER'
            mismatch_id = store.ingest(mismatch)
            before = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def plan(error=False,**options):
                        reply = await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':101,
                            'crafts':1,'suggest_reagent_chain':True,**options})
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    draft = (await plan())['reagent_chain']
                    self.assertTrue(draft['draft_complete'])
                    self.assertEqual([(s['recipe_id'],s['crafts']) for s in draft['steps']],[(102,2),(103,1),(101,1)])
                    self.assertEqual(draft['external_material_requirements'],[{'item_id':10,'quantity':2,'basis':'no_captured_fixed_producer'}])
                    remaining = {r['item_id']:r['quantity'] for r in draft['modeled_remaining_items_if_completed']}
                    self.assertEqual(remaining,{10:0,20:1,30:0,100:1})
                    # Independently replay each modeled action with initial + external items.
                    ledger = {10:7}
                    for step in draft['steps']:
                        for material in step['required_reagents']:
                            item,quantity = material['item_id'],material['quantity']
                            self.assertGreaterEqual(ledger.get(item,0),quantity)
                            ledger[item] -= quantity
                        item = step['output_item_id']
                        ledger[item] = ledger.get(item,0)+step['minimum_output_if_completed']
                        self.assertIsNone(step['captured_nonmaterial_checks']['observed_conditions_met'])
                    self.assertEqual(ledger,remaining)
                    owned = (await plan(snapshot_id=cases['owned_intermediate']))['reagent_chain']
                    self.assertFalse(owned['requires_external_materials'])
                    self.assertEqual([(s['recipe_id'],s['crafts']) for s in owned['steps']],[(102,1),(103,1),(101,1)])
                    self.assertFalse((await plan(inventory_snapshot_id=inventory_id))['reagent_chain']['requires_external_materials'])
                    await plan(inventory_snapshot_id=mismatch_id,error=True)
                    await plan(snapshot_id=cases['root_learning_conflict'],error=True)
                    repeated = (await plan(snapshot_id=cases['repeated_producer']))['reagent_chain']
                    self.assertEqual([s['recipe_id'] for s in repeated['steps']],[102,102,103,101])
                    self.assertFalse(repeated['requires_external_materials'])
                    target = (await plan(crafts=None,target_output_quantity=5))['reagent_chain']
                    self.assertEqual(target['root_crafts'],5)
                    self.assertEqual(target['external_material_requirements'][0]['quantity'],26)
                    for name,reason in [('ambiguous','ambiguous_captured_producer'),('cycle','cyclic_uncovered_demand'),
                        ('missing_yield','output_or_minimum_yield_unavailable'),('learning_conflict','recipe_observation_conflict_or_unavailable'),
                        ('missing_inventory','inventory_unobserved'),('craft_limit','craft_count_budget'),
                        ('unsupported','unsupported_materials'),('nine_steps','step_budget')]:
                        blocked = (await plan(snapshot_id=cases[name]))['reagent_chain']
                        self.assertFalse(blocked['draft_complete'],name)
                        self.assertEqual(blocked['blocked_reason']['reason'],reason,name)
                        self.assertIsNone(blocked['external_material_requirements'])
                        self.assertIsNone(blocked['modeled_remaining_items_if_completed'])
                        self.assertLessEqual(len(blocked['steps']),8)
                    free = (await plan(snapshot_id=cases['no_cost']))['reagent_chain']
                    self.assertTrue(free['draft_complete'])
                    self.assertFalse(free['requires_external_materials'])
                    eight = (await plan(snapshot_id=cases['eight_steps']))['reagent_chain']
                    self.assertTrue(eight['draft_complete'])
                    self.assertEqual(len(eight['steps']),8)
                    self.assertEqual(eight['external_material_requirements'][0]['item_id'],99999)
                    for bad in [{'recipe_id':None},{'additional_recipes':[{'snapshot_id':key,'recipe_id':102,'crafts':1}]},
                        {'suggest_dependency_order':True},{'suggest_reagent_chain':'true'}]:
                        await plan(error=True,**bad)
                    no_chain = await plan(suggest_reagent_chain=False)
                    self.assertNotIn('reagent_chain',no_chain)
                    for field in ['complete_ownership_verified','minimum_yields_verified','optimisation_verified','craftability_verified','live','game_execution']:
                        self.assertFalse(draft[field])
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
