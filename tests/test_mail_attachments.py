"""Private mail content is excluded from the pending-attachment contract."""
import copy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore, validate_snapshot
from test_snapshots import fixture


class MailAttachmentTests(unittest.IsolatedAsyncioTestCase):
    async def test_pending_attachment_readback_and_rejection(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            value=fixture()
            value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            mail={'coverage':'loaded_inbox_attachments','loaded_messages':1,'total_messages':4,'messages':{
                '1':{'1':{'kind':'item','item_id':720,'count':2},
                     '2':{'kind':'currency','currency_id':720,'count':4},
                     '3':{'kind':'item','item_id':720,'count':5},'16':{'kind':'unknown'}}}}
            value['sections']['mail_attachments']=mail
            key=store.ingest(value)
            capped=copy.deepcopy(value)
            capped['character']['name']='Capped'
            capped['sections']['currency_balances']={'coverage':'visible_currency_ids_only','entries':{
                '720':{'name':'Fixture currency','quantity':2,'max_quantity':4,'max_weekly_quantity':0,
                    'earned_this_week':0,'total_earned':0,'cap_uses_total_earned':False,
                    'account_wide':False,'inactive':False}}}
            capped_key=store.ingest(capped)
            empty=copy.deepcopy(value)
            empty['character']['name']='Empty'
            empty['sections']['mail_attachments'].update(loaded_messages=0,total_messages=0,messages={})
            empty_key=store.ingest(empty)
            missing=copy.deepcopy(value)
            missing['character']['name']='Missing'
            missing['sections'].pop('mail_attachments')
            missing_key=store.ingest(missing)
            later=copy.deepcopy(value)
            later['captured_at']='2025-01-02T00:00:00Z'
            later['sections'].pop('mail_attachments')
            store.ingest(later)
            many=copy.deepcopy(value)
            many['character']['name']='Many'
            many['sections']['mail_attachments'].update(loaded_messages=2,total_messages=5,messages={
                str(message):{str(slot):{'kind':'item','item_id':720,'count':1} for slot in range(1,17)}
                for message in range(1,3)})
            many_key=store.ingest(many)
            bad_keys=[]
            for change in [
                lambda x:x['client'].update(family='retail'),
                lambda x:x['sections']['mail_attachments'].update(sender='private'),
                lambda x:x['sections']['mail_attachments'].update(coverage='complete_inbox'),
                lambda x:x['sections']['mail_attachments'].update(total_messages=0),
                lambda x:x['sections']['mail_attachments'].update(loaded_messages=True),
                lambda x:x['sections']['mail_attachments']['messages']['1']['1'].update(count=0),
                lambda x:x['sections']['mail_attachments']['messages']['1']['1'].update(currency_id=720),
                lambda x:x['sections']['mail_attachments']['messages']['1']['16'].update(item_id=720),
                lambda x:x['sections']['mail_attachments']['messages']['1'].update({'17':{'kind':'unknown'}}),
            ]:
                bad=copy.deepcopy(value);change(bad)
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
                bad_key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/f'{bad_key}.json').write_bytes(raw)
                bad_keys.append(bad_key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def search(**arguments):
                        response=await session.call_tool('list_snapshots',{'family':'forever',**arguments})
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)
                    query={'character':value['character'],'mail_attachment':{'kind':'item','id':720}}
                    rows=(await search(**query))['snapshots']
                    self.assertEqual([row['snapshot_id'] for row in rows],[key])
                    match=rows[0]['mail_attachment_match']
                    self.assertEqual(match['quantity'],7)
                    self.assertEqual(match['matched_locations'],2)
                    self.assertEqual([row['attachment_slot'] for row in match['locations']],[1,3])
                    self.assertEqual(match['total_messages'],4)
                    self.assertEqual(match['unloaded_messages'],3)
                    self.assertEqual(match['unknown_attachment_slots'],1)
                    for minimum,expected in [(0,1),(7,1),(8,0),(2**64-1,0)]:
                        q={**query,'mail_attachment':{'kind':'item','id':720,'minimum_quantity':minimum}}
                        self.assertEqual(len((await search(**q))['snapshots']),expected)
                    rows=(await search(character=value['character'],mail_attachment={'kind':'currency','id':720}))['snapshots']
                    self.assertEqual(rows[0]['mail_attachment_match']['quantity'],4)
                    for maximum in [0,1,2,2**64-1]:
                        filtered=(await search(mail_attachment={'kind':'currency','id':720,
                            'maximum_quantity_above_recorded_headroom':maximum}))['snapshots']
                        self.assertEqual([row['snapshot_id'] for row in filtered],[capped_key] if maximum >= 2 else [])
                        if filtered:
                            match=filtered[0]['mail_attachment_match']
                            self.assertEqual(match['hypothetical_cap_projection'],{
                                'proposed_quantity':4,'quantity_within_recorded_headroom':2,
                                'quantity_above_recorded_headroom':2})
                            self.assertFalse(match['mail_cap_application_verified'])
                            self.assertEqual(match['unloaded_messages'],3)
                    self.assertEqual((await search(character=value['character'],mail_attachment={
                        'kind':'item','id':999,'minimum_quantity':0}))['snapshots'],[])
                    self.assertEqual((await search(**query,latest_per_character=True))['snapshots'],[])
                    self.assertEqual(len((await search(**query,latest_per_character=True,
                        captured_through='2025-01-01T00:00:00Z'))['snapshots']),1)
                    rows=(await search(character=many['character'],mail_attachment={'kind':'item','id':720}))['snapshots']
                    self.assertEqual(rows[0]['snapshot_id'],many_key)
                    match=rows[0]['mail_attachment_match']
                    self.assertEqual((match['quantity'],match['matched_locations'],match['truncated']),(32,32,True))
                    self.assertEqual([(row['message_slot'],row['attachment_slot']) for row in match['locations']],
                        [(1,slot) for slot in range(1,17)]+[(2,slot) for slot in range(1,5)])
                    self.assertTrue(all('mail_attachment_match' not in row for row in (await search())['snapshots']))
                    for bad in [{'kind':'unknown','id':720},{'kind':'item','id':0},{'kind':'item','id':True},
                        {'kind':'item','id':720,'minimum_quantity':-1},{'kind':'item','id':720,'minimum_quantity':True},
                        {'kind':'item','id':720,'minimum_quantity':2**64},{'kind':'item','id':720,'sender':'x'},
                        {'kind':'item','id':720,'maximum_quantity_above_recorded_headroom':0},
                        {'kind':'currency','id':720,'maximum_quantity_above_recorded_headroom':-1},{}]:
                        response=await session.call_tool('list_snapshots',{'family':'forever','mail_attachment':bad})
                        self.assertTrue(response.isError)
                    response=await session.call_tool('list_snapshots',{'family':'retail','mail_attachment':{'kind':'item','id':720}})
                    self.assertTrue(response.isError)
                    response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'mail_attachments'})
                    self.assertFalse(response.isError)
                    self.assertEqual(json.loads(response.content[0].text)['data'],mail)
                    for source,count in [(key,4),(empty_key,0),(missing_key,None)]:
                        response=await session.call_tool('character_summary',{'snapshot_id':source})
                        self.assertFalse(response.isError)
                        summary=json.loads(response.content[0].text)['mail_attachment_counts']
                        self.assertEqual(summary['observed_slots'] if summary else None,count)
                        if summary:
                            self.assertEqual(summary['unloaded_messages'],
                                summary['total_messages']-summary['loaded_messages'])
                    response=await session.call_tool('find_snapshot_item',{'snapshot_id':key,'item_id':720})
                    self.assertFalse(response.isError)
                    self.assertEqual(json.loads(response.content[0].text)['matched_locations'],0)
                    for source in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':source,'section':'mail_attachments'})
                        self.assertTrue(response.isError)


if __name__=='__main__':
    unittest.main()
