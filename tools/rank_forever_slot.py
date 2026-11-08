"""Rank equipment combinations from a saved Forever snapshot using local MCP and WoWSims."""

import argparse
import asyncio
from itertools import combinations, product
import json
import logging
from math import fsum, isfinite, prod
import os
from pathlib import Path
from time import monotonic

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from tools.run_forever_sim import (
    CLASS_IDS, ConflictingConsumables, check_consumable_groups, compare, compare_bulk,
    encounter_key, encounter_summary, player_spec, read_object, request_from, single_player,
    SimulationCache,
)


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


class GearSearchLimit(ValueError):
    """A local search limit with a diagnostic safe to print."""


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
                          selected_slots=None, set_id=None, storage=None, candidate_filters=None):
    candidates = []
    skipped = []
    offset = 0
    representatives = {}
    count = None
    filters = dict(candidate_filters or {})
    if item_ids:
        filters["item_ids"] = item_ids
    if set_id is not None:
        filters["set_id"] = set_id
    if storage is not None:
        filters["storage"] = storage
    while True:
        page = await call("review_forever_owned_gear", simulator_slot=slot, offset=offset, limit=50, **filters)
        if count is None:
            count = page["filtered_item_locations"]
            if count < 0:
                raise ValueError("Invalid candidate count")
            if count > 256:
                raise GearSearchLimit(
                    "A slot exceeds the 256-location scan limit. Narrow the search with --storage, --item-id or --set-id."
                )
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
                  can_dual_wield=True, character=None, set_id=None, max_drafts=64, storage=None,
                  max_swaps=None, candidate_filters=None):
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
            simultaneous_slots = (slot,) if max_swaps == 1 else slots
            candidates, exclusions, coverage = await slot_candidates(
                call, slot, multi, collapse_identical, item_ids, can_dual_wield,
                simultaneous_slots, set_id, storage, candidate_filters,
            )
            skipped.extend(exclusions)
            options.append([None, *candidates])
        if max_swaps is not None:
            candidate_groups = [choices[1:] for choices in options if len(choices) > 1]
            slot_groups = (
                group
                for size in range(1, max_swaps + 1)
                for group in combinations(candidate_groups, size)
            )
            groups = []
            combination_count = 1
            for group in slot_groups:
                combination_count += prod(map(len, group))
                if combination_count > 1024:
                    break
                groups.append(group)
            drafts_to_scan = (draft for group in groups for draft in product(*group))
        else:
            combination_count = prod(map(len, options))
            drafts_to_scan = product(*options)
        if combination_count > 1024:
            raise GearSearchLimit(
                "The search exceeds 1,024 combinations. Use fewer slots, --max-swaps or a narrower candidate filter."
            )
        proposed = []
        for combination in drafts_to_scan:
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
            if len(proposed) > max_drafts:
                raise GearSearchLimit(
                    "The search exceeds --max-drafts. Narrow the candidates or increase that limit up to 128."
                )
        batch_size = 1
        if len(proposed) > 1:
            tools = (await session.list_tools()).tools
            if any(tool.name == "review_equipment_selection"
                   and "additional_drafts" in tool.inputSchema.get("properties", {}) for tool in tools):
                batch_size = 8
        reviews = []
        for start in range(0, len(proposed), batch_size):
            primary, *additional = proposed[start:start + batch_size]
            arguments = {**primary, "additional_drafts": additional} if additional else primary
            review = await call("review_equipment_selection", **arguments)
            extra = review.pop("additional_draft_reviews", [])
            if not isinstance(extra, list) or len(extra) != len(additional):
                raise ValueError("Local MCP returned an incomplete equipment review batch")
            reviews.extend([review, *extra])
        for draft_args, draft in zip(proposed, reviews, strict=True):
            selections = draft_args["selections"]
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
        search = {
            "candidate_locations_by_slot": {
                slot: len(choices) - 1 for slot, choices in zip(slots, options, strict=True)
            },
            "combinations_including_baseline": combination_count,
            "proposed_alternative_drafts": len(proposed),
            "resolved_drafts_including_baseline": len(drafts),
            "distinct_simulator_gear_sets": len({
                json.dumps(draft["checks"]["simulator_equipment_spec"], sort_keys=True)
                for _, draft in drafts
            }),
        }
        return drafts, skipped, coverage, stats.get("data") or {}, selected_capture, set_observation, search


