"""Net chain leaves join exact historical owner reference IDs, without stock or eligibility inference."""
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


def refs(ids, total=None):
    return {'entries':{str(i):True for i in ids},'distinct_id_count':len(ids) if total is None else total,
        'truncated':total is not None and total>8}


class ChainCapturedSourcesTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_mcp_provenance_statuses_net_demand_context_and_composition(self):
        root=Path(__file__).resolve().parents[1]
        binary=Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name=='nt' else 'azeroth-operator')).resolve(strict=True)
        base=fixture()
        base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['bags']={'0':{'1':{'item_id':2840,'count':1,'link':'PRIVATE_LINK'}}}
        base['sections']['quests']={'42':{'title':'Fixture quest','level':60}}
        base['sections']['profession_recipe_materials']={'coverage':'reported_known_spell_schematics',
            'profession_id':164,'entries':{'101':fixed(100,{'20':3,'30':1,'40':1,'50':1,'60':1}),
                '102':fixed(20,{'2840':2},2)}}
        base['sections']['recipe_item_sources']={'coverage':'first_reported_fixed_recipe_reagents',
            'provider':'questiedb','provider_mode':'baked','provider_contract_version':4,'profession_id':164,
            'reported_item_count':4,'entries':{'2840':{'status':'reported','npc_drops':refs(list(range(1,9)),10),
                'object_drops':refs([]),'container_drops':refs([4]),'quest_rewards':refs([77])},
                '30':{'status':'not_in_provider'},'40':{'status':'unavailable'},'50':{'status':'reported'}}}
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder)); key=store.ingest(base)
            bank=deepcopy(base); bank['sections']['character_bank']={'6':{'1':{'item_id':2840,'count':2}}}
            bank_key=store.ingest(bank)
            missing=deepcopy(base); del missing['sections']['recipe_item_sources']; missing_key=store.ingest(missing)
            empty=deepcopy(base); empty['sections']['profession_recipe_materials']['entries']['101']['reagents']={}
            empty_key=store.ingest(empty)
            blocked=deepcopy(base); blocked['sections']['profession_recipe_materials']['entries']['102']['reagents']={'100':1}
            blocked_key=store.ingest(blocked)
            ambiguous=deepcopy(base)
            ambiguous['sections']['profession_recipe_materials']['entries']['103']=fixed(20,{'60':1},2)
            ambiguous_key=store.ingest(ambiguous)
            inventory=deepcopy(base); inventory['captured_at']='2025-01-02T00:00:00Z'
            inventory['sections']['bags']['0']['1']['count']=2
            inventory['sections']['recipe_item_sources']['entries']['2840']={'status':'reported','vendors':refs([999])}
            inventory_key=store.ingest(inventory)
            older=deepcopy(base); older['captured_at']='2024-12-31T00:00:00Z'
            older['sections']['recipe_item_sources'].update(profession_id=165,provider_mode='source',reported_item_count=1,
                entries={'2840':{'status':'reported','vendors':refs([91])}})
            older_key=store.ingest(older)
            wrong=[]
            for section,field,value in [('character','name','Other'),('character','realm','Other'),
                ('client','build','70863'),('client','interface',16002),('client','variant','test')]:
                saved=deepcopy(older); saved[section][field]=value; wrong.append(store.ingest(saved))
            before={p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder}; env.pop('AZEROTH_FOREVER_CATALOGUE',None)
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    schema=next(t for t in (await session.list_tools()).tools if t.name=='review_forever_recipe').inputSchema
                    self.assertIn('reagent_chain_captured_sources',schema['properties'])
                    self.assertIn('reagent_source_snapshot_id',schema['properties'])
                    async def call(snapshot=key,**extra):
                        reply=await session.call_tool('review_forever_recipe',{'snapshot_id':snapshot,'recipe_id':101,
                            'crafts':1,'suggest_reagent_chain':True,**extra})
                        self.assertFalse(reply.isError,reply)
                        result=json.loads(reply.content[0].text)
                        self.assertNotIn('PRIVATE',json.dumps(result))
                        return result
                    plain=await call(); self.assertNotIn('captured_sources',plain['reagent_chain'])
                    self.assertEqual(plain,await call(reagent_chain_captured_sources=False))
                    result=await call(reagent_chain_captured_sources=True)
                    source=result['reagent_chain'].pop('captured_sources'); self.assertEqual(result,plain)
                    self.assertEqual(source['snapshot_id'],key)
                    self.assertEqual(source['captured_at'],base['captured_at'])
                    self.assertTrue(source['section_available'])
                    self.assertEqual(source['external_item_count'],5)
                    self.assertEqual(source['captured_item_count'],4)
                    self.assertEqual(source['uncaptured_item_count'],1)
                    self.assertEqual(source['items_reported_by_provider'],2)
                    self.assertEqual(source['items_with_retained_sources'],1)
                    rows={r['item_id']:r for r in source['records']}
                    self.assertEqual(rows[2840]['quantity'],3)
                    quest_request={'snapshot_id':key,'section':'quests','quest_id':42,
                        'quest_inventory_requirements':{'storage':'bags','items':[{'item_id':2840,'quantity':4}]},
                        'quest_inventory_source_snapshot_id':key}
                    reply=await session.call_tool('inspect_snapshot',quest_request)
                    self.assertFalse(reply.isError,reply)
                    report=json.loads(reply.content[0].text)['quest_inventory_captured_source_review']
                    self.assertEqual(report['records'],[rows[2840]])
                    self.assertEqual(report['snapshot_id'],key)
                    self.assertIs(report['quest_item_binding_verified'],False)
                    bank_reply=await session.call_tool('inspect_snapshot',{**quest_request,
                        'quest_inventory_bank_withdrawals':True,'quest_inventory_snapshot_id':bank_key})
                    self.assertFalse(bank_reply.isError,bank_reply)
                    bank_sources=json.loads(bank_reply.content[0].text)['quest_inventory_captured_source_review']
                    residual=bank_sources.pop('after_observed_bank_withdrawals')
                    self.assertEqual(bank_sources,report)
                    self.assertEqual(residual['records'],[{**rows[2840],'quantity':1}])
                    reply=await session.call_tool('inspect_snapshot',{**quest_request,
                        'quest_inventory_source_snapshot_id':older_key,'quest_inventory_snapshot_id':inventory_key})
                    self.assertFalse(reply.isError,reply)
                    report=json.loads(reply.content[0].text)['quest_inventory_captured_source_review']
                    self.assertEqual(report['records'][0]['quantity'],2)
                    self.assertEqual(report['records'][0]['references']['vendors'],refs([91]))
                    self.assertEqual(report['snapshot_id'],older_key)
                    self.assertNotIn(20,rows)
                    self.assertEqual(rows[2840]['references'],{k:v for k,v in
                        base['sections']['recipe_item_sources']['entries']['2840'].items() if k!='status'})
                    self.assertEqual(rows[2840]['unknown_reference_categories'],['vendors'])
                    self.assertEqual(rows[2840]['truncated_reference_categories'],['npc_drops'])
                    self.assertEqual(rows[30]['status'],'not_in_provider'); self.assertIsNone(rows[30]['references'])
                    self.assertEqual(rows[40]['status'],'unavailable')
                    self.assertEqual(rows[50]['references'],{}); self.assertEqual(len(rows[50]['unknown_reference_categories']),5)
                    preview=(await call(ambiguous_key,reagent_chain_captured_sources=True,
                                        preview_reagent_producer_choices=True))['reagent_chain']
                    for candidate in preview['producer_previews']['candidate_previews']:
                        explicit=(await call(ambiguous_key,reagent_chain_captured_sources=True,
                            reagent_producer_choices=[{'item_id':20,'recipe_id':candidate['recipe_id']}]))[
                                'reagent_chain']['captured_sources']
                        summary=candidate['captured_source_summary']
                        self.assertEqual(summary['snapshot_id'],ambiguous_key)
                        self.assertEqual(summary['records'],explicit['records'])
                        self.assertIs(summary['records_truncated'],False)
                    self.assertEqual(rows[60]['status'],'item_not_captured')
                    for flag in ['dataset_revision_verified','complete_world_sources_verified','current_availability_verified',
                        'acquisition_eligibility_verified','ownership_verified','live','game_execution']:
                        self.assertFalse(source[flag])
                    absent=(await call(missing_key,reagent_chain_captured_sources=True))['reagent_chain']['captured_sources']
                    self.assertFalse(absent['section_available']); self.assertIsNone(absent['source_capture_coverage'])
                    self.assertEqual(absent['captured_item_count'],0)
                    self.assertTrue(all(r['status']=='capture_unavailable' for r in absent['records']))
                    self.assertEqual((await call(empty_key,reagent_chain_captured_sources=True))
                        ['reagent_chain']['captured_sources']['records'],[])
                    self.assertIsNone((await call(blocked_key,reagent_chain_captured_sources=True))['reagent_chain']['captured_sources'])
                    later=await call(reagent_chain_captured_sources=True,inventory_snapshot_id=inventory_key)
                    self.assertEqual(later['reagent_chain']['inventory_captured_at'],inventory['captured_at'])
                    self.assertEqual(later['reagent_chain']['captured_sources']['snapshot_id'],key)
                    self.assertEqual(later['reagent_chain']['captured_sources']['captured_at'],base['captured_at'])
                    self.assertEqual(next(r for r in later['reagent_chain']['captured_sources']['records'] if r['item_id']==2840)['quantity'],2)
                    self.assertNotIn('999',json.dumps(later['reagent_chain']['captured_sources']))
                    explicit=await call(reagent_chain_captured_sources=True,reagent_source_snapshot_id=older_key)
                    chosen=explicit['reagent_chain']['captured_sources']
                    self.assertEqual(chosen['snapshot_id'],older_key); self.assertEqual(chosen['captured_at'],older['captured_at'])
                    self.assertEqual(chosen['source_capture_coverage']['profession_id'],165)
                    self.assertEqual(next(r for r in chosen['records'] if r['item_id']==2840)['references'],{'vendors':refs([91])})
                    composed=await call(reagent_chain_captured_sources=True,reagent_chain_stock_plan=True,merchant_reagent_plan=True,
                        reagent_producer_choices=[{'item_id':20,'recipe_id':102}])
                    self.assertEqual(composed['reagent_chain']['captured_sources'],source)
                    for selector in wrong+['0'*64,'../PRIVATE']:
                        for snapshot in [key,blocked_key]:
                            reply=await session.call_tool('review_forever_recipe',{'snapshot_id':snapshot,'recipe_id':101,
                                'crafts':1,'suggest_reagent_chain':True,'reagent_chain_captured_sources':True,
                                'reagent_source_snapshot_id':selector})
                            self.assertTrue(reply.isError,reply); self.assertNotIn('PRIVATE',str(reply))
                    for bad in [dict(suggest_reagent_chain=False),dict(recipe_id=None),dict(reagent_chain_captured_sources=1),
                        dict(reagent_chain_captured_sources=False,reagent_source_snapshot_id=older_key),
                        dict(additional_recipes=[{'snapshot_id':key,'recipe_id':101,'crafts':1}]),dict(suggest_dependency_order=True)]:
                        reply=await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':101,'crafts':1,
                            'suggest_reagent_chain':True,'reagent_chain_captured_sources':True,**bad})
                        self.assertTrue(reply.isError,reply)
            cache=Path(os.environ.get('AZEROTH_FOREVER_CATALOGUE') or root/'local-data/forever/catalogue.json')
            if cache.is_file():
                env['AZEROTH_FOREVER_CATALOGUE']=str(cache.resolve())
                async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                    async with ClientSession(read,write) as session:
                        await session.initialize()
                        reply=await session.call_tool('review_forever_recipe',{'snapshot_id':key,'recipe_id':101,'crafts':1,
                            'suggest_reagent_chain':True,'reagent_chain_captured_sources':True,'reagent_chain_catalogue_sources':True})
                        self.assertFalse(reply.isError,reply)
                        combined=json.loads(reply.content[0].text)['reagent_chain']
                        self.assertEqual(combined['captured_sources'],source)
                        self.assertIn('catalogue_sources',combined)
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


if __name__=='__main__': unittest.main()
