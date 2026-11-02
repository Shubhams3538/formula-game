# To do

## Track and cameras

- Eau Rouge: the car now gets through Eau Rouge/Raidillon without lifting
  (scrubbing out to the exit kerb), but the AI still lifts at Raidillon
  (~215-245 km/h depending on pace). The converted mesh's Raidillon is tighter than the real corner
  (~115 m best-line radius); a cleaner fix needs track-geometry work.
- Fix the TV camera position.

## Car artwork

- Continue custom team artwork on the separate livery branch stacked on 2.0.
  The core branch uses neutral stock body paint and retains shared geometry.

## 2026 grid (roster only)

- Update the roster to the 2026 season: 11 teams including Cadillac, 2026 team IDs
  and names (e.g. `alphatauri` -> Racing Bulls, `alfaromeo` -> Audi), 2026 driver
  names, abbreviations and numbers. Covers `assets/cars/gen22/teams.json` and anything
  keyed on team ID in menus, grid size and AI.
- Keep the ground-effect era (2022–2025) regulations, car model and physics as
  they are. Do not adopt the 2026 technical regulations.

## Other

- Car selection menu redesign.
- Measure uncapped frame rate on a real display (headless Weston caps at 60).
