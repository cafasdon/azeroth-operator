import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from tools.verify_source import verify


class SourceVerificationTests(unittest.TestCase):
    def test_failure_stops_before_protocol_and_overrides_stale_binary(self):
        with tempfile.TemporaryDirectory() as temp:
            calls = []
            def invoke(command, **options):
                calls.append(command)
                self.assertEqual(options["timeout"], 600)
                self.assertEqual(options["env"]["CARGO_TARGET_DIR"], str(Path(temp).resolve() / "target"))
                self.assertTrue(options["env"]["AZEROTH_TEST_BINARY"].startswith(str(Path(temp).resolve())))
                return subprocess.CompletedProcess(command, 9 if command[1] == "build" else 0)
            with patch.dict(os.environ, {"AZEROTH_TEST_BINARY": "stale-binary", "CARGO_TARGET_DIR": "elsewhere"}):
                with self.assertRaisesRegex(RuntimeError, "build"):
                    verify(temp, invoke)
            self.assertEqual([call[1] for call in calls], ["fmt", "build"])
