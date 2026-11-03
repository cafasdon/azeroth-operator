"""First installation only, run as root on personal CT290 after profile qualification."""
import os
from pathlib import Path
import secrets
import shutil
import subprocess


def run(*args):
    subprocess.run(args, check=True, timeout=90)


if __name__ == "__main__":
    if os.geteuid() != 0 or Path("/etc/hostname").read_text().strip() != "mcp-gateway":
        raise SystemExit("Expected root on personal mcp-gateway")
    unit = Path("/etc/systemd/system/azeroth-mcp.service")
    config = Path("/etc/azeroth-operator")
    installed = Path("/opt/azeroth-operator")
    if any(path.exists() for path in (unit, config, installed)):
        raise SystemExit("Existing installation; reconcile instead of overwriting")
    source = Path(__file__).resolve().parent
    config.mkdir(mode=0o700)
    installed.mkdir(mode=0o755)
    descriptor = os.open(config / "gateway.env", os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    with os.fdopen(descriptor, "w") as handle:
        handle.write("MCP_GATEWAY_AUTH_TOKEN=" + secrets.token_hex(32) + "\n")
    shutil.copyfile(source / "check_gateway_auth.py", installed / "check_gateway_auth.py")
    shutil.copyfile(source / "azeroth-mcp.service", unit)
    run("systemctl", "daemon-reload")
    try:
        run("systemctl", "start", "azeroth-mcp.service")
        run("systemctl", "is-active", "--quiet", "azeroth-mcp.service")
    except BaseException:
        subprocess.run(["systemctl", "stop", "azeroth-mcp.service"], timeout=45, check=False)
        raise
    print("Loopback service started; authentication and HTTP qualification still required")
