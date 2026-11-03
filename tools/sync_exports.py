"""One bounded sync pass over explicit WoW account roots; never interacts with the game."""
import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import tempfile
import time

MAX_EXPORTS = 8
MAX_ACCOUNTS = 128
PASS_SECONDS = 540


def absolute(value):
    path = Path(value)
    if not path.is_absolute():
        raise ValueError("Absolute paths required")
    return path


def ordinary(path):
    """Reject reparse points at every existing path component, including junctions."""
    for part in [path, *path.parents]:
        try:
            info = part.lstat()
        except FileNotFoundError:
            continue
        if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
            raise ValueError("Linked paths are not supported")
    return path


def load_config(path):
    with ordinary(absolute(path)).open("rb") as handle:
        data = handle.read(16385)
    if len(data) > 16384:
        raise ValueError("Configuration too large")
    config = json.loads(data)
    if set(config) != {"binary", "ssh_config", "state", "clients"}:
        raise ValueError("Unexpected configuration fields")
    for name in ("binary", "ssh_config", "state"):
        config[name] = ordinary(absolute(config[name]))
    if not isinstance(config["clients"], list) or not 1 <= len(config["clients"]) <= 3:
        raise ValueError("Expected one to three client roots")
    seen = set()
    for client in config["clients"]:
        if set(client) != {"family", "accounts"} or client["family"] not in {"retail", "classic", "forever"}:
            raise ValueError("Invalid client root")
        if client["family"] in seen:
            raise ValueError("Duplicate client family")
        seen.add(client["family"])
        client["accounts"] = ordinary(absolute(client["accounts"]))
    return config


def discover(root):
    ordinary(root)
    if not root.exists():
        return []
    exports = []
    with os.scandir(root) as entries:
        for index, entry in enumerate(entries):
            if index >= MAX_ACCOUNTS:
                raise ValueError("Account scan limit exceeded")
            account = ordinary(Path(entry.path))
            if not account.is_dir():
                continue
            export = ordinary(account / "SavedVariables" / "AzerothOperator.lua")
            if export.is_file():
                exports.append(export)
                if len(exports) > MAX_EXPORTS:
                    raise ValueError("Export limit exceeded")
    return sorted(exports)


@contextmanager
def exclusive(state):
    ordinary(state)
    state.mkdir(parents=True, exist_ok=True)
    lock = ordinary(state / "sync.lock")
    with lock.open("a+b") as handle:
        if os.name == "nt":
            import msvcrt
            handle.seek(0)
            if not handle.read(1):
                handle.write(b"0")
                handle.flush()
            handle.seek(0)
            msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
        else:
            import fcntl
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield


def invoke_sync(argv, **options):
    """Reap a timed-out process tree, including the transport child."""
    timeout = options.pop("timeout")
    options.pop("check")
    if os.name != "nt":
        options["start_new_session"] = True
    with subprocess.Popen(argv, **options) as child:
        try:
            stdout, _ = child.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            try:
                if os.name == "nt":
                    subprocess.run(
                        [str(Path(os.environ["SystemRoot"]) / "System32" / "taskkill.exe"),
                         "/PID", str(child.pid), "/T", "/F"],
                        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL, timeout=10, check=False,
                        creationflags=subprocess.CREATE_NO_WINDOW,
                    )
                else:
                    os.killpg(child.pid, signal.SIGKILL)
            finally:
                child.kill()
                child.communicate(timeout=5)
            raise
        return subprocess.CompletedProcess(argv, child.returncode, stdout)


def run_pass(config, invoke=invoke_sync, clock=time.monotonic):
    start = clock()
    jobs = []
    results = []
    for client in config["clients"]:
        try:
            exports = discover(client["accounts"])
        except (OSError, ValueError):
            results.append({"family": client["family"], "status": "failed_retry_next_pass"})
            continue
        if not exports:
            results.append({"family": client["family"], "status": "waiting_for_export"})
        jobs.extend((client["family"], path) for path in exports)
    if len(jobs) > MAX_EXPORTS:
        raise ValueError("Total export limit exceeded")
    environment = dict(os.environ, AZEROTH_UPLOAD_SSH_CONFIG=str(config["ssh_config"]))
    for family, export in jobs:
        remaining = PASS_SECONDS - (clock() - start)
        # Give the Rust transport its full 60-second deadline so it reaps SSH itself.
        if remaining < 65:
            results.append({"family": family, "status": "deferred_deadline"})
            continue
        try:
            output = invoke(
                [str(config["binary"]), "sync", family, str(export), str(config["state"] / "receipts")],
                env=environment, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL, timeout=65, check=False,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
            if output.returncode or len(output.stdout) > 4096:
                raise ValueError("Sync failed")
            result = json.loads(output.stdout)
            if not isinstance(result, dict):
                raise ValueError("Expected sync object")
            status = result.get("status")
            if status not in {"uploaded", "unchanged", "waiting_for_export"}:
                raise ValueError("Unexpected sync status")
            item = {"family": family, "status": status}
            if status != "waiting_for_export":
                identifier = result.get("snapshot_id", "")
                if not isinstance(identifier, str) or not re.fullmatch("[a-f0-9]{64}", identifier):
                    raise ValueError("Invalid receipt identifier")
                item["snapshot_id"] = identifier
                if status == "unchanged":
                    item["remote_rechecked"] = False
            results.append(item)
        except (OSError, ValueError, subprocess.TimeoutExpired):
            results.append({"family": family, "status": "failed_retry_next_pass"})
    return {"checked_at": datetime.now(timezone.utc).isoformat(), "live_game_access": False, "results": results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", required=True)
    args = parser.parse_args()
    config = load_config(args.config)
    with exclusive(config["state"]):
        result = run_pass(config)
        destination = ordinary(config["state"] / "status.json")
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=config["state"], delete=False) as handle:
            temporary = Path(handle.name)
            json.dump(result, handle)
        try:
            os.replace(temporary, destination)
        finally:
            temporary.unlink(missing_ok=True)
        print(json.dumps(result))
        return int(any(row["status"] in {"failed_retry_next_pass", "deferred_deadline"} for row in result["results"]))


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, TypeError, KeyError):
        raise SystemExit("Sync pass failed; inspect local configuration and retry")
