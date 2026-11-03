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
            sample["sections"]["equipment"] = {"16": {"item_id": 123}}
            sample["sections"]["operation_receipts"] = {"a" * 64: {"status":"outcome_unknown", "kind":"move"}}
            sample["sections"]["macros"] = {"1": {"name":"AO Test", "icon":134400,
                "body":"/say test", "scope":"account"}}
            sample["sections"]["bags"] = {"0": {"1": {"item_id": 123, "count": 2},
                                                     "2": {"item_id": 123, "count": 3}}}
            sample["sections"]["talents"] = {"config_id":7,"nodes":{},"loadout_export":{
                "status":"available","format":"blizzard_traits","import_string":"QUJDRA==",
                "spec_id":263,"serialization_version":2,"staged_changes":False}}
            key = SnapshotStore(Path(folder)).ingest(sample)
            params = StdioServerParameters(command=rust_binary or sys.executable,
                args=[] if rust_binary else [str(Path(__file__).resolve().parents[1] / "server.py")],
                env={**os.environ, "AZEROTH_DATA_DIR": folder})
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    tools = await session.list_tools()
                    expected = {"capabilities", "inspect_snapshot"}
                    if rust_binary:
                        expected.update({"list_snapshots", "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout", "prepare_macro_operation"})
                    self.assertEqual({t.name for t in tools.tools}, expected)
                    if rust_binary:
                        for tool in tools.tools:
                            self.assertTrue(tool.annotations.readOnlyHint)
                            self.assertFalse(tool.annotations.destructiveHint)
                        capabilities = await session.call_tool("capabilities", {})
                        contract = json.loads(capabilities.content[0].text)
                        self.assertFalse(contract["game_execution"])
                        self.assertEqual(set(contract["implemented"]), expected)
                        self.assertEqual(contract["live_client_qualified"], [])
                        with self.assertRaisesRegex(McpError, "tool not found"):
                            await session.call_tool("equip_item", {"slot":16,"item_id":123})
                        listing = await session.call_tool("list_snapshots", {"family": "retail"})
                        self.assertEqual(json.loads(listing.content[0].text)["snapshots"][0]["snapshot_id"], key)
                        other = await session.call_tool("list_snapshots", {"family": "forever"})
                        self.assertEqual(json.loads(other.content[0].text)["snapshots"], [])
                        comparison = await session.call_tool("compare_snapshots", {"before_id": key, "after_id": key})
                        self.assertEqual(json.loads(comparison.content[0].text)["sections"]["equipment"]["status"], "unchanged")
                        summary = await session.call_tool("character_summary", {"snapshot_id": key})
                        self.assertFalse(json.loads(summary.content[0].text)["live"])
                        carried = json.loads(summary.content[0].text)["carried_item_review"]
                        self.assertEqual(carried["quantity_observed"], 5)
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
                            self.assertTrue(data["import_code"].startswith("AO-MACRO-1|"))
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
