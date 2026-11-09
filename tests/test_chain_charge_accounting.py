"""Repeated captured producers share saved charges, never imaginary recharge."""
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


class ChainChargeAccountingTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_repeated_producer_cumulative_charges(self):
        binary = (Path(__file__).resolve().parents[1]/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base = fixture()
        base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        base['character']['name'] = 'PRIVATE_CHARACTER'
        base['sections']['bags'] = {'0':{'1':{'item_id':10,'count':20}}}
        base['sections']['profession_recipe_materials'] = {
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
                '101':fixed(100,{'20':1,'30':1}),
                '102':fixed(20,{'10':1}),
                '103':fixed(30,{'20':1})}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            keys = {}
            for charges in [None,0,1,2,3,4294967295]:
                saved = deepcopy(base)
                if charges is not None:
                    saved['sections']['profession_recipe_materials']['entries']['102']['cooldown'] = {
                        'charges':charges,'max_charges':max(1,charges),'remaining_seconds':60}
                keys[charges] = store.ingest(saved)
            future = deepcopy(base)
            future['captured_at'] = '2025-01-02T00:00:00Z'
            future['sections']['profession_recipe_materials']['entries']['102']['cooldown'] = {
                'charges':9,'max_charges':9,'remaining_seconds':0}
            inventory = store.ingest(future)
            before = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def plan(key,**extra):
                        reply = await session.call_tool('review_forever_recipe',{
                            'snapshot_id':key,'recipe_id':101,'crafts':1,
                            'suggest_reagent_chain':True,'reagent_chain_stock_plan':True,**extra})
                        self.assertFalse(reply.isError,reply)
                        self.assertNotIn('PRIVATE',str(reply))
                        return json.loads(reply.content[0].text)['reagent_chain']
                    # Red regression: both requests individually fit one charge, but the second reuses it.
                    one = await plan(keys[1])
                    repeated = [row for row in one['steps'] if row['recipe_id']==102]
                    self.assertEqual(len(repeated),2)
                    self.assertTrue(repeated[0]['captured_nonmaterial_checks']['observed_charges_cover_request'])
                    self.assertTrue(repeated[1]['captured_nonmaterial_checks']['observed_charges_cover_request'])
                    self.assertIs(repeated[1]['captured_nonmaterial_checks'].get('observed_charges_cover_cumulative_crafts'),False,
                        'second producer step must not reuse the first saved charge')
                    for crafts,charges,key in [(count,n,key) for count in [1,2,1000] for n,key in keys.items()]:
                        chain = await plan(key,crafts=crafts)
                        self.assertTrue(chain['draft_complete'])  # Conditional material draft, not readiness.
                        self.assertEqual([row['recipe_id'] for row in chain['steps']],[102,102,103,101])
                        ledger = chain['stock_plan']['nonmaterial_requirements']['charge_accounting']
                        totals = {}
                        for index,(step,row) in enumerate(zip(chain['steps'],ledger,strict=True),1):
                            rid = step['recipe_id']; count = step['crafts']
                            prior = totals.get(rid,0); totals[rid] = prior+count
                            captured = charges if rid==102 else None
                            expected = {'requested_crafts_through_step':totals[rid],
                                'observed_charges':captured,
                                'observed_charges_before_step':None if captured is None else max(0,captured-prior),
                                'observed_charges_remaining_if_completed':None if captured is None else max(0,captured-totals[rid]),
                                'step_crafts_covered_by_observed_charges':None if captured is None else min(count,max(0,captured-prior)),
                                'shortfall_vs_cumulative_crafts':None if captured is None else max(0,totals[rid]-captured),
                                'covers_cumulative_crafts':None if captured is None else captured>=totals[rid]}
                            self.assertEqual(step['captured_charge_accounting'],expected)
                            self.assertEqual(row,{'recipe_number':index,'recipe_id':rid,**expected})
                            self.assertIs(step['captured_nonmaterial_checks']['observed_charges_cover_cumulative_crafts'],expected['covers_cumulative_crafts'])
                        failure = chain['stock_plan']['nonmaterial_requirements']['first_observed_shortfall']
                        if charges is not None and charges < 2*crafts:
                            self.assertEqual(failure['recipe_number'],1 if charges < crafts else 2)
                            self.assertIn('observed_charges_cover_cumulative_crafts',failure['failed_checks'])
                        else: self.assertIsNone(failure)
                        self.assertFalse(chain['craftability_verified']); self.assertFalse(chain['game_execution'])
                    later = await plan(keys[1],inventory_snapshot_id=inventory)
                    self.assertEqual(later['recipe_captured_at'],base['captured_at'])
                    self.assertEqual(later['inventory_captured_at'],future['captured_at'])
                    self.assertEqual(later['steps'][1]['captured_charge_accounting']['observed_charges'],1)
                    ordinary = await plan(keys[1],reagent_chain_stock_plan=False)
                    self.assertNotIn('stock_plan',ordinary)
                    self.assertEqual(ordinary['steps'],one['steps'])
                    batch = await session.call_tool('review_forever_recipe',{
                        'snapshot_id':keys[1],'recipe_id':101,'crafts':1,
                        'additional_recipes':[{'snapshot_id':keys[1],'recipe_id':102,'crafts':1},
                            {'snapshot_id':keys[1],'recipe_id':103,'crafts':1}]})
                    self.assertFalse(batch.isError,batch)
                    self.assertEqual(json.loads(batch.content[0].text)['ordered_nonmaterial_requirements'],{
                        'first_observed_shortfall':None,'recipes_with_unknown_checks':3})
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
