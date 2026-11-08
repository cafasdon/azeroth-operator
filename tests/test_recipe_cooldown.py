"""Exercise selected-recipe cooldown searches through the real Rust MCP."""

import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class RecipeCooldownTests(unittest.IsolatedAsyncioTestCase):
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
