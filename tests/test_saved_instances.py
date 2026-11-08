"""Saved lockout cache observations omit private IDs and remain owner scoped."""

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


class SavedInstancesTests(unittest.IsolatedAsyncioTestCase):
    async def test_readback_and_validation(self):
        binary = Path(__file__).resolve().parents[1] / "target/debug" / (
            "azeroth-operator.exe" if os.name == "nt" else "azeroth-operator")
        binary = Path(os.environ.get("AZEROTH_TEST_BINARY", binary)).resolve(strict=True)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            store = SnapshotStore(root)
            value = fixture()
            value["client"] = {"family": "forever", "variant": "live", "build": "70862", "interface": 16001}
            value["sections"]["saved_instances"] = {"coverage": "updated_cache", "update_age_seconds": 10, "entries": {
                "1": {"name": "Raid", "difficulty_id": 3, "reset_seconds": 124, "locked": True,
                      "extended": False, "is_raid": True, "max_players": 40},
                "2": {"name": "Dungeon", "difficulty_id": 1, "reset_seconds": -1, "locked": False,
                      "extended": False, "is_raid": False, "max_players": 5}}}
            key = store.ingest(value)
            empty = copy.deepcopy(value)
            empty["sections"]["saved_instances"]["entries"] = {}
            empty_key = store.ingest(empty)
            missing = copy.deepcopy(value)
            missing["sections"].pop("saved_instances")
            missing_key = store.ingest(missing)
            invalid = []
            for change in (
                lambda x: x["client"].update(family="retail"),
                lambda x: x["sections"]["saved_instances"].update(coverage="live"),
                lambda x: x["sections"]["saved_instances"].update(update_age_seconds=True),
                lambda x: x["sections"]["saved_instances"]["entries"]["1"].update(lockout_id="private"),
                lambda x: x["sections"]["saved_instances"]["entries"]["1"].update(locked=1),
                lambda x: x["sections"]["saved_instances"]["entries"]["1"].update(reset_seconds=0.5),
                lambda x: x["sections"]["saved_instances"]["entries"].pop("1"),
            ):
                bad = copy.deepcopy(value)
                change(bad)
                with self.assertRaises(ValueError): validate_snapshot(bad)
                raw = json.dumps(bad, sort_keys=True, separators=(",", ":")).encode()
                bad_key = hashlib.sha256(raw).hexdigest()
                (root/f"{bad_key}.json").write_bytes(raw)
                invalid.append(bad_key)
            env = {**os.environ, "AZEROTH_DATA_DIR": directory}
            env.pop("AZEROTH_FOREVER_CATALOGUE", None)
            async with stdio_client(StdioServerParameters(command=str(binary), env=env)) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    response = await session.call_tool("inspect_snapshot", {"snapshot_id": key, "section": "saved_instances"})
                    self.assertFalse(response.isError)
                    self.assertEqual(json.loads(response.content[0].text)["data"], value["sections"]["saved_instances"])
                    for source, count, locked in ((key, 2, 1), (empty_key, 0, 0), (missing_key, None, None)):
                        response = await session.call_tool("character_summary", {"snapshot_id": source})
                        data = json.loads(response.content[0].text)
                        self.assertEqual(data["saved_instances_observed"], count)
                        self.assertEqual(data["locked_instances_observed"], locked)
                    for source in invalid:
                        self.assertTrue((await session.call_tool("inspect_snapshot", {"snapshot_id": source, "section": "saved_instances"})).isError)


if __name__ == "__main__": unittest.main()
