"""Strict owner-local passive battlefield status slots, without queue actions."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from mcp import ClientSession,StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore,validate_snapshot
from test_snapshots import fixture


class BattlefieldQueueTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_status_only_schema_coverage_and_isolation(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        section={'coverage':'reported_battlefield_queue_slots','reported_slot_count':5,
            'entries':{str(i):{'reported_status':status} for i,status in enumerate(['none','queued','confirm','active','locked'],1)}}
        invalid=[None,{},[],{**section,'coverage':'all_queues'},{**section,'player':'PRIVATE'},
            {**section,'entries':[]},{**section,'entries':{}},{**section,'reported_slot_count':4}]
        for value in [None,True,False,-1,33,1.0,'5',[],{}]:invalid.append({**section,'reported_slot_count':value})
        for key in ['0','01','6','PRIVATE']:
            changed=deepcopy(section);changed['entries'][key]=changed['entries'].pop('1');invalid.append(changed)
        for row in [None,{},[],{'reported_status':None},{'reported_status':True},{'reported_status':1},
            {'reported_status':'PRIVATE'},{'reported_status':'Queued'},
            {'reported_status':'queued','name':'PRIVATE'},{'reported_status':'none','queue_id':1}]:
            changed=deepcopy(section);changed['entries']['1']=row;invalid.append(changed)
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as foreign:
            store=SnapshotStore(Path(folder));keys=[]
            zero={'coverage':'reported_battlefield_queue_slots','reported_slot_count':0,'entries':{}}
            maximum={'coverage':'reported_battlefield_queue_slots','reported_slot_count':32,
                'entries':{str(i):{'reported_status':'none'} for i in range(1,33)}}
            for day,current in enumerate([None,zero,section,maximum],1):
                saved=deepcopy(base);saved['captured_at']=f'2025-01-{day:02}T00:00:00Z'
                if current is not None:saved['sections']['battlefield_queue_slots']=current
                keys.append(store.ingest(saved))
            malformed=[]
            for current in invalid:
                saved=deepcopy(base);saved['sections']['battlefield_queue_slots']=current
                with self.assertRaises(ValueError):validate_snapshot(saved)
                malformed.append(saved)
            for family,interface in [('retail',120000),('classic',11500)]:
                saved=deepcopy(base);saved['client']={'family':family,'variant':'live','build':'1','interface':interface}
                saved['sections'].pop('selected_recipe',None);saved['sections']['battlefield_queue_slots']=section
                with self.assertRaises(ValueError):validate_snapshot(saved)
                malformed.append(saved)
            bad=[]
            for saved in malformed:
                raw=json.dumps(saved,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/(key+'.json')).write_bytes(raw);bad.append(key)
            private=deepcopy(base);private['character']['name']='PRIVATE_OTHER_OWNER'
            private['sections']['battlefield_queue_slots']=section
            private_key=SnapshotStore(Path(foreign)).ingest(private)
            original={p:p.read_bytes() for d in [folder,foreign] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args);self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(foreign,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    for key,expected in zip(keys,[None,zero,section,maximum]):
                        result=await call('inspect_snapshot',snapshot_id=key,section='battlefield_queue_slots')
                        self.assertEqual(result['available'],expected is not None);self.assertEqual(result['data'],expected)
                    listed=await call('list_snapshots',family='forever',section_capture={
                        'section':'battlefield_queue_slots','status':'observed'})
                    self.assertEqual({r['snapshot_id'] for r in listed['snapshots']},set(keys[1:]))
                    for key in bad+[private_key]:
                        await call('inspect_snapshot',True,snapshot_id=key,section='battlefield_queue_slots')
            for path,content in original.items():self.assertEqual(path.read_bytes(),content)


if __name__=='__main__':unittest.main()
