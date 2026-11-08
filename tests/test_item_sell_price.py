"""Captured vendor arithmetic never credits a wallet or invents missing prices."""
from copy import deepcopy
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


class ItemSellPriceTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_partial_values_and_overflow(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        item={'item_id':720,'link':'item:720:0:0:0:0:0:0','sell_price_copper':25}
        value['sections']['equipment']={'16':deepcopy(item)}
        value['sections']['bags']={'0':{'1':{**item,'count':3},'2':{**item,'count':7,'sell_price_copper':0},
            '3':{'item_id':721,'count':2}}}
        value['sections']['character_bank']={'6':{'1':{**item,'count':4}}}
        value['sections']['account_bank']={};value['sections']['wallet']={'copper':40}
        absent=fixture();absent['client']=deepcopy(value['client'])
        huge=deepcopy(value);huge['sections']['bags']={'0':{'1':{**item,'count':2**32-1,'sell_price_copper':9_999_999_999_999}}}
        sum_overflow=deepcopy(value);sum_overflow['sections']['bags']={'0':{
            str(i):{**item,'count':1_000_000,'sell_price_copper':9_999_999_999_999} for i in [1,2]}}
        near_limit=deepcopy(value);near_limit['sections']['bags']={'0':{'1':{**item,'count':1_000_000,'sell_price_copper':9_999_999_999_999}}}
        unpriced=deepcopy(value);unpriced['sections']['bags']={'0':{'1':{'item_id':720,'count':3}}}
        malformed=[]
        for price in [None,True,-1,0.5,10_000_000_000_000,'PRIVATE',{},[]]:
            bad=deepcopy(value);bad['sections']['bags']['0']['1']['sell_price_copper']=price;malformed.append(bad)
        bad=deepcopy(value);bad['sections']['bags']['0']['1'].pop('link');malformed.append(bad)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));keys=[store.ingest(v) for v in [value,absent,huge,sum_overflow,near_limit,unpriced]]
            bad_keys=[]
            for bad in malformed:
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,**args):
                        response=await session.call_tool(tool,args);self.assertFalse(response.isError,response.content)
                        return json.loads(response.content[0].text)
                    for section in ['equipment','bags','character_bank','account_bank']:
                        self.assertEqual((await call('inspect_snapshot',snapshot_id=keys[0],section=section))['data'],value['sections'][section])
                    summary=await call('character_summary',snapshot_id=keys[0]);values=summary['captured_vendor_price_values']
                    self.assertEqual(values['equipment']['observed_total_copper'],25)
                    self.assertEqual(values['bags']['known_price_subtotal_copper'],75)
                    self.assertIsNone(values['bags']['observed_total_copper'])
                    self.assertEqual(values['bags']['observed_units'],12);self.assertEqual(values['bags']['priced_units'],10)
                    self.assertEqual(values['bags']['unpriced_units'],2);self.assertEqual(values['bags']['priced_locations'],2)
                    self.assertEqual(values['character_bank']['observed_total_copper'],100)
                    self.assertEqual(values['account_bank']['observed_total_copper'],0)
                    self.assertFalse(values['bags']['sale_eligibility_verified'])
                    self.assertEqual((await call('inspect_snapshot',snapshot_id=keys[0],section='wallet'))['data'],{'copper':40})
                    missing=(await call('character_summary',snapshot_id=keys[1]))['captured_vendor_price_values']
                    self.assertIsNone(missing['bags']);self.assertEqual(missing['equipment']['observed_total_copper'],0)
                    for key in keys[2:4]:
                        overflow=(await call('character_summary',snapshot_id=key))['captured_vendor_price_values']['bags']
                        self.assertTrue(overflow['arithmetic_overflow']);self.assertIsNone(overflow['known_price_subtotal_copper'])
                        self.assertIsNone(overflow['observed_total_copper'])
                    large=(await call('character_summary',snapshot_id=keys[4]))['captured_vendor_price_values']['bags']
                    self.assertFalse(large['arithmetic_overflow']);self.assertEqual(large['observed_total_copper'],9_999_999_999_999_000_000)
                    unpriced_value=(await call('character_summary',snapshot_id=keys[5]))['captured_vendor_price_values']['bags']
                    self.assertEqual(unpriced_value['priced_locations'],0);self.assertEqual(unpriced_value['unpriced_units'],3)
                    self.assertEqual(unpriced_value['known_price_subtotal_copper'],0);self.assertIsNone(unpriced_value['observed_total_copper'])
                    for bad in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'bags'})
                        self.assertTrue(response.isError);self.assertNotIn('PRIVATE',response.content[0].text)


if __name__=='__main__':unittest.main()
