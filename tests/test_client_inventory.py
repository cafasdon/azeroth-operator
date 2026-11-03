from pathlib import Path
import tempfile
import unittest
from tools.inspect_client import inspect, build_versions, read_metadata


class InventoryTests(unittest.TestCase):
    def test_family_identity_and_conflicting_builds(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            client = root / "client"
            client.mkdir()
            for family, product in [("retail", "wow"), ("forever", "wowf"),
                                    ("classic", "wow_classic_era")]:
                (client / ".flavor.info").write_text("Product Flavor!STRING:0\n" + product + "\n")
                (root / ".build.info").write_text("Product!STRING:0|Active!DEC:1|Version!STRING:0\n"
                    + product + "|1|1.2.3.4\nother|1|9.9.9.9\n" + product + "|0|2.3.4.5\n")
                result = inspect(str(client), family, "explicit-test")
                self.assertEqual(result["build"], "1.2.3.4")
                self.assertFalse(result["live_qualified"])
                self.assertIsNone(result["interface"])
                with self.assertRaises(ValueError):
                    inspect(str(client), "retail" if family != "retail" else "forever", "test")
                with (root / ".build.info").open("a") as handle:
                    handle.write(product + "|1|5.6.7.8\n")
                self.assertEqual(inspect(str(client), family, "test")["status"], "ambiguous_build")
            self.assertEqual(inspect(str(root / "absent"), "classic", "era")["status"], "not_installed")

    def test_malformed_and_bounded_metadata(self):
        for text in ("", "Product|Active|Version|Product\n", "Product|Active|Version\nwow|1\n",
                     "Product|Active|Version\nwow|1|garbage\n"):
            with self.assertRaises(ValueError):
                build_versions(text, "wow")
        self.assertEqual(build_versions("Product|Active|Version\nwow|0|1.2.3.4", "wow"), [])
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "metadata"
            path.write_bytes(b"x" * 100)
            with self.assertRaises(ValueError):
                read_metadata(path, 99)
            self.assertEqual(len(read_metadata(path, 100)), 100)


if __name__ == "__main__":
    unittest.main()
