"""Acquisition recipe projections retain selected capacity and cooldown meaning."""
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


class CatalogueRecipeCapacityTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_selected_projection_guards_and_bulk_separation(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = root/'local-data/forever/catalogue.json'
        if not catalogue.is_file(): self.skipTest('Pinned local catalogue required; no download')
        binary = (root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base = fixture()
        base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        base['character']['name'] = 'PRIVATE_CHARACTER'
        base['sections']['bags'] = {'0':{'1':{'item_id':2840,'count':6}}}
        base['sections']['selected_recipe'] = {'recipe_id':2675,'name':'PRIVATE_RECIPE',
            'learned':True,'requirements':'fixed_items','reagents':{},'output_item_id':2870,
            'base_output_quantity':{'minimum':1,'maximum':1},
            'cooldown':{'charges':1,'max_charges':2,'remaining_seconds':60}}
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder)); keys = {}
            for kind in ['free','zero','unknown_charges','required','missing_stock','unsupported',
                'wrong_recipe','wrong_output','unknown_output','unlearned','learning_conflict','bulk_only','both','missing']:
                saved = deepcopy(base); row = saved['sections']['selected_recipe']
                if kind=='zero': row['cooldown']['charges'] = 0
                if kind=='unknown_charges': row['cooldown'] = {'remaining_seconds':0}
                if kind in ['required','missing_stock']: row['reagents'] = {'2840':2}
                if kind=='missing_stock': saved['sections'].pop('bags')
                if kind=='unsupported': row['requirements'] = 'unsupported_required_slots'
                if kind=='wrong_recipe': row['recipe_id'] = 999
                if kind=='wrong_output': row['output_item_id'] = 3844
                if kind=='unknown_output':
                    row.pop('output_item_id'); row.pop('base_output_quantity')
                if kind=='unlearned': row['learned'] = False
                if kind=='learning_conflict': saved['sections']['tracked_recipes'] = {
                    'coverage':'player_tracked_recipes_only','crafts':{'2675':{'learned':False}},'recrafts':{}}
                if kind in ['bulk_only','both']:
                    saved['sections']['profession_recipe_materials'] = {
                        'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
                            '2675':{'status':'fixed_items','reagents':{},'output_item_id':2870,
                                'base_output_quantity':{'minimum':1,'maximum':1},
                                'cooldown':{'charges':0,'max_charges':2,'remaining_seconds':0}}}}
                if kind in ['bulk_only','missing']: saved['sections'].pop('selected_recipe')
                keys[kind] = store.ingest(saved)
            before = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for kind,key in keys.items():
                        reply = await session.call_tool('lookup_forever_catalogue',{
                            'section':'items','key':'2870','acquisition_snapshot_id':key})
                        self.assertFalse(reply.isError,reply); self.assertNotIn('PRIVATE',str(reply))
                        result = json.loads(reply.content[0].text)
                        source = next(row for row in result['acquisition_progress']['sources'] if row.get('recipe_spell_id')==2675)
                        absent = kind in ['wrong_recipe','wrong_output','bulk_only','missing']
                        if absent:
                            for field in ['material_capacity_basis','material_and_charge_capacity','captured_cooldown_state']:
                                self.assertIsNone(source[field],(kind,field))
                        else:
                            review = await session.call_tool('review_forever_recipe',{'snapshot_id':key,'crafts':1})
                            self.assertFalse(review.isError,review)
                            direct = json.loads(review.content[0].text)
                            for field in ['material_capacity','material_capacity_basis','material_and_charge_capacity','captured_cooldown_state']:
                                self.assertEqual(source[field],direct[field],(kind,field))
                        if kind=='learning_conflict': self.assertTrue(source['recipe_learning']['learning_conflict'])
                        if kind=='both':
                            self.assertEqual(source['captured_cooldown_state'],'available')
                            self.assertEqual(source['material_and_charge_capacity']['bags']['supported_crafts'],1)
                            bulk = source['bulk_recipe_review']['review']
                            self.assertEqual(bulk['captured_cooldown_state'],'waiting')
                            self.assertEqual(bulk['material_and_charge_capacity']['bags']['supported_crafts'],0)
                        self.assertEqual(result['acquisition_progress']['captured_at'],base['captured_at'])
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
