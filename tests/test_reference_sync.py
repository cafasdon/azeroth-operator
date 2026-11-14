"""Exercise the real flat cache with bounded streams and no network access."""
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from tools import reference_sync as reference


def lock(files, revision="a", provider="Questie/QuestieDB"):
    return {"provider": provider, "revision": revision * 40,
            "source_files": {name: hashlib.sha256(data).hexdigest() for name, data in files.items()}}


class ReferenceSyncTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="reference-sync-")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base / "cache"
        self.cache = reference.ReferenceCache(self.root, max_bytes=20000, reserve_bytes=0)
        self.files = {"data/a.json": b"one", "data/b.json": b"two"}
        self.calls = []

    def fetch(self, url):
        self.calls.append(url)
        self.assertTrue(url.startswith("https://raw.githubusercontent.com/Questie/QuestieDB/"))
        name = url.split("/", 6)[6]
        return io.BytesIO(self.files[name])

    def initial(self):
        return self.cache.sync(lock(self.files), fetch=self.fetch)

    def snapshot(self):
        return {p.relative_to(self.root).as_posix(): p.read_bytes()
                for p in self.root.rglob("*") if p.is_file()}

    def test_incremental_revisions_reuse_and_gc(self):
        first = self.initial()
        self.assertEqual(first["downloaded_files"], 2)
        self.calls.clear()
        self.files["data/b.json"] = b"changed"
        second = self.cache.sync(lock(self.files, "b"), fetch=self.fetch)
        self.assertEqual(len(self.calls), 1)
        self.assertTrue(self.calls[0].endswith("/data/b.json"))
        self.assertEqual(second["reused_files"], 1)
        self.assertEqual((second["removed_files"], second["removed_bytes"]), (1, 3))
        self.calls.clear()
        del self.files["data/a.json"]
        third = self.cache.sync(lock(self.files, "c"), fetch=self.fetch)
        self.assertEqual(self.calls, [])
        self.assertEqual((third["removed_files"], third["object_count"]), (1, 1))
        again = self.cache.sync(lock(self.files, "c"), fetch=self.fetch)
        self.assertEqual(self.calls, [])
        self.assertEqual(again["downloaded_bytes"], 0)
        self.assertEqual(self.cache.read_file("data/b.json"), b"changed")
        self.assertEqual(self.cache.query("data/b.json")["bytes"], 7)
        self.assertEqual(again["cache_bytes"], sum(len(v) for v in self.snapshot().values()))

    def test_duplicate_content_stores_and_downloads_one_object(self):
        self.files["data/b.json"] = b"one"
        result = self.initial()
        self.assertEqual((result["object_count"], result["downloaded_files"]), (1, 1))

    def test_failed_hash_keeps_previous_state_and_removes_staging(self):
        self.initial()
        before = self.snapshot()
        target = lock({"data/a.json": b"new-a", "data/b.json": b"new-b"}, "b")
        self.files = {"data/a.json": b"new-a", "data/b.json": b"WRONG"}
        with self.assertRaisesRegex(ValueError, "checksum"):
            self.cache.sync(target, fetch=self.fetch)
        self.assertEqual(self.snapshot(), before)

    def test_stream_quota_stops_before_write_and_preserves_active(self):
        self.initial()
        before = self.snapshot()
        chunks = []

        class Stream(io.BytesIO):
            def read(inner, size):
                chunks.append(size)
                return super().read(size)

        payload = b"x" * 100000
        with self.assertRaisesRegex(ValueError, "quota"):
            self.cache.sync(lock({"data/a.json": payload}, "b"), fetch=lambda _: Stream(payload))
        self.assertEqual(chunks, [reference.CHUNK])
        self.assertEqual(self.snapshot(), before)

    def test_transient_space_counts_old_new_and_metadata(self):
        self.files = {"data/a.json": b"a" * 11000}
        self.initial()
        before = self.snapshot()
        self.files = {"data/a.json": b"b" * 11000}
        with self.assertRaisesRegex(ValueError, "quota"):
            self.cache.sync(lock(self.files, "b"), fetch=self.fetch)
        self.assertEqual(self.snapshot(), before)

    def test_metadata_alone_cannot_exceed_quota(self):
        tiny = reference.ReferenceCache(self.base / "tiny", max_bytes=180, reserve_bytes=0)
        with self.assertRaisesRegex(ValueError, "quota"):
            tiny.sync(lock(self.files), fetch=self.fetch)
        self.assertEqual(self.calls, [])
        self.assertIsNone(tiny.status()["revision"])

    def test_free_disk_reserve_checked_before_metadata_and_each_chunk(self):
        self.initial()
        before = self.snapshot()
        usage = shutil.disk_usage(self.root)
        self.files = {"data/a.json": b"x" * 1000}
        for free in (0, 500):
            with self.subTest(free=free), patch.object(reference.shutil, "disk_usage", return_value=usage._replace(free=free)):
                with self.assertRaisesRegex(ValueError, "reserve"):
                    self.cache.sync(lock(self.files, "b"), fetch=self.fetch)
            self.assertEqual(self.snapshot(), before)

    def test_corruption_fails_closed_offline_and_on_sync(self):
        self.initial()
        blob = next((self.root / "objects").iterdir())
        blob.write_bytes(b"CORRUPT")
        before = self.snapshot()
        for operation in (self.cache.status, lambda: self.cache.query("data/a.json"),
                          lambda: self.cache.read_file("data/a.json"), lambda: self.initial()):
            with self.assertRaisesRegex(ValueError, "Corrupt"):
                operation()
        self.assertEqual(self.snapshot(), before)

    def test_unknown_files_and_directories_are_never_removed(self):
        self.initial()
        for parent, name in ((self.root, "private.json"), (self.root / "objects", "not-a-hash")):
            stranger = parent / name
            stranger.write_bytes(b"private")
            before = self.snapshot()
            with self.assertRaisesRegex(ValueError, "Unknown"):
                self.initial()
            self.assertEqual(self.snapshot(), before)
            stranger.unlink()
        directory = self.root / "unknown"
        directory.mkdir()
        with self.assertRaisesRegex(ValueError, "Unknown"):
            self.initial()
        self.assertTrue(directory.is_dir())

    def test_nonempty_unowned_directory_is_rejected(self):
        foreign = self.base / "foreign"
        foreign.mkdir()
        (foreign / "history.json").write_bytes(b"private")
        with self.assertRaises(ValueError):
            reference.ReferenceCache(foreign)
        self.assertEqual(list(foreign.iterdir()), [foreign / "history.json"])

    def test_new_nested_cache_creates_missing_parents(self):
        root = self.base / "local-data" / "reference-cache" / "world"
        self.assertFalse(root.parent.exists())
        cache = reference.ReferenceCache(root, reserve_bytes=0)
        cache.sync(lock(self.files), fetch=self.fetch)
        self.assertEqual(cache.read_file("data/a.json"), b"one")
        self.assertEqual(reference.ReferenceCache(root).status()["revision"], "a" * 40)

    def test_owned_pending_files_cleaned_without_old_patch_history(self):
        self.initial()
        (self.root / ".pending-blob").write_bytes(b"interrupted")
        (self.root / ".pending.json").write_bytes(b"interrupted")
        self.initial()
        self.assertEqual({p.name for p in self.root.iterdir()}, {reference.MARKER, "active.json", "objects", ".lock"})

    def test_symlink_rejection_and_outside_data_preserved(self):
        self.initial()
        outside = self.base / "outside"
        outside.write_bytes(b"private")
        linked = self.root / ".pending-blob"
        try:
            linked.symlink_to(outside)
        except OSError as error:
            self.skipTest(f"Host does not support unprivileged symlinks: {error}")
        with self.assertRaisesRegex(ValueError, "Linked"):
            self.initial()
        self.assertEqual(outside.read_bytes(), b"private")
        self.assertTrue(linked.is_symlink())
        linked.unlink()

    def test_seed_directory_and_single_catalogue_are_hash_checked(self):
        seed = self.base / "seed"
        (seed / "data").mkdir(parents=True)
        for name, content in self.files.items():
            (seed / name).write_bytes(content)
        result = self.cache.sync(lock(self.files), seed=seed, fetch=lambda _: self.fail("Network called"))
        self.assertEqual((result["seeded_files"], result["downloaded_files"]), (2, 0))
        catalogue = reference.ReferenceCache(self.base / "catalogue", reserve_bytes=0)
        data = b'{"items":[]}'
        seed_file = self.base / "db.json"
        seed_file.write_bytes(data)
        pin = {"provider": "wowsims/forever", "revision": "b" * 40, "sha256": hashlib.sha256(data).hexdigest()}
        catalogue.sync(pin, seed=seed_file)
        self.assertEqual(catalogue.read_file("assets/database/db.json"), data)
        pin["revision"] = "c" * 40
        pin["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "checksum"):
            catalogue.sync(pin, seed=seed_file)
        self.assertEqual(catalogue.status()["revision"], "b" * 40)

    def test_network_failure_and_promotion_failure_preserve_old_active(self):
        self.initial()
        before = self.snapshot()
        self.files["data/a.json"] = b"new"
        with self.assertRaises(OSError):
            self.cache.sync(lock(self.files, "b"), fetch=lambda _: (_ for _ in ()).throw(OSError("network")))
        self.assertEqual(self.snapshot(), before)
        replace = reference.os.replace

        def fail_manifest(src, dst):
            if Path(dst).name == "active.json":
                raise OSError("promotion failed")
            replace(src, dst)

        with patch.object(reference.os, "replace", side_effect=fail_manifest), self.assertRaises(OSError):
            self.cache.sync(lock(self.files, "b"), fetch=self.fetch)
        self.assertEqual(self.snapshot(), before)

    def test_invalid_provider_revision_path_digest_and_unknown_queries(self):
        self.initial()
        before = self.snapshot()
        for overrides in ({"provider": "attacker/repo"}, {"revision": "main"},
                          {"source_files": {"../secret": "a" * 64}},
                          {"source_files": {"a?url": "a" * 64}},
                          {"source_files": {"a": "bad"}}):
            with self.subTest(overrides=overrides), self.assertRaises(ValueError):
                self.cache.sync({**lock(self.files), **overrides}, fetch=self.fetch)
        for name in ("../reference-cache.json", "/data/a.json", "unknown", None, []):
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.cache.read_file(name)
        self.assertEqual(self.snapshot(), before)

    def test_root_limits_and_concurrent_writer_fail_closed(self):
        for path in (Path("relative"), Path(self.root.anchor), self.base / "x" / ".." / "cache"):
            with self.assertRaises(ValueError):
                reference.ReferenceCache(path)
        with self.assertRaisesRegex(ValueError, "limits"):
            reference.ReferenceCache(self.root, max_bytes=30000)
        with self.cache._guard():
            with self.assertRaises(ValueError):
                self.cache.status()

    def test_real_locks_parse_and_real_cli_status_query_read(self):
        project = Path(__file__).resolve().parents[1]
        for name in ("world", "catalogue"):
            self.assertTrue(reference.load_lock(project / f"clients/forever/{name}.lock.json")["files"])
        self.initial()
        command = [sys.executable, str(project / "tools/reference_sync.py")]
        result = subprocess.run([*command, "status", "--cache", str(self.root)], capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["revision"], "a" * 40)
        result = subprocess.run([*command, "query", "--cache", str(self.root), "--name", "data/a.json"], capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["bytes"], 3)
        result = subprocess.run([*command, "read-file", "--cache", str(self.root), "--name", "data/a.json"], capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, b"one")

    def test_redirects_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "redirect"):
            reference.NoRedirect().redirect_request(None, None, 302, "Found", {}, "https://evil.test/file")

    def test_marker_changes_and_oversized_manifest_preserve_old_active(self):
        self.initial()
        before = self.snapshot()
        files = {"a" * 230 + str(i): "0" * 64 for i in range(4096)}
        with self.assertRaisesRegex(ValueError, "manifest exceeds"):
            self.cache.sync({"provider": "Questie/QuestieDB", "revision": "b" * 40, "files": files})
        self.assertEqual(self.snapshot(), before)
        marker = self.root / reference.MARKER
        marker.unlink()
        with self.assertRaisesRegex(ValueError, "ownership"):
            self.initial()
        self.assertEqual((self.root / "active.json").read_bytes(), before["active.json"])
        self.assertEqual((self.root / ".lock").read_bytes(), b"\0")

    def test_os_lock_recovers_after_real_child_crash(self):
        self.initial()
        before = self.snapshot()
        child = "from tools.reference_sync import ReferenceCache; import os,sys\nwith ReferenceCache(sys.argv[1])._guard(): os._exit(23)"
        result = subprocess.run([sys.executable, "-c", child, str(self.root)],
                                capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 23, result.stderr)
        reopened = reference.ReferenceCache(self.root)
        self.assertEqual(reopened.status()["revision"], "a" * 40)
        self.assertEqual(reopened.sync(lock(self.files), fetch=self.fetch)["downloaded_files"], 0)
        self.assertEqual(self.snapshot(), before)

    def test_stable_lock_supports_open_reader_and_rejects_busy_writer(self):
        self.initial()
        lock_path = self.root / ".lock"
        inode = lock_path.stat().st_ino
        with lock_path.open("rb"):
            self.assertEqual(self.cache.status()["revision"], "a" * 40)
        with self.cache._guard():
            result = subprocess.run([sys.executable, "tools/reference_sync.py", "status", "--cache", str(self.root)],
                                    capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 1, result.stdout)
            self.assertIn("Cache is busy", result.stderr)
        self.assertEqual(lock_path.stat().st_ino, inode)
        self.assertEqual(lock_path.stat().st_size, 1)
        self.assertEqual(self.cache.status()["revision"], "a" * 40)

    def test_lock_rejects_oversize_or_hard_link_without_removal(self):
        self.initial()
        lock_path = self.root / ".lock"
        lock_path.write_bytes(b"too large")
        with self.assertRaisesRegex(ValueError, "lock file"):
            self.cache.status()
        self.assertEqual(lock_path.read_bytes(), b"too large")
        lock_path.write_bytes(b"\0")
        outside = self.base / "lock-link"
        os.link(lock_path, outside)
        with self.assertRaisesRegex(ValueError, "lock file"):
            self.cache.status()
        self.assertEqual(outside.read_bytes(), b"\0")
        self.assertTrue(lock_path.exists())

    def test_exact_noop_succeeds_without_duplicate_manifest_space(self):
        tight = reference.ReferenceCache(self.base / "tight", max_bytes=400, reserve_bytes=0)
        tight.sync(lock(self.files), fetch=self.fetch)
        current = tight.status()
        self.assertGreater(current["cache_bytes"] + len(reference.encode(reference.spec(lock(self.files)))), 400)
        self.calls.clear()
        usage = shutil.disk_usage(tight.root)
        with patch.object(reference.os, "replace", side_effect=AssertionError("No-op must not promote")), \
                patch.object(reference.shutil, "disk_usage", return_value=usage._replace(free=0)):
            result = tight.sync(lock(self.files), fetch=self.fetch)
        self.assertEqual(self.calls, [])
        self.assertEqual(result["downloaded_files"], 0)
        self.assertEqual(result["reused_files"], 2)
        self.assertEqual(result["cache_bytes"], current["cache_bytes"])
        self.assertFalse((tight.root / ".pending.json").exists())

    def test_orphan_gc_precedes_staging_and_keeps_requested_objects(self):
        cache = reference.ReferenceCache(self.base / "orphan-cache", max_bytes=1300, reserve_bytes=0)
        old, new = {"a": b"old"}, {"a": b"new"}
        cache.sync(lock(old), fetch=lambda _: io.BytesIO(b"old"))
        orphan = b"x" * 1000
        orphan_path = cache.root / "objects" / hashlib.sha256(orphan).hexdigest()
        orphan_path.write_bytes(orphan)
        desired_path = cache.root / "objects" / hashlib.sha256(b"new").hexdigest()
        desired_path.write_bytes(b"new")
        self.assertLessEqual(cache.status()["cache_bytes"], 1300)
        self.assertGreater(cache.status()["cache_bytes"] + len(reference.encode(reference.spec(lock(new, "b")))), 1300)
        result = cache.sync(lock(new, "b"), fetch=lambda _: self.fail("Requested blob must be reused"))
        self.assertEqual((result["removed_files"], result["removed_bytes"]), (2, 1003))
        self.assertEqual((result["reused_files"], result["downloaded_files"]), (1, 0))
        self.assertFalse(orphan_path.exists())
        self.assertTrue(desired_path.exists())
        self.assertEqual(cache.read_file("a"), b"new")

    def test_orphan_gc_then_failed_sync_preserves_old_active(self):
        self.initial()
        active = (self.root / "active.json").read_bytes()
        orphan = b"orphan"
        path = self.root / "objects" / hashlib.sha256(orphan).hexdigest()
        path.write_bytes(orphan)
        with self.assertRaisesRegex(ValueError, "checksum"):
            self.cache.sync(lock({"data/a.json": b"new"}, "b"), fetch=lambda _: io.BytesIO(b"bad"))
        self.assertFalse(path.exists())
        self.assertEqual((self.root / "active.json").read_bytes(), active)
        self.assertEqual(self.cache.read_file("data/a.json"), b"one")


if __name__ == "__main__":
    unittest.main()
