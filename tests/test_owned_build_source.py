"""Owner-bound saved selection through the real export reader and private index."""
from copy import deepcopy
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest

from shared.snapshots import SnapshotStore
from tools.account_inventory import AccountInventory
from tools.owned_build_source import OwnedBuildSource

ROOT = Path(__file__).resolve().parents[1]


class OwnedBuildSourceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=ROOT / 'local-data')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.store = SnapshotStore(self.root / 'snapshots')
        self.database = self.root / 'inventory.sqlite'
        self.inventory = AccountInventory(self.database)
        self.export = self.root / 'AzerothOperator.lua'
        self.client = dict(family='forever', variant='live', build='70862', interface=16001)
        self.now = datetime.now(timezone.utc).replace(microsecond=0)
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or ROOT / 'target/debug' /
                      ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator'))
        self.config = dict(reader=str(binary.resolve(strict=True)), snapshots=str(self.store.root),
                           active_export=str(self.export), inventory_database=str(self.database),
                           account_key='accountA', client=self.client)

    def sample(self, name='Alice', realm='Test', minutes=10):
        return dict(schema_version=1, source='addon',
                    captured_at=(self.now - timedelta(minutes=minutes)).isoformat().replace('+00:00', 'Z'),
                    client=deepcopy(self.client), character=dict(name=name, realm=realm),
                    sections=dict(stats=dict(level=22, client_version='synthetic',
                                             **{'class': 'SHAMAN', 'race': 'DRAENEI'}),
                                  equipment={'1': dict(item_id=10)}, bags={}))

    def ingest(self, value, account='accountA'):
        identifier = self.store.ingest(value)
        self.inventory.ingest(self.store.root, account, identifier)
        return identifier

    def save_export(self, value):
        payload = json.dumps(value, separators=(',', ':')).encode().hex()
        self.export.write_text('AzerothOperatorExport = "' + payload + '"\n'
                               'AzerothOperatorOperations = nil\n', encoding='utf-8')

    def source(self):
        return OwnedBuildSource(deepcopy(self.config))

    def assert_selected(self, result, identifier, basis, value):
        self.assertEqual(result['snapshot_id'], identifier)
        self.assertEqual(result['snapshot'], value)
        self.assertEqual(result['selection_basis'], basis)
        self.assertFalse(result['live'])
        self.assertFalse(result['active_character_verified'])
        self.assertGreaterEqual(result['age_seconds'], 0)

    def test_saved_export_selects_its_character_despite_newer_alt(self):
        value = self.sample(minutes=60)
        identifier = self.ingest(value)
        self.save_export(value)
        self.ingest(self.sample('Bob', minutes=1))
        self.assert_selected(self.source().select(), identifier, 'saved_active_export', value)

    def test_equivalent_utc_timestamp_survives_reader_normalisation(self):
        value = self.sample()
        value['captured_at'] = value['captured_at'].replace('Z', '+00:00')
        identifier = self.ingest(value)
        self.save_export(value)
        self.assert_selected(self.source().select(), identifier, 'saved_active_export', value)

    def test_empty_unavailable_map_survives_reader_omission(self):
        value = self.sample()
        value['unavailable'] = {}
        identifier = self.ingest(value)
        self.save_export(value)
        self.assert_selected(self.source().select(), identifier, 'saved_active_export', value)

    def test_explicit_case_insensitive_character_overrides_missing_or_invalid_export(self):
        value = self.sample('Bob')
        identifier = self.ingest(value)
        for export_bytes in (None, b'AzerothOperatorExport = os.execute("bad")'):
            with self.subTest(export_bytes=export_bytes):
                if export_bytes is None:
                    self.export.unlink(missing_ok=True)
                else:
                    self.export.write_bytes(export_bytes)
                self.assert_selected(self.source().select(dict(name='bOB', realm='tEST')),
                                     identifier, 'explicit_saved_character', value)

    def test_account_and_realm_partitions_do_not_supply_fallback(self):
        original = self.sample()
        identifier = self.ingest(original)
        self.save_export(original)
        self.ingest(self.sample('Alice', 'Elsewhere', 5))
        self.ingest(self.sample('Foreign', minutes=1), account='accountB')
        source = self.source()
        self.assert_selected(source.select(), identifier, 'saved_active_export', original)
        for identity in (dict(name='Foreign', realm='Test'), dict(name='Alice', realm='Absent')):
            with self.subTest(identity=identity), self.assertRaises(ValueError):
                source.select(identity)
        rows = source.list_characters()['characters']
        self.assertEqual({(r['character']['name'], r['character']['realm']) for r in rows},
                         {('Alice', 'Test'), ('Alice', 'Elsewhere')})

    def test_list_filters_exact_client_including_build(self):
        self.ingest(self.sample())
        incompatible = self.sample('WrongBuild')
        incompatible['client']['build'] = '70863'
        self.ingest(incompatible)
        rows = self.source().list_characters()['characters']
        self.assertEqual([r['character']['name'] for r in rows], ['Alice'])
        with self.assertRaises(ValueError):
            self.source().select(dict(name='WrongBuild', realm='Test'))

    def test_default_refuses_stale_same_character_export(self):
        old = self.sample(minutes=20)
        self.ingest(old)
        self.save_export(old)
        self.ingest(self.sample(minutes=10))
        with self.assertRaises(ValueError):
            self.source().select()

    def test_default_refuses_same_identity_payload_change(self):
        value = self.sample()
        self.ingest(value)
        changed = deepcopy(value)
        changed['sections']['equipment']['1']['item_id'] = 11
        self.save_export(changed)
        with self.assertRaises(ValueError):
            self.source().select()

    def test_client_mismatch_export_never_falls_back(self):
        value = self.sample()
        self.ingest(value)
        value['client']['build'] = '70863'
        self.save_export(value)
        with self.assertRaises(ValueError):
            self.source().select()

    def test_deleted_or_tampered_snapshot_refuses_both_selection_routes(self):
        value = self.sample()
        identifier = self.ingest(value)
        self.save_export(value)
        path = self.store.root / (identifier + '.json')
        for replacement in (None, b'{}'):
            with self.subTest(replacement=replacement):
                if replacement is None:
                    path.unlink(missing_ok=True)
                else:
                    path.write_bytes(replacement)
                for explicit in (None, dict(name='Alice', realm='Test')):
                    with self.subTest(explicit=explicit), self.assertRaises(ValueError):
                        self.source().select(explicit)

    def test_future_export_refuses_without_using_valid_index_row(self):
        value = self.sample()
        self.ingest(value)
        value['captured_at'] = (self.now + timedelta(minutes=10)).isoformat()
        self.save_export(value)
        with self.assertRaises(ValueError):
            self.source().select()

    def test_missing_sections_stay_unknown_and_reads_do_not_write(self):
        value = self.sample()
        identifier = self.ingest(value)
        self.save_export(value)
        before = {p.relative_to(self.root): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in self.root.rglob('*') if p.is_file()}
        source = self.source()
        result = source.select()
        self.assert_selected(result, identifier, 'saved_active_export', value)
        self.assertNotIn('character_bank', result['snapshot']['sections'])
        row = source.list_characters()['characters'][0]
        self.assertFalse(row['storage']['character_bank']['observed'])
        self.assertIsNone(row['storage']['character_bank']['quantity'])
        after = {p.relative_to(self.root): hashlib.sha256(p.read_bytes()).hexdigest()
                 for p in self.root.rglob('*') if p.is_file()}
        self.assertEqual(before, after)

    def test_invalid_selectors_and_extra_config_fields_refuse(self):
        self.ingest(self.sample())
        for character in ({}, {'name': 'Alice'}, {'name': 'Alice', 'realm': 'Test', 'account': 'B'},
                          {'name': '', 'realm': 'Test'}):
            with self.subTest(character=character), self.assertRaises(ValueError):
                self.source().select(character)
        self.config['unexpected'] = 'forbidden'
        with self.assertRaises(ValueError):
            self.source()


if __name__ == '__main__':
    unittest.main()
