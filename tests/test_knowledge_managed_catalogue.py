"""Knowledge reads exact pinned managed catalogues without changing their sources."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from tools import forever_knowledge as knowledge
from tools import reference_sync as reference
from tools import sync_forever_catalogue as catalogue_sync


class ManagedCatalogueTests(unittest.TestCase):
    def setUp(self):
        local = knowledge.ROOT / 'local-data'
        local.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix='knowledge-managed-test-', dir=local)
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.source = {name: [] for name, _ in knowledge.SOURCES.values()}
        self.source['items'] = [{'id': 118, 'name': 'Minor Healing Potion'}]
        self.data = json.dumps(self.source).encode()
        self.pin = {'provider': 'wowsims/forever', 'revision': 'a' * 40,
                    'sha256': hashlib.sha256(self.data).hexdigest()}
        for module in (knowledge, catalogue_sync):
            patcher = patch.object(module, 'PIN', self.pin)
            patcher.start()
            self.addCleanup(patcher.stop)
        self.original = self.base / 'catalogue.json'
        self.original.write_bytes(self.data)
        self.root = self.base / 'cache'
        self.cache = reference.ReferenceCache(self.root, reserve_bytes=0)
        self.cache.sync(self.pin, seed=self.original)

    def snapshot(self):
        return {path.relative_to(self.base).as_posix(): path.read_bytes()
                for path in self.base.rglob('*') if path.is_file()}

    def reject_unchanged(self, message, root=None):
        before = self.snapshot()
        with self.assertRaisesRegex(ValueError, message):
            knowledge.read_catalogue(root or self.root)
        self.assertEqual(self.snapshot(), before)

    def test_seeded_cache_and_original_json_return_identical_records_without_writes(self):
        before = self.snapshot()
        with patch.object(reference.ReferenceCache, 'sync', side_effect=AssertionError('Unexpected sync')), \
                patch.object(knowledge, 'load_model', side_effect=AssertionError('Unexpected model load')):
            self.assertEqual(knowledge.read_catalogue(self.root), self.source)
            self.assertEqual(knowledge.read_catalogue(self.original), self.source)
            self.assertEqual(knowledge.documents(self.root), knowledge.documents(self.original))
        self.assertEqual(self.snapshot(), before)

    def test_identical_bytes_at_another_revision_fail_closed(self):
        self.cache.sync({**self.pin, 'revision': 'b' * 40}, seed=self.original)
        self.reject_unchanged('does not match source pin')

    def test_wrong_provider_fails_closed_even_with_identical_bytes(self):
        root = self.base / 'other-provider'
        cache = reference.ReferenceCache(root, reserve_bytes=0)
        cache.sync({'provider': 'Questie/QuestieDB', 'revision': self.pin['revision'],
                    'files': {'assets/database/db.json': self.pin['sha256']}}, seed=self.original)
        self.reject_unchanged('does not match source pin', root)

    def test_valid_object_with_wrong_file_hash_fails_closed(self):
        self.original.write_bytes(self.data + b' ')
        other = {**self.pin, 'sha256': hashlib.sha256(self.original.read_bytes()).hexdigest()}
        self.cache.sync(other, seed=self.original)
        self.reject_unchanged('does not match source pin')

    def test_corrupt_object_fails_closed(self):
        (self.root / 'objects' / self.pin['sha256']).write_bytes(b'corrupt')
        self.reject_unchanged('Corrupt cached object')

    def test_invalid_marker_fails_closed(self):
        (self.root / reference.MARKER).write_text('{}')
        self.reject_unchanged('Invalid ownership marker')

    def test_missing_marker_and_empty_directory_do_not_initialise_a_cache(self):
        (self.root / reference.MARKER).unlink()
        self.reject_unchanged('ownership marker')
        empty = self.base / 'empty'
        empty.mkdir()
        self.reject_unchanged('ownership marker', empty)
        self.assertEqual(list(empty.iterdir()), [])
        missing = self.base / 'missing'
        with self.assertRaises(FileNotFoundError):
            knowledge.read_catalogue(missing)
        self.assertFalse(missing.exists())

    def test_original_json_still_checks_the_hash(self):
        self.original.write_bytes(self.data + b' ')
        self.reject_unchanged('checksum or size mismatch', self.original)

    def test_byte_cap_applies_to_both_sources_even_with_matching_hash(self):
        data = b' ' * knowledge.MAX_BYTES + self.data
        self.original.write_bytes(data)
        self.pin['sha256'] = hashlib.sha256(data).hexdigest()
        self.cache.sync(self.pin, seed=self.original)
        for path in (self.root, self.original):
            with self.subTest(path=path):
                self.reject_unchanged('checksum or size mismatch', path)

    def test_contended_cache_fails_and_released_cache_reads(self):
        before = self.snapshot()
        with self.cache.snapshot(self.pin):
            with self.assertRaisesRegex(ValueError, 'Cache is busy'):
                knowledge.read_catalogue(self.root)
        self.assertEqual(knowledge.read_catalogue(self.root), self.source)
        self.assertEqual(self.snapshot(), before)

    def test_lock_is_held_during_the_actual_bounded_read(self):
        original_open = Path.open
        target = self.root / 'objects' / self.pin['sha256']
        reads = []
        owner = self

        class Reader:
            def __init__(self, stream):
                self.stream = stream

            def __getattr__(self, name):
                return getattr(self.stream, name)

            def __enter__(self):
                return self

            def __exit__(self, *args):
                return self.stream.__exit__(*args)

            def read(self, size=-1):
                if size == knowledge.MAX_BYTES + 1:
                    reads.append(size)
                    with owner.assertRaisesRegex(ValueError, 'Cache is busy'):
                        with owner.cache.snapshot(owner.pin):
                            pass
                return self.stream.read(size)

        def open_path(path, *args, **kwargs):
            stream = original_open(path, *args, **kwargs)
            return Reader(stream) if path == target else stream

        before = self.snapshot()
        with patch.object(Path, 'open', open_path):
            self.assertEqual(knowledge.read_catalogue(self.root), self.source)
        self.assertEqual(reads, [knowledge.MAX_BYTES + 1])
        self.assertEqual(self.snapshot(), before)


if __name__ == '__main__':
    unittest.main()
