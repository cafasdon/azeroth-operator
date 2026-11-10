"""Loaded attachment observations do not imply deliveries or inventory transfers."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


def item(identifier,count):return {'kind':'item','item_id':identifier,'count':count}
def currency(identifier,count):return {'kind':'currency','currency_id':identifier,'count':count}


class MailComparisonTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_known_totals_and_partial_coverage(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            def capture(messages,total=None,timestamp='2025-01-01T00:00:00Z',**changes):
                value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
                value['captured_at']=timestamp
                if messages is not None:
                    value['sections']['mail_attachments']={'coverage':'loaded_inbox_attachments',
                        'loaded_messages':len(messages),'total_messages':len(messages) if total is None else total,
                        'messages':messages}
                for field,change in changes.items():value[field].update(change)
                return store.ingest(value)
            old={'1':{'1':item(720,2),'2':currency(720,4),'3':currency(3,1),'16':{'kind':'unknown'}},
                 '2':{'1':item(720,5),'2':item(10,3),'3':item(2,1)}}
            new={'1':{'1':item(720,3),'2':currency(720,2),'3':currency(20,2),'15':{'kind':'unknown'}},
                 '2':{'1':item(720,3),'2':item(10,3),'3':item(30,9),'4':currency(720,2),'16':{'kind':'unknown'}}}
            before=capture(old,total=4);after=capture(new,total=5)
            moved={str(3-int(message)):{str(17-int(slot)):row for slot,row in rows.items()}
                   for message,rows in old.items()}
            moved_key=capture(moved,total=4)
            missing=capture(None);empty=capture({})
            unknown=capture({'1':{'1':{'kind':'unknown'}}})
            huge=capture({str(message):{str(slot):item(2**32-1,2**32-1) for slot in range(1,17)}
                          for message in range(1,101)})
            tiny=capture({'1':{'1':item(2**32-1,1)}})
            carried_before=capture(old,sections={'bags':{'0':{'1':{'item_id':720,'count':1}}}})
            carried_after=capture(new,sections={'bags':{'0':{'1':{'item_id':720,'count':3}}}})
            def balance_capture(messages, quantity, account_wide):
                return capture(messages,sections={'currency_balances':{
                    'coverage':'visible_currency_ids_only','entries':{'720':{
                        'name':'Fixture currency','quantity':quantity,'account_wide':account_wide,'inactive':False,
                        'max_quantity':6,'max_weekly_quantity':0,'earned_this_week':0,
                        'total_earned':0,'cap_uses_total_earned':False,
                    }},
                }})
            balance_before=balance_capture(old,5,True)
            balance_after=balance_capture(new,8,True)
            balance_other_scope=balance_capture(new,8,False)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def compare(a,b,enabled=True):
                        args={'before_id':a,'after_id':b}
                        if enabled:args['mail_attachment_changes']=True
                        response=await session.call_tool('compare_snapshots',args)
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)
                    delta=(await compare(before,after))['mail_attachment_changes']
                    reply = await session.call_tool('compare_snapshots',{
                        'before_id':carried_before,'after_id':carried_after,
                        'mail_attachment_item_id':720,'mail_attachment_compare_carried_item':True,
                    })
                    self.assertFalse(reply.isError,reply)
                    focused = json.loads(reply.content[0].text)['mail_attachment_item_comparison']
                    self.assertEqual(focused['known_quantity_delta'],-1)
                    self.assertEqual(focused['carried_comparison'],{
                        'before_quantity':1,'after_quantity':3,'quantity_delta':2,
                        'mail_counts_toward_carried_stock':False,'inventory_transfers_verified':False,
                    })
                    rejected = await session.call_tool('compare_snapshots',{
                        'before_id':before,'after_id':after,'mail_attachment_currency_id':720,
                        'mail_attachment_compare_carried_item':True,
                    })
                    self.assertTrue(rejected.isError,rejected)
                    reply = await session.call_tool('compare_snapshots',{
                        'before_id':before,'after_id':after,
                        'mail_attachment_item_id':720,'mail_attachment_currency_id':720,
                    })
                    self.assertFalse(reply.isError,reply)
                    focused = json.loads(reply.content[0].text)
                    self.assertNotIn('mail_attachment_changes',focused)
                    currency_comparison = focused['mail_attachment_currency_comparison']
                    self.assertEqual(currency_comparison['currency_id'],720)
                    self.assertEqual(currency_comparison['before']['known_quantity'],4)
                    self.assertEqual(currency_comparison['after']['known_quantity'],4)
                    self.assertEqual(currency_comparison['before']['known_attachment_slots'],1)
                    self.assertEqual(currency_comparison['after']['known_attachment_slots'],2)
                    self.assertEqual(currency_comparison['known_quantity_delta'],0)
                    self.assertEqual(focused['mail_attachment_item_comparison']['known_quantity_delta'],-1)
                    for target, same_scope, balance_delta in [
                        (balance_after,True,3),(balance_other_scope,False,None),(after,None,None),
                    ]:
                        reply = await session.call_tool('compare_snapshots',{
                            'before_id':balance_before,'after_id':target,'mail_attachment_currency_id':720,
                            'mail_attachment_compare_currency_balance':True,
                        })
                        self.assertFalse(reply.isError,reply)
                        focused = json.loads(reply.content[0].text)['mail_attachment_currency_comparison']
                        self.assertEqual(focused['known_quantity_delta'],0)
                        balance = focused['balance_comparison']
                        self.assertEqual(balance['before'],{'quantity':5,'account_wide':True})
                        self.assertIs(balance['same_scope'],same_scope)
                        self.assertEqual(balance['quantity_delta'],balance_delta)
                        self.assertFalse(balance['mail_counts_toward_currency_balance'])
                        self.assertFalse(balance['currency_transfers_verified'])
                        caps = focused['hypothetical_cap_projection']
                        self.assertFalse(caps['mail_cap_application_verified'])
                        self.assertEqual(caps['before'],{
                            'proposed_quantity':4,'quantity_within_recorded_headroom':1,
                            'quantity_above_recorded_headroom':3,
                        })
                        self.assertEqual(caps['after'],None if target == after else {
                            'proposed_quantity':4,'quantity_within_recorded_headroom':0,
                            'quantity_above_recorded_headroom':4,
                        })
                    rejected = await session.call_tool('compare_snapshots',{
                        'before_id':before,'after_id':after,'mail_attachment_item_id':720,
                        'mail_attachment_compare_currency_balance':True,
                    })
                    self.assertTrue(rejected.isError,rejected)
                    for a,b,expected_before,expected_after,expected_delta in [
                        (before,after,7,6,-1),(before,empty,7,None,None),
                        (missing,after,None,6,None),(after,after,6,6,0),
                    ]:
                        reply = await session.call_tool('compare_snapshots',{
                            'before_id':a,'after_id':b,'mail_attachment_item_id':720,
                        })
                        self.assertFalse(reply.isError,reply)
                        focused = json.loads(reply.content[0].text)
                        self.assertNotIn('mail_attachment_changes',focused)
                        item_comparison = focused['mail_attachment_item_comparison']
                        self.assertEqual(item_comparison['item_id'],720)
                        self.assertEqual((item_comparison['before'] or {}).get('known_quantity'),expected_before)
                        self.assertEqual((item_comparison['after'] or {}).get('known_quantity'),expected_after)
                        self.assertEqual(item_comparison['known_quantity_delta'],expected_delta)
                        self.assertFalse(item_comparison['deliveries_or_retrievals_verified'])
                        if a == missing:
                            self.assertIsNone(item_comparison['before'])
                    self.assertEqual(delta,{'coverage':'loaded_inbox_attachments','message_identity':'unverified',
                        'inventory_transfers':'unverified','deliveries_or_retrievals':'unverified',
                        'before_loaded_messages':2,'after_loaded_messages':2,'before_total_messages':4,'after_total_messages':5,
                        'before_unloaded_messages':2,'after_unloaded_messages':3,
                        'before_unknown_attachment_slots':1,'after_unknown_attachment_slots':2,
                        'observed_only_before':[{'kind':'currency','id':3,'known_quantity':1,'attachment_slots':1},
                                                {'kind':'item','id':2,'known_quantity':1,'attachment_slots':1}],
                        'observed_only_after':[{'kind':'currency','id':20,'known_quantity':2,'attachment_slots':1},
                                               {'kind':'item','id':30,'known_quantity':9,'attachment_slots':1}],
                        'shared_observation_changes':[
                            {'kind':'currency','id':720,'before_known_quantity':4,'after_known_quantity':4,'known_quantity_delta':0,
                             'before_attachment_slots':1,'after_attachment_slots':2},
                            {'kind':'item','id':720,'before_known_quantity':7,'after_known_quantity':6,'known_quantity_delta':-1,
                             'before_attachment_slots':2,'after_attachment_slots':2}]})
                    reordered=(await compare(before,moved_key))['mail_attachment_changes']
                    for field in ['observed_only_before','observed_only_after','shared_observation_changes']:
                        self.assertEqual(reordered[field],[])
                    for a,b in [(before,missing),(missing,after),(missing,missing)]:
                        self.assertIsNone((await compare(a,b))['mail_attachment_changes'])
                    for target,unknown_slots in [(empty,0),(unknown,1)]:
                        result=(await compare(before,target))['mail_attachment_changes']
                        self.assertEqual(result['shared_observation_changes'],[])
                        self.assertEqual([(row['kind'],row['id']) for row in result['observed_only_before']],
                            [('currency',3),('currency',720),('item',2),('item',10),('item',720)])
                        self.assertEqual(result['after_unknown_attachment_slots'],unknown_slots)
                        self.assertTrue(all('known_quantity_delta' not in row for row in result['observed_only_before']))
                    empty_comparison=(await compare(empty,empty))['mail_attachment_changes']
                    self.assertEqual(empty_comparison['observed_only_after'],[])
                    self.assertEqual(empty_comparison['after_unloaded_messages'],0)
                    self.assertNotIn('mail_attachment_changes',await compare(before,after,enabled=False))
                    for a,b,sign in [(tiny,huge,1),(huge,tiny,-1)]:
                        result=(await compare(a,b))['mail_attachment_changes']['shared_observation_changes'][0]
                        self.assertEqual(result['known_quantity_delta'],sign*(1600*(2**32-1)-1))
                    retail=store.ingest(fixture())
                    for a,b in [(retail,retail),(before,capture(new,character={'name':'Other'})),
                                (before,capture(new,character={'realm':'Other'})),
                                (before,capture(new,client={'build':'different'})),
                                (capture(new,timestamp='2025-01-02T00:00:00Z'),before)]:
                        response=await session.call_tool('compare_snapshots',{'before_id':a,'after_id':b,'mail_attachment_changes':True})
                        self.assertTrue(response.isError)


if __name__=='__main__':unittest.main()
