"""Bounded quest graph checks through the real cache reader, CLI and offline stdio MCP."""
import asyncio
from contextlib import contextmanager
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from tools import forever_world as world


LOCAL = world.ROOT / 'local-data'
# This runs the actual CLI parser and FastMCP server with a synthetic pinned cache.
BOOT = '''
import json, socket, subprocess, sys
from tools import forever_world as world
world.PIN = json.loads(open(sys.argv.pop(1), encoding='utf-8').read())
connect = socket.socket.connect
def offline(sock, address):
    if isinstance(address, tuple) and address[0] in ('127.0.0.1', '::1'):
        return connect(sock, address)
    raise AssertionError('External network forbidden')
socket.socket.connect = offline
def audit(event, args):
    if event == 'open':
        mode, flags = args[1:3]
        if (isinstance(mode, str) and any(c in mode for c in 'wax+')) or flags & 0x703:
            raise AssertionError('File write forbidden')
    if event in ('subprocess.Popen', 'socket.getaddrinfo', 'os.remove', 'os.rename', 'os.mkdir'):
        raise AssertionError('Side effect forbidden: ' + event)
sys.addaudithook(audit)
world.main(sys.argv[1:])
'''


@contextmanager
def fixture(records):
    LOCAL.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='quest-walk-', dir=LOCAL) as folder:
        base = Path(folder)
        generation = {'contract': 4, 'derived_passes': 0, 'static_corrections': 0}
        rows = [{'kind': 'quest', 'id': identity, 'fields': {'name': f'Quest {identity}', **fields}}
                for identity, fields in records.items()]
        lines = [json.dumps(row, sort_keys=True).encode() for row in [generation, *rows]]
        data = b'\n'.join(lines) + b'\n'
        cache = base / 'world.jsonl'
        cache.write_bytes(data)
        pin = {'provider': 'synthetic/QuestieDB', 'revision': 'a' * 40,
               'cache_sha256': hashlib.sha256(data).hexdigest(), 'counts': {'quest': len(rows)},
               'generation': generation, 'source_files': {}}
        pin_path = base / 'pin.json'
        pin_path.write_text(json.dumps(pin), encoding='utf-8')
        with patch.object(world, 'PIN', pin):
            yield world.World(cache), cache, pin_path


