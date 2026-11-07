"""Run the pinned local simulator with an explicit single-player template and MCP exports."""

import argparse
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess
import tempfile
import time

SPECS = frozenset((
    "balanceDruid", "feralCatDruid", "feralBearDruid", "restorationDruid",
    "hunter", "mage", "holyPaladin", "protectionPaladin", "retributionPaladin",
    "dpsPriest", "healerPriest", "rogue", "elementalShaman", "enhancementShaman",
    "restorationShaman", "warlock", "dpsWarrior", "protectionWarrior",
))


def read_object(path, limit):
    with path.open("rb") as handle:
        data = handle.read(limit + 1)
    if len(data) > limit:
        raise ValueError("JSON input exceeds size limit")
    value = json.loads(data)
    if not isinstance(value, dict):
        raise ValueError("Expected a JSON object")
    return value


def request_from(template, equipment_review, encounter_lookup):
    raid = deepcopy(template["raid"])
    players = [player for party in raid["parties"] for player in party.get("players", [])]
    if len(players) != 1:
        raise ValueError("Provide exactly one player")
    player = players[0]
    rotation = player["rotation"]
    if rotation.get("type") != "TypeAPL" or not any(
        entry.get("action") for entry in rotation.get("priorityList", [])
    ):
        raise ValueError("Provide a single-player template with an explicit APL")
    specs = SPECS & player.keys()
    if (len(specs) != 1 or not isinstance(player[next(iter(specs))], dict)
            or not isinstance(player.get("talentsString"), str)
            or not player.get("class") or player["class"] == "ClassUnknown"
            or not player.get("race") or player["race"] == "RaceUnknown"):
        raise ValueError("Provide an explicit class, race, spec and talent string")
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


def verify_engine(engine, expected_hash):
    if not engine.is_absolute() or not engine.is_file():
        raise ValueError("Provide an absolute path to the trusted local engine")
    with engine.open("rb") as handle:
        if hashlib.file_digest(handle, "sha256").hexdigest() != expected_hash.lower():
            raise ValueError("Engine SHA-256 mismatch")


def run(engine, expected_hash, request, deadline=None, paired=False):
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
        if paired:
            values = dps["allValues"]
            if (not isinstance(values, list) or len(values) != summary["iterations"]
                    or not all(math.isfinite(value) for value in values)):
                raise ValueError("Simulator returned invalid paired samples")
            summary["_dps_samples"] = values
        return summary


def compare(engine, expected_hash, requests, paired=False):
    keys = [json.dumps(request, sort_keys=True, separators=(",", ":"), allow_nan=False)
            for request in requests]
    unique = dict(zip(keys, requests))
    iterations = [request["simOptions"]["iterations"] for request in unique.values()]
    if sum(iterations) > 100_000:
        raise ValueError("Comparison iteration budget exceeded")
    if paired and (not 2 <= iterations[0] <= 5000
                   or any(request["simOptions"] != requests[0]["simOptions"] for request in unique.values())):
        raise ValueError("Paired comparisons require one seed and 2 to 5000 matching iterations")
    deadline = time.monotonic() + 120
    results = {}
    for key, request in unique.items():
        results[key] = run(engine, expected_hash, request, deadline=deadline, paired=paired)
        if time.monotonic() >= deadline:
            raise ValueError("Comparison deadline exceeded")
    baseline = results[keys[0]]["mean_dps"]
    paired_errors = {}
    if paired:
        baseline_samples = results[keys[0]]["_dps_samples"]
        for key, result in results.items():
            samples = result.pop("_dps_samples")
            differences = (value - reference for value, reference in zip(samples, baseline_samples, strict=True))
            paired_errors[key] = statistics.stdev(differences) / math.sqrt(len(samples))
    if time.monotonic() >= deadline:
        raise ValueError("Comparison deadline exceeded")
    return {
        "unique_runs": len(results),
        "baseline_review": 1,
        "results": [
            {"review": index, **results[key],
             "mean_dps_delta_from_baseline": results[key]["mean_dps"] - baseline,
             **({"paired_dps_standard_error": paired_errors[key]} if paired else {})}
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
    parser.add_argument("--paired", action="store_true", help="Report paired DPS standard errors for a comparison")
    args = parser.parse_args()
    try:
        if len(args.equipment_review) > 8:
            raise ValueError("Use at most eight equipment reviews")
        if args.paired and len(args.equipment_review) < 2:
            raise ValueError("Paired mode requires multiple equipment reviews")
        limit = 4 * 1024 * 1024
        template = read_object(args.template, limit)
        encounter = read_object(args.encounter_lookup, limit)
        requests = [request_from(template, read_object(path, limit), encounter)
                    for path in args.equipment_review]
        result = (run(args.engine, args.engine_sha256, requests[0]) if len(requests) == 1
                  else compare(args.engine, args.engine_sha256, requests, paired=args.paired))
        print(json.dumps(result, allow_nan=False))
    except (OSError, ValueError, TypeError, AttributeError, KeyError, RecursionError, subprocess.SubprocessError) as error:
        parser.exit(1, f"Forever simulation failed ({type(error).__name__}). Check inputs and engine.\n")


if __name__ == "__main__":
    main()
