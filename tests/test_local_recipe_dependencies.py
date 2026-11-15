"""Synthetic local dependency graphs through the real capture builder and callers."""
import asyncio
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from shared.snapshots import SnapshotStore
from tools import forever_local as local

ROOT = Path(__file__).resolve().parents[1]
PRIVATE = 'PRIVATE_OWNER_83922'


class LocalRecipeDependenciesTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=ROOT / 'local-data')
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        self.database = self.folder / 'reference.sqlite'

    def build(self, recipes=None, equipment=None):
        # recipe id -> (reagents, output item, optional observed output range)
        recipes = recipes or {100: ({20: 3}, 10, None), 200: ({30: 2}, 20, (1, 2))}
        entries = {}
        for identity, (reagents, output, quantity) in recipes.items():
            row = {'status': 'fixed_items', 'reagents': {str(k): v for k, v in reagents.items()},
                   'reported_craftable_count': 23}
            if output: row['output_item_id'] = output
            if quantity: row['base_output_quantity'] = {'minimum': quantity[0], 'maximum': quantity[1]}
            entries[str(identity)] = row
        snapshot = {'schema_version': 1, 'source': 'addon', 'captured_at': '2025-01-01T00:00:00Z',
            'client': {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001},
            'character': {'name': PRIVATE, 'realm': PRIVATE},
            'sections': {
                'equipment': equipment or {'1': {'item_id': 30, 'guid': PRIVATE,
                    'link': '|Hitem:30:0|h[Observed Ore]|h'}},
                'profession_recipe_view': {'coverage': 'filtered_local_recipe_list', 'profession_id': 164,
                    'entries': {str(k): {'name': f'Recipe {k}', 'learned': True} for k in recipes}},
                'profession_recipe_materials': {'coverage': 'reported_known_spell_schematics',
                    'profession_id': 164, 'entries': entries}}}
        store = SnapshotStore(self.folder / 'owner')
        snapshot_id = store.ingest(snapshot)
        local.build(store.root, snapshot_id, self.database)
        knowledge = local.LocalForever(self.database)
        self.addCleanup(knowledge.close)
        return knowledge

    def assert_claims(self, result):
        for key in ('all_recipes_covered', 'craftability_verified', 'unlock_prerequisites_verified',
                    'ownership_checked', 'item_variant_equivalence_verified', 'plan_generated', 'live', 'game_actions'):
            self.assertIs(result[key], False)
        self.assertEqual(result['coverage'], 'incomplete_single_capture')
        self.assertEqual(result['graph_coverage_scope'], 'reachable_stored_recipe_ingredients_and_observed_producers')
        self.assertNotIn(PRIVATE, local.encoded(result))
        for field in ('reported_craftable_count', 'learned', 'guid', 'character', 'base_material'):
            self.assertNotIn(field, local.encoded(result))

    def test_chain_exact_edges_safe_fields_and_no_writes_or_network(self):
        with patch.object(socket, 'socket', side_effect=AssertionError('No network')):
            knowledge = self.build()
            before = {p: p.read_bytes() for p in self.folder.rglob('*') if p.is_file()}
            statements = []
            knowledge.connection.set_trace_callback(statements.append)
            graph = knowledge.recipe_dependencies(100)
            self.assertEqual(graph, knowledge.recipe_dependencies(100))
        self.assertEqual(before, {p: p.read_bytes() for p in self.folder.rglob('*') if p.is_file()})
        self.assertTrue(all(sql.startswith('SELECT ') for sql in statements))
        self.assertEqual([(n['kind'], n['id'], n['depth']) for n in graph['nodes']],
                         [('recipe', 100, 0), ('item', 20, 1), ('recipe', 200, 2), ('item', 30, 3)])
        self.assertEqual(graph['nodes'][-1]['fields'], {'observed_names': ['Observed Ore']})
        self.assertEqual(graph['nodes'][-1]['expansion'], 'no_observed_producer')
        self.assertEqual([(e['source_key'], e['relation'], e['target_key']) for e in graph['edges']], [
            ('local:recipe:100', 'reagent', 'local:item:20'),
            ('local:item:20', 'observed_producer', 'local:recipe:200'),
            ('local:recipe:200', 'reagent', 'local:item:30')])
        self.assertEqual(graph['edges'][0]['quantity'], 3)
        self.assertEqual((graph['edges'][1]['minimum'], graph['edges'][1]['maximum']), (1, 2))
        self.assertFalse(graph['partial']); self.assertTrue(graph['stored_graph_complete'])
        self.assert_claims(graph)

    def test_branches_cycle_and_shared_subgraph_expand_once_in_sorted_bfs_order(self):
        knowledge = self.build({300: ({40: 7}, 20, None), 100: ({20: 3}, 10, None),
                               200: ({40: 2}, 20, (1, 1)), 400: ({20: 1}, 40, None)})
        graph = knowledge.recipe_dependencies(100, depth=8)
        self.assertEqual([n['record_key'] for n in graph['nodes']], ['local:recipe:100', 'local:item:20',
            'local:recipe:200', 'local:recipe:300', 'local:item:40', 'local:recipe:400'])
        self.assertEqual(len(graph['edges']), 7)
        producers = [e for e in graph['edges'] if e['source_key'] == 'local:item:20']
        self.assertEqual([e['target_id'] for e in producers], [200, 300])
        self.assertNotIn('minimum', producers[1])
        self.assertTrue(graph['stored_graph_complete']); self.assertFalse(graph['partial'])
        self.assertEqual(graph['frontier'], [])
        self.assert_claims(graph)

    def test_depth_node_and_edge_budget_partialness(self):
        knowledge = self.build()
        for parameters, reason, count in [({'depth': 1}, 'depth_limit', 2),
                ({'node_budget': 1}, 'node_budget', 1), ({'edge_budget': 1}, 'edge_budget', 2)]:
            with self.subTest(parameters=parameters):
                graph = knowledge.recipe_dependencies(100, **parameters)
                self.assertTrue(graph['partial']); self.assertFalse(graph['stored_graph_complete'])
                self.assertEqual(len(graph['nodes']), count)
                self.assertEqual(graph['frontier'][0]['reasons'], [reason])
                self.assertEqual(graph['frontier'][0]['omitted_edges'], 1)
                self.assertEqual(graph['nodes'][-1]['expansion'], 'partial')
                self.assert_claims(graph)
        # Reaching a numeric bound does not itself mean observed edges were omitted.
        graph = knowledge.recipe_dependencies(100, depth=3, node_budget=4, edge_budget=3)
        self.assertFalse(graph['partial']); self.assertEqual(graph['frontier'], [])
        self.assertEqual(graph['nodes'][-1]['expansion'], 'no_observed_producer')

    def test_unknown_root_empty_recipe_and_terminal_item_are_distinct(self):
        knowledge = self.build({100: ({}, 10, None)})
        graph = knowledge.recipe_dependencies(999)
        self.assertFalse(graph['found']); self.assertFalse(graph['stored_graph_complete'])
        self.assertEqual(graph['nodes'], []); self.assertEqual(graph['edges'], [])
        self.assert_claims(graph)
        graph = knowledge.recipe_dependencies(100)
        self.assertEqual(graph['nodes'][0]['expansion'], 'no_observed_reagents')
        self.assertTrue(graph['stored_graph_complete'])

    def test_all_direct_bounds_reject_bool_float_string_and_out_of_range(self):
        knowledge = self.build()
        for name, maximum in [('recipe_id', 4294967295), ('depth', 8), ('node_budget', 128), ('edge_budget', 256)]:
            for value in (True, False, 1.0, '1', None, 0, -1, maximum + 1):
                args = {'recipe_id': 100, name: value}
                with self.subTest(args=args), self.assertRaises(ValueError):
                    knowledge.recipe_dependencies(**args)
        self.assert_claims(knowledge.recipe_dependencies(100, depth=8, node_budget=128, edge_budget=256))

    def test_large_graph_response_retains_byte_cap(self):
        # Stored public native fields can exceed a response budget without private data.
        equipment = {str(slot): {'item_id': 20, 'required_level': slot,
            'link': '|Hitem:20:0|h[Observed material]|h', 'native_stats': {
                'ITEM_MOD_' + 'X' * 70 + str(i) + '_SHORT': i for i in range(128)}} for slot in range(1, 8)}
        knowledge = self.build(equipment=equipment)
        with self.assertRaisesRegex(ValueError, 'response exceeds byte limit'):
            knowledge.recipe_dependencies(100)

    def test_actual_cli_and_stdio_tool_schema_calls_and_guards(self):
        knowledge = self.build()
        before = self.database.read_bytes()
        environment = {**os.environ, 'PYTHONPATH': str(ROOT), 'TEMP': str(self.folder), 'TMP': str(self.folder)}
        command = [sys.executable, '-B', '-m', 'tools.forever_local', 'recipe-dependencies',
                   '--database', str(self.database), '--entity-id', '100']
        reply = subprocess.run([*command, '--depth', '3', '--node-budget', '4', '--edge-budget', '3'],
            cwd=ROOT, env=environment, capture_output=True, text=True, timeout=20, check=True)
        self.assertEqual(json.loads(reply.stdout), knowledge.recipe_dependencies(100, 3, 4, 3))
        for extra in (['--depth', '0'], ['--node-budget', '129'], ['--edge-budget', '1.5']):
            reply = subprocess.run([*command, *extra], cwd=ROOT, env=environment,
                capture_output=True, text=True, timeout=20)
            self.assertNotEqual(reply.returncode, 0); self.assertEqual(reply.stdout, '')

        async def exercise():
            # Windows asyncio may connect local self-pipes; all non-loopback traffic fails.
            script = ('import socket,runpy,sys; original=socket.socket.connect;'
                "socket.socket.connect=lambda sock,address:original(sock,address) if isinstance(address,tuple) "
                "and address[0] in ('127.0.0.1','::1') else (_ for _ in ()).throw(RuntimeError('No network'));"
                "sys.argv=['forever_local','serve',*sys.argv[1:]];"
                "runpy.run_module('tools.forever_local',run_name='__main__')")
            params = StdioServerParameters(command=sys.executable, args=['-B', '-c', script,
                '--database', str(self.database)], cwd=str(ROOT), env=environment)
            with tempfile.TemporaryFile(mode='w+', dir=self.folder) as errors:
                async with stdio_client(params, errlog=errors) as (read, write):
                    async with ClientSession(read, write) as session:
                        await session.initialize()
                        tool = next(t for t in (await session.list_tools()).tools
                                    if t.name == 'get_local_forever_recipe_dependencies')
                        self.assertTrue(tool.annotations.readOnlyHint)
                        self.assertFalse(tool.annotations.openWorldHint)
                        self.assertFalse(tool.annotations.destructiveHint)
                        properties = tool.inputSchema['properties']
                        self.assertEqual(set(properties), {'recipe_id', 'depth', 'node_budget', 'edge_budget'})
                        self.assertEqual(tool.inputSchema['required'], ['recipe_id'])
                        for key, high, default in [('depth', 8, 4), ('node_budget', 128, 64), ('edge_budget', 256, 128)]:
                            self.assertEqual((properties[key]['type'], properties[key]['minimum'],
                                properties[key]['maximum'], properties[key]['default']), ('integer', 1, high, default))
                        result = await session.call_tool(tool.name, {'recipe_id': 100})
                        self.assertFalse(result.isError)
                        self.assertEqual(json.loads(result.content[0].text), knowledge.recipe_dependencies(100))
                        for args in ({'recipe_id': True}, {'recipe_id': 100, 'depth': 1.0},
                                     {'recipe_id': 100, 'node_budget': False}, {'recipe_id': 100, 'edge_budget': 257}):
                            self.assertTrue((await session.call_tool(tool.name, args)).isError)
        asyncio.run(asyncio.wait_for(exercise(), 30))
        self.assertEqual(before, self.database.read_bytes())


if __name__ == '__main__':
    unittest.main()
