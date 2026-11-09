"""Explicit owner-store character selection never substitutes a newer alt."""
import asyncio
from argparse import Namespace
from copy import deepcopy
import os
from pathlib import Path
import tempfile
from time import monotonic
import unittest
from unittest.mock import patch

from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from tools.evaluate_forever_owned_gear import selected_snapshot


class OwnedCharacterSelectionTests(unittest.TestCase):
    def test_real_mcp_explicit_identity_and_missing_target(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get('AZEROTH_TEST_BINARY') or root/'target/debug'/
            ('azeroth-operator.exe' if os.name == 'nt' else 'azeroth-operator')).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            store = SnapshotStore(Path(folder))
            value = fixture()
            value['client'] = {'family':'forever','variant':'live','build':'70862','interface':16001}
            expected = store.ingest(value)
            alt = deepcopy(value)
            alt['character']['name'] = 'NewerAlt'
            alt['captured_at'] = '2025-01-03T00:00:00Z'
            store.ingest(alt)
            args = Namespace(snapshot_id=None, character_name='example', character_realm='test', mcp=binary)
            with patch.dict(os.environ, {'AZEROTH_DATA_DIR':folder}):
                self.assertEqual(asyncio.run(selected_snapshot(args, monotonic()+15)),
                    (expected, 'explicit_saved_character'))
                args.character_name = 'Missing'
                with self.assertRaises(Exception):
                    asyncio.run(selected_snapshot(args, monotonic()+15))
            self.assertEqual(len(list(Path(folder).glob('*.json'))), 2)

    def test_invalid_selectors_and_expired_deadline_never_launch(self):
        for snapshot, name, realm in [(None,None,None),(None,'Example',None),
                (None,None,'Test'),(None,'','Test'),('a'*64,'Example','Test'),('bad',None,None)]:
            with self.subTest(snapshot=snapshot, name=name, realm=realm):
                with self.assertRaises(ValueError):
                    asyncio.run(selected_snapshot(Namespace(snapshot_id=snapshot,
                        character_name=name,character_realm=realm),monotonic()+10))
        with self.assertRaises(ValueError):
            asyncio.run(selected_snapshot(Namespace(snapshot_id=None,character_name='Example',
                character_realm='Test'),monotonic()-1))
        self.assertEqual(asyncio.run(selected_snapshot(Namespace(snapshot_id='a'*64),monotonic()+10)),
            ('a'*64,'explicit_saved_snapshot'))


if __name__ == '__main__':
    unittest.main()