def audit_shortlist(staged, fixed, requests, top):
    canonical = {}
    aliases = {}
    for review, request in enumerate(requests, 1):
        gear = single_player(request["raid"])["equipment"]
        key = json.dumps(gear, sort_keys=True, separators=(",", ":"), allow_nan=False)
        aliases[review] = canonical.setdefault(key, review)

    def unique_order(result):
        ordered = sorted(result["results"], key=lambda row: (-row["mean_dps"], row["review"]))
        return list(dict.fromkeys(aliases[row["review"]] for row in ordered if aliases[row["review"]] != 1))

    staged_reviews = unique_order(staged) if staged is not None else []
    fixed_reviews = unique_order(fixed)[:top]
    return {
        "fixed_iterations": requests[0]["simOptions"]["iterations"],
        "fixed_unique_gear_sets": fixed["unique_runs"],
        "staged_shortlist_reviews": staged_reviews,
        "fixed_top_reviews": fixed_reviews,
        "fixed_top_missing_from_shortlist": [review for review in fixed_reviews if review not in staged_reviews],
    }


def tradeoff_reviews(rows, score_for):
    scores = [score_for(row) for row in rows]
    return [
        row["review"]
        for row, score in zip(rows, scores, strict=True)
        if not any(other != score and all(a <= b for a, b in zip(other, score, strict=True))
                   for other in scores)
    ]


