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
                     '2':{'kind':'currency','currency_id':3,'count':4},'16':{'kind':'unknown'}}}}
            value['sections']['mail_attachments']=mail
            key=store.ingest(value)
            empty=copy.deepcopy(value)
            empty['sections']['mail_attachments'].update(loaded_messages=0,total_messages=0,messages={})
            empty_key=store.ingest(empty)
            missing=copy.deepcopy(value)
            missing['sections'].pop('mail_attachments')
            missing_key=store.ingest(missing)
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
                    response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'mail_attachments'})
                    self.assertFalse(response.isError)
                    self.assertEqual(json.loads(response.content[0].text)['data'],mail)
                    for source,count in [(key,3),(empty_key,0),(missing_key,None)]:
                        response=await session.call_tool('character_summary',{'snapshot_id':source})
                        self.assertFalse(response.isError)
                        summary=json.loads(response.content[0].text)['mail_attachment_counts']
                        self.assertEqual(summary['observed_slots'] if summary else None,count)
                    response=await session.call_tool('find_snapshot_item',{'snapshot_id':key,'item_id':720})
                    self.assertFalse(response.isError)
                    self.assertEqual(json.loads(response.content[0].text)['matched_locations'],0)
                    for source in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':source,'section':'mail_attachments'})
                        self.assertTrue(response.isError)


if __name__=='__main__':
    unittest.main()
