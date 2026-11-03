import io
import tempfile
import unittest
import zipfile
from pathlib import Path
from tools.package_addon import render_entries
from tools.package_bigwigs import get_packager, verified_contents


class BigWigsTests(unittest.TestCase):
    def test_allowlist_and_payload_verification(self):
        entries = render_entries("classic", "era", 11500, "fixture")
        for mutation in ("none", "extra", "changed", "duplicate", "traversal"):
            stream = io.BytesIO()
            with zipfile.ZipFile(stream, "w") as archive:
                for name, data in entries.items():
                    archive.writestr("AzerothOperator/" + name, data if mutation != "changed" else b"changed")
                if mutation == "extra": archive.writestr("AzerothOperator/secret.txt", b"fixture")
                if mutation == "traversal": archive.writestr("../outside", b"fixture")
                if mutation == "duplicate":
                    with self.assertWarns(UserWarning):
                        archive.writestr("AzerothOperator/Core.lua", entries["Core.lua"])
            stream.seek(0)
            if mutation == "none": self.assertEqual(verified_contents(stream, entries), entries)
            else:
                with self.assertRaises(ValueError): verified_contents(stream, entries)

    def test_modified_cached_tool_refused_before_execution(self):
        with tempfile.TemporaryDirectory() as folder:
            cache = Path(folder)
            revision = "a" * 40
            (cache / (revision + "-release.sh")).write_text("unexpected code")
            with self.assertRaisesRegex(ValueError, "checksum"):
                get_packager(cache, {"repository": "https://github.com/BigWigsMods/packager",
                                    "commit": revision, "release_sh_sha256": "0" * 64})
            with self.assertRaisesRegex(ValueError, "pin"):
                get_packager(cache, {"commit": "../escape", "release_sh_sha256": "0" * 64})
