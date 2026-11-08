import copy
import tempfile
import unittest
from pathlib import Path
from shared.snapshots import SnapshotStore


def fixture():
    return {"schema_version": 1, "source": "fixture", "captured_at": "2025-01-01T00:00:00Z",
            "client": {"family": "retail", "variant": "live", "build": "123", "interface": 120105},
            "character": {"name": "Example", "realm": "Test"}, "sections": {"equipment": {}}}


class SnapshotTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.store = SnapshotStore(Path(self.temp.name))

    def test_roundtrip_dedup_and_freshness(self):
        value = fixture()
        key = self.store.ingest(value)
        self.assertEqual(key, self.store.ingest(copy.deepcopy(value)))
        self.assertEqual(value, self.store.read(key))
        result = self.store.inspect(key, "equipment")
        self.assertTrue(result["available"])
        self.assertFalse(result["live"])
        self.assertGreater(result["age_seconds"], 0)
        self.assertFalse(self.store.inspect(key, "bags")["available"])

    def test_action_bar_requires_at_least_one_observed_slot(self):
        sample = fixture()
        observation = {
            "coverage": "loaded_builtin_buttons_only",
            "observed_buttons": 2,
            "missing_buttons": 0,
            "slots": {},
        }
        sample["sections"]["action_bar_observation"] = observation
        with self.assertRaisesRegex(ValueError, "Invalid action bar coverage"):
            self.store.ingest(sample)
        observation["slots"] = {"1": {"empty": True}}
        observation["button_hotkeys"] = {
            "1": {"button": "ActionButton1", "slot": 1},
            "2": {"button": "OtherButton1", "slot": 1, "key1": "SHIFT-1"},
        }
        key = self.store.ingest(sample)
        self.assertEqual(self.store.read(key)["sections"]["action_bar_observation"], observation)
        observation["button_hotkeys"]["2"]["slot"] = 2
        with self.assertRaisesRegex(ValueError, "Invalid action button hotkeys"):
            self.store.ingest(sample)

    def test_traversal_and_tampering_rejected(self):
        with self.assertRaises(ValueError):
            self.store.read("../secret")
        key = self.store.ingest(fixture())
        (self.store.root / (key + ".json")).write_text("{}")
        with self.assertRaises(ValueError):
            self.store.read(key)

    def test_unknown_family_source_and_future_rejected(self):
        for field, value in (("source", "memory_reader"), ("captured_at", "2999-01-01T00:00:00Z")):
            snapshot = fixture()
            snapshot[field] = value
            with self.assertRaises(ValueError):
                self.store.ingest(snapshot)
        snapshot = fixture()
        snapshot["client"]["family"] = "unknown"
        with self.assertRaises(ValueError):
            self.store.ingest(snapshot)

    def test_tracked_recipes_require_forever_and_valid_recipe_maps(self):
        value = fixture()
        recipes = {"coverage": "player_tracked_recipes_only", "crafts": {"123": {"name": "Recipe", "learned": False}},
                   "recrafts": {"123": {}}}
        value["sections"]["tracked_recipes"] = recipes
        with self.assertRaises(ValueError):
            self.store.ingest(value)
        value["client"].update(family="forever", interface=16001)
        key = self.store.ingest(value)
        self.assertEqual(self.store.inspect(key, "tracked_recipes")["data"], recipes)
        for invalid in ("not an object", {**recipes, "coverage": "other_character"},
                        {**recipes, "crafts": {"0": {}}}, {**recipes, "crafts": {"1": {"learned": 1}}}):
            value["sections"]["tracked_recipes"] = invalid
            with self.assertRaises(ValueError):
                self.store.ingest(value)
