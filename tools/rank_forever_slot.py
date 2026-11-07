"""Rank single-slot swaps from a saved Forever snapshot using local MCP and WoWSims."""

import argparse
import asyncio
import json
import logging
import os
from pathlib import Path

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from tools.run_forever_sim import compare, read_object, request_from


async def collect(binary, snapshot_id, slot, errlog):
    params = StdioServerParameters(command=str(binary), env=dict(os.environ))
    async with stdio_client(params, errlog=errlog) as (reader, writer), ClientSession(reader, writer) as session:
        await session.initialize()

        async def call(tool, **args):
            result = await session.call_tool(tool, {"snapshot_id": snapshot_id, **args})
            if result.isError:
                raise ValueError("Local MCP rejected the snapshot or request")
            return json.loads(result.content[0].text)

        baseline = await call("review_equipment_selection", selections=[])
        if not baseline["checks"]["simulator_equipment_spec"]:
            raise ValueError("Captured baseline has unresolved equipment")
        drafts = [(None, baseline)]
        skipped = []
        offset = 0
        while True:
            page = await call("review_forever_owned_gear", simulator_slot=slot, offset=offset, limit=50)
            count = page["filtered_item_locations"]
            if count > 64:
                raise ValueError("Slot exceeds the 64-location ranking limit")
            for row in page["items"]:
                selection = row["draft_selection"]
                if selection is None:
                    raise ValueError("Local MCP did not supply a draft selection")
                source = selection["source"]
                if source["kind"] == "equipment" and source["slot"] != selection["target_slot"]:
                    skipped.append({"selection": selection, "reason": "would_vacate_another_slot"})
                    continue
                draft = await call("review_equipment_selection", selections=[selection])
                if draft["checks"]["simulator_equipment_spec"]:
                    drafts.append((selection, draft))
                else:
                    skipped.append({"selection": selection, "reason": "unresolved_simulator_draft"})
            offset += len(page["items"])
            if not page["has_more"]:
                if offset != count:
                    raise ValueError("Incomplete candidate listing")
                return drafts, skipped, page["coverage"]
            if not page["items"] or offset >= count:
                raise ValueError("Invalid candidate pagination")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mcp", type=Path, required=True, help="Absolute path to the trusted local MCP executable")
    parser.add_argument("--snapshot-id", required=True)
    parser.add_argument("--slot", required=True, help="Simulator slot, such as main_hand or finger1")
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--engine-sha256", required=True)
    parser.add_argument("--template", type=Path, required=True)
    parser.add_argument("--encounter-lookup", type=Path, required=True)
    parser.add_argument("--paired", action="store_true")
    args = parser.parse_args()
    try:
        if not args.mcp.is_absolute() or not args.mcp.is_file():
            raise ValueError("Provide an absolute local MCP executable path")
        template = read_object(args.template, 4 * 1024 * 1024)
        encounter = read_object(args.encounter_lookup, 4 * 1024 * 1024)
        logging.disable(logging.CRITICAL)
        with open(os.devnull, "w") as errlog:
            drafts, skipped, coverage = asyncio.run(asyncio.wait_for(
                collect(args.mcp, args.snapshot_id, args.slot, errlog), timeout=45,
            ))
        requests = [request_from(template, draft, encounter) for _, draft in drafts]
        result = compare(args.engine, args.engine_sha256, requests, paired=args.paired)
        for row, (selection, _) in zip(result["results"], drafts, strict=True):
            row["selection"] = selection
        result["results"].sort(key=lambda row: -row["mean_dps"])
        result.update(slot=args.slot, skipped=skipped, storage_coverage=coverage)
        print(json.dumps(result, allow_nan=False))
    except Exception as error:
        # MCP and simulator failures may embed paths or private response text.
        parser.exit(1, f"Forever slot ranking failed ({type(error).__name__}). Check inputs and budgets.\n")


if __name__ == "__main__":
    main()
