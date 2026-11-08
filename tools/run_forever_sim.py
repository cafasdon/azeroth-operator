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
import sqlite3
import stat
import statistics
import subprocess
import tempfile
import time

if __package__:
    from .simulation_cache import SimulationCache
    from .simulation_journal import SimulationJournal
    from .sync_exports import ordinary
    from .sync_forever_catalogue import MAX_BYTES, verify
else:
    from simulation_cache import SimulationCache
    from simulation_journal import SimulationJournal
    from sync_exports import ordinary
    from sync_forever_catalogue import MAX_BYTES, verify

# Pinned 9dc32ac constants.go and NewCharacter use this fixed level.
SIMULATOR_CHARACTER_LEVEL = 60


class UnsupportedSimulationLevel(ValueError):
    """The pinned simulator cannot represent the captured character level."""


class ConflictingNativeSlotCategory(ValueError):
    """Native item categories contradict the projected equipment placement."""


# Mirrors planning::native_slot_category for legacy saved review metadata.
NATIVE_CATEGORY_SLOTS = {
    'HEAD':{1},'NECK':{2},'SHOULDER':{3},'BODY':{4},'CHEST':{5},'ROBE':{5},'WAIST':{6},
    'LEGS':{7},'FEET':{8},'WRIST':{9},'HAND':{10},'FINGER':{11,12},'TRINKET':{13,14},'CLOAK':{15},
    'WEAPONMAINHAND':{16},'WEAPONOFFHAND':{17},'SHIELD':{17},'HOLDABLE':{17},'WEAPON':{16,17},
    '2HWEAPON':{16,17},'RANGED':{18},'RANGEDRIGHT':{18},'THROWN':{18},'RELIC':{18},'TABARD':{19},
}
PROJECTED_SLOT_INDEX = {slot:index for index,slot in enumerate([1,2,3,15,5,9,10,6,7,8,11,12,13,14,16,17,18])}


def check_native_slot_categories(review, equipment):
    if review['checks'].get('native_slot_category_conflicts'):
        raise ConflictingNativeSlotCategory('Native slot-category conflicts require a refreshed equipment review before simulation')
    changes=review.get('changes',[])
    if not isinstance(changes,list):raise ValueError('Invalid equipment review changes')
    for change in changes:
        if not isinstance(change,dict):raise ValueError('Invalid equipment review change')
        target=change.get('target_slot')
        if type(target) is not int or not 1<=target<=19:raise ValueError('Invalid equipment review target')
        index=PROJECTED_SLOT_INDEX.get(target)
        if index is None:continue
        projected=equipment['items'][index]
        # Cleared or vacated slots contribute no simulated item.
        if not isinstance(projected,dict) or not projected.get('id'):continue
        annotation=change.get('native_slot_category')
        conflicting=isinstance(annotation,dict) and annotation.get('target_slot_category_matches') is False
        observed=change.get('selected_item_observed')
        location=observed.get('equip_location') if isinstance(observed,dict) else None
        token=location.removeprefix('INVTYPE_') if isinstance(location,str) and location.startswith('INVTYPE_') else None
        slots=NATIVE_CATEGORY_SLOTS.get(token)
        if slots is not None and target not in slots:conflicting=True
        if conflicting:
            raise ConflictingNativeSlotCategory('Native slot-category conflicts require a refreshed equipment review before simulation')


# Class enum IDs from the pinned Forever common.proto.
CLASS_IDS = {
    "warrior": 1, "paladin": 2, "hunter": 3, "rogue": 4, "priest": 5,
    "shaman": 7, "mage": 8, "warlock": 9, "druid": 11,
}
# Race enum IDs from the pinned Forever common.proto.
CAPTURED_RACES = {
    "bloodelf": 1, "dwarf": 2, "gnome": 3, "human": 4, "nightelf": 5,
    "orc": 6, "tauren": 7, "troll": 8, "undead": 9,
    "highorderskyborne": 10, "windshaperskyborne": 11,
}


class ConflictingCharacterIdentity(ValueError):
    """The template contradicts the captured class or race."""


def check_character_identity(player, observed):
    if observed is None:
        observed = {}
    if not isinstance(observed, dict):
        raise ValueError("Invalid captured character identity")
    checks = {}
    for field, known in (("class", CLASS_IDS), ("race", CAPTURED_RACES)):
        token = observed.get(field)
        if token is not None and not isinstance(token, str):
            raise ValueError("Invalid captured character identity")
        token = (token or "").lower()
        if field == "race" and token == "scourge":
            token = "undead"
        checks[field] = "unverified"
        if token in known:
            declared = player.get(field)
            if isinstance(declared, str):
                matches = declared.removeprefix(field.capitalize()).lower() == token
            else:
                matches = type(declared) is int and declared == known[token]
            if not matches:
                raise ConflictingCharacterIdentity(
                    "Simulator template class or race disagrees with the saved character observation. Match the template to that capture."
                )
            checks[field] = "matches"
    return checks


