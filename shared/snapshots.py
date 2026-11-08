"""Bounded, data-only snapshots. Never evaluate Lua or accept tool-supplied paths."""
import hashlib
import json
import re
import os
import tempfile
from datetime import datetime, timezone
from pathlib import Path

from shared.clients import ClientFamily, ClientIdentity

MAX_BYTES = 4 * 1024 * 1024
SECTIONS = frozenset({"equipment", "bags", "character_bank", "account_bank", "selected_recipe", "tracked_recipes", "reputation", "currency_balances", "wallet", "experience", "spellbook", "flight_map", "durability_alerts", "repair_quote", "merchant_stock", "trainer_services", "pet_care", "bag_capacity", "instance_context", "tracked_achievements", "talents", "stats", "professions", "quests", "currencies", "achievements", "macros", "bindings", "action_bars", "operation_receipts", "binding_state", "action_bar_observation", "equipment_sets"})


def validate_snapshot(value):
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise ValueError("Expected snapshot schema_version 1")
    c = value["client"]
    ClientIdentity(ClientFamily(c["family"]), c["build"], c["interface"], c["variant"])
    for key in ("name", "realm"):
        if not isinstance(value["character"].get(key), str) or not value["character"][key].strip():
            raise ValueError("Character name and realm required")
    if value.get("source") not in ("addon", "official_api", "fixture"):
        raise ValueError("Explicit snapshot source required")
    captured = datetime.fromisoformat(value["captured_at"].replace("Z", "+00:00"))
    if captured.tzinfo is None or captured > datetime.now(timezone.utc):
        raise ValueError("Capture time must be timezone-aware and not in the future")
    sections = value.get("sections")
    if not isinstance(sections, dict) or set(sections) - SECTIONS:
        raise ValueError("Unsupported snapshot section")
    stats = sections.get("stats")
    if isinstance(stats, dict) and {"hearthstone_item_id", "hearthstone_cooldown_seconds"} & stats.keys():
        item = stats.get("hearthstone_item_id")
        cooldown = stats.get("hearthstone_cooldown_seconds")
        if (c["family"] != "forever" or type(item) is not int or not 1 <= item <= 4_294_967_295
                or ("hearthstone_cooldown_seconds" in stats and (type(cooldown) is not int or not 0 <= cooldown <= 4_294_967_295))):
            raise ValueError("Invalid Hearthstone cooldown observation")
    if isinstance(stats, dict) and "hearthstone_bind_name" in stats:
        location = stats["hearthstone_bind_name"]
        if c["family"] != "forever" or not isinstance(location, str) or not 1 <= len(location.encode()) <= 256:
            raise ValueError("Invalid Forever Hearthstone bind name")
    if isinstance(stats, dict) and "new_mail_indicator" in stats:
        if c["family"] != "forever" or type(stats["new_mail_indicator"]) is not bool:
            raise ValueError("Invalid Forever new mail indicator")
    if isinstance(stats, dict) and {"map_min_level", "map_max_level"} & stats.keys():
        map_id, low, high = (stats.get(key) for key in ("map_id", "map_min_level", "map_max_level"))
        if (any(type(n) is not int for n in (map_id, low, high))
                or not 1 <= map_id <= 4_294_967_295 or not 1 <= low <= high <= 255):
            raise ValueError("Invalid captured map level range")
    for name in ("wallet", "repair_quote"):
        if name in sections:
            observation = sections[name]
            if (c["family"] != "forever" or not isinstance(observation, dict) or set(observation) != {"copper"}
                    or type(observation["copper"]) is not int or not 0 <= observation["copper"] <= 9_999_999_999_999):
                raise ValueError(f"Invalid Forever {name} observation")
    if "pet_care" in sections:
        pet = sections["pet_care"]
        if (c["family"] != "forever" or not isinstance(pet, dict)
                or set(pet) - {"present", "happiness", "loyalty", "food_types"}
                or type(pet.get("present")) is not bool or (not pet["present"] and len(pet) != 1)):
            raise ValueError("Invalid pet care data")
        if "happiness" in pet and (type(pet["happiness"]) is not int or not 0 <= pet["happiness"] <= 255):
            raise ValueError("Invalid pet happiness")
        if "loyalty" in pet and (not isinstance(pet["loyalty"], str) or not 1 <= len(pet["loyalty"].encode()) <= 256):
            raise ValueError("Invalid pet loyalty")
        if "food_types" in pet:
            foods = pet["food_types"]
            if (not isinstance(foods, dict) or len(foods) > 16
                    or set(foods) != {str(i) for i in range(1, len(foods) + 1)}
                    or any(not isinstance(name, str) or not 1 <= len(name.encode()) <= 256 for name in foods.values())):
                raise ValueError("Invalid pet food types")
    if "trainer_services" in sections:
        trainer = sections["trainer_services"]
        if (c["family"] != "forever" or not isinstance(trainer, dict)
                or set(trainer) - {"pet_training_points_available"} != {"kind", "entries"}
                or trainer["kind"] not in ("general", "tradeskills", "pet")):
            raise ValueError("Invalid trainer services")
        pet = trainer["kind"] == "pet"
        if "pet_training_points_available" in trainer:
            points = trainer["pet_training_points_available"]
            if not pet or type(points) is not int or not 0 <= points <= 4_294_967_295:
                raise ValueError("Invalid pet training point balance")
        rows = trainer["entries"]
        if (not isinstance(rows, dict) or not 1 <= len(rows) <= 512
                or set(rows) != {str(i) for i in range(1, len(rows) + 1)}):
            raise ValueError("Invalid trainer service count")
        for row in rows.values():
            if not isinstance(row, dict) or set(row) - {"name", "status", "required_level", "rank_label", "price_copper",
                                                        "skill_requirement", "ability_requirements", "price_training_points"}:
                raise ValueError("Invalid trainer service")
            name, status = row.get("name"), row.get("status")
            if "skill_requirement" in row:
                skill = row["skill_requirement"]
                if (status == "used" or not isinstance(skill, dict) or set(skill) != {"name", "required_rank", "met"}
                        or not isinstance(skill["name"], str) or not 1 <= len(skill["name"].encode()) <= 256
                        or type(skill["required_rank"]) is not int or not 0 <= skill["required_rank"] <= 4_294_967_295
                        or type(skill["met"]) is not bool):
                    raise ValueError("Invalid trainer skill requirement")
            if "ability_requirements" in row:
                abilities = row["ability_requirements"]
                if (status == "used" or not isinstance(abilities, dict) or len(abilities) > 16
                        or set(abilities) != {str(i) for i in range(1, len(abilities) + 1)}
                        or any(not isinstance(ability, dict) or set(ability) != {"name", "met"}
                               or not isinstance(ability["name"], str) or not 1 <= len(ability["name"].encode()) <= 256
                               or type(ability["met"]) is not bool for ability in abilities.values())):
                    raise ValueError("Invalid trainer ability requirements")
            level, rank, price = row.get("required_level"), row.get("rank_label"), row.get("price_copper")
            points_price = row.get("price_training_points")
            if (not isinstance(name, str) or not 1 <= len(name.encode()) <= 256
                    or status not in ("available", "unavailable", "used")
                    or ("required_level" in row and (type(level) is not int or not 1 <= level <= 255))
                    or ("rank_label" in row and (not isinstance(rank, str) or not 1 <= len(rank.encode()) <= 256))
                    or ("price_copper" in row and (pet or status == "used" or type(price) is not int or not 0 <= price <= 9_999_999_999_999))
                    or ("price_training_points" in row and (not pet or status == "used" or type(points_price) is not int or not 0 <= points_price <= 4_294_967_295))):
                raise ValueError("Invalid trainer service")
    if "merchant_stock" in sections:
        rows = sections["merchant_stock"]
        if (c["family"] != "forever" or not isinstance(rows, dict) or not 1 <= len(rows) <= 256
                or set(rows) != {str(i) for i in range(1, len(rows) + 1)}):
            raise ValueError("Invalid merchant stock")
        keys = {"price_copper", "bundle_size", "available", "extended_cost", "cost_components"}
        for row in rows.values():
            if not isinstance(row, dict) or set(row) - keys not in ({"item_id"}, {"currency_id"}):
                raise ValueError("Invalid merchant offer")
            if "cost_components" in row:
                costs = row["cost_components"]
                if (row.get("extended_cost") is not True or not isinstance(costs, dict) or not 1 <= len(costs) <= 16
                        or set(costs) != {str(i) for i in range(1, len(costs) + 1)}):
                    raise ValueError("Invalid merchant costs")
                for cost in costs.values():
                    if not isinstance(cost, dict) or set(cost) not in ({"quantity", "item_id"}, {"quantity", "currency_name"}):
                        raise ValueError("Invalid merchant cost component")
                    quantity = cost["quantity"]
                    if "item_id" in cost:
                        item = cost["item_id"]
                        valid_identity = type(item) is int and 1 <= item <= 4_294_967_295
                    else:
                        currency = cost["currency_name"]
                        valid_identity = isinstance(currency, str) and 1 <= len(currency.encode()) <= 256
                    if not valid_identity or type(quantity) is not int or not 1 <= quantity <= 4_294_967_295:
                        raise ValueError("Invalid merchant cost component")
            identity = row.get("item_id", row.get("currency_id"))
            price, bundle, available = (row.get(key) for key in ("price_copper", "bundle_size", "available"))
            if (any(type(n) is not int for n in (identity, price, bundle, available))
                    or not 1 <= identity <= 4_294_967_295 or not 1 <= bundle <= 4_294_967_295
                    or not 0 <= price <= 9_999_999_999_999 or not -1 <= available <= 2_147_483_647
                    or type(row.get("extended_cost")) is not bool):
                raise ValueError("Invalid merchant offer")
    if "instance_context" in sections:
        instance = sections["instance_context"]
        if c["family"] != "forever" or not isinstance(instance, dict):
            raise ValueError("Invalid instance context")
        if instance != {"kind": "none"}:
            name, difficulty = instance.get("name"), instance.get("difficulty_id")
            map_id = instance.get("instance_map_id")
            if (set(instance) - {"kind", "name", "difficulty_id", "instance_map_id", "lock_warning"}
                    or instance.get("kind") not in ("party", "raid", "pvp", "arena", "scenario")
                    or not isinstance(name, str) or not 1 <= len(name.encode()) <= 256
                    or type(difficulty) is not int or not 0 <= difficulty <= 4_294_967_295
                    or ("instance_map_id" in instance and (type(map_id) is not int or not 1 <= map_id <= 4_294_967_295))):
                raise ValueError("Invalid instance context")
            if "lock_warning" in instance:
                warning = instance["lock_warning"]
                keys = {"remaining_seconds", "encounters_total", "encounters_completed"}
                if (instance["kind"] not in ("party", "raid") or not isinstance(warning, dict)
                        or set(warning) - {"encounters"} != keys
                        or any(type(warning[key]) is not int for key in keys)
                        or not 0 <= warning["remaining_seconds"] <= 4_294_967_295
                        or not 1 <= warning["encounters_total"] <= 256
                        or not 0 <= warning["encounters_completed"] <= warning["encounters_total"]):
                    raise ValueError("Invalid instance lock warning")
                if "encounters" in warning:
                    rows = warning["encounters"]
                    if (warning["encounters_completed"] == 0 or not isinstance(rows, dict)
                            or set(rows) != {str(index) for index in range(1, warning["encounters_total"] + 1)}
                            or any(not isinstance(row, dict) or set(row) != {"name", "killed"}
                                   or not isinstance(row["name"], str) or not 1 <= len(row["name"].encode()) <= 256
                                   or type(row["killed"]) is not bool for row in rows.values())
                            or sum(row["killed"] for row in rows.values()) != warning["encounters_completed"]):
                        raise ValueError("Invalid lock encounter rows")
    if "bag_capacity" in sections:
        bags = sections["bag_capacity"]
        if (c["family"] != "forever" or not isinstance(bags, dict) or len(bags) not in (5, 6)
                or set(bags) != {str(index) for index in range(len(bags))}):
            raise ValueError("Invalid carried bag range")
        for slot, bag in bags.items():
            if not isinstance(bag, dict) or set(bag) - {"total_slots", "free_slots", "family_mask"}:
                raise ValueError("Invalid bag capacity")
            total, free = bag.get("total_slots"), bag.get("free_slots")
            if (type(total) is not int or type(free) is not int
                    or not 0 <= free <= total <= 1024 or (slot == "0" and total == 0)):
                raise ValueError("Invalid bag capacity")
            if "family_mask" in bag and (total == 0 or type(bag["family_mask"]) is not int
                                         or not 0 <= bag["family_mask"] <= 4_294_967_295):
                raise ValueError("Invalid bag family")
    if "durability_alerts" in sections:
        alerts = sections["durability_alerts"]
        slots = {"head", "shoulders", "chest", "waist", "legs", "feet", "wrists", "hands",
                 "main_hand", "off_hand", "ranged"}
        if (c["family"] != "forever" or not isinstance(alerts, dict)
                or any(slot not in slots or alert not in ("low", "broken") for slot, alert in alerts.items())):
            raise ValueError("Invalid durability alert")
    if "flight_map" in sections:
        flight_map = sections["flight_map"]
        if (c["family"] != "forever" or not isinstance(flight_map, dict) or set(flight_map) != {"map_id", "entries"}
                or type(flight_map["map_id"]) is not int or not 1 <= flight_map["map_id"] <= 4_294_967_295
                or not isinstance(flight_map["entries"], dict) or len(flight_map["entries"]) > 512):
            raise ValueError("Invalid Forever flight map")
        for slot, row in flight_map["entries"].items():
            required = {"node_id", "name", "state", "map_transition"}
            if (not isinstance(slot, str) or not re.fullmatch(r"[1-9][0-9]{0,3}", slot) or int(slot) > 4096
                    or not isinstance(row, dict) or not required <= row.keys() or set(row) - required - {"fare_copper"}
                    or type(row["node_id"]) is not int or not 1 <= row["node_id"] <= 4_294_967_295
                    or not isinstance(row["name"], str) or not 1 <= len(row["name"].encode("utf-8")) <= 256
                    or row["state"] not in ("current", "reachable", "unreachable") or type(row["map_transition"]) is not bool):
                raise ValueError("Invalid Forever flight destination")
            fare = row.get("fare_copper")
            if fare is not None and (type(fare) is not int or not 0 <= fare <= 9_999_999_999_999
                                     or row["state"] != "reachable" or row["map_transition"]):
                raise ValueError("Invalid Forever flight fare")
    if "spellbook" in sections:
        book = sections["spellbook"]
        if (c["family"] != "forever" or not isinstance(book, dict) or set(book) != {"coverage", "entries"}
                or book["coverage"] != "visible_player_spell_slots"
                or not isinstance(book["entries"], dict) or len(book["entries"]) > 1024):
            raise ValueError("Invalid Forever spellbook observation")
        for slot, spell in book["entries"].items():
            if (not isinstance(slot, str) or not re.fullmatch(r"[1-9][0-9]{0,3}", slot) or int(slot) > 4096
                    or not isinstance(spell, dict) or set(spell) - {"base_spell_id", "spell_id", "name", "passive", "off_spec"}
                    or any(type(spell.get(key)) is not int or not 1 <= spell[key] <= 4_294_967_295
                           for key in ("base_spell_id", "spell_id"))):
                raise ValueError("Invalid Forever spellbook entry")
            name = spell.get("name")
            if (name is not None and (not isinstance(name, str) or not 1 <= len(name.encode("utf-8")) <= 256)
                    or any(spell.get(key) is not None and type(spell[key]) is not bool for key in ("passive", "off_spec"))):
                raise ValueError("Invalid Forever spellbook metadata")
    if "experience" in sections:
        xp = sections["experience"]
        if (c["family"] != "forever" or not isinstance(xp, dict)
                or set(xp) - {"status", "current_xp", "level_threshold_xp", "xp_disabled", "rested_xp", "resting"}):
            raise ValueError("Invalid Forever experience observation")
        status = xp.get("status")
        current, threshold = xp.get("current_xp"), xp.get("level_threshold_xp")
        disabled, rested = xp.get("xp_disabled"), xp.get("rested_xp")
        resting = xp.get("resting")
        if status == "progress":
            if (type(current) is not int or type(threshold) is not int
                    or not 0 <= current <= threshold <= 4_294_967_295 or threshold == 0
                    or (disabled is not None and type(disabled) is not bool)
                    or (resting is not None and type(resting) is not bool)
                    or (rested is not None and (type(rested) is not int or not 0 <= rested <= 4_294_967_295))):
                raise ValueError("Invalid XP progress")
        elif status not in ("trial_capped", "level_capped") or any(
                field is not None for field in (current, threshold, disabled, rested, resting)):
            raise ValueError("Invalid capped XP observation")
    if "tracked_recipes" in sections:
        recipes = sections["tracked_recipes"]
        if (c["family"] != "forever" or not isinstance(recipes, dict)
                or set(recipes) != {"coverage", "crafts", "recrafts"}
                or recipes["coverage"] != "player_tracked_recipes_only"):
            raise ValueError("Invalid tracked recipe observation")
        for group in (recipes["crafts"], recipes["recrafts"]):
            if not isinstance(group, dict) or len(group) > 128:
                raise ValueError("Invalid tracked recipe group")
            for recipe_id, recipe in group.items():
                if (not isinstance(recipe_id, str) or not re.fullmatch(r"[1-9][0-9]{0,9}", recipe_id)
                        or int(recipe_id) > 4_294_967_295 or not isinstance(recipe, dict)
                        or set(recipe) - {"name", "learned"}):
                    raise ValueError("Invalid tracked recipe entry")
                name = recipe.get("name")
                if name is not None and (not isinstance(name, str) or not 1 <= len(name.encode("utf-8")) <= 256):
                    raise ValueError("Invalid tracked recipe name")
                learned = recipe.get("learned")
                if learned is not None and type(learned) is not bool:
                    raise ValueError("Invalid tracked recipe learned status")
    return value


