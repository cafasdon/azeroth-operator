"""Validate mount observations through both readers and the real Rust MCP."""

import copy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore, validate_snapshot
from test_snapshots import fixture


def mount_snapshot():
    value = fixture()
    value["client"] = {"family": "forever", "variant": "live", "build": "70862", "interface": 16001}
    value["sections"]["mount_collection"] = {"catalogue_count": 3, "entries": {
        "2": {"name": "Fixture mount", "spell_id": 40002, "usable": True, "favorite": False}}}
    return value


class MountCollectionTests(unittest.IsolatedAsyncioTestCase):
    async def test_readers_and_mcp(self):
        binary = Path(__file__).resolve().parents[1] / "target/debug" / (
            "azeroth-operator.exe" if os.name == "nt" else "azeroth-operator")
        binary = Path(os.environ.get("AZEROTH_TEST_BINARY", binary)).resolve(strict=True)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            store = SnapshotStore(root)
            captured = mount_snapshot()
            key = store.ingest(captured)
            self.assertEqual(store.inspect(key, "mount_collection")["data"], captured["sections"]["mount_collection"])
            empty = mount_snapshot()
            empty["sections"]["mount_collection"]["entries"] = {}
            empty_key = store.ingest(empty)
            missing = mount_snapshot()
            del missing["sections"]["mount_collection"]
            missing_key = store.ingest(missing)
            before = copy.deepcopy(captured)
            before["sections"]["mount_collection"]["entries"]["20"] = {
                "name": "Before only", "spell_id": 40020, "usable": True, "favorite": False}
            before_key = store.ingest(before)
            after = copy.deepcopy(before)
            after["captured_at"] = "2025-01-02T00:00:00Z"
            mounts = after["sections"]["mount_collection"]
            mounts["catalogue_count"] = 4
            mounts["entries"].pop("20")
            mounts["entries"]["2"].update(name="Translated name", usable=False, favorite=True)
            for mount_id in (3, 10):
                mounts["entries"][str(mount_id)] = {
                    "name": "After only", "spell_id": 40000 + mount_id, "usable": True, "favorite": False}
            after_key = store.ingest(after)
            retail_key = store.ingest(fixture())
            invalid = []
            for mutate in (
                lambda s: s["client"].update(family="retail"),
                lambda s: s["sections"]["mount_collection"].update(catalogue_count=0),
                lambda s: s["sections"]["mount_collection"].update(catalogue_count=True),
                lambda s: s["sections"]["mount_collection"].update(catalogue_count=4097),
                lambda s: s["sections"]["mount_collection"]["entries"]["2"].update(usable=1),
                lambda s: s["sections"]["mount_collection"]["entries"]["2"].update(spell_id=0),
                lambda s: s["sections"]["mount_collection"]["entries"]["2"].update(name="x"*257),
                lambda s: s["sections"]["mount_collection"]["entries"].update({"02":s["sections"]["mount_collection"]["entries"].pop("2")}),
                lambda s: s["sections"]["mount_collection"].update(extra="unexpected"),
            ):
                value = copy.deepcopy(captured)
                mutate(value)
                with self.assertRaises(ValueError):
                    validate_snapshot(value)
                raw = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
                bad_key = hashlib.sha256(raw).hexdigest()
                (root / f"{bad_key}.json").write_bytes(raw)
                invalid.append(bad_key)
            env = {**os.environ, "AZEROTH_DATA_DIR": directory}
            env.pop("AZEROTH_FOREVER_CATALOGUE", None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    captured_result = await session.call_tool("inspect_snapshot", {"snapshot_id": key, "section": "mount_collection"})
                    self.assertFalse(captured_result.isError)
                    self.assertEqual(json.loads(captured_result.content[0].text)["data"], captured["sections"]["mount_collection"])
                    for snapshot_id, count in ((key, 1), (empty_key, 0), (missing_key, None)):
                        response = await session.call_tool("character_summary", {"snapshot_id": snapshot_id})
                        self.assertFalse(response.isError)
                        self.assertEqual(json.loads(response.content[0].text)["collected_mounts_observed"], count)
                    for bad_key in invalid:
                        response = await session.call_tool("inspect_snapshot", {"snapshot_id": bad_key, "section": "mount_collection"})
                        self.assertTrue(response.isError)
                    found = await session.call_tool("inspect_snapshot", {
                        "snapshot_id": key, "section": "mount_collection", "mount_id": 2})
                    selected = json.loads(found.content[0].text)
                    self.assertTrue(selected["observed_collected"])
                    self.assertEqual(selected["data"]["spell_id"], 40002)
                    for source, mount_id, available in ((key, 99, True), (missing_key, 2, False)):
                        response = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": source, "section": "mount_collection", "mount_id": mount_id})
                        data = json.loads(response.content[0].text)
                        self.assertEqual(data["available"], available)
                        self.assertIsNone(data["observed_collected"])
                        self.assertIsNone(data["data"])
                    compared = await session.call_tool("compare_snapshots", {
                        "before_id": before_key, "after_id": after_key, "mount_collection_changes": True})
                    self.assertFalse(compared.isError)
                    changes = json.loads(compared.content[0].text)["mount_collection_changes"]
                    self.assertEqual(changes["observed_only_before"], [20])
                    self.assertEqual(changes["observed_only_after"], [3, 10])
                    self.assertEqual(changes["collected_count_delta"], 1)
                    self.assertEqual(changes["changes"], [{"mount_id": 2, "fields": {
                        "usable": {"before": True, "after": False},
                        "favorite": {"before": False, "after": True}}}])
                    missing_comparison = await session.call_tool("compare_snapshots", {
                        "before_id": before_key, "after_id": missing_key, "mount_collection_changes": True})
                    self.assertIsNone(json.loads(missing_comparison.content[0].text)["mount_collection_changes"])
                    ordinary = await session.call_tool("compare_snapshots", {"before_id": before_key, "after_id": after_key})
                    self.assertNotIn("mount_collection_changes", json.loads(ordinary.content[0].text))
                    wrong_family = await session.call_tool("compare_snapshots", {
                        "before_id": retail_key, "after_id": retail_key, "mount_collection_changes": True})
                    self.assertTrue(wrong_family.isError)
                    for args in ({"snapshot_id": key, "mount_id": 2},
                                 {"snapshot_id": key, "section": "equipment", "mount_id": 2},
                                 {"snapshot_id": retail_key, "section": "mount_collection", "mount_id": 2}):
                        self.assertTrue((await session.call_tool("inspect_snapshot", args)).isError)


if __name__ == "__main__":
    unittest.main()
