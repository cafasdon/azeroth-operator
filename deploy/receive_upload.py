"""Forced SSH command on personal pylon. No paths, commands or destinations from client."""
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import sys
import uuid

STORE = "/var/lib/azeroth-operator/rodrigo/snapshots"


def command(image, name):
    if not re.fullmatch("sha256:[a-f0-9]{64}", image) or not re.fullmatch("azeroth-upload-[a-f0-9]{32}", name):
        raise ValueError("Invalid receiver configuration")
    return ["lxc-attach", "-n", "290", "--", "docker", "run", "--rm", "-i",
            "--name", name, "--network", "none", "--read-only", "--cap-drop", "ALL",
            "--security-opt", "no-new-privileges", "--memory", "128m", "--cpus", "1",
            "-v", STORE + ":/data", image, "import-stdin"]


def receive():
    if os.environ.get("SSH_ORIGINAL_COMMAND") or os.environ.get("SSH_TTY"):
        raise ValueError("Only snapshot upload is permitted")
    if Path("/etc/hostname").read_text().strip() != "pylon":
        raise ValueError("Wrong receiver host")
    descriptor = os.open("/etc/azeroth-upload/image", os.O_RDONLY | os.O_NOFOLLOW)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or stat.S_IMODE(info.st_mode) & 0o022:
            raise ValueError("Unsafe image configuration")
        with os.fdopen(descriptor, "r", closefd=False) as handle:
            image = handle.read(128).strip()
    finally:
        os.close(descriptor)
    name = "azeroth-upload-" + uuid.uuid4().hex
    args = command(image, name)
    # The SSH forced command must also use timeout(1), covering child cleanup.
    signal.alarm(30)
    payload = sys.stdin.buffer.read(4 * 1024 * 1024 + 1)
    signal.alarm(0)
    if not payload or len(payload) > 4 * 1024 * 1024:
        raise ValueError("Invalid upload size")
    try:
        result = subprocess.run(args, input=payload, capture_output=True, timeout=25)
        if result.returncode or not re.fullmatch(b"[a-f0-9]{64}\n", result.stdout):
            raise ValueError("Snapshot import failed")
        sys.stdout.buffer.write(result.stdout)
    finally:
        subprocess.run(["pct", "exec", "290", "--", "docker", "rm", "-f", name],
                       capture_output=True, timeout=10, check=False)


if __name__ == "__main__":
    try:
        receive()
    except Exception:
        print("Upload rejected or incomplete", file=sys.stderr)
        raise SystemExit(1) from None