# Pinned common.proto Profession enum and DBC skill-line mapping, also used
# by src/forever_catalogue.rs for item profession requirements.
PROFESSIONS = {
    171: (1, "Alchemy"), 164: (2, "Blacksmithing"), 333: (3, "Enchanting"),
    202: (4, "Engineering"), 182: (5, "Herbalism"), 773: (6, "Inscription"),
    165: (8, "Leatherworking"), 186: (9, "Mining"), 393: (10, "Skinning"),
    197: (11, "Tailoring"),
}
SECONDARY_SKILL_LINES = frozenset((129, 185, 356, 794))


class ConflictingProfessions(ValueError):
    """The template uses different professions from the captured character."""


def check_profession_identity(player, observed, require_match=True):
    """Check observed profession identities only; ranks and recipes stay unverified."""
    names = {name: identifier for identifier, name in PROFESSIONS.values()}
    names["ProfessionUnknown"] = 0
    declared = []
    for field in ("profession1", "profession2"):
        value = player.get(field, 0)
        if isinstance(value, str):
            value = names.get(value)
        if type(value) is not int or value not in (0, *names.values()):
            raise ValueError("Invalid simulator template profession")
        if value:
            declared.append(value)
    if len(set(declared)) != len(declared):
        raise ValueError("Duplicate simulator template profession")
    report = {"status": "unverified", "template_profession_ids": sorted(declared),
              "ranks_and_recipes": "unverified"}
    if observed is None:
        return report
    if not isinstance(observed, dict):
        raise ValueError("Invalid captured professions observation")
    captured = []
    complete = True
    for row in observed.values():
        if not isinstance(row, dict) or type(row.get("skill_line")) is not int:
            raise ValueError("Invalid captured professions observation")
        skill = row["skill_line"]
        if skill in SECONDARY_SKILL_LINES:
            continue
        if skill not in PROFESSIONS:
            complete = False
        else:
            captured.append(PROFESSIONS[skill][0])
    if len(captured) > 2 or len(set(captured)) != len(captured):
        complete = False
    report["captured_profession_ids"] = sorted(set(captured))
    if set(captured) - set(declared) or (complete and sorted(captured) != sorted(declared)):
        if require_match:
            raise ConflictingProfessions("Simulator template professions disagree with the saved character observation; match the template to that capture or use --plan-only.")
        report["status"] = "mismatch"
        return report
    if not complete:
        return report
    report["status"] = "matches"
    return report


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


class UnsupportedSimulationPetLevel(ValueError):
    """The pinned Hunter pet model cannot represent the captured pet level."""


HUNTER_PET_TYPES = ("PetNone", "Bat", "Bear", "BirdOfPrey", "Boar", "CarrionBird",
                    "Cat", "CoreHound", "Crab", "Crocilisk", "Fox", "Gorilla", "Hyena",
                    "Raptor", "Scorpid", "Spider", "Tallstrider", "Turtle", "WindSerpent", "Wolf")


def proto_option(value, camel, snake, default):
    """Read protobuf JSON spellings without allowing a duplicate-field bypass."""
    if not isinstance(value, dict):
        raise ValueError("Invalid Hunter pet options")
    if camel in value and snake in value:
        raise ValueError("Duplicate Hunter pet option")
    selected = value.get(camel, value.get(snake))
    return default if selected is None else selected


