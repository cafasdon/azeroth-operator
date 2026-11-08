"""Captured native level requirements block only known shortfalls, not unknowns."""
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore, validate_snapshot
from test_snapshots import fixture
from tools.rank_forever_slot import collect


class NativeLevelTests(unittest.IsolatedAsyncioTestCase):
    async def test_native_limit_reaches_reviews_and_ranker(self):
        root=Path(__file__).resolve().parents[1]
        catalogue=Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE',root/'local-data/forever/catalogue.json'))
        if not catalogue.is_file():self.skipTest('Requires pinned local catalogue; no automatic download')
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            value=fixture()
            value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['stats']={'level':20,'class':'HUNTER','race':'Human','client_version':'1.60.1'}
            value['sections']['bags']={'0':{'1':{'item_id':4549,'count':1,
                'link':'item:4549:0:0:0:0:0:0','required_level':30}}}
            cases=[]
            for level in [30,20,0,None]:
                sample=copy.deepcopy(value)
                sample['sections']['bags']['0']['1']['required_level']=level
                cases.append((store.ingest(sample),level))
            for level in [True,-1,0.5,2**32]:
                bad=copy.deepcopy(value);bad['sections']['bags']['0']['1']['required_level']=level
                with self.assertRaises(ValueError):validate_snapshot(bad)
            selection={'target_slot':11,'source':{'kind':'bag','bag':0,'slot':1},'expected_item_id':4549}
            env={**os.environ,'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for key,level in cases:
                        response=await session.call_tool('review_equipment_selection',{'snapshot_id':key,'selections':[selection]})
                        self.assertFalse(response.isError)
                        checks=json.loads(response.content[0].text)['checks']
                        if level==30:
                            self.assertIsNone(checks['simulator_equipment_spec'])
                            self.assertEqual(checks['native_required_level_shortfalls'][0]['levels_remaining'],10)
                        else:
                            self.assertIsNotNone(checks['simulator_equipment_spec'])
                            self.assertEqual(checks['native_required_level_shortfalls'],[])
                    response=await session.call_tool('review_forever_owned_gear',{'snapshot_id':cases[0][0],'simulator_slot':'finger1'})
                    row=json.loads(response.content[0].text)['items'][0]
                    self.assertEqual(row['native_required_level_shortfall']['required_level'],30)
            with tempfile.TemporaryFile(mode='w+') as errlog,patch.dict(os.environ,env):
                drafts,skipped,*_=await collect(binary,cases[0][0],['finger1'],errlog)
                self.assertEqual(len(drafts),1)
                self.assertEqual(skipped[0]['reason'],'native_required_level_shortfall')


if __name__=='__main__':unittest.main()
