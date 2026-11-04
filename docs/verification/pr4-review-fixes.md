# PR 4 review fixes

The review findings were checked against gameplay head `0b61a886`. Duplicate
threads share fixes. Seventh gear keeps its existing ratio and the 98.5 percent
rev-limit reserve; the automatic gearbox test now exercises actual shifts.
Driver pace intentionally changes braking corners, while power-limited bends
stay flat. A synthetic test checks both cases.

## Surfaces

| Surface | Applies | Change and evidence |
|---|---|---|
| Main and paused settings | Yes | Wrapped footer scrolling, keyboard focus, wrapped card descriptions. Native pointer proof at 1920x1080, 958x1078 and 640x600. |
| Camera editor | Yes | Both-axis scrolling limits housing width. Positive slider rails checked down to 180 points. |
| Car preview | Yes | Failed loads retry every five seconds; offscreen completed previews stop rendering. Proof checks both car selections and mesh/camera isolation. |
| HUD timing and recovery | Yes | Every entrant waits for a valid boundary after recovery or a skipped cell. Tests cover same-cell resets, backward jumps, personal versus overall bests, and the finishing boundary. |
| CLI and saved settings | Yes | Unassisted DRS stays manual, explicit saved DRS settings survive aid overrides, old team IDs migrate to successors. |
| Simulation | Yes | Launch lanes retain their own path and grip plan, approaching cars receive clearance in packs, following distance uses contact grip, reverse cars leave no wake, rear ABS reserve can transfer braking to spare front capacity. |
| Linux and Windows packaging | Yes | Same verified ZIP bytes are extracted, failed extraction cleans up, release drafts can be retried, preview instructions include the asset URL. Seven Python packaging tests pass. |
| Asset tools | Yes | Interrupted sky downloads use a temporary file. Explicit barriers survive kerb-textured materials. Eleven track tool tests pass. |
| Audio | Yes | Thunder envelope stops decaying while paused. Mixer test passes; the live test app now initializes Storm. |
| Network transports | No | This PR has no multiplayer transport. The separate PR4 multiplayer integration checkout is outside these changes. |
| Frozen custom engine | No | Excluded by the active workspace contract. |

## Verification

- `cargo test --workspace --lib --bins`: 138 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- All ten asset-backed ignored simulation tests passed with the converted Spa
  assets. The tow probe also asserts a positive gain at 15, 30, 60 and 90 metres.
- Three native responsive menu proofs passed. The track-selection proof opens
  compact navigation explicitly, then asserts the first completed Track selection.
- Native track-selection and HUD timing proofs passed. The HUD proof captured
  five states, checked DRS Ready/Open, and retained all nine final-lap minisector
  colors at the finish. The finished screenshot was visually inspected.
- The published 304,617,680-byte asset archive reports the pinned SHA-256
  `68ae3a48b726ffa5789ca049ebc438e77db8aca6fa91a46755c1376ae4053e7d`.
  Its public download returns a ZIP header. An FBS copy was also downloaded and
  verified against that full SHA-256.

Native checks use isolated Weston displays and NVIDIA Vulkan. They do not
replace wheel/gamepad driving or live listening. The tagged Linux/Windows release
workflow was not published as a game release during this review. The archive's
experimental circuits still predate local conversions, as recorded in TODO.md.
