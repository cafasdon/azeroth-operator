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
                        forever["sections"]["wallet"] = {"copper": 5_000_000_001}
                        forever["sections"]["professions"] = {
                            "1": {"name": "Fixture profession", "rank": 20, "max_rank": 75, "skill_line": 164}}
                        forever["sections"]["quests"] = {
                            "42":{"title":"Fixture quest","level":10,"ready_for_turn_in":False,
                                  "timer":{"total_seconds":100,"remaining_seconds":74},
                                  "objectives":{
                                      "1":{"kind":"item","finished":False,"fulfilled":0,"required":5},
                                      "2":{"kind":"event","finished":False},
                                      "3":{"kind":"item","finished":True,"fulfilled":5,"required":5},
                                      "4":{"kind":"item","finished":False,"fulfilled":6,"required":5},
                                  }},
                            "43":{"title":"Legacy quest","level":10},
                            "10":{"title":"Turn-in quest","level":10,"ready_for_turn_in":True,
                                  "objectives":{"1":{"kind":"event","finished":True}}},
                            "2":{"title":"Another turn-in","level":10,"ready_for_turn_in":True},
                            "44":{"title":"Failed quest","level":10,"ready_for_turn_in":False,"failed":True}}
                        forever["sections"]["tracked_achievements"] = {"coverage":"tracked_player_achievements_only","entries":{
                            "101":{"name":"Fixture achievement","points":5,"completed":False,"earned_by_character":False,
                                   "account_wide":True,"criteria":{
                                       "1":{"criterion_id":301,"completed":False,"quantity":1,"required":5},
                                       "2":{"criterion_id":302,"completed":True},
                                       "3":{"completed":False},
                                       "4":{"criterion_id":304,"completed":False,"quantity":6,"required":5}}},
                            "102":{"name":"Finished achievement","points":5,"completed":True,
                                   "earned_by_character":True,"account_wide":False,"criteria":{}}}}
                        forever["sections"]["currency_balances"] = {"coverage":"visible_currency_ids_only","entries":{
                            "100":{"name":"Fixture currency","quantity":0,"account_wide":True,"inactive":False,"max_quantity":100,
                                   "max_weekly_quantity":40,"earned_this_week":10,"total_earned":60,
                                   "cap_uses_total_earned":True}}}
                        forever["sections"]["reputation"] = {"coverage":"visible_faction_rows_only","entries":{
                            "10":{"name":"Fixture faction","kind":"standard","account_wide":False,"reaction":4,
                                  "standing":-50,"threshold":-100,"next_threshold":0}}}
                        forever["sections"]["tracked_recipes"] = {"coverage": "player_tracked_recipes_only",
                            "crafts": {"100": {"name": "Tracked fixture"}, "101": {}},
                            "recrafts": {"100": {}}}
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
                        tracked_recipes = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": forever_key, "section": "tracked_recipes"})
                        self.assertFalse(tracked_recipes.isError)
                        self.assertEqual(json.loads(tracked_recipes.content[0].text)["data"],
                                         forever["sections"]["tracked_recipes"])
                        tracked_after = json.loads(json.dumps(forever))
                        tracked_after["character"]["realm"] = "First"
                        tracked_after["captured_at"] = "2025-01-02T00:00:00Z"
                        groups = tracked_after["sections"]["tracked_recipes"]
                        groups["crafts"] = {"2": {}, "11": {}, "100": {"name": "Renamed fixture"}}
                        groups["recrafts"] = {}
                        after_tracking_key = SnapshotStore(Path(folder)).ingest(tracked_after)
                        async def compare_tracking(after_id):
                            response = await session.call_tool("compare_snapshots", {
                                "before_id": forever_key, "after_id": after_id})
                            self.assertFalse(response.isError)
                            return json.loads(response.content[0].text)["tracked_recipe_changes"]
                        self.assertEqual(await compare_tracking(after_tracking_key), {
                            "crafts": {"added_recipe_ids": [2, 11], "removed_recipe_ids": [101]},
                            "recrafts": {"added_recipe_ids": [], "removed_recipe_ids": [100]},
                        })
                        del tracked_after["sections"]["tracked_recipes"]
                        missing_tracking_key = SnapshotStore(Path(folder)).ingest(tracked_after)
                        self.assertIsNone(await compare_tracking(missing_tracking_key))
                        bank_summary = await session.call_tool("character_summary", {"snapshot_id":forever_key})
                        bank_summary_data = json.loads(bank_summary.content[0].text)
                        self.assertEqual(bank_summary_data["character_bank_item_slots_observed"], 1)
                        self.assertEqual(bank_summary_data["account_bank_item_slots_observed"], 0)
                        self.assertEqual(bank_summary_data["reputation_factions_observed"], 1)
                        self.assertEqual(bank_summary_data["standard_reputation_workload"], {
                            "capped_factions": 0, "factions_with_observed_rank_gap": 1,
                            "closest_to_next_rank": [
                                {"faction_id": 10, "name": "Fixture faction", "account_wide": False,
                                 "remaining_to_next_rank": 50},
                            ],
                        })
                        self.assertEqual(bank_summary_data["quest_turn_in"], {"ready_observed":2,"status_observed":4,"status_unknown":1})
                        self.assertEqual(bank_summary_data["quest_objective_workload"], {
                            "unfinished_objectives_by_kind": {"event": 1, "item": 2},
                            "quests_without_objective_observations": 2,
                            "quests_with_unfinished_objectives": 1,
                            "quests_with_most_unfinished_objectives": [
                                {"quest_id": 42, "title": "Fixture quest", "unfinished_objectives": 3},
                            ],
                        })
                        self.assertEqual(bank_summary_data["quest_checklist"], {
                            "ready_for_turn_in": [2, 10], "turn_in_not_ready": [42],
                            "failed": [44], "turn_in_unknown": [43],
                        })
                        empty_quests = json.loads(json.dumps(forever))
                        empty_quests["sections"]["quests"] = {}
                        empty_key = SnapshotStore(Path(folder)).ingest(empty_quests)
                        empty_summary = await session.call_tool("character_summary", {"snapshot_id":empty_key})
                        self.assertEqual(json.loads(empty_summary.content[0].text)["quest_checklist"], {
                            "ready_for_turn_in": [], "turn_in_not_ready": [],
                            "failed": [], "turn_in_unknown": [],
                        })
                        quests = await session.call_tool("inspect_snapshot", {"snapshot_id":forever_key,"section":"quests"})
                        self.assertEqual(json.loads(quests.content[0].text)["data"]["42"]["objectives"]["1"]["fulfilled"], 0)
                        async def inspect_quest(snapshot_id, quest_id):
                            response = await session.call_tool("inspect_snapshot", {
                                "snapshot_id": snapshot_id, "section": "quests", "quest_id": quest_id})
                            self.assertFalse(response.isError)
                            return json.loads(response.content[0].text)
                        quest_detail = await inspect_quest(forever_key, 42)
                        self.assertTrue(quest_detail["quest_observed"])
                        self.assertEqual(quest_detail["data"]["title"], "Fixture quest")
                        self.assertEqual(quest_detail["unfinished_objectives"], {
                            "1": {"kind": "item", "remaining_count": 5},
                            "2": {"kind": "event", "remaining_count": None},
                            "4": {"kind": "item", "remaining_count": 0},
                        })
                        self.assertEqual((await inspect_quest(forever_key, 10))["unfinished_objectives"], {})
                        self.assertIsNone((await inspect_quest(forever_key, 43))["unfinished_objectives"])
                        self.assertFalse((await inspect_quest(empty_key, 42))["quest_observed"])
                        self.assertIsNone((await inspect_quest(key, 42))["quest_observed"])
                        bad_selector = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": forever_key, "section": "equipment", "quest_id": 42})
                        self.assertTrue(bad_selector.isError)
                        self.assertEqual(bank_summary_data["tracked_achievements_observed"], 2)
                        self.assertEqual(bank_summary_data["tracked_achievement_workload"], {
                            "achievements_with_unfinished_criteria": 1,
                            "completed_without_character_credit_ids": [],
                            "achievements_with_most_unfinished_criteria": [
                                {"achievement_id": 101, "name": "Fixture achievement",
                                 "completed": False, "earned_by_character": False,
                                 "account_wide": True, "unfinished_criteria": 3},
                            ],
                        })
                        training = json.loads(json.dumps(forever))
                        training["sections"]["professions"].update({
                            "2": {"name": "Fixture capped", "rank": 75, "max_rank": 75, "skill_line": 165},
                            "3": {"name": "Fixture unknown cap", "rank": 0, "max_rank": 0, "skill_line": 166},
                            "4": {"name": "Fixture inconsistent", "rank": 76, "max_rank": 75, "skill_line": 164},
                        })
                        training_key = SnapshotStore(Path(folder)).ingest(training)
                        professions = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": training_key, "section": "professions"})
                        self.assertFalse(professions.isError)
                        self.assertEqual(json.loads(professions.content[0].text)["profession_training_progress"], {
                            "1": {"skill_line": 164, "remaining_to_observed_cap": 55},
                            "2": {"skill_line": 165, "remaining_to_observed_cap": 0},
                            "3": {"skill_line": 166, "remaining_to_observed_cap": None},
                            "4": {"skill_line": 164, "remaining_to_observed_cap": None},
                        })
                        missing_professions = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": key, "section": "professions"})
                        self.assertIsNone(json.loads(missing_professions.content[0].text)["profession_training_progress"])
                        achievement = await session.call_tool("inspect_snapshot", {"snapshot_id":forever_key,"section":"tracked_achievements"})
                        achievement_data = json.loads(achievement.content[0].text)
                        self.assertEqual(achievement_data["data"]["entries"]["101"]["criteria"]["1"]["quantity"], 1)
                        self.assertEqual(achievement_data["tracked_achievement_work"], {
                            "101": {"1": {"criterion_id": 301, "remaining_count": 4},
                                    "3": {"criterion_id": None, "remaining_count": None},
                                    "4": {"criterion_id": 304, "remaining_count": 0}},
                            "102": {},
                        })
                        missing_achievements = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": key, "section": "tracked_achievements"})
                        self.assertIsNone(json.loads(missing_achievements.content[0].text)["tracked_achievement_work"])
                        self.assertEqual(bank_summary_data["currency_balances_observed"], 1)
                        self.assertEqual(bank_summary_data["currency_weekly_cap_summary"], {
                            "weekly_caps_observed": 1, "weekly_caps_reached": 0,
                            "closest_weekly_caps": [
                                {"currency_id": 100, "name": "Fixture currency", "weekly_cap": 40,
                                 "earned_this_week": 10, "weekly_remaining": 30,
                                 "inactive": False, "can_earn_per_week": None},
                            ],
                        })
                        currency = await session.call_tool("inspect_snapshot", {"snapshot_id":forever_key,"section":"currency_balances"})
                        currency_data = json.loads(currency.content[0].text)
                        self.assertEqual(currency_data["data"]["entries"]["100"]["quantity"], 0)
                        self.assertNotIn("can_earn_per_week", currency_data["data"]["entries"]["100"])
                        self.assertEqual(currency_data["currency_cap_headroom"], {
                            "100": {"weekly_remaining": 30, "maximum_remaining": 40,
                                    "maximum_basis": "total_earned"},
                        })
                        async def inspect_caps(snapshot_id):
                            result = await session.call_tool("inspect_snapshot", {
                                "snapshot_id": snapshot_id, "section": "currency_balances"})
                            self.assertFalse(result.isError)
                            return json.loads(result.content[0].text)
                        self.assertIsNone((await inspect_caps(key))["currency_cap_headroom"])
                        other_caps = json.loads(json.dumps(forever))
                        entries = other_caps["sections"]["currency_balances"]["entries"]
                        entries["100"].update(cap_uses_total_earned=False, max_weekly_quantity=0, quantity=101,
                                              can_earn_per_week=False)
                        entries["101"] = {**entries["100"], "max_quantity": 0}
                        entries["102"] = {**entries["101"], "inactive": True,
                                          "max_weekly_quantity": 5, "earned_this_week": 6,
                                          "can_earn_per_week": True}
                        other_key = SnapshotStore(Path(folder)).ingest(other_caps)
                        other_data = await inspect_caps(other_key)
                        self.assertIs(other_data["data"]["entries"]["100"]["can_earn_per_week"], False)
                        self.assertIs(other_data["data"]["entries"]["102"]["can_earn_per_week"], True)
                        self.assertEqual(other_data["currency_cap_headroom"], {
                            "100": {"weekly_remaining": None, "maximum_remaining": 0,
                                    "maximum_basis": "quantity"},
                            "102": {"weekly_remaining": 0, "maximum_remaining": None,
                                    "maximum_basis": None},
                        })
                        wallet = await session.call_tool("inspect_snapshot", {"snapshot_id": forever_key, "section": "wallet"})
                        self.assertFalse(wallet.isError)
                        self.assertEqual(json.loads(wallet.content[0].text)["data"], {"copper": 5_000_000_001})
                        reputation = await session.call_tool("inspect_snapshot", {"snapshot_id":forever_key,"section":"reputation"})
                        reputation_result = json.loads(reputation.content[0].text)
                        reputation_data = reputation_result["data"]
                        self.assertEqual(reputation_result["special_reputation_progress"], {})
                        self.assertEqual(reputation_result["standard_reputation_progress"], {
                            "10": {"capped": False, "earned_in_rank": 50, "rank_width": 100,
                                   "remaining_to_next_rank": 50},
                        })
                        more_factions = json.loads(json.dumps(forever))
                        more_factions["character"]["realm"] = "First"
                        factions = more_factions["sections"]["reputation"]["entries"]
                        factions["11"] = {**factions["10"], "reaction": 8, "next_threshold": 0}
                        factions["12"] = {**factions["10"], "kind": "renown", "renown": {
                            "level": 5, "earned_in_level": 12, "level_threshold": 100, "capped": False,
                        }}
                        factions["13"] = {**factions["10"], "standing": -101}
                        factions["14"] = {**factions["10"], "kind": "friendship", "friendship": {
                            "standing": 35, "threshold": 20, "next_threshold": 100,
                        }}
                        factions["15"] = {**factions["10"], "kind": "paragon", "paragon": {
                            "current_value": 250, "reward_threshold": 100, "reward_quest_id": 314,
                            "reward_pending": True, "level_too_low": False,
                        }}
                        factions["16"] = {**factions["15"], "paragon": {
                            **factions["15"]["paragon"], "level_too_low": True,
                        }}
                        factions["17"] = {**factions["14"], "friendship": {
                            "standing": 100, "threshold": 100,
                        }}
                        factions["18"] = {**factions["12"], "renown": {
                            **factions["12"]["renown"], "capped": True,
                        }}
                        factions["19"] = {**factions["15"], "paragon": {
                            **factions["15"]["paragon"], "reward_pending": False,
                        }}
                        factions["20"] = factions["12"]
                        factions["21"] = {**factions["12"], "renown": {
                            **factions["12"]["renown"], "earned_in_level": 101,
                        }}
                        more_key = SnapshotStore(Path(folder)).ingest(more_factions)
                        reward_summary = await session.call_tool("character_summary", {"snapshot_id": more_key})
                        self.assertFalse(reward_summary.isError)
                        paragon = json.loads(reward_summary.content[0].text)["paragon_reward_summary"]
                        reward = {"name": "Fixture faction", "account_wide": False, "reward_quest_id": 314}
                        self.assertEqual(paragon["pending_by_faction_id"], {"15": reward})
                        self.assertEqual(paragon["level_restricted_by_faction_id"], {"16": reward})
                        more = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": more_key, "section": "reputation"})
                        more_data = json.loads(more.content[0].text)
                        self.assertEqual(more_data["data"]["entries"]["12"]["renown"], factions["12"]["renown"])
                        self.assertEqual(more_data["data"]["entries"]["14"]["friendship"], factions["14"]["friendship"])
                        self.assertEqual(more_data["data"]["entries"]["15"]["paragon"], factions["15"]["paragon"])
                        self.assertEqual(more_data["special_reputation_progress"], {
                            "12": {"kind": "renown", "capped": False, "remaining_to_next_rank": 88},
                            "14": {"kind": "friendship", "capped": False, "remaining_to_next_rank": 65},
                            "15": {"kind": "paragon", "reward_status": "pending"},
                            "16": {"kind": "paragon", "reward_status": "pending_level_restricted"},
                            "17": {"kind": "friendship", "capped": True, "remaining_to_next_rank": None},
                            "18": {"kind": "renown", "capped": True, "remaining_to_next_rank": None},
                            "19": {"kind": "paragon", "reward_status": "no_pending_reward"},
                            "20": {"kind": "renown", "capped": False, "remaining_to_next_rank": 88},
                            "21": {"kind": "renown", "capped": False, "remaining_to_next_rank": None},
                        })
                        progress = more_data["standard_reputation_progress"]
                        self.assertEqual(set(progress), {"10", "11"})
                        self.assertEqual(progress["11"], {"capped": True})
                        self.assertIs(more_data["live"], False)
                        next_factions = json.loads(json.dumps(more_factions))
                        next_factions["captured_at"] = "2025-01-02T00:00:00Z"
                        next_rows = next_factions["sections"]["reputation"]["entries"]
                        next_rows["12"]["renown"].update(level=6, earned_in_level=2)
                        next_rows["14"]["friendship"].update(standing=105, threshold=100, next_threshold=200)
                        next_rows["15"]["paragon"].update(reward_pending=False, level_too_low=True)
                        next_rows["18"]["renown"].update(capped=False, earned_in_level=25)
                        next_rows["20"]["renown"]["earned_in_level"] = 40
                        next_rows["21"]["renown"]["earned_in_level"] = 12
                        next_rows["16"]["account_wide"] = True
                        next_key = SnapshotStore(Path(folder)).ingest(next_factions)
                        async def compare_faction(faction_id, before_id=more_key):
                            response = await session.call_tool("compare_snapshots", {
                                "before_id": before_id, "after_id": next_key, "faction_id": faction_id})
                            self.assertFalse(response.isError)
                            return json.loads(response.content[0].text)["reputation_change"]["special_progress_change"]
                        self.assertEqual(await compare_faction(12), {
                            "kind": "renown", "level_delta": 1, "earned_in_level_delta": None,
                            "capped_changed": False,
                        })
                        self.assertEqual(await compare_faction(14), {"kind": "friendship", "standing_delta": 70})
                        self.assertEqual(await compare_faction(15), {
                            "kind": "paragon", "reward_pending_changed": True, "level_restriction_changed": True,
                        })
                        self.assertEqual(await compare_faction(18), {
                            "kind": "renown", "level_delta": 0, "earned_in_level_delta": None,
                            "capped_changed": True,
                        })
                        self.assertEqual(await compare_faction(20), {
                            "kind": "renown", "level_delta": 0, "earned_in_level_delta": 28,
                            "capped_changed": False,
                        })
                        self.assertIsNone((await compare_faction(21))["earned_in_level_delta"])
                        self.assertIsNone(await compare_faction(16))
                        self.assertIsNone(await compare_faction(12, forever_key))
                        missing = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": key, "section": "reputation"})
                        missing_data = json.loads(missing.content[0].text)
                        self.assertIsNone(missing_data["standard_reputation_progress"])
                        self.assertIsNone(missing_data["special_reputation_progress"])
                        self.assertEqual(reputation_data["coverage"], "visible_faction_rows_only")
                        self.assertEqual(reputation_data["entries"]["10"]["standing"], -50)
                        reputation_after = json.loads(json.dumps(forever))
                        reputation_after["character"]["realm"] = "First"
                        reputation_after["sections"]["reputation"]["entries"]["10"]["standing"] = -20
                        reputation_after["sections"]["wallet"]["copper"] = 0
                        reputation_after["sections"]["currency_balances"]["entries"]["100"]["quantity"] = 3
                        reputation_after["sections"]["tracked_achievements"]["entries"]["101"]["criteria"]["1"]["quantity"] = 2
                        reputation_after["sections"]["quests"]["42"]["objectives"]["1"]["fulfilled"] = 2
                        after_key = SnapshotStore(Path(folder)).ingest(reputation_after)
                        rep_diff = await session.call_tool("compare_snapshots", {
                            "before_id":forever_key,"after_id":after_key,"faction_id":10,"wallet":True})
                        self.assertFalse(rep_diff.isError)
                        self.assertEqual(json.loads(rep_diff.content[0].text)["sections"]["reputation"]["changed_paths"],
                                         ["/entries/10/standing"])
                        self.assertEqual(json.loads(rep_diff.content[0].text)["sections"]["currency_balances"]["changed_paths"],
                                         ["/entries/100/quantity"])
                        self.assertEqual(json.loads(rep_diff.content[0].text)["sections"]["tracked_achievements"]["changed_paths"],
                                         ["/entries/101/criteria/1/quantity"])
                        self.assertEqual(json.loads(rep_diff.content[0].text)["sections"]["quests"]["changed_paths"],
                                         ["/42/objectives/1/fulfilled"])
                        standing = json.loads(rep_diff.content[0].text)["reputation_change"]
                        self.assertEqual(standing["standing_delta"], 30)
                        self.assertEqual(standing["before"]["standing"], -50)
                        self.assertEqual(standing["after"]["standing"], -20)
                        self.assertEqual(json.loads(rep_diff.content[0].text)["wallet_change"], {
                            "before_copper": 5_000_000_001, "after_copper": 0, "delta_copper": -5_000_000_001})
                        reputation_after["sections"]["reputation"]["entries"]["10"]["kind"] = "renown"
                        del reputation_after["sections"]["wallet"]
                        renown_key = SnapshotStore(Path(folder)).ingest(reputation_after)
                        renown = await session.call_tool("compare_snapshots", {
                            "before_id":after_key,"after_id":renown_key,"faction_id":10,"wallet":True})
                        self.assertFalse(renown.isError)
                        self.assertIsNone(json.loads(renown.content[0].text)["reputation_change"]["standing_delta"])
                        self.assertEqual(json.loads(renown.content[0].text)["wallet_change"], {
                            "before_copper": 0, "after_copper": None, "delta_copper": None})
                        reputation_after["sections"]["quests"]["42"]["ready_for_turn_in"] = True
                        reputation_after["sections"]["quests"]["42"]["timer"]["remaining_seconds"] = 0
                        reputation_after["sections"]["quests"]["42"]["objectives"]["2"]["finished"] = True
                        reputation_after["sections"]["tracked_achievements"]["entries"]["101"]["completed"] = True
                        criteria = reputation_after["sections"]["tracked_achievements"]["entries"]["101"]["criteria"]
                        criteria["1"], criteria["2"] = criteria["2"], criteria["1"]
                        criteria["2"]["quantity"] = 3
                        profession = reputation_after["sections"]["professions"].pop("1")
                        profession["rank"] = 25
                        reputation_after["sections"]["professions"]["2"] = profession
                        progress_key = SnapshotStore(Path(folder)).ingest(reputation_after)
                        progress = await session.call_tool("compare_snapshots", {
                            "before_id": after_key, "after_id": progress_key, "quest_id": 42,
                            "achievement_id": 101, "profession_skill_line": 164})
                        self.assertFalse(progress.isError)
                        progress_data = json.loads(progress.content[0].text)
                        quest = progress_data["quest_change"]
                        self.assertTrue(quest["ready_for_turn_in_changed"])
                        self.assertIsNone(quest["failed_changed"])
                        self.assertEqual(quest["timer_change"], {
                            "before_observed": True, "after_observed": True,
                            "remaining_seconds_delta": -74, "total_seconds_delta": 0,
                        })
                        self.assertEqual(quest["objective_counts_by_kind"], {
                            "before": {"event": {"finished": 0, "unfinished": 1},
                                       "item": {"finished": 1, "unfinished": 2}},
                            "after": {"event": {"finished": 1, "unfinished": 0},
                                      "item": {"finished": 1, "unfinished": 2}},
                        })
                        self.assertEqual(quest["after"]["objectives"]["1"]["fulfilled"], 2)
                        achievement = progress_data["achievement_change"]
                        self.assertTrue(achievement["completed_changed"])
                        self.assertFalse(achievement["earned_by_character_changed"])
                        comparison = achievement["criterion_comparison"]
                        self.assertEqual(comparison["matched_criterion_count"], 3)
                        self.assertEqual(len(comparison["changes"]), 1)
                        self.assertEqual(comparison["changes"][0]["criterion_id"], 301)
                        self.assertEqual(comparison["changes"][0]["quantity_delta"], 1)
                        profession_change = progress_data["profession_change"]
                        self.assertEqual(profession_change["rank_delta"], 5)
                        self.assertEqual(profession_change["before_status"], "observed")
                        self.assertEqual(profession_change["after"]["rank"], 25)
                        reputation_after["sections"]["professions"]["1"] = dict(profession, rank=30)
                        ambiguous_key = SnapshotStore(Path(folder)).ingest(reputation_after)
                        ambiguous = await session.call_tool("compare_snapshots", {
                            "before_id": progress_key, "after_id": ambiguous_key, "profession_skill_line": 164})
                        self.assertFalse(ambiguous.isError)
                        ambiguous_profession = json.loads(ambiguous.content[0].text)["profession_change"]
                        self.assertEqual(ambiguous_profession["after_status"], "ambiguous")
                        self.assertIsNone(ambiguous_profession["rank_delta"])
                        recipe = await session.call_tool("review_forever_recipe", {"snapshot_id":forever_key,"crafts":2})
                        self.assertFalse(recipe.isError)
                        recipe_data = json.loads(recipe.content[0].text)
                        self.assertEqual(recipe_data["materials"], [{
                            "item_id":744,"required":6,"quantity_observed":1,"shortfall_vs_observed":5,
                            "quantity_by_storage":{"bags":None,"character_bank":1,"account_bank":0},
                            "shortfall_vs_bags":None,
                            "suggested_bank_withdrawals":None,
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
                        refreshed = json.loads(json.dumps(forever))
                        refreshed["captured_at"] = "2025-01-02T00:00:00Z"
                        del refreshed["sections"]["selected_recipe"]
                        del refreshed["sections"]["character_bank"]
                        refreshed["sections"]["bags"]["0"]["1"]["count"] = 12
                        inventory_key = SnapshotStore(Path(folder)).ingest(refreshed)
                        recipe_request = {"snapshot_id": stocked_key, "crafts": 1,
                                          "inventory_snapshot_id": inventory_key}
                        refreshed_review = await session.call_tool("review_forever_recipe", recipe_request)
                        self.assertFalse(refreshed_review.isError)
                        refresh = json.loads(refreshed_review.content[0].text)
                        self.assertEqual(refresh["inventory_snapshot_id"], inventory_key)
                        self.assertEqual(refresh["inventory_captured_at"], "2025-01-02T00:00:00Z")
                        self.assertEqual(refresh["captured_at"], forever["captured_at"])
                        self.assertEqual(refresh["unobserved_storage"], ["character_bank"])
                        self.assertEqual(next(row["quantity_observed"] for row in refresh["materials"]
                                              if row["item_id"] == 744), 12)
                        mismatch = await session.call_tool("review_forever_recipe", {
                            **recipe_request, "inventory_snapshot_id": forever_key})
                        self.assertTrue(mismatch.isError)
                        merchant = json.loads(json.dumps(forever))
                        del merchant["sections"]["selected_recipe"]
                        merchant["sections"]["merchant_stock"] = {
                            "1": {"item_id": 744, "bundle_size": 5, "price_copper": 12,
                                  "extended_cost": False, "available": -1},
                            "2": {"item_id": 744, "bundle_size": 5, "price_copper": 1,
                                  "extended_cost": False, "available": 0},
                        }
                        merchant["sections"]["wallet"] = {"copper": 100}
                        merchant_key = SnapshotStore(Path(folder)).ingest(merchant)
                        priced_inventory = json.loads(json.dumps(refreshed))
                        priced_inventory["sections"]["wallet"] = {"copper": 11}
                        priced_inventory_key = SnapshotStore(Path(folder)).ingest(priced_inventory)
                        priced = await session.call_tool("review_forever_recipe", {
                            **recipe_request, "crafts": 5, "inventory_snapshot_id": priced_inventory_key,
                            "merchant_snapshot_id": merchant_key, "merchant_reagent_plan": True,
                        })
                        self.assertFalse(priced.isError)
                        basket = json.loads(priced.content[0].text)["merchant_reagent_plan"]
                        self.assertEqual(basket["merchant_captured_at"], merchant["captured_at"])
                        reagent = next(row for row in basket["reagents"] if row["item_id"] == 744)
                        self.assertEqual(reagent["shortfall_vs_observed"], 3)
                        self.assertEqual(reagent["lowest_unlimited_copper_only_offer"],
                                         {"slot": 1, "required_copper": 12})
                        self.assertIsNone(basket["unlimited_copper_only_total"])
                        priced_inventory["sections"]["bags"]["0"]["2"]["count"] = 10
                        priced_inventory_key = SnapshotStore(Path(folder)).ingest(priced_inventory)
                        priced = await session.call_tool("review_forever_recipe", {
                            **recipe_request, "crafts": 5, "inventory_snapshot_id": priced_inventory_key,
                            "merchant_snapshot_id": merchant_key, "merchant_reagent_plan": True,
                        })
                        self.assertFalse(priced.isError)
                        basket = json.loads(priced.content[0].text)["merchant_reagent_plan"]
                        self.assertEqual(basket["unlimited_copper_only_total"], 12)
                        self.assertFalse(basket["unlimited_copper_only_affordable"])
                        self.assertEqual(basket["unlimited_copper_only_shortfall"], 1)
                        second_recipe = json.loads(json.dumps(forever))
                        second_recipe["sections"]["selected_recipe"].update({
                            "recipe_id": 101, "reagents": {"744": 5}, "relative_difficulty": "trivial",
                            "output_item_id": 456, "base_output_quantity": {"minimum": 2, "maximum": 3},
                            "can_skill_up": False, "skill_ups_reported": 0,
                            "cooldown": {"remaining_seconds": 30, "daily": False, "charges": 0, "max_charges": 3},
                            "crafting_conditions": {"1": {"name": "Fixture area", "kind": "area", "met": False}}})
                        second_key = SnapshotStore(Path(folder)).ingest(second_recipe)
                        batch_request = {**recipe_request, "additional_recipes": [
                            {"snapshot_id": second_key, "crafts": 2}]}
                        batch = await session.call_tool("review_forever_recipe", batch_request)
                        self.assertFalse(batch.isError)
                        batch_data = json.loads(batch.content[0].text)
                        combined = {row["item_id"]: row for row in batch_data["combined_materials"]}
                        self.assertEqual(combined[744], {
                            "item_id": 744, "required": 13, "quantity_observed": 12, "shortfall_vs_observed": 1,
                            "quantity_by_storage": {"bags": 12, "character_bank": None, "account_bank": 0},
                            "shortfall_vs_bags": 1,
                        })
                        self.assertEqual(combined[10]["shortfall_vs_bags"], 0)
                        bank_batch = await session.call_tool("review_forever_recipe", {
                            **batch_request, "inventory_snapshot_id": stocked_key, "crafts": 2,
                            "additional_recipes": [{"snapshot_id": second_key, "crafts": 1}]})
                        self.assertFalse(bank_batch.isError)
                        bank_data = json.loads(bank_batch.content[0].text)
                        bank_materials = {row["item_id"]: row for row in bank_data["combined_materials"]}
                        self.assertEqual(bank_materials[744]["quantity_by_storage"]["character_bank"], 1)
                        self.assertEqual(bank_materials[744]["shortfall_vs_bags"], 3)
                        self.assertEqual(bank_materials[744]["shortfall_vs_observed"], 2)
                        self.assertEqual(batch_data["additional_recipe_reviews"][0]["snapshot_id"], second_key)
                        training_recipe = batch_data["additional_recipe_reviews"][0]["recipe"]
                        self.assertEqual(training_recipe["relative_difficulty"], "trivial")
                        self.assertIs(training_recipe["can_skill_up"], False)
                        self.assertEqual(training_recipe["skill_ups_reported"], 0)
                        self.assertIsNone(batch_data["base_output_if_completed"])
                        self.assertEqual(batch_data["additional_recipe_reviews"][0]["base_output_if_completed"],
                                         {"item_id": 456, "minimum": 4, "maximum": 6})
                        self.assertEqual(batch_data["combined_base_output_if_completed"], {
                            "items": [{"item_id": 456, "minimum": 4, "maximum": 6}],
                            "recipes_without_observed_output_item": 1,
                        })
                        target = await session.call_tool("review_forever_recipe", {
                            "snapshot_id": second_key, "target_output_quantity": 5})
                        self.assertFalse(target.isError)
                        target_data = json.loads(target.content[0].text)
                        self.assertEqual(target_data["target_output_quantity"], 5)
                        self.assertEqual(target_data["requested_crafts"], 3)
                        self.assertEqual(target_data["shortfall_vs_observed_recipe_charges"], 3)
                        self.assertEqual(target_data["recipe"]["cooldown"]["remaining_seconds"], 30)
                        self.assertIsNone(batch_data["shortfall_vs_observed_recipe_charges"])
                        self.assertIsNone(batch_data["unmet_crafting_conditions"])
                        self.assertEqual(target_data["unmet_crafting_conditions"], {
                            "1": {"name": "Fixture area", "kind": "area", "met": False},
                        })
                        self.assertEqual(target_data["materials"][0]["required"], 15)
                        self.assertEqual(target_data["base_output_if_completed"],
                                         {"item_id": 456, "minimum": 6, "maximum": 9})
                        target_batch = await session.call_tool("review_forever_recipe", {
                            **batch_request, "additional_recipes": [
                                {"snapshot_id": second_key, "target_output_quantity": 3}]})
                        self.assertFalse(target_batch.isError)
                        target_batch_data = json.loads(target_batch.content[0].text)
                        self.assertEqual(target_batch_data["combined_materials"], batch_data["combined_materials"])
                        self.assertEqual(target_batch_data["additional_recipe_reviews"][0]["target_output_quantity"], 3)
                        for invalid in ({}, {"target_output_quantity": 0}, {"target_output_quantity": 2001},
                                        {"target_output_quantity": 1, "crafts": 1}):
                            rejected = await session.call_tool("review_forever_recipe", {
                                "snapshot_id": second_key, **invalid})
                            self.assertTrue(rejected.isError)
                        unavailable_output = await session.call_tool("review_forever_recipe", {
                            "snapshot_id": stocked_key, "target_output_quantity": 1})
                        self.assertTrue(unavailable_output.isError)
                        extra_output = json.loads(json.dumps(second_recipe))
                        extra_output["sections"]["selected_recipe"]["recipe_id"] = 102
                        extra_key = SnapshotStore(Path(folder)).ingest(extra_output)
                        extra_request = {**batch_request, "additional_recipes": [
                            *batch_request["additional_recipes"], {"snapshot_id": extra_key, "crafts": 1}]}
                        async def combined_output(request):
                            response = await session.call_tool("review_forever_recipe", request)
                            self.assertFalse(response.isError)
                            return json.loads(response.content[0].text)["combined_base_output_if_completed"]
                        self.assertEqual((await combined_output(extra_request))["items"],
                                         [{"item_id": 456, "minimum": 6, "maximum": 9}])
                        del extra_output["sections"]["selected_recipe"]["base_output_quantity"]
                        unknown_output_key = SnapshotStore(Path(folder)).ingest(extra_output)
                        extra_request["additional_recipes"][-1]["snapshot_id"] = unknown_output_key
                        self.assertEqual((await combined_output(extra_request))["items"],
                                         [{"item_id": 456, "minimum": None, "maximum": None}])
                        intermediate_user = json.loads(json.dumps(second_recipe))
                        intermediate_user["sections"]["selected_recipe"].update(
                            recipe_id=103, output_item_id=789, reagents={"456": 5})
                        intermediate_key = SnapshotStore(Path(folder)).ingest(intermediate_user)
                        producer = {"snapshot_id": second_key, "crafts": 2}
                        consumer = {"snapshot_id": intermediate_key, "crafts": 1}
                        async def starting_materials(*recipes, stock_key=inventory_key):
                            response = await session.call_tool("review_forever_recipe", {
                                **batch_request, "inventory_snapshot_id": stock_key,
                                "additional_recipes": list(recipes)})
                            self.assertFalse(response.isError)
                            return json.loads(response.content[0].text)["ordered_starting_materials"]
                        forward = await starting_materials(producer, consumer)
                        reverse = await starting_materials(consumer, producer)
                        self.assertEqual(next(row for row in forward if row["item_id"] == 456),
                                         {"item_id": 456, "required_before_batch": 1,
                                          "quantity_observed": 0, "shortfall_vs_observed": 1,
                                          "shortfall_vs_bags": 1,
                                          "suggested_bank_withdrawals": {"character_bank": None, "account_bank": 0}})
                        self.assertEqual(next(row for row in reverse if row["item_id"] == 456)
                                         ["required_before_batch"], 5)
                        unknown = await starting_materials(
                            {"snapshot_id": unknown_output_key, "crafts": 2}, consumer)
                        self.assertEqual(next(row for row in unknown if row["item_id"] == 456)
                                         ["required_before_batch"], 5)
                        bank_stock = json.loads(json.dumps(refreshed))
                        bank_stock["sections"]["character_bank"] = {
                            "6": {"1": {"item_id": 456, "count": 2}}}
                        bank_stock["sections"]["account_bank"] = {
                            "13": {"1": {"item_id": 456, "count": 9}}}
                        bank_stock_key = SnapshotStore(Path(folder)).ingest(bank_stock)
                        withdrawal_plan = await starting_materials(consumer, producer, stock_key=bank_stock_key)
                        item_plan = next(row for row in withdrawal_plan if row["item_id"] == 456)
                        self.assertEqual(item_plan["shortfall_vs_observed"], 0)
                        self.assertEqual(item_plan["shortfall_vs_bags"], 5)
                        self.assertEqual(item_plan["suggested_bank_withdrawals"],
                                         {"character_bank": 2, "account_bank": 3})
                        del bank_stock["sections"]["bags"]
                        missing_bags_key = SnapshotStore(Path(folder)).ingest(bank_stock)
                        bank_only = await starting_materials(consumer, producer, stock_key=missing_bags_key)
                        self.assertIsNone(next(row for row in bank_only if row["item_id"] == 456)
                                          ["suggested_bank_withdrawals"])
                        duplicate = await session.call_tool("review_forever_recipe", {
                            **recipe_request, "additional_recipes": [{"snapshot_id": stocked_key, "crafts": 2}]})
                        self.assertTrue(duplicate.isError)
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
                        across = await session.call_tool("find_snapshot_item", {
                            **search, "additional_snapshot_ids": [forever_key]})
                        self.assertFalse(across.isError)
                        across_data = json.loads(across.content[0].text)
                        self.assertEqual(across_data["quantity_by_storage"]["character_bank"], 1)
                        extra = across_data["additional_snapshot_results"][0]
                        self.assertEqual(extra["snapshot_id"], forever_key)
                        self.assertEqual(extra["matched_locations"], 1)
                        self.assertEqual(extra["quantity_by_storage"]["character_bank"], 1)
                        self.assertIsNone(extra["quantity_by_storage"]["bags"])
                        for invalid in ([key], [capped_key]):
                            self.assertTrue((await session.call_tool("find_snapshot_item", {
                                **search, "additional_snapshot_ids": invalid})).isError)
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
                        self.assertEqual(currency["before"]["earned_this_week"], 10)
                        self.assertEqual(currency["weekly_earned_delta"], -9)
                        self.assertEqual(currency["weekly_headroom_change"], {
                            "before_remaining": 30, "after_remaining": 39,
                            "reached_cap": False, "left_cap": False,
                        })
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
                        self.assertIsNone(currency["weekly_earned_delta"])
                        self.assertIsNone(currency["weekly_headroom_change"])
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
                        self.assertIsNone(json.loads(summary.content[0].text)["quest_checklist"])
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
