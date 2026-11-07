import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from deploy.install_forever_capture import install
from tools.package_addon import package
from tools.verify_installed_addon import FILES


@unittest.skipUnless(os.name == "nt", "Windows no-replace rename")
class ForeverInstallTests(unittest.TestCase):
    def test_cli_installs_exact_files_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            client = root / "_forever_"
            addons = client / "Interface" / "AddOns"
            addons.mkdir(parents=True)
            (client / ".flavor.info").write_text("Product Flavor!STRING:0\nwowf\n")
            (root / ".build.info").write_text("Product!STRING:0|Active!DEC:1|Version!STRING:0\nwowf|1|1.60.1.70862\n")
            archive, _ = package("forever", "live", 16001, "0.27.0-dev", root)
            manifest = archive.with_suffix(".manifest.json")
            command = [sys.executable, "-m", "deploy.install_forever_capture", "--client", str(client),
                       "--manifest", str(manifest), "--expected-build"]
            target = addons / "AzerothOperator"

            def call(build):
                return subprocess.run([*command, build], capture_output=True, text=True, timeout=15)

            self.assertNotEqual(call("1.60.1.00001").returncode, 0)
            self.assertFalse(target.exists())
            original = archive.read_bytes()
            archive.write_bytes(original + b"changed")
            self.assertNotEqual(call("1.60.1.70862").returncode, 0)
            self.assertFalse(target.exists())
            archive.write_bytes(original)
            with patch("deploy.install_forever_capture.verify", return_value={"status":"drift","verified_files":0}):
                with self.assertRaisesRegex(ValueError, "directory was created but verification failed"):
                    install(str(client), str(manifest), "1.60.1.70862")
            self.assertEqual({path.name for path in target.iterdir()}, FILES)
            target.rename(addons / "verified-failure-fixture")
            installed = call("1.60.1.70862")
            self.assertEqual(installed.returncode, 0, installed.stderr)
            self.assertEqual(json.loads(installed.stdout)["verified_files"], 4)
            self.assertEqual({path.name for path in target.iterdir()}, FILES)
            marker = target / "Core.lua"
            marker.write_bytes(b"preserve existing")
            self.assertNotEqual(call("1.60.1.70862").returncode, 0)
            self.assertEqual(marker.read_bytes(), b"preserve existing")
