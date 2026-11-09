"""Toy views remain explicitly partial and validated by both snapshot readers."""

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


class ToyCollectionTests(unittest.IsolatedAsyncioTestCase):
    async def test_filtered_view_readback_and_validation(self):
        binary = Path(__file__).resolve().parents[1] / "target/debug" / (
            "azeroth-operator.exe" if os.name == "nt" else "azeroth-operator")
        binary = Path(os.environ.get("AZEROTH_TEST_BINARY", binary)).resolve(strict=True)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            store = SnapshotStore(root)
            captured = fixture()
            captured["client"] = {"family": "forever", "variant": "live", "build": "70862", "interface": 16001}
            captured["sections"]["toy_collection_view"] = {"coverage": "filtered_journal", "displayed_count": 2,
                "entries": {"1": {"name": "Owned", "collected": True, "favorite": False},
                            "2": {"name": "Not collected", "collected": False, "favorite": False}}}
            key = store.ingest(captured)
            empty = copy.deepcopy(captured)
            empty["sections"]["toy_collection_view"].update(displayed_count=0, entries={})
            empty_key = store.ingest(empty)
            missing = copy.deepcopy(captured)
            missing["sections"].pop("toy_collection_view")
            missing_key = store.ingest(missing)
            after = copy.deepcopy(captured)
            after["captured_at"] = "2025-01-02T00:00:00Z"
            view = after["sections"]["toy_collection_view"]
            view["entries"].pop("2")
            view["entries"]["1"].update(name="Translated", collected=False, favorite=True)
            for item_id in (3, 10):
                view["entries"][str(item_id)] = {"name": "New view row", "collected": True, "favorite": False}
            view["displayed_count"] = 3
            after_key = store.ingest(after)
            retail_key = store.ingest(fixture())
            invalid = []
            for change in (
                lambda x: x["client"].update(family="retail"),
                lambda x: x["sections"]["toy_collection_view"].update(coverage="complete"),
                lambda x: x["sections"]["toy_collection_view"].update(displayed_count=1),
                lambda x: x["sections"]["toy_collection_view"].update(displayed_count=True),
                lambda x: x["sections"]["toy_collection_view"]["entries"]["1"].update(collected=1),
                lambda x: x["sections"]["toy_collection_view"]["entries"]["1"].update(favorite=None),
                lambda x: x["sections"]["toy_collection_view"]["entries"]["1"].update(name="x"*257),
                lambda x: x["sections"]["toy_collection_view"]["entries"].update({"01":x["sections"]["toy_collection_view"]["entries"].pop("1")}),
            ):
                bad = copy.deepcopy(captured)
                change(bad)
                with self.assertRaises(ValueError):
                    validate_snapshot(bad)
                raw = json.dumps(bad, sort_keys=True, separators=(",", ":")).encode()
                bad_key = hashlib.sha256(raw).hexdigest()
                (root / f"{bad_key}.json").write_bytes(raw)
                invalid.append(bad_key)
            env = {**os.environ, "AZEROTH_DATA_DIR": directory}
            env.pop("AZEROTH_FOREVER_CATALOGUE", None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    for selector, expected in [
                        ({"item_id":1,"collected":True,"favorite":False},[key]),
                        ({"item_id":1,"collected":False,"favorite":True},[after_key]),
                        ({"item_id":1,"collected":True,"favorite":True},[]),
                        ({"item_id":2,"collected":False},[key]),
                        ({"item_id":99,"collected":False},[]),
                    ]:
                        found = await session.call_tool("list_snapshots", {
                            "family":"forever","toy_in_view":selector,
                        })
                        self.assertFalse(found.isError,found)
                        rows = json.loads(found.content[0].text)["snapshots"]
                        self.assertEqual([row["snapshot_id"] for row in rows],expected)
                        for row in rows:
                            self.assertEqual(row["toy_in_view_match"],{
                                "coverage":"filtered_journal",**selector,
                                "favorite":selector.get("favorite",False)})
                    for family, selector in [("retail",{"item_id":1}),("forever",{"item_id":0})]:
                        rejected=await session.call_tool("list_snapshots",{"family":family,"toy_in_view":selector})
                        self.assertTrue(rejected.isError)
                    result = await session.call_tool("inspect_snapshot", {"snapshot_id": key, "section": "toy_collection_view"})
                    self.assertFalse(result.isError)
                    self.assertEqual(json.loads(result.content[0].text)["data"], captured["sections"]["toy_collection_view"])
                    for source, flags, expected in [
                        (key, {"toy_collected":True}, ["1"]),
                        (after_key, {"toy_collected":False,"toy_favorite":True}, ["1"]),
                        (after_key, {"toy_collected":True,"toy_favorite":True}, []),
                        (missing_key, {"toy_favorite":False}, None),
                    ]:
                        reply=await session.call_tool("inspect_snapshot",{"snapshot_id":source,
                            "section":"toy_collection_view",**flags})
                        self.assertFalse(reply.isError,reply)
                        filtered=json.loads(reply.content[0].text)
                        self.assertEqual(filtered["toy_matched_count"],None if expected is None else len(expected))
                        if expected is not None:
                            self.assertEqual(list(filtered["data"]["entries"]),expected)
                            self.assertEqual(filtered["data"]["displayed_count"],2 if source == key else 3)
                    for source,section,extra in [
                        (retail_key,"toy_collection_view",{}),(key,"equipment",{}),
                        (key,"toy_collection_view",{"toy_item_id":1}),
                        (key,"toy_collection_view",{"toy_in_view":{"item_id":1}}),
                    ]:
                        reply=await session.call_tool("inspect_snapshot",{"snapshot_id":source,
                            "section":section,"toy_favorite":True,**extra})
                        self.assertTrue(reply.isError)
                    selector = {"item_id":1,"collected":True,"favorite":False}
                    for source, matched in [(key,True),(after_key,False),(missing_key,False),(empty_key,False)]:
                        response = await session.call_tool("inspect_snapshot", {"snapshot_id":source,
                            "section":"toy_collection_view","toy_in_view":selector})
                        self.assertFalse(response.isError,response)
                        inspected = json.loads(response.content[0].text)
                        self.assertEqual(inspected["toy_in_view_match"],
                                         {"coverage":"filtered_journal",**selector} if matched else None)
                        if source == key:
                            self.assertEqual(inspected["data"],captured["sections"]["toy_collection_view"])
                    for source, section, extra in [
                        (retail_key,"toy_collection_view",{}),(key,"equipment",{}),
                        (key,"toy_collection_view",{"toy_item_id":1}),
                    ]:
                        response = await session.call_tool("inspect_snapshot", {"snapshot_id":source,
                            "section":section,"toy_in_view":selector,**extra})
                        self.assertTrue(response.isError)
                    for source, rows, collected in ((key, 2, 1), (empty_key, 0, 0), (missing_key, None, None)):
                        result = await session.call_tool("character_summary", {"snapshot_id": source})
                        data = json.loads(result.content[0].text)
                        self.assertEqual(data["toy_rows_observed"], rows)
                        self.assertEqual(data["collected_toys_in_view"], collected)
                        self.assertEqual(data["favorite_toys_in_view"], None if rows is None else 0)
                        self.assertEqual(data["collected_favorite_toys_in_view"], None if rows is None else 0)
                    favorite = await session.call_tool("character_summary", {"snapshot_id":after_key})
                    self.assertFalse(favorite.isError,favorite)
                    favorite_counts = json.loads(favorite.content[0].text)
                    self.assertEqual(favorite_counts["favorite_toys_in_view"],1)
                    self.assertEqual(favorite_counts["collected_favorite_toys_in_view"],0)
                    for source in invalid:
                        self.assertTrue((await session.call_tool("inspect_snapshot", {"snapshot_id": source, "section": "toy_collection_view"})).isError)
                    for source, item_id, observed, collected in (
                        (key, 1, True, True), (key, 2, True, False),
                        (key, 99, False, None), (empty_key, 1, False, None), (missing_key, 1, None, None),
                    ):
                        selected = await session.call_tool("inspect_snapshot", {
                            "snapshot_id": source, "section": "toy_collection_view", "toy_item_id": item_id})
                        self.assertFalse(selected.isError)
                        data = json.loads(selected.content[0].text)
                        self.assertEqual(data["observed_in_view"], observed)
                        self.assertEqual(data["collected_in_capture"], collected)
                    compared = await session.call_tool("compare_snapshots", {
                        "before_id": key, "after_id": after_key, "toy_view_changes": True})
                    self.assertFalse(compared.isError)
                    change = json.loads(compared.content[0].text)["toy_view_changes"]
                    self.assertEqual(change["coverage"], "filtered_journal")
                    self.assertEqual(change["observed_only_before"], [2])
                    self.assertEqual(change["observed_only_after"], [3, 10])
                    self.assertEqual(change["changes"], [{"item_id": 1, "fields": {
                        "collected": {"before": True, "after": False},
                        "favorite": {"before": False, "after": True}}}])
                    unknown = await session.call_tool("compare_snapshots", {
                        "before_id": key, "after_id": missing_key, "toy_view_changes": True})
                    self.assertIsNone(json.loads(unknown.content[0].text)["toy_view_changes"])
                    ordinary = await session.call_tool("compare_snapshots", {"before_id": key, "after_id": after_key})
                    self.assertNotIn("toy_view_changes", json.loads(ordinary.content[0].text))
                    wrong_family = await session.call_tool("compare_snapshots", {
                        "before_id": retail_key, "after_id": retail_key, "toy_view_changes": True})
                    self.assertTrue(wrong_family.isError)
                    for args in ({"snapshot_id": key, "toy_item_id": 1},
                                 {"snapshot_id": key, "section": "equipment", "toy_item_id": 1},
                                 {"snapshot_id": key, "section": "toy_collection_view", "toy_item_id": 1, "mount_id": 1},
                                 {"snapshot_id": retail_key, "section": "toy_collection_view", "toy_item_id": 1}):
                        self.assertTrue((await session.call_tool("inspect_snapshot", args)).isError)


if __name__ == "__main__":
    unittest.main()
