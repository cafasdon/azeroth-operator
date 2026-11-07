"""Rank equipment combinations from a saved Forever snapshot using local MCP and WoWSims."""

import argparse
import asyncio
from itertools import product
import json
import logging
from math import prod
import os
from pathlib import Path

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from tools.run_forever_sim import compare, compare_bulk, read_object, request_from


async def slot_candidates(call, slot, multi):
    candidates = []
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
            if source["kind"] == "equipment":
                same_slot = source["slot"] == selection["target_slot"]
                if multi and same_slot:
                    continue
                if not multi and not same_slot:
                    skipped.append({"selection": selection, "reason": "would_vacate_another_slot"})
                    continue
            candidates.append(selection)
        offset += len(page["items"])
        if not page["has_more"]:
            if offset != count:
                raise ValueError("Incomplete candidate listing")
            return candidates, skipped, page["coverage"]
        if not page["items"] or offset >= count:
            raise ValueError("Invalid candidate pagination")


async def collect(binary, snapshot_id, slots, errlog):
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
        drafts = [([], baseline)]
        skipped = []
        options = []
        coverage = None
        multi = len(slots) > 1
        for slot in slots:
            candidates, exclusions, coverage = await slot_candidates(call, slot, multi)
            skipped.extend(exclusions)
            options.append([None, *candidates])
        if prod(map(len, options)) > 65:
            raise ValueError("Slot combinations exceed the 64-draft ranking limit")
        for combination in product(*options):
            selections = [choice for choice in combination if choice is not None]
            if not selections:
                continue
            targets = {selection["target_slot"] for selection in selections}
            if any(selection["source"]["kind"] == "equipment" and selection["source"]["slot"] not in targets
                   for selection in selections):
                skipped.append({"selections": selections, "reason": "would_vacate_another_slot"})
                continue
            sources = {json.dumps(selection["source"], sort_keys=True) for selection in selections}
            if len(sources) != len(selections):
                skipped.append({"selections": selections, "reason": "reused_source_location"})
                continue
            draft_args = {"selections": selections}
            if 16 in targets and 17 not in targets:
                draft_args["allow_two_hand_off_hand_clear"] = True
            draft = await call("review_equipment_selection", **draft_args)
            if draft["checks"]["simulator_equipment_spec"]:
                drafts.append((selections, draft))
            elif multi:
                skipped.append({"selections": selections, "reason": "unresolved_simulator_draft"})
            else:
                skipped.append({"selection": selections[0], "reason": "unresolved_simulator_draft"})
        return drafts, skipped, coverage


def rank_encounter(args, template, encounter, drafts):
    requests = [request_from(template, draft, encounter) for _, draft in drafts]
    result = (compare_bulk(args.bulk_engine, args.bulk_engine_sha256, requests, args.top)
              if args.bulk_engine else None)
    if result is None:
        result = compare(args.engine, args.engine_sha256, requests, paired=args.paired)
    for row in result["results"]:
        selections, draft = drafts[row["review"] - 1]
        cleared = draft["checks"].get("simulator_cleared_equipment_slots")
        if cleared:
            row["cleared_equipment_slots"] = cleared
        if len(args.slot) == 1:
            row["selection"] = selections[0] if selections else None
        else:
            row["selections"] = selections
    result["results"].sort(key=lambda row: -row["mean_dps"])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mcp", type=Path, required=True, help="Absolute path to the trusted local MCP executable")
    parser.add_argument("--snapshot-id", required=True)
    parser.add_argument("--slot", action="append", required=True, help="Simulator slot. Repeat for combinations.")
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--engine-sha256", required=True)
    parser.add_argument("--template", type=Path, required=True)
    parser.add_argument("--encounter-lookup", type=Path, action="append", required=True,
                        help="Encounter export. Repeat for separate rankings.")
    parser.add_argument("--paired", action="store_true")
    parser.add_argument("--bulk-engine", type=Path, help="Trusted local Forever bulk adapter")
    parser.add_argument("--bulk-engine-sha256")
    parser.add_argument("--top", type=int, default=5, help="Distinct gear sets to return in bulk mode")
    args = parser.parse_args()
    try:
        if not args.mcp.is_absolute() or not args.mcp.is_file():
            raise ValueError("Provide an absolute local MCP executable path")
        if len(args.slot) > 4 or len(set(args.slot)) != len(args.slot):
            raise ValueError("Provide up to four distinct slots")
        if bool(args.bulk_engine) != bool(args.bulk_engine_sha256) or not 1 <= args.top <= 64:
            raise ValueError("Provide both bulk adapter arguments and a bounded shortlist")
        if len(args.encounter_lookup) > 4:
            raise ValueError("Provide up to four encounter exports")
        template = read_object(args.template, 4 * 1024 * 1024)
        encounters = [read_object(path, 4 * 1024 * 1024) for path in args.encounter_lookup]
        logging.disable(logging.CRITICAL)
        with open(os.devnull, "w") as errlog:
            drafts, skipped, coverage = asyncio.run(asyncio.wait_for(
                collect(args.mcp, args.snapshot_id, args.slot, errlog), timeout=45,
            ))
        if len(encounters) == 1:
            result = rank_encounter(args, template, encounters[0], drafts)
        else:
            for encounter in encounters:
                request_from(template, drafts[0][1], encounter)
            rankings = []
            for index, encounter in enumerate(encounters, 1):
                ranking = rank_encounter(args, template, encounter, drafts)
                profile = encounter["simulation_input"]["encounter"]
                ranking.update(
                    encounter_index=index,
                    duration_seconds=profile["duration"],
                    duration_variation_seconds=profile["durationVariation"],
                    target_count=len(profile["targets"]),
                )
                rankings.append(ranking)
            result = {"encounters": rankings}
        result.update(skipped=skipped, storage_coverage=coverage)
        if len(args.slot) == 1:
            result["slot"] = args.slot[0]
        else:
            result["slots"] = args.slot
        print(json.dumps(result, allow_nan=False))
    except Exception as error:
        # MCP and simulator failures may embed paths or private response text.
        parser.exit(1, f"Forever slot ranking failed ({type(error).__name__}). Check inputs and budgets.\n")


if __name__ == "__main__":
    main()
