"""Remove only the exact synthetic gateway qualification snapshot on CT290."""
import hashlib
import argparse
import json
import re
from pathlib import Path

IDENTITY = "ef93cc06b211e8e3c728f4d927ddada4133b6b78aae4ebe050a9a6968fa3006c"

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", default=IDENTITY)
    args = parser.parse_args()
    if not re.fullmatch("[a-f0-9]{64}", args.snapshot):
        raise SystemExit("Invalid fixture identity")
    IDENTITY = args.snapshot
    path = Path("/var/lib/azeroth-operator/rodrigo/snapshots") / (IDENTITY + ".json")
    if path.is_symlink():
        raise SystemExit("Refusing symlink")
    raw = path.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == IDENTITY
    snapshot = json.loads(raw)
    assert snapshot["source"] == "fixture"
    assert snapshot["character"] == {"name": "GatewayFixture", "realm": "Synthetic"}
    path.unlink()
    print("Synthetic qualification snapshot removed")
