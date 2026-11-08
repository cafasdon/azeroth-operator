"""Bounded, opt-in per-user cache of numeric simulator summaries, never requests."""

import hashlib
from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3

if __package__:
    from .sync_exports import ordinary
else:
    from sync_exports import ordinary

MAX_ENTRY_BYTES = 512 * 1024
MAX_ENTRIES = 128
VERSION = 1


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()


def private_windows_path(path):
    """Require this user's ownership and a conservative DACL before SQLite access."""
    import ctypes
    from ctypes import wintypes

    class ACL(ctypes.Structure):
        _fields_ = [("revision", ctypes.c_byte), ("reserved", ctypes.c_byte),
                    ("size", wintypes.WORD), ("count", wintypes.WORD), ("reserved2", wintypes.WORD)]

    class SidAndAttributes(ctypes.Structure):
        _fields_ = [("sid", ctypes.c_void_p), ("attributes", wintypes.DWORD)]

    advapi = ctypes.WinDLL("advapi32", use_last_error=True)
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.GetCurrentProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.LocalFree.argtypes = [ctypes.c_void_p]
    kernel.LocalFree.restype = ctypes.c_void_p
    advapi.OpenProcessToken.argtypes = [wintypes.HANDLE, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
    advapi.GetTokenInformation.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p,
                                          wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
    advapi.ConvertSidToStringSidW.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_wchar_p)]
    advapi.GetNamedSecurityInfoW.argtypes = [wintypes.LPWSTR, ctypes.c_int, wintypes.DWORD,
        ctypes.POINTER(ctypes.c_void_p), ctypes.c_void_p, ctypes.POINTER(ctypes.c_void_p),
        ctypes.c_void_p, ctypes.POINTER(ctypes.c_void_p)]
    advapi.GetNamedSecurityInfoW.restype = wintypes.DWORD
    advapi.GetAce.argtypes = [ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(ctypes.c_void_p)]

    def sid_text(sid):
        text = ctypes.c_wchar_p()
        if not advapi.ConvertSidToStringSidW(sid, ctypes.byref(text)):
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            return text.value
        finally:
            kernel.LocalFree(text)

    token = wintypes.HANDLE()
    if not advapi.OpenProcessToken(kernel.GetCurrentProcess(), 8, ctypes.byref(token)):
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        size = wintypes.DWORD()
        advapi.GetTokenInformation(token, 1, None, 0, ctypes.byref(size))
        buffer = ctypes.create_string_buffer(size.value)
        if not advapi.GetTokenInformation(token, 1, buffer, size, ctypes.byref(size)):
            raise ctypes.WinError(ctypes.get_last_error())
        user = sid_text(ctypes.cast(buffer, ctypes.POINTER(SidAndAttributes)).contents.sid)
    finally:
        kernel.CloseHandle(token)

    owner, dacl, descriptor = ctypes.c_void_p(), ctypes.c_void_p(), ctypes.c_void_p()
    error = advapi.GetNamedSecurityInfoW(str(path), 1, 5, ctypes.byref(owner), None,
                                        ctypes.byref(dacl), None, ctypes.byref(descriptor))
    if error:
        raise ctypes.WinError(error)
    try:
        if not owner or sid_text(owner) != user or not dacl:
            return False
        allowed = {user, "S-1-5-18", "S-1-5-32-544", "S-1-3-4"}
        for index in range(ctypes.cast(dacl, ctypes.POINTER(ACL)).contents.count):
            ace = ctypes.c_void_p()
            if not advapi.GetAce(dacl, index, ctypes.byref(ace)):
                raise ctypes.WinError(ctypes.get_last_error())
            kind = ctypes.c_ubyte.from_address(ace.value).value
            if kind == 1:  # Denials cannot grant another account access.
                continue
            if kind != 0 or sid_text(ace.value + 8) not in allowed:
                return False
        return True
    finally:
        kernel.LocalFree(descriptor)


