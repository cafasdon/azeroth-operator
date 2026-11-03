"""Update only an existing, hash-matched addon; retain its exact previous files."""
import argparse
import json
import os
from pathlib import Path
import tempfile
from tools.package_addon import render_entries
from tools.package_bigwigs import sha, verified_contents
from tools.sync_exports import ordinary
from tools.verify_installed_addon import FILES, OPERATION_FILES, BINDING_FILES, EQUIPMENT_FILES, BAG_FILES, TALENT_FILES, ACTION_FILES, EQUIPMENT_SET_FILES, EQUIPMENT_SET_USE_FILES


def replace(path, data):
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as handle:
        handle.write(data)
        handle.flush()
        os.fsync(handle.fileno())
        pending = Path(handle.name)
    try:
        os.replace(pending, path)
    finally:
        pending.unlink(missing_ok=True)


def install(client, old_manifest, new_manifest, backups):
    client = ordinary(Path(client).absolute()).resolve(strict=True)
    old = json.loads(Path(old_manifest).read_text())
    new = json.loads(Path(new_manifest).read_text())
    if any(old[key] != new[key] for key in ("family", "variant", "interface")):
        raise ValueError("Client identity changed; fresh qualification required")
    product = {"retail":"wow", "forever":"wowf"}.get(new["family"])
    if not product or ordinary(client / ".flavor.info").read_text().splitlines() != ["Product Flavor!STRING:0", product]:
        raise ValueError("Client product mismatch or unsupported discovery")
    entries = render_entries(new["family"], new["variant"], new["interface"], new["version"])
    if Path(new["archive"]).name != new["archive"]:
        raise ValueError("Invalid archive path")
    archive = ordinary(Path(new_manifest).resolve().parent / new["archive"])
    if sha(archive.read_bytes()) != new["sha256"]:
        raise ValueError("Archive hash mismatch")
    verified_contents(archive, entries)
    old_files = set(old["files"])
    if new["files"] != {name:sha(data) for name,data in entries.items()} or old_files not in (FILES, OPERATION_FILES, BINDING_FILES, EQUIPMENT_FILES, BAG_FILES, TALENT_FILES, ACTION_FILES, EQUIPMENT_SET_FILES, EQUIPMENT_SET_USE_FILES) or not old_files <= set(entries):
        raise ValueError("File manifest mismatch")
    target = ordinary(client / "Interface" / "AddOns" / "AzerothOperator")
    if {path.name for path in target.iterdir()} != old_files:
        raise ValueError("Unexpected installed files; preserve and reconcile")
    previous = {name:ordinary(target / name).read_bytes() for name in old_files}
    if any(sha(data) != old["files"][name] for name,data in previous.items()):
        raise ValueError("Installed addon changed; refusing overwrite")
    backups = ordinary(Path(backups).absolute())
    backups.mkdir(parents=True, exist_ok=True)
    backup = Path(tempfile.mkdtemp(prefix="addon-update-", dir=backups))
    for name,data in previous.items():
        (backup / name).write_bytes(data)
    (backup / "manifest.json").write_text(json.dumps(old, indent=2))
    written = []
    try:
        # Publish the TOC last so newly referenced modules already exist.
        for name in sorted(entries, key=lambda n: (n == "AzerothOperator.toc", n)):
            data = entries[name]
            if name in previous and ordinary(target / name).read_bytes() != previous[name]:
                raise ValueError("Installed file changed during update")
            if name not in previous and (target / name).exists():
                raise ValueError("New file appeared during update")
            replace(target / name, data)
            written.append(name)
        if any(sha((target / name).read_bytes()) != new["files"][name] for name in entries):
            raise ValueError("Installed file readback mismatch")
    except Exception:
        # Roll back only our own surviving writes; preserve concurrent user edits.
        for name in written:
            path = ordinary(target / name)
            if path.read_bytes() == entries[name]:
                if name in previous:
                    replace(path, previous[name])
                else:
                    path.unlink()
        raise
    return {"version":new["version"],"family":new["family"],"verified_files":len(entries),
            "backup":str(backup),"live_qualified":False,"requires_client_reload_or_restart":True}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--client", required=True)
    parser.add_argument("--old-manifest", required=True)
    parser.add_argument("--new-manifest", required=True)
    parser.add_argument("--backups", required=True)
    args = parser.parse_args()
    print(json.dumps(install(args.client,args.old_manifest,args.new_manifest,args.backups)))