def rank_encounter(args, template, encounter, drafts, set_piece_ids=None, finalist_reviews=None):
    if finalist_reviews is not None:
        drafts = [drafts[review - 1] for review in finalist_reviews]
    requests = [request_from(template, draft, encounter) for _, draft in drafts]
    tank_ranking = args.rank_by in ("dtps", "tps", "death")
    healing_ranking = args.rank_by in ("hps", "oom")
    start = monotonic()
    cache = getattr(args, "simulation_cache", None)
    screening = finalist_reviews is None and args.bulk_engine
    fixed = (compare(args.engine, args.engine_sha256, requests, paired=args.paired, cache=cache)
             if screening and args.audit_shortlist else None)
    after_fixed = monotonic()
    result = (compare_bulk(args.bulk_engine, args.bulk_engine_sha256, requests, args.top, cache=cache)
              if screening else None)
    staged_wall = monotonic() - after_fixed if result is not None else None
    audit = audit_shortlist(result, fixed, requests, args.top) if fixed is not None else None
    if result is None:
        result = fixed if fixed is not None else compare(
            args.engine, args.engine_sha256, requests, paired=args.paired,
            tank_metrics=tank_ranking, healing_metrics=healing_ranking, cache=cache)
    result.update(encounter_summary(requests[0]))
    if args.confirm_leading_seed is not None and not (screening and args.encounter_weight):
        pair = result.get("observed_leading_dps_pair")
        if result.get("mode") == "staged_bulk":
            leaders = {}
            for row in sorted(result["results"], key=lambda row: (-row["mean_dps"], row["review"])):
                review = row["review"]
                gear = single_player(requests[review - 1]["raid"])["equipment"]
                leaders.setdefault(json.dumps(gear, sort_keys=True), review)
                if len(leaders) == 2:
                    leading, runner_up = leaders.values()
                    pair = {"leading_review": leading, "runner_up_review": runner_up}
                    break
        confirmation = {"seed": str(args.confirm_leading_seed), "status": "no_distinct_pair"}
        if pair is not None:
            # Keep the original runner-up as the reference even if the order reverses.
            reviews = (pair["runner_up_review"], pair["leading_review"])
            selected = [requests[review - 1] for review in reviews]
            fresh = [
                {**request, "simOptions": {**request["simOptions"], "randomSeed": confirmation["seed"]}}
                for request in selected
            ]
            _, leader = compare(args.engine, args.engine_sha256, fresh, paired=True, cache=cache)["results"]
            confirmation.update(
                status="compared",
                leading_review=reviews[1],
                runner_up_review=reviews[0],
                iterations=leader["iterations"],
                mean_dps_gap=leader["mean_dps_delta_from_baseline"],
                paired_dps_standard_error=leader["paired_dps_standard_error"],
                paired_dps_gap_interval_approx_95=leader["paired_dps_delta_interval_approx_95"],
            )
        result["leading_dps_confirmation"] = confirmation
    if audit is not None:
        audit["fixed_wall_seconds"] = after_fixed - start
        audit["staged_wall_seconds"] = staged_wall
        fixed_rows = {row["review"]: row for row in fixed["results"]}
        audit["fixed_top_details"] = [
            {**fixed_rows[review], "selections": drafts[review - 1][0]}
            for review in audit["fixed_top_reviews"]
        ]
        result["shortlist_audit"] = audit
    for row in result["results"]:
        selections, draft = drafts[row["review"] - 1]
        checks = draft["checks"]
        if set_piece_ids is not None:
            equipment = checks["simulator_equipment_spec"]["items"]
            row["selected_set_equipped_pieces"] = sum(item.get("id") in set_piece_ids for item in equipment)
        weapon_skills = checks.get("catalogue_projected_weapon_skills")
        if weapon_skills is not None:
            row["projected_weapon_skills"] = weapon_skills
        cleared = checks.get("simulator_cleared_equipment_slots")
        if cleared:
            row["cleared_equipment_slots"] = cleared
        if len(args.slot) == 1:
            row["selection"] = selections[0] if selections else None
        else:
            row["selections"] = selections
    metric = {"death": "simulated_deaths", "oom": "mean_seconds_oom"}.get(args.rank_by, f"mean_{args.rank_by}")
    if args.rank_by != "dps":
        result["ranked_by"] = metric
    result["results"].sort(key=lambda row: row[metric], reverse=args.rank_by in ("dps", "tps", "hps"))
    if tank_ranking:
        result["tank_tradeoff_reviews"] = tradeoff_reviews(
            result["results"], lambda row: (row["simulated_deaths"], row["mean_dtps"], -row["mean_tps"]))
    if healing_ranking:
        result["healing_tradeoff_reviews"] = tradeoff_reviews(
            result["results"], lambda row: (row["mean_seconds_oom"], -row["mean_hps"]))
    if finalist_reviews is not None:
        for row in result["results"]:
            row["review"] = finalist_reviews[row["review"] - 1]
        for field in ("observed_leading_dps_pair", "leading_dps_confirmation"):
            pair = result.get(field, {})
            for key in ("leading_review", "runner_up_review"):
                if key in pair:
                    pair[key] = finalist_reviews[pair[key] - 1]
    return result


def rank_encounters(args, template, encounters, keys, drafts, set_piece_ids, finalist_reviews=None):
    rankings = []
    simulated = {}
    for index, (encounter, key) in enumerate(zip(encounters, keys, strict=True), 1):
        previous = simulated.get(key)
        if previous is None:
            ranking = rank_encounter(args, template, encounter, drafts, set_piece_ids, finalist_reviews)
            simulated[key] = index
        else:
            ranking = {**rankings[previous - 1], "reused_encounter_index": previous}
        ranking["encounter_index"] = index
        rankings.append(ranking)
    return rankings


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
    first_rows = {row["review"]: row for row in rankings[0]["results"]}
    for row in results:
        first = first_rows[row["review"]]
        for field in ("selected_set_equipped_pieces", "projected_weapon_skills"):
            if field in first:
                row[field] = first[field]
    results.sort(key=lambda row: (-row["weighted_dps_gain_pct"], row["review"]))
    return {"encounter_weights": weights, "results": results}