def user_protected(data, key, decrypt=False):
    """DPAPI binds Windows payloads to this OS user and the exact request key."""
    if os.name != "nt":
        return data
    import ctypes
    from ctypes import wintypes

    class Blob(ctypes.Structure):
        _fields_ = [("size", wintypes.DWORD), ("data", ctypes.POINTER(ctypes.c_ubyte))]

    source_buffer = ctypes.create_string_buffer(data)
    entropy_buffer = ctypes.create_string_buffer(key.encode("ascii"))
    source = Blob(len(data), ctypes.cast(source_buffer, ctypes.POINTER(ctypes.c_ubyte)))
    entropy = Blob(len(key), ctypes.cast(entropy_buffer, ctypes.POINTER(ctypes.c_ubyte)))
    output = Blob()
    crypt = ctypes.WinDLL("crypt32", use_last_error=True)
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    function = crypt.CryptUnprotectData if decrypt else crypt.CryptProtectData
    function.argtypes = [ctypes.POINTER(Blob), ctypes.c_void_p, ctypes.POINTER(Blob),
                         ctypes.c_void_p, ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(Blob)]
    function.restype = wintypes.BOOL
    kernel.LocalFree.argtypes = [ctypes.c_void_p]
    kernel.LocalFree.restype = ctypes.c_void_p
    if not function(ctypes.byref(source), None, ctypes.byref(entropy), None, None, 1, ctypes.byref(output)):
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        return ctypes.string_at(output.data, output.size)
    finally:
        kernel.LocalFree(output.data)


class SimulationCache:
    def __init__(self, root):
        self.root = Path(root) if root is not None else None

    @classmethod
    def for_current_user(cls):
        if os.name == "nt":
            base = Path(os.environ.get("LOCALAPPDATA", ""))
        else:
            base = Path.home() / ".cache"
        if not base.is_absolute():
            return cls(None)
        return cls(base / "AzerothOperator" / "simulation-cache")

    def key(self, engine_hash, request, paired, tank_metrics, healing_metrics):
        return hashlib.sha256(encoded({
            "version": VERSION, "engine": engine_hash.lower(), "request": request,
            "paired": paired, "tank": tank_metrics, "healing": healing_metrics,
        })).hexdigest()

    def connect(self):
        if self.root is None:
            raise ValueError("No private simulation cache location")
        ordinary(self.root).mkdir(parents=True, exist_ok=True, mode=0o700)
        if os.name == "nt" and not private_windows_path(self.root):
            raise ValueError("Simulation cache must be private to this user")
        path = ordinary(self.root / "results.sqlite3")
        # Reject linked journal files as well as the database itself.
        for suffix in ("-journal", "-wal", "-shm"):
            journal = ordinary(Path(str(path) + suffix))
            if os.name == "nt" and journal.exists() and not private_windows_path(journal):
                raise ValueError("Simulation cache journal must be private")
        if os.name == "nt" and path.exists() and not private_windows_path(path):
            raise ValueError("Simulation cache database must be private")
        if os.name != "nt":
            info = self.root.stat()
            if info.st_uid != os.getuid() or info.st_mode & 0o077:
                raise ValueError("Simulation cache must be private to this user")
        connection = sqlite3.connect(path, timeout=1)
        try:
            if os.name != "nt":
                path.chmod(0o600)
            connection.execute("CREATE TABLE IF NOT EXISTS results "
                               "(key TEXT PRIMARY KEY, body BLOB NOT NULL, digest TEXT NOT NULL, "
                               "created INTEGER NOT NULL DEFAULT (unixepoch()))")
        except BaseException:
            connection.close()
            raise
        return connection

    def get(self, key):
        try:
            with closing(self.connect()) as connection, connection:
                row = connection.execute(
                    "SELECT body, digest FROM results WHERE key=? AND length(body)<=?",
                    (key, MAX_ENTRY_BYTES + 16384),
                ).fetchone()
            if row is None:
                return None
            body, digest = row
            if hashlib.sha256(body).hexdigest() != digest:
                return None
            plaintext = user_protected(body, key, decrypt=True)
            if len(plaintext) > MAX_ENTRY_BYTES:
                return None
            result = json.loads(plaintext)
            return result if isinstance(result, dict) else None
        except (OSError, ValueError, TypeError, sqlite3.Error):
            return None

    def put(self, key, summary):
        try:
            body = encoded(summary)
            if len(body) > MAX_ENTRY_BYTES:
                return
            body = user_protected(body, key)
            with closing(self.connect()) as connection, connection:
                connection.execute("INSERT OR REPLACE INTO results (key, body, digest) VALUES (?, ?, ?)",
                                   (key, body, hashlib.sha256(body).hexdigest()))
                connection.execute("DELETE FROM results WHERE key NOT IN "
                                   "(SELECT key FROM results ORDER BY created DESC, rowid DESC LIMIT ?)",
                                   (MAX_ENTRIES,))
        except (OSError, ValueError, TypeError, sqlite3.Error):
            # Caching is optional: disk problems must not prevent a fresh simulation.
            return
