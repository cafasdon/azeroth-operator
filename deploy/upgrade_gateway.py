"""Replace only the dedicated Azeroth reader profile, retaining rollback evidence."""
import argparse
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from tools.gateway_profile import profile, STORE


def run(*args):
    return subprocess.run(args, check=True, timeout=90, stdout=subprocess.DEVNULL)


def check_current(value, expected):
    if value.get("id") != "azeroth-operator" or len(value.get("servers", [])) != 1:
        raise ValueError("Unexpected profile identity or server count")
    entry = value["servers"][0]
    server = entry["snapshot"]["server"]
    if entry["image"] != expected or server["image"] != expected:
        raise ValueError("Current image changed; reconcile before upgrade")
    if server["volumes"] != [STORE + ":/data:ro"]:
        raise ValueError("Unexpected store mount")
    if server["env"] != [{"name": "AZEROTH_DATA_DIR", "value": "/data"}]:
        raise ValueError("Unexpected server environment")


def wait_ready():
    deadline = time.monotonic() + 30
    while True:
        try:
            with socket.create_connection(("127.0.0.1", 8823), timeout=1):
                return
        except OSError:
            if time.monotonic() >= deadline:
                raise TimeoutError("Dedicated reader did not listen") from None
            time.sleep(0.2)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expected-image", required=True)
    parser.add_argument("--image", required=True)
    args = parser.parse_args()
    for image in [args.expected_image, args.image]:
        if not re.fullmatch(r"sha256:[a-f0-9]{64}", image):
            raise SystemExit("Immutable images required")
    if os.geteuid() != 0 or Path("/etc/hostname").read_text().strip() != "mcp-gateway":
        raise SystemExit("Expected root on personal CT290")
    run("docker", "image", "inspect", args.image)
    evidence = Path(tempfile.mkdtemp(prefix="azeroth-reader-upgrade-", dir="/var/tmp"))
    before = evidence / "before.json"
    after = evidence / "after.json"
    candidate = evidence / "candidate.json"
    run("docker", "mcp", "profile", "export", "azeroth-operator", str(before))
    check_current(json.loads(before.read_text()), args.expected_image)
    candidate.write_text(json.dumps(profile(args.image)))
    try:
        run("docker", "mcp", "profile", "import", str(candidate))
        run("docker", "mcp", "profile", "export", "azeroth-operator", str(after))
        check_current(json.loads(after.read_text()), args.image)
        if json.loads(after.read_text())["servers"][0]["tools"] != profile(args.image)["servers"][0]["tools"]:
            raise ValueError("Tool allowlist mismatch")
        run("systemctl", "restart", "azeroth-mcp.service")
        run("systemctl", "is-active", "--quiet", "azeroth-mcp.service")
        wait_ready()
        run("python3", str(Path(__file__).with_name("probe_gateway.py")), "--require-current-inspection")
        run("systemctl", "enable", "azeroth-mcp.service")
        run("systemctl", "is-enabled", "--quiet", "azeroth-mcp.service")
    except BaseException:
        run("systemctl", "disable", "--now", "azeroth-mcp.service")
        print("Reader stopped after failed verification. Previous profile retained as evidence.", file=sys.stderr)
        raise
    print(json.dumps({"reader_image": args.image, "evidence": str(evidence), "loopback_probe": "passed"}))
