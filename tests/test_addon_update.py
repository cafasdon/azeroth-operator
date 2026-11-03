import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from deploy import install_addon_update
from deploy.install_addon_update import install
from tools.package_addon import package, render_entries


class AddonUpdateTests(unittest.TestCase):
    def test_hash_guard_backup_and_client_identity(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            game = root / "game"
            addon = game / "Interface" / "AddOns" / "AzerothOperator"
            addon.mkdir(parents=True)
            (game / ".flavor.info").write_text("Product Flavor!STRING:0\nwow\n")
            old_archive, old = package("retail","live",120105,"old-dev",root / "packages")
            new_archive, new = package("retail","live",120105,"new-dev",root / "packages")
            old_entries = render_entries("retail","live",120105,"old-dev")
            # Model the installed exporter-only release, before macro modules existed.
            for name in ("MacroOperations.lua", "MacroImport.lua", "MacroUI.lua"):
                del old_entries[name]
                del old["files"][name]
            old_archive.with_suffix(".manifest.json").write_text(json.dumps(old))
            for name,data in old_entries.items(): (addon / name).write_bytes(data)
            args = [game,old_archive.with_suffix(".manifest.json"),new_archive.with_suffix(".manifest.json"),root / "backups"]
            (addon / "Core.lua").write_text("user modification")
            with self.assertRaisesRegex(ValueError,"changed"):
                install(*args)
            self.assertFalse((root / "backups").exists())
            (addon / "Core.lua").write_bytes(old_entries["Core.lua"])
            (game / ".flavor.info").write_text("Product Flavor!STRING:0\nwowf\n")
            with self.assertRaisesRegex(ValueError,"product"):
                install(*args)
            (game / ".flavor.info").write_text("Product Flavor!STRING:0\nwow\n")
            original_replace = install_addon_update.replace
            def concurrent_edit(path, data):
                original_replace(path, data)
                if path.name == "Collectors.lua":
                    (addon / "Core.lua").write_text("concurrent user edit")
            with patch.object(install_addon_update, "replace", concurrent_edit):
                with self.assertRaisesRegex(ValueError,"during update"):
                    install(*args)
            self.assertEqual((addon / "Core.lua").read_text(), "concurrent user edit")
            self.assertEqual((addon / "AzerothOperator.toc").read_bytes(),old_entries["AzerothOperator.toc"])
            (addon / "Core.lua").write_bytes(old_entries["Core.lua"])
            def fail_after_new_module(path, data):
                if path.name == "MacroUI.lua":
                    raise OSError("simulated module write failure")
                original_replace(path, data)
            with patch.object(install_addon_update, "replace", fail_after_new_module):
                with self.assertRaisesRegex(OSError,"simulated"):
                    install(*args)
            self.assertEqual({p.name for p in addon.iterdir()}, set(old_entries))
            for name, data in old_entries.items():
                self.assertEqual((addon / name).read_bytes(), data)
            result = install(*args)
            self.assertEqual(result["verified_files"],7)
            backup = Path(result["backup"])
            self.assertEqual((backup / "AzerothOperator.toc").read_bytes(),old_entries["AzerothOperator.toc"])
            self.assertEqual(json.loads((backup / "manifest.json").read_text()),old)
            self.assertIn("new-dev",(addon / "AzerothOperator.toc").read_text())
