"""Run the pinned local simulator with an explicit single-player template and MCP exports."""

import argparse
from copy import deepcopy
from decimal import Decimal, InvalidOperation
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat
import statistics
import subprocess
import tempfile
import time

if __package__:
    from .sync_exports import ordinary
    from .sync_forever_catalogue import MAX_BYTES, verify
else:
    from sync_exports import ordinary
    from sync_forever_catalogue import MAX_BYTES, verify

# Class enum IDs from the pinned Forever common.proto.
CLASS_IDS = {
    "warrior": 1, "paladin": 2, "hunter": 3, "rogue": 4, "priest": 5,
    "shaman": 7, "mage": 8, "warlock": 9, "druid": 11,
}
SPEC_CLASSES = {
    "balanceDruid": "druid", "feralCatDruid": "druid", "feralBearDruid": "druid",
    "restorationDruid": "druid", "hunter": "hunter", "mage": "mage",
    "holyPaladin": "paladin", "protectionPaladin": "paladin", "retributionPaladin": "paladin",
    "dpsPriest": "priest", "healerPriest": "priest", "rogue": "rogue",
    "elementalShaman": "shaman", "enhancementShaman": "shaman", "restorationShaman": "shaman",
    "warlock": "warlock", "dpsWarrior": "warrior", "protectionWarrior": "warrior",
}

CONSUMABLE_FIELDS = (
    "resource_pot_id", "defensive_pot_id", "offensive_pot_id", "flask_id",
    "food_id", "conjured_id", "explosive_id", "ohImbue_id", "oh_weapon_imbue_id",
    "mhImbue_id", "mh_weapon_imbue_id", "swap_oh_imbue_id",
    "swap_oh_weapon_imbue_id", "swap_mh_imbue_id", "swap_mh_weapon_imbue_id",
)
CONSUMABLE_LIST_FIELDS = ("elixir_ids", "extra_ids", "potions", "conjured_items")
JSON_NUMBER = re.compile(r"-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?")


class ConflictingConsumables(ValueError):
    pass


def check_consumable_groups(template):
    cache = os.environ.get("AZEROTH_FOREVER_CATALOGUE")
    consumes = single_player(template["raid"]).get("consumables")
    if not cache or consumes is None:
        return
    if not isinstance(consumes, dict):
        raise ValueError("Expected consumable selections as an object")
    selected = set()
    for field in (*CONSUMABLE_FIELDS, *CONSUMABLE_LIST_FIELDS):
        first, *rest = field.split("_")
        camel = first + "".join(part[0].upper() + part[1:] for part in rest)
        for name in dict.fromkeys((field, camel)):
            value = consumes.get(name)
            if value is None:
                continue
            values = value if field in CONSUMABLE_LIST_FIELDS else [value]
            if not isinstance(values, list):
                raise ValueError("Expected a list of consumable IDs")
            for item in values:
                if isinstance(item, str) and JSON_NUMBER.fullmatch(item):
                    try:
                        item = Decimal(item)
                    except InvalidOperation:
                        raise ValueError("Invalid consumable ID") from None
                if (type(item) not in (int, float, Decimal) or not 0 <= item <= 2**31 - 1
                        or item != int(item)):
                    raise ValueError("Expected a nonnegative integer consumable ID")
                if item:
                    selected.add(int(item))
    if len(selected) < 2:
        return
    path = ordinary(Path(cache).absolute())
    if not stat.S_ISREG(path.stat().st_mode):
        raise ValueError("Forever catalogue must be a regular file")
    with path.open("rb") as stream:
        data = stream.read(MAX_BYTES + 1)
    verify(data)
    seen = set()
    for row in json.loads(data)["consumables"]:
        group = row.get("groupKey")
        if row["id"] in selected and group:
            if group in seen:
                raise ConflictingConsumables
            seen.add(group)


def read_object(path, limit):
    with path.open("rb") as handle:
        data = handle.read(limit + 1)
    if len(data) > limit:
        raise ValueError("JSON input exceeds size limit")
    value = json.loads(data)
    if not isinstance(value, dict):
        raise ValueError("Expected a JSON object")
    return value


