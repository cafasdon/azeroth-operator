"""Base inventory structure admission matches the unchanged Rust reader."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


def sample(storage='bags'):
    value=fixture(); value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
    item={'item_id':100}
    if storage=='equipment': value['sections']['equipment']={'11':item}
    else:
        item['count']=1
        value['sections'][storage]={{'bags':'0','character_bank':'6','account_bank':'7'}[storage]:{'1':item}}
    return value


class InventoryItemShapeParityTests(unittest.IsolatedAsyncioTestCase):
    def test_unsupported_item_field_rejected_before_writes(self):
        value=sample(); value['sections']['bags']['0']['1']['unsupported_private_field']='PRIVATE'
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            with self.assertRaises(ValueError): store.ingest(value)
            self.assertEqual(list(Path(folder).iterdir()),[])
            data=json.dumps(value,sort_keys=True,separators=(',',':')).encode()
            key=hashlib.sha256(data).hexdigest(); path=Path(folder)/f'{key}.json'; path.write_bytes(data)
            with self.assertRaises(ValueError): store.read(key)
            self.assertEqual(path.read_bytes(),data)

    async def test_actual_mcp_structure_identity_and_bank_contract(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        cases=[]
        for storage in ['equipment','bags','character_bank','account_bank']:
            base=sample(storage)
            for invalid in [None,False,1,'PRIVATE',[],[{}]]:
                value=deepcopy(base); value['sections'][storage]=invalid; cases.append((value,False))
            group=base['sections'][storage] if storage=='equipment' else next(iter(base['sections'][storage].values()))
            native_slot='11' if storage=='equipment' else '1'
            for slot in ['0','01','+1',' 1','1.0','١',str(20 if storage=='equipment' else 1025)]:
                value=deepcopy(base)
                target=value['sections'][storage] if storage=='equipment' else next(iter(value['sections'][storage].values()))
                target[slot]=target.pop(native_slot); cases.append((value,False))
            for invalid in [None,False,1,'PRIVATE',[],{}]:
                value=deepcopy(base)
                target=value['sections'][storage] if storage=='equipment' else next(iter(value['sections'][storage].values()))
                target[native_slot]=invalid; cases.append((value,False))
            for field,values in [('item_id',[None,False,0,-1,1.0,4294967296,'100']),
                    ('link',[False,1,[],{},'x'*4097,'é'*2049]),('guid',['',False,1,[],{},'x'*129,'😀'*33,'x\x00','x\x85']),
                    ('required_level',[True,-1,1.0,4294967296]),('PRIVATE',['PRIVATE'])]:
                for invalid in values:
                    value=deepcopy(base)
                    target=value['sections'][storage] if storage=='equipment' else next(iter(value['sections'][storage].values()))
                    target[native_slot][field]=invalid; cases.append((value,False))
            for fields in [{'link':None,'guid':None,'required_level':None},
                    {'link':'','guid':'x\u200d'}, {'link':'x'*4096,'guid':'😀'*32},
                    {'item_id':4294967295,'link':'é'*2048},
                    {'link':'item:100:0','required_level':4294967295}]:
                value=deepcopy(base)
                target=value['sections'][storage] if storage=='equipment' else next(iter(value['sections'][storage].values()))
                target[native_slot].update(fields); cases.append((value,True))
            if storage!='equipment':
                container=next(iter(base['sections'][storage]))
                for bad in ['-1','00','+1','65','١']:
                    value=deepcopy(base); value['sections'][storage][bad]=value['sections'][storage].pop(container); cases.append((value,False))
                for invalid in [None,False,1,'PRIVATE',[]]:
                    value=deepcopy(base); value['sections'][storage][container]=invalid; cases.append((value,False))
        for family in ['retail','classic']:
            for storage in ['character_bank','account_bank']:
                value=sample(storage); value['client']['family']=family; cases.append((value,False))
        for first,second in [('bags','character_bank'),('bags','account_bank'),('character_bank','account_bank')]:
            value=sample(first); container=next(iter(value['sections'][first]))
            value['sections'][second]={container:{'1':{'item_id':100,'count':1}}}; cases.append((value,False))
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); raw=base/'rust'; raw.mkdir(); store=SnapshotStore(base/'python')
            env={**os.environ,'AZEROTH_DATA_DIR':str(raw)}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for index,(value,expected) in enumerate(cases):
                        with self.subTest(index=index):
                            before={path.name:path.read_bytes() for path in store.root.glob('*.json')}
                            if expected: store.ingest(value)
                            else:
                                with self.assertRaises(ValueError) as stopped: store.ingest(value)
                                self.assertNotIn('PRIVATE',str(stopped.exception))
                                self.assertEqual({path.name:path.read_bytes() for path in store.root.glob('*.json')},before)
                            data=json.dumps(value,sort_keys=True,separators=(',',':')).encode()
                            key=hashlib.sha256(data).hexdigest(); (raw/f'{key}.json').write_bytes(data)
                            reply=await session.call_tool('inspect_snapshot',{'snapshot_id':key})
                            self.assertEqual(not bool(reply.isError),expected,reply)
                            if not expected: self.assertNotIn('PRIVATE',str(reply))
            print(f'inventory_item_shape_parity_cases={len(cases)}')


if __name__=='__main__':
    unittest.main()