def encounter_sensitivity(rankings, review_count):
    """Describe observed DPS differences without treating shortlist omissions as losses."""
    observations = [{} for _ in range(review_count)]
    leaders = [[] for _ in range(review_count)]
    observed = [[] for _ in range(review_count)]
    for index, ranking in enumerate(rankings, 1):
        rows = ranking["results"]
        baseline = next(row["mean_dps"] for row in rows if row["review"] == ranking["baseline_review"])
        best = max(row["mean_dps"] for row in rows)
        for row in rows:
            review = row["review"] - 1
            observed[review].append(index)
            if row["mean_dps"] == best:
                leaders[review].append(index)
            if baseline > 0:
                gain = 100 * ((row["mean_dps"] - baseline) / baseline)
                if isfinite(gain):
                    observations[review][index] = gain
    results = []
    for review in range(review_count):
        gains = observations[review]
        values = list(gains.values())
        minimum, maximum = (min(values), max(values)) if values else (None, None)
        results.append({
            "review": review + 1,
            "gain_pct_by_encounter": [gains.get(index) for index in range(1, len(rankings) + 1)],
            "missing_encounters": [index for index in range(1, len(rankings) + 1)
                                   if index not in observed[review]],
            "fully_compared": len(gains) == len(rankings),
            "minimum_observed_gain_pct": minimum,
            "maximum_observed_gain_pct": maximum,
            "changes_sign_in_observed_encounters": minimum < 0 < maximum if len(values) >= 2 else None,
            "leading_point_estimate_encounters": leaders[review],
        })
    return {"basis": "dps_point_estimates", "encounter_count": len(rankings), "results": results}


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
    parser.add_argument("--storage", choices=("equipment", "bags", "character_bank", "account_bank"),
                        help="Restrict alternatives to one captured storage section.")
    parser.add_argument("--minimum-required-level", type=int, help="Inclusive minimum recorded equip level.")
    parser.add_argument("--maximum-required-level", type=int, help="Inclusive maximum recorded equip level.")
    parser.add_argument("--observed-enchant-effect-id", type=int, help="Match an exact captured enchant effect ID.")
    parser.add_argument("--observed-random-suffix-id", type=int, help="Match an exact captured signed suffix ID.")
    swap_limit = parser.add_mutually_exclusive_group()
    swap_limit.add_argument("--single-swaps", action="store_const", const=1, dest="max_swaps",
                            help="Compare one selected replacement at a time across the requested slots.")
    swap_limit.add_argument("--max-swaps", type=int, choices=range(1, len(SIMULATOR_SLOTS) + 1),
                            metavar="1-17",
                            help="Compare up to this many selected replacements across the requested slots.")
    parser.add_argument("--max-drafts", type=int, default=64,
                        help="Maximum proposed changes (1–128, default 64).")
    parser.add_argument("--plan-only", action="store_true",
                        help="List resolved equipment drafts without launching a simulator.")
    parser.add_argument("--engine", type=Path)
    parser.add_argument("--engine-sha256")
    parser.add_argument("--template", type=Path, required=True)
    parser.add_argument("--encounter-lookup", type=Path, action="append",
                        help="Encounter export. Repeat for separate rankings.")
    parser.add_argument("--encounter-weight", type=float, action="append",
                        help="Relative weight in encounter input order. Bulk mode cross-tests the combined shortlist.")
    parser.add_argument("--paired", action="store_true")
    parser.add_argument("--cache", action="store_true", help="Reuse exact simulation results in this OS user's local cache")
    parser.add_argument("--confirm-leading-seed", type=int,
                        help="Rerun the top two distinct DPS sets with this fresh seed.")
    parser.add_argument("--rank-by", choices=("dps", "dtps", "tps", "death", "hps", "oom"), default="dps",
                        help="Rank by damage dealt, damage taken, threat, deaths, healing or time out of mana")
    parser.add_argument("--bulk-engine", type=Path, help="Trusted local Forever bulk adapter")
    parser.add_argument("--bulk-engine-sha256")
    parser.add_argument("--top", type=int, default=5, help="Distinct gear sets to return in bulk mode")
    parser.add_argument("--audit-shortlist", action="store_true",
                        help="Compare the staged shortlist with a bounded fixed-iteration run of every candidate")
    args = parser.parse_args()
    minimum, maximum = args.minimum_required_level, args.maximum_required_level
    if (any(value is not None and not 0 <= value <= 2**32 - 1 for value in (minimum, maximum))
            or minimum is not None and maximum is not None and minimum > maximum):
        parser.error("Provide ordered non-negative required-level bounds")
    enchant, suffix = args.observed_enchant_effect_id, args.observed_random_suffix_id
    if enchant is not None and not 1 <= enchant <= 2**32 - 1:
        parser.error("Provide a positive enchant effect ID")
    if suffix is not None and not -(2**31) <= suffix < 2**31:
        parser.error("Provide a signed 32-bit suffix ID")
    candidate_filters = {
        field: value for field, value in {
            "minimum_required_level": minimum,
            "maximum_required_level": maximum,
            "observed_enchant_effect_id": enchant,
            "observed_random_suffix_id": suffix,
        }.items() if value is not None
    }
    if args.plan_only:
        if (args.engine or args.engine_sha256 or args.encounter_lookup or args.bulk_engine
                or args.bulk_engine_sha256 or args.paired or args.audit_shortlist
                or args.encounter_weight or args.rank_by != "dps"
                or args.confirm_leading_seed is not None or args.cache):
            parser.error("--plan-only takes no simulator or encounter options")
    elif not (args.engine and args.engine_sha256 and args.encounter_lookup):
        parser.error("--engine, --engine-sha256 and --encounter-lookup are required for simulation")
    if args.confirm_leading_seed is not None and (
        not 1 <= args.confirm_leading_seed <= 2**32 - 1
        or not args.paired or args.rank_by != "dps"
    ):
        parser.error("Confirmation requires a positive 32-bit seed and paired DPS mode")
    try:
        args.simulation_cache = SimulationCache.for_current_user() if args.cache else None
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
        if args.audit_shortlist and not args.bulk_engine:
            raise ValueError("Shortlist audit requires the bulk adapter")
        if args.rank_by != "dps" and (args.bulk_engine or args.encounter_weight is not None):
            raise ValueError("Non-DPS ranking requires fixed, unweighted encounter comparisons")
        if not 1 <= args.max_drafts <= 128:
            raise ValueError("Draft limit is out of range")
        if len(args.encounter_lookup or []) > 4:
            raise ValueError("Provide up to four encounter exports")
        weights = args.encounter_weight
        if weights is not None:
            if (len(weights) < 2 or len(weights) != len(args.encounter_lookup)
                    or any(not isfinite(weight) or weight <= 0 for weight in weights)):
                raise ValueError("Provide one positive finite weight per encounter")
            largest = max(weights)
            scaled = [weight / largest for weight in weights]
            total = fsum(scaled)
            weights = [weight / total for weight in scaled]
            if any(weight == 0 for weight in weights):
                raise ValueError("Encounter weights are too far apart to normalise")
        template = read_object(args.template, 4 * 1024 * 1024)
        check_consumable_groups(template)
        can_dual_wield = player_spec(single_player(template["raid"])) in DUAL_WIELD_SPECS
        encounters = [read_object(path, 4 * 1024 * 1024) for path in args.encounter_lookup or []]
        logging.disable(logging.CRITICAL)
        with open(os.devnull, "w") as errlog:
            drafts, skipped, coverage, stats, selected_capture, set_observation, search = asyncio.run(asyncio.wait_for(
                collect(args.mcp, args.snapshot_id, args.slot, errlog,
                        args.collapse_identical_items, args.item_id, can_dual_wield,
                        args.character, args.set_id, args.max_drafts, args.storage,
                        args.max_swaps, candidate_filters), timeout=45,
            ))
        identity = check_template_identity(template, stats)
        set_piece_ids = ({piece["item_id"] for piece in set_observation["catalogue_pieces"]}
                         if set_observation is not None else None)
        if args.confirm_leading_seed is not None:
            for encounter in encounters:
                options = request_from(template, drafts[0][1], encounter)["simOptions"]
                iterations = options["iterations"]
                # The engine advances the seed once per iteration.
                gap = int(options["randomSeed"]) - args.confirm_leading_seed
                # Adaptive selection can use more samples than the requested fixed count.
                if args.bulk_engine and gap < iterations:
                    parser.exit(1, "Adaptive confirmation seed must precede each selection seed by at least the requested iterations.\n")
                if not 200 <= iterations <= 5000 or abs(gap) < iterations:
                    parser.exit(1, "Confirmation requires 200–5,000 iterations and non-overlapping seed ranges.\n")
        if args.plan_only:
            result = {
                "mode": "plan_only",
                "baseline_review": 1,
                "resolved_drafts": [
                    {"review": index, "selections": selections,
                     "cleared_equipment_slots": draft["checks"].get("simulator_cleared_equipment_slots", [])}
                    for index, (selections, draft) in enumerate(drafts, 1)
                ],
            }
        elif len(encounters) == 1:
            result = rank_encounter(args, template, encounters[0], drafts, set_piece_ids)
        else:
            # Validate every export before running any simulations. Within this command
            # only the encounter and simulator options vary across profiles.
            keys = []
            for encounter in encounters:
                request = request_from(template, drafts[0][1], encounter)
                keys.append(encounter_key(request))
            rankings = rank_encounters(args, template, encounters, keys, drafts, set_piece_ids)
            result = {}
            if weights is not None and args.bulk_engine:
                finalist_reviews = sorted({row["review"] for ranking in rankings for row in ranking["results"]})
                result.update(screening_encounters=rankings, finalist_reviews=finalist_reviews)
                rankings = rank_encounters(
                    args, template, encounters, keys, drafts, set_piece_ids, finalist_reviews)
            result["encounters"] = rankings
            if args.rank_by == "dps":
                result["encounter_sensitivity"] = encounter_sensitivity(rankings, len(drafts))
            if weights is not None:
                result["weighted_ranking"] = weighted_ranking(rankings, weights)
        result.update(skipped=skipped, storage_coverage=coverage, template_identity=identity,
                      gear_search=search)
        if args.storage is not None:
            result["requested_storage"] = args.storage
        if minimum is not None or maximum is not None:
            result["requested_required_levels"] = {"minimum": minimum, "maximum": maximum}
        if enchant is not None or suffix is not None:
            result["requested_observed_modifiers"] = {"enchant_effect_id": enchant, "random_suffix_id": suffix}
        if args.max_swaps is not None:
            result["max_swaps"] = args.max_swaps
        if args.max_swaps == 1:
            result["single_swaps"] = True
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
    except ConflictingConsumables:
        parser.exit(1, "Forever template selects multiple items in the same simulator buff group.\n")
    except Exception as error:
        # The stdio client's task groups wrap errors raised during collection.
        while isinstance(error, ExceptionGroup) and len(error.exceptions) == 1:
            error = error.exceptions[0]
        if isinstance(error, GearSearchLimit):
            parser.exit(1, f"{error}\n")
        # MCP and simulator failures may embed paths or private response text.
        parser.exit(1, f"Forever slot ranking failed ({type(error).__name__}). Check inputs and budgets.\n")


if __name__ == "__main__":
    main()
