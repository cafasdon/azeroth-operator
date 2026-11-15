"""Synthetic planning coverage and real offline caller checks; no simulator claims."""
import asyncio
from copy import deepcopy
import hashlib
import itertools
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from jsonschema import Draft202012Validator
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from tools.build_search import (MAX_BYTES, MAX_INPUT_BYTES, HASH_KEYS, encoded, input_schema,
                                prepare_build_search_window, prepare_request)

ROOT = Path(__file__).resolve().parents[1]


def ref(value):
    return hashlib.sha256(str(value).encode()).hexdigest()


def request():
    return dict(case=dict(client=dict(family='forever', variant='classic', build='1.15.7', interface=11507),
                          **{key: ref(key) for key in HASH_KEYS}, character_level=14, class_id=1,
                          specialization='arms', objective='dps'),
                axes=[dict(name='weapon', category='gear', candidates=[ref('w0'), ref('w1')]),
                      dict(name='talents', category='talents', candidates=[ref('t0'), ref('t1'), ref('t2')]),
                      dict(name='rotation', category='rotation', candidates=[ref('r0'), ref('r1')])])


class SearchTests(unittest.TestCase):
    def test_all_small_coordinates_and_window_independent_identity(self):
        source = request()
        original = deepcopy(source)
        whole = prepare_build_search_window(**source)
        expected = list(itertools.product(*(axis['candidates'] for axis in source['axes'])))
        self.assertEqual([tuple(c['selection'].values()) for c in whole['candidates']], expected)
        self.assertEqual(whole['total_decimal'], '12')
        for size in (1, 2, 3, 5, 12, 64):
            collected, offset = [], '0'
            while offset is not None:
                window = prepare_request(source | dict(offset=offset, limit=size, expected_search_id=whole['search_id']))
                collected.extend(window['candidates'])
                offset = window['next_offset']
            self.assertEqual(collected, whole['candidates'])
        self.assertEqual(len({c['candidate_sha256'] for c in whole['candidates']}), 12)
        for candidate in whole['candidates']:
            binding = dict(search_id=whole['search_id'], ordinal_decimal=candidate['ordinal_decimal'],
                           selection=candidate['selection'])
            canonical = json.dumps(binding, sort_keys=True, separators=(',', ':')).encode('ascii')
            self.assertEqual(candidate['candidate_sha256'], hashlib.sha256(canonical).hexdigest())
        self.assertEqual([c['ordinal_decimal'] for c in whole['candidates']], list(map(str, range(12))))
        self.assertEqual(source, original)
        self.assertEqual(whole['case'], source['case'])
        self.assertEqual(whole['objective_direction'], 'maximize')
        self.assertTrue(whole['plan_only'])
        for flag in ('simulation_performed', 'objective_evaluated', 'candidate_legality_verified',
                     'model_compatibility_verified', 'source_data_complete_verified', 'all_owned_items_covered',
                     'global_optimum_verified', 'constraints_verified', 'snapshot_reference_verified',
                     'active_character_verified'):
            self.assertIs(whole[flag], False)
        self.assertIn('not proof', whole['coverage'])
        self.assertNotIn('candidates', whole['axes'][0])
        detached = prepare_request(source)
        detached['case']['client']['build'] = 'changed'
        self.assertEqual(source, original)

    def test_toy_interaction_winner_is_in_declared_matrix_only(self):
        # Toy arithmetic, not a game model: independent single-axis improvement misses (1, 1).
        value = request()
        value['axes'] = [dict(name=name, category=category, candidates=[ref(0), ref(1)])
                         for name, category in [('gear', 'gear'), ('talents', 'talents')]]
        planned = prepare_request(value)
        scores = {(ref(0), ref(0)): 5, (ref(0), ref(1)): 4, (ref(1), ref(0)): 4, (ref(1), ref(1)): 10}
        winner = max(planned['candidates'], key=lambda c: scores[tuple(c['selection'].values())])
        self.assertEqual(winner['ordinal_decimal'], '3')
        self.assertIs(planned['objective_evaluated'], False)
        self.assertNotIn('score', winner)

    def test_resume_binds_every_case_field_axis_and_order(self):
        source = request()
        identity = prepare_request(source)['search_id']
        variants = []
        for key in HASH_KEYS:
            value = deepcopy(source)
            value['case'][key] = ref('changed')
            variants.append(value)
        for key, change in [('character_level', 60), ('class_id', 2), ('specialization', 'fury'), ('objective', 'dtps')]:
            value = deepcopy(source)
            value['case'][key] = change
            variants.append(value)
        for key, change in [('variant', 'era'), ('build', '1.15.8'), ('interface', 11508)]:
            value = deepcopy(source)
            value['case']['client'][key] = change
            variants.append(value)
        for change in ('axis_order', 'value_order', 'name', 'category', 'reference'):
            value = deepcopy(source)
            if change == 'axis_order':
                value['axes'].reverse()
            elif change == 'value_order':
                value['axes'][0]['candidates'].reverse()
            elif change == 'reference':
                value['axes'][0]['candidates'][0] = ref('other')
            else:
                value['axes'][0][change] = 'trinket' if change == 'name' else 'consumables'
            variants.append(value)
        for value in variants:
            with self.subTest(value=value):
                self.assertNotEqual(prepare_request(value)['search_id'], identity)
                with self.assertRaisesRegex(ValueError, 'resume rejected'):
                    prepare_request(value | dict(offset='1', expected_search_id=identity))
        reordered = {k: source[k] for k in reversed(source)}
        reordered['case'] = {k: source['case'][k] for k in reversed(source['case'])}
        self.assertEqual(prepare_request(reordered)['search_id'], identity)
        for objective in ('dtps', 'death', 'oom'):
            value = deepcopy(source)
            value['case']['objective'] = objective
            self.assertEqual(prepare_request(value)['objective_direction'], 'minimize')

    def test_huge_matrix_last_window_bounded_and_no_effects(self):
        value = request()
        value['axes'] = [dict(name=f'axis_{i}', category='gear', candidates=[ref(n) for n in range(128)])
                         for i in range(32)]
        total = 128 ** 32
        before = time.monotonic()
        with patch('builtins.open', side_effect=AssertionError('No files')), \
                patch('socket.socket', side_effect=AssertionError('No network')), \
                patch('subprocess.Popen', side_effect=AssertionError('No engine')):
            result = prepare_request(value | dict(offset=str(total - 2), limit=64))
        self.assertLess(time.monotonic() - before, 3)
        self.assertEqual(result['total_decimal'], str(2 ** 224))
        self.assertEqual(result['window_count'], 2)
        self.assertEqual(result['end_decimal'], str(total))
        self.assertIsNone(result['next_offset'])
        self.assertEqual(set(result['candidates'][-1]['selection'].values()), {ref(127)})
        self.assertLess(len(encoded({'content': [{'type': 'text', 'text': encoded(result)}], 'isError': False}).encode()), MAX_BYTES)
        for axis in value['axes']:
            axis['name'] = axis['name'].ljust(64, '_')
        self.assertEqual(prepare_request(value)['window_count'], 16)
        with self.assertRaisesRegex(ValueError, 'transport budget'):
            prepare_request(value | dict(limit=64))

    def test_terminal_offsets_and_strict_scalars(self):
        value = request()
        terminal = prepare_request(value | dict(offset='12'))
        self.assertEqual(terminal['candidates'], [])
        self.assertEqual(terminal['start_decimal'], terminal['end_decimal'])
        self.assertIsNone(terminal['next_offset'])
        for offset in ('13', '9' * 80, '9' * 81, '', '00', '01', '-1', '+1', ' 1', '1.0', '1\n', 0, True, 1.0, None):
            with self.subTest(offset=offset), self.assertRaises(ValueError):
                prepare_request(value | dict(offset=offset))
        for limit in (0, -1, 65, True, 1.0, '1', None):
            with self.subTest(limit=limit), self.assertRaises(ValueError):
                prepare_request(value | dict(limit=limit))
        for key, maximum in [('character_level', 255), ('class_id', 255), ('interface', 4294967295)]:
            for invalid in (0, -1, maximum + 1, True, 1.0, '1', None):
                bad = deepcopy(value)
                target = bad['case']['client'] if key == 'interface' else bad['case']
                target[key] = invalid
                with self.subTest(key=key, invalid=invalid), self.assertRaises(ValueError):
                    prepare_request(bad)
        for level in (1, 14, 60, 255):
            value['case']['character_level'] = level
            self.assertFalse(prepare_request(value)['model_compatibility_verified'])

    def test_hashes_nested_fields_and_container_guards(self):
        invalids = [None, {}, [], True, '', 'a' * 63, 'a' * 65, 'A' * 64, 'g' * 64, '\n' + 'a' * 63]
        for invalid in invalids:
            for key in HASH_KEYS:
                value = request()
                value['case'][key] = invalid
                with self.subTest(key=key, invalid=invalid), self.assertRaises(ValueError):
                    prepare_request(value)
            value = request()
            value['axes'][0]['candidates'][0] = invalid
            with self.assertRaises(ValueError):
                prepare_request(value)
            if invalid is not None:
                with self.assertRaises(ValueError):
                    prepare_request(request() | dict(expected_search_id=invalid))
        for path in ((), ('case',), ('case', 'client'), ('axes', 0)):
            for remove in (False, True):
                value = request()
                target = value
                for key in path:
                    target = target[key]
                if remove:
                    del target[next(iter(target))]
                else:
                    target['extra'] = 'denied'
                with self.subTest(path=path, remove=remove), self.assertRaises(ValueError):
                    prepare_request(value)
        for axes in (None, (), [], [request()['axes'][0]] * 33, [None], [request()['axes'][0]] * 2):
            with self.assertRaises(ValueError):
                prepare_request(request() | dict(axes=axes))
        for pool in (None, (), [], [ref(0)] * 2, [ref(i) for i in range(129)]):
            value = request()
            value['axes'][0]['candidates'] = pool
            with self.assertRaises(ValueError):
                prepare_request(value)
        for invalid in ('x\n', 'x\x00', 'x\x7f', 'é', '\ud800', '', 'x' * 65, True):
            value = request()
            value['case']['specialization'] = invalid
            with self.assertRaises(ValueError):
                prepare_request(value)
        for invalid in ('1axis', 'bad-name', 'x\n', '', 'a' * 65, True, {}):
            value = request()
            value['axes'][0]['name'] = invalid
            with self.assertRaises(ValueError):
                prepare_request(value)
        for target, key in [('case', 'objective'), ('client', 'family'), ('axis', 'category')]:
            value = request()
            obj = value['case'] if target == 'case' else value['case']['client'] if target == 'client' else value['axes'][0]
            obj[key] = ['bad']
            with self.assertRaises(ValueError):
                prepare_request(value)

    def test_schema_matches_nested_contract(self):
        schema = input_schema()
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema)
        validator.validate(request())
        self.assertEqual(set(schema['properties']), {'case', 'axes', 'offset', 'limit', 'expected_search_id'})
        for obj in (schema, schema['properties']['case'], schema['properties']['case']['properties']['client'],
                    schema['properties']['axes']['items']):
            self.assertFalse(obj['additionalProperties'])
        value = request()
        value['case']['specialization'] = 'arms\n'
        self.assertTrue(list(validator.iter_errors(value)))


