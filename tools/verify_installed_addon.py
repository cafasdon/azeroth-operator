"""Read-only installed addon integrity check against an explicit local manifest."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import stat
from tools.inspect_client import read_metadata
from tools.sync_exports import ordinary, absolute

FILES = {"Adapter.lua", "Collectors.lua", "Core.lua", "AzerothOperator.toc"}
OPERATION_FILES = FILES | {"MacroOperations.lua", "MacroImport.lua", "MacroUI.lua"}
BINDING_FILES = OPERATION_FILES | {"BindingOperations.lua", "BindingImport.lua"}
EQUIPMENT_FILES = BINDING_FILES | {"EquipmentOperations.lua", "EquipmentImport.lua"}
BAG_FILES = EQUIPMENT_FILES | {"BagOperations.lua", "BagImport.lua"}
TALENT_FILES = BAG_FILES | {"TalentOperations.lua", "TalentImport.lua"}
ACTION_FILES = TALENT_FILES | {"ActionOperations.lua", "ActionImport.lua"}
EQUIPMENT_SET_FILES = ACTION_FILES | {"EquipmentSetOperations.lua", "EquipmentSetImport.lua"}
EQUIPMENT_SET_USE_FILES = EQUIPMENT_SET_FILES | {"EquipmentSetUse.lua"}
EQUIPMENT_SET_SPEC_FILES = EQUIPMENT_SET_USE_FILES | {"EquipmentSetSpec.lua", "EquipmentSetSpecImport.lua"}
EQUIPMENT_SET_SAVE_FILES = EQUIPMENT_SET_SPEC_FILES | {"EquipmentSetSaveState.lua", "EquipmentSetSave.lua", "EquipmentSetSaveImport.lua"}
MAX_FILE = 1024 * 1024


TALENT_METADATA_FILES = EQUIPMENT_SET_SAVE_FILES | {"TalentMetadata.lua", "TalentMetadataImport.lua"}

def verify(client, manifest):
    client = ordinary(absolute(client))
    receipt = json.loads(read_metadata(ordinary(absolute(manifest)), 16384))
    if not isinstance(receipt, dict) or not isinstance(receipt.get("files"), dict):
        raise ValueError("Invalid manifest")
    files = set(receipt["files"])
    if files not in (FILES, OPERATION_FILES, BINDING_FILES, EQUIPMENT_FILES, BAG_FILES, TALENT_FILES, ACTION_FILES, EQUIPMENT_SET_FILES, EQUIPMENT_SET_USE_FILES, EQUIPMENT_SET_SPEC_FILES, EQUIPMENT_SET_SAVE_FILES, TALENT_METADATA_FILES) or any(
        not isinstance(value, str) or not re.fullmatch("[a-f0-9]{64}", value)
        for value in receipt["files"].values()
    ):
        raise ValueError("Invalid file hashes")
    product = {"retail": "wow", "forever": "wowf"}.get(receipt.get("family"))
    if not product:
        raise ValueError("Unsupported installed-client family")
    if read_metadata(client / ".flavor.info", 4096).splitlines() != ["Product Flavor!STRING:0", product]:
        raise ValueError("Product mismatch")
    target = ordinary(client / "Interface" / "AddOns" / "AzerothOperator")
    problems = []
    verified = 0
    if not target.exists():
        return {"status": "not_installed", "verified_files": 0, "live_qualified": False}
    # Stop enumeration after the expected count plus one; never emit arbitrary names.
    observed = set()
    for path in target.iterdir():
        observed.add(path.name)
        if len(observed) > len(files):
            break
    if observed - files:
        problems.append({"reason": "unexpected_entries"})
    for name in sorted(files):
        path = ordinary(target / name)
        try:
            info = path.stat()
        except FileNotFoundError:
            problems.append({"file": name, "reason": "missing"})
            continue
        if not stat.S_ISREG(info.st_mode):
            problems.append({"file": name, "reason": "not_regular"})
            continue
        with path.open("rb") as handle:
            data = handle.read(MAX_FILE + 1)
        if len(data) > MAX_FILE:
            problems.append({"file": name, "reason": "too_large"})
        elif hashlib.sha256(data).hexdigest() != receipt["files"][name]:
            problems.append({"file": name, "reason": "hash_mismatch"})
        else:
            verified += 1
    return {"status": "matches_manifest" if not problems else "drift",
            "verified_files": verified, "problems": problems, "live_qualified": False,
            "manifest_trust": "caller_supplied", "game_execution": False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--client", required=True)
    parser.add_argument("--manifest", required=True)
    args = parser.parse_args()
    try:
        result = verify(args.client, args.manifest)
        print(json.dumps(result))
        raise SystemExit(0 if result["status"] == "matches_manifest" else 1)
    except (ValueError, OSError, UnicodeError, TypeError) as error:
        parser.exit(1, f"Addon verification rejected: {type(error).__name__}\n")