def check_hunter_pet_level(player, observed, require_level=True):
    report = {"level": "not_applicable", "simulator_pet_level": None,
              "family_and_training": "unverified", "happiness": "unverified"}
    if player_spec(player) != "hunter":
        return report
    options = player["hunter"].get("options")
    options = {} if options is None else options
    class_options = proto_option(options, "classOptions", "class_options", {})
    pet_type = proto_option(class_options, "petType", "pet_type", 0)
    if isinstance(pet_type, str):
        pet_type = HUNTER_PET_TYPES.index(pet_type) if pet_type in HUNTER_PET_TYPES else None
    if type(pet_type) is not int or not 0 <= pet_type < len(HUNTER_PET_TYPES):
        raise ValueError("Invalid Hunter pet type")
    uptime = proto_option(class_options, "petUptime", "pet_uptime", 0)
    if isinstance(uptime, str) and JSON_NUMBER.fullmatch(uptime):
        uptime = float(uptime)
    if type(uptime) not in (int, float) or not 0 <= uptime <= 1:
        raise ValueError("Invalid Hunter pet uptime")
    report["template_pet_type"] = HUNTER_PET_TYPES[pet_type]
    report["template_pet_uptime"] = uptime
    if not pet_type or uptime == 0:
        return report
    if observed is not None and not isinstance(observed, dict):
        raise ValueError("Invalid captured active pet")
    observed = observed or {}
    level = observed.get("level")
    known = observed.get("present") is True and type(level) is int and level > 0
    report["simulator_pet_level"] = SIMULATOR_CHARACTER_LEVEL
    report["captured_pet_level"] = level if known else None
    report["level"] = ("matches" if level == SIMULATOR_CHARACTER_LEVEL else "unsupported_for_simulation") \
        if known else "unverified"
    if require_level and report["level"] != "matches":
        raise UnsupportedSimulationPetLevel(
            "Hunter pet simulation requires a saved active-pet level of 60. Re-export the saved capture with an updated reader; other or unknown pet levels remain plan-only."
        )
    return report


def request_from(template, equipment_review, encounter_lookup):
    captured_level=equipment_review.get("captured_character_level")
    if type(captured_level) is not int or captured_level!=SIMULATOR_CHARACTER_LEVEL:
        raise UnsupportedSimulationLevel("Pinned Forever simulation requires an explicitly captured level of 60.")
    raid = deepcopy(template["raid"])
    player = single_player(raid)
    check_character_identity(player, equipment_review.get("captured_character_identity"))
    check_profession_identity(player, equipment_review.get("captured_professions"))
    healing_dummy_count(raid)
    rotation = player["rotation"]
    if rotation.get("type") != "TypeAPL" or not any(
        entry.get("action") for entry in rotation.get("priorityList", [])
    ):
        raise ValueError("Provide a single-player template with an explicit APL")
    spec = player_spec(player)
    check_hunter_pet_level(player, equipment_review.get("captured_active_pet"))
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
    if equipment_review["checks"].get("native_slot_category_conflicts"):
        check_native_slot_categories(equipment_review,equipment)
    if equipment_review["client"]["family"] != "forever" or not equipment or len(equipment["items"]) != 17:
        raise ValueError("Provide a resolved Forever equipment review")
    check_native_slot_categories(equipment_review,equipment)
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


def wilson_interval_95(count, iterations):
    fraction = count / iterations
    z = 1.959963984540054
    adjustment = z * z / iterations
    scale = 1 + adjustment
    center = (fraction + adjustment / 2) / scale
    radius = z * math.sqrt((fraction * (1 - fraction) + adjustment / 4) / iterations) / scale
    return {
        "lower": 0.0 if count == 0 else center - radius,
        "upper": 1.0 if count == iterations else center + radius,
    }


def run(engine, expected_hash, request, deadline=None, paired=False, tank_metrics=False, healing_metrics=False,
        cache=None):
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
    if deadline is not None and time.monotonic() >= deadline:
        raise ValueError("Comparison deadline exceeded")
    cache_key = cache.key(expected_hash, request, paired, tank_metrics, healing_metrics) if cache else None
    if cache:
        cached = cache.get(cache_key)
        if cached is not None:
            return {**cached, "cache_status": "hit"}
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
            summary["simulated_death_fraction_wilson_95"] = wilson_interval_95(
                summary["simulated_deaths"], summary["iterations"])
        if healing_metrics:
            mean, deviation = metrics["hps"]["avg"], metrics["hps"]["stdev"]
            oom = metrics["secondsOomAvg"]
            if not all(type(value) in (int, float) and math.isfinite(value) and value >= 0
                       for value in (mean, deviation, oom)):
                raise ValueError("Simulator returned invalid healing metrics")
            summary.update(mean_hps=mean, hps_standard_deviation=deviation, mean_seconds_oom=oom)
            duration = result.get("avgIterationDuration")
            if type(duration) in (int, float) and math.isfinite(duration) and duration > 0:
                summary["mean_iteration_duration_seconds"] = duration
                if oom <= duration:
                    summary["simulated_time_oom_fraction"] = oom / duration
            if paired:
                paired_distributions["hps"] = metrics["hps"]
        for metric, distribution in paired_distributions.items():
            values = distribution["allValues"]
            if (not isinstance(values, list) or len(values) != summary["iterations"]
                    or not all(type(value) in (int, float) and math.isfinite(value) for value in values)):
                raise ValueError("Simulator returned invalid paired samples")
            summary[f"_{metric}_samples"] = values
        verify_engine(engine, expected_hash)
        if cache:
            cache.put(cache_key, summary)
            summary["cache_status"] = "miss"
        return summary


