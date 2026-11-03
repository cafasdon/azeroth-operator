import tempfile
import unittest
import zipfile
from pathlib import Path
from tools.package_addon import package


class PackagingTests(unittest.TestCase):
    def test_repeatable_and_isolated(self):
        with tempfile.TemporaryDirectory() as folder:
            a, receipt = package("retail", "live", 120105, "0.1.0-dev", folder)
            raw = a.read_bytes()
            package("retail", "live", 120105, "0.1.0-dev", folder)
            self.assertEqual(raw, a.read_bytes())
            self.assertEqual(receipt["qualification"], "development_only")
            with zipfile.ZipFile(a) as archive:
                self.assertEqual(len(archive.namelist()), 4)
                toc = archive.read("AzerothOperator/AzerothOperator.toc").decode()
                for line in toc.splitlines():
                    if line and not line.startswith("#"):
                        self.assertIn("AzerothOperator/" + line, archive.namelist())
                self.assertIn('AO.family = "retail"', archive.read("AzerothOperator/Adapter.lua").decode())
            b, _ = package("forever", "beta", 16001, "0.1.0-dev", folder)
            self.assertNotEqual(a, b)
            with self.assertRaises(ValueError):
                package("retail", "../escape", 120105, "dev", folder)
            self.assertEqual(len(list(Path(folder).glob("*.zip"))), 2)
