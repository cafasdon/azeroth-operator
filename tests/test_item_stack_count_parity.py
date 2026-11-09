"""Python ingestion agrees with the real Rust reader on item stack counts."""
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


class ItemStackCountParityTests(unittest.IsolatedAsyncioTestCase):
    def test_missing_count_rejected_before_ingestion_writes(self):
        value=fixture(); value['sections']['bags']={'0':{'1':{'item_id':100}}}
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            with self.assertRaises(ValueError): store.ingest(value)
            self.assertEqual(list(Path(folder).iterdir()),[])

    async def test_actual_mcp_count_contract_by_storage_and_family(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        missing=object()
        cases=[missing,None,False,True,0,-1,1,4294967295,4294967296,1.0,.5,'1',[],{},'PRIVATE']
        scopes=[(family,storage) for family in ['retail','classic','forever'] for storage in ['equipment','bags']]
        scopes += [('forever','character_bank'),('forever','account_bank')]
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); raw=base/'rust'; raw.mkdir(); store=SnapshotStore(base/'python')
            env={**os.environ,'AZEROTH_DATA_DIR':str(raw)}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for family,storage in scopes:
                        for count in cases:
                            value=fixture(); value['client']['family']=family
                            item={'item_id':100}
                            if count is not missing: item['count']=deepcopy(count)
                            if storage=='equipment': value['sections']['equipment']={'11':item}
                            else:
                                container={'bags':'0','character_bank':'6','account_bank':'7'}[storage]
                                value['sections'][storage]={container:{'1':item}}
                            expected=(count is missing or count is None) if storage=='equipment' else (
                                type(count) is int and 1<=count<=4294967295)
                            with self.subTest(family=family,storage=storage,count=count):
                                prior={path.name:path.read_bytes() for path in store.root.glob('*.json')}
                                if expected: store.ingest(value)
                                else:
                                    with self.assertRaises(ValueError): store.ingest(value)
                                    self.assertEqual({path.name:path.read_bytes() for path in store.root.glob('*.json')},prior)
                                data=json.dumps(value,sort_keys=True,separators=(',',':')).encode()
                                key=hashlib.sha256(data).hexdigest(); (raw/f'{key}.json').write_bytes(data)
                                reply=await session.call_tool('inspect_snapshot',{'snapshot_id':key})
                                self.assertEqual(not bool(reply.isError),expected,reply)
                                if not expected: self.assertNotIn('PRIVATE',str(reply))


if __name__=='__main__':
    unittest.main()
