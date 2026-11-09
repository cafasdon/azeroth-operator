"""Prepare printed chain stock from historical bags/banks without moving items."""
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


class ReagentChainStockTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_starting_materials_withdrawals_unknowns_and_quotes(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['bags'] = {'0':{'1':{'item_id':10,'count':2,'link':'PRIVATE_LINK'}}}
        base['sections']['character_bank'] = {'6':{'1':{'item_id':10,'count':3}}}
        base['sections']['account_bank'] = {'7':{'1':{'item_id':10,'count':2}}}
        base['sections']['wallet'] = {'copper':4}
        base['sections']['merchant_stock'] = {'1':{'item_id':10,'bundle_size':1,'price_copper':2,
            'available':-1,'extended_cost':False}}
        base['sections']['profession_recipe_materials'] = {
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
                '101':fixed(100,{'20':3,'30':1}),'102':fixed(20,{'10':2},2),'103':fixed(30,{'10':3})}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            key = store.ingest(base)
            cases = {}
            for name in ['intermediate','short','missing_bags','missing_banks','blocked','free','repeated']:
                saved = deepcopy(base)
                rows = saved['sections']['profession_recipe_materials']['entries']
                if name=='intermediate': saved['sections']['account_bank'] = {'7':{'1':{'item_id':20,'count':1}}}
                if name=='short': saved['sections']['account_bank']['7']['1']['count'] = 1
                if name=='missing_bags':
                    saved['sections'].pop('bags')
                    saved['sections']['character_bank']['6']['1']['count'] = 5
                if name=='missing_banks':
                    saved['sections'].pop('character_bank')
                    saved['sections'].pop('account_bank')
                if name=='blocked': rows['104'] = fixed(20,{'11':1})
                if name=='free':
                    for field in ['bags','character_bank','account_bank']: saved['sections'].pop(field)
                    rows['101']['reagents'] = {}
                if name=='repeated':
                    rows['101']['reagents'] = {'20':1,'30':1}
                    rows['102']['reagents'] = {'10':1}
                    rows['103']['reagents'] = {'20':3}
                cases[name] = store.ingest(saved)
            inventory = deepcopy(base)
            inventory['captured_at'] = '2025-01-02T00:00:00Z'
            inventory_id = store.ingest(inventory)
            wrong = deepcopy(inventory)
            wrong['character']['name'] = 'PRIVATE_OTHER'
            wrong_id = store.ingest(wrong)
            before = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    tools = {tool.name:tool for tool in (await session.list_tools()).tools}
                    self.assertEqual(tools['review_forever_recipe'].inputSchema['properties']['reagent_chain_stock_plan']['type'],'boolean')
                    async def plan(error=False,**options):
                        reply = await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':101,
                            'crafts':1,'suggest_reagent_chain':True,'reagent_chain_stock_plan':True,**options})
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    results = {}
                    for name,sid in [('base',key),*cases.items()]: results[name] = await plan(snapshot_id=sid)
                    stock = results['base']['reagent_chain']['stock_plan']
                    material = {row['item_id']:row for row in stock['starting_materials']}
                    self.assertEqual(material[10]['required_before_batch'],7)
                    self.assertEqual(material[10]['shortfall_vs_bags'],5)
                    self.assertEqual(material[10]['suggested_bank_withdrawals'],{'character_bank':3,'account_bank':2})
                    self.assertEqual(stock['material_coverage'],{'bags':False,'all_observed_storage':True})
                    self.assertIsNone(stock['material_checkpoint']['first_uncovered_recipe'])
                    self.assertEqual(stock['material_checkpoint']['covered_prefix_recipes'],3)
                    self.assertEqual(stock['nonmaterial_requirements']['recipes_with_unknown_checks'],3)
                    intermediate = {row['item_id']:row for row in results['intermediate']['reagent_chain']['stock_plan']['starting_materials']}
                    self.assertEqual(intermediate[20]['required_before_batch'],1)
                    self.assertEqual(intermediate[20]['suggested_bank_withdrawals'],{'character_bank':0,'account_bank':1})
                    self.assertEqual(intermediate[10]['required_before_batch'],5)
                    short = results['short']['reagent_chain']['stock_plan']
                    self.assertFalse(short['material_coverage']['all_observed_storage'])
                    self.assertEqual(short['material_checkpoint']['first_uncovered_recipe']['recipe_id'],103)
                    self.assertEqual(short['material_checkpoint']['covered_prefix_recipes'],1)
                    missing = results['missing_bags']['reagent_chain']['stock_plan']
                    self.assertEqual(missing['material_coverage'],{'bags':None,'all_observed_storage':True})
                    raw = next(row for row in missing['starting_materials'] if row['item_id']==10)
                    self.assertIsNone(raw['suggested_bank_withdrawals'])
                    self.assertIsNone(raw['shortfall_vs_bags'])
                    banks = next(row for row in results['missing_banks']['reagent_chain']['stock_plan']['starting_materials'] if row['item_id']==10)
                    self.assertEqual(banks['suggested_bank_withdrawals'],{'character_bank':None,'account_bank':None})
                    self.assertIsNone(results['blocked']['reagent_chain']['stock_plan'])
                    self.assertEqual(results['free']['reagent_chain']['stock_plan']['starting_materials'],[])
                    self.assertEqual(results['free']['reagent_chain']['stock_plan']['material_coverage'],{'bags':True,'all_observed_storage':True})
                    self.assertEqual([step['recipe_id'] for step in results['repeated']['reagent_chain']['steps']],[102,102,103,101])
                    for name,result in results.items():
                        chain = result['reagent_chain']
                        if not chain['draft_complete']: continue
                        # Independent prefix deficits define starting needs for printed step order.
                        balance,needed = {},{}
                        for step in chain['steps']:
                            for row in step['required_reagents']:
                                item = row['item_id']
                                balance[item] = balance.get(item,0)-row['quantity']
                                needed[item] = max(needed.get(item,0),-balance[item])
                            item = step['output_item_id']
                            balance[item] = balance.get(item,0)+step['minimum_output_if_completed']
                        actual = {row['item_id']:row['required_before_batch'] for row in chain['stock_plan']['starting_materials']}
                        self.assertEqual(actual,needed,name)
                        for field in ['withdrawal_eligibility_verified','current_stock_verified','minimum_yields_verified','craftability_verified','live','game_execution']:
                            self.assertFalse(chain['stock_plan'][field])
                    quoted = (await plan(snapshot_id=cases['short'],merchant_reagent_plan=True))['reagent_chain']
                    self.assertTrue(quoted['merchant_reagent_plan']['unlimited_copper_only_affordable'])
                    self.assertEqual(quoted['merchant_reagent_plan']['unlimited_copper_only_total'],2)
                    self.assertFalse(quoted['stock_plan']['material_coverage']['all_observed_storage'])
                    self.assertFalse(quoted['stock_plan']['craftability_verified'])
                    later = (await plan(inventory_snapshot_id=inventory_id))['reagent_chain']['stock_plan']
                    self.assertEqual(later['inventory_captured_at'],'2025-01-02T00:00:00Z')
                    await plan(inventory_snapshot_id=wrong_id,error=True)
                    await plan(suggest_reagent_chain=False,error=True)
                    await plan(reagent_chain_stock_plan='true',error=True)
                    ordinary = await plan(reagent_chain_stock_plan=False)
                    self.assertNotIn('stock_plan',ordinary['reagent_chain'])
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
