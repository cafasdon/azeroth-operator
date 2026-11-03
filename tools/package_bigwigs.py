"""Pinned BigWigs build in a fresh staging repo and network-disabled Linux namespace."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import signal
import stat
import subprocess
import tempfile
import urllib.request
import zipfile
from tools.package_addon import ROOT, render_entries


def sha(data):
    return hashlib.sha256(data).hexdigest()


def get_packager(cache, lock):
    if (lock.get("repository") != "https://github.com/BigWigsMods/packager"
            or not re.fullmatch("[a-f0-9]{40}", lock.get("commit", ""))
            or not re.fullmatch("[a-f0-9]{64}", lock.get("release_sh_sha256", ""))):
        raise ValueError("Invalid packager pin")
    cache.mkdir(parents=True, exist_ok=True)
    script = cache / (lock["commit"] + "-release.sh")
    if not script.exists():
        url = f'https://raw.githubusercontent.com/BigWigsMods/packager/{lock["commit"]}/release.sh'
        with urllib.request.urlopen(url, timeout=30) as response:
            data = response.read(1024 * 1024 + 1)
        if len(data) > 1024 * 1024 or sha(data) != lock["release_sh_sha256"]:
            raise ValueError("Downloaded packager checksum mismatch")
        script.write_bytes(data)
    if script.is_symlink() or script.stat().st_size > 1024 * 1024 or sha(script.read_bytes()) != lock["release_sh_sha256"]:
        raise ValueError("Packager checksum mismatch")
    return script.resolve()


def verified_contents(archive, expected):
    with zipfile.ZipFile(archive) as package:
        names = [entry.filename for entry in package.infolist() if not entry.is_dir()]
        allowed = {"AzerothOperator/" + name for name in expected}
        if len(names) != len(allowed) or set(names) != allowed:
            raise ValueError("Unexpected or duplicate packaged files")
        for entry in package.infolist():
            if entry.filename not in allowed and entry.filename != "AzerothOperator/":
                raise ValueError("Unexpected archive path")
            if stat.S_ISLNK(entry.external_attr >> 16):
                raise ValueError("Archive contains a symlink")
            if entry.file_size > 1024 * 1024:
                raise ValueError("Oversized package entry")
        actual = {name: package.read("AzerothOperator/" + name) for name in expected}
        if actual != expected:
            raise ValueError("Packager changed source contents unexpectedly")
        return actual


def build(family, variant, interface, version, destination):
    if os.name != "posix":
        raise ValueError("Run in Linux/WSL with unshare network isolation")
    entries = render_entries(family, variant, interface, version)
    lock = json.loads((ROOT / "tools" / "packager.lock.json").read_text())
    destination = Path(destination).resolve()
    destination.mkdir(parents=True, exist_ok=True)
    script = get_packager(ROOT / "dist" / "packager-cache", lock)
    with tempfile.TemporaryDirectory(prefix="bigwigs-", dir=destination) as temporary:
        workspace = Path(temporary)
        stage, output, home = (workspace / name for name in ("stage", "output", "home"))
        for path in (stage, output, home):
            path.mkdir()
        for name, data in entries.items():
            (stage / name).write_bytes(data)
        (stage / "CHANGELOG.txt").write_text("Development-only export package; not live-qualified.\n")
        (stage / ".pkgmeta").write_text(
            "package-as: AzerothOperator\nmanual-changelog: CHANGELOG.txt\n"
            "ignore:\n  - CHANGELOG.txt\n  - .pkgmeta\n")
        environment = {"PATH": "/usr/bin:/bin", "HOME": str(home), "LC_ALL": "C", "TZ": "UTC",
                       "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
                       "GIT_TERMINAL_PROMPT": "0", "GIT_AUTHOR_DATE": "2020-01-01T00:00:00Z",
                       "GIT_COMMITTER_DATE": "2020-01-01T00:00:00Z"}
        def run(*command):
            with subprocess.Popen(command, cwd=stage, env=environment, start_new_session=True,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
                try:
                    stdout, stderr = process.communicate(timeout=90)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.communicate(timeout=5)
                    raise
                if process.returncode:
                    raise subprocess.CalledProcessError(process.returncode, command, stdout, stderr)
                return subprocess.CompletedProcess(command, process.returncode, stdout, stderr)
        run("git", "init", "-q")
        run("git", "add", *entries, "CHANGELOG.txt", ".pkgmeta")
        identity = ["git", "-c", "user.name=Azeroth Build", "-c", "user.email=build@invalid.example"]
        run(*identity, "commit", "-q", "-m", "Stage isolated addon source")
        run(*identity, "tag", "-a", version, "-m", "Development package")
        # No credentials, dotenv, shell profiles, external downloads or uploads.
        run("unshare", "--user", "--map-root-user", "--net", "bash", "--noprofile", "--norc",
            str(script), "-d", "-e", "-l", "-u", "-t", str(stage), "-r", str(output))
        archives = list(output.glob("*.zip"))
        if len(archives) != 1:
            raise ValueError("Expected exactly one upstream ZIP")
        actual = verified_contents(archives[0], entries)
        upstream_sha = sha(archives[0].read_bytes())
        archive = destination / f"AzerothOperator-{version}-{family}-{variant}.zip"
        # Normalise ZIP metadata only, after verifying all actual upstream payload bytes.
        with zipfile.ZipFile(archive, "w") as package:
            for name, data in sorted(actual.items()):
                info = zipfile.ZipInfo("AzerothOperator/" + name, (2020, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                package.writestr(info, data)
        verified_contents(archive, entries)
        receipt = {"archive": archive.name, "sha256": sha(archive.read_bytes()),
                   "family": family, "variant": variant, "interface": interface, "version": version,
                   "qualification": "development_only", "packager": lock,
                   "upstream_zip_sha256": upstream_sha, "network_isolated": True,
                   "uploads_enabled": False, "metadata_normalised": True,
                   "files": {name: sha(data) for name, data in actual.items()}}
        archive.with_suffix(".manifest.json").write_text(json.dumps(receipt, indent=2) + "\n")
        return receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--family", required=True, choices=["retail", "classic", "forever"])
    parser.add_argument("--variant", required=True)
    parser.add_argument("--interface", required=True, type=int)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist" / "bigwigs")
    args = parser.parse_args()
    try:
        print(json.dumps(build(args.family, args.variant, args.interface, args.version, args.output), indent=2))
    except subprocess.CalledProcessError as error:
        # The subprocess environment and staging tree contain no account data or secrets.
        raise SystemExit(error.stderr.decode(errors="replace")[-4000:] + error.stdout.decode(errors="replace")[-4000:])
