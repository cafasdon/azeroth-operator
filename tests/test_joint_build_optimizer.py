"""Complete-matrix oracle, fail-before-execution checks, and real CLI/MCP callers."""
import asyncio
from copy import deepcopy
import itertools
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from tools import joint_build_optimizer as joint
from tools import build_search as planner

ROOT = Path(__file__).resolve().parents[1]


def player(request):
    return request['raid']['parties'][0]['players'][0]


def manifest():
    request = dict(type='SimTypeIndividual', simOptions=dict(iterations=2, randomSeed='1'),
                   encounter=dict(duration=30, targets=[{}]),
                   raid=dict(parties=[dict(players=[dict(name='PRIVATE-NAME', race='RaceHuman',
                       **{'class': 'ClassWarrior'}, dpsWarrior={}, talentsString='0',
                       rotation=dict(type='TypeAPL', priorityList=[dict(action=dict(startAttack={}))]),
                       equipment=dict(items=[{} for _ in range(17)], preserveSlots=True))])]))
    value = dict(case=dict(client=dict(family='forever', variant='classic', build='1.15.7', interface=11507),
                          snapshot_id='a'*64, constraints_sha256='b'*64, character_level=60,
                          class_id=1, specialization='dpsWarrior', objective='dps',
                          engine_sha256=joint.layout.ENGINE_SHA256, data_manifest_sha256=joint.CATALOGUE_SHA256,
                          encounter_sha256=planner.digest(request['encounter'])),
                 axes=[], references={}, base_request=request)
    add_axis(value, 'gear', [dict(items=[dict(id=identifier)] + [{} for _ in range(16)], preserveSlots=True)
                             for identifier in (100, 101)])
    add_axis(value, 'talents', ['0', '1'])
    return value


def add_axis(value, category, choices):
    axis = dict(name=category, category=category, candidates=[])
    for choice in choices:
        entry = dict(category=category, value=choice)
        ref = planner.digest(entry)
        value['references'][ref] = entry
        axis['candidates'].append(ref)
    value['axes'].append(axis)


def replace_reference(value, axis_index, choice_index, replacement):
    axis = value['axes'][axis_index]
    old = axis['candidates'][choice_index]
    entry = dict(category=axis['category'], value=replacement)
    new = planner.digest(entry)
    del value['references'][old]
    value['references'][new] = entry
    axis['candidates'][choice_index] = new


def summaries(engine, engine_hash, requests, cache=None):
    rows = []
    for request in requests:
        p = player(request)
        # Independent interaction trap: either isolated change loses; both win.
        score = {(100, '0'): 5, (101, '0'): 4, (100, '1'): 4, (101, '1'): 10}[
            p['equipment']['items'][0]['id'], p['talentsString']]
        rows.append(dict(mean_dps=score, dps_standard_deviation=2,
                         **request['simOptions'] | {'seed': request['simOptions']['randomSeed']},
                         engine_sha256=engine_hash))
    return dict(results=rows)


