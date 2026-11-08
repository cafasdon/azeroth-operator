"""Real on-disk cache checks around the fixed simulation caller."""

from copy import deepcopy
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from tools.run_forever_sim import run, compare
from tools.simulation_cache import SimulationCache, MAX_ENTRIES, MAX_ENTRY_BYTES


class SimulationCacheTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.cache = SimulationCache(self.root / "cache")
        self.engine = self.root / "engine.exe"
        self.engine.write_bytes(b"synthetic engine")
        self.sha = hashlib.sha256(self.engine.read_bytes()).hexdigest()
        self.request = {
            "raid": {"parties": [{"players": [{"name": "PrivateName",
                "equipment": {"items": [{"id": 1}]}}]}]},
            "encounter": {"duration": 90},
            "simOptions": {"iterations": 3, "randomSeed": "10"},
        }

    def launch(self, argv, **kwargs):
        request = json.loads(Path(argv[argv.index("--infile") + 1]).read_text())
        count = request["simOptions"]["iterations"]
        output = {"iterationsDone": count, "raidMetrics": {"dps": {
            "avg": 100, "stdev": 2, "allValues": [99, 100, 101][:count],
        }}}
        Path(argv[argv.index("--outfile") + 1]).write_text(json.dumps(output))

    def test_reuse_across_instances_without_request_storage(self):
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch) as launch:
            first = run(self.engine, self.sha, self.request, cache=self.cache)
            second = run(self.engine, self.sha, self.request,
                         cache=SimulationCache(self.cache.root))
        self.assertEqual(launch.call_count, 1)
        self.assertEqual(first.pop("cache_status"), "miss")
        self.assertEqual(second.pop("cache_status"), "hit")
        self.assertEqual(first, second)
        self.assertNotIn(b"PrivateName", (self.cache.root / "results.sqlite3").read_bytes())

    def test_full_input_and_mode_keys(self):
        key = self.cache.key(self.sha, self.request, False, False, False)
        variants = []
        for field, value in (("iterations", 4), ("randomSeed", "11")):
            changed = deepcopy(self.request)
            changed["simOptions"][field] = value
            variants.append(changed)
        changed = deepcopy(self.request)
        changed["encounter"]["duration"] = 180
        variants.append(changed)
        changed = deepcopy(self.request)
        changed["raid"]["parties"][0]["players"][0]["equipment"]["items"][0]["id"] = 2
        variants.append(changed)
        for changed in variants:
            self.assertNotEqual(key, self.cache.key(self.sha, changed, False, False, False))
        for modes in ((True, False, False), (False, True, False), (False, False, True)):
            self.assertNotEqual(key, self.cache.key(self.sha, self.request, *modes))
        self.assertNotEqual(key, self.cache.key("1" * 64, self.request, False, False, False))

    def test_corruption_recomputes_and_engine_is_still_verified(self):
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch) as launch:
            run(self.engine, self.sha, self.request, cache=self.cache)
            with closing(sqlite3.connect(self.cache.root / "results.sqlite3")) as db, db:
                db.execute("UPDATE results SET body=?", (b'{}',))
            self.assertEqual(run(self.engine, self.sha, self.request, cache=self.cache)["cache_status"], "miss")
            self.assertEqual(launch.call_count, 2)
            self.engine.write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                run(self.engine, self.sha, self.request, cache=self.cache)
            self.assertEqual(launch.call_count, 2)

    def test_paired_comparison_does_not_mutate_cached_samples(self):
        alternative = deepcopy(self.request)
        alternative["raid"]["parties"][0]["players"][0]["equipment"]["items"][0]["id"] = 2
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch) as launch:
            first = compare(self.engine, self.sha, [self.request, alternative], paired=True, cache=self.cache)
            second = compare(self.engine, self.sha, [self.request, alternative], paired=True, cache=self.cache)
        self.assertEqual(launch.call_count, 2)
        self.assertEqual([row["cache_status"] for row in second["results"]], ["hit", "hit"])
        self.assertEqual(first["observed_leading_dps_pair"], second["observed_leading_dps_pair"])

    def test_owner_roots_and_limits(self):
        self.cache.put("x", {"mean_dps": 1})
        other = SimulationCache(self.root / "other")
        self.assertIsNone(other.get("x"))
        self.cache.put("oversized", {"data": "x" * MAX_ENTRY_BYTES})
        self.assertIsNone(self.cache.get("oversized"))
        for index in range(MAX_ENTRIES + 2):
            self.cache.put(str(index), {"mean_dps": index})
        with closing(sqlite3.connect(self.cache.root / "results.sqlite3")) as db, db:
            self.assertEqual(db.execute("SELECT COUNT(*) FROM results").fetchone()[0], MAX_ENTRIES)

    def test_unavailable_cache_is_optional(self):
        self.cache.root.write_bytes(b"not a directory")
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch):
            self.assertEqual(run(self.engine, self.sha, self.request, cache=self.cache)["mean_dps"], 100)

    def test_malformed_database_is_optional_and_closes(self):
        self.cache.root.mkdir()
        database = self.cache.root / "results.sqlite3"
        database.write_bytes(b"not sqlite")
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch):
            self.assertEqual(run(self.engine, self.sha, self.request, cache=self.cache)["mean_dps"], 100)
        database.unlink()

    @unittest.skipUnless(os.name == "nt", "Windows cache location")
    def test_missing_or_relative_os_cache_location_disables_cache(self):
        for value in (None, "relative"):
            with self.subTest(value=value), patch.dict(os.environ):
                if value is None:
                    os.environ.pop("LOCALAPPDATA", None)
                else:
                    os.environ["LOCALAPPDATA"] = value
                cache = SimulationCache.for_current_user()
                self.assertIsNone(cache.root)
                with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch):
                    self.assertEqual(run(self.engine, self.sha, self.request, cache=cache)["mean_dps"], 100)

    @unittest.skipUnless(os.name == "nt", "Windows DPAPI")
    def test_windows_body_encrypted_and_bound_to_request_key(self):
        key = self.cache.key(self.sha, self.request, False, False, False)
        summary = {"mean_dps": 987654321.12345}
        self.cache.put(key, summary)
        with closing(sqlite3.connect(self.cache.root / "results.sqlite3")) as db, db:
            body, digest = db.execute("SELECT body,digest FROM results WHERE key=?", (key,)).fetchone()
            self.assertNotIn(b"987654321", body)
            self.assertNotIn(b"mean_dps", body)
            db.execute("INSERT INTO results(key,body,digest) VALUES(?,?,?)", ("different", body, digest))
        self.assertEqual(self.cache.get(key), summary)
        self.assertIsNone(self.cache.get("different"))

    @unittest.skipUnless(os.name == "nt", "Windows ACLs")
    def test_windows_permissive_root_or_database_is_not_used(self):
        for target_kind in ("root", "database"):
            with self.subTest(target=target_kind):
                cache = SimulationCache(self.root / target_kind)
                key = cache.key(self.sha, self.request, False, False, False)
                cache.put(key, {"mean_dps": 123})
                target = cache.root if target_kind == "root" else cache.root / "results.sqlite3"
                subprocess.run(["icacls", str(target), "/grant", "*S-1-1-0:(R,W)"],
                               check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=10)
                self.assertIsNone(cache.get(key))
                with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch) as launch:
                    self.assertEqual(run(self.engine, self.sha, self.request, cache=cache)["mean_dps"], 100)
                    self.assertEqual(launch.call_count, 1)


if __name__ == "__main__":
    unittest.main()
