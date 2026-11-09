"""Importer unavailable metadata must obey the runtime's section/reason contract."""
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


class SnapshotUnavailableTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_unavailable_metadata_parity(self):
        root = Path(__file__).resolve().parents[1]
        binary = (root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base = fixture()
        valid = [{},{'bags':'api_unavailable_or_failed'},{'bags':'é'*128},
            {'bags':'\x1c'},{'bags':'\u2003reason\u2003'}]
        invalid = [None,[],False,'text',{'equipment':'api_unavailable_or_failed'},
            {'not_a_section':'reason'},{'bags':None},{'bags':1},{'bags':False},{'bags':{}},
            {'bags':''},{'bags':' \t\r\n'},{'bags':'\u2003\u00a0'},
            {'bags':'é'*129},{'bags':'x'*257}]
        # The reported concrete conflict, with an otherwise valid new section.
        conflict = deepcopy(base)
        conflict['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        conflict['sections']['loot_window'] = {'coverage':'visible_loot_window_slots','slot_count':0,'entries':{}}
        conflict['unavailable'] = {'loot_window':'api_unavailable_or_failed'}
        with self.assertRaises(ValueError): validate_snapshot(conflict)
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder)); good = []; bad = []
            for metadata in valid:
                sample = deepcopy(base); sample['unavailable'] = metadata
                good.append((store.ingest(sample),metadata))
            for metadata in invalid:
                sample = deepcopy(base); sample['unavailable'] = metadata
                with self.assertRaises(ValueError): validate_snapshot(sample)
                raw = json.dumps(sample,sort_keys=True,separators=(',',':')).encode()
                key = hashlib.sha256(raw).hexdigest(); (Path(folder)/f'{key}.json').write_bytes(raw); bad.append(key)
            raw = json.dumps(conflict,sort_keys=True,separators=(',',':')).encode()
            key = hashlib.sha256(raw).hexdigest(); (Path(folder)/f'{key}.json').write_bytes(raw); bad.append(key)
            before = {p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env = {**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for key,metadata in good:
                        result = await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'bags'})
                        self.assertFalse(result.isError,result)
                        self.assertEqual(json.loads(result.content[0].text)['unavailable_reason'],metadata.get('bags'))
                    for key in bad:
                        result = await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'bags'})
                        self.assertTrue(result.isError,result)
                        self.assertNotIn('é',str(result))
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