class SnapshotStore:
    def __init__(self, root: Path):
        self.root = root.resolve()

    def read(self, snapshot_id: str):
        if not re.fullmatch(r"[a-f0-9]{64}", snapshot_id):
            raise ValueError("Invalid snapshot id")
        path = self.root / (snapshot_id + ".json")
        if path.resolve().parent != self.root:
            raise ValueError("Snapshot path escaped store")
        with path.open("rb") as stream:
            raw = stream.read(MAX_BYTES + 1)
        if len(raw) > MAX_BYTES:
            raise ValueError("Snapshot exceeds size limit")
        if hashlib.sha256(raw).hexdigest() != snapshot_id:
            raise ValueError("Snapshot content hash mismatch")
        return validate_snapshot(json.loads(raw))

    def ingest(self, value):
        validate_snapshot(value)
        raw = json.dumps(value, ensure_ascii=False, sort_keys=True, allow_nan=False).encode("utf-8")
        if len(raw) > MAX_BYTES:
            raise ValueError("Snapshot exceeds size limit")
        snapshot_id = hashlib.sha256(raw).hexdigest()
        self.root.mkdir(parents=True, exist_ok=True)
        path = self.root / (snapshot_id + ".json")
        if path.exists():
            self.read(snapshot_id)
            return snapshot_id
        descriptor, pending = tempfile.mkstemp(prefix=".pending-", dir=self.root)
        try:
            with os.fdopen(descriptor, "wb") as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(pending, path)
        finally:
            Path(pending).unlink(missing_ok=True)
        return snapshot_id

    def inspect(self, snapshot_id, section=None):
        value = self.read(snapshot_id)
        captured = datetime.fromisoformat(value["captured_at"].replace("Z", "+00:00"))
        age = (datetime.now(timezone.utc) - captured).total_seconds()
        result = {k: value[k] for k in ("client", "character", "source", "captured_at")}
        result.update(snapshot_id=snapshot_id, age_seconds=round(age), live=False)
        if section is not None:
            if section not in SECTIONS:
                raise ValueError("Unsupported section")
            result["available"] = section in value["sections"]
            result["data"] = value["sections"].get(section)
        else:
            result["available_sections"] = sorted(value["sections"])
        return result