def single_player(raid):
    players = [player for party in raid["parties"] for player in party.get("players", [])]
    if len(players) != 1:
        raise ValueError("Provide exactly one player")
    return players[0]


def healing_dummy_count(raid):
    dummy_count = raid.get("targetDummies", raid.get("target_dummies", 0))
    if type(dummy_count) is not int or not 0 <= dummy_count <= 24:
        raise ValueError("Provide zero to 24 simulated healing dummies")
    if dummy_count:
        active_count = raid.get("numActiveParties", raid.get("num_active_parties", 0))
        if type(active_count) is not int or active_count < 0:
            raise ValueError("Invalid active party count")
        parties = raid["parties"]
        active = parties[:active_count or len(parties)]
        if dummy_count > 5 * len(active) - 1 or not any(party.get("players") for party in active):
            raise ValueError("Provide an active player and enough party slots for healing dummies")
    return dummy_count


def simulated_player_metrics(request_raid, metrics_raid):
    if healing_dummy_count(request_raid) == 0:
        return single_player(metrics_raid)
    name = single_player(request_raid).get("name")
    if not name:
        raise ValueError("Healing dummies require a named simulator player")
    matches = [
        player for party in metrics_raid["parties"] for player in party.get("players", [])
        if player.get("name") == name
    ]
    if len(matches) != 1:
        raise ValueError("Could not identify the simulated player's metrics among healing dummies")
    return matches[0]


def player_spec(player):
    specs = SPEC_CLASSES.keys() & player.keys()
    if len(specs) != 1:
        raise ValueError("Provide exactly one supported specialisation")
    spec = next(iter(specs))
    if not isinstance(player[spec], dict):
        raise ValueError("Provide an explicit specialisation configuration")
    return spec


def request_from(template, equipment_review, encounter_lookup):
    raid = deepcopy(template["raid"])
    player = single_player(raid)
    healing_dummy_count(raid)
    rotation = player["rotation"]
    if rotation.get("type") != "TypeAPL" or not any(
        entry.get("action") for entry in rotation.get("priorityList", [])
    ):
        raise ValueError("Provide a single-player template with an explicit APL")
    spec = player_spec(player)
    if (not isinstance(player.get("talentsString"), str)
            or not player.get("class") or player["class"] == "ClassUnknown"
            or not player.get("race") or player["race"] == "RaceUnknown"):
        raise ValueError("Provide an explicit class, race, spec and talent string")
    spec_class = SPEC_CLASSES[spec]
    declared_class = player["class"]
    if not (declared_class == f"Class{spec_class.capitalize()}"
            or type(declared_class) is int and declared_class == CLASS_IDS[spec_class]):
        raise ValueError("Simulator template class and specialisation disagree")
    if player.get("enableItemSwap") or player.get("enable_item_swap"):
        raise ValueError("Disable template item swapping")
    equipment = equipment_review["checks"]["simulator_equipment_spec"]
    if equipment_review["client"]["family"] != "forever" or not equipment or len(equipment["items"]) != 17:
        raise ValueError("Provide a resolved Forever equipment review")
    profile = encounter_lookup["simulation_input"]
    if encounter_lookup["family"] != "forever" or encounter_lookup["section"] != "encounters":
        raise ValueError("Provide a Forever encounter lookup with simulation settings")
    options = profile["simOptions"]
    if type(options["iterations"]) is not int or not 1 <= options["iterations"] <= 100_000:
        raise ValueError("Iteration count out of range")
    seed = options["randomSeed"]
    if not isinstance(seed, str) or not seed.isascii() or not seed.isdecimal() or not 1 <= int(seed) <= 2**32 - 1:
        raise ValueError("Seed out of range")
    encounter = profile["encounter"]
    duration, variation = encounter["duration"], encounter["durationVariation"]
    if not (1 <= duration <= 3600 and 0 <= variation < duration
            and 0 <= encounter["executeProportion20"] <= encounter["executeProportion35"] <= 1
            and isinstance(encounter["targets"], list) and encounter["targets"]):
        raise ValueError("Provide a bounded encounter with explicit targets")
    player["equipment"] = equipment
    player["name"] = "Player"
    return {
        "raid": raid,
        "encounter": encounter,
        "simOptions": {"iterations": options["iterations"], "randomSeed": seed},
        "type": "SimTypeIndividual",
    }


