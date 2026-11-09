"""Native count observations are historical hints, never material or eligibility gates."""
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


class ReportedCraftableCountTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_counts_origin_requested_delta_and_independent_planning(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['bags'] = {'0':{'1':{'item_id':10,'count':6}}}
        base['sections']['wallet'] = {'copper':4}
        base['sections']['merchant_stock'] = {'1':{'item_id':10,'bundle_size':1,'price_copper':1,
            'available':-1,'extended_cost':False}}
        base['sections']['profession_recipe_materials'] = {
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{'101':fixed(100,{'10':2})}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            cases = {}
            for count in [None,0,1,7,4294967295]:
                saved = deepcopy(base)
                if count is not None: saved['sections']['profession_recipe_materials']['entries']['101']['reported_craftable_count'] = count
                cases[count] = store.ingest(saved)
            inventory = deepcopy(base)
            inventory['captured_at'] = '2025-01-02T00:00:00Z'
            inventory['sections']['bags'] = {}
            inventory['sections']['profession_recipe_materials']['entries']['101']['reported_craftable_count'] = 999
            inventory_id = store.ingest(inventory)
            before = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def review(key,**extra):
                        reply = await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':101,'crafts':3,**extra})
                        self.assertFalse(reply.isError,reply)
                        return json.loads(reply.content[0].text)
                    baseline = await review(cases[None])
                    self.assertNotIn('reported_craftable_count_observation',baseline)
                    for count,key in cases.items():
                        result = await review(key)
                        if count is not None:
                            observed = result.pop('reported_craftable_count_observation')
                            self.assertEqual(observed['count'],count)
                            self.assertEqual(observed['shortfall_vs_requested_crafts'],max(0,3-count))
                            self.assertEqual(observed['captured_at'],base['captured_at'])
                            self.assertFalse(observed['current_craftability_verified'])
                        self.assertEqual(result,baseline)
                        found = await session.call_tool('search_forever_recipes',{'snapshot_id':key,'recipe_id':101})
                        self.assertFalse(found.isError,found)
                        row = json.loads(found.content[0].text)['records'][0]
                        self.assertEqual(row.get('reported_craftable_count'),count)
                    later = await review(cases[7],inventory_snapshot_id=inventory_id)
                    self.assertEqual(later['reported_craftable_count_observation']['count'],7)
                    self.assertEqual(later['reported_craftable_count_observation']['captured_at'],base['captured_at'])
                    self.assertEqual(later['inventory_captured_at'],'2025-01-02T00:00:00Z')
                    self.assertEqual(later['material_capacity']['bags']['supported_crafts'],0)
                    chain = (await review(cases[0],suggest_reagent_chain=True,reagent_chain_stock_plan=True,
                        merchant_reagent_plan=True))['reagent_chain']
                    self.assertTrue(chain['draft_complete'])
                    self.assertTrue(chain['stock_plan']['material_coverage']['bags'])
                    self.assertEqual(chain['merchant_reagent_plan']['unlimited_copper_only_total'],0)
                    self.assertFalse(chain['craftability_verified'])
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
