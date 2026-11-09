"""Bulk cooldown observations retain their recipe source through planning."""
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


class BulkRecipeCooldownTests(unittest.IsolatedAsyncioTestCase):
    async def test_reader_parity_search_plans_batch_and_capture_provenance(self):
        root=Path(__file__).resolve().parents[1]
        binary=(root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base=fixture(); base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['character']['name']='PRIVATE_CHARACTER'
        base['sections']['bags']={'0':{'1':{'item_id':10,'count':6}}}
        base['sections']['profession_recipe_materials']={
            'coverage':'reported_known_spell_schematics','profession_id':164,'entries':{
                '101':{'status':'fixed_items','reagents':{'10':2}},
                '102':{'status':'fixed_items','reagents':{},'cooldown':{'charges':0,'max_charges':2}}}}
        valid=[None,{'remaining_seconds':0},{'remaining_seconds':60,'daily':True},
            {'charges':0,'max_charges':2},{'charges':2,'max_charges':3},
            {'remaining_seconds':60,'daily':False,'charges':1,'max_charges':2},
            {'remaining_seconds':4294967295,'charges':4294967295,'max_charges':4294967295}]
        invalid=[None,{},True,[],{'daily':False},{'remaining_seconds':None},{'remaining_seconds':True},
            {'remaining_seconds':1.5},{'remaining_seconds':-1},{'remaining_seconds':4294967296},
            {'charges':1},{'max_charges':2},{'charges':0,'max_charges':0},
            {'charges':3,'max_charges':2},{'charges':False,'max_charges':2},
            {'charges':1,'max_charges':None},{'remaining_seconds':0,'daily':None},
            {'remaining_seconds':0,'daily':0},{'remaining_seconds':0,'PRIVATE':'PRIVATE'}]
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder)); good=[]; bad=[]
            for cooldown in valid:
                saved=deepcopy(base)
                if cooldown is not None: saved['sections']['profession_recipe_materials']['entries']['101']['cooldown']=cooldown
                good.append(store.ingest(saved))
            for cooldown in invalid:
                saved=deepcopy(base); saved['sections']['profession_recipe_materials']['entries']['101']['cooldown']=cooldown
                with self.assertRaises(ValueError): validate_snapshot(saved)
                raw=json.dumps(saved,sort_keys=True,separators=(',',':')).encode()
                key=hashlib.sha256(raw).hexdigest(); (Path(folder)/f'{key}.json').write_bytes(raw); bad.append(key)
            unsupported=deepcopy(base)
            unsupported['sections']['profession_recipe_materials']['entries']['101']={'status':'unavailable','cooldown':{'remaining_seconds':0}}
            with self.assertRaises(ValueError): validate_snapshot(unsupported)
            raw=json.dumps(unsupported,sort_keys=True,separators=(',',':')).encode()
            key=hashlib.sha256(raw).hexdigest(); (Path(folder)/f'{key}.json').write_bytes(raw); bad.append(key)
            inventory=deepcopy(base); inventory['captured_at']='2025-01-02T00:00:00Z'
            inventory['sections']['bags']['0']['1']['count']=2
            inventory['sections']['profession_recipe_materials']['entries']['101']['cooldown']={'charges':9,'max_charges':9}
            newer=store.ingest(inventory)
            selected=deepcopy(base)
            selected['sections']['profession_recipe_materials']['entries']['101']['cooldown']=valid[5]
            selected['sections']['selected_recipe']={'recipe_id':101,'name':'Fixture','learned':True,
                'requirements':'fixed_items','reagents':{'10':2},'cooldown':{'charges':9,'max_charges':9}}
            same_id=store.ingest(selected)
            before={p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def review(key,**extra):
                        reply=await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':101,'crafts':3,**extra})
                        self.assertFalse(reply.isError,reply); self.assertNotIn('PRIVATE',str(reply))
                        return json.loads(reply.content[0].text)
                    for key,cooldown in zip(good,valid,strict=True):
                        plan=await review(key)
                        self.assertEqual(plan['recipe'].get('cooldown'),cooldown)
                        self.assertFalse(plan['craftability_verified']); self.assertFalse(plan['game_execution'])
                        charges=None if cooldown is None else cooldown.get('charges')
                        self.assertEqual(plan['captured_preparation']['observed_charges_cover_request'],None if charges is None else charges>=3)
                        self.assertEqual(plan['material_and_charge_capacity']['bags'],None if charges is None else
                            {'supported_crafts':min(3,charges),'shortfall_vs_requested':3-min(3,charges)})
                        found=await session.call_tool('search_forever_recipes',{'snapshot_id':key,'recipe_id':101})
                        self.assertFalse(found.isError,found)
                        self.assertEqual(json.loads(found.content[0].text)['records'][0].get('cooldown'),cooldown)
                    plan=await review(good[5],inventory_snapshot_id=newer)
                    self.assertEqual(plan['recipe']['cooldown'],valid[5])
                    self.assertEqual(plan['estimated_cooldown_end_at'],'2025-01-01T00:01:00Z')
                    self.assertEqual(plan['inventory_captured_at'],inventory['captured_at'])
                    self.assertEqual(plan['material_and_charge_capacity']['bags']['supported_crafts'],1)
                    self.assertEqual((await review(same_id))['recipe']['cooldown'],valid[5])
                    batch=await review(good[4],additional_recipes=[{'snapshot_id':good[4],'recipe_id':102,'crafts':1}])
                    failure=batch['ordered_nonmaterial_requirements']['first_observed_shortfall']
                    self.assertEqual((failure['recipe_number'],failure['recipe_id']),(1,101))
                    self.assertIn('observed_charges_cover_request',failure['failed_checks'])
                    for key in bad:
                        reply=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'profession_recipe_materials'})
                        self.assertTrue(reply.isError,reply); self.assertNotIn('PRIVATE',str(reply))
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
