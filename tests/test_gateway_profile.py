import unittest
from tools.gateway_profile import profile, TOOLS, STORE


class GatewayProfileTests(unittest.TestCase):
    def test_dedicated_profile_has_only_read_tools_and_store(self):
        image = "sha256:" + "a" * 64
        result = profile(image)
        self.assertEqual(result["id"], "azeroth-operator")
        self.assertEqual(len(result["servers"]), 1)
        entry = result["servers"][0]
        self.assertEqual(entry["tools"], TOOLS)
        self.assertNotIn("import-stdin", entry["tools"])
        server = entry["snapshot"]["server"]
        self.assertEqual(server["image"], image)
        self.assertEqual(server["volumes"], [STORE + ":/data:ro"])

    def test_tags_and_untrusted_image_syntax_rejected(self):
        for image in ["latest", "azeroth:latest", "sha256:" + "A" * 64,
                      "sha256:" + "a" * 63, "sha256:" + "a" * 64 + "\n"]:
            with self.assertRaises(ValueError):
                profile(image)
