"""Checks tools/car/teams.json against the gen22 roster contract.

Run with: python3 -I tools/car/test_teams.py
"""

import json
import re
import unittest
from pathlib import Path

TEAMS_PATH = Path(__file__).resolve().parent / "teams.json"

# 2022 constructors' order, with each team's race drivers as (name, abbr, number).
EXPECTED_GRID = {
    "redbull": [("Max Verstappen", "VER", 1), ("Sergio Perez", "PER", 11)],
    "ferrari": [("Charles Leclerc", "LEC", 16), ("Carlos Sainz", "SAI", 55)],
    "mercedes": [("Lewis Hamilton", "HAM", 44), ("George Russell", "RUS", 63)],
    "alpine": [("Esteban Ocon", "OCO", 31), ("Fernando Alonso", "ALO", 14)],
    "mclaren": [("Lando Norris", "NOR", 4), ("Daniel Ricciardo", "RIC", 3)],
    "alfaromeo": [("Valtteri Bottas", "BOT", 77), ("Zhou Guanyu", "ZHO", 24)],
    "astonmartin": [("Sebastian Vettel", "VET", 5), ("Lance Stroll", "STR", 18)],
    "haas": [("Kevin Magnussen", "MAG", 20), ("Mick Schumacher", "MSC", 47)],
    "alphatauri": [("Pierre Gasly", "GAS", 10), ("Yuki Tsunoda", "TSU", 22)],
    "williams": [("Alexander Albon", "ALB", 23), ("Nicholas Latifi", "LAT", 6)],
}

HEX_RE = re.compile(r"^#[0-9A-Fa-f]{6}$")
ID_RE = re.compile(r"^[a-z]+$")
ABBR_RE = re.compile(r"^[A-Z]{3}$")
ROLES = {"primary", "secondary", "accent", "carbon"}
SHORT_NAME_MAX = 12


class TeamsContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = json.loads(TEAMS_PATH.read_text(encoding="utf-8"))
        cls.teams = cls.data["teams"]

    def test_season_and_grid_order(self):
        self.assertEqual(self.data["season"], 2022)
        self.assertEqual(len(self.teams), 10)
        self.assertEqual([t["id"] for t in self.teams], list(EXPECTED_GRID))

    def test_team_names(self):
        for team in self.teams:
            with self.subTest(team=team["id"]):
                self.assertIsInstance(team["name"], str)
                self.assertTrue(team["name"].strip())
                self.assertIsInstance(team["short"], str)
                self.assertTrue(team["short"].strip())
                self.assertLessEqual(len(team["short"]), SHORT_NAME_MAX)

    def test_drivers_match_2022_grid(self):
        self.assertEqual(sum(len(t["drivers"]) for t in self.teams), 20)
        for team in self.teams:
            with self.subTest(team=team["id"]):
                got = [(d["name"], d["abbr"], d["number"]) for d in team["drivers"]]
                self.assertEqual(got, EXPECTED_GRID[team["id"]])

    def test_ids_and_numbers_unique(self):
        team_ids = [t["id"] for t in self.teams]
        driver_ids = [d["id"] for t in self.teams for d in t["drivers"]]
        numbers = [d["number"] for t in self.teams for d in t["drivers"]]
        abbrs = [d["abbr"] for t in self.teams for d in t["drivers"]]
        self.assertEqual(len(set(team_ids)), len(team_ids))
        self.assertEqual(len(set(driver_ids)), len(driver_ids))
        self.assertEqual(len(set(numbers)), len(numbers))
        self.assertEqual(len(set(abbrs)), len(abbrs))
        for team_id in team_ids:
            self.assertRegex(team_id, ID_RE)
        for driver_id in driver_ids:
            self.assertRegex(driver_id, ID_RE)
        for abbr in abbrs:
            self.assertRegex(abbr, ABBR_RE)

    def test_colours(self):
        for team in self.teams:
            with self.subTest(team=team["id"]):
                self.assertEqual(set(team["colors"]), ROLES)
                for role, value in team["colors"].items():
                    self.assertRegex(value, HEX_RE, msg=role)


if __name__ == '__main__':
    unittest.main()
