"""Price net external chain leaves once; quote and wallet capture contexts stay explicit."""
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


def offer(item,size,price):
    return {'item_id':item,'bundle_size':size,'price_copper':price,'available':-1,'extended_cost':False}


class ReagentChainMerchantTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_net_leaf_quotes_wallet_unknowns_choices_and_context(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['bags'] = {'0':{'1':{'item_id':10,'count':5}}}
        base['sections']['wallet'] = {'copper':6}
        base['sections']['merchant_stock'] = {'1':offer(10,3,7),'2':offer(20,1,100),'3':offer(30,1,100)}
        base['sections']['profession_recipe_materials'] = {
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
                '101':fixed(100,{'20':3,'30':1}),'102':fixed(20,{'10':2},2),'103':fixed(30,{'10':3})}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            key = store.ingest(base)
            variants = {}
            for name in ['missing_offers','missing_wallet','limited','extended','unquoted','blocked','overflow','alternative',
                'mixed_alternatives','overflow_with_valid_alternative']:
                saved = deepcopy(base)
                if name=='missing_offers': saved['sections'].pop('merchant_stock')
                if name=='missing_wallet': saved['sections'].pop('wallet')
                if name=='limited': saved['sections']['merchant_stock']['1']['available'] = 100
                if name=='extended': saved['sections']['merchant_stock']['1']['extended_cost'] = True
                if name=='unquoted': saved['sections']['merchant_stock']['1']['item_id'] = 99
                if name in ['blocked','alternative']:
                    saved['sections']['profession_recipe_materials']['entries']['104'] = fixed(20,{'11':1})
                if name=='alternative': saved['sections']['merchant_stock']['4'] = offer(11,1,2)
                if name in ['overflow','overflow_with_valid_alternative']:
                    saved['sections']['profession_recipe_materials']['entries']['101']['reagents'] = {'10':4294967295}
                    saved['sections']['merchant_stock']['1'] = offer(10,1,4294967295)
                if name=='overflow_with_valid_alternative': saved['sections']['merchant_stock']['4'] = offer(10,1,1)
                if name=='mixed_alternatives':
                    saved['sections']['merchant_stock']['4'] = {**offer(10,1,1),'available':100}
                    saved['sections']['merchant_stock']['5'] = {**offer(10,1,1),'extended_cost':True}
                variants[name] = store.ingest(saved)
            merchant = deepcopy(base)
            merchant['captured_at'] = '2025-01-03T00:00:00Z'
            merchant['sections']['wallet']['copper'] = 99999
            merchant_id = store.ingest(merchant)
            wrong = deepcopy(merchant)
            wrong['character']['name'] = 'PRIVATE_OTHER'
            wrong_id = store.ingest(wrong)
            inventory = deepcopy(base)
            inventory['captured_at'] = '2025-01-02T00:00:00Z'
            inventory['sections']['bags']['0']['1']['count'] = 4
            inventory['sections']['wallet']['copper'] = 1
            inventory_id = store.ingest(inventory)
            enough = deepcopy(inventory)
            enough['sections']['bags']['0']['1']['count'] = 7
            enough_id = store.ingest(enough)
            before = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def plan(error=False,**options):
                        reply = await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':101,
                            'crafts':1,'suggest_reagent_chain':True,'merchant_reagent_plan':True,**options})
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    result = await plan()
                    self.assertNotIn('merchant_reagent_plan',result)
                    quote = result['reagent_chain']['merchant_reagent_plan']
                    self.assertEqual(quote['basis'],'chain_external_material_shortfalls')
                    self.assertEqual([row['item_id'] for row in quote['reagents']],[10])
                    self.assertEqual(quote['reagents'][0]['shortfall_vs_observed'],2)
                    self.assertEqual(quote['reagents'][0]['offers_by_slot']['1']['units'],3)
                    self.assertEqual(quote['reagents'][0]['offers_by_slot']['1']['extra_units'],1)
                    self.assertEqual(quote['unlimited_copper_only_total'],7)
                    self.assertEqual(quote['unlimited_copper_only_shortfall'],1)
                    self.assertFalse(quote['unlimited_copper_only_affordable'])
                    external = await plan(merchant_snapshot_id=merchant_id)
                    self.assertFalse(external['reagent_chain']['merchant_reagent_plan']['unlimited_copper_only_affordable'])
                    later = await plan(inventory_snapshot_id=inventory_id,merchant_snapshot_id=merchant_id)
                    priced = later['reagent_chain']['merchant_reagent_plan']
                    self.assertEqual(priced['reagents'][0]['shortfall_vs_observed'],3)
                    self.assertEqual(priced['unlimited_copper_only_shortfall'],6)
                    self.assertEqual(priced['inventory_captured_at'],'2025-01-02T00:00:00Z')
                    self.assertEqual(priced['merchant_captured_at'],'2025-01-03T00:00:00Z')
                    zero = (await plan(inventory_snapshot_id=enough_id))['reagent_chain']['merchant_reagent_plan']
                    self.assertEqual(zero['reagents'],[])
                    self.assertEqual(zero['unlimited_copper_only_total'],0)
                    self.assertTrue(zero['unlimited_copper_only_affordable'])
                    no_wallet = (await plan(snapshot_id=variants['missing_wallet']))['reagent_chain']['merchant_reagent_plan']
                    self.assertEqual(no_wallet['unlimited_copper_only_total'],7)
                    self.assertIsNone(no_wallet['unlimited_copper_only_affordable'])
                    self.assertIsNone(no_wallet['unlimited_copper_only_shortfall'])
                    for name in ['missing_offers','blocked']:
                        chain = (await plan(snapshot_id=variants[name]))['reagent_chain']
                        self.assertIsNone(chain['merchant_reagent_plan'])
                    for name in ['limited','extended','unquoted','overflow']:
                        q = (await plan(snapshot_id=variants[name],crafts=1000 if name=='overflow' else 1))['reagent_chain']['merchant_reagent_plan']
                        self.assertIsNone(q['unlimited_copper_only_total'],name)
                        self.assertIsNone(q['unlimited_copper_only_affordable'],name)
                    mixed = (await plan(snapshot_id=variants['mixed_alternatives']))['reagent_chain']['merchant_reagent_plan']
                    self.assertEqual(mixed['unlimited_copper_only_total'],7)
                    self.assertEqual(mixed['reagents'][0]['lowest_unlimited_copper_only_offer']['slot'],1)
                    alternate = (await plan(snapshot_id=variants['overflow_with_valid_alternative'],crafts=1000))['reagent_chain']['merchant_reagent_plan']
                    self.assertEqual(alternate['unlimited_copper_only_total'],4294967295*1000-5)
                    self.assertIsNone(alternate['reagents'][0]['offers_by_slot']['1']['required_copper'])
                    self.assertEqual(alternate['reagents'][0]['lowest_unlimited_copper_only_offer']['slot'],4)
                    for producer,total in [(102,7),(104,6)]:
                        selected = await plan(snapshot_id=variants['alternative'],
                            reagent_producer_choices=[{'item_id':20,'recipe_id':producer}])
                        self.assertEqual(selected['reagent_chain']['merchant_reagent_plan']['unlimited_copper_only_total'],total)
                    await plan(merchant_snapshot_id=wrong_id,error=True)
                    await plan(snapshot_id=variants['blocked'],merchant_snapshot_id=wrong_id,error=True)
                    await plan(merchant_reagent_plan=False,merchant_snapshot_id=merchant_id,error=True)
                    ordinary = await plan(suggest_reagent_chain=False)
                    self.assertNotIn('reagent_chain',ordinary)
                    self.assertEqual(ordinary['merchant_reagent_plan']['unlimited_copper_only_total'],400)
                    self.assertNotIn('merchant_reagent_plan',(await plan(merchant_reagent_plan=False))['reagent_chain'])
                    for field in ['purchase_eligibility_verified','live','game_execution']:
                        self.assertFalse(quote[field])
                    self.assertFalse(result['reagent_chain']['craftability_verified'])
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