class CallerTests(unittest.TestCase):
    def test_real_cli_and_json_guards(self):
        with tempfile.TemporaryDirectory(dir=ROOT / 'local-data', prefix='build-search-test-') as folder:
            path = Path(folder) / 'input.json'
            def run(input_path):
                return subprocess.run([sys.executable, '-m', 'tools.build_search', 'prepare', '--input', str(input_path)],
                                      cwd=ROOT, capture_output=True, encoding='utf-8', timeout=15)
            path.write_text(encoded(request()), encoding='utf-8')
            result = run(path)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout), prepare_request(request()))
            for payload in ('{}', encoded(request()).replace('"class_id":1', '"class_id":1,"class_id":2'),
                            encoded(request()).replace('"class_id":1', '"class_id":NaN'),
                            encoded(request()).replace('"class_id":1', '"class_id":1.0'),
                            ' ' * (MAX_INPUT_BYTES + 1), '[' * 2000):
                path.write_text(payload, encoding='utf-8')
                failed = run(path)
                self.assertNotEqual(failed.returncode, 0)
                self.assertLess(len(failed.stderr), 1024)
            self.assertNotEqual(run('relative.json').returncode, 0)
            self.assertNotEqual(run(folder).returncode, 0)
            path.write_text(encoded(request()), encoding='utf-8')
            link = Path(folder) / 'linked.json'
            os.link(path, link)
            self.assertNotEqual(run(link).returncode, 0)

    def test_real_stdio_schema_results_and_bounded_errors(self):
        async def check():
            params = StdioServerParameters(command=sys.executable, args=['-m', 'tools.build_search', 'serve'], cwd=str(ROOT))
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    listed = await session.list_tools()
                    self.assertEqual([t.name for t in listed.tools], ['prepare_build_search_window'])
                    self.assertEqual(listed.tools[0].inputSchema, input_schema())
                    result = await session.call_tool('prepare_build_search_window', request())
                    self.assertFalse(result.isError)
                    self.assertIsNone(result.structuredContent)
                    self.assertEqual(json.loads(result.content[0].text), prepare_request(request()))
                    self.assertLess(len(result.model_dump_json().encode()), MAX_BYTES)
                    maximum = request()
                    maximum['axes'] = [dict(name=f'axis_{i}'.ljust(64, '_'), category='gear',
                                            candidates=[ref(n) for n in range(128)]) for i in range(32)]
                    large = await session.call_tool('prepare_build_search_window', maximum)
                    self.assertFalse(large.isError)
                    self.assertLess(len(large.model_dump_json().encode()), MAX_BYTES)
                    oversized = await session.call_tool('prepare_build_search_window', maximum | dict(limit=64))
                    self.assertTrue(oversized.isError)
                    self.assertLess(len(oversized.model_dump_json().encode()), 1024)
                    bad_case = request()
                    bad_case['case']['client']['interface'] = True
                    for value in [request() | dict(limit=True), request() | dict(limit=1.0), bad_case,
                                  request() | dict(offset='00'), request() | dict(database='secret'),
                                  request() | dict(axes='X' * MAX_INPUT_BYTES), request() | dict(expected_search_id=ref('wrong'))]:
                        failed = await session.call_tool('prepare_build_search_window', value)
                        self.assertTrue(failed.isError)
                        self.assertLess(len(failed.model_dump_json().encode()), 1024)
                    unknown = await session.call_tool('run_simulator', request())
                    self.assertTrue(unknown.isError)
                    self.assertLess(len(unknown.model_dump_json().encode()), 1024)
        asyncio.run(asyncio.wait_for(check(), timeout=30))


if __name__ == '__main__':
    unittest.main()
