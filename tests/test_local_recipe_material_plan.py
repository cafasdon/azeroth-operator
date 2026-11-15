"""Offline material arithmetic from synthetic captures, including CLI and MCP callers."""
import asyncio
from contextlib import closing
import json
import os
from pathlib import Path
import socket
import sqlite3
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
PRIVATE = 'PRIVATE_OWNER_MATERIAL_PLAN'


class LocalRecipeMaterialPlanTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=ROOT / 'local-data')
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        self.database = self.folder / 'reference.sqlite'

    def build(self, recipes=None, equipment=None):
        recipes = recipes or {100: ({20: 3}, 10, None), 200: ({30: 2}, 20, (2, 2))}
        entries = {}
        for identity, (reagents, output, quantity) in recipes.items():
            row = {'status': 'fixed_items', 'reagents': {str(k): v for k, v in reagents.items()},
                   'reported_craftable_count': 99}
            if output: row['output_item_id'] = output
            if quantity: row['base_output_quantity'] = {'minimum': quantity[0], 'maximum': quantity[1]}
            entries[str(identity)] = row
        snapshot = {'schema_version': 1, 'source': 'addon', 'captured_at': '2025-01-01T00:00:00Z',
            'client': {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001},
            'character': {'name': PRIVATE, 'realm': PRIVATE}, 'sections': {
                'equipment': equipment or {'1': {'item_id': 30, 'guid': PRIVATE}},
                'bags': {'0': {'1': {'item_id': 20, 'count': 999}}},
                'profession_recipe_view': {'coverage': 'filtered_local_recipe_list', 'profession_id': 164,
                    'entries': {str(k): {'name': f'Recipe {k}', 'learned': True} for k in recipes}},
                'profession_recipe_materials': {'coverage': 'reported_known_spell_schematics',
                    'profession_id': 164, 'entries': entries}}}
        store = SnapshotStore(self.folder / 'owner')
        local.build(store.root, store.ingest(snapshot), self.database)
        return self.load()

    def load(self):
        knowledge = local.LocalForever(self.database)
        self.addCleanup(knowledge.close)
        return knowledge

    def requirements(self, result):
        return {row['item_id']: row['quantity'] for row in result['unexpanded_item_requirements']}

    def assert_claims(self, result):
        self.assertIs(result['plan_generated'], True)
        for key in ('live', 'game_actions', 'ownership_checked', 'inventory_subtracted',
                    'craftability_verified', 'unlock_prerequisites_verified',
                    'item_variant_equivalence_verified', 'all_recipes_covered'):
            self.assertIs(result[key], False)
        self.assertEqual(result['coverage'], 'incomplete_single_capture')
        self.assertEqual(result['source'], 'owner_addon_capture')
        for field in (PRIVATE, 'reported_craftable_count', 'learned', 'guid', 'character',
                      'base_material', 'shortage', 'unowned'):
            self.assertNotIn(field, local.encoded(result))

    def test_direct_count_no_automatic_choice_inventory_or_root_yield_needed(self):
        knowledge = self.build()
        result = knowledge.recipe_material_plan(100, 3)
        self.assertEqual(self.requirements(result), {20: 9})
        self.assertEqual(len(result['crafting_steps']), 1)
        step = result['crafting_steps'][0]
        self.assertEqual(step['crafts'], 3)
        self.assertEqual(step['reagents'], [{'item_id': 20, 'recorded_quantity_per_craft': 3,
                                             'required_quantity': 9}])
        self.assertNotIn('fixed_output_yield', step)
        self.assert_claims(result)

    def test_chain_fixed_yields_excess_and_determinism_without_writes_or_network(self):
        with patch.object(socket, 'socket', side_effect=AssertionError('No network')):
            knowledge = self.build({100: ({20: 3}, None, None),
                200: ({30: 3}, 20, (2, 2)), 300: ({40: 5}, 30, (4, 4))})
            before = {p: p.read_bytes() for p in self.folder.rglob('*') if p.is_file()}
            statements = []
            knowledge.connection.set_trace_callback(statements.append)
            result = knowledge.recipe_material_plan(100, 3, {'20': 200, '30': 300})
            self.assertEqual(result, knowledge.recipe_material_plan(100, 3, {'30': 300, '20': 200}))
        self.assertEqual(before, {p: p.read_bytes() for p in self.folder.rglob('*') if p.is_file()})
        self.assertTrue(all(sql.startswith('SELECT ') for sql in statements))
        self.assertEqual(self.requirements(result), {40: 20})
        steps = result['crafting_steps']
        self.assertEqual([(s['recipe_id'], s['crafts'], s['depth']) for s in steps],
                         [(300, 4, 2), (200, 5, 1), (100, 3, 0)])
        self.assertEqual([(s['fixed_output_yield'], s['required_output_quantity'],
            s['output_quantity'], s['excess_output_quantity']) for s in steps[:2]],
            [(4, 15, 16, 1), (2, 9, 10, 1)])
        self.assert_claims(result)

    def test_shared_demand_aggregates_before_rounding_at_unequal_depths(self):
        # Root demands one bar directly and one through another intermediate.
        knowledge = self.build({100: ({20: 1, 30: 1}, 10, None),
            200: ({40: 7}, 20, (2, 2)), 300: ({20: 1}, 30, (1, 1))})
        result = knowledge.recipe_material_plan(100, producer_choices={'20': 200, '30': 300})
        self.assertEqual(self.requirements(result), {40: 7})  # Per-path rounding would consume 14.
        self.assertEqual([(s['recipe_id'], s['crafts']) for s in result['crafting_steps']],
                         [(200, 1), (300, 1), (100, 1)])
        self.assertEqual(result['crafting_steps'][0]['required_output_quantity'], 2)
        self.assertEqual(result['crafting_steps'][0]['depth'], 2)

    def test_explicit_alternative_and_unexpanded_aggregate(self):
        knowledge = self.build({100: ({20: 3, 40: 1}, 10, None),
            200: ({40: 2}, 20, (1, 1)), 300: ({40: 5}, 20, (3, 3))})
        self.assertEqual(self.requirements(knowledge.recipe_material_plan(100)), {20: 3, 40: 1})
        self.assertEqual(self.requirements(knowledge.recipe_material_plan(100, producer_choices={'20': 200})), {40: 7})
        self.assertEqual(self.requirements(knowledge.recipe_material_plan(100, producer_choices={'20': 300})), {40: 6})

    def test_missing_empty_root_and_unknown_variable_or_wrong_output(self):
        knowledge = self.build({100: ({20: 1}, None, None), 200: ({}, 20, None),
            300: ({}, 20, (1, 2)), 400: ({}, 30, (1, 1)), 500: ({}, None, None)})
        with self.assertRaisesRegex(ValueError, 'not recorded'): knowledge.recipe_material_plan(999)
        self.assertEqual(self.requirements(knowledge.recipe_material_plan(500)), {})
        for producer in (200, 300, 400, 500, 999):
            with self.subTest(producer=producer), self.assertRaises(ValueError):
                knowledge.recipe_material_plan(100, producer_choices={'20': producer})

    def test_cycles_unused_choices_and_reused_producer(self):
        knowledge = self.build({100: ({20: 1}, 10, (1, 1)), 200: ({10: 1}, 20, (1, 1)),
                               300: ({20: 1}, 20, (1, 1))})
        for choices, message in [({'20': 200, '10': 100}, 'cycle'),
                ({'20': 300}, 'cycle'), ({'30': 200}, 'Unused'),
                ({'20': 200, '30': 200}, 'multiple chosen')]:
            with self.subTest(choices=choices), self.assertRaisesRegex(ValueError, message):
                knowledge.recipe_material_plan(100, producer_choices=choices)

    def test_strict_ids_counts_and_choices(self):
        knowledge = self.build()
        for name, maximum in [('recipe_id', 4294967295), ('crafts', 100000)]:
            for value in (True, False, 1.0, '1', None, 0, -1, maximum + 1):
                with self.subTest(name=name, value=value), self.assertRaises(ValueError):
                    knowledge.recipe_material_plan(**{'recipe_id': 100, name: value})
        bad = [[], '20', 1, True, {20: 200}, {'020': 200}, {'+20': 200}, {'20 ': 200},
               {'0': 200}, {'4294967296': 200}, {'1' * 2000: 200}, {'20': True},
               {'20': 200.0}, {'20': '200'}, {'20': 0}, {'20': -1}, {'20': 4294967296},
               {str(i): i for i in range(1, 34)}]
        for choices in bad:
            with self.subTest(choices=choices), self.assertRaises(ValueError):
                knowledge.recipe_material_plan(100, producer_choices=choices)
        self.assertEqual(self.requirements(knowledge.recipe_material_plan(100, 100000)), {20: 300000})

    def test_depth_and_edge_budgets_fail_whole_plan(self):
        recipes = {i: ({i + 1001: 1}, i + 1000, (1, 1)) for i in range(1, 11)}
        knowledge = self.build(recipes)
        choices = {str(i + 1000): i for i in range(2, 10)}
        self.assertEqual(len(knowledge.recipe_material_plan(1, producer_choices=choices)['crafting_steps']), 9)
        with self.assertRaisesRegex(ValueError, 'depth'):
            knowledge.recipe_material_plan(1, producer_choices={**choices, '1010': 10})
        recipes = {100: ({i: 1 for i in range(1, 5)}, None, None)}
        recipes.update({200 + i: ({j: 1 for j in range(1000, 1064)}, i, (1, 1)) for i in range(1, 5)})
        knowledge = self.build(recipes)
        with self.assertRaisesRegex(ValueError, 'edge budget'):
            knowledge.recipe_material_plan(100, producer_choices={str(i): 200 + i for i in range(1, 5)})
        for i in range(1, 5): del recipes[200 + i][0][1063]
        result = self.build(recipes).recipe_material_plan(100, producer_choices={str(i): 200 + i for i in range(1, 5)})
        self.assertEqual(sum(len(step['reagents']) for step in result['crafting_steps']), 256)

    def test_maximum_choice_count_and_uint_ids(self):
        recipes = {4294967295: ({i: 1 for i in range(1, 33)}, None, None)}
        recipes.update({200 + i: ({4294967295: 1}, i, (1, 1)) for i in range(1, 33)})
        result = self.build(recipes).recipe_material_plan(4294967295,
            producer_choices={str(i): 200 + i for i in range(1, 33)})
        self.assertEqual(len(result['crafting_steps']), 33)
        self.assertEqual(self.requirements(result), {4294967295: 32})

    def test_json_safe_arithmetic_rejects_product_sum_and_rounded_output_overflow(self):
        limit = 4294967295
        cases = [
            ({100: ({20: limit}, None, None), 200: ({30: limit}, 20, (1, 1))}, 1, {'20': 200}),
            ({100: ({20: 1, 30: 1}, None, None), 200: ({40: limit}, 20, (1, 1)),
              300: ({40: limit}, 30, (1, 1))}, 100000, {'20': 200, '30': 300}),
        ]
        # The second case is within range; aggregating large demands must stay exact.
        knowledge = self.build(cases[1][0])
        self.assertEqual(self.requirements(knowledge.recipe_material_plan(100, *cases[1][1:])), {40: 858993459000000})
        with self.assertRaisesRegex(ValueError, 'bounded integer'):
            self.build(cases[0][0]).recipe_material_plan(100, *cases[0][1:])
        # Each branch fits, but the two contributions to item 40 overflow when added.
        recipes = {100: ({20: 2000000, 30: 2000000}, None, None),
            200: ({40: limit}, 20, (1, 1)), 300: ({40: limit}, 30, (1, 1))}
        with self.assertRaisesRegex(ValueError, 'bounded integer'):
            self.build(recipes).recipe_material_plan(100, producer_choices={'20': 200, '30': 300})
        # Demand is safe, but rounding to a multiple of the chosen yield is not.
        recipes = {100: ({20: 2097152}, None, None), 200: ({30: limit}, 20, (1, 1)),
                   300: ({}, 30, (limit - 1, limit - 1))}
        with self.assertRaisesRegex(ValueError, 'bounded integer'):
            self.build(recipes).recipe_material_plan(100, producer_choices={'20': 200, '30': 300})

    def test_multiple_recorded_outputs_rejected_and_loaded_snapshot_immutable(self):
        original = self.build()
        # The public database schema supports multiple outputs, unlike the capture builder.
        with closing(sqlite3.connect(self.database)) as connection, connection:
            connection.execute('INSERT INTO edges VALUES (?,?,?,?)', (200, 'output', 30, '{"minimum":1,"maximum":1}'))
            manifest = json.loads(connection.execute('SELECT value FROM manifest').fetchone()[0])
            del manifest['content_sha256']
            records = connection.execute('SELECT * FROM records ORDER BY kind,id').fetchall()
            edges = connection.execute('SELECT * FROM edges ORDER BY recipe,relation,target').fetchall()
            manifest['content_sha256'] = local.content_digest(manifest, records, edges)
            connection.execute('UPDATE manifest SET value=?', (local.encoded(manifest),))
        self.assertEqual(self.requirements(original.recipe_material_plan(100, producer_choices={'20': 200})), {30: 4})
        with self.assertRaisesRegex(ValueError, 'exactly one matching output'):
            self.load().recipe_material_plan(100, producer_choices={'20': 200})

    def test_response_byte_cap_from_real_public_fields(self):
        equipment = {str(slot): {'item_id': 20, 'required_level': slot,
            'link': '|Hitem:20:0|h[Observed material]|h',
            'native_stats': {'ITEM_MOD_' + 'X' * 70 + str(i) + '_SHORT': i for i in range(128)}}
            for slot in range(1, 8)}
        knowledge = self.build(equipment=equipment)
        with self.assertRaisesRegex(ValueError, 'response exceeds byte limit'):
            knowledge.recipe_material_plan(100)

    def test_actual_cli_and_stdio_with_strict_integer_arguments(self):
        knowledge = self.build()
        before = self.database.read_bytes()
        environment = {**os.environ, 'PYTHONPATH': str(ROOT), 'TEMP': str(self.folder), 'TMP': str(self.folder)}
        command = [sys.executable, '-B', '-m', 'tools.forever_local', 'recipe-material-plan',
                   '--database', str(self.database), '--entity-id', '100']
        reply = subprocess.run([*command, '--crafts', '3', '--producer-choices-json', '{"20":200}'],
            cwd=ROOT, env=environment, capture_output=True, text=True, timeout=20, check=True)
        self.assertEqual(json.loads(reply.stdout), knowledge.recipe_material_plan(100, 3, {'20': 200}))
        for extra in (['--crafts', '0'], ['--crafts', '1.0'], ['--crafts', 'true'],
                ['--producer-choices-json', '{"20":200,"20":200}'],
                ['--producer-choices-json', '{"20":true}'], ['--producer-choices-json', 'null']):
            reply = subprocess.run([*command, *extra], cwd=ROOT, env=environment,
                capture_output=True, text=True, timeout=20)
            self.assertNotEqual(reply.returncode, 0); self.assertEqual(reply.stdout, '')

        async def exercise():
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
                                    if t.name == 'plan_local_forever_recipe_materials')
                        self.assertTrue(tool.annotations.readOnlyHint)
                        self.assertFalse(tool.annotations.openWorldHint)
                        self.assertFalse(tool.annotations.destructiveHint)
                        properties = tool.inputSchema['properties']
                        self.assertEqual(set(properties), {'recipe_id', 'crafts', 'producer_choices'})
                        self.assertEqual(properties['crafts']['maximum'], 100000)
                        result = await session.call_tool(tool.name, {'recipe_id': 100, 'crafts': 3,
                                                                     'producer_choices': {'20': 200}})
                        self.assertFalse(result.isError)
                        self.assertEqual(json.loads(result.content[0].text), knowledge.recipe_material_plan(100, 3, {'20': 200}))
                        for args in ({'recipe_id': True}, {'recipe_id': 100, 'crafts': 1.0},
                                {'recipe_id': 100, 'crafts': False}, {'recipe_id': 100, 'crafts': '1'},
                                {'recipe_id': 100, 'crafts': 100001},
                                {'recipe_id': 100, 'producer_choices': {'20': True}},
                                {'recipe_id': 100, 'producer_choices': {'20': 200.0}},
                                {'recipe_id': 100, 'producer_choices': {'020': 200}}):
                            self.assertTrue((await session.call_tool(tool.name, args)).isError)
        asyncio.run(asyncio.wait_for(exercise(), 30))
        self.assertEqual(before, self.database.read_bytes())


if __name__ == '__main__':
    unittest.main()
