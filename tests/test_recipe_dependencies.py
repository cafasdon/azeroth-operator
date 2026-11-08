"""Exercise producer-first batch planning through the Rust MCP without game access."""

import copy
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


def recipe(recipe_id, output, reagents, quantity=1):
    row = {"recipe_id": recipe_id, "name": "Synthetic recipe", "learned": True,
           "requirements": "fixed_items", "reagents": reagents}
    if output is not None:
        row.update(output_item_id=output, base_output_quantity={"minimum": quantity, "maximum": quantity})
    return row


class RecipeDependenciesTests(unittest.IsolatedAsyncioTestCase):
    async def test_dependency_order_and_unknowns(self):
        binary = Path(__file__).resolve().parents[1] / "target/debug" / (
            "azeroth-operator.exe" if os.name == "nt" else "azeroth-operator")
        binary = Path(os.environ.get("AZEROTH_TEST_BINARY", binary)).resolve(strict=True)
        with tempfile.TemporaryDirectory() as directory:
            store = SnapshotStore(Path(directory))
            base = fixture()
            base["client"] = {"family": "forever", "variant": "live", "build": "70862", "interface": 16001}
            base["sections"]["bags"] = {"0": {"1": {"item_id": 7, "count": 2}}}
            base["sections"]["character_bank"] = {}
            base["sections"]["account_bank"] = {}

            def save(row):
                snapshot = copy.deepcopy(base)
                snapshot["sections"]["selected_recipe"] = row
                return store.ingest(snapshot)

            chain = [save(recipe(101, 30, {"20": 1})), save(recipe(102, 20, {"10": 1})),
                     save(recipe(103, 10, {"7": 2}))]
            cycle = [save(recipe(201, 10, {"20": 1})), save(recipe(202, 20, {"10": 1}))]
            unknown = [save(recipe(301, None, {"10": 1})), chain[2]]
            short = [save(recipe(401, 40, {"10": 3})), chain[2]]
            unlearned = recipe(501, 10, {"7": 2})
            unlearned["learned"] = False
            gated = [short[0], save(unlearned)]
            independent = [chain[2], save(recipe(601, 50, {"8": 1}))]
            inventory = copy.deepcopy(base)
            inventory["sections"]["bags"]["0"]["1"]["count"] = 1
            inventory["sections"]["wallet"] = {"copper": 10}
            inventory_id = store.ingest(inventory)
            merchant = copy.deepcopy(inventory)
            merchant["sections"]["wallet"] = {"copper": 999}
            merchant["sections"]["merchant_stock"] = {
                str(slot): {"item_id": item, "bundle_size": 1, "price_copper": price,
                            "available": -1, "extended_cost": False}
                for slot, item, price in ((1, 7, 4), (2, 10, 50), (3, 20, 100))
            }
            merchant_id = store.ingest(merchant)
            merchant["character"]["name"] = "Other"
            wrong_merchant_id = store.ingest(merchant)
            env = {**os.environ, "AZEROTH_DATA_DIR": directory}
            env.pop("AZEROTH_FOREVER_CATALOGUE", None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()

                    async def plan(ids, flag=True, **options):
                        response = await session.call_tool("review_forever_recipe", {
                            "snapshot_id": ids[0], "crafts": 1, "suggest_dependency_order": flag,
                            "additional_recipes": [{"snapshot_id": key, "crafts": 1} for key in ids[1:]],
                            **options,
                        })
                        self.assertFalse(response.isError)
                        return json.loads(response.content[0].text)

                    data = await plan(chain)
                    suggested = data["dependency_order"]
                    self.assertEqual(suggested["status"], "suggested")
                    self.assertEqual(suggested["suggested_recipe_numbers"], [3, 2, 1])
                    required = {row["item_id"]: row["required_before_batch"] for row in suggested["starting_materials"]}
                    self.assertEqual(required, {7: 2, 10: 0, 20: 0})
                    self.assertEqual(suggested["material_checkpoint"]["covered_prefix_recipes"], 3)
                    self.assertEqual(suggested["material_coverage"], {"bags": True, "all_observed_storage": True})
                    self.assertEqual(data["ordered_material_checkpoint"]["covered_prefix_recipes"], 0)
                    self.assertFalse(data["craftability_verified"])
                    self.assertFalse(data["game_execution"])
                    self.assertNotIn("dependency_order", await plan(chain, False))
                    cyclic = (await plan(cycle))["dependency_order"]
                    self.assertEqual(cyclic["status"], "cyclic_dependencies")
                    self.assertEqual(cyclic["blocked_recipe_numbers"], [1, 2])
                    self.assertIsNone(cyclic["suggested_recipe_numbers"])
                    self.assertIsNone(cyclic["starting_materials"])
                    incomplete = (await plan(unknown))["dependency_order"]
                    self.assertEqual(incomplete["status"], "incomplete_observations")
                    self.assertEqual(incomplete["unresolved_output_recipe_numbers"], [1])
                    self.assertIsNone(incomplete["suggested_recipe_numbers"])
                    shortage = (await plan(short))["dependency_order"]
                    self.assertEqual(shortage["suggested_recipe_numbers"], [2, 1])
                    self.assertFalse(shortage["material_coverage"]["all_observed_storage"])
                    need = {row["item_id"]: row["required_before_batch"] for row in shortage["starting_materials"]}
                    self.assertEqual(need[10], 2)
                    blocked = shortage["material_checkpoint"]["first_uncovered_recipe"]
                    self.assertEqual(blocked["recipe_number"], 1)
                    self.assertEqual(blocked["suggested_step"], 2)
                    preparation = (await plan(gated))["dependency_order"]["nonmaterial_requirements"]["first_observed_shortfall"]
                    self.assertEqual(preparation["recipe_number"], 2)
                    self.assertEqual(preparation["suggested_step"], 1)
                    self.assertIn("recipe_learned", preparation["failed_checks"])
                    self.assertEqual((await plan(independent))["dependency_order"]["suggested_recipe_numbers"], [1, 2])
                    quote_options = {"inventory_snapshot_id": inventory_id,
                                     "merchant_reagent_plan": True, "merchant_snapshot_id": merchant_id}
                    quoted = await plan(chain, **quote_options)
                    self.assertEqual(quoted["merchant_reagent_plan"]["unlimited_copper_only_total"], 154)
                    suggested_quote = quoted["dependency_order"]["merchant_reagent_plan"]
                    self.assertEqual(suggested_quote["unlimited_copper_only_total"], 4)
                    self.assertTrue(suggested_quote["unlimited_copper_only_affordable"])
                    self.assertEqual([row["item_id"] for row in suggested_quote["reagents"]], [7])
                    first = suggested_quote["first_uncovered_recipe_budget"]
                    self.assertEqual(first["recipe_number"], 3)
                    self.assertEqual(first["suggested_step"], 1)
                    partial_quote = (await plan(short, **quote_options))["dependency_order"]["merchant_reagent_plan"]
                    self.assertEqual(partial_quote["unlimited_copper_only_total"], 104)
                    self.assertFalse(partial_quote["unlimited_copper_only_affordable"])
                    self.assertEqual(partial_quote["unlimited_copper_only_shortfall"], 94)
                    self.assertIsNone((await plan(cycle, **quote_options))["dependency_order"]["merchant_reagent_plan"])
                    self.assertIsNone((await plan(unknown, **quote_options))["dependency_order"]["merchant_reagent_plan"])
                    missing_offers = await plan(chain, **{**quote_options, "merchant_snapshot_id": inventory_id})
                    self.assertIsNone(missing_offers["dependency_order"]["merchant_reagent_plan"])
                    wrong = await session.call_tool("review_forever_recipe", {
                        "snapshot_id": chain[0], "crafts": 1, "additional_recipes": [{"snapshot_id": key, "crafts": 1} for key in chain[1:]],
                        "suggest_dependency_order": True, **{**quote_options, "merchant_snapshot_id": wrong_merchant_id}})
                    self.assertTrue(wrong.isError)
                    single = await session.call_tool("review_forever_recipe", {
                        "snapshot_id": chain[0], "crafts": 1, "suggest_dependency_order": True})
                    self.assertTrue(single.isError)


if __name__ == "__main__":
    unittest.main()
