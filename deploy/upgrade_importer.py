"""Qualify an immutable importer in an isolated store, then atomically replace its pin."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tempfile
import uuid


def qualify(image):
    with tempfile.TemporaryDirectory(prefix="azeroth-import-test-", dir="/var/tmp") as temporary:
        root = Path(temporary)
        os.chown(root, 10006, 10006)
        snapshot = {"schema_version":1,"source":"fixture","captured_at":"2025-01-01T00:00:00Z",
                    "client":{"family":"retail","variant":"live","build":"qualification","interface":120105},
                    "character":{"name":"GatewayFixture","realm":"Synthetic"},"sections":{"equipment":{}}}
        ids = []
        for extended in (False, True):
            if extended:
                snapshot["sections"]["operation_receipts"] = {"a" * 64: {"status":"outcome_unknown", "kind":"move"}}
                snapshot["sections"]["operation_receipts"]["b" * 64] = {"status":"already_selected", "kind":"set_binding"}
                snapshot["sections"]["operation_receipts"]["c" * 64] = {"status":"applied", "kind":"equip_bag_item"}
                snapshot["sections"]["equipment"] = {"1":{"item_id":123,"guid":"Item-Equipped"}}
                snapshot["sections"]["bags"] = {"0":{"1":{"item_id":123,"count":1,"guid":"Item-Carried"}}}
                snapshot["sections"]["operation_receipts"]["d" * 64] = {"status":"applied", "kind":"move_bag_item"}
                snapshot["sections"]["operation_receipts"]["e" * 64] = {"status":"outcome_unknown", "kind":"split_bag_item"}
                snapshot["sections"]["binding_state"] = {"binding_set":2,"coverage":"bound_keys_only"}
                snapshot["sections"]["bindings"] = {"1":{"command":"ACTIONBUTTON1","key1":"CTRL-1","context":0}}
                snapshot["sections"]["talents"] = {"config_id":7,"nodes":{},"loadout_export":{
                    "status":"available","format":"blizzard_traits","import_string":"QUJDRA==",
                    "spec_id":263,"serialization_version":2,"staged_changes":False}}
                snapshot["sections"]["operation_receipts"]["f" * 64] = {"status":"outcome_unknown", "kind":"load_talent_config"}
                snapshot["sections"]["talents"]["saved_loadouts"] = {"spec_id":263,"selected_config_id":8,"entries":{
                    "9":{"name":"Synthetic saved build","import_string":"RUZHSA==","serialization_version":2,"uses_shared_action_bars":False}}}
                snapshot["sections"]["operation_receipts"]["1" * 64] = {"status":"outcome_unknown", "kind":"move_action_slot"}
                snapshot["sections"]["action_bar_observation"]={"coverage":"loaded_builtin_buttons_only",
                    "observed_buttons":2,"missing_buttons":1,"slots":{"1":{"empty":False,"kind":"spell","id":123},"2":{"empty":True}}}
                snapshot["sections"]["equipment_sets"]={"coverage":"native_catalogue_raw_slots","can_use":True,"entries":{"0":{
                    "name":"Synthetic set","icon":134400,"is_equipped":False,"num_items":1,"num_equipped":0,
                    "num_in_inventory":1,"num_lost":0,"num_ignored":0,"assigned_spec":2,
                    "slots":{str(i):{} for i in range(1,20)}}}}
                snapshot["sections"]["equipment_sets"]["entries"]["0"]["slots"]["16"]={"ignored":False,"raw_item_id":123,"raw_location":999,
                    "observed_item":{"kind":"bag","bag":0,"slot":1,"item_id":123,"guid":"Item-Carried","locked":False}}
                snapshot["sections"]["operation_receipts"]["2" * 64]={"status":"applied","kind":"modify_equipment_set"}
                snapshot["sections"]["operation_receipts"]["3" * 64]={"status":"outcome_unknown","kind":"delete_equipment_set"}
                snapshot["sections"]["operation_receipts"]["4" * 64]={"status":"outcome_unknown","kind":"use_equipment_set"}
                snapshot["sections"]["equipment_sets"]["specializations"]={"1":261,"2":262,"3":263}
                snapshot["sections"]["operation_receipts"]["5" * 64]={"status":"applied","kind":"assign_equipment_set_spec"}
                snapshot["sections"]["operation_receipts"]["6" * 64]={"status":"outcome_unknown","kind":"unassign_equipment_set_spec"}
                snapshot["sections"]["operation_receipts"]["7" * 64]={"status":"applied","kind":"create_equipment_set"}
                snapshot["sections"]["operation_receipts"]["8" * 64]={"status":"outcome_unknown","kind":"save_equipment_set"}
                snapshot["sections"]["operation_receipts"]["9" * 64]={"status":"applied","kind":"rename_talent_config"}
                snapshot["sections"]["operation_receipts"]["0" * 64]={"status":"outcome_unknown","kind":"delete_talent_config"}
                snapshot["sections"]["operation_receipts"]["a" * 64]={"status":"applied","kind":"create_talent_config"}
            name = "azeroth-import-test-" + uuid.uuid4().hex
            try:
                result = subprocess.run(["docker","run","--rm","-i","--name",name,"--network","none",
                    "--read-only","--cap-drop","ALL","--security-opt","no-new-privileges","--memory","128m","--cpus","1",
                    "-v",f"{root}:/data",image,"import-stdin"], input=json.dumps(snapshot).encode(),
                    capture_output=True, timeout=25, check=True)
                identifier = result.stdout.decode().strip()
                if not re.fullmatch("[a-f0-9]{64}", identifier):
                    raise ValueError("Invalid import receipt")
                raw = (root / (identifier + ".json")).read_bytes()
                if hashlib.sha256(raw).hexdigest() != identifier or json.loads(raw) != snapshot:
                    raise ValueError("Imported payload mismatch")
                ids.append(identifier)
            finally:
                subprocess.run(["docker","rm","-f",name], capture_output=True, timeout=10, check=False)
        if len(set(ids)) != 2:
            raise ValueError("Expected distinct old/new schema fixtures")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expected-image", required=True)
    parser.add_argument("--image", required=True)
    args = parser.parse_args()
    if not all(re.fullmatch("sha256:[a-f0-9]{64}", image) for image in [args.image,args.expected_image]):
        raise SystemExit("Immutable images required")
    if os.geteuid()!=0 or Path("/etc/hostname").read_text().strip()!="mcp-gateway":
        raise SystemExit("Expected personal CT290 root")
    config = Path("/etc/azeroth-upload/image")
    with (config.parent / ".upgrade.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        info = config.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid!=0 or stat.S_IMODE(info.st_mode)!=0o600:
            raise SystemExit("Unexpected image config ownership/mode")
        if config.read_text().strip()!=args.expected_image:
            raise SystemExit("Importer changed; reconcile before replacing")
        qualify(args.image)
        with tempfile.NamedTemporaryFile(mode="w",dir=config.parent,delete=False) as staged:
            staged.write(args.image + "\n")
            staged.flush()
            os.fsync(staged.fileno())
            pending = Path(staged.name)
        try:
            os.replace(pending,config)
        finally:
            pending.unlink(missing_ok=True)
        if config.read_text().strip()!=args.image or stat.S_IMODE(config.stat().st_mode)!=0o600:
            raise SystemExit("Importer pin readback failed")
        print(json.dumps({"importer_image":args.image,"legacy_and_extended_fixtures":"passed"}))
