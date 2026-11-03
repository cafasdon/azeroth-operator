"""Independent protocol smoke test for any stdio launch command (no credentials)."""
import asyncio
import json
import os
import sys

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.shared.exceptions import McpError


async def smoke(command):
    params = StdioServerParameters(command=command[0], args=command[1:], env=dict(os.environ))
    async with stdio_client(params) as (reader, writer):
        async with ClientSession(reader, writer) as session:
            await session.initialize()
            tools = await session.list_tools()
            expected = {"capabilities", "list_snapshots", "inspect_snapshot",
                        "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout", "prepare_macro_operation", "prepare_binding_operation", "prepare_equipment_operation", "prepare_bag_operation"}
            assert {tool.name for tool in tools.tools} == expected
            for tool in tools.tools:
                assert tool.annotations.readOnlyHint is True
                assert tool.annotations.destructiveHint is False
            capabilities = await session.call_tool("capabilities", {})
            assert not capabilities.isError
            contract = json.loads(capabilities.content[0].text)
            assert set(contract["implemented"]) == expected
            assert contract["game_execution"] is False
            assert contract["live_client_qualified"] == []
            try:
                await session.call_tool("equip_item", {"slot": 16, "item_id": 123})
            except McpError as error:
                assert "tool not found" in str(error)
            else:
                raise AssertionError("Unknown mutation tool was accepted")
            for family in ("retail", "classic", "forever"):
                result = await session.call_tool("list_snapshots", {"family": family})
                assert not result.isError
                assert json.loads(result.content[0].text)["snapshots"] == []
            checks = {
                "inspect_snapshot": {"snapshot_id": "../secret"},
                "character_summary": {"snapshot_id": "0" * 64},
                "captured_talent_loadout": {"snapshot_id": "0" * 64},
                "compare_snapshots": {"before_id": "0" * 64, "after_id": "0" * 64},
                "review_equipment_selection": {"snapshot_id": "0" * 64, "selections": [
                    {"target_slot": 16, "source": {"kind": "equipment", "slot": 16}, "expected_item_id": 123}]},
            }
            for name, arguments in checks.items():
                result = await session.call_tool(name, arguments)
                assert result.isError, f"{name} accepted a missing/invalid snapshot"
            print(json.dumps({"tools": sorted(expected), "families": 3,
                              "empty_store": True, "traversal_denied": True,
                              "invalid_snapshot_tools_rejected": len(checks), "unknown_mutation_rejected": True}))


if __name__ == "__main__":
    command = sys.argv[1:]
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        raise SystemExit("Usage: smoke_mcp.py -- <stdio command> [arguments...]")
    asyncio.run(asyncio.wait_for(smoke(command), timeout=60))