class JointTests(unittest.TestCase):
    def evaluate(self, value):
        with patch.object(joint.adapter, 'verify_engine'), patch.object(joint.adapter, 'compare', side_effect=summaries) as run:
            result = joint.evaluate_joint_build_case(value, ROOT / 'unused-engine.exe')
        return result, run

    def test_independent_cartesian_oracle_and_interaction_trap(self):
        value = manifest()
        original = deepcopy(value)
        result, run = self.evaluate(value)
        expected = list(itertools.product(*(a['candidates'] for a in value['axes'])))
        declared = sorted(result['results'], key=lambda row: int(row['ordinal_decimal']))
        self.assertEqual([tuple(row['selection'].values()) for row in declared], expected)
        self.assertEqual([row['mean_dps'] for row in result['results']], [10, 5, 4, 4])
        self.assertEqual([row['ordinal_decimal'] for row in result['results']], ['3', '0', '1', '2'])
        self.assertEqual([row['rank'] for row in result['results']], [1, 2, 3, 3])
        self.assertEqual([row['tied_observed_mean'] for row in result['results']], [False, False, True, True])
        self.assertEqual(result['evaluated_coordinates'], 4)
        self.assertEqual(result['unique_native_iterations'], 8)
        self.assertEqual(result['resolved_coordinates'], result['declared_coordinates'])
        self.assertEqual(len(run.call_args.args[2]), 4)
        self.assertEqual(value, original)
        self.assertNotIn('PRIVATE-NAME', planner.encoded(result))
        for flag in ('global_optimum_verified', 'active_character_verified', 'candidate_legality_verified',
                     'statistical_superiority_verified', 'snapshot_reference_verified', 'constraints_verified',
                     'all_owned_items_covered', 'game_execution'):
            self.assertIs(result[flag], False)
        for row in declared:
            binding = dict(search_id=result['search_id'], ordinal_decimal=row['ordinal_decimal'], selection=row['selection'])
            self.assertEqual(row['candidate_sha256'], planner.digest(binding))

    def test_all_five_dimensions_are_applied_and_schema_equivalence_deduplicates(self):
        value = manifest()
        add_axis(value, 'rotation', [dict(type='TypeAPL', priorityList=[dict(action=dict(startAttack={}))]),
                                     dict(type=3, priorityList=[dict(action=dict(startAttack={}))])])
        add_axis(value, 'timing', [{}, dict(hpPercentForDefensives=0)])
        add_axis(value, 'consumables', [{}, dict(foodId=0)])
        # Read the enum number from the pinned schema instead of assuming its value.
        enum = joint.request_type().DESCRIPTOR.file.pool.FindEnumTypeByName('proto.APLRotation.Type')
        replace_reference(value, 2, 1, dict(type=enum.values_by_name['TypeAPL'].number,
                                           priorityList=[dict(action=dict(startAttack={}))]))
        result, run = self.evaluate(value)
        self.assertEqual(result['declared_coordinates'], 32)
        self.assertEqual(result['unique_requests'], 4)
        self.assertEqual(result['equivalent_coordinates'], 28)
        self.assertEqual(len(run.call_args.args[2]), 4)
        self.assertTrue(all(set(player(r)) >= {'equipment', 'talentsString', 'rotation', 'cooldowns', 'consumables'}
                            for r in run.call_args.args[2]))

    def assert_no_run(self, value):
        with patch.object(joint.adapter, 'compare') as run, patch.object(joint.adapter, 'verify_engine') as verify:
            with self.assertRaises((ValueError, TypeError, KeyError, joint.json_format.ParseError)):
                joint.evaluate_joint_build_case(value, ROOT / 'unused-engine.exe')
        run.assert_not_called()
        verify.assert_not_called()

    def test_reference_case_and_entire_matrix_preflight_before_execution(self):
        for field, changed in [('character_level', 59), ('class_id', 8), ('specialization', 'mage'),
                               ('objective', 'hps'), ('engine_sha256', 'a'*64),
                               ('data_manifest_sha256', 'a'*64), ('encounter_sha256', 'a'*64)]:
            value = manifest()
            value['case'][field] = changed
            with self.subTest(field=field):
                self.assert_no_run(value)
        value = manifest()
        value['references'][next(iter(value['references']))]['value']['items'][0]['id'] = 999
        self.assert_no_run(value)
        for edit in ('missing', 'unused', 'category', 'same_category', 'last_invalid'):
            value = manifest()
            ref = next(iter(value['references']))
            if edit == 'missing': del value['references'][ref]
            if edit == 'unused': value['references']['f'*64] = value['references'][ref]
            if edit == 'category': value['references'][ref]['category'] = 'rotation'
            if edit == 'same_category': value['axes'][1]['category'] = 'gear'
            if edit == 'last_invalid': replace_reference(value, 1, 1, {'invalid': 'talents'})
            with self.subTest(edit=edit): self.assert_no_run(value)

    def test_strict_native_types_aliases_unknown_fields_and_actionability(self):
        mutations = [lambda r: r.update(unknown=True),
                     lambda r: r['simOptions'].update(iterations=True),
                     lambda r: r['simOptions'].update(iterations='2'),
                     lambda r: r['simOptions'].update(randomSeed='01'),
                     lambda r: r['simOptions'].update(randomSeed=1),
                     lambda r: r['simOptions'].update(random_seed='1'),
                     lambda r: player(r).update(talents_string='1'),
                     lambda r: player(r).update(mage={}),
                     lambda r: player(r).update(database={'items': [{}]}),
                     lambda r: player(r).update(rotation={'type': 'TypeAPL', 'priorityList': [{'action': {}}]}),
                     lambda r: player(r).update(rotation={'type': 'TypeAPL', 'priorityList': [{'hide': True, 'action': {'startAttack': {}}}]}),
                     lambda r: player(r)['equipment'].update(preserveSlots=False),
                     lambda r: player(r)['equipment']['items'].pop(),
                     lambda r: r['raid']['parties'][0]['players'].append(deepcopy(player(r))),
                     lambda r: r['encounter'].update(duration=True),
                     lambda r: r['encounter'].update(duration=float('inf'))]
        for index, change in enumerate(mutations):
            value = manifest()
            change(value['base_request'])
            value['case']['encounter_sha256'] = planner.digest(value['base_request']['encounter']) if index < 15 else value['case']['encounter_sha256']
            with self.subTest(index=index): self.assert_no_run(value)

    def test_matrix_and_iteration_budgets_and_base_request_identity(self):
        value = manifest()
        identity = joint.preflight(value)[0]['search_id']
        for mutate in (lambda v: v['base_request']['simOptions'].update(iterations=3),
                       lambda v: v['base_request']['simOptions'].update(randomSeed='2'),
                       lambda v: player(v['base_request']).update(reactionTimeMs=99)):
            drift = deepcopy(value)
            mutate(drift)
            self.assertNotEqual(identity, joint.preflight(drift)[0]['search_id'])
            with self.assertRaises(ValueError): joint.preflight(drift, identity)
        value['base_request']['simOptions']['iterations'] = 5000
        add_axis(value, 'timing', [dict(hpPercentForDefensives=i/10) for i in range(6)])
        self.assert_no_run(value)  # 24 unique requests, 120000 native iterations.
        value['base_request']['simOptions']['iterations'] = 2
        add_axis(value, 'consumables', [dict(foodId=i) for i in range(3)])
        self.assert_no_run(value)  # 72 declared coordinates.
        with patch.object(joint, 'DESCRIPTOR_SHA256', '0'*64): self.assert_no_run(manifest())
        with patch.object(joint.planner, 'MAX_BYTES', 1): self.assert_no_run(manifest())

    def test_bad_native_results_never_report_success(self):
        for bad in (True, '1', -1, float('nan'), float('inf')):
            for field in ('mean_dps', 'dps_standard_deviation'):
                def compare(*args, **kwargs):
                    result = summaries(*args, **kwargs)
                    result['results'][-1][field] = bad
                    return result
                with patch.object(joint.adapter, 'verify_engine'), patch.object(joint.adapter, 'compare', side_effect=compare):
                    with self.subTest(bad=bad, field=field), self.assertRaises(ValueError):
                        joint.evaluate_joint_build_case(manifest(), ROOT/'unused.exe')

    def test_inactive_player_or_invalid_active_party_count_rejects_before_execution(self):
        value = manifest()
        raid = value['base_request']['raid']
        raid['parties'] = [{}, raid['parties'][0]]
        raid['numActiveParties'] = 1
        self.assert_no_run(value)
        for active in (-1, 2):
            value = manifest()
            value['base_request']['raid']['numActiveParties'] = active
            self.assert_no_run(value)
        for active in (0, 1):
            value = manifest()
            value['base_request']['raid']['numActiveParties'] = active
            self.assertEqual(len(joint.preflight(value)[1]), 4)

    def test_actual_cli_and_mcp_reject_overrides_without_engine_execution(self):
        with tempfile.TemporaryDirectory(prefix='joint-test-') as folder:
            path = Path(folder)/'case.json'
            path.write_text(planner.encoded(manifest()))
            prepare = subprocess.run([sys.executable, '-m', 'tools.joint_build_optimizer', 'prepare', '--input', str(path)],
                                     cwd=ROOT, capture_output=True, text=True, timeout=30)
            self.assertEqual(prepare.returncode, 0, prepare.stderr)
            expected = json.loads(prepare.stdout)['search_id']
            async def probe():
                params = StdioServerParameters(command=sys.executable, cwd=str(ROOT),
                    args=['-m', 'tools.joint_build_optimizer', 'serve', '--input', str(path), '--engine', str(ROOT/'missing-engine.exe')])
                async with stdio_client(params) as (read, write), ClientSession(read, write) as session:
                    await session.initialize()
                    listing = await session.list_tools()
                    self.assertEqual([t.name for t in listing.tools], ['evaluate_joint_build_case'])
                    self.assertIn(expected, listing.tools[0].description)
                    for arguments in ({}, {'expected_search_id':'a'*64}, {'expected_search_id':expected, 'request':{}},
                                      {'expected_search_id':expected, 'engine':'PRIVATE-PATH'}, {'expected_search_id':expected}):
                        response = await session.call_tool('evaluate_joint_build_case', arguments)
                        self.assertTrue(response.isError)
                        self.assertIn(joint.ERROR, response.content[0].text)
                        self.assertNotIn('PRIVATE', response.content[0].text)
            asyncio.run(asyncio.wait_for(probe(), 30))
            for body in ('{"case":{},"case":{}}', 'x'*(planner.MAX_INPUT_BYTES+1)):
                path.write_text(body)
                run = subprocess.run([sys.executable, '-m', 'tools.joint_build_optimizer', 'run', '--input', str(path),
                                      '--engine', str(ROOT/'missing-engine.exe')], cwd=ROOT,
                                     capture_output=True, text=True, timeout=30)
                self.assertEqual(run.returncode, 1)
                self.assertEqual(run.stderr.strip(), joint.ERROR)
                self.assertEqual(run.stdout, '')

    @unittest.skipUnless(os.environ.get('AZEROTH_JOINT_ENGINE'), 'Explicit pinned local native engine required')
    def test_opt_in_real_native_cli_and_mcp(self):
        engine = Path(os.environ['AZEROTH_JOINT_ENGINE']).resolve(strict=True)
        source = Path(os.environ.get('AZEROTH_JOINT_FIXTURE', ROOT/'tests/fixtures/forever-hunter-bm.json'))
        fixture = json.loads(source.read_text())
        request = (fixture if 'raid' in fixture else dict(
            raid=dict(parties=[dict(players=[fixture['player']])], buffs=fixture['raidBuffs'], debuffs=fixture['debuffs']),
            encounter=fixture['encounter']))
        request['simOptions'] = dict(iterations=2, randomSeed='20261115')
        request['type'] = 'SimTypeIndividual'
        player(request)['equipment']['preserveSlots'] = True
        value = manifest()
        value.update(base_request=request, axes=[], references={})
        value['case'].update(class_id=3, specialization='hunter', encounter_sha256=planner.digest(request['encounter']))
        add_axis(value, 'timing', [{}, dict(hpPercentForDefensives=0)])
        with tempfile.TemporaryDirectory(prefix='joint-native-') as folder:
            path = Path(folder)/'case.json'
            path.write_text(planner.encoded(value))
            result = subprocess.run([sys.executable, '-m', 'tools.joint_build_optimizer', 'run', '--input', str(path),
                                     '--engine', str(engine)], cwd=ROOT, capture_output=True, text=True, timeout=130)
            self.assertEqual(result.returncode, 0, result.stderr)
            actual = json.loads(result.stdout)
            self.assertEqual(actual['unique_requests'], 1)
            self.assertEqual(actual['evaluated_coordinates'], 2)
            self.assertGreater(actual['results'][0]['mean_dps'], 0)
            async def probe():
                params = StdioServerParameters(command=sys.executable, cwd=str(ROOT),
                    args=['-m', 'tools.joint_build_optimizer', 'serve', '--input', str(path), '--engine', str(engine)])
                async with stdio_client(params) as (read, write), ClientSession(read, write) as session:
                    await session.initialize()
                    response = await session.call_tool('evaluate_joint_build_case', {'expected_search_id':actual['search_id']})
                    self.assertFalse(response.isError)
                    self.assertEqual(json.loads(response.content[0].text), actual)
                    concurrent = await asyncio.gather(*(session.call_tool('evaluate_joint_build_case',
                        {'expected_search_id':actual['search_id']}) for _ in range(2)))
                    self.assertEqual(sum(not response.isError for response in concurrent), 1)
                    failure = next(response for response in concurrent if response.isError)
                    self.assertIn(joint.ERROR, failure.content[0].text)
            asyncio.run(asyncio.wait_for(probe(), 130))


if __name__ == '__main__':
    unittest.main()
