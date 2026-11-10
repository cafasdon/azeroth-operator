"""Active-pet family/level observations stay optional and owner-scoped."""
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


class ActivePetDetailsTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_preserves_details_unknowns_and_validation(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));cases=[]
            for pet in [{'present':True,'family_id':1,'level':17,'food_types':{'1':'Meat','2':'Poisson'},
                         'happiness':0,'loyalty':'Très fidèle'},
                        {'present':True}, {'present':True,'food_types':{}}, {'present':False},None]:
                value=fixture()
                value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
                value['sections']['equipment']={'16':{'item_id':123}}
                if pet is not None:value['sections']['pet_care']=pet
                cases.append((store.ingest(value),value))
            bad_keys=[]
            for field in ['family_id','level']:
                for invalid in [None,0,-1,True,1.5,'1',2**32]:
                    bad=deepcopy(cases[0][1]);bad['sections']['pet_care'][field]=invalid
                    with self.assertRaises(ValueError):validate_snapshot(bad)
                    raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
                    key=hashlib.sha256(raw).hexdigest();(Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
                bad=deepcopy(cases[0][1]);bad['sections']['pet_care']={'present':False,field:1}
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(raw).hexdigest();(Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            later=deepcopy(cases[0][1]);later['sections']['pet_care'].update(
                family_id=2,level=18,happiness=2,loyalty=' Loyal ')
            later_key=store.ingest(later)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for key,value in cases:
                        response=await session.call_tool('character_summary',{'snapshot_id':key})
                        self.assertFalse(response.isError)
                        pet=value['sections'].get('pet_care')
                        expected=None if pet is None else {field:pet.get(field) for field in ['present','family_id','level']}
                        self.assertEqual(json.loads(response.content[0].text)['active_pet_observation'],expected)
                        response=await session.call_tool('review_equipment_selection',{'snapshot_id':key,
                            'selections':[{'target_slot':16,'source':{'kind':'equipment','slot':16},'expected_item_id':123}]})
                        self.assertFalse(response.isError)
                        self.assertEqual(json.loads(response.content[0].text)['captured_active_pet'],expected)
                        if pet is not None:
                            response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'pet_care'})
                            self.assertFalse(response.isError)
                            self.assertEqual(json.loads(response.content[0].text)['data'],pet)
                    response=await session.call_tool('compare_snapshots',{'before_id':cases[0][0],'after_id':later_key})
                    self.assertFalse(response.isError)
                    delta=json.loads(response.content[0].text)['pet_care_change']
                    self.assertEqual(delta['before']['family_id'],1)
                    self.assertEqual(delta['after']['family_id'],2)
                    self.assertEqual(delta['before']['level'],17)
                    self.assertEqual(delta['after']['level'],18)
                    self.assertNotIn('level_delta',delta)
                    self.assertEqual(delta['food_type_changes'],
                                     {'observed_only_before':[], 'observed_only_after':[]})
                    for filters, expected in (
                        ({'food_type_name':'  mEAT ', 'family_id':1}, [cases[0][0]]),
                        ({'food_type_name':'poisson', 'minimum_level':18}, [later_key]),
                        ({'food_type_name':'Me', 'present':True}, []),
                        ({'food_type_name':'Meat', 'family_id':3}, []),
                    ):
                        reply=await session.call_tool('list_snapshots',{'family':'forever','active_pet':filters})
                        self.assertFalse(reply.isError)
                        rows=json.loads(reply.content[0].text)['snapshots']
                        self.assertEqual([row['snapshot_id'] for row in rows],expected)
                        if rows:
                            self.assertEqual(rows[0]['active_pet_match']['food_type_name'],
                                             filters['food_type_name'].strip().capitalize())
                    for filters in ({'food_type_name':' '}, {'present':False,'food_type_name':'Meat'}):
                        self.assertTrue((await session.call_tool('list_snapshots',{
                            'family':'forever','active_pet':filters})).isError)
                    for filters, expected in (
                        ({'happiness':0, 'loyalty':' TRÈS FIDÈLE '}, [cases[0][0]]),
                        ({'happiness':2, 'food_type_name':'Meat', 'loyalty':'loyal'}, [later_key]),
                        ({'happiness':0, 'family_id':2}, []),
                        ({'loyalty':'fidèle'}, []),
                    ):
                        reply=await session.call_tool('list_snapshots',{'family':'forever','active_pet':filters})
                        self.assertFalse(reply.isError)
                        rows=json.loads(reply.content[0].text)['snapshots']
                        self.assertEqual([row['snapshot_id'] for row in rows],expected)
                        if rows:
                            self.assertEqual(rows[0]['active_pet_match']['happiness'],filters['happiness'])
                            if 'loyalty' in filters:
                                self.assertEqual(rows[0]['active_pet_match']['loyalty'],
                                                 'Très fidèle' if expected == [cases[0][0]] else ' Loyal ')
                    for filters in ({'loyalty':' '}, {'happiness':256}, {'present':False,'happiness':0}):
                        self.assertTrue((await session.call_tool('list_snapshots',{
                            'family':'forever','active_pet':filters})).isError)
                    for key in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'pet_care'})
                        self.assertTrue(response.isError)
                    changed=deepcopy(later)
                    changed['sections']['pet_care']['food_types']={'1':'Poisson','2':'Fruit','3':'Fruit'}
                    changed_key=store.ingest(changed)
                    for after, expected in (
                        (changed_key, {'observed_only_before':['Meat'], 'observed_only_after':['Fruit']}),
                        (cases[1][0], None),
                        (cases[2][0], {'observed_only_before':['Meat','Poisson'], 'observed_only_after':[]}),
                    ):
                        reply=await session.call_tool('compare_snapshots',{'before_id':cases[0][0],'after_id':after})
                        self.assertFalse(reply.isError)
                        self.assertEqual(json.loads(reply.content[0].text)['pet_care_change']['food_type_changes'],
                                         expected)


if __name__=='__main__':unittest.main()
