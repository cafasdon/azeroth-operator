"""Chain acquisition hints use only pinned public cache, never inferred availability."""
from copy import deepcopy
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_recipe_reagent_chain import fixed


class ChainCatalogueSourcesTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_sources_unknowns_context_modes_and_pin(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        cache = Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root/'local-data/forever/catalogue.json')
        if not cache.is_file(): self.skipTest('Pinned local public catalogue not installed')
        public = json.loads(cache.read_bytes())
        index = {row['id']:row for row in public['items']}
        value = fixture()
        value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['bags'] = {'0':{'1':{'item_id':2505,'count':1,'link':'PRIVATE_LINK'}}}
        value['sections']['profession_recipe_materials'] = {
            'coverage':'reported_known_spell_schematics','profession_id':164,
            'entries':{'101':fixed(99999997,{'2505':3,'744':1,'872':2,'2870':1,'720':1,'2840':4})}}
        with tempfile.TemporaryDirectory() as folder:
            folder = Path(folder)
            store = SnapshotStore(folder/'captures')
            key = store.ingest(value)
            empty = deepcopy(value)
            empty['sections']['profession_recipe_materials']['entries']['101']['reagents'] = {}
            empty_key = store.ingest(empty)
            blocked = deepcopy(value)
            blocked['sections']['profession_recipe_materials']['entries']['102'] = fixed(2505,{'2505':1})
            blocked_key = store.ingest(blocked)
            ambiguous = deepcopy(blocked)
            ambiguous['sections']['profession_recipe_materials']['entries']['103'] = fixed(2505,{'2840':1})
            ambiguous_key = store.ingest(ambiguous)
            later = deepcopy(value)
            later['captured_at'] = '2025-01-02T00:00:00Z'
            later['sections']['bags']['0']['1']['count'] = 3
            later_key = store.ingest(later)
            wrong = deepcopy(later)
            wrong['character']['name'] = 'Other'
            wrong_key = store.ingest(wrong)
            before = {p.name:p.read_bytes() for p in (folder/'captures').glob('*.json')}
            copied = folder/'catalogue.json'
            copied.write_bytes(cache.read_bytes())
            env = {**os.environ,'AZEROTH_DATA_DIR':str(folder/'captures'),'AZEROTH_FOREVER_CATALOGUE':str(copied)}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    schemas = await session.list_tools()
                    schema = next(t for t in schemas.tools if t.name == 'review_forever_recipe').inputSchema
                    self.assertIn('reagent_chain_catalogue_sources',schema['properties'])
                    async def call(snapshot=key,**extra):
                        reply = await session.call_tool('review_forever_recipe',{'snapshot_id':snapshot,
                            'recipe_id':101,'crafts':1,'suggest_reagent_chain':True,**extra})
                        self.assertFalse(reply.isError,reply)
                        result = json.loads(reply.content[0].text)
                        self.assertNotIn('PRIVATE',json.dumps(result))
                        return result
                    baseline = await call()
                    self.assertNotIn('catalogue_sources',baseline['reagent_chain'])
                    self.assertEqual(baseline,await call(reagent_chain_catalogue_sources=False))
                    result = await call(reagent_chain_catalogue_sources=True)
                    sources = result['reagent_chain'].pop('catalogue_sources')
                    self.assertEqual(result,baseline)
                    self.assertEqual(sources['external_item_count'],6)
                    self.assertEqual(sources['matched_item_count'],5)
                    self.assertEqual(sources['unmatched_item_count'],1)
                    self.assertEqual(sources['items_with_recorded_sources'],4)
                    self.assertIn('sha256',sources['catalogue_source'])
                    rows = {row['item_id']:row for row in sources['records']}
                    self.assertEqual(rows[2505]['quantity'],2)  # already net; no second stock subtraction
                    for item in [2505,744,872,2870]:
                        self.assertEqual(rows[item]['sources'],index[item]['sources'][:8])
                        self.assertEqual(rows[item]['source_count'],len(index[item]['sources']))
                        self.assertFalse(rows[item]['sources_truncated'])
                    self.assertTrue(rows[2505]['source_references'])
                    self.assertEqual(rows[720]['status'],'source_metadata_missing')
                    self.assertIsNone(rows[720]['sources'])
                    self.assertEqual(rows[2840]['status'],'item_not_in_catalogue')
                    alternatives = (await call(ambiguous_key,reagent_chain_catalogue_sources=True,
                        preview_reagent_producer_choices=True))['reagent_chain']['producer_previews']['candidate_previews']
                    self.assertIsNone(alternatives[0]['catalogue_source_summary'])
                    explicit = (await call(ambiguous_key,reagent_chain_catalogue_sources=True,
                        reagent_producer_choices=[{'item_id':2505,'recipe_id':103}]))['reagent_chain']['catalogue_sources']
                    summary = alternatives[1]['catalogue_source_summary']
                    self.assertEqual(summary['records'],explicit['records'])
                    self.assertEqual(summary['catalogue_source'],explicit['catalogue_source'])
                    self.assertIs(summary['records_truncated'],False)
                    for flag in ['complete_world_sources_verified','current_availability_verified',
                        'acquisition_eligibility_verified','ownership_verified','live','game_execution']:
                        self.assertFalse(sources[flag])
                    self.assertIsNone((await call(blocked_key,reagent_chain_catalogue_sources=True))
                        ['reagent_chain']['catalogue_sources'])
                    none = (await call(empty_key,reagent_chain_catalogue_sources=True))['reagent_chain']['catalogue_sources']
                    self.assertEqual(none['records'],[])
                    self.assertEqual(none['external_item_count'],0)
                    override = (await call(reagent_chain_catalogue_sources=True,inventory_snapshot_id=later_key))
                    self.assertNotIn(2505,[row['item_id'] for row in override['reagent_chain']['catalogue_sources']['records']])
                    composed = await call(reagent_chain_catalogue_sources=True,reagent_chain_stock_plan=True,
                        merchant_reagent_plan=True)
                    self.assertEqual(composed['reagent_chain']['catalogue_sources'],sources)
                    for bad in [dict(suggest_reagent_chain=False),dict(reagent_chain_catalogue_sources=1),
                        dict(inventory_snapshot_id=wrong_key),dict(additional_recipes=[{'snapshot_id':key,'recipe_id':101,'crafts':1}])]:
                        args = {'snapshot_id':key,'recipe_id':101,'crafts':1,'suggest_reagent_chain':True,
                            'reagent_chain_catalogue_sources':True,**bad}
                        self.assertTrue((await session.call_tool('review_forever_recipe',args)).isError)
                    copied.write_bytes(b'corrupt')
                    self.assertTrue((await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':101,
                        'crafts':1,'suggest_reagent_chain':True,'reagent_chain_catalogue_sources':True})).isError)
                    await call()  # opt-out does not require the catalogue
            env.pop('AZEROTH_FOREVER_CATALOGUE')
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    reply = await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':101,
                        'crafts':1,'suggest_reagent_chain':True,'reagent_chain_catalogue_sources':True})
                    self.assertTrue(reply.isError)
            self.assertEqual(before,{p.name:p.read_bytes() for p in (folder/'captures').glob('*.json')})


if __name__ == '__main__': unittest.main()
