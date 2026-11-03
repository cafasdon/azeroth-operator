"""Remove only the exact synthetic gateway qualification snapshot on CT290."""
import hashlib
import json
from pathlib import Path

IDENTITY = "ef93cc06b211e8e3c728f4d927ddada4133b6b78aae4ebe050a9a6968fa3006c"

if __name__ == "__main__":
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