def encounter_key(request):
    return json.dumps(
        (request["encounter"], request["simOptions"]),
        sort_keys=True, separators=(",", ":"), allow_nan=False,
    )


def encounter_summary(request):
    profile = request["encounter"]
    return {
        "encounter_settings_sha256": hashlib.sha256(encounter_key(request).encode()).hexdigest(),
        "duration_seconds": profile["duration"],
        "duration_variation_seconds": profile["durationVariation"],
        "target_count": len(profile["targets"]),
        "execute_proportion_20": profile["executeProportion20"],
        "execute_proportion_35": profile["executeProportion35"],
        "requested_iterations": request["simOptions"]["iterations"],
    }


def verify_engine(engine, expected_hash):
    if not engine.is_absolute() or not engine.is_file():
        raise ValueError("Provide an absolute path to the trusted local engine")
    with engine.open("rb") as handle:
        if hashlib.file_digest(handle, "sha256").hexdigest() != expected_hash.lower():
            raise ValueError("Engine SHA-256 mismatch")


def run(engine, expected_hash, request, deadline=None, paired=False, tank_metrics=False, healing_metrics=False):
    if tank_metrics:
        raid = request["raid"]
        first_party = raid["parties"][0].get("players", [])
        tanks = raid.get("tanks", [])
        if (not first_party or len(tanks) != 1 or tanks[0].get("type") not in ("Player", 1)
                or tanks[0].get("index", 0) != 0
                or not any(target.get("tankIndex", target.get("tank_index", 0)) == 0
                           for target in request["encounter"]["targets"])):
            raise ValueError("Tank metrics require the first player as the encounter tank")
    verify_engine(engine, expected_hash)
    with tempfile.TemporaryDirectory(prefix="forever-sim-") as folder:
        root = Path(folder)
        source = root / "request.json"
        result_path = root / "result.json"
        if paired:
            request = {**request, "simOptions": {**request["simOptions"], "saveAllValues": True}}
        source.write_text(json.dumps(request, allow_nan=False), encoding="utf-8")
        timeout = 120 if deadline is None else deadline - time.monotonic()
        if timeout <= 0:
            raise ValueError("Comparison deadline exceeded")
        subprocess.run(
            [str(engine), "sim", "--infile", str(source), "--outfile", str(result_path)],
            cwd=root, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL, check=True, timeout=timeout,
        )
        result = read_object(result_path, 16 * 1024 * 1024)
        if result.get("error") or result.get("iterationsDone") != request["simOptions"]["iterations"]:
            raise ValueError("Simulator reported a failed run")
        dps = result["raidMetrics"]["dps"]
        if not all(math.isfinite(dps[key]) for key in ("avg", "stdev")):
            raise ValueError("Simulator returned invalid DPS metrics")
        summary = {
            "iterations": request["simOptions"]["iterations"],
            "seed": request["simOptions"]["randomSeed"],
            "mean_dps": dps["avg"],
            "dps_standard_deviation": dps["stdev"],
            "engine_sha256": expected_hash.lower(),
        }
        paired_distributions = {"dps": dps} if paired else {}
        if tank_metrics or healing_metrics:
            metrics = simulated_player_metrics(request["raid"], result["raidMetrics"])
        if tank_metrics:
            for output, metric in (("dtps", "dtps"), ("tps", "threat")):
                mean, deviation = metrics[metric]["avg"], metrics[metric]["stdev"]
                if (not all(type(value) in (int, float) and math.isfinite(value)
                            for value in (mean, deviation))
                        or deviation < 0 or (output == "dtps" and mean < 0)):
                    raise ValueError("Simulator returned invalid tank metrics")
                summary[f"mean_{output}"] = mean
                summary[f"{output}_standard_deviation"] = deviation
                if paired:
                    paired_distributions[output] = metrics[metric]
            death_fraction = metrics["chanceOfDeath"]
            # The engine combines weighted shard fractions in floating point.
            if type(death_fraction) not in (int, float) or not 0 <= death_fraction <= 1 + 1e-9:
                raise ValueError("Simulator returned an invalid death fraction")
            summary["simulated_death_fraction"] = min(death_fraction, 1.0)
            summary["simulated_deaths"] = round(summary["simulated_death_fraction"] * summary["iterations"])
        if healing_metrics:
            mean, deviation = metrics["hps"]["avg"], metrics["hps"]["stdev"]
            oom = metrics["secondsOomAvg"]
            if not all(type(value) in (int, float) and math.isfinite(value) and value >= 0
                       for value in (mean, deviation, oom)):
                raise ValueError("Simulator returned invalid healing metrics")
            summary.update(mean_hps=mean, hps_standard_deviation=deviation, mean_seconds_oom=oom)
            if paired:
                paired_distributions["hps"] = metrics["hps"]
        for metric, distribution in paired_distributions.items():
            values = distribution["allValues"]
            if (not isinstance(values, list) or len(values) != summary["iterations"]
                    or not all(type(value) in (int, float) and math.isfinite(value) for value in values)):
                raise ValueError("Simulator returned invalid paired samples")
            summary[f"_{metric}_samples"] = values
        return summary


