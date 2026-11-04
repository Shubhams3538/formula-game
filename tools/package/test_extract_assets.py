import tempfile
import hashlib
import unittest
from unittest import mock
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

    def test_failed_copy_is_removed_and_can_be_retried(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "assets.zip"
            with zipfile.ZipFile(archive, "w") as source:
                source.writestr("assets/a.json", b"{}")
                source.writestr("assets/b.json", b"{}")
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            with mock.patch("extract_assets.shutil.copyfileobj", side_effect=OSError("disk full")):
                with self.assertRaises(OSError):
                    extract(archive, root / "output", digest)
            self.assertFalse((root / "output").exists())
            extract(archive, root / "output", digest)
            self.assertTrue((root / "output/b.json").is_file())

    def test_path_replacement_cannot_change_verified_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive, replacement = root / "assets.zip", root / "replacement.zip"
            for path, value in [(archive, b"verified"), (replacement, b"replacement")]:
                with zipfile.ZipFile(path, "w") as source:
                    source.writestr("assets/car.json", value)
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            real_digest = hashlib.sha256
            def replace_after_hash(content):
                result = real_digest(content)
                replacement.replace(archive)
                return result
            with mock.patch("extract_assets.hashlib.sha256", side_effect=replace_after_hash):
                extract(archive, root / "output", digest)
            self.assertEqual((root / "output/car.json").read_bytes(), b"verified")

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
