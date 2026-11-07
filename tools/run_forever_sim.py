"""Run the pinned local simulator with an explicit single-player template and MCP exports."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess
import tempfile

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
    raid = template["raid"]
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


def run(engine, expected_hash, request):
    if not engine.is_absolute() or not engine.is_file():
        raise ValueError("Provide an absolute path to the trusted local engine")
    with engine.open("rb") as handle:
        if hashlib.file_digest(handle, "sha256").hexdigest() != expected_hash.lower():
            raise ValueError("Engine SHA-256 mismatch")
    with tempfile.TemporaryDirectory(prefix="forever-sim-") as folder:
        root = Path(folder)
        source = root / "request.json"
        result_path = root / "result.json"
        source.write_text(json.dumps(request, allow_nan=False), encoding="utf-8")
        subprocess.run(
            [str(engine), "sim", "--infile", str(source), "--outfile", str(result_path)],
            cwd=root, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL, check=True, timeout=120,
        )
        result = read_object(result_path, 16 * 1024 * 1024)
        if result.get("error") or result.get("iterationsDone") != request["simOptions"]["iterations"]:
            raise ValueError("Simulator reported a failed run")
        dps = result["raidMetrics"]["dps"]
        if not all(math.isfinite(dps[key]) for key in ("avg", "stdev")):
            raise ValueError("Simulator returned invalid DPS metrics")
        return {
            "iterations": request["simOptions"]["iterations"],
            "seed": request["simOptions"]["randomSeed"],
            "mean_dps": dps["avg"],
            "dps_standard_deviation": dps["stdev"],
            "engine_sha256": expected_hash.lower(),
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--engine-sha256", required=True)
    parser.add_argument("--template", type=Path, required=True)
    parser.add_argument("--equipment-review", type=Path, required=True)
    parser.add_argument("--encounter-lookup", type=Path, required=True)
    args = parser.parse_args()
    try:
        request = request_from(*(read_object(path, 4 * 1024 * 1024) for path in (
            args.template, args.equipment_review, args.encounter_lookup,
        )))
        print(json.dumps(run(args.engine, args.engine_sha256, request), allow_nan=False))
    except (OSError, ValueError, TypeError, AttributeError, KeyError, RecursionError, subprocess.SubprocessError) as error:
        parser.exit(1, f"Forever simulation failed ({type(error).__name__}). Check inputs and engine.\n")


if __name__ == "__main__":
    main()
