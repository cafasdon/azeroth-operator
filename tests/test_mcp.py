import os
import asyncio
import json
import sys
import subprocess
import tempfile
import unittest
from pathlib import Path
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.shared.exceptions import McpError
from shared.snapshots import SnapshotStore
from test_snapshots import fixture
from tools.gateway_profile import TOOLS


class MCPTests(unittest.IsolatedAsyncioTestCase):
    async def test_rust_stdio_server(self):
        root = Path(__file__).resolve().parents[1]
        binary = Path(os.environ.get("AZEROTH_TEST_BINARY") or
                      root / "target" / "debug" / ("azeroth-operator.exe" if os.name == "nt" else "azeroth-operator"))
        self.assertTrue(binary.is_file(), "Build the Rust server with cargo build --locked before protocol tests")
        await asyncio.wait_for(self.check_stdio_server(str(binary.resolve())), timeout=60)

    async def test_python_reference_server(self):
        await asyncio.wait_for(self.check_stdio_server(None), timeout=60)

    async def test_forever_catalogue_configuration(self):
        binary = Path(__file__).resolve().parents[1] / "target" / "debug" / (
            "azeroth-operator.exe" if os.name == "nt" else "azeroth-operator")
        binary = Path(os.environ.get("AZEROTH_TEST_BINARY", binary)).resolve(strict=True)
        with tempfile.TemporaryDirectory() as folder:
            cache = Path(folder) / "catalogue.json"
            cache.write_text('{"items":[{"id":744,"name":"Unverified"}]}')
            params = StdioServerParameters(command=str(binary), env={
                **os.environ, "AZEROTH_DATA_DIR": folder, "AZEROTH_FOREVER_CATALOGUE": str(cache)})
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    self.assertIn("lookup_forever_catalogue", {t.name for t in (await session.list_tools()).tools})
                    caps = await session.call_tool("capabilities", {})
                    self.assertIn("lookup_forever_catalogue", json.loads(caps.content[0].text)["implemented"])
                    self.assertIn("review_forever_owned_gear", json.loads(caps.content[0].text)["implemented"])
                    response = await session.call_tool("lookup_forever_catalogue", {"section":"items","key":"744"})
                    self.assertTrue(response.isError)
                    self.assertNotIn("Unverified", response.content[0].text)
                    forever = fixture()
                    forever["client"] = {"family":"forever","variant":"live","build":"70862","interface":16001}
                    key = SnapshotStore(Path(folder)).ingest(forever)
                    review = await session.call_tool("review_forever_owned_gear", {"snapshot_id":key})
                    self.assertTrue(review.isError)
                    self.assertNotIn("Unverified", review.content[0].text)

    async def test_snapshot_owner_scope(self):
        binary = Path(__file__).resolve().parents[1] / "target" / "debug" / (
            "azeroth-operator.exe" if os.name == "nt" else "azeroth-operator")
        binary = Path(os.environ.get("AZEROTH_TEST_BINARY", binary)).resolve(strict=True)
        env = dict(os.environ)
        env.pop("AZEROTH_DATA_DIR", None)
        missing = subprocess.run([str(binary)], env=env, stdin=subprocess.DEVNULL,
                                 capture_output=True, text=True, timeout=10)
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("AZEROTH_DATA_DIR", missing.stderr)
        self.assertEqual(missing.stdout, "")
        with tempfile.TemporaryDirectory() as owner_a, tempfile.TemporaryDirectory() as owner_b:
            key = SnapshotStore(Path(owner_a)).ingest(fixture())
            env["AZEROTH_DATA_DIR"] = owner_b
            params = StdioServerParameters(command=str(binary), env=env)
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    result = await session.call_tool("inspect_snapshot", {"snapshot_id":key})
                    self.assertTrue(result.isError)
                    self.assertNotIn(owner_a, result.content[0].text)
                    listed = await session.call_tool("list_snapshots", {"family":"retail"})
                    self.assertEqual(json.loads(listed.content[0].text)["snapshots"], [])

    async def check_stdio_server(self, rust_binary):
        with tempfile.TemporaryDirectory() as folder:
            sample = fixture()
            sample["sections"]["equipment"] = {"16": {"item_id": 123,"guid":"Item-Old"}}
            sample["sections"]["bindings"] = {"1":{"command":"ACTIONBUTTON1","key1":"CTRL-1","context":0}}
            sample["sections"]["binding_state"] = {"binding_set":2,"coverage":"bound_keys_only"}
            sample["sections"]["operation_receipts"] = {"a" * 64: {"status":"outcome_unknown", "kind":"move"}}
            sample["sections"]["macros"] = {"1": {"name":"AO Test", "icon":134400,
                "body":"/say test", "scope":"account"}}
            sample["sections"]["bags"] = {"0": {"1": {"item_id": 123, "count": 1,"guid":"Item-New"},
                                                     "2": {"item_id": 123, "count": 4, "guid":"Item-Stack"}}}
            sample["sections"]["bags"]["0"]["4"]={"item_id":456,"count":2,"guid":"Item-Swap","link":"target"}
            sample["sections"]["bags"]["0"]["1"]["link"]="source"
            sample["sections"]["talents"] = {"config_id":7,"nodes":{},"loadout_export":{
                "status":"available","format":"blizzard_traits","import_string":"QUJDRA==",
                "spec_id":263,"serialization_version":2,"staged_changes":False}}
            sample["sections"]["talents"]["saved_loadouts"]={"spec_id":263,"selected_config_id":8,"entries":{"9":{"name":"Build","import_string":"RUZHSA==","serialization_version":2,"uses_shared_action_bars":True}}}
            sample["sections"]["action_bar_observation"]={"coverage":"loaded_builtin_buttons_only","observed_buttons":2,"missing_buttons":0,
                "slots":{"1":{"empty":False,"kind":"macro","id":1},"2":{"empty":True}}}
            key = SnapshotStore(Path(folder)).ingest(sample)
            sample["sections"]["equipment_sets"]={"coverage":"native_catalogue_raw_slots","can_use":True,"entries":{"0":{
                "name":"Fixture set","icon":134400,"is_equipped":False,"num_items":1,"num_equipped":0,
                "num_in_inventory":1,"num_lost":0,"num_ignored":18,"assigned_spec":2,
                "slots":{str(i):{"ignored":True} for i in range(1,20)}}}}
            sample["sections"]["equipment_sets"]["entries"]["0"]["slots"]["16"]={"ignored":False,"raw_item_id":123,"raw_location":999,
                "observed_item":{"kind":"bag","bag":0,"slot":1,"item_id":123,"guid":"Item-New","locked":False}}
            set_key=SnapshotStore(Path(folder)).ingest(sample)
            sample["sections"]["equipment_sets"]["specializations"]={"1":261,"2":262,"3":263}
            sample["sections"]["equipment_sets"]["entries"]["1"]=json.loads(json.dumps(sample["sections"]["equipment_sets"]["entries"]["0"]))
            sample["sections"]["equipment_sets"]["entries"]["1"].update({"name":"Displaced","assigned_spec":1})
            spec_key=SnapshotStore(Path(folder)).ingest(sample)
            env = {**os.environ, "AZEROTH_DATA_DIR": folder}
            env.pop("AZEROTH_FOREVER_CATALOGUE", None)
            params = StdioServerParameters(command=rust_binary or sys.executable,
                args=[] if rust_binary else [str(Path(__file__).resolve().parents[1] / "server.py")],
                env=env)
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    tools = await session.list_tools()
                    expected = {"capabilities", "inspect_snapshot"}
                    if rust_binary:
                        expected.update({"list_snapshots", "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout", "review_forever_recipe", "find_snapshot_item"})
                    self.assertEqual({t.name for t in tools.tools}, expected)
                    if rust_binary:
                        self.assertEqual(set(TOOLS), expected)
                    if rust_binary:
                        for tool in tools.tools:
                            self.assertTrue(tool.annotations.readOnlyHint)
                            self.assertFalse(tool.annotations.destructiveHint)
                        capabilities = await session.call_tool("capabilities", {})
                        contract = json.loads(capabilities.content[0].text)
                        self.assertFalse(contract["game_execution"])
                        self.assertEqual(contract["policy"]["mode"], "offline_analysis")
                        self.assertFalse(contract["policy"]["generated_game_imports"])
                        self.assertEqual(set(contract["implemented"]), expected)
                        self.assertEqual(contract["live_client_qualified"], [])
                        before_files={p.name:p.read_bytes() for p in Path(folder).glob("*.json")}
                        for blocked in ("prepare_macro_operation", "prepare_binding_operation",
                                        "prepare_equipment_operation", "prepare_bag_operation",
                                        "prepare_talent_operation", "prepare_action_operation",
                                        "prepare_equipment_set_operation", "equip_item"):
                            with self.assertRaisesRegex(McpError, "tool not found"):
                                await session.call_tool(blocked, {"snapshot_id":key, "operation":{}})
                        self.assertEqual(before_files,{p.name:p.read_bytes() for p in Path(folder).glob("*.json")})
                        listing = await session.call_tool("list_snapshots", {"family": "retail"})
                        self.assertEqual({row["snapshot_id"] for row in json.loads(listing.content[0].text)["snapshots"]}, {key,set_key,spec_key})
                        other = await session.call_tool("list_snapshots", {"family": "forever"})
                        self.assertEqual(json.loads(other.content[0].text)["snapshots"], [])
                        forever = fixture()
                        forever["client"] = {"family":"forever","variant":"live","build":"70862","interface":16001}
                        forever["character"] = {"name":"Example","realm":"First"}
                        forever["sections"]["character_bank"] = {"6":{"1":{"item_id":744,"count":1,"link":"item:744:7"}}}
                        forever["sections"]["account_bank"] = {}
                        forever["sections"]["quests"] = {
                            "42":{"title":"Fixture quest","level":10,"ready_for_turn_in":False,
                                  "objectives":{"1":{"kind":"item","finished":False,"fulfilled":0,"required":5}}},
                            "43":{"title":"Legacy quest","level":10}}
                        forever["sections"]["tracked_achievements"] = {"coverage":"tracked_player_achievements_only","entries":{
                            "101":{"name":"Fixture achievement","points":5,"completed":False,"earned_by_character":False,
                                   "account_wide":True,"criteria":{"1":{"criterion_id":301,"completed":False,"quantity":1,"required":2}}}}}
                        forever["sections"]["currency_balances"] = {"coverage":"visible_currency_ids_only","entries":{
                            "100":{"name":"Fixture currency","quantity":0,"account_wide":True,"inactive":False,"max_quantity":100,
                                   "max_weekly_quantity":40,"earned_this_week":10,"total_earned":60,
                                   "cap_uses_total_earned":True}}}
                        forever["sections"]["reputation"] = {"coverage":"visible_faction_rows_only","entries":{
                            "10":{"name":"Fixture faction","kind":"standard","account_wide":False,"reaction":4,
                                  "standing":-50,"threshold":-100,"next_threshold":0}}}
                        forever["sections"]["selected_recipe"] = {"recipe_id":100,"name":"Fixture recipe","learned":True,
                            "requirements":"fixed_items","reagents":{"744":3}}
                        forever_key = SnapshotStore(Path(folder)).ingest(forever)
                        forever["character"]["realm"] = "Second"
                        SnapshotStore(Path(folder)).ingest(forever)
                        filtered = await session.call_tool("list_snapshots", {"family":"forever",
                            "character":{"name":"example","realm":"FIRST"},"limit":1})
                        filtered_data = json.loads(filtered.content[0].text)
                        self.assertEqual([row["snapshot_id"] for row in filtered_data["snapshots"]], [forever_key])
                        self.assertFalse(filtered_data["has_more"])
                        bank = await session.call_tool("inspect_snapshot", {"snapshot_id":forever_key,"section":"character_bank"})
                        self.assertEqual(json.loads(bank.content[0].text)["data"]["6"]["1"]["link"], "item:744:7")
                        bank_summary = await session.call_tool("character_summary", {"snapshot_id":forever_key})
                        bank_summary_data = json.loads(bank_summary.content[0].text)
                        self.assertEqual(bank_summary_data["character_bank_item_slots_observed"], 1)
                        self.assertEqual(bank_summary_data["account_bank_item_slots_observed"], 0)
                        self.assertEqual(bank_summary_data["reputation_factions_observed"], 1)
                        self.assertEqual(bank_summary_data["quest_turn_in"], {"ready_observed":0,"status_observed":1,"status_unknown":1})
                        quests = await session.call_tool("inspect_snapshot", {"snapshot_id":forever_key,"section":"quests"})
                        self.assertEqual(json.loads(quests.content[0].text)["data"]["42"]["objectives"]["1"]["fulfilled"], 0)
                        self.assertEqual(bank_summary_data["tracked_achievements_observed"], 1)
                        achievement = await session.call_tool("inspect_snapshot", {"snapshot_id":forever_key,"section":"tracked_achievements"})
                        self.assertEqual(json.loads(achievement.content[0].text)["data"]["entries"]["101"]["criteria"]["1"]["quantity"], 1)
                        self.assertEqual(bank_summary_data["currency_balances_observed"], 1)
                        currency = await session.call_tool("inspect_snapshot", {"snapshot_id":forever_key,"section":"currency_balances"})
                        self.assertEqual(json.loads(currency.content[0].text)["data"]["entries"]["100"]["quantity"], 0)
                        reputation = await session.call_tool("inspect_snapshot", {"snapshot_id":forever_key,"section":"reputation"})
                        reputation_data = json.loads(reputation.content[0].text)["data"]
                        self.assertEqual(reputation_data["coverage"], "visible_faction_rows_only")
                        self.assertEqual(reputation_data["entries"]["10"]["standing"], -50)
                        reputation_after = json.loads(json.dumps(forever))
                        reputation_after["character"]["realm"] = "First"
                        reputation_after["sections"]["reputation"]["entries"]["10"]["standing"] = -20
                        reputation_after["sections"]["currency_balances"]["entries"]["100"]["quantity"] = 3
                        reputation_after["sections"]["tracked_achievements"]["entries"]["101"]["criteria"]["1"]["quantity"] = 2
                        reputation_after["sections"]["quests"]["42"]["objectives"]["1"]["fulfilled"] = 2
                        after_key = SnapshotStore(Path(folder)).ingest(reputation_after)
                        rep_diff = await session.call_tool("compare_snapshots", {"before_id":forever_key,"after_id":after_key})
                        self.assertFalse(rep_diff.isError)
                        self.assertEqual(json.loads(rep_diff.content[0].text)["sections"]["reputation"]["changed_paths"],
                                         ["/entries/10/standing"])
                        self.assertEqual(json.loads(rep_diff.content[0].text)["sections"]["currency_balances"]["changed_paths"],
                                         ["/entries/100/quantity"])
                        self.assertEqual(json.loads(rep_diff.content[0].text)["sections"]["tracked_achievements"]["changed_paths"],
                                         ["/entries/101/criteria/1/quantity"])
                        self.assertEqual(json.loads(rep_diff.content[0].text)["sections"]["quests"]["changed_paths"],
                                         ["/42/objectives/1/fulfilled"])
                        recipe = await session.call_tool("review_forever_recipe", {"snapshot_id":forever_key,"crafts":2})
                        self.assertFalse(recipe.isError)
                        recipe_data = json.loads(recipe.content[0].text)
                        self.assertEqual(recipe_data["materials"], [{
                            "item_id":744,"required":6,"quantity_observed":1,"shortfall_vs_observed":5,
                            "quantity_by_storage":{"bags":None,"character_bank":1,"account_bank":0},
                            "observed_stack_count":1,
                            "observed_locations":[{"storage":"character_bank","container":"6","slot":"1","count":1}],
                        }])
                        self.assertFalse(recipe_data["craftability_verified"])
                        self.assertEqual(recipe_data["unobserved_storage"], ["bags"])
                        self.assertEqual(recipe_data["material_capacity"], {
                            "bags": None,
                            "all_observed_storage": {"supported_crafts": 0, "limiting_item_ids": [744]},
                        })
                        forever["sections"]["bags"] = {"0": {
                            "1": {"item_id": 744, "count": 8},
                            "2": {"item_id": 10, "count": 6},
                        }}
                        forever["sections"]["selected_recipe"]["reagents"] = {"744": 3, "10": 2}
                        stocked_key = SnapshotStore(Path(folder)).ingest(forever)
                        stocked = await session.call_tool("review_forever_recipe", {
                            "snapshot_id": stocked_key, "crafts": 1})
                        self.assertFalse(stocked.isError)
                        self.assertEqual(json.loads(stocked.content[0].text)["material_capacity"], {
                            "bags": {"supported_crafts": 2, "limiting_item_ids": [744]},
                            "all_observed_storage": {"supported_crafts": 3, "limiting_item_ids": [10, 744]},
                        })
                        forever["sections"]["bags"]["0"] = {
                            str(slot): {"item_id": 744, "count": 2} for slot in range(1, 22)
                        }
                        capped_key = SnapshotStore(Path(folder)).ingest(forever)
                        capped = await session.call_tool("review_forever_recipe", {
                            "snapshot_id": capped_key, "crafts": 1})
                        self.assertFalse(capped.isError)
                        reagents = {row["item_id"]: row for row in json.loads(capped.content[0].text)["materials"]}
                        self.assertEqual(reagents[744]["quantity_observed"], 43)
                        self.assertEqual(reagents[744]["observed_stack_count"], 22)
                        self.assertEqual(len(reagents[744]["observed_locations"]), 20)
                        self.assertEqual(reagents[10]["observed_stack_count"], 0)
                        self.assertEqual(reagents[10]["observed_locations"], [])
                        search = {"snapshot_id": capped_key, "item_id": 744, "limit": 2}
                        matches = await session.call_tool("find_snapshot_item", search)
                        self.assertFalse(matches.isError)
                        found = json.loads(matches.content[0].text)
                        self.assertEqual(found["matched_locations"], 22)
                        self.assertEqual(found["quantity_by_storage"], {
                            "equipment": 0, "bags": 42, "character_bank": 1, "account_bank": 0})
                        self.assertTrue(found["has_more"])
                        last = await session.call_tool("find_snapshot_item", {**search, "offset": 21})
                        tail = json.loads(last.content[0].text)
                        self.assertEqual(tail["locations"], [
                            {"storage": "character_bank", "container": "6", "slot": "1", "count": 1}])
                        self.assertFalse(tail["has_more"])
                        self.assertTrue((await session.call_tool("find_snapshot_item", {**search, "limit": 51})).isError)
                        forever["sections"]["bags"]["0"]["1"]["count"] = 1
                        forever["sections"]["character_bank"]["6"]["1"]["count"] = 2
                        forever["sections"]["currency_balances"]["entries"]["100"]["quantity"] = 5
                        forever["sections"]["currency_balances"]["entries"]["100"]["earned_this_week"] = 1
                        moved_key = SnapshotStore(Path(folder)).ingest(forever)
                        changes = {"before_id": capped_key, "after_id": moved_key,
                                   "item_id": 744, "currency_id": 100}
                        compared = await session.call_tool("compare_snapshots", changes)
                        self.assertFalse(compared.isError)
                        comparison_data = json.loads(compared.content[0].text)
                        currency = comparison_data["currency_balance_change"]
                        self.assertEqual(currency["before"]["quantity"], 0)
                        self.assertEqual(currency["after"]["earned_this_week"], 1)
                        self.assertEqual(currency["change"], {"direction": "increased", "amount": 5})
                        item_change = comparison_data["item_quantity_change"]
                        self.assertEqual(item_change["net_delta"], 0)
                        self.assertEqual(item_change["by_storage"]["bags"], {"before": 42, "after": 41, "delta": -1})
                        self.assertEqual(item_change["by_storage"]["character_bank"], {"before": 1, "after": 2, "delta": 1})
                        del forever["sections"]["bags"]
                        forever["sections"]["currency_balances"]["entries"] = {}
                        partial_key = SnapshotStore(Path(folder)).ingest(forever)
                        partial = await session.call_tool("compare_snapshots", {**changes, "after_id": partial_key})
                        comparison_data = json.loads(partial.content[0].text)
                        currency = comparison_data["currency_balance_change"]
                        self.assertIsNone(currency["after"])
                        self.assertIsNone(currency["change"])
                        item_change = comparison_data["item_quantity_change"]
                        self.assertIsNone(item_change["net_delta"])
                        self.assertEqual(item_change["by_storage"]["bags"], {"before": 42, "after": None, "delta": None})
                        mismatch = await session.call_tool("compare_snapshots", {**changes, "after_id": forever_key})
                        self.assertTrue(mismatch.isError)
                        for bad in ({"snapshot_id":forever_key,"crafts":0},{"snapshot_id":forever_key,"crafts":1001},
                                    {"snapshot_id":key,"crafts":1}):
                            self.assertTrue((await session.call_tool("review_forever_recipe", bad)).isError)
                        comparison = await session.call_tool("compare_snapshots", {"before_id": key, "after_id": key})
                        self.assertEqual(json.loads(comparison.content[0].text)["sections"]["equipment"]["status"], "unchanged")
                        summary = await session.call_tool("character_summary", {"snapshot_id": key})
                        self.assertFalse(json.loads(summary.content[0].text)["live"])
                        carried = json.loads(summary.content[0].text)["carried_item_review"]
                        self.assertEqual(carried["quantity_observed"], 7)
                        self.assertEqual(carried["repeated_item_ids"][0]["occupied_slots_observed"], 2)
                        self.assertEqual(carried["stack_compatibility"], "unknown")
                        observed = sample["sections"]["equipment"]
                        slot, item = next(iter(observed.items()))
                        review = await session.call_tool("review_equipment_selection", {
                            "snapshot_id": key, "selections": [{"target_slot": int(slot),
                            "source": {"kind": "equipment", "slot": int(slot)}, "expected_item_id": item["item_id"]}]})
                        self.assertFalse(review.isError)
                        draft = json.loads(review.content[0].text)
                        self.assertEqual(draft["snapshot_id"], key)
                        self.assertFalse(draft["game_execution"])
                        self.assertEqual(draft["changes"][0]["status"], "already_selected")
                        rejected_review = await session.call_tool("review_equipment_selection", {
                            "snapshot_id": key, "selections": [{"target_slot": int(slot),
                            "source": {"kind": "equipment", "slot": int(slot)}, "expected_item_id": 4294967295}]})
                        self.assertTrue(rejected_review.isError)
                        loadout = await session.call_tool("captured_talent_loadout", {"snapshot_id":key})
                        self.assertFalse(loadout.isError)
                        captured = json.loads(loadout.content[0].text)
                        self.assertEqual(captured["loadout"]["import_string"], "QUJDRA==")
                        self.assertEqual(captured["snapshot_id"], key)
                        self.assertFalse(captured["game_execution"])
                    result = await session.call_tool("inspect_snapshot", {"snapshot_id": key})
                    receipts = await session.call_tool("inspect_snapshot", {"snapshot_id":key,"section":"operation_receipts"})
                    self.assertFalse(receipts.isError)
                    self.assertEqual(json.loads(receipts.content[0].text)["data"]["a" * 64]["status"], "outcome_unknown")
                    self.assertFalse(result.isError)
                    payload = json.loads(result.content[0].text)
                    self.assertFalse(payload["live"])
                    missing = await session.call_tool("inspect_snapshot", {"snapshot_id": "f" * 64})
                    self.assertTrue(missing.isError)
                    self.assertNotIn(folder, missing.content[0].text)
                    rejected = await session.call_tool("inspect_snapshot", {"snapshot_id": "../secret"})
                    self.assertTrue(rejected.isError)
