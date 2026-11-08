"""Find saved pet preparation observations without treating unknowns as zero."""
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


class ActivePetSearchTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_search_ranges_unknowns_and_capture_selection(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));ids={}
            observations={
                'Ready':{'present':True,'family_id':1,'level':60,'training_points':{'total':25,'used':10}},
                'Deficit':{'present':True,'family_id':2,'level':17,'training_points':{'total':10,'used':20}},
                'Zero':{'present':True,'family_id':1,'level':17,'training_points':{'total':0,'used':0}},
                'UnknownPoints':{'present':True,'family_id':1,'level':17},
                'UnknownMetadata':{'present':True},'Absent':{'present':False},'Missing':None,
                'PointsOnly':{'present':True,'training_points':{'total':1,'used':0}},
                'BoundLow':{'present':True,'family_id':2**32-1,'level':1,'training_points':{'total':-2**31,'used':2**31-1}},
                'BoundHigh':{'present':True,'family_id':2**32-1,'level':2**32-1,'training_points':{'total':2**31-1,'used':-2**31}},
            }
            for name,pet in observations.items():
                value=fixture();value['character']['name']=name
                value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
                if pet is not None:value['sections']['pet_care']=pet
                ids[name]=store.ingest(value)
            later=fixture();later['client']=value['client'];later['character']['name']='Ready'
            later['captured_at']='2025-01-02T00:00:00Z';store.ingest(later)
            bad=deepcopy(value);bad['sections']['pet_care']['training_points']['total']=True
            raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
            (Path(folder)/f'{hashlib.sha256(raw).hexdigest()}.json').write_bytes(raw)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def search(active_pet=None,**arguments):
                        args={'family':'forever',**arguments}
                        if active_pet is not None:args['active_pet']=active_pet
                        response=await session.call_tool('list_snapshots',args)
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)
                    for query,names in [
                        ({'present':False},['Absent']),
                        ({'family_id':1},['Ready','Zero','UnknownPoints']),
                        ({'minimum_level':17,'maximum_level':17},['Deficit','Zero','UnknownPoints']),
                        ({'minimum_training_points_balance':1},['Ready','PointsOnly','BoundHigh']),
                        ({'maximum_training_points_balance':-1},['Deficit','BoundLow']),
                        ({'family_id':1,'maximum_training_points_balance':0},['Zero']),
                        ({'minimum_training_points_balance':2**32-1},['BoundHigh']),
                        ({'maximum_training_points_balance':-(2**32-1)},['BoundLow']),
                        ({'family_id':1,'minimum_level':61},[]),
                    ]:
                        rows=(await search(query))['snapshots']
                        self.assertEqual({row['snapshot_id'] for row in rows},{ids[name] for name in names},query)
                    rows=(await search({'present':True}))['snapshots']
                    self.assertEqual(len(rows),8)
                    unknown=next(row for row in rows if row['snapshot_id']==ids['UnknownMetadata'])
                    self.assertEqual(unknown['active_pet_match'],{'present':True,'family_id':None,'level':None,'training_points_balance':None})
                    all_rows=(await search())['snapshots']
                    self.assertTrue(all('active_pet_match' not in row for row in all_rows))
                    full=await search({'family_id':1})
                    page=await search({'family_id':1},offset=1,limit=1)
                    self.assertEqual(page['snapshots'],full['snapshots'][1:2]);self.assertTrue(page['has_more'])
                    self.assertEqual(full['invalid_count'],1)
                    args={'character':{'name':'Ready','realm':'Test'},'latest_per_character':True}
                    self.assertEqual((await search({'present':True},**args))['snapshots'],[])
                    rows=(await search({'present':True},**args,captured_through='2025-01-01T00:00:00Z'))['snapshots']
                    self.assertEqual([row['snapshot_id'] for row in rows],[ids['Ready']])
                    for query in [{},{'present':None},{'present':False,'family_id':1},{'family_id':0},
                        {'family_id':True},{'family_id':2**32},{'minimum_level':0},{'minimum_level':True},
                        {'minimum_level':18,'maximum_level':17},{'minimum_training_points_balance':True},
                        {'minimum_training_points_balance':2**32},{'maximum_training_points_balance':-2**32},
                        {'minimum_training_points_balance':1,'maximum_training_points_balance':0},
                        {'pet_guid':'PRIVATE'}]:
                        response=await session.call_tool('list_snapshots',{'family':'forever','active_pet':query})
                        self.assertTrue(response.isError,query)
                    response=await session.call_tool('list_snapshots',{'family':'retail','active_pet':{'present':True}})
                    self.assertTrue(response.isError)


if __name__=='__main__':unittest.main()
