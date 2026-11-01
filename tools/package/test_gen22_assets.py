"""Package preflight rejects incomplete GEN22 asset trees without building."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('build_appimage', Path(__file__).with_name('build_appimage.py'))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class Gen22PackageTests(unittest.TestCase):
    def test_empty_roster_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ('car.glb', 'car_lod1.glb', 'car.json'):
                (root / name).write_text('fixture')
            (root / 'teams.json').write_text('{"teams": []}')
            with self.assertRaisesRegex(RuntimeError, 'roster must not be empty'):
                package.validate_gen22_assets(root)

    def test_embedded_textures_need_no_external_livery_tree(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            roster = {'teams': [{'id': 'ferrari', 'drivers': [{'id': 'leclerc'}, {'id': 'sainz'}]}]}
            paths = ['car.glb', 'car_lod1.glb', 'car.json', 'teams.json'] + [
                str(Path('tyres') / lod / (compound + '.png'))
                for lod in ('', 'lod1', 'lod2') for compound in ('soft', 'medium', 'hard')
            ]
            for name in paths:
                (root / name).parent.mkdir(parents=True, exist_ok=True)
                (root / name).write_text(json.dumps(roster) if name == 'teams.json' else 'fixture')
            package.validate_gen22_assets(root)
            for name in paths:
                path = root / name
                contents = path.read_bytes()
                path.unlink()
                with self.subTest(name=name), self.assertRaisesRegex(RuntimeError, name):
                    package.validate_gen22_assets(root)
                path.write_bytes(contents)


if __name__ == '__main__':
    unittest.main()
