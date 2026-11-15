"""Bounded, file-incremental cache for two pinned public reference repositories.

Changed upstream files are downloaded whole; this is not an upstream row-delta API.
Each cache holds one active lock. Private character history does not belong here.
"""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import time
import urllib.request

DEFAULT_MAX = 256 * 1024 * 1024
DEFAULT_RESERVE = 64 * 1024 * 1024
CHUNK = 64 * 1024
MARKER = "reference-cache.json"
FORMAT = "azeroth-public-reference-v1"
HEX = re.compile(r"[0-9a-f]{64}\Z")


def encode(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def ordinary(path):
    """Reject links, junctions, and other reparse points, including ancestors."""
    for part in (path, *path.parents):
        if part.is_symlink() or (part.exists() and
                getattr(part.lstat(), "st_file_attributes", 0) & 0x400):
            raise ValueError(f"Linked path rejected: {part}")
    return path


def spec(value):
    if not isinstance(value, dict):
        raise ValueError("Expected a lock object")
    provider, revision = value.get("provider"), value.get("revision")
    if provider not in ("Questie/QuestieDB", "wowsims/forever"):
        raise ValueError("Unsupported public provider")
    if not isinstance(revision, str) or not re.fullmatch("[0-9a-f]{40}", revision):
        raise ValueError("An immutable commit revision is required")
    files = value.get("files", value.get("source_files"))
    if files is None and provider == "wowsims/forever":
        files = {"assets/database/db.json": value.get("sha256")}
    if not isinstance(files, dict) or not files or len(files) > 4096:
        raise ValueError("Expected 1 to 4096 source files")
    for name, digest in files.items():
        if (not isinstance(name, str) or len(name) > 240 or
                any(not re.fullmatch(r"[A-Za-z0-9_-][A-Za-z0-9_.-]*", p)
                    or p.endswith(".") for p in name.split("/"))):
            raise ValueError("Unsafe source path")
        if not isinstance(digest, str) or not HEX.fullmatch(digest):
            raise ValueError("Invalid source SHA-256")
    if provider == "wowsims/forever" and set(files) != {"assets/database/db.json"}:
        raise ValueError("Unsupported catalogue source path")
    result = {"provider": provider, "revision": revision, "files": dict(sorted(files.items()))}
    if len(encode(result)) > 1024 * 1024:
        raise ValueError("Source manifest exceeds 1 MiB")
    return result


def load_lock(path):
    with ordinary(Path(path)).open("rb") as stream:
        data = stream.read(1024 * 1024 + 1)
    if len(data) > 1024 * 1024:
        raise ValueError("Lock exceeds 1 MiB")
    return spec(json.loads(data))


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError("Reference downloads cannot redirect")


def download(url):
    # The only caller constructs this URL from the validated provider/path/commit.
    return urllib.request.build_opener(NoRedirect()).open(url, timeout=30)


class ReferenceCache:
    def __init__(self, root, max_bytes=None, reserve_bytes=None):
        self.root = ordinary(Path(root))
        if not self.root.is_absolute() or self.root == Path(self.root.anchor):
            raise ValueError("Cache must have an explicit absolute non-root path")
        if ".." in self.root.parts:
            raise ValueError("Cache path cannot contain parent traversal")
        for amount in (max_bytes, reserve_bytes):
            if amount is not None and (type(amount) is not int or amount < 0):
                raise ValueError("Storage limits must be non-negative integers")
        self.root.mkdir(parents=True, exist_ok=True)
        marker = self.root / MARKER
        if not any(self.root.iterdir()):
            value = {"format": FORMAT, "max_bytes": DEFAULT_MAX if max_bytes is None else max_bytes,
                     "reserve_bytes": DEFAULT_RESERVE if reserve_bytes is None else reserve_bytes}
            data = encode(value)
            if len(data) > value["max_bytes"] or shutil.disk_usage(self.root).free < len(data) + value["reserve_bytes"]:
                raise ValueError("Insufficient cache quota or free disk space")
            with marker.open("xb") as stream:
                stream.write(data)
            (self.root / "objects").mkdir()
        ordinary(marker)
        if not marker.is_file() or marker.stat().st_size > 1024:
            raise ValueError("Missing or invalid ownership marker")
        value = json.loads(marker.read_bytes())
        if (set(value) != {"format", "max_bytes", "reserve_bytes"} or value["format"] != FORMAT
                or any(type(value[k]) is not int or value[k] < 0 for k in ("max_bytes", "reserve_bytes"))):
            raise ValueError("Invalid ownership marker")
        self.max_bytes, self.reserve_bytes = value["max_bytes"], value["reserve_bytes"]
        if ((max_bytes is not None and max_bytes != self.max_bytes) or
                (reserve_bytes is not None and reserve_bytes != self.reserve_bytes)):
            raise ValueError("Existing cache limits cannot change implicitly; use a new cache")
        self._inspect()

    def _inspect(self):
        ordinary(self.root)
        marker = ordinary(self.root / MARKER)
        expected = {"format": FORMAT, "max_bytes": self.max_bytes, "reserve_bytes": self.reserve_bytes}
        if (not marker.is_file() or marker.stat().st_size > 1024 or
                json.loads(marker.read_bytes()) != expected):
            raise ValueError("Missing or changed ownership marker")
        total, objects = 0, {}
        allowed = {MARKER, "active.json", ".pending.json", ".pending-blob", "objects", ".lock"}
        for entry in self.root.iterdir():
            ordinary(entry)
            if entry.name not in allowed:
                raise ValueError(f"Unknown cache entry: {entry.name}")
            if entry.name == "objects":
                if not entry.is_dir():
                    raise ValueError("Invalid object directory")
                for blob in entry.iterdir():
                    ordinary(blob)
                    if not HEX.fullmatch(blob.name) or not stat.S_ISREG(blob.stat().st_mode):
                        raise ValueError("Unknown object entry")
                    objects[blob.name] = blob.stat().st_size
                total += sum(objects.values())
            elif not stat.S_ISREG(entry.stat().st_mode):
                raise ValueError("Non-regular cache file")
            else:
                if entry.name == ".lock" and (entry.stat().st_nlink != 1 or entry.stat().st_size > 1):
                    raise ValueError("Invalid cache lock file")
                total += entry.stat().st_size
        if not (self.root / "objects").is_dir() or total > self.max_bytes:
            raise ValueError("Invalid cache layout or storage quota exceeded")
        for digest in objects:
            with (self.root / "objects" / digest).open("rb") as stream:
                if hashlib.file_digest(stream, "sha256").hexdigest() != digest:
                    raise ValueError("Corrupt cached object")
        active = self.root / "active.json"
        current = None
        if active.exists():
            if active.stat().st_size > 1024 * 1024:
                raise ValueError("Oversized active manifest")
            value = json.loads(active.read_bytes())
            current = spec(value)
            if current != value or not set(current["files"].values()) <= objects.keys():
                raise ValueError("Invalid or incomplete active manifest")
        return current, objects, total

    @contextmanager
    def _guard(self):
        _, _, total = self._inspect()
        lock = self.root / ".lock"
        if not lock.exists():
            self._room(total, 1)
        # A stable inode plus an OS-held lock survives crashes without a stale lease.
        with os.fdopen(os.open(lock, os.O_CREAT | os.O_RDWR, 0o600), "r+b") as stream:
            info = os.fstat(stream.fileno())
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > 1:
                raise ValueError("Invalid cache lock file")
            try:
                if os.name == "nt":
                    import msvcrt
                    msvcrt.locking(stream.fileno(), msvcrt.LK_NBLCK, 1)
                else:
                    import fcntl
                    fcntl.flock(stream.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as error:
                raise ValueError("Cache is busy") from error
            if info.st_size == 0:
                self._room(total, 1)
                stream.write(b"\0")
                stream.flush()
            yield
        # Closing the handle releases the OS lock, including on abnormal exit.

    def _room(self, total, extra):
        if total + extra > self.max_bytes:
            raise ValueError("Cache quota exceeded, including staging and metadata")
        if shutil.disk_usage(self.root).free < self.reserve_bytes + extra:
            raise ValueError("Free disk reserve would be exceeded")

    def _remove(self, path):
        ordinary(path)
        if not stat.S_ISREG(path.stat().st_mode) or path.parent not in (self.root, self.root / "objects"):
            raise ValueError("Refusing removal outside owned regular cache files")
        size = path.stat().st_size
        path.unlink()
        return size

    def sync(self, lock, *, fetch=download, seed=None):
        target = load_lock(lock) if isinstance(lock, (str, Path)) else spec(lock)
        with self._guard():
            current, objects, total = self._inspect()
            if current and current["provider"] != target["provider"]:
                raise ValueError("A cache cannot switch public providers")
            pending, partial = self.root / ".pending.json", self.root / ".pending-blob"
            for path in (pending, partial):
                if path.exists():
                    total -= self._remove(path)
            report = {k: 0 for k in ("downloaded_files", "downloaded_bytes", "seeded_files", "seeded_bytes",
                                     "reused_files", "reused_bytes", "removed_files", "removed_bytes")}
            keep = set(target["files"].values()) | (set(current["files"].values()) if current else set())
            for digest in objects.keys() - keep:
                size = self._remove(self.root / "objects" / digest)
                total -= size
                del objects[digest]
                report["removed_files"] += 1
                report["removed_bytes"] += size
            if target == current:
                report["reused_files"] = len(target["files"])
                report["reused_bytes"] = sum(objects[digest] for digest in target["files"].values())
                return {**report, **self._status(current, objects, total)}
            created = []
            promoted = False
            try:
                manifest = encode(target)
                self._room(total, len(manifest))
                with pending.open("xb") as stream:
                    stream.write(manifest)
                    stream.flush()
                    os.fsync(stream.fileno())
                total += len(manifest)
                source = ordinary(Path(seed)) if seed is not None else None
                if source and not source.is_dir() and len(target["files"]) != 1:
                    raise ValueError("A single seed file needs a single-file lock")
                for name, digest in target["files"].items():
                    if digest in objects:
                        report["reused_files"] += 1
                        report["reused_bytes"] += objects[digest]
                        continue
                    url = f"https://raw.githubusercontent.com/{target['provider']}/{target['revision']}/{name}"
                    if source:
                        candidate = ordinary(source / name if source.is_dir() else source)
                        if not stat.S_ISREG(candidate.stat().st_mode):
                            raise ValueError("Seed must be a regular file")
                        incoming = candidate.open("rb")
                    else:
                        incoming = fetch(url)
                    size, checksum, deadline = 0, hashlib.sha256(), time.monotonic() + 120
                    with incoming as reader, partial.open("xb") as writer:
                        while True:
                            if time.monotonic() > deadline:
                                raise ValueError("Reference download exceeded its time limit")
                            chunk = reader.read(CHUNK)
                            if not chunk:
                                break
                            self._room(total, len(chunk))
                            writer.write(chunk)
                            checksum.update(chunk)
                            size += len(chunk)
                            total += len(chunk)
                        writer.flush()
                        os.fsync(writer.fileno())
                    if checksum.hexdigest() != digest:
                        raise ValueError(f"Source checksum mismatch: {name}")
                    blob = self.root / "objects" / digest
                    os.replace(partial, blob)
                    created.append(blob)
                    objects[digest] = size
                    kind = "seeded" if source else "downloaded"
                    report[kind + "_files"] += 1
                    report[kind + "_bytes"] += size
                self._inspect()
                os.replace(pending, self.root / "active.json")
                promoted = True
                keep = set(target["files"].values())
                for digest in objects.keys() - keep:
                    report["removed_bytes"] += self._remove(self.root / "objects" / digest)
                    report["removed_files"] += 1
                _, objects, total = self._inspect()
                return {**report, **self._status(target, objects, total)}
            finally:
                for path in (pending, partial, *(created if not promoted else [])):
                    if path.exists():
                        self._remove(path)

    def _status(self, current, objects, total):
        return {"provider": current["provider"] if current else None,
                "revision": current["revision"] if current else None,
                "files": current["files"] if current else {}, "object_count": len(objects),
                "cache_bytes": total, "max_bytes": self.max_bytes, "reserve_bytes": self.reserve_bytes}

    def status(self):
        with self._guard():
            return self._status(*self._inspect())

    def query(self, name):
        with self._guard():
            current, objects, _ = self._inspect()
            if not isinstance(name, str) or not current or name not in current["files"]:
                raise ValueError("Unknown source file")
            digest = current["files"][name]
            return {"name": name, "sha256": digest, "bytes": objects[digest], "revision": current["revision"]}

    def read_file(self, name):
        with self._guard():
            current, _, _ = self._inspect()
            if not isinstance(name, str) or not current or name not in current["files"]:
                raise ValueError("Unknown source file")
            return (self.root / "objects" / current["files"][name]).read_bytes()

    @contextmanager
    def snapshot(self, lock):
        """Yield verified object paths for an exact pin, under one cache lock.

        Read the paths only inside this context. Do not modify the objects.
        """
        expected = load_lock(lock) if isinstance(lock, (str, Path)) else spec(lock)
        with self._guard():
            current, _, _ = self._inspect()
            if current != expected:
                raise ValueError("Reference cache does not match source pin")
            yield {name: self.root / "objects" / digest
                   for name, digest in current["files"].items()}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("sync", "status", "query", "read-file"))
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--max-bytes", type=int)
    parser.add_argument("--reserve-bytes", type=int)
    parser.add_argument("--lock", type=Path)
    parser.add_argument("--seed", type=Path)
    parser.add_argument("--name")
    args = parser.parse_args(argv)
    try:
        cache = ReferenceCache(args.cache, args.max_bytes, args.reserve_bytes)
        if args.command == "sync":
            if args.lock is None:
                raise ValueError("sync needs --lock")
            result = cache.sync(args.lock, seed=args.seed)
        elif args.command == "status":
            result = cache.status()
        elif args.command == "query":
            result = cache.query(args.name)
        else:
            sys.stdout.buffer.write(cache.read_file(args.name))
            return 0
        print(json.dumps(result, sort_keys=True))
        return 0
    except (ValueError, OSError, TypeError) as error:
        print(f"reference-sync: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
