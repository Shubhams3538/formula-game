import tempfile
import hashlib
import unittest
from pathlib import Path
import zipfile

from extract_assets import extract


class ExtractAssetsTests(unittest.TestCase):
    def test_extracts_only_into_asset_destination(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "assets.zip"
            with zipfile.ZipFile(archive, "w") as source:
                source.writestr("assets/cars/car.json", b"{}")
            extract(archive, root / "output", hashlib.sha256(archive.read_bytes()).hexdigest())
            self.assertEqual((root / "output/cars/car.json").read_bytes(), b"{}")

    def test_rejects_code_traversal_and_windows_paths_before_writing(self):
        for path in ["tools/package/build_windows.py", "assets/../../code.py", "assets\\..\\code.py", "assets/C:/code.py", "/assets/code.py"]:
            with self.subTest(path=path), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                archive = root / "assets.zip"
                with zipfile.ZipFile(archive, "w") as source:
                    source.writestr("assets/valid.json", b"{}")
                    source.writestr(path, b"untrusted")
                with self.assertRaises(ValueError):
                    extract(archive, root / "output", hashlib.sha256(archive.read_bytes()).hexdigest())
                self.assertFalse((root / "output").exists())

    def test_rejects_changed_archive_before_writing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "assets.zip"
            with zipfile.ZipFile(archive, "w") as source:
                source.writestr("assets/car.json", b"{}")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                extract(archive, root / "output", "0" * 64)
            self.assertFalse((root / "output").exists())


if __name__ == "__main__":
    unittest.main()
