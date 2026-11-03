"""Bounded, data-only snapshots. Never evaluate Lua or accept tool-supplied paths."""
import hashlib
import json
import re
import os
import tempfile
from datetime import datetime, timezone
from pathlib import Path

from shared.clients import ClientFamily, ClientIdentity

MAX_BYTES = 4 * 1024 * 1024
SECTIONS = frozenset({"equipment", "bags", "talents", "stats", "professions", "quests", "currencies", "achievements", "macros", "bindings", "action_bars", "operation_receipts", "binding_state"})


def validate_snapshot(value):
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise ValueError("Expected snapshot schema_version 1")
    c = value["client"]
    ClientIdentity(ClientFamily(c["family"]), c["build"], c["interface"], c["variant"])
    for key in ("name", "realm"):
        if not isinstance(value["character"].get(key), str) or not value["character"][key].strip():
            raise ValueError("Character name and realm required")
    if value.get("source") not in ("addon", "official_api", "fixture"):
        raise ValueError("Explicit snapshot source required")
    captured = datetime.fromisoformat(value["captured_at"].replace("Z", "+00:00"))
    if captured.tzinfo is None or captured > datetime.now(timezone.utc):
        raise ValueError("Capture time must be timezone-aware and not in the future")
    sections = value.get("sections")
    if not isinstance(sections, dict) or set(sections) - SECTIONS:
        raise ValueError("Unsupported snapshot section")
    return value


class SnapshotStore:
    def __init__(self, root: Path):
        self.root = root.resolve()

    def read(self, snapshot_id: str):
        if not re.fullmatch(r"[a-f0-9]{64}", snapshot_id):
            raise ValueError("Invalid snapshot id")
        path = self.root / (snapshot_id + ".json")
        if path.resolve().parent != self.root:
            raise ValueError("Snapshot path escaped store")
        with path.open("rb") as stream:
            raw = stream.read(MAX_BYTES + 1)
        if len(raw) > MAX_BYTES:
            raise ValueError("Snapshot exceeds size limit")
        if hashlib.sha256(raw).hexdigest() != snapshot_id:
            raise ValueError("Snapshot content hash mismatch")
        return validate_snapshot(json.loads(raw))

    def ingest(self, value):
        validate_snapshot(value)
        raw = json.dumps(value, ensure_ascii=False, sort_keys=True, allow_nan=False).encode("utf-8")
        if len(raw) > MAX_BYTES:
            raise ValueError("Snapshot exceeds size limit")
        snapshot_id = hashlib.sha256(raw).hexdigest()
        self.root.mkdir(parents=True, exist_ok=True)
        path = self.root / (snapshot_id + ".json")
        if path.exists():
            self.read(snapshot_id)
            return snapshot_id
        descriptor, pending = tempfile.mkstemp(prefix=".pending-", dir=self.root)
        try:
            with os.fdopen(descriptor, "wb") as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(pending, path)
        finally:
            Path(pending).unlink(missing_ok=True)
        return snapshot_id

    def inspect(self, snapshot_id, section=None):
        value = self.read(snapshot_id)
        captured = datetime.fromisoformat(value["captured_at"].replace("Z", "+00:00"))
        age = (datetime.now(timezone.utc) - captured).total_seconds()
        result = {k: value[k] for k in ("client", "character", "source", "captured_at")}
        result.update(snapshot_id=snapshot_id, age_seconds=round(age), live=False)
        if section is not None:
            if section not in SECTIONS:
                raise ValueError("Unsupported section")
            result["available"] = section in value["sections"]
            result["data"] = value["sections"].get(section)
        else:
            result["available_sections"] = sorted(value["sections"])
        return result
