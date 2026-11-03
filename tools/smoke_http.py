"""Independent hosted MCP qualification; token only from the process environment."""
import argparse
import asyncio
import json
import os
import re
from mcp import ClientSession
from mcp.client.streamable_http import streamablehttp_client

URL = "https://mcp.cafas.com.br/scopes/azeroth/mcp"


async def smoke(snapshot=None, expect_loadout=False, expect_bags=False):
    token = os.environ["AZEROTH_MCP_TOKEN"]
    async with streamablehttp_client(URL, headers={"Authorization": "Bearer " + token}, timeout=20, sse_read_timeout=30) as (reader, writer, _):
        async with ClientSession(reader, writer) as session:
            await session.initialize()
            tools = await session.list_tools()
            assert {t.name for t in tools.tools} == {"capabilities", "inspect_snapshot", "list_snapshots", "compare_snapshots", "character_summary", "review_equipment_selection", "captured_talent_loadout", "prepare_macro_operation"}
            if snapshot:
                result = await session.call_tool("list_snapshots", {"family": "retail", "limit": 100})
                assert not result.isError
                listing = json.loads(result.content[0].text)
                assert snapshot in {row["snapshot_id"] for row in listing["snapshots"]}
                result = await session.call_tool("inspect_snapshot", {"snapshot_id": snapshot, "section": "equipment"})
                assert not result.isError
                value = json.loads(result.content[0].text)
                assert value["source"] == "fixture" and value["character"]["name"] == "GatewayFixture"
                assert value["data"]["16"]["item_id"] == 123 and value["live"] is False
                result = await session.call_tool("character_summary", {"snapshot_id": snapshot})
                assert not result.isError
                assert json.loads(result.content[0].text)["equipped_slots_observed"] == 1
                if expect_bags:
                    carried = json.loads(result.content[0].text)["carried_item_review"]
                    assert carried["quantity_observed"] == 5
                    assert carried["repeated_item_ids"][0]["occupied_slots_observed"] == 2
                    assert carried["stack_compatibility"] == "unknown"
                    assert carried["game_execution"] is False
                result = await session.call_tool("review_equipment_selection", {"snapshot_id": snapshot,
                    "selections": [{"target_slot": 16, "source": {"kind": "equipment", "slot": 16}, "expected_item_id": 123}]})
                assert not result.isError
                review = json.loads(result.content[0].text)
                assert review["snapshot_id"] == snapshot and review["game_execution"] is False
                assert review["changes"][0]["status"] == "already_selected"
                assert review["checks"]["performance_improvement"] == "unknown"
                result = await session.call_tool("review_equipment_selection", {"snapshot_id": snapshot,
                    "selections": [{"target_slot": 17, "source": {"kind": "equipment", "slot": 16}, "expected_item_id": 123}]})
                assert not result.isError
                changed = json.loads(result.content[0].text)
                assert changed["vacated_equipment_slots"] == [16]
                assert changed["changes"][0]["status"] == "requested_change"
                assert changed["checks"]["slot_compatibility"] == "unknown"
                result = await session.call_tool("review_equipment_selection", {"snapshot_id": snapshot,
                    "selections": [{"target_slot": 17, "source": {"kind": "equipment", "slot": 16}, "expected_item_id": 999}]})
                assert result.isError
                result = await session.call_tool("captured_talent_loadout", {"snapshot_id":snapshot})
                assert not result.isError
                captured = json.loads(result.content[0].text)
                assert captured["game_execution"] is False
                if expect_loadout:
                    assert captured["loadout"]["import_string"] == "QUJDRA=="
                    assert captured["compatibility"] == "not_live_verified"
                else:
                    assert captured["status"] == "unavailable"
            else:
                result = await session.call_tool("list_snapshots", {"family": "retail"})
                assert not result.isError
            print(json.dumps({"public_tls": True, "tools": 7, "fixture_readback": bool(snapshot)}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot")
    parser.add_argument("--expect-loadout", action="store_true")
    parser.add_argument("--expect-bags", action="store_true")
    args = parser.parse_args()
    if args.snapshot and not re.fullmatch("[a-f0-9]{64}", args.snapshot):
        parser.error("Invalid snapshot ID")
    if (args.expect_loadout or args.expect_bags) and not args.snapshot:
        parser.error("Expected snapshot for loadout qualification")
    asyncio.run(asyncio.wait_for(smoke(args.snapshot, args.expect_loadout, args.expect_bags), timeout=60))