def compare(engine, expected_hash, requests, paired=False, tank_metrics=False, healing_metrics=False):
    keys = [json.dumps(request, sort_keys=True, separators=(",", ":"), allow_nan=False)
            for request in requests]
    unique = dict(zip(keys, requests))
    iterations = [request["simOptions"]["iterations"] for request in unique.values()]
    if sum(iterations) > 100_000:
        raise ValueError("Comparison iteration budget exceeded")
    if paired and (not 2 <= iterations[0] <= 5000 or any(
        request["simOptions"] != requests[0]["simOptions"] or request["encounter"] != requests[0]["encounter"]
        for request in unique.values()
    )):
        raise ValueError("Paired comparisons require one encounter, one seed and 2 to 5000 matching iterations")
    deadline = time.monotonic() + 120
    results = {}
    for key, request in unique.items():
        results[key] = run(
            engine, expected_hash, request, deadline=deadline, paired=paired,
            tank_metrics=tank_metrics, healing_metrics=healing_metrics,
        )
        if time.monotonic() >= deadline:
            raise ValueError("Comparison deadline exceeded")
    baseline = results[keys[0]]["mean_dps"]
    if tank_metrics:
        for row in results.values():
            for field in ("mean_dtps", "mean_tps", "simulated_death_fraction"):
                row[f"{field}_delta_from_baseline"] = row[field] - results[keys[0]][field]
    if healing_metrics:
        for row in results.values():
            for field in ("mean_hps", "mean_seconds_oom"):
                row[f"{field}_delta_from_baseline"] = row[field] - results[keys[0]][field]
    if paired:
        metrics = ["dps"]
        if tank_metrics:
            metrics.extend(("dtps", "tps"))
        if healing_metrics:
            metrics.append("hps")
        for metric in metrics:
            field = f"_{metric}_samples"
            baseline_samples = results[keys[0]][field]
            for result in results.values():
                samples = result.pop(field)
                differences = (value - reference for value, reference in zip(samples, baseline_samples, strict=True))
                result[f"paired_{metric}_standard_error"] = statistics.stdev(differences) / math.sqrt(len(samples))
    if time.monotonic() >= deadline:
        raise ValueError("Comparison deadline exceeded")
    return {
        "unique_runs": len(results),
        "baseline_review": 1,
        "results": [
            {"review": index, **results[key],
             "mean_dps_delta_from_baseline": results[key]["mean_dps"] - baseline}
            for index, key in enumerate(keys, 1)
        ],
    }


