"""First-time forced-key receiver provisioning on personal pylon; no private key input."""
import fcntl
import os
from pathlib import Path
import re
import shutil
import stat
import sys


if __name__ == "__main__":
    if os.geteuid() != 0 or Path("/etc/hostname").read_text().strip() != "pylon":
        raise SystemExit("Expected personal pylon root")
    image, public_path = sys.argv[1:]
    if not re.fullmatch("sha256:[a-f0-9]{64}", image):
        raise SystemExit("Immutable image required")
    key = Path(public_path).read_text().strip()
    if not re.fullmatch(r"ssh-ed25519 [A-Za-z0-9+/=]+ azeroth-upload", key):
        raise SystemExit("Unexpected public key")
    destination, config = Path("/opt/azeroth-upload"), Path("/etc/azeroth-upload")
    if destination.exists() or config.exists():
        raise SystemExit("Existing receiver: reconcile rather than overwrite")
    authorised = Path("/root/.ssh/authorized_keys")
    descriptor = os.open(authorised, os.O_RDWR | os.O_APPEND | os.O_NOFOLLOW)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or stat.S_IMODE(info.st_mode) & 0o022:
            raise SystemExit("Unsafe authorised keys file")
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with os.fdopen(descriptor, "r+", closefd=False) as handle:
            before = handle.read()
            if key.split()[1] in before:
                raise SystemExit("Key already registered")
            destination.mkdir(mode=0o755)
            config.mkdir(mode=0o700)
            shutil.copyfile(Path(__file__).with_name("receive_upload.py"), destination / "receive_upload.py")
            (config / "image").write_text(image + "\n")
            os.chmod(config / "image", 0o600)
            entry = 'restrict,command="/usr/bin/timeout 70 /usr/bin/python3 /opt/azeroth-upload/receive_upload.py" ' + key + "\n"
            handle.write(("" if not before or before.endswith("\n") else "\n") + entry)
            handle.flush()
            os.fsync(descriptor)
    finally:
        os.close(descriptor)
    print("Restricted upload key registered; live qualification required")
