"""Bounded, data-only snapshots. Never evaluate Lua or accept tool-supplied paths."""
import hashlib
import json
import re
import os
import math
import tempfile
from datetime import datetime, timezone
from pathlib import Path

from shared.clients import ClientFamily, ClientIdentity

MAX_BYTES = 4 * 1024 * 1024
SECTIONS = frozenset({"equipment", "bags", "character_bank", "account_bank", "selected_recipe", "tracked_recipes", "reputation", "currency_balances", "wallet", "experience", "spellbook", "flight_map", "durability_alerts", "repair_quote", "merchant_stock", "trainer_services", "pet_care", "skill_lines", "bag_capacity", "instance_context", "tracked_achievements", "talents", "stats", "professions", "quests", "currencies", "achievements", "macros", "bindings", "action_bars", "operation_receipts", "binding_state", "action_bar_observation", "equipment_sets"})
SECTIONS = SECTIONS | {"mount_collection", "toy_collection_view", "saved_instances", "saved_world_bosses"}
SECTIONS = SECTIONS | {"profession_recipe_view"}
SECTIONS = SECTIONS | {"mail_attachments"}
SECTIONS = SECTIONS | {"pet_collection_view"}
SECTIONS = SECTIONS | {"equipment_durability"}
SECTIONS = SECTIONS | {"weapon_enchants"}


def _validate_action_bar_observation(observation):
    def index(key, maximum):
        return (isinstance(key, str) and 1 <= len(key) <= 4 and key.isascii()
                and key.isdecimal() and not key.startswith("0") and int(key) <= maximum)

    def text(value, maximum):
        return isinstance(value, str) and 1 <= len(value.encode()) <= maximum

    if (not isinstance(observation, dict)
            or set(observation) - {"coverage", "slots", "observed_buttons", "missing_buttons", "button_hotkeys"}
            or observation.get("coverage") != "loaded_builtin_buttons_only"):
        raise ValueError("Invalid action bar observation")
    observed = observation.get("observed_buttons")
    missing = observation.get("missing_buttons")
    slots = observation.get("slots")
    if (type(observed) is not int or type(missing) is not int
            or observed < 1 or missing < 0 or observed + missing > 2048
            or not isinstance(slots, dict) or not 1 <= len(slots) <= observed):
        raise ValueError("Invalid action bar coverage")
    for slot, row in slots.items():
        if not index(slot, 4096) or not isinstance(row, dict) or type(row.get("empty")) is not bool:
            raise ValueError("Invalid action bar slot")
        if row["empty"]:
            if set(row) != {"empty"}:
                raise ValueError("Empty action contains data")
            continue
        action_id = row.get("id")
        subtype = row.get("sub_type")
        if (set(row) - {"empty", "kind", "id", "sub_type"} or not text(row.get("kind"), 16384)
                or (action_id is not None and (type(action_id) is not int or not 1 <= action_id <= 4_294_967_295))
                or (subtype is not None and (not isinstance(subtype, str) or len(subtype.encode()) > 16384))):
            raise ValueError("Invalid occupied action slot")
    hotkeys = observation.get("button_hotkeys")
    if hotkeys is None:
        return
    if not isinstance(hotkeys, dict) or len(hotkeys) > observed:
        raise ValueError("Invalid action button hotkeys")
    buttons = set()
    for key, row in hotkeys.items():
        if not index(key, observed) or not isinstance(row, dict) or set(row) - {"button", "slot", "key1", "key2"}:
            raise ValueError("Invalid action button hotkeys")
        button, slot = row.get("button"), row.get("slot")
        if (not text(button, 132) or not re.fullmatch(r"[A-Za-z0-9_]+", button) or button in buttons
                or type(slot) is not int or str(slot) not in slots
                or any(row.get(key) is not None and not text(row[key], 128) for key in ("key1", "key2"))):
            raise ValueError("Invalid action button hotkeys")
        buttons.add(button)


