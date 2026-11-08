"""Cross-encounter reporting does not infer performance for unsimulated gear."""

from copy import deepcopy
from contextlib import redirect_stdout
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from tools.rank_forever_slot import encounter_sensitivity, main


def ranking(*scores):
    return {"baseline_review": 1,
            "results": [{"review": review, "mean_dps": score} for review, score in scores]}


class EncounterSensitivityTests(unittest.TestCase):
    def test_gain_reversal_and_different_winners(self):
        profiles = [ranking((1, 100), (2, 110), (3, 105)),
                    ranking((1, 200), (2, 180), (3, 220))]
        before = deepcopy(profiles)
        report = encounter_sensitivity(profiles, 3)
        self.assertEqual(profiles, before)
        baseline, second, third = report["results"]
        self.assertEqual(baseline["gain_pct_by_encounter"], [0, 0])
        self.assertEqual(second["gain_pct_by_encounter"], [10, -10])
        self.assertTrue(second["changes_sign_in_observed_encounters"])
        self.assertEqual(second["leading_point_estimate_encounters"], [1])
        self.assertEqual(third["leading_point_estimate_encounters"], [2])
        self.assertTrue(third["fully_compared"])
        self.assertFalse(third["changes_sign_in_observed_encounters"])

    def test_shortlist_omissions_are_unknown_including_never_returned(self):
        report = encounter_sensitivity([
            ranking((1, 100), (2, 110)), ranking((1, 100), (3, 120))], 4)
        second, third, absent = report["results"][1:]
        self.assertEqual(second["gain_pct_by_encounter"], [10, None])
        self.assertEqual(second["missing_encounters"], [2])
        self.assertFalse(second["fully_compared"])
        self.assertIsNone(second["changes_sign_in_observed_encounters"])
        self.assertEqual(third["gain_pct_by_encounter"], [None, 20])
        self.assertEqual(absent["gain_pct_by_encounter"], [None, None])
        self.assertEqual(absent["missing_encounters"], [1, 2])
        self.assertIsNone(absent["minimum_observed_gain_pct"])
        self.assertEqual(absent["leading_point_estimate_encounters"], [])

    def test_tied_point_estimates_and_zero_baseline(self):
        rows = encounter_sensitivity([
            ranking((1, 100), (2, 100)), ranking((1, 0), (2, 10))], 2)["results"]
        self.assertEqual(rows[0]["leading_point_estimate_encounters"], [1])
        self.assertEqual(rows[1]["leading_point_estimate_encounters"], [1, 2])
        self.assertEqual(rows[1]["gain_pct_by_encounter"], [0, None])
        self.assertEqual(rows[1]["missing_encounters"], [])
        self.assertFalse(rows[1]["fully_compared"])

    def test_extreme_ratio_is_unknown_not_nonfinite_json(self):
        rows = encounter_sensitivity([ranking((1, 1e-300), (2, 1e300))], 2)["results"]
        self.assertIsNone(rows[1]["gain_pct_by_encounter"][0])
        self.assertFalse(rows[1]["fully_compared"])

    def test_main_emits_dps_summary_and_uses_weighted_finalists(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / "local.exe"
            executable.touch()
            template = root / "template.json"
            template.write_text(json.dumps({"raid": {"parties": [{"players": [{
                "class": "ClassHunter", "race": "RaceOrc", "hunter": {}, "talentsString": "",
                "rotation": {"type": "TypeAPL", "priorityList": [{"action": {"castSpell": {}}}]},
            }]}]}}))
            encounters = []
            for index in range(2):
                path = root / f"encounter-{index}.json"
                path.write_text(json.dumps({"family": "forever", "section": "encounters",
                    "simulation_input": {"encounter": {"duration": 45 + index, "durationVariation": 0,
                        "executeProportion20": 0, "executeProportion35": 0, "targets": [{}]},
                        "simOptions": {"iterations": 1000, "randomSeed": "1"}}}))
                encounters.append(path)
            drafts = [([], {"client": {"family": "forever"}, "captured_character_level":60, "checks": {
                "simulator_equipment_spec": {"items": [{}] * 17}}})] * 3

            async def collect(*args, **kwargs):
                return drafts, [], {}, {"level":60}, None, None, {}

            argv = ["rank", "--mcp", str(executable), "--snapshot-id", "fixture", "--slot", "head",
                    "--engine", str(executable), "--engine-sha256", "a" * 64, "--template", str(template)]
            for path in encounters:
                argv += ["--encounter-lookup", str(path)]
            screened = [ranking((1, 100), (2, 110)), ranking((1, 100), (3, 120))]
            final = [ranking((1, 100), (2, 110), (3, 105)), ranking((1, 100), (2, 90), (3, 120))]
            for profiles in (screened, final):
                for profile in profiles:
                    for row in profile["results"]:
                        row["mean_dps_delta_from_baseline"] = row["mean_dps"] - 100
            with patch("sys.argv", argv), patch("tools.rank_forever_slot.collect", collect), \
                    patch("tools.rank_forever_slot.rank_encounters", return_value=screened), redirect_stdout(io.StringIO()) as output:
                main()
            self.assertFalse(json.loads(output.getvalue())["encounter_sensitivity"]["results"][1]["fully_compared"])
            weighted_argv = argv + ["--bulk-engine", str(executable), "--bulk-engine-sha256", "b" * 64,
                                    "--encounter-weight", "1", "--encounter-weight", "1"]
            with patch("sys.argv", weighted_argv), patch("tools.rank_forever_slot.collect", collect), \
                    patch("tools.rank_forever_slot.rank_encounters", side_effect=[screened, final]), redirect_stdout(io.StringIO()) as output:
                main()
            result = json.loads(output.getvalue())
            self.assertTrue(result["encounter_sensitivity"]["results"][1]["fully_compared"])
            self.assertTrue(result["encounter_sensitivity"]["results"][1]["changes_sign_in_observed_encounters"])
            with patch("sys.argv", argv + ["--rank-by", "hps"]), patch("tools.rank_forever_slot.collect", collect), \
                    patch("tools.rank_forever_slot.rank_encounters", return_value=screened), redirect_stdout(io.StringIO()) as output:
                main()
            self.assertNotIn("encounter_sensitivity", json.loads(output.getvalue()))


if __name__ == "__main__":
    unittest.main()
