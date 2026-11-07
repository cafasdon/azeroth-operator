"""Rank equipment combinations from a saved Forever snapshot using local MCP and WoWSims."""

import argparse
import asyncio
from itertools import product
import json
import logging
from math import fsum, isfinite, prod
import os
from pathlib import Path

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from tools.run_forever_sim import CLASS_IDS, compare, compare_bulk, player_spec, read_object, request_from, single_player


SIMULATOR_SLOTS = (
    "head", "neck", "shoulder", "back", "chest", "wrist", "hands", "waist",
    "legs", "feet", "finger1", "finger2", "trinket1", "trinket2",
    "main_hand", "off_hand", "ranged",
)
# Race enum IDs from the pinned Forever common.proto.
CAPTURED_RACES = {
    "bloodelf": 1, "dwarf": 2, "gnome": 3, "human": 4, "nightelf": 5,
    "orc": 6, "tauren": 7, "troll": 8, "undead": 9,
    "highorderskyborne": 10, "windshaperskyborne": 11,
}
# Pinned WoWSims SPEC_CAN_DUAL_WIELD.
DUAL_WIELD_SPECS = frozenset(("hunter", "rogue", "dpsWarrior", "protectionWarrior"))


def check_template_identity(template, stats):
    player = single_player(template["raid"])
    captured_class = (stats.get("class") or "").lower()
    captured_race = (stats.get("race") or "").lower()
    if captured_race == "scourge":
        captured_race = "undead"
    checks = {}
    for field, token, known in (("class", captured_class, CLASS_IDS),
                                ("race", captured_race, CAPTURED_RACES)):
        if token in known:
            expected = player.get(field, "")
            if isinstance(expected, str):
                matches = expected.removeprefix(field.capitalize()).lower() == token
            else:
                matches = type(expected) is int and expected == known[token]
            if not matches:
                raise ValueError("Simulator template disagrees with captured character identity")
            checks[field] = "matches"
        else:
            checks[field] = "unverified"
    return checks


async def slot_candidates(call, slot, multi, collapse_identical, item_ids=None, can_dual_wield=True,
                          selected_slots=None, set_id=None):
    candidates = []
    skipped = []
    offset = 0
    representatives = {}
    count = None
    filters = {"item_ids": item_ids} if item_ids else {}
    if set_id is not None:
        filters["set_id"] = set_id
    while True:
        page = await call("review_forever_owned_gear", simulator_slot=slot, offset=offset, limit=50, **filters)
        if count is None:
            count = page["filtered_item_locations"]
            if not 0 <= count <= 256:
                raise ValueError("Slot exceeds the 256-location scan limit")
        rows = page["items"]
        next_offset = offset + len(rows)
        if (page["filtered_item_locations"] != count or len(rows) != min(50, count - offset)
                or page["has_more"] != (next_offset < count)):
            raise ValueError("Invalid candidate pagination")
        for row in rows:
            selection = row["draft_selection"]
            if selection is None:
                raise ValueError("Local MCP did not supply a draft selection")
            source = selection["source"]
            same_slot = source["kind"] == "equipment" and source["slot"] == selection["target_slot"]
            if source["kind"] == "equipment":
                if multi and same_slot:
                    continue
                if not multi and not same_slot:
                    skipped.append({"selection": selection, "reason": "would_vacate_another_slot"})
                    continue
            if conflict := row.get("catalogue_requirement_conflict"):
                skipped.append({
                    **({"selections": [selection]} if multi else {"selection": selection}),
                    "reason": "catalogue_requirement_conflict",
                    "conflicts": [conflict],
                })
                continue
            reference = row.get("catalogue_base_item") or {}
            if (not can_dual_wield and slot == "off_hand" and not same_slot
                    and reference.get("type") == 13 and reference.get("handType") in (2, 3)
                    and reference.get("weaponType") in (1, 2, 3, 4, 6, 8, 9)):
                skipped.append({
                    **({"selections": [selection]} if multi else {"selection": selection}),
                    "reason": "specialisation_cannot_dual_wield",
                })
                continue
            modifier_review = row.get("catalogue_modifier_candidates", {})
            modifiers = modifier_review or {}
            spec = modifiers.get("simulator_item_spec")
            if modifier_review is None or ("simulator_item_spec" in modifiers and spec is None):
                skipped.append({
                    **({"selections": [selection]} if multi else {"selection": selection}),
                    "reason": "unresolved_simulator_item",
                })
                continue
            if collapse_identical and source["kind"] != "equipment":
                eligible_slots = reference.get("simulator_slots")
                if spec and eligible_slots in (
                    [slot], ["finger1", "finger2"], ["trinket1", "trinket2"], ["main_hand", "off_hand"],
                ):
                    targets = selected_slots if selected_slots is not None else (eligible_slots if multi else [slot])
                    copies_needed = max(1, len(set(eligible_slots).intersection(targets)))
                    key = json.dumps(spec, sort_keys=True, separators=(",", ":"))
                    kept = representatives.setdefault(key, [])
                    if len(kept) == copies_needed:
                        skipped.append({
                            "selection": selection, "reason": "equivalent_simulator_item",
                            "representative": kept[0],
                            **({"retained_copies": kept} if copies_needed > 1 else {}),
                        })
                        continue
                    kept.append(selection)
            candidates.append(selection)
        offset = next_offset
        if not page["has_more"]:
            return candidates, skipped, page["coverage"]


