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


if __name__ == "__main__":
    unittest.main()
