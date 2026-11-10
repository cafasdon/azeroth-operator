"""Pending attachments annotate recipe preparation but never become crafting stock."""
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
from test_recipe_dependencies import recipe


def mail(messages,total=None):
    return {'coverage':'loaded_inbox_attachments','loaded_messages':len(messages),
            'total_messages':len(messages) if total is None else total,'messages':messages}


def without_observations(value):
    if isinstance(value,dict):
        return {k:without_observations(v) for k,v in value.items()
                if k not in {'pending_mail_material_observations','snapshot_id','inventory_snapshot_id'}}
    if isinstance(value,list):return [without_observations(v) for v in value]
    return value


class RecipePendingMailTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_annotations_do_not_change_stock_plans(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            retail_id=store.ingest(fixture())
            base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            base['sections']['selected_recipe']=recipe(100,900,{'10':2,'720':3,'2':1})
            base['sections']['bags']={'0':{'1':{'item_id':10,'count':1},'2':{'item_id':720,'count':1},'3':{'item_id':2,'count':1}}}
            base['sections']['character_bank']={'1':{'1':{'item_id':10,'count':1}}}
            base['sections']['wallet']={'copper':100}
            base['sections']['merchant_stock']={str(i):{'item_id':item,'bundle_size':1,'price_copper':2,
                'available':-1,'extended_cost':False} for i,item in enumerate([2,10,720],1)}
            no_mail=store.ingest(base)
            observed=deepcopy(base)
            observed['sections']['bags']['0']['1'].update(link='item:10:0',reported_stack_limit=4)
            observed['sections']['bag_capacity']={
                '0':{'total_slots':16,'free_slots':1,'family_mask':0},
                **{str(bag):{'total_slots':0,'free_slots':0} for bag in range(1,5)}}
            observed['sections']['mail_attachments']=mail({
                '1':{'1':{'kind':'item','item_id':10,'count':2},'2':{'kind':'currency','currency_id':720,'count':999},'16':{'kind':'unknown'}},
                '2':{'1':{'kind':'item','item_id':10,'count':5},'2':{'kind':'item','item_id':2,'count':9},
                     '3':{'kind':'item','item_id':999,'count':666}}},6)
            with_mail=store.ingest(observed)
            empty=deepcopy(observed);empty['sections']['mail_attachments']=mail({});empty_id=store.ingest(empty)
            unsupported=deepcopy(observed);unsupported['sections']['selected_recipe'].update(requirements='unsupported_required_slots',reagents={})
            unsupported_id=store.ingest(unsupported)
            extra=deepcopy(base);extra['sections']['selected_recipe']=recipe(101,901,{'10':1,'720':1})
            extra['sections']['mail_attachments']=mail({'1':{'1':{'kind':'item','item_id':720,'count':123}}})
            extra_id=store.ingest(extra)
            inventory=deepcopy(base);inventory['captured_at']='2025-01-02T00:00:00Z'
            inventory['sections'].pop('selected_recipe');inventory['sections']['bags']={};inventory['sections']['character_bank']={}
            inventory_no_mail=store.ingest(inventory)
            inventory['sections']['mail_attachments']=mail({'1':{'1':{'kind':'item','item_id':720,'count':11}}})
            inventory_id=store.ingest(inventory)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def mail_space(key, ids, section='mail_attachments', error=False):
                        reply=await session.call_tool('inspect_snapshot',{
                            'snapshot_id':key,'section':section,'mail_attachment_bag_space_item_ids':ids})
                        self.assertEqual(bool(reply.isError),error,reply)
                        if not error:
                            return json.loads(reply.content[0].text)['mail_attachment_bag_space_projection']
                    space=await mail_space(with_mail,[10,720])
                    self.assertEqual([row['known_pending_quantity'] for row in space['items']],[7,0])
                    self.assertEqual(space['unloaded_messages'],4)
                    self.assertEqual(space['unknown_attachment_slots'],1)
                    self.assertEqual(space['estimated_new_stack_slots'],2)
                    self.assertEqual(space['estimated_new_stack_slots_if_same_item_stacks_merge'],1)
                    self.assertTrue(space['fits_recorded_general_slots_if_same_item_stacks_merge'])
                    self.assertIsNone((await mail_space(with_mail,[10,2]))['estimated_new_stack_slots'])
                    self.assertEqual((await mail_space(empty_id,[10]))['estimated_new_stack_slots'],0)
                    self.assertIsNone(await mail_space(no_mail,[10]))
                    for invalid in [[],[10,10],[0],list(range(1,66))]:
                        await mail_space(with_mail,invalid,error=True)
                    await mail_space(with_mail,[10],section='bags',error=True)
                    await mail_space(retail_id,[10],error=True)
                    async def review(key,**arguments):
                        response=await session.call_tool('review_forever_recipe',{'snapshot_id':key,'crafts':2,**arguments})
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)
                    baseline=await review(no_mail,merchant_reagent_plan=True)
                    result=await review(with_mail,merchant_reagent_plan=True)
                    self.assertIsNone(baseline['pending_mail_material_observations'])
                    self.assertEqual(without_observations(result),without_observations(baseline))
                    self.assertEqual(result['pending_mail_material_observations'],{
                        'coverage':'loaded_inbox_attachments','captured_at':'2025-01-01T00:00:00Z',
                        'loaded_messages':2,'total_messages':6,'unloaded_messages':4,
                        'unknown_attachment_slots':1,'counts_toward_material_capacity':False,
                        'items':[{'item_id':2,'known_quantity':9,'known_attachment_slots':1},
                                 {'item_id':10,'known_quantity':7,'known_attachment_slots':2}],
                        'unobserved_reagent_ids':[720]})
                    self.assertFalse(result['craftability_verified'])
                    self.assertEqual((await review(empty_id))['pending_mail_material_observations']['unobserved_reagent_ids'],[2,10,720])
                    self.assertIsNone((await review(unsupported_id))['pending_mail_material_observations'])
                    async def restock(key, include=True, **options):
                        response = await session.call_tool('inspect_snapshot', {
                            'snapshot_id':key,'section':'bags',
                            'bag_restock_requirements':[
                                {'item_id':10,'quantity':4},{'item_id':720,'quantity':2},{'item_id':2,'quantity':1}],
                            'bag_restock_merchant_snapshot_id':no_mail,
                            'bag_restock_pending_mail':include,**options})
                        self.assertFalse(response.isError,response)
                        return json.loads(response.content[0].text)

                    annotated = await restock(with_mail)
                    unannotated = await restock(with_mail, False)
                    field = 'bag_restock_pending_mail_observations'
                    self.assertNotIn(field,unannotated)
                    for unchanged in ['bag_restock_review','bag_restock_merchant_quote']:
                        self.assertEqual(annotated[unchanged],unannotated[unchanged])
                    pending = annotated[field]
                    self.assertEqual(pending['items'], [
                        {'item_id':10,'shortfall_after_observed_bank_withdrawals':2,
                         'known_pending_quantity':7,'known_attachment_slots':2},
                        {'item_id':720,'shortfall_after_observed_bank_withdrawals':1,
                         'known_pending_quantity':None,'known_attachment_slots':None}])
                    self.assertEqual(pending['unloaded_messages'],4)
                    self.assertIs(pending['counts_toward_restock_stock'],False)
                    self.assertIsNone((await restock(no_mail))[field])
                    self.assertIsNone((await restock(empty_id))[field]['items'][0]['known_pending_quantity'])
                    self.assertTrue((await session.call_tool('inspect_snapshot',{
                        'snapshot_id':with_mail,'section':'bags','bag_restock_pending_mail':True})).isError)
                    separate = await restock(no_mail,bag_restock_mail_snapshot_id=inventory_id)
                    separate_pending = separate[field]
                    self.assertEqual(separate_pending['snapshot_id'],inventory_id)
                    self.assertEqual(separate_pending['captured_at'],inventory['captured_at'])
                    self.assertEqual(separate_pending['items'][1]['known_pending_quantity'],11)
                    plain = await restock(no_mail,False)
                    for unchanged in ['bag_restock_review','bag_restock_merchant_quote']:
                        self.assertEqual(separate[unchanged],plain[unchanged])
                    self.assertIsNone((await restock(with_mail,bag_restock_mail_snapshot_id=no_mail))[field])
                    for options in [
                        {'bag_restock_pending_mail':False,'bag_restock_mail_snapshot_id':inventory_id},
                        {'bag_restock_pending_mail':True,'bag_restock_mail_snapshot_id':'0'*64},
                    ]:
                        reply = await session.call_tool('inspect_snapshot',{
                            'snapshot_id':with_mail,'section':'bags',
                            'bag_restock_requirements':[{'item_id':10,'quantity':4}],**options})
                        self.assertTrue(reply.isError)
                    batch={'additional_recipes':[{'snapshot_id':extra_id,'crafts':2}],
                           'suggest_dependency_order':True,'merchant_reagent_plan':True}
                    earlier=await review(no_mail,inventory_snapshot_id=inventory_no_mail,**batch)
                    later=await review(with_mail,inventory_snapshot_id=inventory_id,**batch)
                    self.assertEqual(without_observations(earlier),without_observations(later))
                    for entry in [later,*later['additional_recipe_reviews']]:
                        observation=entry['pending_mail_material_observations']
                        self.assertEqual(observation['captured_at'],'2025-01-02T00:00:00Z')
                        self.assertEqual(observation['items'],[{'item_id':720,'known_quantity':11,'known_attachment_slots':1}])
                        self.assertFalse(observation['counts_toward_material_capacity'])
                    for change in [('character','name','Other'),('client','build','other')]:
                        bad=deepcopy(inventory);bad[change[0]][change[1]]=change[2]
                        response=await session.call_tool('review_forever_recipe',{'snapshot_id':with_mail,'crafts':2,
                            'inventory_snapshot_id':store.ingest(bad)})
                        self.assertTrue(response.isError)


if __name__=='__main__':unittest.main()
