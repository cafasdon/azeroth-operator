"""Numeric inbox quotes remain outside wallet/inventory through the real MCP."""
from copy import deepcopy
from itertools import combinations
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_mail_funds_validation import fixture
from test_snapshots import fixture as base_fixture

BOUNDARY=['mailbox_funds_spendable','inventory_credit','current_availability_verified',
    'mail_identity_continuity_verified','game_execution','live']


class MailFundsMcpTests(unittest.IsolatedAsyncioTestCase):
    async def test_inspection_summary_and_selected_cod_budget(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder));value=fixture();key=store.ingest(value)
            wallets={5:key}
            for amount in [0,100,None]:
                sample=deepcopy(value)
                if amount is None:del sample['sections']['wallet']
                else:sample['sections']['wallet']['copper']=amount
                wallets[amount]=store.ingest(sample)
            absent=deepcopy(value);del absent['sections']['mail_funds'];absent_key=store.ingest(absent)
            empty=deepcopy(value);empty['sections']['mail_funds'].update(loaded_messages=0,total_messages=0,messages={})
            empty_key=store.ingest(empty)
            complete=deepcopy(value);complete['sections']['mail_funds']['total_messages']=3
            complete['sections']['mail_funds']['messages']['3']={'status':'reported','money_copper':0,'cod_copper':0}
            complete_key=store.ingest(complete)
            retail_key=store.ingest(base_fixture())
            other=deepcopy(value);other['character']['name']='PRIVATE_OWNER'
            other_key=SnapshotStore(Path(foreign)).ingest(other)
            original={p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,
                    'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def tool(name,error=False,**args):
                        reply=await session.call_tool(name,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    async def call(snapshot=key,page=None,error=False,section='mail_funds',**extra):
                        args={'snapshot_id':snapshot,'section':section,**extra}
                        if page is not None:args['mail_funds_review']=page
                        return await tool('inspect_snapshot',error=error,**args)
                    raw=await call();self.assertEqual(raw['data'],value['sections']['mail_funds'])
                    summary=(await tool('character_summary',snapshot_id=key))['mail_funds_observation']
                    self.assertEqual(summary['known_money_copper'],100)
                    self.assertEqual(summary['known_cod_quote_copper'],20)
                    self.assertEqual(summary['unavailable_message_count'],1)
                    self.assertEqual(summary['unloaded_messages'],2)
                    self.assertIsNone(summary['total_money_copper']);self.assertIsNone(summary['total_cod_quote_copper'])
                    for flag in BOUNDARY:self.assertFalse(summary[flag])
                    full=(await tool('character_summary',snapshot_id=complete_key))['mail_funds_observation']
                    self.assertEqual(full['total_money_copper'],100);self.assertEqual(full['total_cod_quote_copper'],20)
                    self.assertIsNone((await tool('character_summary',snapshot_id=absent_key))['mail_funds_observation'])
                    # Independent subset/reserve oracle: inbox money never funds a COD quote.
                    selections=[list(c) for length in range(1,4) for c in combinations([1,2,3],length)]+[[4],[2,4]]
                    for balance,snapshot in wallets.items():
                        for slots in selections:
                            for reserve in [0,5,6,20]:
                                result=(await call(snapshot=snapshot,page={'message_slots':slots,'reserve_copper':reserve}))['data']
                                known_cod=20 if 2 in slots else 0
                                unknown=sum(slot in (3,4) for slot in slots)
                                spendable=None if balance is None else max(balance-reserve,0)
                                fits=(None if balance is None else False if reserve>balance or known_cod>spendable
                                    else True if unknown==0 else None)
                                self.assertEqual(result['known_cod_quote_copper'],known_cod)
                                self.assertEqual(result['unknown_message_count'],unknown)
                                self.assertEqual(result['spendable_after_reserve_copper'],spendable)
                                self.assertIs(result['known_cod_fits_after_reserve'],fits)
                                self.assertEqual(result['total_selected_cod_quote_copper'],None if unknown else known_cod)
                                for flag in BOUNDARY+['payment_eligibility_verified']:self.assertFalse(result[flag])
                    budget=(await call(page={}))['data']
                    self.assertEqual(budget['wallet_copper'],5);self.assertEqual(budget['known_money_copper'],100)
                    self.assertFalse(budget['known_cod_fits_after_reserve'])
                    self.assertEqual(budget['rows'][1]['cod_shortfall_after_reserve'],15)
                    self.assertIsNone(budget['rows'][2]['money_copper'])
                    for offset in [0,1,2,3,4294967295]:
                        page=(await call(page={'limit':1,'offset':offset}))['data']
                        self.assertEqual(page['rows'],budget['rows'][offset:offset+1])
                        self.assertEqual(page['selected_message_count'],3)
                        self.assertEqual(page['known_cod_quote_copper'],20)
                    outside=(await call(page={'message_slots':[4]}))['data']
                    self.assertEqual(outside['rows'][0]['status'],'not_captured')
                    self.assertIsNone(outside['known_cod_fits_after_reserve'])
                    missing=(await call(snapshot=absent_key,page={}))['data']
                    self.assertEqual(missing['status'],'funds_not_recorded');self.assertIsNone(missing['rows'])
                    zero=(await call(snapshot=empty_key,page={}))['data']
                    self.assertEqual(zero['rows'],[]);self.assertEqual(zero['known_money_copper'],0)
                    found=await tool('list_snapshots',family='forever',required_sections=['mail_funds'])
                    self.assertTrue(all(row['snapshot_id']!=absent_key for row in found['snapshots']))
                    for bad in [{'limit':0},{'limit':51},{'offset':-1},{'offset':True},
                            {'message_slots':[]},{'message_slots':[1,1]},{'message_slots':[0]},
                            {'message_slots':[101]},{'message_slots':[True]},{'reserve_copper':True},
                            {'reserve_copper':10000000000000},{'extra':'PRIVATE'}]:
                        await call(page=bad,error=True)
                    await call(page={},section='mail_attachments',error=True)
                    await call(page={},coverage_sections=['mail_funds'],error=True)
                    await call(snapshot=retail_key,page={},error=True)
                    await call(snapshot=other_key,error=True)
            self.assertEqual(original,{p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')})
