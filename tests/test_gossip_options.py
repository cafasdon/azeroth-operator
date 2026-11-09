"""Strict cross-reader visible dialogue observations and actual owner-local MCP."""
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


class GossipOptionTests(unittest.IsolatedAsyncioTestCase):
    async def test_reader_parity_actual_mcp_and_owner_scope(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        section={'coverage':'visible_gossip_options','entries':{
            '1':{'name':'Show your wares','option_id':42,'reported_status':0,'reported_flags':1,
                 'reported_order_index':-1,'spell_id':123},'2':{'name':'Tell me more'}}}
        value['sections']['gossip_options']=section
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryDirectory() as other:
            store=SnapshotStore(Path(folder));key=store.ingest(value)
            empty=deepcopy(value);empty['sections']['gossip_options']['entries']={};empty_key=store.ingest(empty)
            missing=deepcopy(value);del missing['sections']['gossip_options'];missing_key=store.ingest(missing)
            maximum=deepcopy(value);maximum['sections']['gossip_options']['entries']={str(i):{
                'name':'é'*256,'option_id':4294967295,'reported_status':4294967295,
                'reported_flags':0,'reported_order_index':-2147483648} for i in range(1,129)}
            max_key=store.ingest(maximum)
            foreign=deepcopy(value);foreign['character']['name']='PRIVATE_OTHER'
            foreign_key=SnapshotStore(Path(other)).ingest(foreign)
            invalid=[]
            def add(change):
                bad=deepcopy(value);change(bad)
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,ensure_ascii=False,sort_keys=True,allow_nan=False).encode()
                digest=hashlib.sha256(raw).hexdigest();(Path(folder)/(digest+'.json')).write_bytes(raw)
                invalid.append(digest)
            for row in [None,[],{}, {'name':''},{'name':'é'*257},{'name':True},
                        {'name':'A','option_id':0},{'name':'A','spell_id':4294967296},
                        {'name':'A','reported_status':True},{'name':'A','reported_flags':-1},
                        {'name':'A','reported_order_index':2147483648},
                        {'name':'A','reported_status':None},{'name':'A','reported_order_index':None},
                        {'name':'A','entered_text':'PRIVATE_CANARY'}]:
                add(lambda bad,row=row:bad['sections']['gossip_options']['entries'].update({'1':row}))
            add(lambda bad:bad['sections']['gossip_options'].update(coverage='PRIVATE'))
            add(lambda bad:bad['sections']['gossip_options'].update(entries={'2':{'name':'Sparse'}}))
            add(lambda bad:bad['sections']['gossip_options'].update(entries={str(i):{'name':'A'} for i in range(1,130)}))
            add(lambda bad:bad.update(client=fixture()['client']))
            before={p:p.read_bytes() for base in [folder,other] for p in Path(base).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (reader,writer):
                async with ClientSession(reader,writer) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args)
                        self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(other,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    observed=await call('inspect_snapshot',snapshot_id=key,section='gossip_options')
                    self.assertEqual(observed['data'],section);self.assertTrue(observed['available'])
                    summary=await call('character_summary',snapshot_id=key)
                    report=summary['gossip_option_observations']
                    self.assertEqual(report['observed_option_count'],2)
                    self.assertEqual(report['options_with_reported_id'],1)
                    for field in ['npc_identity_verified','selection_eligibility_verified','complete_dialogue_coverage_verified','game_execution']:
                        self.assertIs(report[field],False)
                    self.assertEqual((await call('inspect_snapshot',snapshot_id=empty_key,section='gossip_options'))['data']['entries'],{})
                    absent=await call('inspect_snapshot',snapshot_id=missing_key,section='gossip_options')
                    self.assertFalse(absent['available']);self.assertIsNone(absent['data'])
                    self.assertEqual(len((await call('inspect_snapshot',snapshot_id=max_key,section='gossip_options'))['data']['entries']),128)
                    for bad in [*invalid,foreign_key]:
                        await call('inspect_snapshot',True,snapshot_id=bad,section='gossip_options')
            self.assertEqual(before,{p:p.read_bytes() for p in before})


if __name__=='__main__':unittest.main()
