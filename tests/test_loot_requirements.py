"""Readable loot can cover a conditional request without becoming owned stock."""
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


class LootRequirementTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_conditional_coverage_unknowns_limits_and_no_stock_credit(self):
        binary=(Path(__file__).resolve().parents[1]/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['bags']={'0':{'1':{'item_id':10,'count':2,'link':'item:10:0','reported_stack_limit':6}}}
        base['sections']['bag_capacity']={
            '0':{'total_slots':16,'free_slots':1,'family_mask':0},
            **{str(bag):{'total_slots':0,'free_slots':0} for bag in range(1,5)}}
        base['sections']['selected_recipe']={'recipe_id':101,'name':'Fixture','learned':True,
            'requirements':'fixed_items','reagents':{'10':3}}
        entries={str(i):{'status':'unreadable_or_non_item'} for i in range(1,11)}
        for slot,item,quantity in [(1,10,4),(2,10,3),(3,20,0),(10,10,5)]:
            entries[str(slot)]={'status':'item','item_id':item,'quantity':quantity}
        section={'coverage':'visible_loot_window_slots','slot_count':10,'entries':entries}
        base['sections']['loot_window']=section
        requirements=[{'item_id':20,'quantity':1},{'item_id':10,'quantity':10},{'item_id':30,'quantity':5}]
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));cases=[]
            for mode in ['mixed','missing','empty','unknown','maximum','distinct']:
                saved=deepcopy(base)
                if mode=='missing':saved['sections'].pop('loot_window')
                if mode=='empty':saved['sections']['loot_window']={'coverage':section['coverage'],'slot_count':0,'entries':{}}
                if mode=='unknown':saved['sections']['loot_window']['entries']={str(i):{'status':'unreadable_or_non_item'} for i in range(1,11)}
                if mode=='maximum':saved['sections']['loot_window']={'coverage':section['coverage'],'slot_count':64,'entries':{
                    str(i):{'status':'item','item_id':10,'quantity':4294967295} for i in range(1,65)}}
                if mode=='distinct':saved['sections']['loot_window']={'coverage':section['coverage'],'slot_count':64,'entries':{
                    str(i):{'status':'item','item_id':i,'quantity':1} for i in range(1,65)}}
                cases.append((mode,store.ingest(saved),saved['sections'].get('loot_window')))
            retail=fixture();retail_key=store.ingest(retail)
            before={p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        return None if error else json.loads(reply.content[0].text)
                    async def inspect(key,demands=None,**extra):
                        args={'snapshot_id':key,'section':'loot_window',**extra}
                        if demands is not None:args['loot_requirements']=demands
                        return await call('inspect_snapshot',**args)
                    keys = {mode:key for mode,key,_ in cases}
                    space=(await inspect(keys['mixed'],requirements,loot_estimate_bag_space=True))[
                        'loot_requirement_review']['bag_space_projection']
                    self.assertEqual([row['readable_requested_quantity'] for row in space['items']],[0,10,0])
                    self.assertEqual(space['estimated_new_stack_slots'],2)
                    self.assertFalse(space['fits_recorded_general_slots'])
                    self.assertEqual(space['estimated_new_stack_slots_if_same_item_stacks_merge'],1)
                    self.assertTrue(space['fits_recorded_general_slots_if_same_item_stacks_merge'])
                    unknown=(await inspect(keys['distinct'],requirements,loot_estimate_bag_space=True))[
                        'loot_requirement_review']['bag_space_projection']
                    self.assertIsNone(unknown['estimated_new_stack_slots'])
                    self.assertIsNone((await inspect(keys['missing'],requirements,loot_estimate_bag_space=True))[
                        'loot_requirement_review'])
                    await call('inspect_snapshot',error=True,snapshot_id=keys['mixed'],section='loot_window',
                               loot_estimate_bag_space=True)
                    restock = {'snapshot_id':keys['missing'],'section':'bags',
                        'bag_restock_requirements':[{'item_id':10,'quantity':10},{'item_id':20,'quantity':1}]}
                    plain = await call('inspect_snapshot',**restock)
                    for mode, readable in [('mixed',12),('empty',0),('unknown',0)]:
                        result = await call('inspect_snapshot',**restock,bag_restock_loot_snapshot_id=keys[mode])
                        self.assertEqual(result['bag_restock_review'],plain['bag_restock_review'])
                        loot = result['bag_restock_loot_review']
                        self.assertEqual(loot['snapshot_id'],keys[mode])
                        self.assertEqual(loot['items'][0]['requested_quantity'],8)
                        self.assertEqual(loot['items'][0]['readable_quantity'],readable)
                        self.assertIs(loot['counts_toward_owned_inventory'],False)
                    missing = await call('inspect_snapshot',**restock,bag_restock_loot_snapshot_id=keys['missing'])
                    self.assertIsNone(missing['bag_restock_loot_review'])
                    await call('inspect_snapshot',error=True,**restock,bag_restock_loot_snapshot_id=retail_key)
                    await call('inspect_snapshot',error=True,snapshot_id=keys['missing'],section='bags',
                               bag_restock_loot_snapshot_id=keys['mixed'])
                    for mode,key,loot in cases:
                        plain=await inspect(key)
                        self.assertNotIn('loot_requirement_review',plain)
                        null_option=await inspect(key,loot_requirements=None)
                        # Capture age is recomputed per call, including across a second boundary.
                        for response in [plain,null_option]:
                            self.assertIs(type(response.pop('age_seconds')),int)
                        self.assertEqual(plain,null_option)
                        reqs=([{'item_id':i,'quantity':1} for i in range(1,65)] if mode=='distinct' else
                            [{'item_id':10,'quantity':18446744073709551615}] if mode=='maximum' else requirements)
                        result=await inspect(key,reqs);review=result['loot_requirement_review']
                        if loot is None:
                            self.assertIsNone(review);self.assertFalse(result['available']);continue
                        self.assertEqual(result['data'],loot)
                        expected=[]
                        for demand in reqs:
                            matches=sorted((int(slot),row['quantity']) for slot,row in loot['entries'].items()
                                if row['status']=='item' and row['item_id']==demand['item_id'])
                            total=sum(n for _,n in matches);requested=demand['quantity']
                            expected.append({'item_id':demand['item_id'],'requested_quantity':requested,
                                'readable_quantity':total,'matching_slots':[{'slot':slot,'quantity':n} for slot,n in matches],
                                'covered_quantity_if_readable_items_acquired':min(requested,total),
                                'remaining_request_if_readable_items_acquired':max(0,requested-total),
                                'readable_slots_cover_request':total>=requested})
                        self.assertEqual(review['items'],expected,mode)
                        unknown=sum(row['status']!='item' for row in loot['entries'].values())
                        self.assertEqual(review['unreadable_or_non_item_slots'],unknown)
                        self.assertEqual(review['additional_matching_quantities_unknown'],unknown>0)
                        self.assertEqual(review['all_requests_covered_by_readable_slots'],all(row['readable_slots_cover_request'] for row in expected))
                        self.assertEqual(review['captured_at'],base['captured_at'])
                        self.assertEqual(review['grouping'],'base_item_id_only')
                        for flag in ['counts_toward_owned_inventory','ownership_verified','lootability_verified','game_execution','live']:
                            self.assertFalse(review[flag])
                        found=await call('find_snapshot_item',snapshot_id=key,item_id=10)
                        self.assertEqual(found['quantity_by_storage']['bags'],2)
                        recipe=await call('review_forever_recipe',snapshot_id=key,crafts=1)
                        self.assertEqual(recipe['materials'][0]['quantity_observed'],2)
                        self.assertEqual(recipe['material_capacity']['bags']['supported_crafts'],0)
                    key=cases[0][1]
                    for bad in [[],requirements*2,[{'item_id':1,'quantity':1}]*65,
                        [{'item_id':1,'quantity':0}],[{'item_id':0,'quantity':1}],
                        [{'item_id':True,'quantity':1}],[{'item_id':1,'quantity':True}],
                        [{'item_id':1.0,'quantity':1}],[{'item_id':1,'quantity':1.0}],
                        [{'item_id':4294967296,'quantity':1}],[{'item_id':1,'quantity':18446744073709551616}],
                        [{'item_id':1,'quantity':-1}],[{'item_id':1}],
                        [{'item_id':1,'quantity':1,'extra':True}],{}]:
                        await call('inspect_snapshot',error=True,snapshot_id=key,section='loot_window',loot_requirements=bad)
                    await call('inspect_snapshot',error=True,snapshot_id=key,section='bags',loot_requirements=requirements)
                    await call('inspect_snapshot',error=True,snapshot_id=retail_key,section='loot_window',loot_requirements=requirements)
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__':unittest.main()
