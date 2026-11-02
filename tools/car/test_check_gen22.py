"""Standalone contract checks require the roster without external team textures."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('check_gen22', Path(__file__).with_name('check_gen22.py'))
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class CheckerTests(unittest.TestCase):
    def test_roster_is_required_without_team_texture_directories(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'car.json').write_text('{}')
            with patch.object(sys, 'argv', ['check_gen22.py', str(root)]), patch.object(checker, 'inspect'), patch.object(checker, 'check_tyre_textures'):
                with self.assertRaisesRegex(AssertionError, 'teams.json'):
                    checker.main()
                (root / 'teams.json').write_text(json.dumps({'teams': [{'id':'test','drivers':[]}]}))
                checker.main()
                (root / 'teams.json').write_text('{"teams": []}')
                with self.assertRaisesRegex(AssertionError, 'teams must not be empty'):
                    checker.main()


if __name__ == '__main__':
    unittest.main()
