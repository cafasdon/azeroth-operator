"""Explicit zero-item recipes differ from unavailable material observations."""
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


class RecipeZeroItemCapacityTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_free_required_unknown_scopes_and_charges(self):
        binary = (Path(__file__).resolve().parents[1]/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base = fixture()
        base['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        base['character']['name'] = 'PRIVATE_CHARACTER'
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder)); cases = []
            for kind in ['free','required','unsupported']:
                for storage in ['missing','empty','bags','bank']:
                    for charges in [0,1,None,4294967295]:
                        for source in ['bulk','selected']:
                            saved = deepcopy(base)
                            for name in ['bags','character_bank','account_bank']: saved['sections'].pop(name,None)
                            if storage=='empty': saved['sections']['bags'] = {}
                            if storage=='bags': saved['sections']['bags'] = {'0':{'1':{'item_id':10,'count':6}}}
                            if storage=='bank': saved['sections']['character_bank'] = {'6':{'1':{'item_id':10,'count':10}}}
                            row = fixed(100,{} if kind=='free' else {'10':2},2)
                            if charges is not None: row['cooldown'] = {'charges':charges,'max_charges':max(1,charges)}
                            if source=='bulk':
                                if kind=='unsupported': row = {'status':'unsupported_required_slots'}
                                saved['sections']['profession_recipe_materials'] = {
                                    'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{'101':row}}
                            else:
                                row.pop('status')
                                row.update(recipe_id=101,name='PRIVATE_RECIPE',learned=True,
                                    requirements='fixed_items' if kind!='unsupported' else 'unsupported_required_slots')
                                if kind=='unsupported': row['reagents'] = {}
                                saved['sections']['selected_recipe'] = row
                            cases.append((store.ingest(saved),kind,storage,charges,source))
            future = deepcopy(base)
            future['captured_at'] = '2025-01-02T00:00:00Z'
            future['sections']['bags'] = {}
            future['sections']['profession_recipe_materials'] = {
                'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
                    '101':{**fixed(100,{},2),'cooldown':{'charges':9,'max_charges':9}}}}
            future_key = store.ingest(future)
            before = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for key,kind,storage,charges,source in cases:
                        request = {'snapshot_id':key,'crafts':3}
                        if source=='bulk': request['recipe_id'] = 101
                        reply = await session.call_tool('review_forever_recipe',request)
                        self.assertFalse(reply.isError,reply)
                        self.assertNotIn('PRIVATE_CHARACTER',str(reply))
                        result = json.loads(reply.content[0].text)
                        for scope in ['bags','all_observed_storage']:
                            quantity = (6 if storage=='bags' else 0 if storage=='empty' else
                                10 if storage=='bank' and scope=='all_observed_storage' else None)
                            capacity = None if kind!='required' or quantity is None else quantity//2
                            combined = (charges if kind=='free' else
                                min(capacity,charges) if capacity is not None and charges is not None else None)
                            if kind=='unsupported': combined = None
                            self.assertEqual(result['material_and_charge_capacity'][scope],None if combined is None else {
                                'supported_crafts':combined,'shortfall_vs_requested':max(0,3-combined)},
                                (kind,storage,charges,source,scope))
                            basis = ('no_required_items' if kind=='free' else
                                'observed_required_items' if capacity is not None else 'unavailable')
                            self.assertEqual(result['material_capacity_basis'][scope],basis)
                            self.assertEqual(None if result['material_capacity'][scope] is None else
                                result['material_capacity'][scope]['supported_crafts'],capacity)
                        self.assertFalse(result['craftability_verified']); self.assertFalse(result['game_execution'])
                    original = next(key for key,kind,storage,charges,source in cases
                        if (kind,storage,charges,source)==('free','missing',1,'bulk'))
                    reply = await session.call_tool('review_forever_recipe',{
                        'snapshot_id':original,'recipe_id':101,'target_output_quantity':5,
                        'inventory_snapshot_id':future_key})
                    self.assertFalse(reply.isError,reply)
                    projected = json.loads(reply.content[0].text)
                    self.assertEqual(projected['requested_crafts'],3)
                    self.assertEqual(projected['material_and_charge_capacity']['bags'],{
                        'supported_crafts':1,'shortfall_vs_requested':2})
                    self.assertEqual(projected['captured_at'],base['captured_at'])
                    self.assertEqual(projected['inventory_captured_at'],future['captured_at'])
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
