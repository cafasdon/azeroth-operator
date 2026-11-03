"""Verify packaged bytes and execute each generated adapter through the Lua fixture."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
from tools.package_addon import render_entries
from tools.package_bigwigs import sha, verified_contents


def verify(directory, verifier):
    directory = Path(directory).resolve(strict=True)
    verifier = Path(verifier).resolve(strict=True)
    manifests = sorted(directory.glob("*.manifest.json"))
    if not 1 <= len(manifests) <= 12:
        raise ValueError("Expected one to twelve package manifests")
    results = []
    for path in manifests:
        receipt = json.loads(path.read_text())
        name = receipt["archive"]
        if Path(name).name != name or not name.endswith(".zip"):
            raise ValueError("Invalid archive name")
        archive = directory / name
        if archive.is_symlink() or sha(archive.read_bytes()) != receipt["sha256"]:
            raise ValueError("Archive checksum mismatch")
        entries = render_entries(receipt["family"], receipt["variant"], receipt["interface"], receipt["version"])
        actual = verified_contents(archive, entries)
        if receipt["files"] != {name: sha(data) for name, data in actual.items()}:
            raise ValueError("File manifest mismatch")
        with tempfile.TemporaryDirectory(prefix="lua-fixture-", dir=directory) as temporary:
            stage = Path(temporary)
            for name, data in actual.items():
                (stage / name).write_bytes(data)
            run = subprocess.run([str(verifier), str(stage), receipt["family"], receipt["variant"], str(receipt["interface"])],
                                 timeout=30, check=True, capture_output=True)
            result = json.loads(run.stdout)
        if result.get("live_qualified") is not False or result.get("packaged_lua51_roundtrip") is not True:
            raise ValueError("Invalid fixture result")
        results.append({**result, "archive": receipt["archive"], "sha256": receipt["sha256"]})
    return results


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--verifier", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(verify(args.directory, args.verifier), indent=2))
