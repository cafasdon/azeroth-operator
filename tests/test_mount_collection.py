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
                        summary = json.loads(response.content[0].text)
                        self.assertEqual(summary["collected_mounts_observed"], count)
                        self.assertEqual(summary["mount_observed_flag_counts"],
                                         None if count is None else {
                                             "usable":count,"favorite":0,"usable_favorite":0})
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
                    for source, flags, expected in [
                        (after_key, {"mount_usable":True}, {"3","10"}),
                        (after_key, {"mount_usable":False,"mount_favorite":True}, {"2"}),
                        (after_key, {"mount_usable":True,"mount_favorite":True}, set()),
                        (after_key, {"mount_spell_id":40002,"mount_usable":False}, {"2"}),
                        (after_key, {"mount_spell_id":40002,"mount_usable":True}, set()),
                        (missing_key, {"mount_favorite":False}, None),
                    ]:
                        filtered=await session.call_tool("inspect_snapshot",{"snapshot_id":source,
                            "section":"mount_collection",**flags})
                        self.assertFalse(filtered.isError,filtered)
                        result=json.loads(filtered.content[0].text)
                        self.assertEqual(result["mount_matched_count"],None if expected is None else len(expected))
                        if expected is not None:
                            self.assertEqual(set(result["data"]["entries"]),expected)
                            self.assertEqual(result["data"]["catalogue_count"],4)
                    for source,section,extra in [
                        (retail_key,"mount_collection",{}),(after_key,"equipment",{}),
                        (after_key,"mount_collection",{"mount_id":2}),
                        (after_key,"mount_collection",{"collected_mount":{"mount_id":2}}),
                    ]:
                        filtered=await session.call_tool("inspect_snapshot",{"snapshot_id":source,
                            "section":section,"mount_favorite":True,**extra})
                        self.assertTrue(filtered.isError)
                    selector = {"mount_id":2,"usable":False,"favorite":True}
                    for source, expected in [(after_key, {"spell_id":40002, **selector}),
                                             (before_key,None),(missing_key,None)]:
                        inspected = await session.call_tool("inspect_snapshot", {
                            "snapshot_id":source,"section":"mount_collection","collected_mount":selector,
                        })
                        self.assertFalse(inspected.isError,inspected)
                        self.assertEqual(json.loads(inspected.content[0].text)["collected_mount_match"],expected)
                    for source, section in [(retail_key,"mount_collection"),(after_key,"equipment")]:
                        rejected = await session.call_tool("inspect_snapshot", {
                            "snapshot_id":source,"section":section,"collected_mount":selector,
                        })
                        self.assertTrue(rejected.isError)
                    self.assertEqual(changes["observed_only_before"], [20])
                    self.assertEqual(changes["observed_only_after"], [3, 10])
                    self.assertEqual(changes["collected_count_delta"], 1)
                    self.assertEqual(changes["before_observed_flag_counts"],
                                     {"usable":2,"favorite":0,"usable_favorite":0})
                    self.assertEqual(changes["after_observed_flag_counts"],
                                     {"usable":2,"favorite":1,"usable_favorite":0})
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
                    query = {"family": "forever", "collected_mount": {"mount_id": 2}}
                    filtered = await session.call_tool("list_snapshots", {
                        **query, "collected_mount": {"mount_id": 2, "usable": False, "favorite": True}})
                    matches = json.loads(filtered.content[0].text)["snapshots"]
                    self.assertEqual([row["snapshot_id"] for row in matches], [after_key])
                    self.assertEqual(matches[0]["collected_mount_match"], {
                        "mount_id": 2, "spell_id": 40002, "usable": False, "favorite": True})
                    async def by_name(name, **options):
                        reply = await session.call_tool("list_snapshots", {
                            "family":"forever","collected_mount_name":name,**options})
                        self.assertFalse(reply.isError,reply)
                        return json.loads(reply.content[0].text)["snapshots"]
                    named = await by_name("  AfTeR OnLy ")
                    self.assertEqual([row["snapshot_id"] for row in named], [after_key])
                    report = named[0]["collected_mount_name_match"]
                    self.assertEqual(report["matched_count"], 2)
                    self.assertEqual([row["mount_id"] for row in report["mounts"]], [3,10])
                    self.assertEqual(await by_name("After"), [])
                    self.assertEqual(await by_name("After only",collected_mount={"mount_id":2}), [])
                    self.assertEqual(await by_name("After only",collected_mount={"mount_id":3,"usable":False}), [])
                    matched = await by_name("After only",collected_mount={"mount_id":3,"usable":True})
                    self.assertEqual(matched[0]["collected_mount_name_match"]["matched_count"], 1)
                    for selectors, expected in [
                        ({"collected_mount_spell_id":40010},{after_key}),
                        ({"collected_mount_spell_id":40010,"collected_mount_name":"After only"},{after_key}),
                        ({"collected_mount_spell_id":40010,"collected_mount":{"mount_id":3}},set()),
                        ({"collected_mount_spell_id":40010,"collected_mount_name":"Translated name"},set()),
                    ]:
                        reply=await session.call_tool("list_snapshots",{"family":"forever",**selectors})
                        self.assertFalse(reply.isError,reply)
                        rows=json.loads(reply.content[0].text)["snapshots"]
                        self.assertEqual({row["snapshot_id"] for row in rows},expected)
                        if expected:
                            self.assertEqual([mount["mount_id"] for mount in rows[0]["collected_mount_spell_match"]["mounts"]],[10])
                    self.assertTrue((await session.call_tool("list_snapshots",{
                        "family":"retail","collected_mount_spell_id":40010})).isError)
                    for family,name in [("retail","After only"),("forever"," "),("forever","é"*129)]:
                        self.assertTrue((await session.call_tool("list_snapshots",{
                            "family":family,"collected_mount_name":name})).isError)
                    older = await session.call_tool("list_snapshots", {
                        **query, "collected_mount": {"mount_id": 2, "favorite": False}})
                    self.assertEqual({row["snapshot_id"] for row in json.loads(older.content[0].text)["snapshots"]},
                                     {key, before_key})
                    latest = await session.call_tool("list_snapshots", {
                        **query, "latest_per_character": True, "collected_mount": {"mount_id": 2, "usable": True}})
                    self.assertEqual(json.loads(latest.content[0].text)["snapshots"], [])
                    absent = await session.call_tool("list_snapshots", {
                        **query, "collected_mount": {"mount_id": 99, "usable": False, "favorite": False}})
                    self.assertEqual(json.loads(absent.content[0].text)["snapshots"], [])
                    wrong_character = await session.call_tool("list_snapshots", {
                        **query, "character": {"name": "Other", "realm": "Test"}})
                    self.assertEqual(json.loads(wrong_character.content[0].text)["snapshots"], [])
                    paged = await session.call_tool("list_snapshots", {**query, "offset": 1, "limit": 1})
                    self.assertEqual(len(json.loads(paged.content[0].text)["snapshots"]), 1)
                    for family in ("retail", "classic"):
                        self.assertTrue((await session.call_tool("list_snapshots", {**query, "family": family})).isError)


if __name__ == "__main__":
    unittest.main()
