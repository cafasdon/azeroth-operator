"""Fail closed if the service's dedicated token file/environment is unsafe."""
import hmac
import os
from pathlib import Path
import re
import stat

PATH = Path("/etc/azeroth-operator/gateway.env")


def validate(contents, environment):
    match = re.fullmatch(r"MCP_GATEWAY_AUTH_TOKEN=([0-9a-f]{64})\n", contents)
    if match is None or not hmac.compare_digest(match[1], environment):
        raise ValueError("Invalid dedicated gateway authentication")


if __name__ == "__main__":
    descriptor = os.open(PATH, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        info = os.fstat(descriptor)
        if info.st_uid != 0 or stat.S_IMODE(info.st_mode) != 0o600 or not stat.S_ISREG(info.st_mode):
            raise SystemExit("Unsafe gateway authentication file")
        with os.fdopen(descriptor, "r", closefd=False) as handle:
            validate(handle.read(256), os.environ.get("MCP_GATEWAY_AUTH_TOKEN", ""))
    finally:
        os.close(descriptor)
