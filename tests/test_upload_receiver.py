import unittest
import os
from unittest.mock import patch
from deploy.receive_upload import command, receive, STORE


class ReceiverTests(unittest.TestCase):
    def test_commands_and_ttys_rejected_before_accessing_host(self):
        for environment in [{"SSH_ORIGINAL_COMMAND": "id"}, {"SSH_TTY": "/dev/pts/0"}]:
            with patch.dict(os.environ, environment, clear=True):
                with self.assertRaisesRegex(ValueError, "Only snapshot upload"):
                    receive()
    def test_fixed_command_and_store(self):
        argv = command("sha256:" + "a" * 64, "azeroth-upload-" + "b" * 32)
        self.assertEqual(argv[:5], ["lxc-attach", "-n", "290", "--", "docker"])
        self.assertEqual(argv[-1], "import-stdin")
        self.assertIn(STORE + ":/data", argv)
        self.assertIn("none", argv)
        self.assertNotIn("sh", argv)

    def test_mutable_or_injected_config_rejected(self):
        for image in ["latest", "sha256:" + "a" * 64 + ";sh"]:
            with self.assertRaises(ValueError):
                command(image, "azeroth-upload-" + "b" * 32)
        with self.assertRaises(ValueError):
            command("sha256:" + "a" * 64, "other-container")