async def collect(binary, snapshot_id, slots, errlog, collapse_identical=False, item_ids=None,
                  can_dual_wield=True, character=None, set_id=None):
    params = StdioServerParameters(command=str(binary), env=dict(os.environ))
    async with stdio_client(params, errlog=errlog) as (reader, writer), ClientSession(reader, writer) as session:
        await session.initialize()

        async def request(tool, **args):
            result = await session.call_tool(tool, args)
            if result.isError:
                raise ValueError("Local MCP rejected the snapshot or request")
            return json.loads(result.content[0].text)

        selected_capture = None
        if character is not None:
            name, realm = character
            listing = await request("list_snapshots", family="forever",
                                    character={"name": name, "realm": realm}, limit=2)
            rows = listing["snapshots"]
            if not rows or (len(rows) > 1 and rows[0]["captured_at"] == rows[1]["captured_at"]):
                raise ValueError("No unambiguous newest saved capture for this character")
            snapshot_id = rows[0]["snapshot_id"]
            selected_capture = {"snapshot_id": snapshot_id, "captured_at": rows[0]["captured_at"]}

        async def call(tool, **args):
            return await request(tool, snapshot_id=snapshot_id, **args)

        stats = await call("inspect_snapshot", section="stats")
        baseline = await call("review_equipment_selection", selections=[])
        if not baseline["checks"]["simulator_equipment_spec"]:
            raise ValueError("Captured baseline has unresolved equipment")
        set_observation = None
        if set_id is not None:
            set_observation = (await call("review_forever_owned_gear", set_id=set_id, limit=1))["set_observation"]
        drafts = [([], baseline)]
        skipped = []
        options = []
        coverage = None
        multi = len(slots) > 1
        for slot in slots:
            candidates, exclusions, coverage = await slot_candidates(
                call, slot, multi, collapse_identical, item_ids, can_dual_wield, slots, set_id,
            )
            skipped.extend(exclusions)
            options.append([None, *candidates])
        if prod(map(len, options)) > 1024:
            raise ValueError("Slot combinations exceed the 1024-combination scan limit")
        proposed = []
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
            proposed.append(draft_args)
            if len(proposed) > 64:
                raise ValueError("Slot combinations exceed the 64-draft ranking limit")
        for draft_args in proposed:
            selections = draft_args["selections"]
            draft = await call("review_equipment_selection", **draft_args)
            if draft["checks"]["simulator_equipment_spec"]:
                drafts.append((selections, draft))
            elif conflicts := draft["checks"].get("catalogue_requirement_conflicts"):
                skipped.append({
                    **({"selections": selections} if multi else {"selection": selections[0]}),
                    "reason": "catalogue_requirement_conflict",
                    "conflicts": conflicts,
                })
            elif multi:
                skipped.append({"selections": selections, "reason": "unresolved_simulator_draft"})
            else:
                skipped.append({"selection": selections[0], "reason": "unresolved_simulator_draft"})
        return drafts, skipped, coverage, stats.get("data") or {}, selected_capture, set_observation


