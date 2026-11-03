import unittest
from deploy.receive_upload import command, STORE


class ReceiverTests(unittest.TestCase):
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
