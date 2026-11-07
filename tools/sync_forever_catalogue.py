"""Synchronise the pinned WoWSims Forever database to an explicit local file."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
from tools.sync_exports import ordinary

PIN = json.loads((Path(__file__).resolve().parents[1] / "clients/forever/catalogue.lock.json").read_text())
MAX_BYTES = 8 * 1024 * 1024


def verify(data):
    if len(data) > MAX_BYTES or hashlib.sha256(data).hexdigest() != PIN["sha256"]:
        raise ValueError("Forever catalogue checksum or size mismatch")


def sync(destination):
    destination = ordinary(destination.absolute())
    try:
        if not stat.S_ISREG(destination.stat().st_mode):
            raise OSError("Forever catalogue destination must be a regular file")
        with destination.open("rb") as current:
            verify(current.read(MAX_BYTES + 1))
        return "unchanged"
    except (FileNotFoundError, ValueError):
        pass

    url = f"https://raw.githubusercontent.com/{PIN['provider']}/{PIN['revision']}/assets/database/db.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".forever-sync-", dir=destination.parent) as stage:
        temporary = Path(stage) / "catalogue.json"
        subprocess.run(
            ["curl", "--disable", "--fail", "--silent", "--show-error",
             "--proto", "=https", "--max-time", "30", "--max-filesize", str(MAX_BYTES),
             "--output", str(temporary), url],
            timeout=35, check=True,
        )
        with temporary.open("r+b") as downloaded:
            verify(downloaded.read(MAX_BYTES + 1))
            os.fsync(downloaded.fileno())
        os.replace(temporary, destination)
    return "synced"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps({"status": sync(args.output), **PIN}))


if __name__ == "__main__":
    main()
