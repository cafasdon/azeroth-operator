"""First-install the capture-only Forever addon on Windows without replacing files."""
import argparse
import json
import os
from pathlib import Path
import stat
import tempfile

from tools.inspect_client import inspect, read_metadata
from tools.package_addon import render_entries
from tools.package_bigwigs import sha, verified_contents
from tools.sync_exports import absolute, ordinary
from tools.verify_installed_addon import FILES, verify


def install(client, manifest, expected_build):
    if os.name != "nt":
        raise ValueError("Windows no-replace rename required")
    client = ordinary(absolute(client))
    manifest = ordinary(absolute(manifest))
    receipt = json.loads(read_metadata(manifest, 16384))
    if receipt["family"] != "forever" or receipt["variant"] != "live":
        raise ValueError("Expected a Forever live capture package")
    identity = inspect(str(client), "forever", "live")
    if identity["status"] != "identified" or identity["build"] != expected_build:
        raise ValueError("Forever build differs from the reviewed build")
    entries = render_entries("forever", "live", receipt["interface"], receipt["version"])
    if set(entries) != FILES or receipt["files"] != {name: sha(data) for name, data in entries.items()}:
        raise ValueError("Capture-only package manifest mismatch")
    name = receipt["archive"]
    if Path(name).name != name or not name.endswith(".zip"):
        raise ValueError("Invalid archive filename")
    archive = ordinary(manifest.parent / name)
    info = archive.stat()
    if not stat.S_ISREG(info.st_mode) or info.st_size > 4 * 1024 * 1024:
        raise ValueError("Archive must be a bounded regular file")
    if sha(archive.read_bytes()) != receipt["sha256"]:
        raise ValueError("Archive size or hash mismatch")
    payload = verified_contents(archive, entries)
    addons = ordinary(client / "Interface" / "AddOns")
    target = ordinary(addons / "AzerothOperator")
    if not addons.is_dir() or target.exists():
        raise ValueError("Expected an existing AddOns directory and an absent addon")
    with tempfile.TemporaryDirectory(prefix=".azeroth-stage-", dir=addons.parent) as temporary:
        stage = Path(temporary) / "AzerothOperator"
        stage.mkdir()
        for filename, data in payload.items():
            (stage / filename).write_bytes(data)
        if any(sha((stage / filename).read_bytes()) != digest for filename, digest in receipt["files"].items()):
            raise ValueError("Staged addon hash mismatch")
        # On Windows rename fails if a concurrent install has created the target.
        stage.rename(ordinary(target))
    try:
        result = verify(str(client), str(manifest))
    except (OSError, ValueError, TypeError) as error:
        raise ValueError("Addon directory was created but verification failed; inspect before retrying") from error
    if result["status"] != "matches_manifest" or result["verified_files"] != len(FILES):
        raise ValueError("Addon directory was created but verification failed; inspect before retrying")
    return {"status": "installed", "build": expected_build, "version": receipt["version"],
            "verified_files": result["verified_files"], "live_qualified": False,
            "requires_client_restart": True}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--client", required=True)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--expected-build", required=True)
    args = parser.parse_args()
    print(json.dumps(install(args.client, args.manifest, args.expected_build)))
