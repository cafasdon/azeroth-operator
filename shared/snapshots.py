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
SECTIONS = frozenset({"equipment", "bags", "character_bank", "account_bank", "selected_recipe", "tracked_recipes", "reputation", "currency_balances", "wallet", "experience", "spellbook", "flight_map", "durability_alerts", "tracked_achievements", "talents", "stats", "professions", "quests", "currencies", "achievements", "macros", "bindings", "action_bars", "operation_receipts", "binding_state", "action_bar_observation", "equipment_sets"})


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
    if isinstance(stats, dict) and {"map_min_level", "map_max_level"} & stats.keys():
        map_id, low, high = (stats.get(key) for key in ("map_id", "map_min_level", "map_max_level"))
        if (any(type(n) is not int for n in (map_id, low, high))
                or not 1 <= map_id <= 4_294_967_295 or not 1 <= low <= high <= 255):
            raise ValueError("Invalid captured map level range")
    if "wallet" in sections:
        wallet = sections["wallet"]
        if (c["family"] != "forever" or not isinstance(wallet, dict) or set(wallet) != {"copper"}
                or type(wallet["copper"]) is not int or not 0 <= wallet["copper"] <= 9_999_999_999_999):
            raise ValueError("Invalid Forever wallet observation")
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
                or set(xp) - {"status", "current_xp", "level_threshold_xp", "xp_disabled", "rested_xp"}):
            raise ValueError("Invalid Forever experience observation")
        status = xp.get("status")
        current, threshold = xp.get("current_xp"), xp.get("level_threshold_xp")
        disabled, rested = xp.get("xp_disabled"), xp.get("rested_xp")
        if status == "progress":
            if (type(current) is not int or type(threshold) is not int
                    or not 0 <= current <= threshold <= 4_294_967_295 or threshold == 0
                    or (disabled is not None and type(disabled) is not bool)
                    or (rested is not None and (type(rested) is not int or not 0 <= rested <= 4_294_967_295))):
                raise ValueError("Invalid XP progress")
        elif status not in ("trial_capped", "level_capped") or any(
                field is not None for field in (current, threshold, disabled, rested)):
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
