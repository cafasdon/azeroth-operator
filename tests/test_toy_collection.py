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
                    result = await session.call_tool("inspect_snapshot", {"snapshot_id": key, "section": "toy_collection_view"})
                    self.assertFalse(result.isError)
                    self.assertEqual(json.loads(result.content[0].text)["data"], captured["sections"]["toy_collection_view"])
                    for source, rows, collected in ((key, 2, 1), (empty_key, 0, 0), (missing_key, None, None)):
                        result = await session.call_tool("character_summary", {"snapshot_id": source})
                        data = json.loads(result.content[0].text)
                        self.assertEqual(data["toy_rows_observed"], rows)
                        self.assertEqual(data["collected_toys_in_view"], collected)
                    for source in invalid:
                        self.assertTrue((await session.call_tool("inspect_snapshot", {"snapshot_id": source, "section": "toy_collection_view"})).isError)


if __name__ == "__main__":
    unittest.main()
