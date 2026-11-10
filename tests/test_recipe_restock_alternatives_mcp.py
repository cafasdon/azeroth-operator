"""Actual offline MCP comparison of explicit alternative restock recipes."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_recipe_restock_batch_mcp import sample


def saved():
    value = sample(0)
    value['sections']['bags'] = {'0': {'1': {'item_id': 1, 'count': 1}}}
    entries = value['sections']['profession_recipe_materials']['entries']
    entries['200'] = {'status': 'fixed_items', 'reagents': {'2': 1}, 'output_item_id': 10,
                      'base_output_quantity': {'minimum': 1, 'maximum': 1}}
    value['sections'].update(wallet={'copper': 10}, merchant_stock={
        '1': {'item_id': 1, 'bundle_size': 2, 'price_copper': 5, 'available': -1, 'extended_cost': False},
        '2': {'item_id': 2, 'bundle_size': 1, 'price_copper': 3, 'available': -1, 'extended_cost': False}})
    return value


class RecipeAlternativesTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_integer_cost_oracle_unknowns_guards_and_owner(self):
        root = Path(__file__).resolve().parents[1]
        binary = root/'target/debug/azeroth-operator.exe'
        with tempfile.TemporaryDirectory(dir=root/'local-data') as folder, tempfile.TemporaryDirectory(dir=root/'local-data') as foreign:
            store = SnapshotStore(Path(folder)); value = saved(); key = store.ingest(value)
            other = deepcopy(value); other['character']['name'] = 'PRIVATE_FOREIGN_OWNER'
            foreign_key = SnapshotStore(Path(foreign)).ingest(other)
            before = {p.name:p.read_bytes() for p in Path(folder).rglob('*.json')}
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(snapshot=key, error=False, **extra):
                        reply = await session.call_tool('plan_forever_recipe_restock', {
                            'snapshot_id': snapshot,'recipe_id':100,'target_bag_quantity':4,**extra})
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply)); self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    self.assertNotIn('alternative_recipes',await call())
                    report = (await call(alternative_recipe_ids=[200]))['alternative_recipes']
                    self.assertEqual([r['purchase_copper'] for r in report['candidates']],[10,12])
                    self.assertEqual(report['lowest_requested_quote_recipe_ids'],[100])
                    self.assertEqual(report['lowest_requested_purchase_copper'],10)
                    self.assertEqual([r['affordable_from_saved_wallet'] for r in report['candidates']],[True,False])
                    for owned in range(4):
                        for target in range(1,10):
                            v = saved(); v['sections']['bags']['0']['1']['count'] = owned+1
                            r = (await call(store.ingest(v),target_bag_quantity=target,alternative_recipe_ids=[200]))['alternative_recipes']
                            crafts = (target+1)//2
                            shortage = max(0,2*crafts-(owned+1))
                            expected = [((shortage+1)//2)*5,target*3]
                            self.assertEqual([row['purchase_copper'] for row in r['candidates']],expected)
                            self.assertEqual(r['lowest_requested_purchase_copper'],min(expected))
                    missing = saved(); missing['sections']['merchant_stock'].pop('2')
                    partial = (await call(store.ingest(missing),alternative_recipe_ids=[200]))['alternative_recipes']
                    self.assertIsNone(partial['lowest_requested_quote_recipe_ids'])
                    self.assertEqual(partial['lowest_known_quote_recipe_ids'],[100])
                    unknown = (await call(alternative_recipe_ids=[999]))['alternative_recipes']
                    self.assertEqual(unknown['candidates'][1]['status'],'restock_inputs_unavailable')
                    self.assertIsNone(unknown['lowest_requested_purchase_copper'])
                    zero = saved(); zero['sections']['bags']['0']['2'] = {'item_id':10,'count':4}
                    zero['sections'].pop('merchant_stock'); zero['sections'].pop('wallet')
                    met = (await call(store.ingest(zero),alternative_recipe_ids=[200]))['alternative_recipes']
                    self.assertEqual(met['lowest_requested_purchase_copper'],0)
                    self.assertEqual(met['lowest_requested_quote_recipe_ids'],[100,200])
                    self.assertTrue(all(row['affordable_from_saved_wallet'] is None for row in met['candidates']))
                    for opts in [{'alternative_recipe_ids':[]},{'alternative_recipe_ids':[100]},
                        {'alternative_recipe_ids':[200,200]},{'alternative_recipe_ids':[200]*8},
                        {'alternative_recipe_ids':[True]},{'alternative_recipe_ids':['PRIVATE']},
                        {'alternative_recipe_ids':{'PRIVATE':1}}, {'alternative_recipe_ids':[0]},
                        {'alternative_recipe_ids':[200],'stock_schedule':{}},
                        {'alternative_recipe_ids':[200],'additional_targets':[{'recipe_id':200,'target_bag_quantity':1}]}]:
                        await call(error=True,**opts)
                    mismatch = saved(); mismatch['sections']['profession_recipe_materials']['entries']['200']['output_item_id']=20
                    await call(store.ingest(mismatch),error=True,alternative_recipe_ids=[200])
                    await call(foreign_key,error=True,alternative_recipe_ids=[200])
                    await call('../../PRIVATE',error=True,alternative_recipe_ids=[200])
            for name,data in before.items():
                self.assertEqual(next(Path(folder).rglob(name)).read_bytes(),data)
