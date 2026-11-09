"""Exact-slot MCP inputs require the qualified engine, including cached execution."""
import asyncio
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import tempfile
from time import monotonic
import unittest
from unittest.mock import Mock, patch

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools import evaluate_forever_owned_gear as workflow
from tools import run_forever_sim as adapter
from tools import positional_equipment as layout
from tools.merge_forever_gear_windows import retained_equipment


class PositionalMcpTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_mcp_layout_batch_default_and_native_guards(self):
        root=Path(__file__).resolve().parents[1]
        cache=root/'local-data/forever/catalogue.json'
        if not cache.is_file(): self.skipTest('Local pinned catalogue required')
        binary=(root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        base=fixture(); base['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        base['sections']['stats']={'class':'HUNTER','race':'Orc','level':60,'client_version':'synthetic'}
        base['sections']['bags']={'0':{'1':{'item_id':4549,'count':1,'link':'item:4549:0:0:0:0:0:0'}}}
        with tempfile.TemporaryDirectory() as folder:
            store=SnapshotStore(Path(folder)); key=store.ingest(base)
            conflict=deepcopy(base); conflict['sections']['bags']['0']['1']['equip_location']='INVTYPE_TRINKET'
            conflict_key=store.ingest(conflict)
            level=deepcopy(base); level['sections']['bags']['0']['1']['required_level']=61; level_key=store.ingest(level)
            retail=deepcopy(base); retail['client']['family']='retail'; retail_key=store.ingest(retail)
            before={p.name:p.read_bytes() for p in Path(folder).glob('*.json')}
            env={**os.environ,'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(cache.resolve())}
            selected={'target_slot':12,'source':{'kind':'bag','bag':0,'slot':1},'expected_item_id':4549}
            async with stdio_client(StdioServerParameters(command=str(binary),env=env)) as (read,write):
                async with ClientSession(read,write) as session:
                    await session.initialize()
                    async def call(snapshot=key,**extra):
                        reply=await session.call_tool('review_equipment_selection',{'snapshot_id':snapshot,'selections':[selected],**extra})
                        self.assertFalse(reply.isError,reply)
                        return json.loads(reply.content[0].text)
                    old=await call(); self.assertIsNone(old['checks']['simulator_equipment_spec'])
                    self.assertEqual(old,await call(positional_simulator_equipment=False))
                    exact=await call(positional_simulator_equipment=True)
                    spec=exact['checks']['simulator_equipment_spec']
                    self.assertEqual(exact['checks']['simulator_equipment_contract'],layout.CONTRACT)
                    self.assertIs(spec['preserveSlots'],True)
                    self.assertEqual(spec['items'][10],{})
                    self.assertEqual(spec['items'][11]['id'],4549)
                    self.assertFalse(exact['game_execution']); self.assertFalse(exact['live'])
                    batch=await call(positional_simulator_equipment=True,additional_drafts=[{'selections':[]}])
                    self.assertIs(batch['additional_draft_reviews'][0]['checks']['simulator_equipment_spec']['preserveSlots'],True)
                    for bad in [conflict_key,level_key]:
                        guarded=await call(bad,positional_simulator_equipment=True)
                        self.assertIsNone(guarded['checks']['simulator_equipment_spec'])
                    for fields in [{'positional_simulator_equipment':1},{'positional_simulator_equipment':None},
                        {'snapshot_id':retail_key,'positional_simulator_equipment':True}]:
                        reply=await session.call_tool('review_equipment_selection',{'snapshot_id':key,'selections':[selected],**fields})
                        self.assertTrue(reply.isError,reply)
            self.assertEqual(before,{p.name:p.read_bytes() for p in Path(folder).glob('*.json')})


class PositionalExecutionTests(unittest.TestCase):
    def test_review_contract_and_positional_flag_must_agree(self):
        template,review,encounter=inputs()
        review['checks']['simulator_equipment_contract']=layout.CONTRACT
        with self.assertRaises(ValueError): adapter.request_from(template,review,encounter)
        review['checks']['simulator_equipment_spec']['preserveSlots']=True
        self.assertTrue(layout.request_requires_positional(adapter.request_from(template,review,encounter)))
        for contract in [None,'unsupported']:
            review['checks']['simulator_equipment_contract']=contract
            with self.assertRaises(ValueError): adapter.request_from(template,review,encounter)

    def engine(self):
        if not os.environ.get('AZEROTH_POSITIONAL_ARTIFACTS'): self.skipTest('Explicit qualified engine artifacts required')
        engine=Path(os.environ['AZEROTH_POSITIONAL_ARTIFACTS'])/'wowsimcli-positional.exe'
        self.assertEqual(hashlib.sha256(engine.read_bytes()).hexdigest(),layout.ENGINE_SHA256)
        return engine.resolve(strict=True)

    def test_capability_and_request_guards_precede_cache_or_bulk_execution(self):
        engine=self.engine()
        spec={'items':[{} for _ in range(17)],'preserveSlots':True}
        request={'raid':{'parties':[{'players':[{'equipment':spec}]}]},'simOptions':{'iterations':1,'randomSeed':'1'}}
        adapter.verify_positional_engine(engine,layout.ENGINE_SHA256,monotonic()+10)
        for field in ['preserve_slots','preserveSlots']:
            bad=deepcopy(request); bad['raid']['parties'][0]['players'][0]['equipment'][field]=1
            with self.assertRaises(ValueError): adapter.verify_request_engine(engine,layout.ENGINE_SHA256,bad)
        base=json.dumps(layout.CAPABILITY,separators=(',',':'))
        duplicate=base[:-1]+',"contract":"'+layout.CONTRACT+'"}'
        malformed=[duplicate,json.dumps({**layout.CAPABILITY,'required_item_count':17.0}),
            json.dumps({**layout.CAPABILITY,'native_qualification_performed':0}),
            json.dumps({**layout.CAPABILITY,'extra':'PRIVATE'}),'x'*4097]
        for text in malformed:
            def capability(command,**kwargs): kwargs['stdout'].write(text.encode())
            cache=Mock(); cache.get.return_value={'mean_dps':1}
            with patch.object(adapter.subprocess,'run',side_effect=capability):
                with self.assertRaises(ValueError): adapter.run(engine,layout.ENGINE_SHA256,request,cache=cache)
            cache.key.assert_not_called(); cache.get.assert_not_called()
        with patch.object(adapter.subprocess,'run') as child:
            with self.assertRaises(ValueError): adapter.run(engine,'0'*64,request,cache=Mock())
            with self.assertRaises(ValueError): adapter.compare_bulk(engine,layout.ENGINE_SHA256,[request],1)
            child.assert_not_called()
        for flag in [False,1,None]:
            bad=deepcopy(spec); bad['preserveSlots']=flag
            digest=hashlib.sha256(json.dumps(bad,sort_keys=True,separators=(',',':')).encode()).hexdigest()
            with self.assertRaises(ValueError): retained_equipment({'simulator_equipment_spec':bad,'simulator_gear_sha256':digest})

    def test_actual_all_slot_positional_workflow_resolves_hole_and_reuses_results(self):
        engine=self.engine(); root=Path(__file__).resolve().parents[1]
        if not os.environ.get('AZEROTH_QUALIFY_TEMPLATE'): self.skipTest('Explicit public template required')
        template=Path(os.environ['AZEROTH_QUALIFY_TEMPLATE']).resolve(strict=True)
        self.assertEqual(hashlib.sha256(template.read_bytes()).hexdigest(),
            '761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0')
        binary=(root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        cache=(root/'local-data/forever/catalogue.json').resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); saved=fixture()
            saved['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
            saved['sections']['stats']={'class':'HUNTER','race':'Orc','level':60,'client_version':'synthetic'}
            saved['sections']['professions']={str(i):{'name':'Synthetic','rank':300,'max_rank':300,'skill_line':skill}
                for i,skill in enumerate([202,164],1)}
            saved['sections']['pet_care']={'present':True,'level':60}
            saved['sections']['equipment']={str(slot):{'item_id':item,'link':f'item:{item}:0:0:0:0:0:0'}
                for slot,item in [(16,2042),(18,22347)]}
            saved['sections']['bags']={'0':{'1':{'item_id':4549,'count':1,'link':'item:4549:0:0:0:0:0:0'}}}
            key=SnapshotStore(base).ingest(saved); snapshot=(base/f'{key}.json').read_bytes()
            _,_,encounter=inputs(); encounter['simulation_input']['simOptions']={'iterations':1000,'randomSeed':'20261109'}
            first=base/'first.json'; first.write_text(json.dumps(encounter))
            encounter['simulation_input']['encounter']['duration']=90
            second=base/'second.json'; second.write_text(json.dumps(encounter))
            argv=['--mcp',str(binary),'--snapshot-id',key,'--template',str(template),'--journal',str(base/'plans.sqlite'),
                '--engine',str(engine),'--engine-sha256',layout.ENGINE_SHA256,'--simulation-journal',str(base/'results'),
                '--encounter-lookup',str(first),'--encounter-lookup',str(second),'--encounter-weight','1','--encounter-weight','3',
                '--positional-equipment','--max-windows','64']
            env={'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(cache)}
            with patch.dict(os.environ,env):
                cold=asyncio.run(workflow.execute(workflow.parser().parse_args(argv)))
                plans=(base/'plans.sqlite').read_bytes()
                warm=asyncio.run(workflow.execute(workflow.parser().parse_args([*argv,'--checkpoint-only'])))
                robust=asyncio.run(workflow.execute(workflow.parser().parse_args([*argv,'--checkpoint-only',
                    '--ranking-objective','worst_encounter_gain'])))
                self.assertEqual(robust['ranking_objective'],'worst_encounter_gain')
                self.assertEqual(robust['simulation']['ranking_objective'],'worst_encounter_gain')
                self.assertEqual(robust['simulation']['new_runs'],0)
                self.assertTrue(robust['reported_space_results_complete'])
                self.assertEqual([row['ranking_score'] for row in robust['simulation']['ranked_gear_sets']],
                    sorted((min(row['gain_pct_by_encounter']) for row in warm['simulation']['ranked_gear_sets']),reverse=True))
                self.assertEqual(cold['status'],'reported_space_complete')
                self.assertTrue(cold['reported_space_results_complete'])
                self.assertEqual(cold['collection']['progress']['total_combinations_decimal'],'4')
                self.assertEqual(cold['collection']['progress']['resolution_accounting']['unresolved_proposed_coordinates'],0)
                self.assertEqual(cold['simulation']['new_runs'],6)
                self.assertEqual(warm['simulation']['new_runs'],0)
                self.assertEqual(cold['simulation']['ranked_gear_sets'],warm['simulation']['ranked_gear_sets'])
                self.assertEqual(len(cold['simulation']['ranked_gear_sets']),3)
                for gear in cold['simulation']['ranked_gear_sets']:
                    saved=gear['saved_loadout']
                    self.assertEqual(saved['snapshot_id'],key)
                    self.assertIs(saved['simulator_equipment_spec']['preserveSlots'],True)
                    self.assertEqual(hashlib.sha256(json.dumps(saved['simulator_equipment_spec'],
                        sort_keys=True,separators=(',',':')).encode()).hexdigest(),gear['simulator_gear_sha256'])
                # Layout selection belongs to the collection binding: no mixed resume.
                with self.assertRaises(Exception):
                    asyncio.run(workflow.execute(workflow.parser().parse_args([x for x in argv if x!='--positional-equipment'])))
                with self.assertRaises(Exception):
                    asyncio.run(workflow.execute(workflow.parser().parse_args([*argv,'--engine-sha256','0'*64,
                        '--journal',str(base/'must-not-create.sqlite')])))
                self.assertFalse((base/'must-not-create.sqlite').exists())
            self.assertEqual((base/'plans.sqlite').read_bytes(),plans)
            self.assertEqual((base/f'{key}.json').read_bytes(),snapshot)
            for flag in ['best_in_slot_verified','complete_ownership_verified','game_execution','global_ranking_verified']:
                self.assertFalse(cold[flag])
            print(json.dumps({'qualification':'positional_mcp_owned_workflow','coordinates':4,'unresolved':0,
                'cold_runs':6,'warm_runs':0,'ranked_sets':3,'game_execution':False}))


if __name__=='__main__': unittest.main()
