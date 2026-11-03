# To do

## Track and cameras

- Fix the TV camera position.
- Spa's Raidillon in the converted mesh is tighter than the real corner (~120 m
  best-line radius). Eau Rouge is now flat on the default setup through car
  downforce, but low-wing setups (2/2) still lift; true-to-life geometry needs
  track work.

## Race behaviour (verify in game)

- The AI tow, overtaking and opening-lap behaviour were tuned and checked in
  headless Spa races (asset-backed ignored tests in `sim`). Confirm in real
  races: faster cars pull out and pass, the tow works from ~100 m, and the
  first corner stays clean with a full 22-car grid.
- ABS braking no longer spins the car with a touch of lock; confirm the feel
  with a wheel and a gamepad.
- Grid lanes keep the start two wide but cost the leader ~50 m by 30 s
  (La Source from a lane, not the line). An early merge for the leader did not
  recover it and added contacts.

## Release assets

- Upload `target/packaging/f1-racecraft-assets.zip` (adds the GEN22 car, 2026
  roster and the fixed Spa collision) to the asset archive URL before the next
  release tag; `release.yml` already pins its SHA-256.
- The archive's experimental tracks (Silverstone, Bahrain, COTA) predate the
  local conversions; refresh them once that track work is reviewed.

## Car artwork

- Continue custom team artwork on the separate livery branch stacked on 2.0.
  The core branch uses neutral stock body paint and retains shared geometry.
- The 2026 roster renames teams (`alphatauri` -> `racingbulls`, `alfaromeo` ->
  `audi`) and adds `cadillac`; livery folders keyed on the old IDs need moving
  or recreating on the livery branch.

## Other

- Measure uncapped frame rate on a real display (headless Weston caps at 60).
