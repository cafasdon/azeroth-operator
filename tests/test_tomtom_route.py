"""Offline caller, transport, bounds and injection tests; no game or addon writes."""
import asyncio
from copy import deepcopy
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
from tools.tomtom_route import MAX_BYTES, encoded, input_schema, prepare_request, prepare_route

ROOT = Path(__file__).resolve().parents[1]


def step(**changes):
    return dict(ui_map_id=84, x_percent=12.1234567, y_percent=100, label='Quest stop') | changes


def request(**changes):
    return dict(title='Test route', steps=[step(), step(label='Return')]) | changes


class RouteTests(unittest.TestCase):
    def test_parser_label_normalisation_is_explicit_and_preserves_requested_text(self):
        draft = prepare_route('Route 1,2', [step(label='Stop 1. 2 and 3.4')])
        pin = draft['manifest'][0]
        self.assertEqual(draft['title'], 'Route 1,2')
        self.assertEqual(pin['label'], 'Stop 1. 2 and 3.4')
        self.assertTrue(pin['parser_may_change_pin_label'])
        self.assertTrue(pin['pin_label'].endswith('Route 1,2: Stop 1. 2 and 3.4'))
        self.assertTrue(pin['display_pin_label_variants']['dot'].endswith('Route 1.2: Stop 1 2 and 3.4'))
        self.assertTrue(pin['display_pin_label_variants']['comma'].endswith('Route 1,2: Stop 1 2 and 3,4'))
        simple = prepare_route('Simple', [step(label='Return')])['manifest'][0]
        self.assertFalse(simple['parser_may_change_pin_label'])
        self.assertNotIn('display_pin_label_variants', simple)

    def test_order_duplicates_identity_and_default_active(self):
        source = request(steps=[step(), step(), step(ui_map_id=85)])
        original = deepcopy(source)
        result = prepare_request(source)
        self.assertEqual(source, original)
        self.assertEqual(result, prepare_request(source))
        self.assertRegex(result['route_id'], r'^[0-9a-f]{64}$')
        self.assertEqual([s['number'] for s in result['manifest']], [1, 2, 3])
        self.assertEqual([s['ui_map_id'] for s in result['manifest']], [84, 84, 85])
        self.assertEqual(len({s['pin_label'] for s in result['manifest']}), 3)
        self.assertEqual(len(result['manual_import_text'].splitlines()), 1)
        self.assertIn('01/03 [', result['manual_import_text'])
        self.assertTrue(result['draft_only'])
        for name in ('waypoint_creation_performed', 'automatic_movement', 'auto_advance',
                     'arrow_order_enforced', 'quest_completion_checked', 'native_coordinates_verified',
                     'addon_runtime_verified', 'accessibility_verified'):
            self.assertIs(result[name], False)
        self.assertNotEqual(result['route_id'], prepare_request(request(title='Other'))['route_id'])

    def test_selection_all_pins_and_rounding_disclosed(self):
        value = request(steps=[step(), step(x_percent=0, y_percent=0.000000001)])
        result = prepare_request(value | {'active_step': 2})
        all_pins = prepare_request(value | {'mode': 'all_pins'})
        self.assertEqual(result['route_id'], all_pins['route_id'])
        self.assertEqual(result['manual_import_text'], all_pins['manual_import_text'].splitlines()[1])
        first, second = result['manifest']
        self.assertEqual(first['x_percent'], 12.1234567)
        self.assertEqual(first['rendered_x_percent'], '12.12346')
        self.assertEqual(second['rendered_y_percent'], '0.00000')
        self.assertTrue(first['coordinates_rounded'])
        self.assertTrue(second['coordinates_rounded'])
        exact = prepare_route('Exact', [step(x_percent=-0.0, y_percent=100)])['manifest'][0]
        self.assertEqual(exact['rendered_x_percent'], '0.00000')
        self.assertFalse(exact['coordinates_rounded'])
        for line in all_pins['manual_import_text'].splitlines():
            self.assertRegex(line, r'^/tway #[1-9][0-9]* [0-9]+\.[0-9]{5} [0-9]+\.[0-9]{5} [0-9]{2}/[0-9]{2} \[')

    def test_text_bytes_whitespace_and_injection(self):
        result = prepare_route('  A\u00a0  B ', [step(label=' /run print("x")  ; reset all ')])
        self.assertEqual(result['title'], 'A B')
        self.assertEqual(result['manifest'][0]['label'], '/run print("x") ; reset all')
        self.assertEqual(len(result['manual_import_text'].splitlines()), 1)
        self.assertTrue(result['manual_import_text'].startswith('/tway #84 '))
        prepare_route('é' * 40, [step(label='x' * 80)])
        invalid = ['', ' ', 'é' * 41, 'x' * 81, '|cffffffff', '\n/run x', '\r', '\t',
                   '\0', '\x7f', '\x85', '\u200b', '\u202e', '\u2028', '\u2029', '\ud800']
        for text in invalid:
            for value in (request(title=text), request(steps=[step(label=text)])):
                with self.subTest(text=repr(text)), self.assertRaises(ValueError):
                    prepare_request(value)

    def test_exact_keys_and_all_scalar_types(self):
        invalid = [None, True, 1.0, '1', 0, -1, 4294967296]
        for value in invalid:
            with self.subTest(map=value), self.assertRaises(ValueError):
                prepare_request(request(steps=[step(ui_map_id=value)]))
        for value in [None, True, '10', float('nan'), float('inf'), -float('inf'), -1, 100.000001, 10**1000]:
            for name in ('x_percent', 'y_percent'):
                with self.subTest(coordinate=name, value=str(value)), self.assertRaises(ValueError):
                    prepare_request(request(steps=[step(**{name: value})]))
        for value in [True, 1.0, '1', 0, 3, None]:
            with self.subTest(active=value), self.assertRaises(ValueError):
                prepare_request(request(active_step=value))
        for value in [None, 1, True, 'automatic', []]:
            with self.subTest(mode=value), self.assertRaises(ValueError):
                prepare_request(request(mode=value))
        for value in [None, [], (), [None], [step()] * 65, {'1': step()}]:
            with self.subTest(steps=value), self.assertRaises(ValueError):
                prepare_request(request(steps=value))
        for extra in ['area_id', 'provider_area_id', 'namespace', 'quest_id', 'command']:
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                prepare_request(request(steps=[step() | {extra: 84}]))
        for value in [None, [], request(database='secret'), {'title': 'A'},
                      request(steps=[{k: v for k, v in step().items() if k != 'ui_map_id'}])]:
            with self.subTest(request=value), self.assertRaises(ValueError):
                prepare_request(value)
        prepare_request(request(steps=[step(ui_map_id=4294967295, x_percent=0, y_percent=100)]))

    def test_maximum_response_and_purity(self):
        value = request(title='é' * 40, steps=[step(label='é' * 40)] * 64, mode='all_pins', active_step=64)
        with patch('builtins.open', side_effect=AssertionError('No files')), \
                patch('socket.socket', side_effect=AssertionError('No network')), \
                patch('subprocess.Popen', side_effect=AssertionError('No processes')):
            result = prepare_request(value)
        envelope = {'content': [{'type': 'text', 'text': encoded(result)}], 'isError': False}
        self.assertLess(len(encoded(envelope).encode()), MAX_BYTES)
        self.assertEqual(len(result['manual_import_text'].splitlines()), 64)
        hostile = request(title='"' * 80, steps=[step(label='\\' * 80)] * 64, mode='all_pins')
        with self.assertRaisesRegex(ValueError, 'transport budget'):
            prepare_request(hostile)

    def test_schema_denies_other_interfaces(self):
        schema = input_schema()
        self.assertFalse(schema['additionalProperties'])
        self.assertFalse(schema['properties']['steps']['items']['additionalProperties'])
        self.assertEqual(set(schema['properties']), {'title', 'steps', 'mode', 'active_step'})
        self.assertEqual(schema['properties']['steps']['maxItems'], 64)


