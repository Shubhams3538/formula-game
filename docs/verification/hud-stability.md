# HUD and dashboard stability

Drivers need readable live values while cornering. The existing charcoal instruments, fonts and semantic colors remain the visual system. The scoped redesign replaces the floating delta box with a timing strip and three sector rows, and replaces large pass banners with one compact position-change notice beside the timing tower.

Hard constraints: Bevy UI stays in `game/`; physics and official timing stay in `sim/`; leave the archived engine and unrelated work untouched; no commits or pushes. Gear, pedals, limiter, DRS, wheel lock and slide warnings remain immediate. Completed lap and sector times remain exact.

| Surface | Applies | Source | Required change | Check | State |
|---|---|---|---|---|---|
| Simulation ticks | yes | `main.rs::step_race` | Sample display averages after every physics tick | Tick/window tests | passed |
| Digital and analog HUD | yes | `hud.rs`, `hud/instruments.rs` | Shared averaged numbers, timing and pass redesign | Vulkan landscape/narrow captures | passed |
| Full and compact wheel LCD | yes | `hud/wheel.rs`, `wheel_dash.rs` | Consume the same averaged telemetry and delta | Native cockpit + painter checks | passed |
| Restart, pause, camera changes | yes | `menu.rs`, `controls.rs` | Reset with session; freeze averages when no physics ticks run | Lifecycle tests | passed |
| Benchmark stepping and snapshots | yes | `benchmark.rs` | Sample its independent stepping path; reset on snapshots | Workspace checks | passed |
| Race events and official results | yes | `sim/src/race.rs` | No simulation changes; confirm pass only for presentation | Pass reversal/finish tests | passed |
| Network/providers | no | Local Bevy game | No network contract involved | N/A | N/A |
| Legacy engine | no | `archive/custom-engine` | Frozen workspace | Diff scope | passed |
| User documentation | yes | `README.md` | Explain short display averaging and exact completed timings | Review | passed |

## Verification

- `cargo test -p f1-the-game hud:: -- --nocapture`: focused averaging, pass reversal/confirmation/expiry, split memory, clock and delta-reference tests passed.
- `cargo test --workspace --lib --bins`: 76 game tests and 42 simulation tests passed; 15 device/asset-specific tests remain ignored by this command.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` and `git diff --check`: passed.
- `rendered_timing_and_pass_states`: passed at 1920×1080 and 958×1078 on NVIDIA RTX 3050 Vulkan, full eight-car GEN22 field with the supported Half texture setting. Captures cover digital gain, analog loss, first-lap empty state and the real offscreen wheel LCD. Timing/pass states are synthetic fixtures; they verify presentation, not race events.
- `rendered_game_flows`: passed at 1280×720 with RB6. Real driving, gear changes, chase/cockpit/TV camera changes, pause/resume, track recovery, both instrument layouts, both minimaps and results; all twelve captures completed.
- Reviewed downscaled contact sheets at `/tmp/f1-hud-stability/review-contact.jpg`, `timing-contact.jpg` and `driving-contact.jpg`. Individual PNGs and logs are under `wide/`, `narrow/` and `driving/` in that directory. Renderer subprocess status files each contain 0.

The runtime drive does not complete a full lap or perform a human-driven pass. Exact lap-boundary interpolation and sector retention, and both pass directions with rapid reversals, are covered by focused deterministic tests. Human judgement of the smoothing feel during a full race remains untested. No simulation files or archived engine files changed. No commit or push was made.

## Finish review

The first finish review accepted the timing hierarchy, digit alignment, sector states, peripheral pass position and shared averages. It found narrow timing/status labels too small. Those labels now have an 11 px floor and sector values/pass names a 12 px floor; columns and panel placement remain fixed. The narrow states were recaptured successfully at 958×1078 after the correction, with status 0. The downscaled full-state/crop sheet is `/tmp/f1-hud-stability/narrow-final-contact.jpg`. Clippy, formatting and diff whitespace checks passed again after the correction.

PRODUCT.md now records only the confirmed user problem, established Bevy instrument scope and timing/readability constraints. DESIGN.md and its sidecar document the shipped visual system. The review's pending-ledger observation came from an earlier read; the completed checks and limits are recorded above.

Final independent design verdict: **ship**.

| Material finding | Verdict |
|---|---|
| Narrow timing/status/pass readability | Resolved; final narrow captures show readable text without collisions or added driving-view obstruction. |
| Product/design context and verification record | Resolved; confirmed scope, typography floors, passed checks and runtime limitations recorded. |

Remaining findings: clear. No regressions visible in the fix batch.
