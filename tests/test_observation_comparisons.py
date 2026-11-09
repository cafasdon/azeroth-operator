"""Opt-in history comparisons report observations, not learning or resource events."""
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


class ObservationComparisonTests(unittest.IsolatedAsyncioTestCase):
    async def test_context_guards_numeric_deltas_bounds_and_unknowns(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        base = fixture()
        base['client'] = {'family': 'forever', 'variant': 'live', 'build': '70862', 'interface': 16001}
        base['sections']['profession_spells'] = {'coverage': 'reported_known_profession_spells',
            'profession_id': 164, 'entries': {'2': {}, '10': {}, '100': {}}}
        base['sections']['player_resources'] = {'coverage': 'readable_player_resources',
            'health': {'current': 4294967295, 'maximum': 4294967295},
            'primary_power': {'current': 10, 'maximum': 100, 'power_type': 0, 'units': 'unmodified'}}
        later = deepcopy(base)
        later['captured_at'] = '2025-01-02T00:00:00Z'
        later['sections']['profession_spells']['entries'] = {'2': {}, '20': {}}
        later['sections']['player_resources']['health'] = {'current': 0, 'maximum': 0}
        later['sections']['player_resources']['primary_power'].update(current=30, maximum=200)
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            old, new = store.ingest(base), store.ingest(later)
            env = {**os.environ, 'AZEROTH_DATA_DIR': folder}
            env.pop('AZEROTH_FOREVER_CATALOGUE', None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    async def compare(a=old, b=new, **options):
                        response = await session.call_tool('compare_snapshots', {'before_id': a, 'after_id': b, **options})
                        self.assertFalse(response.isError, response)
                        return json.loads(response.content[0].text)
                    ordinary = await compare()
                    self.assertNotIn('profession_spell_changes', ordinary)
                    self.assertNotIn('player_resource_changes', ordinary)
                    result = await compare(profession_spell_changes=True, player_resource_changes=True)
                    spells = result['profession_spell_changes']
                    self.assertEqual(spells['newly_reported_spell_ids'], [20])
                    self.assertEqual(spells['no_longer_reported_spell_ids'], [10, 100])
                    self.assertEqual(spells['before_reported_count'], 3)
                    self.assertEqual(spells['after_reported_count'], 2)
                    self.assertEqual(spells['shared_reported_spell_count'], 1)
                    self.assertEqual(spells['shared_reported_spell_ids'], [2])
                    self.assertIs(spells['shared_reported_truncated'], False)
                    self.assertFalse(spells['learning_verified'])
                    self.assertFalse(spells['complete_recipe_knowledge_verified'])
                    counters = result['player_resource_changes']
                    self.assertEqual(counters['health']['current_delta'], -4294967295)
                    self.assertEqual(counters['health']['maximum_delta'], -4294967295)
                    self.assertEqual(counters['primary_power']['current_delta'], 20)
                    self.assertEqual(counters['primary_power']['maximum_delta'], 100)
                    self.assertFalse(counters['primary_power_type_changed'])
                    self.assertFalse(counters['resource_events_verified'])
                    self.assertFalse(counters['readiness_verified'])
                    changed = deepcopy(later)
                    changed['sections']['profession_spells']['profession_id'] = 165
                    changed['sections']['player_resources']['primary_power']['power_type'] = 3
                    data = await compare(b=store.ingest(changed), profession_spell_changes=True, player_resource_changes=True)
                    self.assertIsNone(data['profession_spell_changes'])
                    self.assertIsNone(data['player_resource_changes']['primary_power'])
                    self.assertTrue(data['player_resource_changes']['primary_power_type_changed'])
                    self.assertIsNotNone(data['player_resource_changes']['health'])
                    for section, field in [('profession_spells', 'profession_spell_changes'),
                                           ('player_resources', 'player_resource_changes')]:
                        missing = deepcopy(later)
                        missing['sections'].pop(section)
                        data = await compare(b=store.ingest(missing), **{field: True})
                        self.assertIsNone(data[field])
                    partial = deepcopy(later)
                    partial['sections']['player_resources'].pop('health')
                    partial['sections']['player_resources'].pop('primary_power')
                    partial['sections']['player_resources']['health'] = {'current': 1, 'maximum': 1}
                    data = await compare(b=store.ingest(partial), player_resource_changes=True)
                    self.assertIsNone(data['player_resource_changes']['primary_power'])
                    self.assertIsNone(data['player_resource_changes']['primary_power_type_changed'])
                    many_before, many_after = deepcopy(base), deepcopy(later)
                    many_before['sections']['profession_spells']['entries'] = {str(i): {} for i in range(1, 52)}
                    many_after['sections']['profession_spells']['entries'] = {str(i): {} for i in range(100, 151)}
                    bounded = (await compare(store.ingest(many_before), store.ingest(many_after),
                                             profession_spell_changes=True))['profession_spell_changes']
                    self.assertEqual(bounded['newly_reported_spell_count'], 51)
                    self.assertEqual(bounded['no_longer_reported_spell_count'], 51)
                    self.assertEqual(bounded['newly_reported_spell_ids'], list(range(100, 150)))
                    self.assertEqual(bounded['no_longer_reported_spell_ids'], list(range(1, 51)))
                    self.assertTrue(bounded['newly_reported_truncated'])
                    self.assertTrue(bounded['no_longer_reported_truncated'])
                    many_after['sections']['profession_spells']['entries'].update(
                        many_before['sections']['profession_spells']['entries'])
                    shared = (await compare(store.ingest(many_before), store.ingest(many_after),
                                            profession_spell_changes=True))['profession_spell_changes']
                    self.assertEqual(shared['shared_reported_spell_count'], 51)
                    self.assertEqual(shared['shared_reported_spell_ids'], list(range(1, 51)))
                    self.assertIs(shared['shared_reported_truncated'], True)
                    for mismatch in ['character', 'build', 'variant']:
                        bad = deepcopy(later)
                        if mismatch == 'character':
                            bad['character']['name'] = 'PRIVATE_CHARACTER'
                        else:
                            bad['client'][mismatch] += '-other'
                        response = await session.call_tool('compare_snapshots', {'before_id': old,
                            'after_id': store.ingest(bad), 'profession_spell_changes': True, 'player_resource_changes': True})
                        self.assertTrue(response.isError)
                        self.assertNotIn('PRIVATE_CHARACTER', str(response))
                    response = await session.call_tool('compare_snapshots', {'before_id': new, 'after_id': old,
                        'profession_spell_changes': True, 'player_resource_changes': True})
                    self.assertTrue(response.isError)
                    unchanged = await compare(old, old, profession_spell_changes=True, player_resource_changes=True)
                    self.assertEqual(unchanged['profession_spell_changes']['newly_reported_spell_ids'], [])
                    self.assertEqual(unchanged['profession_spell_changes']['no_longer_reported_spell_ids'], [])
                    self.assertEqual(unchanged['player_resource_changes']['health']['current_delta'], 0)
                    retail = fixture()
                    retail_key = store.ingest(retail)
                    for flag in ['profession_spell_changes', 'player_resource_changes']:
                        response = await session.call_tool('compare_snapshots', {'before_id': retail_key,
                            'after_id': retail_key, flag: True})
                        self.assertTrue(response.isError)


if __name__ == '__main__':
    unittest.main()
