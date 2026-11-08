"""Filtered local profession observations through the owner-scoped MCP reader."""
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


class ProfessionRecipeViewTests(unittest.IsolatedAsyncioTestCase):
    async def test_readback_unknowns_and_schema_parity(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            value=fixture()
            value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            view={'coverage':'filtered_local_recipe_list','profession_id':164,
                  'entries':{'7':{'name':'Recipe','learned':True},'8':{'learned':False},'9':{}}}
            value['sections']['profession_recipe_view']=view
            key=store.ingest(value)
            empty=copy.deepcopy(value)
            empty['sections']['profession_recipe_view']['entries']={}
            empty_key=store.ingest(empty)
            missing=copy.deepcopy(value)
            missing['sections'].pop('profession_recipe_view')
            missing_key=store.ingest(missing)
            bad_keys=[]
            for change in [
                lambda x:x['client'].update(family='retail'),
                lambda x:x['sections']['profession_recipe_view'].update(coverage='all_known_recipes'),
                lambda x:x['sections']['profession_recipe_view'].update(profession_id=True),
                lambda x:x['sections']['profession_recipe_view']['entries']['7'].update(crafter='private'),
                lambda x:x['sections']['profession_recipe_view']['entries']['7'].update(learned=1),
                lambda x:x['sections']['profession_recipe_view']['entries']['7'].update(name='é'*129),
                lambda x:x['sections']['profession_recipe_view']['entries'].update({'07':{}}),
            ]:
                bad=copy.deepcopy(value)
                change(bad)
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
                bad_key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/f'{bad_key}.json').write_bytes(raw)
                bad_keys.append(bad_key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'profession_recipe_view'})
                    self.assertFalse(response.isError)
                    self.assertEqual(json.loads(response.content[0].text)['data'],view)
                    for source,expected in [(key,{'observed':3,'learned':1,'unlearned':1,'learning_unknown':1}),
                        (empty_key,{'observed':0,'learned':0,'unlearned':0,'learning_unknown':0}),(missing_key,None)]:
                        response=await session.call_tool('character_summary',{'snapshot_id':source})
                        self.assertFalse(response.isError)
                        self.assertEqual(json.loads(response.content[0].text)['profession_recipe_view_counts'],expected)
                    for source in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':source,'section':'profession_recipe_view'})
                        self.assertTrue(response.isError)


if __name__=='__main__':
    unittest.main()
