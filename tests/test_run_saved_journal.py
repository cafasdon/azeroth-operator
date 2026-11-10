"""Rank across stable saved pages; incomplete work never becomes full-journal proof."""
import asyncio
from contextlib import redirect_stdout, redirect_stderr, closing
from copy import deepcopy
import hashlib
import io
import json
import os
from pathlib import Path
from time import monotonic
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from test_simulator_level import inputs
from tools.journal_forever_gear_windows import append_report, journal_status
from tools.rank_forever_slot import collect, plan_report
from tools import run_forever_saved_journal as runner
from tools import journal_forever_gear_windows as journal_tools


class SavedJournalTests(unittest.TestCase):
    def test_encounter_floor_validation_precedes_journal_access(self):
        for value in [True,False,'0',[],{},float('nan'),float('inf'),-float('inf'),10**400]:
            with patch.object(runner.journal,'journal_status') as read:
                with self.assertRaises(ValueError):
                    asyncio.run(runner.execute(SimpleNamespace(minimum_encounter_improvement=value)))
                read.assert_not_called()

    def test_evidence_preserves_native_fields_without_samples_or_metadata(self):
        for metric in runner.roles.FIELDS:
            row = {'iterations': 200, 'seed': '42', 'request_sha256': 'a'*64,
                runner.roles.FIELDS[metric]: 3, '_dps_samples': [99], 'private_name': 'PRIVATE'}
            if metric in ('dps','dtps','tps','hps'):
                row[f'{metric}_standard_deviation'] = 2
                row[f'paired_{metric}_standard_error'] = .5
                row[f'paired_{metric}_delta_interval_approx_95'] = {
                    'lower': -1, 'upper': 1, 'includes_zero': True}
            if metric == 'death':
                row['simulated_deaths'] = 3
                row['simulated_death_fraction_wilson_95'] = {'lower': .01, 'upper': .1}
            evidence = runner.encounter_evidence(row, {'request_sha256':'b'*64,
                runner.roles.FIELDS[metric]: .5, 'private_name':'PRIVATE', '_dps_samples':[99]}, metric, 2)
            self.assertEqual(evidence['metric'], metric)
            self.assertEqual(evidence['baseline_metric_value'], .5)
            for field, value in row.items():
                if field not in ('_dps_samples','private_name'):
                    self.assertEqual(evidence[field], value)
            self.assertNotIn('PRIVATE', json.dumps(evidence))
            self.assertNotIn('_dps_samples', evidence)
            self.assertEqual(evidence['baseline_request_sha256'], 'b'*64)
            self.assertEqual(evidence['encounter_index'], 2)
            self.assertFalse(evidence['selection_adjusted'])
            self.assertFalse(evidence['aggregate_ranking_uncertainty_verified'])
            self.assertEqual(evidence['comparison_basis'], 'equipped_baseline')
            self.assertEqual(evidence['interval_basis'], 'marginal_death_fraction' if metric == 'death'
                else 'not_available' if metric == 'oom' else 'candidate_minus_baseline')

    def test_objective_validation_precedes_journal_access(self):
        for value in [None,True,1,[],{},'unknown']:
            with patch.object(runner.journal,'journal_status') as read:
                with self.assertRaises(ValueError): asyncio.run(runner.execute(SimpleNamespace(ranking_objective=value)))
                read.assert_not_called()
        self.assertEqual(runner.ranking_objective(SimpleNamespace()),'weighted_gain')

    def test_real_module_empty_journal_and_invalid_options(self):
        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            journal = base/'PRIVATE_empty.sqlite'
            with closing(journal_tools.connect(journal,write=True)[0]) as connection:
                connection.commit()
            before = journal.read_bytes()
            args = [sys.executable,'-m','tools.run_forever_saved_journal','--journal',str(journal),
                '--mcp',str(base/'must-not-launch.exe'),'--engine',str(base/'must-not-launch.exe'),
                '--engine-sha256','a'*64,'--template',str(base/'missing.json'),
                '--encounter-lookup',str(base/'missing.json'),'--simulation-journal',str(base/'must-not-create')]
            proc = subprocess.run(args,cwd=root,capture_output=True,text=True,timeout=20)
            self.assertEqual(proc.returncode,0,proc.stderr)
            result = json.loads(proc.stdout)
            self.assertEqual(result['stop_reason'],'no_saved_work')
            self.assertFalse(result['all_saved_resolved_results_complete'])
            self.assertFalse(result['simulation_performed'])
            self.assertEqual(result['encounter_contexts'], [])
            self.assertEqual(result['unvisited_encounter_indices'], [1])
            self.assertEqual(result['ranked_gear_sets'],[])
            self.assertFalse((base/'must-not-create').exists())
            self.assertEqual(journal.read_bytes(),before)
            for option,value in [('--top','0'),('--max-pages','0'),('--page-limit','65'),('--iteration-budget','0'),
                ('--ranking-objective','unknown'),('--minimum-encounter-improvement','nan'),
                ('--minimum-encounter-improvement','inf')]:
                invalid = subprocess.run([*args,option,value],cwd=root,capture_output=True,text=True,timeout=20)
                self.assertEqual(invalid.returncode,2 if option=='--ranking-objective' else 1)
                self.assertEqual(invalid.stdout,'')
                self.assertNotIn('PRIVATE',invalid.stderr)

    def test_real_mcp_multi_page_shared_budgets_weighted_ranking_and_revalidation(self):
        root = Path(__file__).resolve().parents[1]
        catalogue = root / 'local-data/forever/catalogue.json'
        if not catalogue.is_file(): self.skipTest('Requires local pinned catalogue; no download')
        binary = (root / 'target/debug/azeroth-operator.exe').resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder, tempfile.TemporaryFile(mode='w+') as errlog:
            base = Path(folder)
            value = fixture()
            value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
            value['sections']['stats'] = {'level':60,'class':'HUNTER','race':'Orc','client_version':'x'}
            value['sections']['bag_capacity']={str(i):{'total_slots':16 if i==0 else 8 if i==1 else 0,'free_slots':0} for i in range(5)}
            value['sections']['bag_capacity']['1']['equipped_item']={'item_id':8217,'inventory_slot':20}
            value['sections']['bags'] = {'0': {'1': {
                'item_id':4549,'count':1,'link':'item:4549:0:0:0:0:0:0'}}}
            value['sections']['character_bank'] = {'6': {'1': {
                'item_id':4550,'count':1,'link':'item:4550:0:0:0:0:0:0'}}}
            key = SnapshotStore(base).ingest(value)
            env = {**os.environ,'AZEROTH_DATA_DIR':folder,'AZEROTH_FOREVER_CATALOGUE':str(catalogue.resolve())}
            journal = base / 'PRIVATE_gears.sqlite'
            for offset in range(3):
                with patch.dict(os.environ,env):
                    drafts,skipped,coverage,_,_,_,search = asyncio.run(collect(binary,key,['finger1'],errlog,
                        combination_offset=offset,combination_limit=1))
                page = plan_report(drafts,True,True)
                page.update(gear_search=search,skipped=skipped,storage_coverage=coverage,best_in_slot_verified=False)
                append_report(journal,page)
            before = journal.read_bytes()
            template,_,encounter = inputs()
            encounter['simulation_input']['simOptions']['iterations'] = 3
            second = deepcopy(encounter)
            second['simulation_input']['encounter']['duration'] = 90
            for name,data in [('template',template),('first',encounter),('second',second)]:
                (base/f'{name}.json').write_text(json.dumps(data))
            engine = base/'PRIVATE_engine.exe'
            engine.write_bytes(b'synthetic')
            argv = ['--mcp',str(binary),'--journal',str(journal),'--template',str(base/'template.json'),
                '--encounter-lookup',str(base/'first.json'),'--encounter-lookup',str(base/'second.json'),
                '--encounter-weight','1','--encounter-weight','3','--engine',str(engine),
                '--engine-sha256',hashlib.sha256(engine.read_bytes()).hexdigest(),
                '--simulation-journal',str(base/'results'),'--page-limit','2','--max-new-runs','1']
            def launch(extra=(),fails=False):
                out,err = io.StringIO(),io.StringIO()
                with patch.dict(os.environ,env),redirect_stdout(out),redirect_stderr(err):
                    if fails:
                        with self.assertRaises(SystemExit) as stopped: runner.main([*argv,*extra])
                        self.assertEqual(stopped.exception.code,1)
                        self.assertEqual(out.getvalue(),'')
                    else: runner.main([*argv,*extra])
                self.assertNotIn('PRIVATE',out.getvalue()+err.getvalue())
                self.assertEqual(journal.read_bytes(),before)
                return None if fails else json.loads(out.getvalue())
            def engine_stub(command,**kwargs):
                request = json.loads(Path(command[command.index('--infile')+1]).read_text())
                ids = {row.get('id') for row in request['raid']['parties'][0]['players'][0]['equipment']['items']}
                mean = 100
                if 4549 in ids: mean = 110 if request['encounter']['duration'] == 45 else 130
                if 4550 in ids: mean = 120 if request['encounter']['duration'] == 45 else 90
                Path(command[command.index('--outfile')+1]).write_text(json.dumps({
                    'iterationsDone':3,'raidMetrics':{'dps':{'avg':mean,'stdev':1}}}))
            with patch('tools.run_forever_sim.subprocess.run',side_effect=engine_stub) as calls:
                for index in range(6):
                    result = launch()
                    self.assertEqual((result['new_runs'],result['new_iterations']),(1,3))
                    self.assertEqual(result['all_saved_resolved_results_complete'],index == 5)
                self.assertEqual(calls.call_count,6)
                original_argv=argv[:]
                weight_indices=[i+1 for i,value in enumerate(argv) if value=='--encounter-weight']
                argv[weight_indices[0]]='9'; argv[weight_indices[1]]='1'
                average=launch(['--checkpoint-only','--top','1'])
                robust=launch(['--checkpoint-only','--top','1','--ranking-objective','worst_encounter_gain'])
                protected=launch(['--checkpoint-only','--top','1','--minimum-encounter-improvement','0'])
                boundary=launch(['--checkpoint-only','--minimum-encounter-improvement','10'])
                none=launch(['--checkpoint-only','--minimum-encounter-improvement','31'])
                self.assertEqual(protected['ranked_gear_sets'][0]['saved_loadout']['selections'][0]['expected_item_id'],4549)
                self.assertEqual(len(boundary['ranked_gear_sets']),1)
                self.assertEqual(boundary['minimum_encounter_improvement'],10)
                self.assertEqual(none['ranked_gear_sets'],[])
                for constrained in [protected,boundary,none]:
                    self.assertEqual(constrained['new_runs'],0)
                    self.assertTrue(constrained['all_saved_resolved_results_complete'])
                    self.assertFalse(constrained['best_in_slot_verified'])
                    self.assertGreater(constrained['encounter_floor_excluded_candidate_instances'],0)
                self.assertEqual(average['ranking_objective'],'weighted_gain')
                self.assertEqual(robust['ranking_objective'],'worst_encounter_gain')
                self.assertEqual(average['ranked_gear_sets'][0]['saved_loadout']['selections'][0]['expected_item_id'],4550)
                best=robust['ranked_gear_sets'][0]
                self.assertEqual(best['saved_loadout']['selections'][0]['expected_item_id'],4549)
                self.assertEqual((best['minimum_dps_gain_pct'],best['maximum_dps_gain_pct'],best['ranking_score']),(10,30,10))
                self.assertAlmostEqual(best['weighted_dps_gain_pct'],12)
                for ranked in [average,robust]:
                    self.assertEqual(ranked['new_runs'],0)
                    self.assertTrue(ranked['all_saved_resolved_results_complete'])
                    self.assertFalse(ranked['best_in_slot_verified'])
                self.assertEqual(calls.call_count,6)
                argv[:]=original_argv
                self.assertEqual(result['ranking_scope'],'all_saved_resolved_requests')
                self.assertEqual(result['completed_window_count'],3)
                contexts = result['encounter_contexts']
                self.assertEqual([context['encounter_index'] for context in contexts], [1,2])
                self.assertEqual([context['weight'] for context in contexts], [.25,.75])
                self.assertEqual([context['encounter_settings']['duration_seconds'] for context in contexts], [45,90])
                self.assertEqual([context['baseline']['mean_dps'] for context in contexts], [100,100])
                for context in contexts:
                    observation=context['hunter_quiver_observation']
                    self.assertEqual(observation['observed_pinned_quiver_item_ids'],[8217])
                    self.assertEqual(observation['comparison'],'none_selection_unverified')
                    self.assertIs(observation['template_haste_below_observed_reference'],True)
                    self.assertIs(observation['quiver_bonus_verified'],False)
                    self.assertEqual(observation['template_ammo_reference']['item_id'],0)
                    self.assertEqual(observation['template_ammo_reference']['bonus_ranged_dps'],0)
                    self.assertIs(observation['template_ammo_reference']['identity_verified'],False)
                    pet=observation['template_pet_reference']
                    self.assertEqual(pet['attack_speed'],'PetAttackSpeedNone')
                    self.assertIs(pet['native_effect_verified'],False)
                    self.assertIs(pet['identity_verified'],False)
                self.assertEqual(result['unvisited_encounter_indices'], [])
                self.assertEqual(len(result['ranked_gear_sets']),3)
                for row,expected in zip(result['ranked_gear_sets'],[25,0,-2.5],strict=True):
                    self.assertAlmostEqual(row['weighted_dps_gain_pct'],expected)
                self.assertEqual([row['source_window_offset_decimal'] for row in result['ranked_gear_sets']],['1','0','2'])
                self.assertEqual(len({row['simulator_gear_sha256'] for row in result['ranked_gear_sets']}),3)
                winner=result['ranked_gear_sets'][0]
                evidence=winner['simulation_evidence_by_encounter']
                self.assertEqual([row['encounter_index'] for row in evidence],[1,2])
                self.assertEqual([row['mean_dps'] for row in evidence],[110,130])
                self.assertEqual([row['baseline_metric_value'] for row in evidence], [100,100])
                self.assertEqual([row['baseline_request_sha256'] for row in evidence],
                    [context['baseline']['request_sha256'] for context in contexts])
                self.assertTrue(all(row['iterations']==3 and row['dps_standard_deviation']==1 for row in evidence))
                self.assertTrue(all('paired_dps_delta_interval_approx_95' not in row for row in evidence))
                saved=winner['saved_loadout']
                self.assertEqual(saved['snapshot_id'],key)
                self.assertEqual(saved['basis'],'validated_saved_plan')
                self.assertEqual(saved['selections'],[{'target_slot':11,
                    'source':{'kind':'bag','bag':0,'slot':1},'expected_item_id':4549}])
                self.assertEqual(saved['bank_retrievals'], [])
                self.assertEqual(result['ranked_gear_sets'][2]['saved_loadout']['bank_retrievals'], [
                    {'target_slot':11,'source':{'kind':'character_bank','container':6,'slot':1},
                     'expected_item_id':4550}])
                self.assertEqual(saved['simulator_equipment_spec']['items'][10]['id'],4549)
                self.assertEqual(saved['cleared_equipment_slots'],[])
                self.assertEqual(hashlib.sha256(journal_tools.encoded(saved['simulator_equipment_spec'])).hexdigest(),
                    winner['simulator_gear_sha256'])
                self.assertEqual(result['ranked_gear_sets'][1]['saved_loadout']['selections'],[])
                for flag in ['current_locations_verified','native_equip_eligibility_verified','game_execution']:
                    self.assertFalse(saved[flag])
                for field,bad in [('source_window_offset_decimal','999'),('source_window_offset_decimal','01'),
                    ('source_window_review',999),('source_window_review',True),('simulator_gear_sha256','0'*64)]:
                    broken=deepcopy(winner); broken[field]=bad
                    with self.assertRaises(ValueError): runner.attach_saved_loadouts(journal,
                        {winner['simulator_gear_sha256']:broken},result['journal_coverage'],monotonic()+10)
                for field in ['snapshot_id','scope_sha256','collection_binding_sha256']:
                    badsource=deepcopy(result['journal_coverage']); badsource[field]='0'*64
                    with self.assertRaises(ValueError): runner.attach_saved_loadouts(journal,
                        {winner['simulator_gear_sha256']:deepcopy(winner)},badsource,monotonic()+10)
                self.assertEqual(journal.read_bytes(),before)
                cached = launch(['--checkpoint-only'])
                self.assertEqual((cached['new_runs'],cached['checkpoint_hits']),(0,8))
                self.assertEqual(result['ranked_gear_sets'],cached['ranked_gear_sets'])
                self.assertEqual(result['encounter_contexts'],cached['encounter_contexts'])
                # Real cached requests, but one alternative lacks its second encounter.
                # Retain the fully compared alternative without treating this page as done.
                from tools.simulation_journal import SimulationJournal
                original_key, original_get = SimulationJournal.key, SimulationJournal.get
                hidden = set()
                def request_key(instance, engine, request, *flags):
                    key = original_key(instance, engine, request, *flags)
                    ids = {item.get('id') for item in request['raid']['parties'][0]['players'][0]['equipment']['items']}
                    if 4550 in ids and request['encounter']['duration'] == 90:
                        hidden.add(key)
                    return key
                def available(instance, key):
                    return None if key in hidden else original_get(instance, key)
                with patch.object(SimulationJournal, 'key', request_key), patch.object(SimulationJournal, 'get', available):
                    subset = launch(['--checkpoint-only','--page-limit','3'])
                self.assertEqual(subset['ranking_scope'],'completed_candidates_across_all_requested_encounters')
                self.assertEqual(subset['incomplete_page_completed_candidate_count'],2)
                self.assertFalse(subset['all_saved_resolved_results_complete'])
                self.assertEqual(subset['completed_window_count'],0)
                self.assertEqual(subset['stop_reason'],'incomplete_page')
                self.assertAlmostEqual(subset['ranked_gear_sets'][0]['weighted_dps_gain_pct'],25)
                self.assertEqual(subset['ranked_gear_sets'][1]['weighted_dps_gain_pct'],0)
                self.assertEqual(subset['ranked_gear_sets'][0]['saved_loadout']['selections'][0]['expected_item_id'],4549)
                self.assertEqual(subset['ranked_gear_sets'][0]['simulation_evidence_by_encounter'][1]['mean_dps'],130)
                self.assertEqual(subset['new_runs'],0)
                narrow = launch(['--checkpoint-only','--page-limit','1','--top','1'])
                self.assertEqual(narrow['ranked_gear_sets'],cached['ranked_gear_sets'][:1])
                self.assertEqual(narrow['encounter_contexts'],cached['encounter_contexts'])
                self.assertTrue(all(context['baseline']['mean_dps'] == 100 for context in narrow['encounter_contexts']))
                partial = launch(['--checkpoint-only','--max-pages','1'])
                self.assertFalse(partial['all_saved_resolved_results_complete'])
                self.assertEqual(partial['ranking_scope'],'completed_pages_only')
                self.assertEqual(partial['stop_reason'],'page_budget')
                self.assertEqual(calls.call_count,6)
                # Detect valid source changes even if aggregate coverage metadata stays unchanged.
                count = {'value':0}
                def changed_status(*args,**kwargs):
                    current = journal_status(*args,**kwargs)
                    count['value'] += 1
                    if count['value'] == 2: current['journal_content_sha256'] = '0'*64
                    return current
                with patch.object(runner.journal,'journal_status',side_effect=changed_status):
                    launch(['--checkpoint-only'],fails=True)
                original = runner.pages.execute
                async def changed_profile(options,*,deadline=None):
                    if options.after_window_offset is not None:
                        different = deepcopy(template)
                        different['raid']['parties'][0]['players'][0]['name'] = 'PRIVATE_CHANGED'
                        (base/'template.json').write_text(json.dumps(different))
                    return await original(options,deadline=deadline)
                with patch.object(runner.pages,'execute',changed_profile):
                    launch(['--checkpoint-only'],fails=True)
                self.assertEqual(calls.call_count,6)
            for flag in ['global_ranking_verified','best_in_slot_verified','feasible_set_coverage_verified','game_execution']:
                self.assertFalse(result[flag])
        proc = subprocess.run([sys.executable,'-m','tools.run_forever_saved_journal','--help'],cwd=root,
            capture_output=True,text=True,timeout=20)
        self.assertEqual(proc.returncode,0,proc.stderr)


if __name__ == '__main__': unittest.main()
