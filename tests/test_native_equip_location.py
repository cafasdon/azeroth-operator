"""Native location categories annotate review drafts without proving equip eligibility."""
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


class NativeEquipLocationTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_native_categories_and_readers(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        value=fixture();value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['bags']={'0':{'1':{'item_id':721,'count':1,'link':'item:721:0:0:0:0:0:0'}}}
        cases={'HEAD':[1],'NECK':[2],'SHOULDER':[3],'BODY':[4],'CHEST':[5],'ROBE':[5],'WAIST':[6],
            'LEGS':[7],'FEET':[8],'WRIST':[9],'HAND':[10],'FINGER':[11,12],'TRINKET':[13,14],'CLOAK':[15],
            'WEAPONMAINHAND':[16],'WEAPONOFFHAND':[17],'SHIELD':[17],'HOLDABLE':[17],'WEAPON':[16,17],
            '2HWEAPON':[16,17],'RANGED':[18],'RANGEDRIGHT':[18],'THROWN':[18],'RELIC':[18],'TABARD':[19]}
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder));keys={}
            for token in [*cases,'FUTURE_99','BAG','']:
                sample=deepcopy(value);sample['sections']['bags']['0']['1']['equip_location']='INVTYPE_'+token if token else ''
                keys[token]=store.ingest(sample)
            legacy_key=store.ingest(value)
            bad_keys=[]
            for location in [None,True,0,[],{},'finger','INVTYPE_','INVTYPE_HEAD\n','INVTYPE_é','INVTYPE_'+('A'*57)]:
                bad=deepcopy(value);bad['sections']['bags']['0']['1']['equip_location']=location
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            bad=deepcopy(value);bad['sections']['bags']['0']['1']['equip_location']='INVTYPE_FINGER';bad['sections']['bags']['0']['1'].pop('link')
            with self.assertRaises(ValueError):validate_snapshot(bad)
            raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode();key=hashlib.sha256(raw).hexdigest()
            (Path(folder)/f'{key}.json').write_bytes(raw);bad_keys.append(key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def review(key,target):
                        response=await session.call_tool('review_equipment_selection',{'snapshot_id':key,'selections':[
                            {'target_slot':target,'expected_item_id':721,'source':{'kind':'bag','bag':0,'slot':1}}]})
                        self.assertFalse(response.isError,response.content)
                        result=json.loads(response.content[0].text)
                        self.assertEqual(result['checks']['slot_compatibility'],'unknown')
                        return result['changes'][0]['native_slot_category']
                    for token,slots in cases.items():
                        for target in [slots[0],1 if slots[0]!=1 else 2]:
                            category=await review(keys[token],target)
                            self.assertEqual(category['candidate_equipment_slots'],slots)
                            self.assertEqual(category['target_slot_category_matches'],target in slots)
                            self.assertFalse(category['eligibility_verified'])
                    for target in [11,12,13]:
                        self.assertEqual((await review(keys['FINGER'],target))['target_slot_category_matches'],target in [11,12])
                    self.assertIsNone((await review(keys['2HWEAPON'],17))['target_slot_category_matches'])
                    for token in ['FUTURE_99','BAG','']:
                        category=await review(keys[token],16)
                        self.assertIsNone(category['candidate_equipment_slots']);self.assertIsNone(category['target_slot_category_matches'])
                    self.assertIsNone((await review(legacy_key,16))['observed_equip_location'])
                    for bad in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':bad,'section':'bags'})
                        self.assertTrue(response.isError)


if __name__=='__main__':unittest.main()
