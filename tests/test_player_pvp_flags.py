"""Owner-local raw PvP flags retain every reported combination without inference."""
from copy import deepcopy
from pathlib import Path
import tempfile
import unittest

from shared.snapshots import SECTIONS, SnapshotStore, validate_snapshot
from test_snapshots import fixture


FIELDS = ('is_pvp', 'is_free_for_all', 'is_sanctuary')


def sample(flags=(False, False, False)):
    value = fixture()
    value['client'].update(family='forever', build='70862', interface=16001)
    value['sections']['player_pvp_flags'] = {
        'coverage': 'reported_player_pvp_flags', **dict(zip(FIELDS, flags)),
    }
    return value


class PlayerPvpFlagTests(unittest.TestCase):
    def setUp(self):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.store = SnapshotStore(Path(folder.name))

    def test_all_eight_raw_combinations_roundtrip_without_mutation(self):
        self.assertIn('player_pvp_flags', SECTIONS)
        keys = set()
        for mask in range(8):
            with self.subTest(mask=mask):
                value = sample(tuple(bool(mask & (1 << i)) for i in range(3)))
                original = deepcopy(value)
                self.assertIs(validate_snapshot(value), value)
                key = self.store.ingest(value)
                keys.add(key)
                path = self.store.root / (key + '.json')
                raw = path.read_bytes()
                self.assertEqual(self.store.read(key), original)
                result = self.store.inspect(key, 'player_pvp_flags')
                self.assertTrue(result['available'])
                self.assertFalse(result['live'])
                self.assertEqual(result['data'], original['sections']['player_pvp_flags'])
                for field in FIELDS:
                    self.assertIs(type(result['data'][field]), bool)
                self.assertEqual(value, original)
                result['data']['is_pvp'] = not result['data']['is_pvp']
                self.assertEqual(self.store.read(key), original)
                self.assertEqual(path.read_bytes(), raw)
        self.assertEqual(len(keys), 8)

    def test_missing_section_and_known_false_are_distinct(self):
        observed = sample()
        missing = deepcopy(observed)
        del missing['sections']['player_pvp_flags']
        missing_key = self.store.ingest(missing)
        false_key = self.store.ingest(observed)
        self.assertNotEqual(missing_key, false_key)
        result = self.store.inspect(missing_key, 'player_pvp_flags')
        self.assertFalse(result['available'])
        self.assertIsNone(result['data'])
        result = self.store.inspect(false_key, 'player_pvp_flags')
        self.assertTrue(result['available'])
        self.assertTrue(all(result['data'][field] is False for field in FIELDS))
        missing['unavailable'] = {'player_pvp_flags': 'Player PvP APIs unavailable'}
        unavailable_key = self.store.ingest(missing)
        self.assertFalse(self.store.inspect(unavailable_key, 'player_pvp_flags')['available'])

    def test_malformed_sections_and_fields_are_rejected(self):
        section = sample()['sections']['player_pvp_flags']
        invalid = [None, False, True, 0, 1, 1.0, 'flags', [], {},
                   {**section, 'coverage': 'inferred_pvp'},
                   {**section, 'coverage': None},
                   {**section, 'coverage': True},
                   {**section, 'unknown': False},
                   {**section, 'unknown': None}]
        for field in ('coverage', *FIELDS):
            missing = deepcopy(section)
            del missing[field]
            invalid.append(missing)
        for field in FIELDS:
            for primitive in (None, 0, 1, -1, 0.0, 1.0, 'false', 'true', [], {}):
                invalid.append({**section, field: primitive})
        for index, malformed in enumerate(invalid):
            with self.subTest(index=index, malformed=malformed):
                value = sample()
                value['sections']['player_pvp_flags'] = malformed
                original = deepcopy(value)
                with self.assertRaises(ValueError):
                    self.store.ingest(value)
                self.assertEqual(value, original)
        self.assertEqual(list(self.store.root.iterdir()), [])

    def test_other_client_families_refused(self):
        for family, interface in (('retail', 120105), ('classic', 11500)):
            with self.subTest(family=family):
                value = sample()
                value['client'].update(family=family, interface=interface)
                with self.assertRaisesRegex(ValueError, 'Invalid player PvP flag observation'):
                    self.store.ingest(value)

    def test_existing_source_and_unavailable_rules_apply(self):
        value = sample()
        value['source'] = 'memory_reader'
        with self.assertRaisesRegex(ValueError, 'Explicit snapshot source required'):
            self.store.ingest(value)
        value = sample()
        value['unavailable'] = {'player_pvp_flags': 'Unavailable'}
        with self.assertRaisesRegex(ValueError, 'Invalid unavailable section'):
            self.store.ingest(value)


if __name__ == '__main__':
    unittest.main()
