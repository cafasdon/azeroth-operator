"""Strict owner-local raw life-state flags, not inferred alive or resurrection rights."""
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


class PlayerLifeStateTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_raw_flags_schema_coverage_and_owner_isolation(self):
        root=Path(__file__).resolve().parents[1]
        binary=root/'target/debug'/('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')
        base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        fields=['is_dead','is_ghost','is_dead_or_ghost','is_feigning_death']
        section={'coverage':'reported_player_life_flags',**dict.fromkeys(fields,False)}
        invalid=[None,{},[],{**section,'coverage':'alive'},{**section,'name':'PRIVATE'}]
        for field in fields:
            changed=deepcopy(section);del changed[field];invalid.append(changed)
            for value in [None,0,1,-1,1.0,'PRIVATE',[],{}]:invalid.append({**section,field:value})
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryDirectory() as other:
            store=SnapshotStore(Path(folder));valid=[]
            absent=deepcopy(base);absent_key=store.ingest(absent)
            for mask in range(16):
                current={'coverage':'reported_player_life_flags',**{field:bool(mask&(1<<i)) for i,field in enumerate(fields)}}
                saved=deepcopy(base);saved['sections']['player_life_state']=current
                saved['captured_at']=f'2025-01-{mask+2:02}T00:00:00Z';valid.append((store.ingest(saved),current))
            malformed=[]
            for current in invalid:
                saved=deepcopy(base);saved['sections']['player_life_state']=current
                with self.assertRaises(ValueError):validate_snapshot(saved)
                malformed.append(saved)
            for family,interface in [('retail',120000),('classic',11500)]:
                saved=deepcopy(base);saved['client']={'family':family,'variant':'live','build':'1','interface':interface}
                saved['sections'].pop('selected_recipe',None);saved['sections']['player_life_state']=section
                with self.assertRaises(ValueError):validate_snapshot(saved)
                malformed.append(saved)
            bad=[]
            for saved in malformed:
                raw=json.dumps(saved,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/(key+'.json')).write_bytes(raw);bad.append(key)
            private=deepcopy(base);private['character']['name']='PRIVATE_OTHER_OWNER';private['sections']['player_life_state']=section
            private_key=SnapshotStore(Path(other)).ingest(private)
            original={p:p.read_bytes() for d in [folder,other] for p in Path(d).glob('*.json')}
            async with stdio_client(StdioServerParameters(command=str(binary),env={**os.environ,'AZEROTH_DATA_DIR':folder})) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(tool,error=False,**args):
                        reply=await session.call_tool(tool,args);self.assertEqual(bool(reply.isError),error,reply)
                        self.assertNotIn('PRIVATE',str(reply));self.assertNotIn(other,str(reply))
                        return None if error else json.loads(reply.content[0].text)
                    missing=await call('inspect_snapshot',snapshot_id=absent_key,section='player_life_state')
                    self.assertFalse(missing['available']);self.assertIsNone(missing['data'])
                    for key,current in valid:
                        result=await call('inspect_snapshot',snapshot_id=key,section='player_life_state')
                        self.assertTrue(result['available']);self.assertEqual(result['data'],current)
                    listed=await call('list_snapshots',family='forever',section_capture={
                        'section':'player_life_state','status':'observed'})
                    self.assertEqual({row['snapshot_id'] for row in listed['snapshots']},{key for key,_ in valid})
                    for key in bad+[private_key]:await call('inspect_snapshot',True,snapshot_id=key,section='player_life_state')
            for path,content in original.items():self.assertEqual(path.read_bytes(),content)


if __name__=='__main__':unittest.main()