def compare(engine, expected_hash, requests, paired=False, tank_metrics=False, healing_metrics=False, cache=None):
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
            cache=cache,
        )
        if time.monotonic() >= deadline:
            raise ValueError("Comparison deadline exceeded")
    return comparison_from_summaries(requests, [results[key] for key in keys], paired,
                                     tank_metrics, healing_metrics, deadline)


def comparison_from_summaries(requests, summaries, paired=False, tank_metrics=False, healing_metrics=False, deadline=None):
    """Compare already validated completed results without executing more work."""
    if not requests or len(requests) != len(summaries):
        raise ValueError("Comparison requires one completed result per request")
    deadline = time.monotonic() + 120 if deadline is None else deadline
    keys = [json.dumps(request, sort_keys=True, separators=(",", ":"), allow_nan=False) for request in requests]
    unique = dict(zip(keys, requests))
    if paired and (not 2 <= requests[0]['simOptions']['iterations'] <= 5000 or any(
        request['simOptions'] != requests[0]['simOptions'] or request['encounter'] != requests[0]['encounter']
        for request in requests)):
        raise ValueError("Paired comparisons require matching bounded iterations and encounters")
    results = {}
    for key, summary in zip(keys, summaries, strict=True):
        if key in results and results[key] != summary:
            raise ValueError("Identical requests require identical completed results")
        results[key] = deepcopy(summary)
    baseline = results[keys[0]]["mean_dps"]
    leading_pair = None
    if paired and len(unique) >= 2 and not (tank_metrics or healing_metrics):
        leader, runner_up = sorted(unique, key=lambda key: -results[key]["mean_dps"])[:2]
        leader_row, runner_up_row = results[leader], results[runner_up]
        samples = leader_row["_dps_samples"]
        differences = (a - b for a, b in zip(samples, runner_up_row["_dps_samples"], strict=True))
        leading_pair = {
            "leading_review": keys.index(leader) + 1,
            "runner_up_review": keys.index(runner_up) + 1,
            "mean_dps_gap": leader_row["mean_dps"] - runner_up_row["mean_dps"],
            "paired_dps_standard_error": statistics.stdev(differences) / math.sqrt(len(samples)),
        }
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
    rows = []
    for index, key in enumerate(keys, 1):
        row = {"review": index, **results[key],
               "mean_dps_delta_from_baseline": results[key]["mean_dps"] - baseline}
        if paired and key != keys[0] and row["iterations"] >= 200:
            for metric in metrics:
                delta = row[f"mean_{metric}_delta_from_baseline"]
                half_width = 1.96 * row[f"paired_{metric}_standard_error"]
                lower, upper = delta - half_width, delta + half_width
                row[f"paired_{metric}_delta_interval_approx_95"] = {
                    "lower": lower, "upper": upper, "includes_zero": lower <= 0 <= upper,
                }
        rows.append(row)
    comparison = {
        "unique_runs": len(results),
        "baseline_review": 1,
        "results": rows,
    }
    if leading_pair is not None:
        comparison["observed_leading_dps_pair"] = leading_pair
    return comparison


def cacheable_stage_metrics(stages):
    """Only persist the pinned adapter's numeric stage telemetry and enum label."""
    fields = {"inputGearSets", "survivors", "iterations", "concurrency", "durationSeconds",
              "targetErrorPct", "observedErrorPct", "baselineAvgDps", "bestCandidateAvgDps"}
    labels = {"BulkSimStageUnknown", "BulkSimStageLow", "BulkSimStageMedium", "BulkSimStageHigh",
              "BulkSimStageFinalist"}
    return isinstance(stages, list) and all(
        isinstance(stage, dict) and set(stage) <= fields | {"stage"}
        and type(stage.get("stage")) is str
        and stage.get("stage") in labels
        and all((type(value) is int or type(value) is float and math.isfinite(value))
                for name, value in stage.items() if name != "stage")
        for stage in stages
    )