def compare_bulk(engine, expected_hash, requests, top_results):
    verify_engine(engine, expected_hash)
    baseline = requests[0]
    candidates = []
    reviews = {}
    seen = {}
    for review, request in enumerate(requests, 1):
        if request["simOptions"] != baseline["simOptions"]:
            raise ValueError("Bulk comparisons require matching simulation settings")
        first_party = request["raid"]["parties"][0].get("players", [])
        if not first_party:
            raise ValueError("Bulk comparisons require the player in the first party")
        equipment = first_party[0]["equipment"]
        key = json.dumps(equipment, sort_keys=True, separators=(",", ":"), allow_nan=False)
        index = seen.setdefault(key, review)
        reviews.setdefault(index, []).append(review)
        if index == review and review != 1:
            candidates.append({"index": index, "gear": equipment})
    if not candidates:
        return None
    payload = {"baseRequest": baseline, "candidates": candidates,
               "topResults": min(top_results, len(candidates))}
    with tempfile.TemporaryDirectory(prefix="forever-bulk-") as folder:
        output = Path(folder) / "result.json"
        with output.open("wb") as stream:
            subprocess.run([str(engine)], input=json.dumps(payload, allow_nan=False).encode(),
                           stdout=stream, stderr=subprocess.DEVNULL, cwd=folder,
                           timeout=120, check=True)
        result = read_object(output, 1024 * 1024)
    baseline_row = result["baseline"]
    if baseline_row["candidateIndex"] != -1:
        raise ValueError("Invalid bulk baseline")
    returned = [row["candidateIndex"] for row in result["topResults"]]
    if (not 1 <= len(returned) <= payload["topResults"] or len(returned) != len(set(returned))
            or any(index == 1 or index not in reviews for index in returned)):
        raise ValueError("Invalid bulk shortlist")
    rows = []
    baseline_dps = baseline_row["dpsMetrics"]["avg"]
    for index, row in zip([1, *returned], [baseline_row, *result["topResults"]], strict=True):
        dps = row["dpsMetrics"]
        paired_error = row["pairedErrorToBaseline"]
        if any(type(value) not in (int, float) or not math.isfinite(value) or value < 0
               for value in (dps["avg"], dps["stdev"], paired_error)):
            raise ValueError("Bulk simulation returned invalid metrics")
        rows.extend({
            "review": review,
            "mean_dps": dps["avg"],
            "dps_standard_deviation": dps["stdev"],
            "mean_dps_delta_from_baseline": dps["avg"] - baseline_dps,
            "paired_dps_standard_error": paired_error if index != 1 and paired_error else None,
        } for review in reviews[index])
    return {"mode": "staged_bulk", "baseline_review": 1, "results": rows,
            "engine_sha256": expected_hash.lower(), "seed": baseline["simOptions"]["randomSeed"],
            "unique_gear_sets": len(reviews), "stage_metrics": result["stageMetrics"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--engine-sha256", required=True)
    parser.add_argument("--template", type=Path, required=True)
    parser.add_argument("--equipment-review", type=Path, action="append", required=True)
    parser.add_argument("--encounter-lookup", type=Path, required=True)
    parser.add_argument("--paired", action="store_true", help="Report paired standard errors for a comparison")
    parser.add_argument("--tank-metrics", action="store_true",
                        help="Report damage taken, threat per second and simulated deaths for an explicit tank")
    parser.add_argument("--healing-metrics", action="store_true",
                        help="Report simulated healing per second and time out of mana")
    args = parser.parse_args()
    try:
        if len(args.equipment_review) > 8:
            raise ValueError("Use at most eight equipment reviews")
        if args.paired and len(args.equipment_review) < 2:
            raise ValueError("Paired mode requires multiple equipment reviews")
        limit = 4 * 1024 * 1024
        template = read_object(args.template, limit)
        check_consumable_groups(template)
        encounter = read_object(args.encounter_lookup, limit)
        requests = [request_from(template, read_object(path, limit), encounter)
                    for path in args.equipment_review]
        result = (run(args.engine, args.engine_sha256, requests[0], tank_metrics=args.tank_metrics,
                      healing_metrics=args.healing_metrics)
                  if len(requests) == 1 else compare(
                      args.engine, args.engine_sha256, requests, paired=args.paired, tank_metrics=args.tank_metrics,
                      healing_metrics=args.healing_metrics))
        result.update(encounter_summary(requests[0]))
        print(json.dumps(result, allow_nan=False))
    except ConflictingConsumables:
        parser.exit(1, "Forever template selects multiple items in the same simulator buff group.\n")
    except (OSError, ValueError, TypeError, AttributeError, KeyError, RecursionError, subprocess.SubprocessError) as error:
        parser.exit(1, f"Forever simulation failed ({type(error).__name__}). Check inputs and engine.\n")


if __name__ == "__main__":
    main()
