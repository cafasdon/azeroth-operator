"""Positional plan search retains legal in-scope equipment-source vacancies."""
import os
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from tools.rank_forever_slot import collect, plan_report
from tools.journal_forever_gear_windows import append_report, journal_status
from tools.merge_forever_gear_windows import merge_reports
from tools import evaluate_forever_owned_gear as workflow
from tools import positional_equipment as layout
from test_simulator_level import inputs


class EquippedVacancyTests(unittest.IsolatedAsyncioTestCase):
    async def test_opt_in_actual_engine_owned_workflow_preserves_vacated_source(self):
        if not all(os.environ.get(key) for key in ['AZEROTH_POSITIONAL_ARTIFACTS','AZEROTH_QUALIFY_TEMPLATE']):
            self.skipTest('Explicit local engine/template required; no download/build')
        root=Path(__file__).resolve().parents[1]
        engine=Path(os.environ['AZEROTH_POSITIONAL_ARTIFACTS'])/'wowsimcli-positional.exe'
        template=Path(os.environ['AZEROTH_QUALIFY_TEMPLATE']).resolve(strict=True)
        self.assertEqual(hashlib.sha256(engine.read_bytes()).hexdigest(),layout.ENGINE_SHA256)
        self.assertEqual(hashlib.sha256(template.read_bytes()).hexdigest(),
            '761afca4f5bca1d8b5825b6411d60cb804fd1b792cf7927aff7f3dbd3ee96dd0')
        value=fixture(); value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['stats']={'level':60,'class':'HUNTER','race':'Orc','client_version':'synthetic'}
        value['sections']['equipment']={str(slot):{'item_id':item,'link':f'item:{item}:0:0:0:0:0:0'}
            for slot,item in [(11,4549),(16,2042),(18,22347)]}
        value['sections']['professions']={str(i):{'name':'Synthetic','rank':300,'max_rank':300,'skill_line':skill}
            for i,skill in enumerate([202,164],1)}
        value['sections']['pet_care']={'present':True,'level':60}
        with tempfile.TemporaryDirectory() as folder:
            base=Path(folder); key=SnapshotStore(base).ingest(value); before=(base/f'{key}.json').read_bytes()
            _,_,encounter=inputs(); encounter['simulation_input']['simOptions']={'iterations':1000,'randomSeed':'20261109'}
            first=base/'first.json'; first.write_text(json.dumps(encounter))
            encounter['simulation_input']['encounter']['duration']=90
            second=base/'second.json'; second.write_text(json.dumps(encounter))
            argv=['--mcp',str((root/'target/debug/azeroth-operator.exe').resolve(strict=True)),
                '--snapshot-id',key,'--template',str(template),'--journal',str(base/'plans.sqlite'),
                '--engine',str(engine.resolve(strict=True)),'--engine-sha256',layout.ENGINE_SHA256,
                '--simulation-journal',str(base/'results'),'--encounter-lookup',str(first),
                '--encounter-lookup',str(second),'--positional-equipment','--max-windows','64']
            with patch.dict(os.environ,{'AZEROTH_DATA_DIR':folder,
                'AZEROTH_FOREVER_CATALOGUE':str((root/'local-data/forever/catalogue.json').resolve(strict=True))}):
                cold=await workflow.execute(workflow.parser().parse_args(argv))
                warm=await workflow.execute(workflow.parser().parse_args([*argv,'--checkpoint-only']))
            self.assertTrue(cold['reported_space_results_complete'])
            self.assertEqual(cold['simulation']['new_runs'],4)
            self.assertEqual(warm['simulation']['new_runs'],0)
            moved=[row['saved_loadout'] for row in cold['simulation']['ranked_gear_sets'] if row['saved_loadout']['selections']]
            self.assertEqual(len(moved),1)
            self.assertEqual(moved[0]['selections'][0]['source'],{'kind':'equipment','slot':11})
            self.assertEqual(moved[0]['simulator_equipment_spec']['items'][10],{})
            self.assertEqual(moved[0]['simulator_equipment_spec']['items'][11]['id'],4549)
            self.assertEqual(cold['simulation']['ranked_gear_sets'],warm['simulation']['ranked_gear_sets'])
            self.assertFalse(cold['best_in_slot_verified']); self.assertFalse(cold['game_execution'])
            self.assertEqual((base/f'{key}.json').read_bytes(),before)

    async def test_actual_mcp_vacancy_scope_budget_and_retained_plan(self):
        root=Path(__file__).resolve().parents[1]
        binary=(root/'target/debug/azeroth-operator.exe').resolve(strict=True)
        catalogue=root/'local-data/forever/catalogue.json'
        if not catalogue.is_file(): self.skipTest('Pinned local catalogue required; no downloads')
        value=fixture(); value['client']={'family':'forever','variant':'live','build':'70862','interface':16001}
        value['sections']['stats']={'level':60,'class':'HUNTER','race':'Orc','client_version':'synthetic'}
        value['sections']['equipment']={'11':{'item_id':4549,'link':'item:4549:0:0:0:0:0:0'}}
        with tempfile.TemporaryDirectory() as folder,tempfile.TemporaryFile(mode='w+') as errlog:
            base=Path(folder); key=SnapshotStore(base).ingest(value)
            before=(base/f'{key}.json').read_bytes()
            env={'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            async def run(slots=('finger1','finger2'),positional=True,max_swaps=None):
                with patch.dict(os.environ,env):
                    return await collect(binary,key,list(slots),errlog,collapse_identical=True,
                        combination_limit=64,positional_equipment=positional,max_swaps=max_swaps)
            result=await run()
            drafts,skipped,coverage,_,_,_,search=result
            self.assertEqual(len(drafts),2)
            selected,review=drafts[1]
            self.assertEqual(selected,[{'target_slot':12,'source':{'kind':'equipment','slot':11},'expected_item_id':4549}])
            self.assertEqual(review['vacated_equipment_slots'],[11])
            spec=review['checks']['simulator_equipment_spec']
            self.assertIs(spec['preserveSlots'],True)
            self.assertEqual(spec['items'][10],{})
            self.assertEqual(spec['items'][11]['id'],4549)
            self.assertEqual([row['status'] for row in search['combination_window']['coordinate_outcomes']],['baseline','resolved'])
            self.assertFalse(any(row['reason']=='would_vacate_another_slot' for row in skipped))
            artifact=plan_report(drafts,True,True)
            artifact.update(gear_search=search,skipped=skipped,storage_coverage=coverage,best_in_slot_verified=False)
            self.assertEqual(merge_reports([artifact])['resolution_accounting']['unresolved_proposed_coordinates'],0)
            journal=base/'plans.sqlite'; append_report(journal,artifact)
            self.assertTrue(journal_status(journal)['reported_coordinate_coverage_complete'])
            for options in [{'positional':False},{'slots':('finger2',)}]:
                limited=await run(**options)
                self.assertEqual(len(limited[0]),1)
            self.assertEqual(len((await run(max_swaps=1))[0]),2)
            self.assertEqual(len((await run(max_swaps=2))[0]),2)
            self.assertEqual((base/f'{key}.json').read_bytes(),before)


if __name__=='__main__': unittest.main()
