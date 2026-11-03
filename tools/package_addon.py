"""Local deterministic test package. Distribution publishing remains a separate gate."""
import argparse
import hashlib
import json
import re
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def render_entries(family, variant, interface, version):
    if family not in {"retail", "classic", "forever"}:
        raise ValueError("Unknown family")
    for value in (variant, version):
        if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}", value):
            raise ValueError("Invalid variant/version")
    if type(interface) is not int or not 1 <= interface <= 999999:
        raise ValueError("Invalid interface")
    adapter = f'local _, AO = ...\nAO.family = "{family}"\nAO.variant = "{variant}"\nAO.interface = {interface}\n'
    toc = f"""## Interface: {interface}
## Title: Azeroth Operator
## Notes: Character export and player-applied macros; development build, not live-qualified
## Author: cafasdon
## Version: {version}
## SavedVariables: AzerothOperatorExport, AzerothOperatorOperations
Adapter.lua
Collectors.lua
Core.lua
MacroOperations.lua
BindingOperations.lua
BindingImport.lua
EquipmentOperations.lua
EquipmentImport.lua
BagOperations.lua
BagImport.lua
TalentOperations.lua
TalentImport.lua
ActionOperations.lua
ActionImport.lua
EquipmentSetOperations.lua
EquipmentSetImport.lua
MacroImport.lua
MacroUI.lua
"""
    entries = {"Adapter.lua": adapter.encode(), "AzerothOperator.toc": toc.encode()}
    for name in ("Collectors.lua", "Core.lua", "MacroOperations.lua", "BindingOperations.lua", "BindingImport.lua", "EquipmentOperations.lua", "EquipmentImport.lua", "BagOperations.lua", "BagImport.lua", "TalentOperations.lua", "TalentImport.lua", "ActionOperations.lua", "ActionImport.lua", "EquipmentSetOperations.lua", "EquipmentSetImport.lua", "MacroImport.lua", "MacroUI.lua"):
        entries[name] = (ROOT / "addon" / name).read_text(encoding="utf-8").replace("\r\n", "\n").encode()
    return entries


def package(family, variant, interface, version, destination):
    entries = render_entries(family, variant, interface, version)
    destination = Path(destination)
    destination.mkdir(parents=True, exist_ok=True)
    archive = destination / f"AzerothOperator-{version}-{family}-{variant}.zip"
    # Stable timestamps/order for repeatable archives; no repository-wide glob.
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
        for name, data in sorted(entries.items()):
            info = zipfile.ZipInfo("AzerothOperator/" + name, (2020, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            output.writestr(info, data)
    with zipfile.ZipFile(archive) as check:
        assert check.testzip() is None
        assert set(check.namelist()) == {"AzerothOperator/" + name for name in entries}
        for name, data in entries.items():
            assert check.read("AzerothOperator/" + name) == data
    receipt = {"archive": archive.name, "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
               "family": family, "variant": variant, "interface": interface,
               "version": version, "qualification": "development_only",
               "files": {name: hashlib.sha256(data).hexdigest() for name, data in entries.items()}}
    archive.with_suffix(".manifest.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    return archive, receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--family", required=True, choices=["retail", "classic", "forever"])
    parser.add_argument("--variant", required=True)
    parser.add_argument("--interface", required=True, type=int)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    archive, receipt = package(args.family, args.variant, args.interface, args.version, args.output)
    print(json.dumps(receipt, indent=2))
