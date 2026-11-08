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
                            "recrafts": {"100": {"learned": True}}}
                        forever["sections"]["selected_recipe"] = {"recipe_id":100,"name":"Fixture recipe","learned":True,
                            "requirements":"fixed_items","reagents":{"744":3}}
                        forever["sections"]["flight_map"] = {"map_id": 1414, "entries": {
                            "1": {"node_id": 12, "name": "Fixture origin", "state": "current",
                                  "map_transition": False},
                            "2": {"node_id": 14, "name": "Fixture destination", "state": "reachable",
                                  "map_transition": False, "fare_copper": 0},
                        }}
                        forever["sections"]["trainer_services"] = {"kind": "tradeskills", "entries": {
                            "1": {"name": "Fixture Potion", "status": "available", "price_copper": 10},
                            "2": {"name": "Other Lesson", "status": "used"},
                            "3": {"name": "Expensive Lesson", "status": "available", "price_copper": 5_000_000_001},
                            "4": {"name": "Blocked Lesson", "status": "unavailable",
                                  "skill_requirement": {"name": "Alchemy", "required_rank": 50, "met": False},
                                  "ability_requirements": {
                                      "1": {"name": "Brew", "met": False},
                                      "2": {"name": "Distil", "met": False},
                                  }},
                            "5": {"name": "Brew", "status": "available", "price_copper": 15},
                        }}
                        forever["sections"]["skill_lines"] = {
                            "171": {"name": "Alchemy", "rank": 35, "max_rank": 75, "modifier": 10},
                        }
                        forever["sections"]["spellbook"] = {"coverage": "visible_player_spell_slots", "entries": {
                            "1": {"base_spell_id": 77, "spell_id": 78, "name": "Brew"},
                        }}
                        forever_key = SnapshotStore(Path(folder)).ingest(forever)
                        forever["character"]["realm"] = "Second"
                        SnapshotStore(Path(folder)).ingest(forever)
                        filtered = await session.call_tool("list_snapshots", {"family":"forever",
                            "character":{"name":"example","realm":"FIRST"},"limit":1})
                        filtered_data = json.loads(filtered.content[0].text)
                        self.assertEqual([row["snapshot_id"] for row in filtered_data["snapshots"]], [forever_key])
                        trainers = await session.call_tool("list_snapshots", {
                            "family": "forever", "trainer_service_name": " PoTiOn ",
                            "trainer_service_status": "available",
                            "trainer_max_price_copper": 10,
                            "trainer_affordable_only": True,
                            "character": {"name": "example", "realm": "FIRST"}, "limit": 1,
                        })
                        self.assertFalse(trainers.isError)
                        visits = json.loads(trainers.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in visits], [forever_key])
                        self.assertEqual(visits[0]["trainer_match"], {
                            "lesson_count": 1, "kind": "tradeskills", "captured_player_map_id": None,
                        })
                        ordered_trainer = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": forever_key, "section": "trainer_services",
                            "trainer_service_slots": [1, 2], "trainer_wallet_projection": True,
                        })
                        self.assertFalse(ordered_trainer.isError)
                        ordered_budget = json.loads(ordered_trainer.content[0].text)[
                            "trainer_wallet_projection"]["ordered_lesson_budget"]
                        self.assertEqual(ordered_budget, {
                            "covered_prefix_lessons": 1,
                            "first_uncovered_lesson": {"slot": 2, "reason": "lesson_not_available"},
                        })
                        blocked_trainer = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": forever_key, "section": "trainer_services",
                            "trainer_service_slots": [1, 4, 5], "trainer_wallet_projection": True,
                            "trainer_requirements_review": True,
                        })
                        self.assertFalse(blocked_trainer.isError)
                        blocked = json.loads(blocked_trainer.content[0].text)[
                            "trainer_wallet_projection"]["ordered_lesson_budget"]["first_uncovered_lesson"]
                        self.assertEqual(blocked, {
                            "slot": 4, "reason": "lesson_not_available",
                            "observed_requirements": {"level_shortfall": None,
                                "skill_requirement_unmet": True, "unmet_ability_names": ["Brew", "Distil"],
                                "unmet_ability_names_observed_in_spellbook": ["Brew"],
                                "available_lesson_slots_by_unmet_ability_name": {"Brew": [5]},
                                "captured_skill_rank_comparison": {"skill_line_id": 171,
                                    "captured_rank": 35, "required_rank": 50, "rank_shortfall": 15}},
                        })
                        short_trainer = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": forever_key, "section": "trainer_services",
                            "trainer_service_slots": [1, 3], "trainer_wallet_projection": True,
                        })
                        self.assertFalse(short_trainer.isError)
                        self.assertEqual(json.loads(short_trainer.content[0].text)[
                            "trainer_wallet_projection"]["ordered_lesson_budget"], {
                            "covered_prefix_lessons": 1,
                            "first_uncovered_lesson": {
                                "slot": 3, "reason": "insufficient_balance", "additional_balance_needed": 10,
                            },
                        })
                        crossed = await session.call_tool("list_snapshots", {
                            "family": "forever", "trainer_service_name": "Potion",
                            "trainer_service_status": "used",
                        })
                        self.assertFalse(crossed.isError)
                        self.assertEqual(json.loads(crossed.content[0].text)["snapshots"], [])
                        too_expensive = await session.call_tool("list_snapshots", {
                            "family": "forever", "trainer_service_name": "Potion",
                            "trainer_max_price_copper": 9,
                        })
                        self.assertFalse(too_expensive.isError)
                        self.assertEqual(json.loads(too_expensive.content[0].text)["snapshots"], [])
                        flight = await session.call_tool("list_snapshots", {
                            "family": "forever", "character": {"name": "example", "realm": "FIRST"},
                            "flight_origin_node_id": 12, "flight_destination_node_id": 14,
                            "flight_max_fare_copper": 0, "limit": 1,
                        })
                        self.assertFalse(flight.isError)
                        route = json.loads(flight.content[0].text)["snapshots"][0]
                        self.assertEqual(route["snapshot_id"], forever_key)
                        self.assertEqual(route["flight_match"], {
                            "map_id": 1414, "origin_node_id": 12, "origin_name": "Fixture origin",
                            "destination_node_id": 14, "destination_name": "Fixture destination",
                            "fare_copper": 0,
                        })
                        other_origin = await session.call_tool("list_snapshots", {
                            "family": "forever", "flight_origin_node_id": 13, "flight_destination_node_id": 14,
                        })
                        self.assertFalse(other_origin.isError)
                        self.assertEqual(json.loads(other_origin.content[0].text)["snapshots"], [])
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            "family": "forever", "flight_origin_node_id": 12,
                        })).isError)
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            "family": "forever", "flight_max_fare_copper": 0,
                        })).isError)
                        self.assertFalse(filtered_data["has_more"])
                        affordable_capture = json.loads(json.dumps(forever))
                        affordable_capture["sections"]["flight_map"]["entries"]["2"]["node_id"] = 15
                        affordable_capture["sections"].pop("wallet", None)
                        SnapshotStore(Path(folder)).ingest(affordable_capture)
                        affordable_capture["sections"]["wallet"] = {"copper": 0}
                        affordable_capture["sections"]["stats"] = {
                            "level": 10, "class": "Hunter", "race": "Dwarf",
                            "client_version": "fixture", "map_id": 27,
                        }
                        affordable_key = SnapshotStore(Path(folder)).ingest(affordable_capture)
                        affordable = await session.call_tool("list_snapshots", {
                            "family": "forever", "flight_destination_node_id": 15,
                            "flight_affordable_only": True, "flight_departure_map_id": 27,
                        })
                        self.assertFalse(affordable.isError)
                        affordable_rows = json.loads(affordable.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in affordable_rows], [affordable_key])
                        self.assertNotIn("wallet", affordable_rows[0])
                        self.assertEqual(affordable_rows[0]["flight_match"]["captured_departure_map_id"], 27)
                        self.assertEqual(affordable_rows[0]["flight_match"]["map_id"], 1414)
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            "family": "forever", "flight_affordable_only": True,
                        })).isError)
                        bank = await session.call_tool("inspect_snapshot", {"snapshot_id":forever_key,"section":"character_bank"})
                        route_start = json.loads(json.dumps(forever))
                        route_start["sections"]["repair_quote"] = {"copper": 2}
                        route_start["sections"]["flight_map"]["entries"]["3"] = {
                            "node_id": 15, "name": "Route target", "state": "reachable",
                            "map_transition": False, "fare_copper": 20,
                        }
                        route_start_id = SnapshotStore(Path(folder)).ingest(route_start)
                        route_onward = json.loads(json.dumps(route_start))
                        route_onward["sections"]["repair_quote"] = {"copper": 999}
                        route_onward["sections"]["flight_map"]["entries"] = {
                            "1": {"node_id": 14, "name": "Connection", "state": "current",
                                  "map_transition": False},
                            "2": {"node_id": 15, "name": "Route target", "state": "reachable",
                                  "map_transition": False, "fare_copper": 3},
                        }
                        route_onward_id = SnapshotStore(Path(folder)).ingest(route_onward)
                        route_request = {"snapshot_id": route_start_id, "section": "flight_map",
                                         "flight_route_search": {"destination_node_id": 15,
                                                                 "onward_snapshot_ids": [route_onward_id]}}
                        planned = await session.call_tool("inspect_snapshot", route_request)
                        self.assertFalse(planned.isError)
                        plan = json.loads(planned.content[0].text)["flight_route_search"]
                        self.assertEqual(plan["total_fare_copper"], 3)
                        direct_request = {
                            **route_request, "flight_route_search": {
                                **route_request["flight_route_search"], "max_legs": 1,
                            },
                        }
                        direct = await session.call_tool("inspect_snapshot", direct_request)
                        self.assertFalse(direct.isError)
                        direct_plan = json.loads(direct.content[0].text)["flight_route_search"]
                        self.assertEqual(direct_plan["total_fare_copper"], 20)
                        self.assertEqual(len(direct_plan["legs"]), 1)
                        fewest_request = {
                            **route_request, "flight_route_search": {
                                **route_request["flight_route_search"], "objective": "fewest_legs",
                            },
                        }
                        fewest = await session.call_tool("inspect_snapshot", fewest_request)
                        self.assertFalse(fewest.isError)
                        self.assertEqual(json.loads(fewest.content[0].text)["flight_route_search"],
                                         direct_plan)
                        constrained_request = {
                            **fewest_request, "flight_wallet_reserve_copper": 4_999_999_996,
                            "flight_route_search": {
                                **fewest_request["flight_route_search"],
                                "require_affordable": True, "include_repair_quote": True,
                            },
                        }
                        constrained = await session.call_tool("inspect_snapshot", constrained_request)
                        self.assertFalse(constrained.isError)
                        constrained_plan = json.loads(constrained.content[0].text)["flight_route_search"]
                        self.assertEqual(constrained_plan["total_fare_copper"], 3)
                        self.assertTrue(constrained_plan["affordable_with_repairs_and_reserve"])
                        too_expensive = await session.call_tool("inspect_snapshot", {
                            **constrained_request, "flight_wallet_reserve_copper": 4_999_999_997,
                        })
                        self.assertFalse(too_expensive.isError)
                        self.assertEqual(json.loads(too_expensive.content[0].text)[
                            "flight_route_search"]["status"], "no_affordable_route")
                        self.assertTrue((await session.call_tool("inspect_snapshot", {
                            **fewest_request, "flight_route_search": {
                                **route_request["flight_route_search"], "objective": "fastest",
                            },
                        })).isError)
                        for invalid in (0, 9):
                            self.assertTrue((await session.call_tool("inspect_snapshot", {
                                **direct_request, "flight_route_search": {
                                    **route_request["flight_route_search"], "max_legs": invalid,
                                },
                            })).isError)
                        self.assertEqual([leg["snapshot_id"] for leg in plan["legs"]],
                                         [route_start_id, route_onward_id])
                        self.assertTrue(plan["affordable_from_starting_wallet"])
                        self.assertNotIn("affordable_with_reserve", plan)
                        self.assertNotIn("starting_repair_quote_copper", plan)
                        for reserve, expected in [(4_999_999_996, True), (4_999_999_997, False)]:
                            repair_budget = await session.call_tool("inspect_snapshot", {
                                **route_request, "flight_wallet_reserve_copper": reserve,
                                "flight_route_search": {
                                    **route_request["flight_route_search"], "include_repair_quote": True,
                                },
                            })
                            self.assertFalse(repair_budget.isError)
                            repair_plan = json.loads(repair_budget.content[0].text)["flight_route_search"]
                            self.assertEqual(repair_plan["starting_repair_quote_copper"], 2)
                            self.assertTrue(repair_plan["affordable_with_reserve"])
                            self.assertEqual(repair_plan["affordable_with_repairs_and_reserve"], expected)
                        for reserve, expected in [(4_999_999_998, True), (4_999_999_999, False)]:
                            reserved = await session.call_tool("inspect_snapshot", {
                                **route_request, "flight_wallet_reserve_copper": reserve,
                            })
                            self.assertFalse(reserved.isError)
                            result = json.loads(reserved.content[0].text)
                            self.assertEqual(result["flight_wallet_reserve_copper"], reserve)
                            reserved_plan = result["flight_route_search"]
                            self.assertTrue(reserved_plan["affordable_from_starting_wallet"])
                            self.assertEqual(reserved_plan["affordable_with_reserve"], expected)
                            self.assertEqual(reserved_plan["legs"], plan["legs"])
                        del route_onward["sections"]["flight_map"]
                        without_panel = SnapshotStore(Path(folder)).ingest(route_onward)
                        self.assertTrue((await session.call_tool("inspect_snapshot", {
                            **route_request, "flight_route_search": {
                                "destination_node_id": 15, "onward_snapshot_ids": [without_panel],
                            },
                        })).isError)
                        self.assertTrue((await session.call_tool("inspect_snapshot", {
                            **route_request, "section": "wallet",
                        })).isError)
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
                        groups["crafts"] = {"2": {}, "11": {},
                                            "100": {"name": "Renamed fixture", "learned": False}}
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
                        tracking_query = {"family": "forever", "tracked_recipe_id": 100,
                                          "character": tracked_after["character"]}
                        tracked_visits = await session.call_tool("list_snapshots", tracking_query)
                        self.assertFalse(tracked_visits.isError)
                        visits = json.loads(tracked_visits.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in visits],
                                         [after_tracking_key, forever_key])
                        self.assertEqual([row["tracked_recipe_match"] for row in visits], [
                            {"recipe_id": 100, "tracked_as_craft": True, "tracked_as_recraft": False,
                             "craft_learned": False, "recraft_learned": None},
                            {"recipe_id": 100, "tracked_as_craft": True, "tracked_as_recraft": True,
                             "craft_learned": None, "recraft_learned": True},
                        ])
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            "family": "retail", "tracked_recipe_id": 100,
                        })).isError)
                        for kind, learning, expected in [
                            ("craft", "unlearned", [after_tracking_key]),
                            ("recraft", "unlearned", []),
                            ("craft", "unknown", [forever_key]),
                            ("recraft", "learned", [forever_key]),
                            ("craft", "learned", []),
                        ]:
                            matched_tracking = await session.call_tool("list_snapshots", {
                                **tracking_query, "tracked_recipe_kind": kind,
                                "tracked_recipe_learning": learning,
                            })
                            self.assertFalse(matched_tracking.isError)
                            matches = json.loads(matched_tracking.content[0].text)["snapshots"]
                            self.assertEqual([row["snapshot_id"] for row in matches], expected)
                        for kind, learning, expected in [
                            ("craft", "unlearned", [(after_tracking_key, 1, 0)]),
                            ("recraft", "unlearned", []),
                            ("craft", "unknown", [(after_tracking_key, 2, 0), (forever_key, 2, 0)]),
                            ("recraft", "learned", [(forever_key, 0, 1)]),
                        ]:
                            matching_counts = await session.call_tool("list_snapshots", {
                                "family": "forever", "character": tracked_after["character"],
                                "tracked_recipe_kind": kind, "tracked_recipe_learning": learning,
                            })
                            self.assertFalse(matching_counts.isError)
                            matches = json.loads(matching_counts.content[0].text)["snapshots"]
                            self.assertEqual([
                                (row["snapshot_id"], row["tracked_recipe_counts"]["crafts"],
                                 row["tracked_recipe_counts"]["recrafts"]) for row in matches
                            ], expected)
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            "family": "retail", "tracked_recipe_learning": "unknown",
                        })).isError)
                        bank_summary = await session.call_tool("character_summary", {"snapshot_id":forever_key})
                        bank_summary_data = json.loads(bank_summary.content[0].text)
                        self.assertEqual(bank_summary_data["character_bank_item_slots_observed"], 1)
                        self.assertEqual(bank_summary_data["account_bank_item_slots_observed"], 0)
                        self.assertEqual(bank_summary_data["reputation_factions_observed"], 1)
                        self.assertEqual(bank_summary_data["flight_fare_summary"], {
                            "map_id": 1414, "origin_node_id": 12,
                            "known_fare_count": 1, "unknown_fare_count": 0,
                            "cheapest_destinations": [
                                {"node_id": 14, "name": "Fixture destination", "fare_copper": 0},
                            ],
                        })
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
                        profession_filter = {"family": "forever", "profession_skill_line": 164,
                                             "profession_min_rank": 20}
                        matching = await session.call_tool("list_snapshots", profession_filter)
                        self.assertFalse(matching.isError)
                        matches = json.loads(matching.content[0].text)["snapshots"]
                        self.assertIn(forever_key, [row["snapshot_id"] for row in matches])
                        self.assertNotIn(training_key, [row["snapshot_id"] for row in matches])
                        self.assertTrue(all(row["profession_match"] == {
                            "skill_line": 164, "rank": 20, "max_rank": 75,
                        } for row in matches))
                        higher_rank = await session.call_tool("list_snapshots", {
                            **profession_filter, "profession_min_rank": 21,
                        })
                        self.assertFalse(higher_rank.isError)
                        self.assertEqual(json.loads(higher_rank.content[0].text)["snapshots"], [])
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            "family": "forever", "profession_min_rank": 20,
                        })).isError)
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
                        more_factions["sections"]["quests"]["314"] = {
                            "title": "Fixture reward", "level": 10, "ready_for_turn_in": True,
                        }
                        more_key = SnapshotStore(Path(folder)).ingest(more_factions)
                        reward_summary = await session.call_tool("character_summary", {"snapshot_id": more_key})
                        self.assertFalse(reward_summary.isError)
                        paragon = json.loads(reward_summary.content[0].text)["paragon_reward_summary"]
                        reward = {"name": "Fixture faction", "account_wide": False, "reward_quest_id": 314,
                                  "reward_quest_observed": True, "reward_quest_readiness": "ready"}
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
                        travel = json.loads(json.dumps(reputation_after))
                        travel["sections"]["stats"] = {
                            "level": 60, "class": "Hunter", "race": "Human", "client_version": "1.60.1",
                            "map_id": 1414, "map_name": "First map", "map_parent_id": 100,
                            "hearthstone_bind_name": "First inn",
                            "hearthstone_item_id": 6948, "hearthstone_cooldown_seconds": 30,
                        }
                        travel["sections"]["flight_map"]["entries"]["2"]["fare_copper"] = 1
                        travel["sections"]["repair_quote"] = {"copper": 1}
                        travel["sections"]["bags"] = {"0": {"1": {"item_id": 117, "count": 3}}}
                        travel["sections"]["pet_care"] = {
                            "present": True, "happiness": 2, "loyalty": "Fixture loyal",
                            "carried_food": {"117": 3},
                        }
                        travel["sections"]["bag_capacity"] = {
                            "0": {"total_slots": 16, "free_slots": 1, "family_mask": 0},
                            "1": {"total_slots": 8, "free_slots": 5, "family_mask": 32},
                            "2": {"total_slots": 8, "free_slots": 4},
                            "3": {"total_slots": 0, "free_slots": 0},
                            "4": {"total_slots": 0, "free_slots": 0},
                        }
                        travel["sections"]["wallet"]["copper"] = 1
                        travel_before = SnapshotStore(Path(folder)).ingest(travel)
                        travel["captured_at"] = "2025-01-03T00:00:00Z"
                        travel["sections"]["bags"]["0"]["1"]["count"] = 1
                        travel["sections"]["pet_care"]["carried_food"] = {"117": 1}
                        del travel["sections"]["bag_capacity"]
                        travel["sections"]["wallet"]["copper"] = 0
                        travel["sections"]["stats"].update({
                            "map_id": 1415, "map_name": "Second map", "hearthstone_bind_name": "Second inn",
                            "hearthstone_cooldown_seconds": 0,
                        })
                        travel_after = SnapshotStore(Path(folder)).ingest(travel)
                        for filter_args in [{"minimum_quantity": 3},
                                            {"minimum_quantity": 3, "item_id": 117}]:
                            pet_food = await session.call_tool("list_snapshots", {
                                "family": "forever", "pet_food": filter_args,
                            })
                            self.assertFalse(pet_food.isError)
                            food_rows = json.loads(pet_food.content[0].text)["snapshots"]
                            self.assertEqual([row["snapshot_id"] for row in food_rows], [travel_before])
                            self.assertEqual(food_rows[0]["pet_food_match"], {
                                "item_id": filter_args.get("item_id"), "accepted_quantity": 3,
                            })
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            "family": "retail", "pet_food": {"minimum_quantity": 0},
                        })).isError)
                        bag_query = {"family": "forever", "minimum_free_bag_slots": 10,
                                     "minimum_general_free_bag_slots": 1}
                        bag_visits = await session.call_tool("list_snapshots", bag_query)
                        self.assertFalse(bag_visits.isError)
                        bag_rows = json.loads(bag_visits.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in bag_rows], [travel_before])
                        self.assertEqual(bag_rows[0]["carried_bag_space"], {
                            "total_slots": 32, "free_slots": 10, "known_general_free_slots": 1,
                            "free_slots_by_family_mask": {"0": 1, "32": 5},
                            "unknown_family_free_slots": 4,
                        })
                        insufficient = await session.call_tool("list_snapshots", {
                            **bag_query, "minimum_general_free_bag_slots": 2,
                        })
                        self.assertFalse(insufficient.isError)
                        self.assertEqual(json.loads(insufficient.content[0].text)["snapshots"], [])
                        for family_mask, minimum, expected in [
                            (32, 5, [travel_before]), (32, 6, []), (96, 0, []), (0, 1, [travel_before]),
                        ]:
                            by_family = await session.call_tool("list_snapshots", {
                                **bag_query, "bag_family_space": {
                                    "family_mask": family_mask, "minimum_free_slots": minimum,
                                },
                            })
                            self.assertFalse(by_family.isError)
                            family_rows = json.loads(by_family.content[0].text)["snapshots"]
                            self.assertEqual([row["snapshot_id"] for row in family_rows], expected)
                        repair_query = {"family": "forever", "repair_max_quote_copper": 1,
                                        "repair_affordable_only": True}
                        repairs = await session.call_tool("list_snapshots", repair_query)
                        self.assertFalse(repairs.isError)
                        repair_rows = json.loads(repairs.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in repair_rows], [travel_before])
                        self.assertEqual(repair_rows[0]["repair_quote_match"], {"copper": 1})
                        self.assertNotIn("wallet", repair_rows[0])
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            **repair_query, "family": "retail",
                        })).isError)
                        map_query = {"family": "forever", "player_parent_map_id": 100}
                        maps = await session.call_tool("list_snapshots", {**map_query, "limit": 1})
                        self.assertFalse(maps.isError)
                        map_results = json.loads(maps.content[0].text)
                        self.assertTrue(map_results["has_more"])
                        self.assertEqual([row["snapshot_id"] for row in map_results["snapshots"]],
                                         [travel_after])
                        exact = await session.call_tool("list_snapshots", {
                            **map_query, "player_map_id": 1414,
                        })
                        self.assertFalse(exact.isError)
                        exact_maps = json.loads(exact.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in exact_maps], [travel_before])
                        self.assertEqual(exact_maps[0]["player_map_match"], {
                            "map_id": 1414, "map_name": "First map", "map_parent_id": 100,
                        })
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            "family": "retail", "player_map_id": 1414,
                        })).isError)
                        changed_travel = await session.call_tool("compare_snapshots", {
                            "before_id": travel_before, "after_id": travel_after,
                        })
                        self.assertFalse(changed_travel.isError)
                        travel_changes = json.loads(changed_travel.content[0].text)
                        self.assertEqual(travel_changes["pet_care_change"], {
                            "before": {"present": True, "happiness": 2, "loyalty": "Fixture loyal"},
                            "after": {"present": True, "happiness": 2, "loyalty": "Fixture loyal"},
                            "accepted_carried_food_changes": {
                                "117": {"before_quantity": 3, "after_quantity": 1, "quantity_delta": -2},
                            },
                        })
                        self.assertEqual(travel_changes["flight_map_changes"]["changes"], [])
                        self.assertEqual(travel_changes["flight_map_changes"]["affordability_changes"], [
                            {"node_id": 14, "before_affordable": True, "after_affordable": False},
                        ])
                        context = travel_changes["travel_context_change"]
                        self.assertTrue(context["map_changed"])
                        self.assertTrue(context["hearthstone_bind_name_changed"])
                        self.assertFalse(context["hearthstone_item_changed"])
                        self.assertEqual(context["hearthstone_cooldown_seconds_delta"], -30)
                        self.assertNotIn("class", context["after"])
                        missing_space = travel_changes["carried_bag_space_change"]
                        self.assertIsNone(missing_space["family_space_changes"])
                        self.assertIsNone(missing_space["unknown_family_free_slots_delta"])
                        travel["sections"]["bag_capacity"] = {
                            "0": {"total_slots": 16, "free_slots": 1, "family_mask": 0},
                            "1": {"total_slots": 8, "free_slots": 2, "family_mask": 32},
                            "2": {"total_slots": 8, "free_slots": 4, "family_mask": 96},
                            "3": {"total_slots": 0, "free_slots": 0},
                            "4": {"total_slots": 0, "free_slots": 0},
                        }
                        changed_space_id = SnapshotStore(Path(folder)).ingest(travel)
                        changed_space = await session.call_tool("compare_snapshots", {
                            "before_id": travel_before, "after_id": changed_space_id,
                        })
                        self.assertFalse(changed_space.isError)
                        space = json.loads(changed_space.content[0].text)["carried_bag_space_change"]
                        self.assertEqual(space["unknown_family_free_slots_delta"], -4)
                        self.assertEqual(space["family_space_changes"], {
                            "32": {"before_free_slots": 5, "after_free_slots": 2, "free_slots_delta": -3},
                            "96": {"before_free_slots": None, "after_free_slots": 4, "free_slots_delta": None},
                        })
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
                        reputation_after["sections"]["trainer_services"]["entries"]["1"]["price_copper"] = 0
                        del reputation_after["sections"]["trainer_services"]["entries"]["3"]["price_copper"]
                        blocked_lesson = reputation_after["sections"]["trainer_services"]["entries"]["4"]
                        reputation_after["sections"]["trainer_services"]["entries"]["2"]["name"] = "New Lesson"
                        blocked_lesson["skill_requirement"]["met"] = True
                        abilities = blocked_lesson["ability_requirements"]
                        abilities["1"], abilities["2"] = abilities["2"], abilities["1"]
                        reputation_after["sections"]["quests"]["42"]["objectives"]["2"]["finished"] = True
                        reputation_after["sections"]["tracked_achievements"]["entries"]["101"]["completed"] = True
                        criteria = reputation_after["sections"]["tracked_achievements"]["entries"]["101"]["criteria"]
                        criteria["1"], criteria["2"] = criteria["2"], criteria["1"]
                        criteria["2"]["quantity"] = 3
                        profession = reputation_after["sections"]["professions"].pop("1")
                        profession["rank"] = 25
                        profession["max_rank"] = 150
                        reputation_after["sections"]["professions"]["2"] = profession
                        reputation_after["sections"]["professions"]["3"] = {
                            "name": "Another profession", "rank": 1, "max_rank": 75, "skill_line": 165,
                        }
                        progress_key = SnapshotStore(Path(folder)).ingest(reputation_after)
                        progress = await session.call_tool("compare_snapshots", {
                            "before_id": after_key, "after_id": progress_key, "quest_id": 42,
                            "achievement_id": 101, "profession_skill_line": 164})
                        self.assertFalse(progress.isError)
                        progress_data = json.loads(progress.content[0].text)
                        trainer_changes = progress_data["trainer_service_changes"]
                        self.assertEqual(trainer_changes["price_unit"], "copper")
                        self.assertEqual(trainer_changes["status_changes"], [])
                        self.assertEqual(trainer_changes["lessons_observed_only_before"], [
                            {"name": "Other Lesson", "rank_label": None, "ambiguous": False},
                        ])
                        self.assertEqual(trainer_changes["lessons_observed_only_after"], [
                            {"name": "New Lesson", "rank_label": None, "ambiguous": False},
                        ])
                        requirement = trainer_changes["requirement_changes"]
                        self.assertEqual([row["name"] for row in requirement], ["Blocked Lesson"])
                        self.assertFalse(requirement[0]["before"]["skill_requirement"]["met"])
                        self.assertTrue(requirement[0]["after"]["skill_requirement"]["met"])
                        self.assertEqual(requirement[0]["before"]["ability_requirements"],
                                         requirement[0]["after"]["ability_requirements"])
                        self.assertEqual(trainer_changes["price_changes"], [
                            {"name": "Expensive Lesson", "rank_label": None,
                             "before_price": 5_000_000_001, "after_price": None, "price_delta": None},
                            {"name": "Fixture Potion", "rank_label": None,
                             "before_price": 10, "after_price": 0, "price_delta": -10},
                        ])
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
                        self.assertEqual(progress_data["profession_progress"], {
                            "changes": [{"skill_line": 164, "before_rank": 20, "after_rank": 25,
                                         "rank_delta": 5, "before_max_rank": 75, "after_max_rank": 150}],
                            "observed_only_before": [], "observed_only_after": [165],
                            "ambiguous_skill_lines": [],
                        })
                        reputation_after["sections"]["professions"]["1"] = dict(profession, rank=30)
                        ambiguous_key = SnapshotStore(Path(folder)).ingest(reputation_after)
                        ambiguous = await session.call_tool("compare_snapshots", {
                            "before_id": progress_key, "after_id": ambiguous_key, "profession_skill_line": 164})
                        self.assertFalse(ambiguous.isError)
                        ambiguous_data = json.loads(ambiguous.content[0].text)
                        ambiguous_profession = ambiguous_data["profession_change"]
                        self.assertEqual(ambiguous_profession["after_status"], "ambiguous")
                        self.assertIsNone(ambiguous_profession["rank_delta"])
                        self.assertEqual(ambiguous_data["profession_progress"], {
                            "changes": [], "observed_only_before": [], "observed_only_after": [],
                            "ambiguous_skill_lines": [164],
                        })
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
                        self.assertEqual(recipe_data["material_capacity_by_item"], {
                            "744": {"bags": None, "all_observed_storage": 0,
                                    "craft_shortfall_vs_bags": None, "craft_shortfall_vs_observed_storage": 2,
                                    "reagent_units_left_in_bags": None, "reagent_units_left_in_observed_storage": None},
                        })
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
                        self.assertEqual(json.loads(stocked.content[0].text)["material_capacity_by_item"], {
                            "744": {"bags": 2, "all_observed_storage": 3,
                                    "craft_shortfall_vs_bags": 0, "craft_shortfall_vs_observed_storage": 0,
                                    "reagent_units_left_in_bags": 5, "reagent_units_left_in_observed_storage": 6},
                            "10": {"bags": 3, "all_observed_storage": 3,
                                   "craft_shortfall_vs_bags": 0, "craft_shortfall_vs_observed_storage": 0,
                                   "reagent_units_left_in_bags": 4, "reagent_units_left_in_observed_storage": 4},
                        })
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
                        merchant["sections"]["merchant_stock"]["3"] = {
                            "currency_id": 744, "bundle_size": 1, "price_copper": 2,
                            "extended_cost": False, "available": -1,
                        }
                        merchant_key = SnapshotStore(Path(folder)).ingest(merchant)
                        food_merchant = json.loads(json.dumps(merchant))
                        food_merchant["character"]["realm"] = "First"
                        food_merchant["sections"]["merchant_stock"] = {
                            "1": {"item_id": 117, "bundle_size": 5, "price_copper": 8,
                                  "extended_cost": False, "available": -1},
                            "2": {"currency_id": 117, "bundle_size": 1, "price_copper": 0,
                                  "extended_cost": False, "available": -1},
                        }
                        food_merchant_id = SnapshotStore(Path(folder)).ingest(food_merchant)
                        food_offer_query = {"snapshot_id": food_merchant_id, "section": "merchant_stock",
                                            "merchant_pet_food_snapshot_id": travel_before}
                        food_offers = await session.call_tool("inspect_snapshot", food_offer_query)
                        self.assertFalse(food_offers.isError)
                        food_data = json.loads(food_offers.content[0].text)
                        self.assertEqual(list(food_data["data"]), ["1"])
                        self.assertEqual(food_data["merchant_match_count"], 1)
                        food_listing = {"family": "forever", "merchant_pet_food_snapshot_id": travel_before,
                                        "merchant_affordable_only": True}
                        food_visits = await session.call_tool("list_snapshots", food_listing)
                        self.assertFalse(food_visits.isError)
                        food_rows = json.loads(food_visits.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in food_rows], [food_merchant_id])
                        self.assertEqual(food_rows[0]["merchant_match"]["offer_count"], 1)
                        self.assertEqual(food_rows[0]["merchant_match"]["pet_food_snapshot_id"], travel_before)
                        for invalid_food in [{"merchant_item_id": 117},
                                             {"merchant_pet_food_snapshot_id": forever_key}]:
                            self.assertTrue((await session.call_tool("list_snapshots", {
                                **food_listing, **invalid_food,
                            })).isError)
                        capped_food = await session.call_tool("list_snapshots", {
                            **food_listing, "merchant_max_price_copper": 7,
                        })
                        self.assertFalse(capped_food.isError)
                        self.assertEqual(json.loads(capped_food.content[0].text)["snapshots"], [])
                        unknown_food = await session.call_tool("inspect_snapshot", {
                            **food_offer_query, "merchant_pet_food_snapshot_id": forever_key,
                        })
                        self.assertFalse(unknown_food.isError)
                        self.assertIsNone(json.loads(unknown_food.content[0].text)["merchant_match_count"])
                        self.assertTrue((await session.call_tool("inspect_snapshot", {
                            **food_offer_query, "merchant_item_id": 117,
                        })).isError)
                        currency_visits = await session.call_tool("list_snapshots", {
                            "family": "forever", "merchant_currency_id": 744, "merchant_max_price_copper": 2,
                        })
                        self.assertFalse(currency_visits.isError)
                        visits = json.loads(currency_visits.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in visits], [merchant_key])
                        self.assertEqual(visits[0]["merchant_match"], {
                            "currency_id": 744, "offer_count": 1,
                            "captured_player_map_id": None, "captured_player_map_name": None,
                        })
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            "family": "forever", "merchant_item_id": 744, "merchant_currency_id": 744,
                        })).isError)
                        above_ceiling = await session.call_tool("list_snapshots", {
                            "family": "forever", "merchant_currency_id": 744, "merchant_max_price_copper": 1,
                        })
                        self.assertFalse(above_ceiling.isError)
                        self.assertEqual(json.loads(above_ceiling.content[0].text)["snapshots"], [])
                        in_stock = await session.call_tool("list_snapshots", {
                            "family": "forever", "merchant_item_id": 744,
                            "merchant_in_stock_only": True, "merchant_copper_only": True,
                        })
                        self.assertFalse(in_stock.isError)
                        in_stock_rows = json.loads(in_stock.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in in_stock_rows], [merchant_key])
                        self.assertEqual(in_stock_rows[0]["merchant_match"]["offer_count"], 1)
                        affordability = {"family": "forever", "merchant_item_id": 744,
                                         "merchant_affordable_only": True}
                        affordable = await session.call_tool("list_snapshots", affordability)
                        self.assertFalse(affordable.isError)
                        affordable_rows = json.loads(affordable.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in affordable_rows], [merchant_key])
                        self.assertEqual(affordable_rows[0]["merchant_match"]["offer_count"], 1)
                        ceiling = await session.call_tool("list_snapshots", {
                            **affordability, "merchant_max_price_copper": 1,
                        })
                        self.assertFalse(ceiling.isError)
                        self.assertEqual(json.loads(ceiling.content[0].text)["snapshots"], [])
                        self.assertTrue((await session.call_tool("list_snapshots", {
                            "family": "forever", "merchant_affordable_only": True,
                        })).isError)
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
                        self.assertEqual(batch_data["ordered_nonmaterial_requirements"], {
                            "first_observed_shortfall": {
                                "recipe_number": 2, "recipe_id": 101,
                                "failed_checks": ["observed_conditions_met", "observed_charges_cover_request"],
                            },
                            "recipes_with_unknown_checks": 1,
                        })
                        priced_batch = await session.call_tool("review_forever_recipe", {
                            **batch_request, "merchant_reagent_plan": True,
                            "merchant_snapshot_id": merchant_key,
                        })
                        self.assertFalse(priced_batch.isError)
                        checkpoint_budget = json.loads(priced_batch.content[0].text)[
                            "merchant_reagent_plan"]["first_uncovered_recipe_budget"]
                        self.assertEqual((checkpoint_budget["recipe_number"], checkpoint_budget["recipe_id"]), (2, 101))
                        self.assertEqual(checkpoint_budget["unlimited_copper_only_total"], 12)
                        self.assertEqual(checkpoint_budget["reagents"][0]["shortfall_vs_observed"], 1)
                        self.assertEqual(batch_data["ordered_starting_material_coverage"], {
                            "bags": False, "all_observed_storage": False,
                        })
                        self.assertEqual(batch_data["ordered_material_checkpoint"], {
                            "covered_prefix_recipes": 1,
                            "first_uncovered_recipe": {
                                "recipe_number": 2, "recipe_id": 101,
                                "reason": "shortfall_vs_observed_stock",
                                "blocking_reagents_by_item_id": {
                                    "744": {"starting_units_needed_through_recipe": 13,
                                            "shortfall_vs_observed": 1, "shortfall_vs_bags": 1,
                                            "suggested_bank_withdrawals": {
                                                "character_bank": None, "account_bank": 0,
                                            }},
                                },
                            },
                        })
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
                        self.assertEqual(target_data["captured_preparation"], {
                            "recipe_learned": True, "observed_conditions_met": False,
                            "observed_charges_cover_request": False, "bags_cover_fixed_reagents": False,
                            "observed_storage_covers_fixed_reagents": False,
                        })
                        self.assertEqual(batch_data["captured_preparation"], {
                            "recipe_learned": True, "observed_conditions_met": None,
                            "observed_charges_cover_request": None, "bags_cover_fixed_reagents": True,
                            "observed_storage_covers_fixed_reagents": True,
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
                                          "estimated_units_after_batch_in_observed_storage": None,
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
                        self.assertEqual(item_plan["estimated_units_after_batch_in_observed_storage"], 10)
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
