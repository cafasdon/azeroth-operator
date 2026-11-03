"""Independent protocol smoke test for any stdio launch command (no credentials)."""
import asyncio
import json
import os
import sys

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client


async def smoke(command):
    params = StdioServerParameters(command=command[0], args=command[1:], env=dict(os.environ))
    async with stdio_client(params) as (reader, writer):
        async with ClientSession(reader, writer) as session:
            await session.initialize()
            tools = await session.list_tools()
            expected = {"capabilities", "list_snapshots", "inspect_snapshot",
                        "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout"}
            assert {tool.name for tool in tools.tools} == expected
            for tool in tools.tools:
                assert tool.annotations.readOnlyHint is True
            capabilities = await session.call_tool("capabilities", {})
            assert not capabilities.isError
            for family in ("retail", "classic", "forever"):
                result = await session.call_tool("list_snapshots", {"family": family})
                assert not result.isError
                assert json.loads(result.content[0].text)["snapshots"] == []
            result = await session.call_tool("inspect_snapshot", {"snapshot_id": "../secret"})
            assert result.isError
            print(json.dumps({"tools": sorted(expected), "families": 3,
                              "empty_store": True, "traversal_denied": True}))


if __name__ == "__main__":
    command = sys.argv[1:]
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        raise SystemExit("Usage: smoke_mcp.py -- <stdio command> [arguments...]")
    asyncio.run(asyncio.wait_for(smoke(command), timeout=60))
