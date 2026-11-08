"""Bulk comparison preserves encounter and character context when deduplicating gear."""

from copy import deepcopy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

from tools.run_forever_sim import compare_bulk


def request():
    return {
        "raid": {"parties": [{"players": [{
            "equipment": {"items": [{"id": 1}]},
            "rotation": {"type": "APL"}, "talentsString": "123",
        }]}], "buffs": {"arcaneBrilliance": True}},
        "encounter": {"duration": 90, "targets": [{"level": 63}]},
        "simOptions": {"iterations": 1000, "randomSeed": "1234"},
        "type": "SimTypeIndividual",
    }


class BulkContextTests(unittest.TestCase):
    def test_rejects_context_changes_before_launch(self):
        changes = [
            lambda r: r["encounter"].update(duration=180),
            lambda r: r["encounter"]["targets"].append({"level": 60}),
            lambda r: r["raid"]["parties"][0]["players"][0].update(rotation={"type": "Auto"}),
            lambda r: r["raid"]["parties"][0]["players"][0].update(talentsString="456"),
            lambda r: r["raid"]["buffs"].update(arcaneBrilliance=False),
            lambda r: r["raid"]["buffs"].update(arcaneBrilliance=1),
            lambda r: r["simOptions"].update(randomSeed="5678"),
        ]
        for index, change in enumerate(changes):
            with self.subTest(change=index):
                baseline = request()
                candidate = deepcopy(baseline)
                change(candidate)
                with patch("tools.run_forever_sim.verify_engine"), patch("tools.run_forever_sim.subprocess.run") as launch:
                    with self.assertRaisesRegex(ValueError, "matching"):
                        compare_bulk(Path("unused"), "0" * 64, [baseline, candidate], 5)
                    launch.assert_not_called()

    def test_gear_only_changes_and_duplicates_preserve_mapping(self):
        baseline = request()
        candidate = deepcopy(baseline)
        candidate["raid"]["parties"][0]["players"][0]["equipment"]["items"][0]["id"] = 2
        inputs = [baseline, candidate, deepcopy(candidate), deepcopy(baseline)]
        before = deepcopy(inputs)
        captured = []

        def launch(*args, **kwargs):
            captured.append(json.loads(kwargs["input"]))
            metric = {"dpsMetrics": {"avg": 100, "stdev": 1}, "pairedErrorToBaseline": 0}
            result = {"baseline": {**metric, "candidateIndex": -1},
                      "topResults": [{**metric, "candidateIndex": 2}], "stageMetrics": []}
            kwargs["stdout"].write(json.dumps(result).encode())

        with patch("tools.run_forever_sim.verify_engine"), patch("tools.run_forever_sim.subprocess.run", side_effect=launch):
            result = compare_bulk(Path("unused"), "0" * 64, inputs, 5)
        self.assertEqual(inputs, before)
        self.assertEqual(len(captured), 1)
        self.assertEqual(len(captured[0]["candidates"]), 1)
        self.assertEqual(captured[0]["baseRequest"], baseline)
        self.assertEqual(result["unique_gear_sets"], 2)
        self.assertEqual([row["review"] for row in result["results"]], [1, 4, 2, 3])


if __name__ == "__main__":
    unittest.main()
