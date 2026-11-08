"""Bulk caching preserves the exact candidate pool and review mapping."""

from contextlib import closing
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import patch

from tools.run_forever_sim import compare_bulk
from tools.simulation_cache import SimulationCache


class BulkCacheTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.cache = SimulationCache(self.root / "cache")
        self.engine = self.root / "engine.exe"
        self.engine.write_bytes(b"synthetic bulk engine")
        self.sha = hashlib.sha256(self.engine.read_bytes()).hexdigest()
        baseline = {"raid": {"parties": [{"players": [{"name": "PrivateName",
            "equipment": {"items": [{"id": 1}]}}]}]},
            "encounter": {"duration": 45}, "simOptions": {"iterations": 1000, "randomSeed": "10"}}
        self.requests = [baseline, deepcopy(baseline), deepcopy(baseline)]
        for index, request in enumerate(self.requests, 1):
            request["raid"]["parties"][0]["players"][0]["equipment"]["items"][0]["id"] = index
        self.stages = [{"stage": "BulkSimStageHigh", "iterations": 10000, "durationSeconds": 1.5}]

    def launch(self, *args, **kwargs):
        payload = json.loads(kwargs["input"])
        metric = {"dpsMetrics": {"avg": 100, "stdev": 1}, "pairedErrorToBaseline": 0}
        result = {"baseline": {**metric, "candidateIndex": -1}, "topResults": [
            {**metric, "candidateIndex": row["index"]}
            for row in payload["candidates"][:payload["topResults"]]
        ], "stageMetrics": self.stages}
        kwargs["stdout"].write(json.dumps(result).encode())

    def compare(self, requests=None, top=1, cache=None):
        return compare_bulk(self.engine, self.sha, requests or self.requests, top,
                            cache=cache or self.cache)

    def test_reuse_preserves_duplicates_without_mutating_cache(self):
        requests = [self.requests[0], self.requests[1], deepcopy(self.requests[1]), deepcopy(self.requests[0])]
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch) as launch:
            cold = self.compare(requests)
            warm = self.compare(requests, cache=SimulationCache(self.cache.root))
            self.assertEqual(launch.call_count, 1)
            self.assertEqual(cold.pop("cache_status"), "miss")
            self.assertEqual(warm.pop("cache_status"), "hit")
            self.assertEqual(cold, warm)
            self.assertEqual([row["review"] for row in warm["results"]], [1, 4, 2, 3])
            warm["results"][0]["mean_dps"] = -1
            self.assertEqual(self.compare(requests)["results"][0]["mean_dps"], 100)

    def test_pool_order_settings_and_shortlist_size_have_separate_entries(self):
        cases = [(self.requests, 1), (self.requests, 2),
                 ([self.requests[0], self.requests[2], self.requests[1]], 1),
                 (self.requests[:2], 1), ([*self.requests, deepcopy(self.requests[1])], 1)]
        for field, value in (("duration", 90), ("seed", "11")):
            changed = deepcopy(self.requests)
            for row in changed:
                if field == "duration":
                    row["encounter"][field] = value
                else:
                    row["simOptions"]["randomSeed"] = value
            cases.append((changed, 1))
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch) as launch:
            for requests, top in cases:
                self.assertEqual(self.compare(requests, top)["cache_status"], "miss")
                self.assertEqual(self.compare(requests, top)["cache_status"], "hit")
            self.assertEqual(launch.call_count, len(cases))

    def test_invalid_context_and_changed_engine_still_rejected(self):
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch) as launch:
            self.compare()
            invalid = deepcopy(self.requests)
            invalid[1]["encounter"]["duration"] = 99
            with self.assertRaisesRegex(ValueError, "matching"):
                self.compare(invalid)
            self.engine.write_bytes(b"changed engine")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                self.compare()
            self.assertEqual(launch.call_count, 1)

    def test_unknown_stage_fields_are_not_persisted(self):
        self.stages[0]["private_label"] = "PrivateName"
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch) as launch:
            self.assertEqual(self.compare()["cache_status"], "miss")
            self.assertEqual(self.compare()["cache_status"], "miss")
            self.assertEqual(launch.call_count, 2)

    def test_large_integer_telemetry_preserves_result(self):
        self.stages[0]["iterations"] = 10**400
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch):
            cold = self.compare()
            warm = self.compare()
        self.assertEqual(cold["stage_metrics"], warm["stage_metrics"])
        self.assertEqual(warm["cache_status"], "hit")

    def test_corrupt_bulk_entry_recomputes(self):
        with patch("tools.run_forever_sim.subprocess.run", side_effect=self.launch) as launch:
            self.compare()
            with closing(sqlite3.connect(self.cache.root / "results.sqlite3")) as db, db:
                db.execute("UPDATE results SET digest='bad'")
            self.assertEqual(self.compare()["cache_status"], "miss")
            self.assertEqual(launch.call_count, 2)

    def test_all_duplicate_pool_never_reads_or_writes_cache(self):
        with patch.object(self.cache, "get") as get, patch.object(self.cache, "put") as put:
            self.assertIsNone(self.compare([self.requests[0], deepcopy(self.requests[0])]))
            get.assert_not_called()
            put.assert_not_called()


if __name__ == "__main__":
    unittest.main()
