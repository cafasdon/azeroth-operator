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
SECTIONS = SECTIONS | {"pet_action_bar"}
SECTIONS = SECTIONS | {"player_resources"}
SECTIONS = SECTIONS | {"profession_spells"}
SECTIONS = SECTIONS | {"profession_recipe_materials"}
SECTIONS = SECTIONS | {"recipe_item_sources"}
SECTIONS = SECTIONS | {"player_auras"}
SECTIONS = SECTIONS | {"gossip_quests"}
SECTIONS = SECTIONS | {"gossip_options"}
SECTIONS = SECTIONS | {"merchant_buyback"}
SECTIONS = SECTIONS | {"loot_window"}
SECTIONS = SECTIONS | {"player_trade_offer"}
SECTIONS = SECTIONS | {"player_pvp_progression"}
SECTIONS = SECTIONS | {"group_context"}
SECTIONS = SECTIONS | {"bag_item_cooldowns"}


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


def _validate_bag_item_cooldowns(observation, bags):
    def index(key, low, high):
        return (isinstance(key,str) and 1 <= len(key) <= 4 and key.isascii()
            and key.isdecimal() and str(int(key)) == key and low <= int(key) <= high)
    if (not isinstance(observation,dict) or set(observation) != {'coverage','captured_item_locations','entries'}
            or observation['coverage'] != 'captured_carried_items_only'
            or type(observation['captured_item_locations']) is not int
            or not 0 <= observation['captured_item_locations'] <= 4096
            or not isinstance(bags,dict) or not isinstance(observation['entries'],dict)
            or len(observation['entries']) > 65 or observation['entries'].keys() != bags.keys()):
        raise ValueError('Invalid bag cooldown coverage')
    count = 0
    for bag,rows in observation['entries'].items():
        if (not index(bag,0,64) or not isinstance(rows,dict) or not isinstance(bags[bag],dict)
                or rows.keys() != bags[bag].keys()):
            raise ValueError('Invalid bag cooldown source')
        for slot,row in rows.items():
            count += 1
            if count > 4096 or not index(slot,1,1024) or not isinstance(row,dict):
                raise ValueError('Invalid bag cooldown slot')
            status = row.get('status')
            fields = {'item_id','status'} | ({'enabled','remaining_seconds'} if status == 'reported' else set())
            if (status not in ('reported','unavailable') or set(row) != fields
                    or type(row.get('item_id')) is not int or not 1 <= row['item_id'] <= 4_294_967_295
                    or not isinstance(bags[bag][slot],dict) or row['item_id'] != bags[bag][slot].get('item_id')
                    or (status == 'reported' and (type(row['enabled']) is not bool
                        or type(row['remaining_seconds']) is not int or not 0 <= row['remaining_seconds'] <= 4_294_967_295))):
                raise ValueError('Invalid bag cooldown row')
    if count != observation['captured_item_locations']:
        raise ValueError('Invalid bag cooldown count')


def _inventory_index(key, minimum, maximum):
    return (isinstance(key,str) and 1 <= len(key) <= 10 and key.isascii()
        and key.isdecimal() and str(int(key)) == key and minimum <= int(key) <= maximum)


def _validate_inventory_item_identity(item):
    allowed={'item_id','link','count','guid','required_level','sell_price_copper','equip_location','native_stats'}
    if (not isinstance(item,dict) or set(item)-allowed or type(item.get('item_id')) is not int
            or not 1 <= item['item_id'] <= 4_294_967_295):
        raise ValueError('Invalid inventory item fields or identity')
    link=item.get('link')
    if link is not None and (not isinstance(link,str) or len(link.encode()) > 4096):
        raise ValueError('Invalid inventory item link')
    guid=item.get('guid')
    if guid is not None and (not isinstance(guid,str) or not 1 <= len(guid.encode()) <= 128
            or any(ord(character)<32 or 127<=ord(character)<=159 for character in guid)):
        raise ValueError('Invalid inventory item GUID')


