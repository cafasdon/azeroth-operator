"""Bounded local source verification; never uploads, installs or publishes."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
from tools.sync_exports import invoke_sync

ROOT = Path(__file__).resolve().parents[1]


def verify(root=ROOT, invoke=invoke_sync):
    root = Path(root).resolve(strict=True)
    target = root / "target"
    binary = target / "debug" / ("azeroth-operator.exe" if os.name == "nt" else "azeroth-operator")
    env = {**os.environ, "CARGO_TARGET_DIR": str(target),
           "AZEROTH_TEST_BINARY": str(binary), "CARGO_TERM_COLOR": "never"}
    steps = [
        ("format", ["cargo", "fmt", "--all", "--", "--check"]),
        ("build", ["cargo", "build", "--locked"]),
        ("rust_tests", ["cargo", "test", "--locked"]),
        ("clippy", ["cargo", "clippy", "--all-targets", "--locked", "--", "-D", "warnings"]),
        ("python_protocol_and_tools", [sys.executable, "-m", "unittest", "discover", "-s", "tests", "-q"]),
    ]
    passed = []
    for name, command in steps:
        print(f"Verifying {name}", flush=True)
        result = invoke(command, cwd=root, env=env, stdin=subprocess.DEVNULL,
                        timeout=600, check=False)
        if result.returncode:
            raise RuntimeError(f"Verification failed: {name} (exit {result.returncode})")
        passed.append(name)
    return {"passed": passed, "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            "live_qualified": False, "deployed": False}


if __name__ == "__main__":
    try:
        print(json.dumps(verify()))
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