class QuestLinkWalkTests(unittest.TestCase):
    def test_all_exact_relationships_and_public_summaries(self):
        grouped = ('preQuestGroup', 'preQuestSingle', 'childQuests', 'inGroupWith',
                   'exclusiveTo', 'breadcrumbs')
        scalar = ('nextQuestInChain', 'parentQuest', 'breadcrumbForQuestId',
                  'availableUntilCompleted', 'availableStartingWith', 'disabledByQuest')
        fields = {name: {'2': 2, '1': 3} for name in grouped}
        fields.update({name: 2 for name in scalar})
        fields['unrelatedQuestField'] = 999
        with fixture({1: fields, 2: {'nextQuestInChain': 4}, 3: {}, 4: {}}) as (db, cache, _):
            before = cache.read_bytes()
            with (patch('socket.socket.connect', side_effect=AssertionError('network')),
                  patch.object(world.subprocess, 'run', side_effect=AssertionError('process'))):
                result = db.walk_quest_links(1)
            self.assertEqual(cache.read_bytes(), before)
            self.assertEqual([node['id'] for node in result['nodes']], [1, 2, 3, 4])
            self.assertEqual([node['depth'] for node in result['nodes']], [0, 1, 1, 2])
            expected = [(field, position) for field in sorted((*grouped, *scalar))
                        for position in (('1', '2') if field in grouped else ('1',))]
            self.assertEqual([(edge['field'], edge['position']) for edge in result['edges'][:-1]], expected)
            self.assertEqual(result['edge_count'], 19)
            self.assertTrue(result['stored_graph_complete'])
            self.assertEqual(result['frontier'], [])
            for node in result['nodes']:
                for key, value in db.summary(('quest', node['id'])).items():
                    self.assertEqual(node[key], value)
                raw = next(line for line in before.splitlines()[1:] if json.loads(line)['id'] == node['id'])
                self.assertEqual(node['record_sha256'], hashlib.sha256(raw).hexdigest())
                self.assertNotIn('fields', node)
            for key in ('live', 'native_availability_verified', 'complete_world_data_verified',
                        'character_state_checked', 'quest_eligibility_verified', 'route_generated'):
                self.assertIs(result[key], False)
            self.assertEqual(result['nodes'][-1]['status'], 'no_recorded_links')

    def test_cycles_shared_references_and_verbatim_positions(self):
        with fixture({1: {'childQuests': {'2': 2, '10': 3, '1': 2}},
                      2: {'parentQuest': 1, 'nextQuestInChain': 3},
                      3: {'nextQuestInChain': 2}}) as (db, _, _):
            result = db.walk_quest_links(1)
            self.assertEqual([node['id'] for node in result['nodes']], [1, 2, 3])
            self.assertEqual([edge['position'] for edge in result['edges'][:3]], ['1', '10', '2'])
            self.assertEqual(result['edge_count'], 6)
            self.assertTrue(result['stored_graph_complete'])
            self.assertEqual(result, db.walk_quest_links(1))

    def test_missing_unsupported_and_unknown_root(self):
        with fixture({1: {'preQuestGroup': [2], 'nextQuestInChain': True,
                         'childQuests': {'1': 99, '2': 0, '3': {'id': 2}}}}) as (db, _, _):
            result = db.walk_quest_links(1)
            self.assertFalse(result['stored_graph_complete'])
            self.assertEqual(result['unsupported_reference_count'], 4)
            self.assertEqual(result['missing_node_count'], 1)
            self.assertEqual(result['nodes'][1]['id'], 99)
            self.assertFalse(result['nodes'][1]['found'])
            self.assertEqual(result['nodes'][1]['status'], 'missing')
            self.assertIsNone(result['nodes'][1]['record_sha256'])
            self.assertEqual(result['frontier'], [])
            for edge in result['edges']:
                if edge['status'] == 'unsupported':
                    self.assertIsNone(edge['target_quest_id'])
            malformed = result['edges'][-1]
            self.assertEqual((malformed['field'], malformed['position']), ('preQuestGroup', None))
            unknown = db.walk_quest_links(42)
            self.assertEqual(unknown['status'], 'root_not_found')
            self.assertFalse(unknown['found'])
            self.assertFalse(unknown['stored_graph_complete'])
            self.assertEqual(unknown['edge_count'], 0)

    def test_depth_and_exact_node_and_edge_boundaries(self):
        with fixture({1: {'nextQuestInChain': 2}, 2: {'nextQuestInChain': 3}, 3: {}}) as (db, _, _):
            complete = db.walk_quest_links(1, depth=2, node_budget=3, edge_budget=2)
            self.assertTrue(complete['stored_graph_complete'])
            for args, reason, source in [({'depth': 1}, 'depth_limit', 2),
                                        ({'node_budget': 2}, 'node_budget', 2),
                                        ({'edge_budget': 1}, 'edge_budget', 2)]:
                result = db.walk_quest_links(1, **args)
                self.assertFalse(result['stored_graph_complete'])
                self.assertEqual(result['omitted_link_count'], 1)
                self.assertEqual(result['frontier'], [{'quest_id': source, 'depth': 1,
                                  'reason': reason, 'omitted_link_count': 1}])
                self.assertEqual(result['nodes'][-1]['status'], 'stopped')
            tiny = db.walk_quest_links(1, node_budget=1)
            self.assertEqual((tiny['node_count'], tiny['edge_count']), (1, 0))
            self.assertEqual(tiny['frontier'][0]['reason'], 'node_budget')
        with fixture({1: {}}) as (db, _, _):
            self.assertTrue(db.walk_quest_links(1, node_budget=1, edge_budget=1)['stored_graph_complete'])

    def test_budget_does_not_hide_remaining_existing_or_unsupported_references(self):
        with fixture({1: {'childQuests': {'1': 2, '2': 1, '3': False}}}) as (db, _, _):
            result = db.walk_quest_links(1, node_budget=1)
            self.assertEqual(result['edge_count'], 2)
            self.assertEqual([edge['status'] for edge in result['edges']], ['found', 'unsupported'])
            self.assertEqual(result['omitted_link_count'], 1)

    def test_maximum_budgets_bound_dense_and_wide_graphs(self):
        with fixture({1: {'childQuests': {str(i): 1 for i in range(300)}}}) as (db, _, _):
            result = db.walk_quest_links(1, edge_budget=256)
            self.assertEqual((result['node_count'], result['edge_count']), (1, 256))
            self.assertEqual(result['omitted_link_count'], 44)
            self.assertEqual(len(result['frontier']), 1)
        records = {i: {} for i in range(1, 131)}
        records[1]['childQuests'] = {str(i): i for i in range(2, 131)}
        with fixture(records) as (db, _, _):
            result = db.walk_quest_links(1, node_budget=128, edge_budget=256)
            self.assertEqual((result['node_count'], result['edge_count']), (128, 127))
            self.assertEqual(result['omitted_link_count'], 2)
            self.assertEqual(result['frontier'][0]['reason'], 'node_budget')

    def test_strict_types_and_all_ranges(self):
        with fixture({1: {}}) as (db, _, _):
            for parameter, maximum in [('quest_id', 2**32-1), ('depth', 8),
                                       ('node_budget', 128), ('edge_budget', 256)]:
                for value in [False, True, 1.0, '1', None, 0, -1, maximum+1]:
                    with self.subTest(parameter=parameter, value=value), self.assertRaises(ValueError):
                        db.walk_quest_links(**{'quest_id': 1, parameter: value})
            self.assertTrue(db.walk_quest_links(1, 8, 128, 256)['stored_graph_complete'])

    def test_actual_response_limit_and_old_get_search_unchanged(self):
        with fixture({1: {'name': 'x' * (256 * 1024)}}) as (db, _, _):
            self.assertEqual(world.MAX_RESPONSE, 256 * 1024)
            with self.assertRaisesRegex(ValueError, 'response exceeds byte limit'):
                db.walk_quest_links(1)
        with fixture({1: {'nextQuestInChain': 2}, 2: {}}) as (db, _, _):
            before = [db.get('quest', 1), db.search('Quest'), db.get('quest', 1, include_quest_links=True)]
            db.walk_quest_links(1)
            self.assertEqual(before, [db.get('quest', 1), db.search('Quest'),
                                      db.get('quest', 1, include_quest_links=True)])
            self.assertNotIn('quest_links', before[0])

    def test_actual_cli_offline_no_writes(self):
        with fixture({1: {'nextQuestInChain': 2}, 2: {}}) as (db, cache, pin):
            before = {path.name: path.read_bytes() for path in cache.parent.iterdir()}
            command = [sys.executable, '-B', '-c', BOOT, str(pin)]
            good = subprocess.run([*command, 'quest-links', '--cache', str(cache), '--entity-id', '1',
                                   '--depth', '2', '--node-budget', '2', '--edge-budget', '1'],
                                  capture_output=True, text=True, timeout=20)
            self.assertEqual(good.returncode, 0, good.stderr)
            self.assertEqual(json.loads(good.stdout), db.walk_quest_links(1, 2, 2, 1))
            for args in [[], ['--depth', '0'], ['--depth', '1.0'], ['--node-budget', '129'],
                         ['--edge-budget', '257']]:
                invocation = [*command, 'quest-links', '--cache', str(cache)]
                if args:
                    invocation += ['--entity-id', '1', *args]
                reply = subprocess.run(invocation, capture_output=True, text=True, timeout=20)
                self.assertNotEqual(reply.returncode, 0)
            self.assertEqual(before, {path.name: path.read_bytes() for path in cache.parent.iterdir()})


class QuestLinkWalkMcpTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_stdio_schema_and_calls_offline_no_writes(self):
        with fixture({1: {'nextQuestInChain': 2}, 2: {}}) as (db, cache, pin):
            before = {path.name: path.read_bytes() for path in cache.parent.iterdir()}
            with tempfile.TemporaryFile(mode='w+', dir=LOCAL) as errors:
                params = StdioServerParameters(command=sys.executable,
                    args=['-B', '-c', BOOT, str(pin), 'serve', '--cache', str(cache)], cwd=str(world.ROOT))
                async with stdio_client(params, errlog=errors) as (read, write):
                    async with ClientSession(read, write) as session:
                        await asyncio.wait_for(session.initialize(), 15)
                        tools = {tool.name: tool for tool in (await session.list_tools()).tools}
                        tool = tools['walk_forever_quest_links']
                        self.assertTrue(tool.annotations.readOnlyHint)
                        self.assertFalse(tool.annotations.openWorldHint)
                        self.assertFalse(tool.annotations.destructiveHint)
                        properties = tool.inputSchema['properties']
                        self.assertEqual(tool.inputSchema['required'], ['quest_id'])
                        for name, maximum, default in [('quest_id', 2**32-1, None), ('depth', 8, 3),
                                                       ('node_budget', 128, 64), ('edge_budget', 256, 128)]:
                            self.assertEqual(properties[name]['type'], 'integer')
                            self.assertEqual(properties[name]['minimum'], 1)
                            self.assertEqual(properties[name]['maximum'], maximum)
                            if default is not None:
                                self.assertEqual(properties[name]['default'], default)
                        reply = await asyncio.wait_for(session.call_tool(tool.name, {'quest_id': 1}), 10)
                        self.assertFalse(reply.isError, reply)
                        self.assertEqual(json.loads(reply.content[0].text), db.walk_quest_links(1))
                        for args in [{'quest_id': True}, {'quest_id': 1.0}, {'quest_id': 0},
                                     {'quest_id': 1, 'depth': False}, {'quest_id': 1, 'depth': 9},
                                     {'quest_id': 1, 'node_budget': 1.0}, {'quest_id': 1, 'edge_budget': 257}]:
                            reply = await asyncio.wait_for(session.call_tool(tool.name, args), 10)
                            self.assertTrue(reply.isError, reply)
                        for name, args, expected in [
                            ('get_forever_world_record', {'kind': 'quest', 'entity_id': 1}, db.get('quest', 1)),
                            ('search_forever_world_records', {'query': 'Quest'}, db.search('Quest'))]:
                            reply = await asyncio.wait_for(session.call_tool(name, args), 10)
                            self.assertFalse(reply.isError, reply)
                            self.assertEqual(json.loads(reply.content[0].text), expected)
            self.assertEqual(before, {path.name: path.read_bytes() for path in cache.parent.iterdir()})


if __name__ == '__main__':
    unittest.main()