class CallerTests(unittest.TestCase):
    def test_real_cli_and_input_guards(self):
        with tempfile.TemporaryDirectory(dir=ROOT / 'local-data', prefix='tomtom-test-') as folder:
            path = Path(folder) / 'input.json'
            def run(args):
                return subprocess.run([sys.executable, '-m', 'tools.tomtom_route', *args],
                                      cwd=ROOT, capture_output=True, encoding='utf-8', timeout=15)
            path.write_text(encoded(request()), encoding='utf-8')
            output = run(['prepare', '--input', str(path)])
            self.assertEqual(output.returncode, 0, output.stderr)
            self.assertEqual(json.loads(output.stdout), prepare_request(request()))
            for text in ['{}', '{"title":"a","title":"b","steps":[]}',
                         encoded(request()).replace('12.1234567', 'NaN'), ' ' * (MAX_BYTES + 1)]:
                path.write_text(text, encoding='utf-8')
                self.assertNotEqual(run(['prepare', '--input', str(path)]).returncode, 0)
            self.assertNotEqual(run(['prepare', '--input', 'input.json']).returncode, 0)
            self.assertNotEqual(run(['prepare', '--input', str(Path(folder))]).returncode, 0)
            self.assertNotEqual(run(['serve', '--input', str(path)]).returncode, 0)
            path.write_text(encoded(request()), encoding='utf-8')
            link = Path(folder) / 'hardlink.json'
            os.link(path, link)
            self.assertNotEqual(run(['prepare', '--input', str(link)]).returncode, 0)

    def test_real_stdio_tool_and_guard_failures(self):
        async def check():
            params = StdioServerParameters(command=sys.executable,
                       args=['-m', 'tools.tomtom_route', 'serve'], cwd=str(ROOT))
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    listing = await session.list_tools()
                    self.assertEqual([t.name for t in listing.tools], ['prepare_tomtom_route'])
                    self.assertEqual(listing.tools[0].inputSchema, input_schema())
                    result = await session.call_tool('prepare_tomtom_route', request())
                    self.assertFalse(result.isError)
                    self.assertIsNone(result.structuredContent)
                    self.assertEqual(json.loads(result.content[0].text), prepare_request(request()))
                    self.assertLess(len(result.model_dump_json().encode()), MAX_BYTES)
                    for value in [request(database='x'), request(active_step=1.0), request(active_step=True),
                                  request(steps=[step(ui_map_id=84.0)]), request(steps=[step(ui_map_id=True)]),
                                  request(steps=[step(area_id=84)]), request(steps=[step(label='x\n/run x')]),
                                  request(steps=[step(x_percent='1')]), request(active_step=3),
                                  request(title='x' * MAX_BYTES * 2), request(steps=[step()] * 65)]:
                        failed = await session.call_tool('prepare_tomtom_route', value)
                        self.assertTrue(failed.isError, value)
                        self.assertLess(len(failed.model_dump_json().encode()), MAX_BYTES)
                    unknown = await session.call_tool('execute_route', request())
                    self.assertTrue(unknown.isError)
        asyncio.run(check())


if __name__ == '__main__':
    unittest.main()
