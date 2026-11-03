import json
import os
from pathlib import Path
import subprocess
import tempfile
import sys
import unittest
from unittest.mock import patch
from tools.sync_exports import PASS_SECONDS, discover, exclusive, invoke_sync, load_config, run_pass


class SyncTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.config = dict(binary=self.root / "operator", ssh_config=self.root / "ssh_config",
                           state=self.root / "state", clients=[dict(family="retail", accounts=self.root / "accounts")])

    def export(self, name="account"):
        path = self.root / "accounts" / name / "SavedVariables" / "AzerothOperator.lua"
        path.parent.mkdir(parents=True)
        path.write_text("fixture")
        return path

    def test_missing_export_never_launches_process(self):
        def forbidden(*args, **kwargs):
            self.fail("No export must mean no subprocess")
        result = run_pass(self.config, invoke=forbidden)
        self.assertEqual(result["results"], [{"family": "retail", "status": "waiting_for_export"}])

    def test_discovery_failure_is_reported_without_blocking_other_family(self):
        path = self.export()
        self.config["clients"].append(dict(family="forever", accounts=self.root / "forever"))
        calls = []
        def invoke(argv, **options):
            calls.append(argv)
            return subprocess.CompletedProcess(argv, 0, json.dumps(dict(status="uploaded", snapshot_id="b" * 64)).encode())
        with patch("tools.sync_exports.discover", side_effect=[PermissionError("private path"), [path]]):
            result = run_pass(self.config, invoke=invoke)
        self.assertEqual(result["results"], [
            {"family": "retail", "status": "failed_retry_next_pass"},
            {"family": "forever", "status": "uploaded", "snapshot_id": "b" * 64}])
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][2], "forever")
        self.assertNotIn("private path", json.dumps(result))

    def test_exact_arguments_and_retry_status(self):
        path = self.export()
        calls = []
        def invoke(argv, **options):
            calls.append(argv)
            self.assertEqual(options["timeout"], 65)
            self.assertEqual(options["env"]["AZEROTH_UPLOAD_SSH_CONFIG"], str(self.config["ssh_config"]))
            return subprocess.CompletedProcess(argv, 0, json.dumps(dict(status="unchanged", snapshot_id="a" * 64)).encode())
        result = run_pass(self.config, invoke=invoke)
        self.assertEqual(calls[0], [str(self.config["binary"]), "sync", "retail", str(path), str(self.config["state"] / "receipts")])
        self.assertFalse(result["results"][0]["remote_rechecked"])
        def timeout(*args, **kwargs):
            raise subprocess.TimeoutExpired("fixture", 65)
        self.assertEqual(run_pass(self.config, invoke=timeout)["results"][0]["status"], "failed_retry_next_pass")

    def test_limits_and_only_named_export(self):
        path = self.export()
        (path.parent / "OtherAddon.lua").write_text("not ours")
        self.assertEqual(discover(self.root / "accounts"), [path])
        for index in range(8):
            self.export(str(index))
        with self.assertRaisesRegex(ValueError, "Export limit"):
            discover(self.root / "accounts")

    def test_exclusive_and_deadline(self):
        with exclusive(self.config["state"]):
            with self.assertRaises(OSError):
                with exclusive(self.config["state"]):
                    self.fail("Concurrent pass admitted")
        self.export()
        ticks = iter([0, PASS_SECONDS - 60])
        result = run_pass(self.config, clock=lambda: next(ticks))
        self.assertEqual(result["results"][0]["status"], "deferred_deadline")

    def test_config_rejects_relative_roots_and_duplicate_families(self):
        path = self.root / "config.json"
        config = dict(binary=str(self.config["binary"]), ssh_config=str(self.config["ssh_config"]),
                      state=str(self.config["state"]), clients=[dict(family="retail", accounts="relative")])
        path.write_text(json.dumps(config))
        with self.assertRaises(ValueError):
            load_config(path)
        config["clients"][0]["accounts"] = str(self.root / "accounts")
        config["clients"] *= 2
        path.write_text(json.dumps(config))
        with self.assertRaises(ValueError):
            load_config(path)

    @unittest.skipUnless(os.name == "nt", "Windows process-tree qualification")
    def test_timeout_reaps_actual_child_tree(self):
        import ctypes
        pidfile = self.root / "child.pid"
        code = "import subprocess,sys,time; from pathlib import Path; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); Path(sys.argv[1]).write_text(str(p.pid)); time.sleep(60)"
        with self.assertRaises(subprocess.TimeoutExpired):
            invoke_sync([sys.executable, "-c", code, str(pidfile)], timeout=1, check=False,
                        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                        creationflags=subprocess.CREATE_NO_WINDOW)
        pid = int(pidfile.read_text())
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.OpenProcess.restype = ctypes.c_void_p
        kernel.OpenProcess.argtypes = [ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
        kernel.GetExitCodeProcess.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
        kernel.CloseHandle.argtypes = [ctypes.c_void_p]
        handle = kernel.OpenProcess(0x1000, False, pid)
        if handle:
            try:
                code = ctypes.c_ulong()
                self.assertTrue(kernel.GetExitCodeProcess(handle, ctypes.byref(code)))
                self.assertNotEqual(code.value, 259, "Child process still active")
            finally:
                kernel.CloseHandle(handle)
