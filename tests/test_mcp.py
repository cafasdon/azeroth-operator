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
                    response = await session.call_tool("lookup_forever_catalogue", {"section":"items","key":"744"})
                    self.assertTrue(response.isError)
                    self.assertNotIn("Unverified", response.content[0].text)

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
                        expected.update({"list_snapshots", "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout"})
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