def compare_bulk(engine, expected_hash, requests, top_results, cache=None):
    verify_engine(engine, expected_hash)
    baseline = requests[0]
    baseline_context = None
    candidates = []
    reviews = {}
    seen = {}
    for review, request in enumerate(requests, 1):
        if request["simOptions"] != baseline["simOptions"]:
            raise ValueError("Bulk comparisons require matching simulation settings")
        first_party = request["raid"]["parties"][0].get("players", [])
        if not first_party:
            raise ValueError("Bulk comparisons require the player in the first party")
        context = deepcopy(request)
        context["raid"]["parties"][0]["players"][0]["equipment"] = None
        context = json.dumps(context, sort_keys=True, separators=(",", ":"), allow_nan=False)
        if baseline_context is None:
            baseline_context = context
        if context != baseline_context:
            raise ValueError("Bulk comparisons require matching encounters and templates; only equipment may differ")
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
    cache_key = cache.key(expected_hash, {"operation": "bulk", "requests": requests,
                                         "top_results": top_results}, False, False, False) if cache else None
    if cache:
        cached = cache.get(cache_key)
        if cached is not None:
            return {**cached, "cache_status": "hit"}
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
    summary = {"mode": "staged_bulk", "baseline_review": 1, "results": rows,
               "engine_sha256": expected_hash.lower(), "seed": baseline["simOptions"]["randomSeed"],
               "unique_gear_sets": len(reviews), "stage_metrics": result["stageMetrics"]}
    if cache:
        if cacheable_stage_metrics(summary["stage_metrics"]):
            cache.put(cache_key, summary)
        summary["cache_status"] = "miss"
    return summary


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
    persistence = parser.add_mutually_exclusive_group()
    persistence.add_argument("--cache", action="store_true", help="Reuse exact fixed-run results in this OS user's local cache")
    persistence.add_argument("--simulation-journal", type=Path, help="Private non-evicting numeric checkpoint directory")
    parser.add_argument("--simulation-journal-mib", type=int, help="Checkpoint ciphertext budget, 1 to 512 MiB (default 64)")
    args = parser.parse_args()
    if args.simulation_journal_mib is not None and not args.simulation_journal:
        parser.error("--simulation-journal-mib requires --simulation-journal")
    try:
        budget = 64 if args.simulation_journal_mib is None else args.simulation_journal_mib
        cache = (SimulationJournal(args.simulation_journal, max_payload_bytes=budget * 1024 * 1024) if args.simulation_journal else
                 SimulationCache.for_current_user() if args.cache else None)
        if len(args.equipment_review) > 8:
            raise ValueError("Use at most eight equipment reviews")
        if args.paired and len(args.equipment_review) < 2:
            raise ValueError("Paired mode requires multiple equipment reviews")
        limit = 4 * 1024 * 1024
        template = read_object(args.template, limit)
        check_consumable_groups(template)
        encounter = read_object(args.encounter_lookup, limit)
        reviews = [read_object(path, limit) for path in args.equipment_review]
        requests = [request_from(template, review, encounter) for review in reviews]
        result = (run(args.engine, args.engine_sha256, requests[0], tank_metrics=args.tank_metrics,
                      healing_metrics=args.healing_metrics, cache=cache)
                  if len(requests) == 1 else compare(
                      args.engine, args.engine_sha256, requests, paired=args.paired, tank_metrics=args.tank_metrics,
                      healing_metrics=args.healing_metrics, cache=cache))
        result.update(encounter_summary(requests[0]))
        result["profession_identities"] = [check_profession_identity(
            single_player(template["raid"]), review.get("captured_professions")) for review in reviews]
        result["character_identities"] = [check_character_identity(
            single_player(template["raid"]), review.get("captured_character_identity")) for review in reviews]
        result["hunter_pet_assumptions"] = [check_hunter_pet_level(
            single_player(template["raid"]), review.get("captured_active_pet")) for review in reviews]
        print(json.dumps(result, allow_nan=False))
    except ConflictingConsumables:
        parser.exit(1, "Forever template selects multiple items in the same simulator buff group.\n")
    except UnsupportedSimulationLevel:
        parser.exit(1, "Simulation requires a current equipment review with captured_character_level=60. Re-export the saved capture with an updated MCP reader; other or unknown levels remain plan-only.\n")
    except ConflictingProfessions:
        parser.exit(1, "Simulator template professions disagree with the saved character observation. Match the template to that capture before simulation.\n")
    except ConflictingCharacterIdentity:
        parser.exit(1, "Simulator template class or race disagrees with the saved character observation. Match the template to that capture before simulation.\n")
    except ConflictingNativeSlotCategory:
        parser.exit(1, "Native slot-category conflicts require refreshed item observations or catalogue data before simulation.\n")
    except UnsupportedSimulationPetLevel as error:
        parser.exit(1, f"{error}\n")
    except (OSError, ValueError, TypeError, AttributeError, KeyError, RecursionError, sqlite3.Error, subprocess.SubprocessError) as error:
        parser.exit(1, f"Forever simulation failed ({type(error).__name__}). Check inputs and engine.\n")


if __name__ == "__main__":
    main()
