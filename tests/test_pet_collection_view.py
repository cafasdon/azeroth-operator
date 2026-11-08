"""Species-only filtered pet observations preserve ownership scope and privacy."""
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


class PetCollectionViewTests(unittest.IsolatedAsyncioTestCase):
    async def test_species_readback_and_validation(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY',root/'target/debug'/(
            'azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator'))).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder))
            value=fixture()
            value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            view={'coverage':'filtered_journal_species','displayed_rows':3,'entries':{
                '7':{'name':'Species 7','owned_rows':2,'unowned_rows':0},
                '8':{'name':'Species 8','owned_rows':0,'unowned_rows':1}}}
            value['sections']['pet_collection_view']=view
            key=store.ingest(value)
            empty=copy.deepcopy(value)
            empty['character']['name']='Empty'
            empty['sections']['pet_collection_view'].update(displayed_rows=0,entries={})
            empty_key=store.ingest(empty)
            missing=copy.deepcopy(value);missing['character']['name']='Missing';missing['sections'].pop('pet_collection_view')
            missing_key=store.ingest(missing)
            later=copy.deepcopy(value)
            later['captured_at']='2025-01-02T00:00:00Z'
            later['sections'].pop('pet_collection_view')
            store.ingest(later)
            bad_keys=[]
            for change in [
                lambda x:x['client'].update(family='retail'),
                lambda x:x['sections']['pet_collection_view'].update(coverage='full_collection'),
                lambda x:x['sections']['pet_collection_view'].update(displayed_rows=4),
                lambda x:x['sections']['pet_collection_view']['entries']['7'].update(owned_rows=True),
                lambda x:x['sections']['pet_collection_view']['entries']['7'].update(pet_guid='private'),
                lambda x:x['sections']['pet_collection_view']['entries']['7'].update(custom_name='private'),
                lambda x:x['sections']['pet_collection_view']['entries']['7'].update(name='é'*129),
                lambda x:x['sections']['pet_collection_view']['entries'].update({'0':{'name':'Bad','owned_rows':1,'unowned_rows':0}}),
            ]:
                bad=copy.deepcopy(value);change(bad)
                with self.assertRaises(ValueError):validate_snapshot(bad)
                raw=json.dumps(bad,sort_keys=True,separators=(',',':')).encode()
                bad_key=hashlib.sha256(raw).hexdigest()
                (Path(folder)/f'{bad_key}.json').write_bytes(raw)
                bad_keys.append(bad_key)
            env={**os.environ,'AZEROTH_DATA_DIR':folder};env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def search(**arguments):
                        response=await session.call_tool('list_snapshots',{'family':'forever',**arguments})
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)
                    for species,owned,unowned,expected in [(7,2,0,1),(7,3,0,0),(7,1,1,0),
                        (8,0,1,1),(8,1,0,0),(9,0,0,0)]:
                        args={'character':value['character'],'pet_species':{'species_id':species,
                            'minimum_owned_rows':owned,'minimum_unowned_rows':unowned}}
                        rows=(await search(**args))['snapshots']
                        self.assertEqual(len(rows),expected)
                        if rows:
                            self.assertEqual(rows[0]['snapshot_id'],key)
                            self.assertEqual(rows[0]['pet_species_match'],{
                                'coverage':'filtered_journal_species','displayed_rows':3,'species_id':species,
                                **view['entries'][str(species)]})
                    args={'character':value['character'],'pet_species':{'species_id':7}}
                    self.assertEqual(len((await search(**args))['snapshots']),1)
                    self.assertEqual((await search(**args,latest_per_character=True))['snapshots'],[])
                    self.assertEqual(len((await search(**args,latest_per_character=True,
                        captured_through='2025-01-01T00:00:00Z'))['snapshots']),1)
                    self.assertTrue(all('pet_species_match' not in row for row in (await search())['snapshots']))
                    for bad in [{'species_id':0},{'species_id':True},{'species_id':2**32},
                        {'species_id':7,'minimum_owned_rows':-1},{'species_id':7,'minimum_unowned_rows':True},
                        {'species_id':7,'minimum_owned_rows':2**32},{'species_id':7,'pet_guid':'private'},{}]:
                        response=await session.call_tool('list_snapshots',{'family':'forever','pet_species':bad})
                        self.assertTrue(response.isError)
                    response=await session.call_tool('list_snapshots',{'family':'retail','pet_species':{'species_id':7}})
                    self.assertTrue(response.isError)
                    response=await session.call_tool('inspect_snapshot',{'snapshot_id':key,'section':'pet_collection_view'})
                    self.assertFalse(response.isError)
                    self.assertEqual(json.loads(response.content[0].text)['data'],view)
                    for source,expected in [(key,{'displayed_rows':3,'species_observed':2,'owned_rows_observed':2,'unowned_rows_observed':1}),
                        (empty_key,{'displayed_rows':0,'species_observed':0,'owned_rows_observed':0,'unowned_rows_observed':0}),
                        (missing_key,None)]:
                        response=await session.call_tool('character_summary',{'snapshot_id':source})
                        self.assertFalse(response.isError)
                        self.assertEqual(json.loads(response.content[0].text)['pet_collection_view_counts'],expected)
                    for source in bad_keys:
                        response=await session.call_tool('inspect_snapshot',{'snapshot_id':source,'section':'pet_collection_view'})
                        self.assertTrue(response.isError)


if __name__=='__main__':
    unittest.main()
