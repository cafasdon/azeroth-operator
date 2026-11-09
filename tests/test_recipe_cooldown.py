"""Exercise selected-recipe cooldown searches through the real Rust MCP."""

import json
import copy
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class RecipeCooldownTests(unittest.IsolatedAsyncioTestCase):
    async def test_selected_recipe_comparison(self):
        binary = Path(__file__).resolve().parents[1] / "target/debug" / (
            "azeroth-operator.exe" if os.name == "nt" else "azeroth-operator")
        binary = Path(os.environ.get("AZEROTH_TEST_BINARY", binary)).resolve(strict=True)
        with tempfile.TemporaryDirectory() as directory:
            store = SnapshotStore(Path(directory))
            before = fixture()
            before["client"] = {"family": "forever", "variant": "live", "build": "70862", "interface": 16001}
            before["sections"]["selected_recipe"] = {
                "recipe_id": 123, "name": "Recipe", "learned": False,
                "requirements": "fixed_items", "reagents": {"744": 1},
                "cooldown": {"remaining_seconds": 30, "charges": 0, "max_charges": 2},
            }
            before["sections"]["bags"] = {"0":{"1":{"item_id":744,"count":1}}}
            before_id = store.ingest(before)
            after = copy.deepcopy(before)
            after["captured_at"] = "2025-01-02T00:00:00Z"
            recipe = after["sections"]["selected_recipe"]
            recipe["learned"] = True
            recipe["reagents"]["744"] = 2
            recipe["cooldown"].update(remaining_seconds=20, charges=2)
            after["sections"]["bags"]["0"]["1"]["count"] = 4
            after_id = store.ingest(after)
            renamed = copy.deepcopy(after)
            renamed["sections"]["selected_recipe"]["name"] = "Localised recipe"
            renamed_id = store.ingest(renamed)
            changed = copy.deepcopy(after)
            changed["sections"]["selected_recipe"]["recipe_id"] = 456
            changed_id = store.ingest(changed)
            unknown = copy.deepcopy(after)
            unknown["sections"]["selected_recipe"].pop("cooldown")
            unknown_id = store.ingest(unknown)
            missing = copy.deepcopy(after)
            missing["sections"].pop("selected_recipe")
            missing_id = store.ingest(missing)
            retail_id = store.ingest(fixture())
            env = {**os.environ, "AZEROTH_DATA_DIR": directory}
            env.pop("AZEROTH_FOREVER_CATALOGUE", None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()

                    async def compare(old, new, enabled=True):
                        response = await session.call_tool("compare_snapshots", {
                            "before_id": old, "after_id": new, "selected_recipe_changes": enabled})
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)

                    data = (await compare(before_id, after_id))["selected_recipe_changes"]
                    self.assertTrue(data["same_recipe"])
                    self.assertEqual(data["observed_charge_delta"], 2)
                    self.assertEqual(data["changed_observations"]["learned"], {"before": False, "after": True})
                    self.assertEqual(data["changed_observations"]["cooldown_state"], {"before": "waiting", "after": "available"})
                    self.assertEqual(data["changed_observations"]["reagents"], {"before": {"744": 1}, "after": {"744": 2}})
                    self.assertNotIn("selected_recipe_changes", await compare(before_id, after_id, False))
                    self.assertEqual((await compare(after_id, renamed_id))["selected_recipe_changes"]["changed_observations"], {})
                    other = (await compare(before_id, changed_id))["selected_recipe_changes"]
                    self.assertFalse(other["same_recipe"])
                    self.assertIsNone(other["changed_observations"])
                    self.assertIsNone(other["observed_charge_delta"])
                    lost = (await compare(after_id, unknown_id))["selected_recipe_changes"]
                    self.assertTrue(lost["same_recipe"])
                    self.assertIsNone(lost["observed_charge_delta"])
                    self.assertEqual(lost["after"]["cooldown_state"], "unknown")
                    for old, new in ((before_id, missing_id), (missing_id, missing_id)):
                        absent = (await compare(old, new))["selected_recipe_changes"]
                        self.assertIsNone(absent["same_recipe"])
                        self.assertIsNone(absent["changed_observations"])
                    async def preparation(new, crafts=2, error=False):
                        response = await session.call_tool("compare_snapshots", {
                            "before_id":before_id, "after_id":new,
                            "selected_recipe_preparation_crafts":crafts,
                        })
                        self.assertEqual(bool(response.isError), error, response)
                        if not error:
                            return json.loads(response.content[0].text)["selected_recipe_preparation"]

                    prep = await preparation(after_id)
                    self.assertEqual(prep["requested_crafts"], 2)
                    self.assertIs(prep["same_recipe"], True)
                    for side, available in [("before", False), ("after", True)]:
                        checks = prep[side]["checks"]
                        self.assertIs(checks["bags_cover_fixed_reagents"], available)
                        self.assertIs(checks["observed_charges_cover_request"], available)
                        self.assertIsNone(checks["observed_conditions_met"])
                        self.assertEqual(prep[side]["unobserved_storage"], ["character_bank","account_bank"])
                    self.assertIs(prep["craftability_verified"], False)
                    self.assertIs((await preparation(changed_id))["same_recipe"], False)
                    self.assertIsNone((await preparation(missing_id))["after"])
                    self.assertNotIn("selected_recipe_preparation", await compare(before_id, after_id))
                    await preparation(missing_id, 0, error=True)
                    await preparation(after_id, 1001, error=True)
                    rejected = await session.call_tool("compare_snapshots", {
                        "before_id": retail_id, "after_id": retail_id, "selected_recipe_changes": True})
                    self.assertTrue(rejected.isError)

    async def test_saved_cooldowns_and_newest_capture(self):
        binary = Path(__file__).resolve().parents[1] / "target/debug" / (
            "azeroth-operator.exe" if os.name == "nt" else "azeroth-operator")
        binary = Path(os.environ.get("AZEROTH_TEST_BINARY", binary)).resolve(strict=True)
        with tempfile.TemporaryDirectory() as directory:
            store = SnapshotStore(Path(directory))
            keys = {}
            cases = {
                "Recharging": ({"remaining_seconds": 30, "charges": 1, "max_charges": 2}, "available"),
                "Exhausted": ({"remaining_seconds": 0, "charges": 0, "max_charges": 2}, "waiting"),
                "NoTimer": ({"remaining_seconds": 0}, "available"),
                "Timer": ({"remaining_seconds": 30}, "waiting"),
                "Unknown": (None, "unknown"),
            }
            for name, (cooldown, state) in cases.items():
                snapshot = fixture()
                snapshot["client"] = {"family": "forever", "variant": "live", "build": "70862", "interface": 16001}
                snapshot["character"]["name"] = name
                snapshot["sections"]["selected_recipe"] = {
                    "recipe_id": 123, "name": "Synthetic recipe", "learned": True,
                    "requirements": "fixed_items", "reagents": {},
                }
                if cooldown is not None:
                    snapshot["sections"]["selected_recipe"]["cooldown"] = cooldown
                keys[name] = store.ingest(snapshot)
            snapshot["character"]["name"] = "Missing"
            snapshot["sections"].pop("selected_recipe")
            store.ingest(snapshot)
            snapshot["character"]["name"] = "NoTimer"
            snapshot["captured_at"] = "2025-01-02T00:00:00Z"
            store.ingest(snapshot)
            env = {**os.environ, "AZEROTH_DATA_DIR": directory}
            env.pop("AZEROTH_FOREVER_CATALOGUE", None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    tools = await session.list_tools()
                    listing = next(tool for tool in tools.tools if tool.name == "list_snapshots")
                    self.assertIn("recipe_cooldown_state", listing.inputSchema["properties"])
                    for state in ("available", "waiting", "unknown"):
                        result = await session.call_tool("list_snapshots", {
                            "family": "forever", "recipe_id": 123, "recipe_cooldown_state": state,
                        })
                        self.assertFalse(result.isError)
                        rows = json.loads(result.content[0].text)["snapshots"]
                        expected = {keys[name] for name, (_, observed) in cases.items() if observed == state}
                        self.assertEqual({row["snapshot_id"] for row in rows}, expected)
                        for row in rows:
                            self.assertEqual(row["selected_recipe"]["cooldown_state"], state)
                    result = await session.call_tool("list_snapshots", {
                        "family": "forever", "recipe_cooldown_state": "available", "latest_per_character": True,
                    })
                    self.assertEqual({row["snapshot_id"] for row in json.loads(result.content[0].text)["snapshots"]},
                                     {keys["Recharging"]})
                    for name, (_, state) in cases.items():
                        review = await session.call_tool("review_forever_recipe", {"snapshot_id": keys[name], "crafts": 1})
                        self.assertFalse(review.isError)
                        data = json.loads(review.content[0].text)
                        self.assertEqual(data["captured_cooldown_state"], state)
                        self.assertFalse(data["craftability_verified"])
                        self.assertFalse(data["game_execution"])
                    wrong_family = await session.call_tool("list_snapshots", {
                        "family": "retail", "recipe_cooldown_state": "unknown",
                    })
                    self.assertTrue(wrong_family.isError)


if __name__ == "__main__":
    unittest.main()
