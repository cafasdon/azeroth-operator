"""Offline world export from a genuine hash-object cache and a synthetic pin."""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from tools import forever_world as world
from tools import reference_sync as reference


class ManagedWorldTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='managed-world-')
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.source = self.base/'seed'
        lua = world.ROOT/'local-data/world-reference/pinned-source/tools/lua-binary/lua.exe'
        self.assertTrue(lua.is_file(), 'Explicit pinned Lua sync required; tests never download')
        self.files = {
            'tools/lua-binary/lua.exe': lua.read_bytes(),
            'src/config.lua': b'return {contractVersion=1,flavorByName={Forever=1},'
                              b'entityTypes={{name="Quest"}}}',
            'generator/flavor.lua': b'return {load=function() return {Quest={entities={[7]={[1]="Test"}},'
                                    b'meta={names={[1]="name"},entity="Quest"}}},{applied=0,derived=0} end}',
        }
        self.output = (b'{"contract":1,"derived_passes":0,"static_corrections":0}\r\n'
                       b'{"fields":{"name":"Test"},"id":7,"kind":"quest"}\r\n')
        self.pin = {'provider': 'Questie/QuestieDB', 'revision': 'a'*40,
                    'source_files': {name: hashlib.sha256(data).hexdigest()
                                     for name, data in self.files.items()},
                    'cache_sha256': hashlib.sha256(self.output).hexdigest(),
                    'counts': {'quest': 1},
                    'generation': {'contract': 1, 'derived_passes': 0, 'static_corrections': 0}}
        for name, data in self.files.items():
            path = self.source/name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        self.cache = reference.ReferenceCache(self.base/'managed', max_bytes=4*1024*1024, reserve_bytes=0)
        self.cache.sync(self.pin, seed=self.source, fetch=lambda _: self.fail('Network used'))
        self.patch = patch.object(world, 'PIN', self.pin)
        self.patch.start()
        self.addCleanup(self.patch.stop)

    def cache_files(self):
        return {p.relative_to(self.cache.root).as_posix(): p.read_bytes()
                for p in self.cache.root.rglob('*') if p.is_file()}

    def test_managed_build_uses_real_exporter_offline_without_source_cache_writes(self):
        before = self.cache_files()
        target = self.base/'output/world.jsonl'
        with patch('socket.socket.connect', side_effect=AssertionError('Network used')):
            self.assertEqual(world.build(self.cache.root, target)['status'], 'built')
        self.assertEqual(target.read_bytes(), self.output)
        self.assertEqual(world.World(target).get('quest', 7)['record']['name'], 'Test')
        self.assertEqual(self.cache_files(), before)
        self.assertEqual(set(target.parent.iterdir()), {target})
        self.assertEqual(world.build(self.base/'absent', target)['status'], 'unchanged')

    def test_original_source_directory_still_builds_same_bytes(self):
        target = self.base/'original.jsonl'
        self.assertEqual(world.build(self.source, target)['status'], 'built')
        self.assertEqual(target.read_bytes(), self.output)

    def test_exact_pin_provider_revision_and_file_map_fail_before_export(self):
        mutations = [
            {'provider': 'wowsims/forever', 'source_files': {'assets/database/db.json': next(iter(self.pin['source_files'].values()))}},
            {'revision': 'b'*40},
            {'source_files': dict(list(self.pin['source_files'].items())[1:])},
            {'source_files': {**self.pin['source_files'], 'extra.lua': 'b'*64}},
            {'source_files': {**self.pin['source_files'], 'src/config.lua': 'b'*64}},
        ]
        before = self.cache_files()
        for change in mutations:
            with self.subTest(change=change), patch.object(world, 'PIN', {**self.pin, **change}), \
                    patch.object(world.subprocess, 'run') as launch:
                with self.assertRaisesRegex(ValueError, 'source pin'):
                    world.build(self.cache.root, self.base/'must-not-exist.jsonl')
                launch.assert_not_called()
        self.assertFalse((self.base/'must-not-exist.jsonl').exists())
        self.assertEqual(self.cache_files(), before)
        self.assertEqual(self.cache.status()['revision'], self.pin['revision'])

    def test_corruption_fails_before_export_without_repair(self):
        blob = self.cache.root/'objects'/self.pin['source_files']['src/config.lua']
        blob.write_bytes(b'corrupt')
        before = self.cache_files()
        with patch.object(world.subprocess, 'run') as launch:
            with self.assertRaisesRegex(ValueError, 'Corrupt'):
                world.build(self.cache.root, self.base/'must-not-exist.jsonl')
            launch.assert_not_called()
        self.assertEqual(self.cache_files(), before)
        self.assertFalse((self.base/'must-not-exist.jsonl').exists())

    def test_total_source_bound_stops_before_export_and_releases_lock(self):
        before = self.cache_files()
        with patch.object(world, 'MAX_BYTES', sum(map(len, self.files.values())) - 1), \
                patch.object(world.subprocess, 'run') as launch:
            with self.assertRaisesRegex(ValueError, 'byte limit'):
                world.build(self.cache.root, self.base/'must-not-exist.jsonl')
            launch.assert_not_called()
        self.assertEqual(self.cache_files(), before)
        self.assertEqual(self.cache.status()['revision'], self.pin['revision'])

    def test_one_snapshot_blocks_real_sync_between_source_reads(self):
        pin_path = self.base/'pin.json'
        changed_pin = deepcopy(self.pin)
        changed_pin['revision'] = 'b'*40
        pin_path.write_text(json.dumps(changed_pin))
        before = self.cache_files()
        reads = []
        original = world.bounded_read

        def read(path, limit):
            data = original(path, limit)
            reads.append(path)
            if len(reads) == 1:
                child = subprocess.run([sys.executable, '-m', 'tools.reference_sync', 'sync',
                    '--cache', str(self.cache.root), '--lock', str(pin_path), '--seed', str(self.source)],
                    cwd=world.ROOT, capture_output=True, text=True, timeout=15)
                self.assertEqual(child.returncode, 1, child.stdout)
                self.assertIn('Cache is busy', child.stderr)
            return data

        staged = self.base/'stage'
        with patch.object(world, 'bounded_read', side_effect=read):
            self.assertEqual(world.verify_source(self.cache.root, staged), sum(map(len, self.files.values())))
        self.assertEqual(len(reads), len(self.files))
        self.assertEqual({name: (staged/name).read_bytes() for name in self.files}, self.files)
        self.assertEqual(self.cache_files(), before)
        self.assertEqual(self.cache.sync(changed_pin, seed=self.source)['revision'], 'b'*40)

    def test_invalid_marker_never_initialises_a_new_cache(self):
        source = self.base/'malformed'
        source.mkdir()
        (source/reference.MARKER).write_bytes(b'{}')
        before = {p.name: p.read_bytes() for p in source.iterdir()}
        with self.assertRaises(ValueError):
            world.verify_source(source)
        self.assertEqual({p.name: p.read_bytes() for p in source.iterdir()}, before)


if __name__ == '__main__':
    unittest.main()
