"""Explicit captured producers resolve ambiguity without bypassing planner guards."""
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
from test_recipe_reagent_chain import fixed


class RecipeProducerChoiceTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_choices_validation_usage_and_existing_guards(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        value = fixture()
        value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['bags'] = {'0':{'1':{'item_id':10,'count':5},'2':{'item_id':11,'count':3}}}
        rows = {'101':fixed(100,{'20':3,'30':1}), '102':fixed(20,{'10':2},2),
            '103':fixed(30,{'10':3}), '104':fixed(20,{'11':1}), '105':{'status':'unavailable'}}
        for i in range(5): rows[str(300+i)] = fixed(500+i,{})
        value['sections']['profession_recipe_materials'] = {
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':rows}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            key = store.ingest(value)
            variants = {}
            for name in ['owned','unlearned','cycle','unknown_yield','unsupported','many_producers','craft_limit','partial_stock','active_producer']:
                saved = deepcopy(value)
                bulk = saved['sections']['profession_recipe_materials']['entries']
                if name=='owned': saved['sections']['bags']['0']['3'] = {'item_id':20,'count':3}
                if name=='unlearned': saved['sections']['selected_recipe'] = {
                    'recipe_id':104,'name':'PRIVATE','learned':False,'requirements':'fixed_items','reagents':{'11':1}}
                if name=='cycle': bulk['104']['reagents'] = {'100':1}
                if name=='unknown_yield': bulk['104'].pop('base_output_quantity')
                if name=='unsupported': bulk['104'] = {'status':'unsupported_required_slots'}
                if name=='craft_limit': bulk['101']['reagents'] = {'20':4294967295}
                if name=='partial_stock': saved['sections']['bags']['0']['1']['count'] = 2
                if name=='active_producer': bulk['101']['output_item_id'] = 20
                if name=='many_producers':
                    for i in range(32): bulk[str(200+i)] = fixed(20,{'10':1})
                variants[name] = store.ingest(saved)
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
                    default = (await plan())['reagent_chain']
                    self.assertEqual(default['blocked_reason']['reason'],'ambiguous_captured_producer')
                    self.assertEqual(default['blocked_reason']['current_deficit'],3)
                    self.assertEqual(default['blocked_reason']['candidate_estimates'], [
                        {'recipe_id':id,'reported_minimum_output_per_craft':minimum,
                         'already_active_in_chain':False,
                         'crafts_for_current_deficit':crafts,'fits_step_craft_limit':True,
                         'direct_reagents_for_current_deficit':reagents,
                         'direct_reagent_shortfalls_vs_chain_balance':{},
                         'captured_charges':None,'captured_charges_cover_current_deficit':None}
                        for id,minimum,crafts,reagents in [(102,2,2,{'10':4}),(104,1,3,{'11':3})]])
                    partial = (await plan(snapshot_id=variants['partial_stock']))['reagent_chain']
                    active = (await plan(snapshot_id=variants['active_producer']))['reagent_chain']
                    self.assertEqual([(row['recipe_id'],row['already_active_in_chain'])
                                      for row in active['blocked_reason']['candidate_estimates']],
                                     [(101,True),(102,False),(104,False)])
                    cyclic = (await plan(snapshot_id=variants['active_producer'],
                                         reagent_producer_choices=[{'item_id':20,'recipe_id':101}]))[
                                             'reagent_chain']
                    self.assertEqual(cyclic['blocked_reason']['reason'],'cyclic_uncovered_demand')
                    self.assertEqual([row['direct_reagent_shortfalls_vs_chain_balance']
                                      for row in partial['blocked_reason']['candidate_estimates']],
                                     [{'10':2},{}])
                    self.assertEqual(default,(await plan(reagent_producer_choices=[]))['reagent_chain'])
                    self.assertEqual(default,(await plan(reagent_producer_choices=None))['reagent_chain'])
                    self.assertNotIn('producer_previews', default)
                    preview = (await plan(preview_reagent_producer_choices=True))['reagent_chain']
                    report = preview.pop('producer_previews')
                    self.assertEqual(preview, default)
                    self.assertEqual(report['item_id'], 20)
                    self.assertIs(report['truncated'], False)
                    self.assertEqual([(row['recipe_id'], row['draft_complete'],
                                       sum(item['quantity'] for item in row['external_material_requirements']))
                                      for row in report['candidate_previews']], [(102,True,2),(104,True,0)])
                    with_stock = (await plan(preview_reagent_producer_choices=True,
                                             reagent_chain_stock_plan=True))['reagent_chain']
                    for row in with_stock['producer_previews']['candidate_previews']:
                        explicit = (await plan(reagent_chain_stock_plan=True,
                                               reagent_producer_choices=[
                                                   {'item_id':20,'recipe_id':row['recipe_id']}]))['reagent_chain']
                        summary = row['stock_plan_summary']
                        self.assertEqual(summary, {field:explicit['stock_plan'][field] for field in [
                            'material_coverage','captured_chain_checks_pass',
                            'acquisition_shortfalls_vs_observed_storage','suggested_bank_withdrawal_list']})
                    cyclic_preview = (await plan(snapshot_id=variants['active_producer'],
                                                preview_reagent_producer_choices=True))['reagent_chain']
                    self.assertEqual(cyclic_preview['producer_previews']['candidate_previews'][0]['blocked_reason'],
                                     'cyclic_uncovered_demand')
                    await plan(error=True,suggest_reagent_chain=False,preview_reagent_producer_choices=True)
                    for chosen,external,crafts in [(102,2,2),(104,0,3)]:
                        requested = [{'item_id':20,'recipe_id':chosen}]
                        chain = (await plan(reagent_producer_choices=requested))['reagent_chain']
                        self.assertTrue(chain['draft_complete'])
                        self.assertEqual(chain['requested_producer_choices'],requested)
                        self.assertEqual(chain['used_producer_choices'],requested)
                        self.assertEqual((chain['steps'][0]['recipe_id'],chain['steps'][0]['crafts']),(chosen,crafts))
                        self.assertEqual(sum(r['quantity'] for r in chain['external_material_requirements']),external)
                        ledger = {10:5,11:3}
                        for item in chain['external_material_requirements']:
                            ledger[item['item_id']] = ledger.get(item['item_id'],0)+item['quantity']
                        for step in chain['steps']:
                            for item in step['required_reagents']:
                                self.assertGreaterEqual(ledger.get(item['item_id'],0),item['quantity'])
                                ledger[item['item_id']] -= item['quantity']
                            ledger[step['output_item_id']] = ledger.get(step['output_item_id'],0)+step['minimum_output_if_completed']
                        for item in chain['modeled_remaining_items_if_completed']:
                            self.assertEqual(ledger[item['item_id']],item['quantity'])
                        for flag in ['complete_ownership_verified','minimum_yields_verified','optimisation_verified','craftability_verified','game_execution']:
                            self.assertFalse(chain[flag])
                    requested = [{'item_id':20,'recipe_id':104}]
                    owned = (await plan(snapshot_id=variants['owned'],reagent_producer_choices=requested))['reagent_chain']
                    self.assertEqual(owned['used_producer_choices'],[])
                    self.assertNotIn(104,[step['recipe_id'] for step in owned['steps']])
                    for name,reason in [('unlearned','recipe_observation_conflict_or_unavailable'),
                        ('cycle','cyclic_uncovered_demand'),('unknown_yield','output_or_minimum_yield_unavailable'),
                        ('craft_limit','craft_count_budget')]:
                        blocked = (await plan(snapshot_id=variants[name],reagent_producer_choices=requested))['reagent_chain']
                        self.assertFalse(blocked['draft_complete'])
                        self.assertEqual(blocked['blocked_reason']['reason'],reason)
                        self.assertEqual(blocked['used_producer_choices'],requested)
                    await plan(snapshot_id=variants['unsupported'],reagent_producer_choices=requested,error=True)
                    many = (await plan(snapshot_id=variants['many_producers'],
                        reagent_producer_choices=[{'item_id':20,'recipe_id':231}]))['reagent_chain']
                    self.assertTrue(many['draft_complete'])
                    self.assertEqual(many['steps'][0]['recipe_id'],231)
                    eight = [{'item_id':20,'recipe_id':104},{'item_id':30,'recipe_id':103},
                        {'item_id':100,'recipe_id':101},*[{'item_id':500+i,'recipe_id':300+i} for i in range(5)]]
                    chain = (await plan(reagent_producer_choices=list(reversed(eight))))['reagent_chain']
                    self.assertTrue(chain['draft_complete'])
                    self.assertEqual(chain['used_producer_choices'],eight[:2])
                    for bad in [[{'item_id':20,'recipe_id':103}],[{'item_id':20,'recipe_id':999}],
                        [{'item_id':20,'recipe_id':105}],requested*2,eight+[{'item_id':20,'recipe_id':102}],
                        [{}],[{'item_id':0,'recipe_id':104}],[{'item_id':20,'recipe_id':False}],
                        [{'item_id':'20','recipe_id':104}],[{'item_id':20,'recipe_id':104,'unexpected':'PRIVATE'}]]:
                        await plan(reagent_producer_choices=bad,error=True)
                    await plan(suggest_reagent_chain=False,reagent_producer_choices=requested,error=True)
                    await plan(suggest_reagent_chain=False,reagent_producer_choices=[],error=True)
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
