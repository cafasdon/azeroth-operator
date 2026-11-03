import os
import json
import sys
import tempfile
import unittest
from pathlib import Path
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from shared.snapshots import SnapshotStore
from test_snapshots import fixture


class MCPTests(unittest.IsolatedAsyncioTestCase):
    async def test_real_stdio_server(self):
        with tempfile.TemporaryDirectory() as folder:
            key = SnapshotStore(Path(folder)).ingest(fixture())
            rust_binary = os.environ.get("AZEROTH_TEST_BINARY")
            params = StdioServerParameters(command=rust_binary or sys.executable,
                args=[] if rust_binary else [str(Path(__file__).resolve().parents[1] / "server.py")],
                env={**os.environ, "AZEROTH_DATA_DIR": folder})
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    tools = await session.list_tools()
                    expected = {"capabilities", "inspect_snapshot"}
                    if rust_binary:
                        expected.update({"list_snapshots", "compare_snapshots", "character_summary"})
                    self.assertEqual({t.name for t in tools.tools}, expected)
                    if rust_binary:
                        listing = await session.call_tool("list_snapshots", {"family": "retail"})
                        self.assertEqual(json.loads(listing.content[0].text)["snapshots"][0]["snapshot_id"], key)
                        other = await session.call_tool("list_snapshots", {"family": "forever"})
                        self.assertEqual(json.loads(other.content[0].text)["snapshots"], [])
                        comparison = await session.call_tool("compare_snapshots", {"before_id": key, "after_id": key})
                        self.assertEqual(json.loads(comparison.content[0].text)["sections"]["equipment"]["status"], "unchanged")
                        summary = await session.call_tool("character_summary", {"snapshot_id": key})
                        self.assertFalse(json.loads(summary.content[0].text)["live"])
                    result = await session.call_tool("inspect_snapshot", {"snapshot_id": key})
                    self.assertFalse(result.isError)
                    payload = json.loads(result.content[0].text)
                    self.assertFalse(payload["live"])
                    rejected = await session.call_tool("inspect_snapshot", {"snapshot_id": "../secret"})
                    self.assertTrue(rejected.isError)