def _validate_talent_definition_observation(talents, family):
    if not isinstance(talents,dict): return
    root_fields={'observed_tree_ids','observed_node_count','staged_changes_observed','definition_allocation'}
    extra_fields={'definition_id','purchased_rank','max_rank'}
    nodes=talents.get('nodes')
    present=bool(root_fields & talents.keys()) or (isinstance(nodes,dict)
        and any(isinstance(node,dict) and extra_fields & node.keys() for node in nodes.values()))
    if not present: return
    def uint(number,minimum=0):
        return type(number) is int and minimum <= number <= 4_294_967_295
    trees=talents.get('observed_tree_ids'); count=talents.get('observed_node_count')
    allowed={'config_id','name','nodes','has_unspent','unspent_class','unspent_spec','loadout_export','saved_loadouts'}|root_fields
    if (family!='forever' or set(talents)-allowed or not uint(talents.get('config_id'),1)
            or not isinstance(trees,dict) or not 1<=len(trees)<=16
            or any(not _inventory_index(tree,1,4_294_967_295) or observed is not True for tree,observed in trees.items())
            or not isinstance(nodes,dict) or not uint(count) or not len(nodes)<=count<=4096
            or ('staged_changes_observed' in talents and type(talents['staged_changes_observed']) is not bool)):
        raise ValueError('Invalid talent definition observation')
    for node_id,node in nodes.items():
        if (not _inventory_index(node_id,1,4_294_967_295) or not isinstance(node,dict)
                or set(node)-({'entry_id','rank','tree_id'}|extra_fields)
                or any(not uint(node.get(field),1) for field in ['entry_id','rank','tree_id'])
                or str(node['tree_id']) not in trees):
            raise ValueError('Invalid observed talent node')
        present_fields=extra_fields & node.keys()
        if present_fields and (present_fields!=extra_fields or not uint(node['definition_id'],1)
                or not uint(node['max_rank'],1) or not uint(node['purchased_rank'])
                or node['purchased_rank']>node['max_rank']):
            raise ValueError('Invalid observed talent definition ranks')
    if 'definition_allocation' in talents:
        allocation=talents['definition_allocation']
        if (not isinstance(allocation,dict) or set(allocation)!={'coverage','queried_node_count','queried_entry_count','definitions'}
                or allocation['coverage']!='queried_config_tree_entries'
                or not uint(allocation['queried_node_count']) or allocation['queried_node_count']!=count
                or not uint(allocation['queried_entry_count']) or not isinstance(allocation['definitions'],dict)
                or allocation['queried_entry_count']!=len(allocation['definitions']) or len(allocation['definitions'])>4096):
            raise ValueError('Invalid definition allocation coverage')
        entries=set();node_trees={};locations={};node_entries={}
        for definition,row in allocation['definitions'].items():
            if (not _inventory_index(definition,1,4_294_967_295) or not isinstance(row,dict)
                    or set(row)!={'node_id','entry_id','tree_id','purchased_rank','max_rank'}
                    or any(not uint(row[key],1) for key in ['node_id','entry_id','tree_id','max_rank'])
                    or not uint(row['purchased_rank']) or row['purchased_rank']>row['max_rank']
                    or str(row['tree_id']) not in trees or row['entry_id'] in entries
                    or row['node_id'] in node_trees and node_trees[row['node_id']]!=row['tree_id']):
                raise ValueError('Invalid definition allocation row')
            entries.add(row['entry_id']);node_trees[row['node_id']]=row['tree_id']
            node_entries[row['node_id']]=node_entries.get(row['node_id'],0)+1
            if node_entries[row['node_id']]>64:raise ValueError('Definition node entry budget exceeded')
            locations[(row['node_id'],row['entry_id'])]=(int(definition),row)
            if row['purchased_rank']>0:
                active=nodes.get(str(row['node_id']))
                if active is None or active['entry_id']!=row['entry_id'] or active['tree_id']!=row['tree_id']:
                    raise ValueError('Purchased definition active entry mismatch')
        if len(node_trees)>count:raise ValueError('Definition node budget mismatch')
        for node,active in nodes.items():
            observed=locations.get((int(node),active['entry_id']))
            if (observed is None or observed[1]['tree_id']!=active['tree_id']
                    or 'definition_id' in active and (active['definition_id']!=observed[0]
                        or any(active[key]!=observed[1][key] for key in ['purchased_rank','max_rank']))):
                raise ValueError('Active definition allocation mismatch')


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
    unavailable = value.get("unavailable", {})
    _validate_talent_definition_observation(sections.get('talents'),c['family'])
    if 'bag_item_cooldowns' in sections:
        if c['family'] != 'forever': raise ValueError('Bag cooldowns require Forever')
        _validate_bag_item_cooldowns(sections['bag_item_cooldowns'],sections.get('bags'))
    if (not isinstance(unavailable, dict) or set(unavailable) - SECTIONS
            or set(unavailable) & sections.keys()):
        raise ValueError("Invalid unavailable section")
    # Match Rust str::trim's Unicode White_Space set, rather than Python's
    # additional U+001C..U+001F separators; the byte bound is UTF-8 in both readers.
    whitespace = "\t\n\v\f\r \u0085\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000"
    if any(not isinstance(reason, str) or not reason.strip(whitespace)
            or len(reason.encode()) > 256 for reason in unavailable.values()):
        raise ValueError("Invalid unavailable reason")
    if "action_bar_observation" in sections:
        _validate_action_bar_observation(sections["action_bar_observation"])
    bank_sections_present='character_bank' in sections or 'account_bank' in sections
    if bank_sections_present and c['family'] != 'forever':
        raise ValueError('Bank snapshots require Forever')
    inventory_containers=set()
    for storage in ("equipment","bags","character_bank","account_bank"):
        data=sections.get(storage,{})
        if not isinstance(data,dict): raise ValueError('Inventory section must be an object')
        if storage!='equipment':
            for container in data:
                if (not _inventory_index(container,0,64)
                        or (bank_sections_present and container in inventory_containers)):
                    raise ValueError('Invalid or repeated inventory container')
                inventory_containers.add(container)
        groups=[data] if storage=="equipment" else data.values()
        for group in groups:
            if not isinstance(group,dict): raise ValueError('Inventory slots must be an object')
            for slot,item in group.items():
                if not _inventory_index(slot,1,19 if storage=='equipment' else 1024):
                    raise ValueError('Invalid inventory slot')
                _validate_inventory_item_identity(item)
                if isinstance(item,dict):
                    count=item.get('count')
                    if ((storage=='equipment' and count is not None)
                            or (storage!='equipment' and (type(count) is not int
                                or not 1<=count<=4_294_967_295))):
                        raise ValueError('Missing or invalid item stack count')
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
    if "recipe_item_sources" in sections:
        source = sections["recipe_item_sources"]
        def reference_uint(v, low=0, high=4_294_967_295):
            return type(v) is int and low <= v <= high
        def reference_id(k):
            return isinstance(k, str) and k.isascii() and k.isdecimal() and str(int(k)) == k and reference_uint(int(k), 1)
        fields = {"npc_drops", "object_drops", "container_drops", "quest_rewards", "vendors"}
        if (c["family"] != "forever" or not isinstance(source, dict)
                or not {"coverage", "provider", "provider_mode", "provider_contract_version", "profession_id", "reported_item_count", "entries"}<=set(source)
                or set(source)-{"coverage", "provider", "provider_mode", "provider_contract_version", "profession_id", "reported_item_count", "entries", "entity_locations"}
                or source["coverage"] != "first_reported_fixed_recipe_reagents" or source["provider"] != "questiedb"
                or source["provider_mode"] not in ("source", "baked")
                or not reference_uint(source["provider_contract_version"], 1) or not reference_uint(source["profession_id"], 1)
                or not reference_uint(source["reported_item_count"], 0, 16384) or not isinstance(source["entries"], dict)
                or len(source["entries"]) != min(64, source["reported_item_count"])):
            raise ValueError("Invalid reference coverage")
        for item, row in source["entries"].items():
            if not reference_id(item) or not isinstance(row, dict): raise ValueError("Invalid reference row")
            if row.get("status") != "reported":
                if set(row) != {"status"} or row["status"] not in ("not_in_provider", "unavailable"):
                    raise ValueError("Invalid reference status")
                continue
            if set(row) - (fields | {"status"}): raise ValueError("Invalid reference fields")
            for field in fields & set(row):
                ref = row[field]
                if (not isinstance(ref, dict) or set(ref) != {"entries", "distinct_id_count", "truncated"}
                        or not reference_uint(ref["distinct_id_count"], 0, 4096) or type(ref["truncated"]) is not bool
                        or ref["truncated"] != (ref["distinct_id_count"] > 8) or not isinstance(ref["entries"], dict)
                        or len(ref["entries"]) != min(8, ref["distinct_id_count"])
                        or any(not reference_id(k) or v is not True for k, v in ref["entries"].items())):
                    raise ValueError("Invalid reference list")
        if "entity_locations" in source:
            cache=source["entity_locations"]
            if (not isinstance(cache,dict) or set(cache)!={"coordinate_system","reported_npc_count","reported_object_count","npcs","objects"}
                    or cache["coordinate_system"]!="questiedb_area_percent"):
                raise ValueError("Invalid reference location coverage")
            for kind,count_field,categories in [("npcs","reported_npc_count",["npc_drops","vendors"]),
                    ("objects","reported_object_count",["object_drops"])]:
                ids={int(id) for row in source["entries"].values() if row["status"]=="reported"
                    for category in categories for id in row.get(category,{}).get("entries",{})}
                rows=cache[kind]
                if (not reference_uint(cache[count_field]) or cache[count_field]!=len(ids)
                        or not isinstance(rows,dict) or set(rows)!={str(id) for id in sorted(ids)[:32]}):
                    raise ValueError("Invalid retained reference entity coverage")
                for row in rows.values():
                    if not isinstance(row,dict):raise ValueError("Invalid entity reference")
                    if row.get("status")!="reported":
                        if row not in ({"status":"not_in_provider"},{"status":"unavailable"}):raise ValueError("Invalid entity status")
                        continue
                    if set(row)-{"status","spawns"}:raise ValueError("Invalid entity fields")
                    if "spawns" not in row:continue
                    spawns=row["spawns"]
                    if (not isinstance(spawns,dict) or set(spawns)!={"reported_point_count","truncated","entries"}
                            or not reference_uint(spawns["reported_point_count"],0,4096)
                            or type(spawns["truncated"]) is not bool or spawns["truncated"]!=(spawns["reported_point_count"]>16)
                            or not isinstance(spawns["entries"],dict)
                            or set(spawns["entries"])!={str(i) for i in range(1,min(16,spawns["reported_point_count"])+1)}):
                        raise ValueError("Invalid reference point coverage")
                    for point in spawns["entries"].values():
                        if not isinstance(point,dict) or not reference_uint(point.get("area_id"),1):raise ValueError("Invalid reference area")
                        if point.get("kind")=="instance_presence":
                            if set(point)!={"kind","area_id"}:raise ValueError("Invalid instance presence")
                        elif point.get("kind")=="coordinate":
                            if (not {"kind","area_id","x_percent","y_percent"}<=set(point)
                                    or set(point)-{"kind","area_id","x_percent","y_percent","provider_phase_id"}
                                    or any(type(point[key]) not in (int,float) or not math.isfinite(point[key]) or not 0<=point[key]<=100
                                        for key in ("x_percent","y_percent"))
                                    or "provider_phase_id" in point and not reference_uint(point["provider_phase_id"],1)):
                                raise ValueError("Invalid reference coordinate")
                        else:raise ValueError("Invalid reference point kind")
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
        inventory_slots = set()
        for slot, bag in bags.items():
            if not isinstance(bag, dict) or set(bag) - {"total_slots", "free_slots", "family_mask", "equipped_item"}:
                raise ValueError("Invalid bag capacity")
            total, free = bag.get("total_slots"), bag.get("free_slots")
            if (type(total) is not int or type(free) is not int
                    or not 0 <= free <= total <= 1024 or (slot == "0" and total == 0)):
                raise ValueError("Invalid bag capacity")
            if "family_mask" in bag and (total == 0 or type(bag["family_mask"]) is not int
                                         or not 0 <= bag["family_mask"] <= 4_294_967_295):
                raise ValueError("Invalid bag family")
            if "equipped_item" in bag:
                item = bag["equipped_item"]
                if (slot == "0" or total == 0 or not isinstance(item, dict) or set(item) != {"item_id", "inventory_slot"}
                        or type(item["item_id"]) is not int or not 1 <= item["item_id"] <= 4_294_967_295
                        or type(item["inventory_slot"]) is not int or not 1 <= item["inventory_slot"] <= 255
                        or item["inventory_slot"] in inventory_slots):
                    raise ValueError("Invalid equipped bag identity")
                inventory_slots.add(item["inventory_slot"])
    if "pet_action_bar" in sections:
        bar=sections["pet_action_bar"]
        if (c["family"]!="forever" or not isinstance(bar,dict) or set(bar)!={"coverage","pet_present","has_action_bar","entries"}
                or bar["coverage"]!="readable_pet_action_slots" or type(bar["pet_present"]) is not bool
                or type(bar["has_action_bar"]) is not bool or not isinstance(bar["entries"],dict)
                or (not bar["pet_present"] and bar["has_action_bar"]) or (not bar["has_action_bar"] and bar["entries"])):
            raise ValueError("Invalid pet action context")
        for slot,row in bar["entries"].items():
            if (slot not in {str(i) for i in range(1,11)} or not isinstance(row,dict)
                    or row.get("kind") not in ("spell","command")
                    or set(row)-{"kind","spell_id","command_token","is_active","autocast_allowed","autocast_enabled","range_check_performed","in_range"}
                    or any(type(row[field]) is not bool for field in ("is_active","autocast_allowed","autocast_enabled","range_check_performed","in_range") if field in row)):
                raise ValueError("Invalid pet action row")
            if row["kind"]=="spell":
                if type(row.get("spell_id")) is not int or not 1<=row["spell_id"]<=4_294_967_295 or "command_token" in row:
                    raise ValueError("Invalid pet action spell")
            elif ("spell_id" in row or not isinstance(row.get("command_token"),str) or len(row["command_token"])>64
                    or not re.fullmatch(r"PET_[A-Z0-9_]+",row["command_token"])):
                raise ValueError("Invalid pet action command")
            if (row.get("range_check_performed") is True and "in_range" not in row
                    or "in_range" in row and row.get("range_check_performed") is not True):
                raise ValueError("Invalid pet action range observation")
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
    if "profession_recipe_materials" in sections:
        materials = sections["profession_recipe_materials"]
        def material_uint(value):
            return type(value) is int and 1 <= value <= 4_294_967_295
        def material_id(value):
            return (isinstance(value, str) and re.fullmatch(r"[1-9][0-9]{0,9}", value)
                    and int(value) <= 4_294_967_295)
        if (c["family"] != "forever" or not isinstance(materials, dict)
                or set(materials) != {"coverage", "profession_id", "entries"}
                or materials["coverage"] != "reported_known_spell_schematics"
                or not material_uint(materials["profession_id"])
                or not isinstance(materials["entries"], dict) or len(materials["entries"]) > 4096):
            raise ValueError("Invalid profession recipe materials")
        retained = 0
        for key, row in materials["entries"].items():
            if not material_id(key) or not isinstance(row, dict):
                raise ValueError("Invalid recipe material row")
            if row.get("status") != "fixed_items":
                if (set(row) != {"status"} or row["status"] not in
                        ("unavailable", "unsupported_required_slots", "budget_exhausted")):
                    raise ValueError("Invalid recipe material status")
                continue
            if (not {"status", "reagents"} <= set(row)
                    or set(row) - {"status", "reagents", "output_item_id", "base_output_quantity", "reported_craftable_count", "cooldown"}
                    or not isinstance(row["reagents"], dict) or len(row["reagents"]) > 64
                    or any(not material_id(item) or not material_uint(quantity)
                           for item, quantity in row["reagents"].items())
                    or ("output_item_id" in row and not material_uint(row["output_item_id"]))):
                raise ValueError("Invalid recipe material details")
            if ("reported_craftable_count" in row and (type(row["reported_craftable_count"]) is not int
                    or not 0 <= row["reported_craftable_count"] <= 4_294_967_295)):
                raise ValueError("Invalid reported recipe craft count")
            if "cooldown" in row:
                cooldown=row["cooldown"]
                allowed={"remaining_seconds","daily","charges","max_charges"}
                if (not isinstance(cooldown,dict) or not cooldown or set(cooldown)-allowed
                        or any(type(cooldown[field]) is not int or not 0<=cooldown[field]<=4_294_967_295
                            for field in ["remaining_seconds","charges","max_charges"] if field in cooldown)
                        or ("daily" in cooldown and (type(cooldown["daily"]) is not bool or "remaining_seconds" not in cooldown))
                        or (("charges" in cooldown)!=("max_charges" in cooldown))
                        or ("charges" in cooldown and not 0<=cooldown["charges"]<=cooldown["max_charges"]
                            or "max_charges" in cooldown and cooldown["max_charges"]==0)
                        or not {"remaining_seconds","charges"}.intersection(cooldown)):
                    raise ValueError("Invalid bulk recipe cooldown")
            retained += len(row["reagents"])
            if retained > 16384:
                raise ValueError("Recipe material budget exceeded")
            if "base_output_quantity" in row:
                quantity = row["base_output_quantity"]
                if (not isinstance(quantity, dict) or set(quantity) != {"minimum", "maximum"}
                        or not material_uint(row.get("output_item_id"))
                        or not material_uint(quantity["minimum"]) or not material_uint(quantity["maximum"])
                        or quantity["minimum"] > quantity["maximum"]):
                    raise ValueError("Invalid recipe material output range")
    if "profession_spells" in sections:
        spells=sections["profession_spells"]
        if (c["family"]!="forever" or not isinstance(spells,dict) or set(spells)!={"coverage","profession_id","entries"}
                or spells["coverage"]!="reported_known_profession_spells" or type(spells["profession_id"]) is not int
                or not 1<=spells["profession_id"]<=4_294_967_295 or not isinstance(spells["entries"],dict)
                or len(spells["entries"])>4096):
            raise ValueError("Invalid profession spell observation")
        for key,row in spells["entries"].items():
            if (not isinstance(key,str) or not re.fullmatch(r"[1-9][0-9]{0,9}",key)
                    or int(key)>4_294_967_295 or not isinstance(row,dict) or row):
                raise ValueError("Invalid profession spell ID")
    if "group_context" in sections:
        group=sections['group_context']
        if (c['family']!='forever' or not isinstance(group,dict) or set(group)!={'coverage','categories'}
                or group['coverage']!='reported_player_group_context' or not isinstance(group['categories'],dict)
                or not 1<=len(group['categories'])<=2 or set(group['categories'])-{'home','instance'}):
            raise ValueError('Invalid group context')
        for row in group['categories'].values():
            if (not isinstance(row,dict) or not {'in_group','in_raid','reported_member_count'}<=row.keys()
                    or set(row)-{'in_group','in_raid','reported_member_count','player_is_leader'}
                    or type(row['in_group']) is not bool or type(row['in_raid']) is not bool
                    or type(row['reported_member_count']) is not int or not 0<=row['reported_member_count']<=4_294_967_295
                    or 'player_is_leader' in row and type(row['player_is_leader']) is not bool):
                raise ValueError('Invalid group category')
    if "player_pvp_progression" in sections:
        pvp=sections["player_pvp_progression"]
        required={'coverage','faction_id','rank','rank_points','next_rank_threshold'}
        optional={'max_rank','current_week_max_rank','previous_week_max_rank'}
        if (c['family']!='forever' or not isinstance(pvp,dict) or not required<=pvp.keys()
                or set(pvp)-required-optional-{'total_points_by_rank'} or pvp['coverage']!='reported_pvp_rank_faction'
                or type(pvp['faction_id']) is not int or pvp['faction_id']!=2800
                or any(type(pvp[key]) is not int or not 0<=pvp[key]<=4_294_967_295
                       for key in (required|optional)-{'coverage','faction_id'} if key in pvp)):
            raise ValueError('Invalid PvP progression')
        if 'total_points_by_rank' in pvp:
            totals=pvp['total_points_by_rank']
            ranks={str(pvp[key]) for key in {'rank'}|optional if key in pvp}
            if (not isinstance(totals,dict) or not 1<=len(totals)<=4 or not set(totals)<=ranks
                    or any(type(value) is not int or not 0<=value<=4_294_967_295 for value in totals.values())):
                raise ValueError('Invalid PvP total points')
    if "player_trade_offer" in sections:
        trade=sections["player_trade_offer"]
        if (c["family"]!="forever" or not isinstance(trade,dict)
                or set(trade)!={"coverage","entries"}
                or trade["coverage"]!="visible_player_trade_normal_slots"
                or not isinstance(trade["entries"],dict)
                or set(trade["entries"])!={str(i) for i in range(1,7)}):
            raise ValueError("Invalid player trade coverage")
        for row in trade["entries"].values():
            if not isinstance(row,dict):raise ValueError("Invalid player trade slot")
            if row.get("status")=="item":
                if (set(row)!={"status","item_id","quantity"}
                        or any(type(row[key]) is not int or not 1<=row[key]<=4_294_967_295
                               for key in ('item_id','quantity'))):
                    raise ValueError("Invalid player trade item")
            elif row!={"status":"unreadable_or_empty"}:
                raise ValueError("Invalid player trade slot")
    if "loot_window" in sections:
        loot=sections["loot_window"]
        if (c["family"]!="forever" or not isinstance(loot,dict)
                or set(loot)!={"coverage","slot_count","entries"}
                or loot["coverage"]!="visible_loot_window_slots"
                or type(loot["slot_count"]) is not int or not 0<=loot["slot_count"]<=64
                or not isinstance(loot["entries"],dict)
                or set(loot["entries"])!={str(i) for i in range(1,loot["slot_count"]+1)}):
            raise ValueError("Invalid loot window coverage")
        for row in loot["entries"].values():
            if not isinstance(row,dict):raise ValueError("Invalid loot slot")
            if row.get("status")=="item":
                if (set(row)!={"status","item_id","quantity"}
                        or type(row["item_id"]) is not int or not 1<=row["item_id"]<=4_294_967_295
                        or type(row["quantity"]) is not int or not 0<=row["quantity"]<=4_294_967_295):
                    raise ValueError("Invalid loot item")
            elif row.get("status")=="reported_currency":
                if (set(row)!={"status","reported_currency_id","reported_quantity"}
                        or type(row["reported_currency_id"]) is not int or not 1<=row["reported_currency_id"]<=4_294_967_295
                        or type(row["reported_quantity"]) is not int or not 0<=row["reported_quantity"]<=4_294_967_295):
                    raise ValueError("Invalid reported loot currency")
            elif row not in ({"status":"unreadable_or_non_item"},{"status":"money"}):
                raise ValueError("Invalid loot slot status")
    if "merchant_buyback" in sections:
        buyback=sections["merchant_buyback"]
        if (c["family"]!="forever" or not isinstance(buyback,dict) or set(buyback)!={"coverage","entries"}
                or buyback["coverage"]!="visible_merchant_buyback" or not isinstance(buyback["entries"],dict)
                or len(buyback["entries"])>12 or set(buyback["entries"])!={str(i) for i in range(1,len(buyback["entries"])+1)}):
            raise ValueError("Invalid buyback coverage")
        for row in buyback["entries"].values():
            if (not isinstance(row,dict) or not {"item_id","count"}<=row.keys()
                    or set(row)-{"item_id","count","buyback_price_copper"}
                    or any(type(row[field]) is not int or not 1<=row[field]<=4_294_967_295 for field in ("item_id","count"))
                    or "buyback_price_copper" in row and (type(row["buyback_price_copper"]) is not int or not 0<=row["buyback_price_copper"]<=9_999_999_999_999)):
                raise ValueError("Invalid buyback observation")
    if "gossip_options" in sections:
        gossip = sections["gossip_options"]
        if (c["family"] != "forever" or not isinstance(gossip, dict)
                or set(gossip) != {"coverage", "entries"} or gossip["coverage"] != "visible_gossip_options"
                or not isinstance(gossip["entries"], dict) or len(gossip["entries"]) > 128
                or set(gossip["entries"]) != {str(i) for i in range(1, len(gossip["entries"])+1)}):
            raise ValueError("Invalid gossip option coverage")
        for row in gossip["entries"].values():
            if (not isinstance(row, dict) or not {"name"} <= row.keys()
                    or set(row)-{"name", "option_id", "spell_id", "reported_status", "reported_flags", "reported_order_index"}
                    or not isinstance(row["name"], str) or not 1 <= len(row["name"].encode("utf-8")) <= 512):
                raise ValueError("Invalid gossip option")
            for field in ("option_id", "spell_id", "reported_status", "reported_flags", "reported_order_index"):
                if field in row:
                    low, high = ((-2**31, 2**31-1) if field == "reported_order_index" else
                                 (1 if field in ("option_id", "spell_id") else 0, 2**32-1))
                    if type(row[field]) is not int or not low <= row[field] <= high:
                        raise ValueError("Invalid gossip option number")
    if "gossip_quests" in sections:
        gossip=sections["gossip_quests"]
        if (c["family"]!="forever" or not isinstance(gossip,dict) or gossip.get("coverage")!="visible_gossip_quest_lists"
                or set(gossip)-{"coverage","available","active"} or not ({"available","active"}&gossip.keys())):
            raise ValueError("Invalid gossip quest coverage")
        for field in ("available","active"):
            if field not in gossip:continue
            rows=gossip[field]
            if not isinstance(rows,dict) or len(rows)>128:raise ValueError("Invalid gossip quest list")
            for key,row in rows.items():
                if (not isinstance(key,str) or not re.fullmatch(r"[1-9][0-9]{0,9}",key) or int(key)>4_294_967_295
                        or not isinstance(row,dict) or set(row)-{"repeatable","is_complete","is_trivial","is_ignored"}
                        or any(type(value) is not bool for value in row.values())):
                    raise ValueError("Invalid gossip quest observation")
    if "player_auras" in sections:
        auras=sections["player_auras"]
        if (c["family"]!="forever" or not isinstance(auras,dict) or set(auras)!={"coverage","groups"}
                or auras["coverage"]!="readable_nonprivate_player_aura_lists"
                or not isinstance(auras["groups"],dict) or not auras["groups"]
                or set(auras["groups"])-{"helpful","harmful"}):
            raise ValueError("Invalid player aura coverage")
        for group in auras["groups"].values():
            if not isinstance(group,dict) or set(group)!={"entries"} or not isinstance(group["entries"],dict):
                raise ValueError("Invalid player aura group")
            rows=group["entries"]
            if len(rows)>256 or set(rows)!={str(i) for i in range(1,len(rows)+1)}:
                raise ValueError("Invalid player aura rows")
            for row in rows.values():
                if (not isinstance(row,dict) or not {"spell_id"}<=row.keys() or set(row)-{"spell_id","applications"}
                        or type(row["spell_id"]) is not int or not 1<=row["spell_id"]<=4_294_967_295
                        or "applications" in row and (type(row["applications"]) is not int or not 0<=row["applications"]<=4_294_967_295)):
                    raise ValueError("Invalid player aura row")
    if "player_resources" in sections:
        resources=sections["player_resources"]
        if (c["family"]!="forever" or not isinstance(resources,dict)
                or resources.get("coverage")!="readable_player_resources"
                or set(resources)-{"coverage","health","primary_power"}
                or not ({"health","primary_power"}&resources.keys())):
            raise ValueError("Invalid player resource coverage")
        for field in ("health","primary_power"):
            if field not in resources:continue
            row=resources[field]
            fields={"current","maximum"} | ({"power_type","units"} if field=="primary_power" else set())
            if (not isinstance(row,dict) or set(row)!=fields
                    or type(row.get("current")) is not int or type(row.get("maximum")) is not int
                    or not 0<=row["current"]<=row["maximum"]<=4_294_967_295):
                raise ValueError("Invalid player resource pair")
            if field=="primary_power" and (type(row["power_type"]) is not int or not 0<=row["power_type"]<=63 or row["units"]!="unmodified"):
                raise ValueError("Invalid primary power context")
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
                    or not isinstance(spell, dict) or set(spell) - {"base_spell_id", "spell_id", "name", "passive", "off_spec", "level_learned", "usability", "charge_count", "reported_cast_count", "cooldown_state"}
                    or any(type(spell.get(key)) is not int or not 1 <= spell[key] <= 4_294_967_295
                           for key in ("base_spell_id", "spell_id"))):
                raise ValueError("Invalid Forever spellbook entry")
            if "usability" in spell:
                usability=spell["usability"]
                if (not isinstance(usability,dict) or set(usability)!={"usable","insufficient_power"}
                        or any(type(v) is not bool for v in usability.values())):
                    raise ValueError("Invalid spell usability observation")
            if "charge_count" in spell:
                charges=spell["charge_count"]
                if (not isinstance(charges,dict) or set(charges)!={"current","maximum"}
                        or any(type(n) is not int for n in charges.values())
                        or not 0<=charges["current"]<=charges["maximum"]<=4_294_967_295 or charges["maximum"]==0):
                    raise ValueError("Invalid spell charge observation")
            if "reported_cast_count" in spell and (type(spell["reported_cast_count"]) is not int
                    or not 0<=spell["reported_cast_count"]<=4_294_967_295):
                raise ValueError("Invalid spell cast count observation")
            if "cooldown_state" in spell:
                cooldown=spell["cooldown_state"]
                if (not isinstance(cooldown,dict) or set(cooldown)!={"enabled","active"}
                        or any(type(v) is not bool for v in cooldown.values())):
                    raise ValueError("Invalid spell cooldown state")
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