def rank_encounter(args, template, encounter, drafts, set_piece_ids=None):
    requests = [request_from(template, draft, encounter) for _, draft in drafts]
    result = (compare_bulk(args.bulk_engine, args.bulk_engine_sha256, requests, args.top)
              if args.bulk_engine else None)
    if result is None:
        result = compare(args.engine, args.engine_sha256, requests, paired=args.paired)
    for row in result["results"]:
        selections, draft = drafts[row["review"] - 1]
        if set_piece_ids is not None:
            equipment = draft["checks"]["simulator_equipment_spec"]["items"]
            row["selected_set_equipped_pieces"] = sum(item.get("id") in set_piece_ids for item in equipment)
        cleared = draft["checks"].get("simulator_cleared_equipment_slots")
        if cleared:
            row["cleared_equipment_slots"] = cleared
        if len(args.slot) == 1:
            row["selection"] = selections[0] if selections else None
        else:
            row["selections"] = selections
    result["results"].sort(key=lambda row: -row["mean_dps"])
    return result


def weighted_ranking(rankings, weights):
    gains = {}
    for ranking, weight in zip(rankings, weights, strict=True):
        rows = ranking["results"]
        baseline = next(row["mean_dps"] for row in rows if row["review"] == ranking["baseline_review"])
        if baseline <= 0:
            raise ValueError("Weighted ranking requires positive baseline DPS in every encounter")
        for row in rows:
            gains.setdefault(row["review"], []).append(weight * (row["mean_dps_delta_from_baseline"] / baseline))
    if any(len(parts) != len(weights) for parts in gains.values()):
        raise ValueError("Weighted ranking requires every draft in every encounter")
    results = [{"review": review, "weighted_dps_gain_pct": 100 * fsum(parts)}
               for review, parts in gains.items()]
    set_counts = {row["review"]: row["selected_set_equipped_pieces"]
                  for row in rankings[0]["results"] if "selected_set_equipped_pieces" in row}
    for row in results:
        if row["review"] in set_counts:
            row["selected_set_equipped_pieces"] = set_counts[row["review"]]
    results.sort(key=lambda row: (-row["weighted_dps_gain_pct"], row["review"]))
    return {"encounter_weights": weights, "results": results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mcp", type=Path, required=True, help="Absolute path to the trusted local MCP executable")
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument("--snapshot-id")
    target.add_argument("--character", nargs=2, metavar=("NAME", "REALM"),
                        help="Use the newest saved Forever capture for this character and realm.")
    slots = parser.add_mutually_exclusive_group(required=True)
    slots.add_argument("--slot", action="append", choices=SIMULATOR_SLOTS, help="Simulator slot. Repeat for combinations.")
    slots.add_argument("--all-slots", action="store_const", const=SIMULATOR_SLOTS, dest="slot",
                       help="Consider all 17 simulator slots within the search limits.")
    parser.add_argument("--collapse-identical-items", action="store_true",
                        help="Keep only the storage copies needed to fill equivalent simulator slots.")
    parser.add_argument("--item-id", type=int, action="append",
                        help="Restrict alternatives to this base item ID. Repeat for a shortlist.")
    parser.add_argument("--set-id", type=int, help="Restrict alternatives to a pinned catalogue set ID.")
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--engine-sha256", required=True)
    parser.add_argument("--template", type=Path, required=True)
    parser.add_argument("--encounter-lookup", type=Path, action="append", required=True,
                        help="Encounter export. Repeat for separate rankings.")
    parser.add_argument("--encounter-weight", type=float, action="append",
                        help="Relative weight in encounter input order. Fixed comparisons only.")
    parser.add_argument("--paired", action="store_true")
    parser.add_argument("--bulk-engine", type=Path, help="Trusted local Forever bulk adapter")
    parser.add_argument("--bulk-engine-sha256")
    parser.add_argument("--top", type=int, default=5, help="Distinct gear sets to return in bulk mode")
    args = parser.parse_args()
    try:
        if not args.mcp.is_absolute() or not args.mcp.is_file():
            raise ValueError("Provide an absolute local MCP executable path")
        if len(set(args.slot)) != len(args.slot):
            raise ValueError("Provide distinct slots")
        if args.item_id and (len(args.item_id) > 16 or len(set(args.item_id)) != len(args.item_id)
                             or any(not 1 <= item_id <= 2**32 - 1 for item_id in args.item_id)):
            raise ValueError("Provide at most sixteen distinct positive item IDs")
        if args.set_id is not None and not 1 <= args.set_id <= 2**32 - 1:
            raise ValueError("Provide a positive catalogue set ID")
        if bool(args.bulk_engine) != bool(args.bulk_engine_sha256) or not 1 <= args.top <= 64:
            raise ValueError("Provide both bulk adapter arguments and a bounded shortlist")
        if len(args.encounter_lookup) > 4:
            raise ValueError("Provide up to four encounter exports")
        weights = args.encounter_weight
        if weights is not None:
            if (args.bulk_engine or len(weights) < 2 or len(weights) != len(args.encounter_lookup)
                    or any(not isfinite(weight) or weight <= 0 for weight in weights)):
                raise ValueError("Provide one positive finite weight per encounter in fixed mode")
            largest = max(weights)
            scaled = [weight / largest for weight in weights]
            total = fsum(scaled)
            weights = [weight / total for weight in scaled]
            if any(weight == 0 for weight in weights):
                raise ValueError("Encounter weights are too far apart to normalise")
        template = read_object(args.template, 4 * 1024 * 1024)
        can_dual_wield = player_spec(single_player(template["raid"])) in DUAL_WIELD_SPECS
        encounters = [read_object(path, 4 * 1024 * 1024) for path in args.encounter_lookup]
        logging.disable(logging.CRITICAL)
        with open(os.devnull, "w") as errlog:
            drafts, skipped, coverage, stats, selected_capture, set_observation = asyncio.run(asyncio.wait_for(
                collect(args.mcp, args.snapshot_id, args.slot, errlog,
                        args.collapse_identical_items, args.item_id, can_dual_wield,
                        args.character, args.set_id), timeout=45,
            ))
        identity = check_template_identity(template, stats)
        set_piece_ids = ({piece["item_id"] for piece in set_observation["catalogue_pieces"]}
                         if set_observation is not None else None)
        if len(encounters) == 1:
            result = rank_encounter(args, template, encounters[0], drafts, set_piece_ids)
        else:
            for encounter in encounters:
                request_from(template, drafts[0][1], encounter)
            rankings = []
            for index, encounter in enumerate(encounters, 1):
                ranking = rank_encounter(args, template, encounter, drafts, set_piece_ids)
                profile = encounter["simulation_input"]["encounter"]
                ranking.update(
                    encounter_index=index,
                    duration_seconds=profile["duration"],
                    duration_variation_seconds=profile["durationVariation"],
                    target_count=len(profile["targets"]),
                )
                rankings.append(ranking)
            result = {"encounters": rankings}
            if weights is not None:
                result["weighted_ranking"] = weighted_ranking(rankings, weights)
        result.update(skipped=skipped, storage_coverage=coverage, template_identity=identity)
        if selected_capture is not None:
            result["selected_capture"] = selected_capture
        if args.item_id:
            result["requested_item_ids"] = args.item_id
        if args.set_id is not None:
            result["requested_set_id"] = args.set_id
            result["set_catalogue_piece_count"] = len(set_piece_ids)
            result["observed_distinct_set_pieces"] = set_observation["observed_distinct_pieces"]
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
