import json
from pathlib import Path
import tempfile
import unittest
from tools.package_addon import package, render_entries
from tools.verify_installed_addon import verify, MAX_FILE


class InstalledAddonTests(unittest.TestCase):
    def test_drift_detection_never_repairs(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            client = root / "game"
            client.mkdir()
            (client / ".flavor.info").write_text("Product Flavor!STRING:0\nwow\n")
            archive, receipt = package("retail", "live", 120105, "test", root / "packages")
            manifest = archive.with_suffix(".manifest.json")
            self.assertEqual(verify(str(client), str(manifest))["status"], "not_installed")
            target = client / "Interface" / "AddOns" / "AzerothOperator"
            target.mkdir(parents=True)
            entries = render_entries("retail", "live", 120105, "test")
            for name, data in entries.items():
                (target / name).write_bytes(data)
            result = verify(str(client), str(manifest))
            self.assertEqual(result["status"], "matches_manifest")
            self.assertEqual(result["verified_files"], 11)
            (target / "Core.lua").write_bytes(b"user edit")
            (target / "Adapter.lua").unlink()
            (target / "extra.txt").write_text("keep")
            result = verify(str(client), str(manifest))
            self.assertEqual({p["reason"] for p in result["problems"]},
                             {"unexpected_entries", "missing", "hash_mismatch"})
            self.assertEqual((target / "Core.lua").read_bytes(), b"user edit")
            self.assertEqual((target / "extra.txt").read_text(), "keep")
            (target / "Core.lua").write_bytes(b"x" * (MAX_FILE + 1))
            self.assertIn("too_large", {p["reason"] for p in verify(str(client), str(manifest))["problems"]})
            receipt["files"]["../escape"] = receipt["files"].pop("Adapter.lua")
            manifest.write_text(json.dumps(receipt))
            with self.assertRaisesRegex(ValueError, "hashes"):
                verify(str(client), str(manifest))


if __name__ == "__main__":
    unittest.main()
