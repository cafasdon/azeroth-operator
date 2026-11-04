import os
import asyncio
import json
import sys
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
            params = StdioServerParameters(command=rust_binary or sys.executable,
                args=[] if rust_binary else [str(Path(__file__).resolve().parents[1] / "server.py")],
                env={**os.environ, "AZEROTH_DATA_DIR": folder})
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    tools = await session.list_tools()
                    expected = {"capabilities", "inspect_snapshot"}
                    if rust_binary:
                        expected.update({"list_snapshots", "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout", "prepare_macro_operation", "prepare_binding_operation", "prepare_equipment_operation", "prepare_bag_operation", "prepare_talent_operation", "prepare_action_operation", "prepare_equipment_set_operation"})
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
                        self.assertEqual(set(contract["implemented"]), expected)
                        self.assertEqual(contract["live_client_qualified"], [])
                        before_files={p.name:p.read_bytes() for p in Path(folder).glob("*.json")}
                        prepared_set=await session.call_tool("prepare_equipment_set_operation",{"snapshot_id":set_key,
                            "operation":{"operation":"modify","set_id":0,"name":"Renamed set","icon":123}})
                        self.assertFalse(prepared_set.isError)
                        set_data=json.loads(prepared_set.content[0].text)
                        self.assertTrue(set_data["import_code"].startswith("AO-SET-1:"))
                        self.assertEqual(set_data["expected"]["slots"]["16"]["observed_item"]["guid"],"Item-New")
                        self.assertTrue(set_data["addon_execution_enabled"])
                        self.assertEqual(set_data["requires_addon_version"],"0.10.0-dev")
                        self.assertFalse(set_data["game_execution"])
                        deletion=await session.call_tool("prepare_equipment_set_operation",{"snapshot_id":set_key,
                            "operation":{"operation":"delete","set_id":0}})
                        self.assertFalse(deletion.isError)
                        self.assertTrue(json.loads(deletion.content[0].text)["deletes_saved_set"])
                        use=await session.call_tool("prepare_equipment_set_operation",{"snapshot_id":set_key,
                            "operation":{"operation":"use","set_id":0,"expect_empty_slots":[i for i in range(1,20) if i!=16]}})
                        self.assertFalse(use.isError)
                        use_data=json.loads(use.content[0].text)
                        self.assertTrue(use_data["addon_execution_enabled"])
                        self.assertEqual(use_data["requires_addon_version"],"0.11.0-dev")
                        self.assertTrue(use_data["import_code"].startswith("AO-SET-USE-1:"))
                        self.assertEqual(use_data["equipment_before"]["16"],"Item-Old")
                        self.assertEqual(use_data["desired_equipment"]["16"],"Item-New")
                        for action in ["create","save"]:
                            operation={"operation":action,"ignored_slots":[i for i in range(1,20) if i!=16],
                                "expect_empty_slots":[i for i in range(1,20) if i!=16]}
                            if action=="create": operation.update({"name":"New fixture","icon":123})
                            else: operation["set_id"]=0
                            result=await session.call_tool("prepare_equipment_set_operation",{"snapshot_id":set_key,"operation":operation})
                            self.assertFalse(result.isError)
                            draft=json.loads(result.content[0].text)
                            self.assertTrue(draft["addon_execution_enabled"])
                            self.assertEqual(draft["requires_addon_version"],"0.13.0-dev")
                            self.assertTrue(draft["import_code"].startswith("AO-SETSAVE-1:"))
                            self.assertEqual(draft["equipment_before"]["16"],{"guid":"Item-Old","item_id":123})
                            self.assertEqual(draft["kind"],action+"_equipment_set")
                            self.assertEqual(draft["overwrites_saved_contents"],action=="save")
                        for action in ["assign_spec","unassign_spec"]:
                            operation={"operation":action,"set_id":0}
                            if action=="assign_spec": operation["spec_index"]=1
                            result=await session.call_tool("prepare_equipment_set_operation",{"snapshot_id":spec_key,"operation":operation})
                            self.assertFalse(result.isError)
                            draft=json.loads(result.content[0].text)
                            self.assertTrue(draft["addon_execution_enabled"])
                            self.assertEqual(draft["requires_addon_version"],"0.12.0-dev")
                            self.assertTrue(draft["import_code"].startswith("AO-SETSPEC-1:"))
                            self.assertEqual(draft["expected_catalogue"]["0"]["assigned_spec"],2)
                            if action=="assign_spec":
                                self.assertEqual(draft["spec_id"],261)
                                self.assertEqual(draft["displaced_set"]["set_id"],1)
                                self.assertNotIn("assigned_spec",draft["desired_catalogue"]["1"])
                            else: self.assertNotIn("assigned_spec",draft["desired_catalogue"]["0"])
                        for sid,operation in [(key,{"operation":"delete","set_id":0}),
                            (set_key,{"operation":"create","name":"New","icon":123}),
                            (set_key,{"operation":"save"}),
                            (set_key,{"operation":"assign_spec","set_id":0,"spec_index":1}),
                            (spec_key,{"operation":"assign_spec","set_id":0,"spec_index":99}),
                            (spec_key,{"operation":"unassign_spec","set_id":0,"spec_index":1}),
                            (set_key,{"operation":"use","set_id":0}),
                            (set_key,{"operation":"delete","set_id":99}),
                            (set_key,{"operation":"delete","set_id":0,"name":"unexpected"})]:
                            refused=await session.call_tool("prepare_equipment_set_operation",{"snapshot_id":sid,"operation":operation})
                            self.assertTrue(refused.isError)
                        self.assertEqual(before_files,{p.name:p.read_bytes() for p in Path(folder).glob("*.json")})
                        equip=await session.call_tool("prepare_equipment_operation",{"snapshot_id":key,"operation":{"bag":0,"slot":1,"target_slot":16}})
                        self.assertFalse(equip.isError)
                        self.assertTrue(json.loads(equip.content[0].text)["import_code"].startswith("AO-EQUIP-1:"))
                        bag=await session.call_tool("prepare_bag_operation",{"snapshot_id":key,"operation":{"bag":0,"slot":1,"target_bag":0,"target_slot":3,"expect_empty_target":True}})
                        self.assertFalse(bag.isError)
                        self.assertTrue(json.loads(bag.content[0].text)["import_code"].startswith("AO-BAG-1:"))
                        swap=await session.call_tool("prepare_bag_operation",{"snapshot_id":key,"operation":{"operation":"swap","bag":0,"slot":1,"target_bag":0,"target_slot":4}})
                        self.assertFalse(swap.isError)
                        swap_data=json.loads(swap.content[0].text)
                        self.assertEqual(swap_data["kind"],"swap_bag_items")
                        self.assertFalse(swap_data["addon_execution_enabled"])
                        self.assertEqual(swap_data["target"]["guid"],"Item-Swap")
                        split=await session.call_tool("prepare_bag_operation",{"snapshot_id":key,"operation":{"bag":0,"slot":2,"target_bag":0,"target_slot":3,"expect_empty_target":True,"amount":2}})
                        self.assertFalse(split.isError)
                        self.assertEqual(json.loads(split.content[0].text)["kind"],"split_bag_item")
                        action=await session.call_tool("prepare_action_operation",{"snapshot_id":key,"operation":{"source_slot":1,"target_slot":2}})
                        self.assertFalse(action.isError)
                        action_data=json.loads(action.content[0].text)
                        self.assertTrue(action_data["import_code"].startswith("AO-ACTION-1:"))
                        self.assertEqual(action_data["source"]["macro"]["body"],"/say test")
                        self.assertFalse(action_data["game_execution"])
                        copied=await session.call_tool("prepare_action_operation",{"snapshot_id":key,"operation":{"operation":"copy","source_slot":1,"target_slot":2}})
                        self.assertFalse(copied.isError)
                        copy_data=json.loads(copied.content[0].text)
                        self.assertEqual(copy_data["kind"],"copy_action_slot")
                        self.assertTrue(copy_data["import_code"].startswith("AO-ACTIONCOPY-1:"))
                        self.assertTrue(copy_data["addon_execution_enabled"])
                        self.assertEqual(copy_data["requires_addon_version"],"0.18.0-dev")
                        self.assertNotEqual(copy_data["operation_id"],action_data["operation_id"])
                        unknown=await session.call_tool("prepare_action_operation",{"snapshot_id":key,"operation":{"source_slot":1,"target_slot":3}})
                        self.assertTrue(unknown.isError)
                        talent=await session.call_tool("prepare_talent_operation",{"snapshot_id":key,"operation":{"target_config_id":9}})
                        self.assertFalse(talent.isError)
                        self.assertTrue(json.loads(talent.content[0].text)["import_code"].startswith("AO-TALENT-1:"))
                        for kind in ("rename", "delete"):
                            request={"operation":kind,"target_config_id":9}
                            if kind == "rename":
                                request["name"]="Prepared name"
                            draft=await session.call_tool("prepare_talent_operation",{"snapshot_id":key,"operation":request})
                            self.assertFalse(draft.isError)
                            data=json.loads(draft.content[0].text)
                            self.assertEqual(data["kind"],kind+"_talent_config")
                            self.assertTrue(data["execution_supported"])
                            self.assertEqual(data["requires_addon_version"],"0.14.0-dev")
                            self.assertTrue(data["import_code"].startswith("AO-TALMETA-1:"))
                        created=await session.call_tool("prepare_talent_operation",{"snapshot_id":key,"operation":{"operation":"create","name":"New build","expected_new_shared_action_bars":False}})
                        self.assertFalse(created.isError)
                        creation=json.loads(created.content[0].text)
                        self.assertTrue(creation["import_code"].startswith("AO-TALCREATE-1:"))
                        self.assertTrue(creation["execution_supported"])
                        self.assertEqual(creation["requires_addon_version"],"0.15.0-dev")
                        self.assertTrue(creation["selects_new_build"])
                        binding = await session.call_tool("prepare_binding_operation", {"snapshot_id":key,
                            "operation":{"key":"CTRL-1","context":0,"desired":"ACTIONBUTTON2"}})
                        self.assertFalse(binding.isError)
                        prepared_binding=json.loads(binding.content[0].text)
                        self.assertEqual(prepared_binding["expected"],"ACTIONBUTTON1")
                        self.assertTrue(prepared_binding["import_code"].startswith("AO-BIND-1:"))
                        rejected_binding = await session.call_tool("prepare_binding_operation", {"snapshot_id":key,
                            "operation":{"key":"CTRL-1","context":1,"desired":"ACTIONBUTTON2"}})
                        self.assertTrue(rejected_binding.isError)
                        new_binding = await session.call_tool("prepare_binding_operation", {"snapshot_id":key,
                            "operation":{"key":"CTRL-2","context":0,"desired":"ACTIONBUTTON2","expect_unbound":True}})
                        self.assertFalse(new_binding.isError)
                        self.assertEqual(json.loads(new_binding.content[0].text)["precondition_basis"],
                            "explicit_unbound_precondition_not_observed")
                        with self.assertRaisesRegex(McpError, "tool not found"):
                            await session.call_tool("equip_item", {"slot":16,"item_id":123})
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
                        for operation in [
                            {"kind":"create","target_scope":"character","desired":{"name":"New","icon":134400,"body":""}},
                            {"kind":"edit","source_index":1,"desired":{"name":"Edited","icon":134400,"body":"/say changed"}},
                            {"kind":"delete","source_index":1},
                            {"kind":"move","source_index":1,"target_scope":"character"},
                        ]:
                            prepared = await session.call_tool("prepare_macro_operation", {
                                "snapshot_id":key, "operation":operation})
                            self.assertFalse(prepared.isError)
                            data = json.loads(prepared.content[0].text)
                            self.assertEqual(data["kind"], operation["kind"])
                            self.assertEqual(data["snapshot_id"], key)
                            self.assertTrue(data["requires_player_apply"])
                            self.assertTrue(data["import_code"].startswith("AO-MACRO-1:"))
                            self.assertFalse(data["game_execution"])
                        rejected = await session.call_tool("prepare_macro_operation", {
                            "snapshot_id":key,"operation":{"kind":"delete","source_index":999}})
                        self.assertTrue(rejected.isError)
                    result = await session.call_tool("inspect_snapshot", {"snapshot_id": key})
                    receipts = await session.call_tool("inspect_snapshot", {"snapshot_id":key,"section":"operation_receipts"})
                    self.assertFalse(receipts.isError)
                    self.assertEqual(json.loads(receipts.content[0].text)["data"]["a" * 64]["status"], "outcome_unknown")
                    self.assertFalse(result.isError)
                    payload = json.loads(result.content[0].text)
                    self.assertFalse(payload["live"])
                    rejected = await session.call_tool("inspect_snapshot", {"snapshot_id": "../secret"})
                    self.assertTrue(rejected.isError)