def _validate_currency_balances(observation):
    if (not isinstance(observation, dict) or set(observation) != {"coverage", "entries"}
            or observation["coverage"] != "visible_currency_ids_only"
            or not isinstance(observation["entries"], dict) or len(observation["entries"]) > 512):
        raise ValueError("Invalid currency balances")
    counters = {"quantity", "max_quantity", "max_weekly_quantity", "earned_this_week", "total_earned"}
    flags = {"account_wide", "inactive", "cap_uses_total_earned"}
    required = counters | flags | {"name"}
    for key, row in observation["entries"].items():
        if (not isinstance(key, str) or not 1 <= len(key) <= 10 or not key.isascii()
                or not key.isdecimal() or key.startswith("0") or int(key) > 4_294_967_295
                or not isinstance(row, dict) or set(row) - {"can_earn_per_week"} != required
                or not isinstance(row["name"], str) or not 1 <= len(row["name"].encode()) <= 256
                or any(type(row[field]) is not int or not 0 <= row[field] <= 4_294_967_295 for field in counters)
                or any(type(row[field]) is not bool for field in flags)
                or (row.get("can_earn_per_week") is not None and type(row["can_earn_per_week"]) is not bool)):
            raise ValueError("Invalid currency balance entry")


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
    if "action_bar_observation" in sections:
        _validate_action_bar_observation(sections["action_bar_observation"])
    for storage in ("equipment","bags","character_bank","account_bank"):
        data=sections.get(storage,{})
        if not isinstance(data,dict):continue
        groups=[data] if storage=="equipment" else data.values()
        for group in groups:
            if not isinstance(group,dict):continue
            for item in group.values():
                if isinstance(item,dict) and "sell_price_copper" in item:
                    if (type(item["sell_price_copper"]) is not int or not 0<=item["sell_price_copper"]<=9_999_999_999_999
                            or not isinstance(item.get("link"),str) or not 1<=len(item["link"].encode())<=4096):
                        raise ValueError("Invalid linked item sell price")
                if isinstance(item,dict) and "equip_location" in item:
                    location=item["equip_location"]
                    if (not isinstance(location,str) or len(location)>64
                            or (location!="" and not re.fullmatch(r"INVTYPE_[A-Z0-9_]+",location))
                            or not isinstance(item.get("link"),str) or not 1<=len(item["link"].encode())<=4096):
                        raise ValueError("Invalid native equip location")
                if isinstance(item,dict) and "native_stats" in item:
                    stats=item["native_stats"]
                    if (not isinstance(item.get("link"),str) or not 1<=len(item["link"].encode())<=4096
                            or not isinstance(stats,dict) or len(stats)>128
                            or any(not isinstance(key,str) or len(key)>96
                                or not re.fullmatch(r"(?:ITEM_MOD_[A-Z0-9_]+_SHORT|RESISTANCE[0-6]_NAME|EMPTY_SOCKET_[A-Z_]+)",key)
                                or type(n) not in (int,float) or not -4_294_967_295<=n<=4_294_967_295 or not math.isfinite(n)
                                for key,n in stats.items())):
                        raise ValueError("Invalid native item stats")
                if isinstance(item,dict) and item.get("required_level") is not None:
                    if (type(item["required_level"]) is not int or not 0<=item["required_level"]<=4_294_967_295
                            or not isinstance(item.get("link"),str) or not 1<=len(item["link"].encode())<=4096):
                        raise ValueError("Invalid linked item required level")
    if "pet_collection_view" in sections:
        view=sections["pet_collection_view"]
        if (c["family"]!="forever" or not isinstance(view,dict)
                or set(view)!={"coverage","displayed_rows","entries"} or view["coverage"]!="filtered_journal_species"
                or type(view["displayed_rows"]) is not int or not 0<=view["displayed_rows"]<=10000
                or not isinstance(view["entries"],dict) or len(view["entries"])>10000):
            raise ValueError("Invalid pet collection view")
        total=0
        for key,row in view["entries"].items():
            if (not isinstance(key,str) or not key.isascii() or not key.isdecimal()
                    or str(int(key))!=key or not 1<=int(key)<=4_294_967_295
                    or not isinstance(row,dict) or set(row)!={"name","owned_rows","unowned_rows"}
                    or not isinstance(row["name"],str) or not 1<=len(row["name"].encode())<=256
                    or any(type(row[k]) is not int or not 0<=row[k]<=4_294_967_295 for k in ("owned_rows","unowned_rows"))
                    or not 1<=row["owned_rows"]+row["unowned_rows"]<=10000):
                raise ValueError("Invalid pet collection species")
            total+=row["owned_rows"]+row["unowned_rows"]
        if total!=view["displayed_rows"]:raise ValueError("Inconsistent pet journal rows")
    if "mail_attachments" in sections:
        mail=sections["mail_attachments"]
        if (c["family"]!="forever" or not isinstance(mail,dict)
                or set(mail)!={"coverage","loaded_messages","total_messages","messages"}
                or mail["coverage"]!="loaded_inbox_attachments"
                or type(mail["loaded_messages"]) is not int or not 0<=mail["loaded_messages"]<=100
                or type(mail["total_messages"]) is not int or not mail["loaded_messages"]<=mail["total_messages"]<=4_294_967_295
                or not isinstance(mail["messages"],dict)
                or set(mail["messages"])!={str(i) for i in range(1,mail["loaded_messages"]+1)}):
            raise ValueError("Invalid inbox coverage")
        for attachments in mail["messages"].values():
            if not isinstance(attachments,dict) or len(attachments)>16:
                raise ValueError("Invalid inbox attachments")
            for slot,row in attachments.items():
                if (not isinstance(slot,str) or not slot.isascii() or not slot.isdecimal()
                        or str(int(slot))!=slot or not 1<=int(slot)<=16 or not isinstance(row,dict)):
                    raise ValueError("Invalid inbox slot")
                kind=row.get("kind")
                if kind=="unknown":
                    if set(row)!={"kind"}:raise ValueError("Invalid unknown attachment")
                elif kind in ("item","currency"):
                    field="item_id" if kind=="item" else "currency_id"
                    if (set(row)!={"kind",field,"count"} or any(type(row[k]) is not int
                            or not 1<=row[k]<=4_294_967_295 for k in (field,"count"))):
                        raise ValueError("Invalid known attachment")
                else:raise ValueError("Invalid attachment kind")
    if "profession_recipe_view" in sections:
        view = sections["profession_recipe_view"]
        if (c["family"] != "forever" or not isinstance(view, dict)
                or set(view) != {"coverage", "profession_id", "entries"}
                or view["coverage"] != "filtered_local_recipe_list"
                or type(view["profession_id"]) is not int or not 1 <= view["profession_id"] <= 4_294_967_295
                or not isinstance(view["entries"], dict) or len(view["entries"]) > 4096):
            raise ValueError("Invalid profession recipe view")
        for key, row in view["entries"].items():
            if (not isinstance(key, str) or not key.isascii() or not key.isdecimal()
                    or str(int(key)) != key or not 1 <= int(key) <= 4_294_967_295
                    or not isinstance(row, dict) or set(row) - {"name", "learned", "relative_difficulty", "can_skill_up", "skill_ups_reported"}
                    or (row.get("name") is not None and (not isinstance(row["name"], str)
                        or not 1 <= len(row["name"].encode()) <= 256))
                    or (row.get("learned") is not None and type(row["learned"]) is not bool)):
                raise ValueError("Invalid profession recipe row")
            if ("relative_difficulty" in row and row["relative_difficulty"] not in ("optimal","medium","easy","trivial")
                    or "can_skill_up" in row and type(row["can_skill_up"]) is not bool
                    or "skill_ups_reported" in row and (type(row["skill_ups_reported"]) is not int or not 0 <= row["skill_ups_reported"] <= 4_294_967_295)):
                raise ValueError("Invalid recipe skill-up observation")
    if "saved_world_bosses" in sections:
        saved = sections["saved_world_bosses"]
        if (c["family"] != "forever" or not isinstance(saved, dict)
                or set(saved) != {"coverage", "update_age_seconds", "entries"} or saved["coverage"] != "updated_cache"
                or type(saved["update_age_seconds"]) is not int or not 0 <= saved["update_age_seconds"] <= 4_294_967_295
                or not isinstance(saved["entries"], dict) or len(saved["entries"]) > 128
                or set(saved["entries"]) != {str(i) for i in range(1, len(saved["entries"])+1)}):
            raise ValueError("Invalid saved world boss coverage")
        for row in saved["entries"].values():
            if (not isinstance(row, dict) or set(row) != {"name", "reset_seconds"}
                    or type(row["reset_seconds"]) is not int or not -(2**31) <= row["reset_seconds"] < 2**31
                    or not isinstance(row["name"], str) or not 1 <= len(row["name"].encode()) <= 256):
                raise ValueError("Invalid saved world boss row")
    if "saved_instances" in sections:
        saved = sections["saved_instances"]
        if (c["family"] != "forever" or not isinstance(saved, dict)
                or set(saved) != {"coverage", "update_age_seconds", "entries"} or saved["coverage"] != "updated_cache"
                or type(saved["update_age_seconds"]) is not int or not 0 <= saved["update_age_seconds"] <= 4_294_967_295
                or not isinstance(saved["entries"], dict) or len(saved["entries"]) > 128
                or set(saved["entries"]) != {str(i) for i in range(1, len(saved["entries"])+1)}):
            raise ValueError("Invalid saved instance coverage")
        required = {"name", "reset_seconds", "difficulty_id", "locked", "extended", "is_raid", "max_players"}
        for row in saved["entries"].values():
            if (not isinstance(row, dict) or not required <= set(row) or set(row) - required - {"difficulty_name"}
                    or type(row["reset_seconds"]) is not int or not -(2**31) <= row["reset_seconds"] < 2**31
                    or any(type(row[field]) is not int or not 0 <= row[field] <= 4_294_967_295 for field in ("difficulty_id", "max_players"))
                    or any(type(row[field]) is not bool for field in ("locked", "extended", "is_raid"))
                    or not isinstance(row["name"], str) or not 1 <= len(row["name"].encode()) <= 256
                    or (row.get("difficulty_name") is not None and (not isinstance(row["difficulty_name"], str)
                        or not 1 <= len(row["difficulty_name"].encode()) <= 256))):
                raise ValueError("Invalid saved instance row")
    if "toy_collection_view" in sections:
        view = sections["toy_collection_view"]
        if (c["family"] != "forever" or not isinstance(view, dict)
                or set(view) != {"coverage", "displayed_count", "entries"}
                or view["coverage"] != "filtered_journal"
                or type(view["displayed_count"]) is not int or not 0 <= view["displayed_count"] <= 4096
                or not isinstance(view["entries"], dict) or len(view["entries"]) != view["displayed_count"]):
            raise ValueError("Invalid toy journal view")
        for key, row in view["entries"].items():
            if (not isinstance(key, str) or not key.isascii() or not key.isdecimal()
                    or str(int(key)) != key or not 1 <= int(key) <= 4_294_967_295
                    or not isinstance(row, dict) or set(row) != {"name", "collected", "favorite"}
                    or not isinstance(row["name"], str) or not 1 <= len(row["name"].encode()) <= 256
                    or type(row["collected"]) is not bool or type(row["favorite"]) is not bool):
                raise ValueError("Invalid toy observation")
    if "mount_collection" in sections:
        collection = sections["mount_collection"]
        if (c["family"] != "forever" or not isinstance(collection, dict)
                or set(collection) != {"catalogue_count", "entries"}
                or type(collection["catalogue_count"]) is not int
                or not 1 <= collection["catalogue_count"] <= 4096
                or not isinstance(collection["entries"], dict)
                or len(collection["entries"]) > collection["catalogue_count"]):
            raise ValueError("Invalid mount collection")
        for key, row in collection["entries"].items():
            if (not isinstance(key, str) or not key.isascii() or not key.isdecimal()
                    or str(int(key)) != key or not 1 <= int(key) <= 4_294_967_295
                    or not isinstance(row, dict) or set(row) != {"name", "spell_id", "usable", "favorite"}
                    or not isinstance(row["name"], str) or not 1 <= len(row["name"].encode()) <= 256
                    or type(row["spell_id"]) is not int or not 1 <= row["spell_id"] <= 4_294_967_295
                    or type(row["usable"]) is not bool or type(row["favorite"]) is not bool):
                raise ValueError("Invalid collected mount")
    if "currency_balances" in sections:
        if c["family"] != "forever":
            raise ValueError("Currency balances require Forever")
        _validate_currency_balances(sections["currency_balances"])
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
    if "skill_lines" in sections:
        skills = sections["skill_lines"]
        if c["family"] != "forever" or not isinstance(skills, dict) or len(skills) > 512:
            raise ValueError("Invalid skill lines")
        for key, row in skills.items():
            if (not isinstance(key, str) or not 1 <= len(key) <= 10 or not key.isascii()
                    or not key.isdecimal() or key.startswith("0") or int(key) > 4_294_967_295
                    or not isinstance(row, dict) or set(row) - {"name", "rank", "max_rank", "category_id", "modifier", "temporary_points"}):
                raise ValueError("Invalid skill line")
            for field in ("modifier", "temporary_points"):
                if field in row and (type(row[field]) is not int or not -2_147_483_648 <= row[field] <= 2_147_483_647):
                    raise ValueError("Invalid skill modifier")
            name, rank, cap = row.get("name"), row.get("rank"), row.get("max_rank")
            category = row.get("category_id")
            if (not isinstance(name, str) or not 1 <= len(name.encode()) <= 256
                    or type(rank) is not int or type(cap) is not int or not 0 <= rank <= cap <= 4_294_967_295
                    or ("category_id" in row and (type(category) is not int or not 1 <= category <= 4_294_967_295))):
                raise ValueError("Invalid skill line")
    if "pet_care" in sections:
        pet = sections["pet_care"]
        if (c["family"] != "forever" or not isinstance(pet, dict)
                or set(pet) - {"present", "happiness", "loyalty", "food_types", "carried_food", "family_id", "level", "training_points"}
                or type(pet.get("present")) is not bool or (not pet["present"] and len(pet) != 1)):
            raise ValueError("Invalid pet care data")
        if "training_points" in pet:
            points = pet["training_points"]
            if (not isinstance(points, dict) or set(points) != {"total", "used"}
                    or any(type(n) is not int or not -(2**31) <= n < 2**31 for n in points.values())):
                raise ValueError("Invalid pet training points")
        for field in ("family_id", "level"):
            if field in pet and (type(pet[field]) is not int or not 1 <= pet[field] <= 4_294_967_295):
                raise ValueError("Invalid active pet details")
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
        if "carried_food" in pet:
            food = pet["carried_food"]
            bags = sections.get("bags")
            if (not isinstance(bags, dict) or not isinstance(food, dict) or len(food) > 256
                    or any(not isinstance(key, str) or not 1 <= len(key) <= 10 or not key.isascii()
                           or not key.isdecimal() or key.startswith("0") or int(key) > 4_294_967_295
                           or type(count) is not int or not 1 <= count <= 4_294_967_295 for key, count in food.items())):
                raise ValueError("Invalid carried pet food")
            totals = {}
            for bag in bags.values():
                if not isinstance(bag, dict):
                    continue
                for item in bag.values():
                    if isinstance(item, dict):
                        item_id, count = item.get("item_id"), item.get("count")
                        if type(item_id) is int and type(count) is int and 1 <= count <= 4_294_967_295:
                            totals[item_id] = totals.get(item_id, 0) + count
            if any(totals.get(int(key)) != count for key, count in food.items()):
                raise ValueError("Carried pet food does not match bags")
    if "trainer_services" in sections:
        trainer = sections["trainer_services"]
        if (c["family"] != "forever" or not isinstance(trainer, dict)
                or set(trainer) - {"pet_training_points_available", "pet_level", "primary_profession_slots_full"} != {"kind", "entries"}
                or trainer["kind"] not in ("general", "tradeskills", "pet")):
            raise ValueError("Invalid trainer services")
        pet = trainer["kind"] == "pet"
        if "primary_profession_slots_full" in trainer and (pet or type(trainer["primary_profession_slots_full"]) is not bool):
            raise ValueError("Invalid trainer profession slots")
        if "pet_level" in trainer:
            level = trainer["pet_level"]
            if not pet or type(level) is not int or not 1 <= level <= 255:
                raise ValueError("Invalid pet level")
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
                                                        "skill_requirement", "ability_requirements", "price_training_points", "is_profession"}:
                raise ValueError("Invalid trainer service")
            name, status = row.get("name"), row.get("status")
            if "is_profession" in row and (pet or status == "used" or type(row["is_profession"]) is not bool):
                raise ValueError("Invalid trainer profession flag")
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
    if "weapon_enchants" in sections:
        enchants=sections["weapon_enchants"]
        if (c["family"]!="forever" or not isinstance(enchants,dict) or set(enchants)!={"coverage","hands"}
                or enchants["coverage"]!="public_weapon_enchant_lists" or not isinstance(enchants["hands"],dict)
                or set(enchants["hands"])-{"main_hand","off_hand","ranged"}):
            raise ValueError("Invalid weapon enchant coverage")
        for hand in enchants["hands"].values():
            if (not isinstance(hand,dict) or set(hand)!={"entries"} or not isinstance(hand["entries"],dict)
                    or len(hand["entries"])>32 or set(hand["entries"])!={str(i) for i in range(1,len(hand["entries"])+1)}):
                raise ValueError("Invalid weapon enchant list")
            for row in hand["entries"].values():
                if (not isinstance(row,dict) or not {"has_enchant","enchant_type"}<=row.keys()
                        or set(row)-{"has_enchant","enchant_type","enchant_id","charges","remaining_milliseconds"}
                        or type(row["has_enchant"]) is not bool or row["enchant_type"] not in ("none","permanent","temporary","imbue")
                        or any(row["has_enchant"] is not True or type(row[field]) is not int
                            or not (1 if field=="enchant_id" else 0)<=row[field]<=4_294_967_295
                            for field in ("enchant_id","charges","remaining_milliseconds") if field in row)):
                    raise ValueError("Invalid weapon enchant observation")
    if "equipment_durability" in sections:
        durability=sections["equipment_durability"]
        if (c["family"]!="forever" or not isinstance(durability,dict) or set(durability)!={"coverage","entries"}
                or durability["coverage"]!="readable_equipped_slots_only" or not isinstance(durability["entries"],dict)):
            raise ValueError("Invalid equipment durability coverage")
        for slot,row in durability["entries"].items():
            if (not isinstance(slot,str) or slot not in {str(i) for i in range(1,20)}
                    or not isinstance(row,dict) or set(row)!={"current","maximum"}
                    or type(row["current"]) is not int or type(row["maximum"]) is not int
                    or not 0 <= row["current"] <= row["maximum"] <= 4_294_967_295 or row["maximum"]==0):
                raise ValueError("Invalid equipment durability")
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
                    or not isinstance(row, dict) or not required <= row.keys() or set(row) - required - {"fare_copper", "map_position", "observed_route_node_ids"}
                    or type(row["node_id"]) is not int or not 1 <= row["node_id"] <= 4_294_967_295
                    or not isinstance(row["name"], str) or not 1 <= len(row["name"].encode("utf-8")) <= 256
                    or row["state"] not in ("current", "reachable", "unreachable") or type(row["map_transition"]) is not bool):
                raise ValueError("Invalid Forever flight destination")
            if "observed_route_node_ids" in row:
                path=row["observed_route_node_ids"]
                nodes=list(flight_map["entries"].values())
                if (not isinstance(path,dict) or not 2 <= len(path) <= 65
                        or set(path)!={str(i) for i in range(1,len(path)+1)}
                        or any(type(n) is not int or not 1 <= n <= 4_294_967_295 for n in path.values())
                        or len(set(path.values()))!=len(path) or row["state"]!="reachable" or row["map_transition"]):
                    raise ValueError("Invalid flight route nodes")
                origins=[node for node in nodes if isinstance(node,dict) and node.get("state")=="current" and node.get("map_transition") is False]
                if (len(origins)!=1 or path["1"]!=origins[0].get("node_id") or path[str(len(path))]!=row["node_id"]
                        or any(len(matches:=[node for node in nodes if isinstance(node,dict) and node.get("node_id")==n])!=1
                               or matches[0].get("map_transition") is not False for n in path.values())):
                    raise ValueError("Invalid flight route context")
            if "map_position" in row:
                position=row["map_position"]
                if (not isinstance(position,dict) or set(position)!={"x","y"}
                        or any(type(n) not in (int,float) or not 0 <= n <= 1 for n in position.values())):
                    raise ValueError("Invalid flight map position")
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
                    or not isinstance(spell, dict) or set(spell) - {"base_spell_id", "spell_id", "name", "passive", "off_spec", "level_learned", "usability"}
                    or any(type(spell.get(key)) is not int or not 1 <= spell[key] <= 4_294_967_295
                           for key in ("base_spell_id", "spell_id"))):
                raise ValueError("Invalid Forever spellbook entry")
            if "usability" in spell:
                usability=spell["usability"]
                if (not isinstance(usability,dict) or set(usability)!={"usable","insufficient_power"}
                        or any(type(v) is not bool for v in usability.values())):
                    raise ValueError("Invalid spell usability observation")
            if "level_learned" in spell and (type(spell["level_learned"]) is not int or not 0 <= spell["level_learned"] <= 4_294_967_295):
                raise ValueError("Invalid spell learned-level observation")
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
