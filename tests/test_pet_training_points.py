"""Saved pet training counters preserve deficits without inferring pet continuity."""
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


class PetTrainingPointTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_signed_balances_and_strict_nested_contract(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            base=fixture();base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            base['sections']['pet_care']={'present':True,'family_id':1,'level':17}
            cases=[]
            for total,used in [(25,10),(0,0),(10,20),(-10,20),(2**31-1,-2**31),(-2**31,2**31-1)]:
                value=deepcopy(base);value['sections']['pet_care']['training_points']={'total':total,'used':used}
                cases.append((store.ingest(value),value,total-used))
            unknown=store.ingest(base)
            malformed=[None,{}, {'total':1}, {'used':1}, {'total':1,'used':0,'pet_guid':'PRIVATE'}]
            for field in ['total','used']:
                for invalid in [True,None,1.5,'1',2**31,-2**31-1]:
                    bad={'total':1,'used':0};bad[field]=invalid;malformed.append(bad)
            bad_keys=[]
            for points in malformed:
                bad=deepcopy(base);bad['sections']['pet_care']['training_points']=points
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(raw).hexdigest();(Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            bad=deepcopy(base);bad['sections']['pet_care']={'present':False,'training_points':{'total':0,'used':0}}
            with self.assertRaises(ValueError):validate_snapshot(bad)
            raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
            key=hashlib.sha256(raw).hexdigest();(Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    for key,value,balance in cases:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'pet_care'})
                        self.assertFalse(response.isError)
                        self.assertEqual(json.loads(response.content[0].text)['data'],value['sections']['pet_care'])
                        response=await session.call_tool('character_summary',{'snapshot_id':key})
                        self.assertFalse(response.isError)
                        summary=json.loads(response.content[0].text)['active_pet_observation']
                        self.assertEqual(summary['training_points'],value['sections']['pet_care']['training_points'])
                        self.assertEqual(summary['training_points_balance'],balance)
                    response=await session.call_tool('character_summary',{'snapshot_id':unknown})
                    summary=json.loads(response.content[0].text)['active_pet_observation']
                    self.assertNotIn('training_points',summary)
                    self.assertNotIn('training_points_balance',summary)
                    response=await session.call_tool('compare_snapshots',{'before_id':cases[0][0],'after_id':cases[2][0]})
                    self.assertFalse(response.isError)
                    delta=json.loads(response.content[0].text)['pet_care_change']
                    self.assertEqual(delta['before']['training_points'],{'total':25,'used':10})
                    self.assertEqual(delta['after']['training_points'],{'total':10,'used':20})
                    self.assertEqual(delta['before']['training_points_balance'],15)
                    self.assertEqual(delta['after']['training_points_balance'],-10)
                    self.assertIsNone(delta['before_accepted_carried_food_quantity'])
                    self.assertIsNone(delta['after_accepted_carried_food_quantity'])
                    self.assertNotIn('training_points_delta',delta)
                    for key in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'pet_care'})
                        self.assertTrue(response.isError)


if __name__=='__main__':unittest.main()
