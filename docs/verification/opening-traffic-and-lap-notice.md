# Opening traffic, lap notifications and automatic shifting

Verified on 2026-11-02. Changes remain uncommitted.

## Behavior and applicable paths

- Bevy HUD: lap-completion events produce one compact notification containing
  the label and exact lap time. Notifications sit near the top of the screen,
  rather than covering the road with separate 40 px headline cards. Applies to
  both instrument layouts and wide/narrow windows; expires normally after 2.5 s.
- Shared simulation AI: retain room alongside, brake for closing distance, and
  suspend tactical swerves and deliberate late braking inside crowded packs.
  Two-car passing and DRS still pass their existing behavior tests.
- Automatic gearbox: compare available power in the current and next ratios
  rather than waiting only for 98.5% of the rev limiter. Applies to both car
  models and AI/player automatic shifting. Gen22 seventh is also shortened from
  5.7487 to 6.0 overall ratio so manual drivers can reach the shift lights at
  high wing settings. Eighth, all other ratios, engine output, downshift
  protection, and input/network contracts are unchanged.
- No asset conversion, livery, packaging, archive, or protocol changes apply.

## Evidence

The old high-wing Gen22 straight-line probe settled in seventh at 303.7 km/h
and 13,047 rpm, below the 13,297.5 rpm automatic upshift threshold. The revised
probe reaches eighth with low, default, and maximum wings, without DRS or a tow.
With maximum wings it reaches 303.8 km/h at 11,849 rpm in eighth. This corrects
gear selection; it does not add power to overcome aerodynamic drag. After the
manual-gearing follow-up, a high-wing manual probe starting at 260 km/h reaches
the shift lights around 297 km/h, accepts a single real upshift input, and
accelerates to 302 km/h in eighth five seconds later. The same probe failed to
reach the shift lights before shortening seventh. Manual and automatic probes
cover low, default and maximum wings. Existing
launch-time and top-speed acceptance tests pass.

Opening-traffic regression uses eight AI, seeds 5/17/42, and the first 45 seconds
on each generated Spa, Bahrain, Silverstone and COTA track. Across these twelve
starts, car contacts fell from 197 to 22, contacts above 6,000 Ns from 21 to 1,
and the largest impulse from 33,423 to 7,480 Ns. The remaining severe contact
was in Bahrain at 29.29 seconds, around station 1,275 m. All cars in the final
parent-run probe progressed more than 1,330 m. This is not collision-free racing.
After shortening seventh, the same twelve-start regression passed again with
19 contacts, no contacts above 6,000 Ns, and a largest impulse of 4,782 Ns.

Vulkan timing fixtures passed and were visually inspected at 1920x1080 and
958x1078 under isolated Weston/Xwayland. Captures include the actual new
lap-completion event, both HUD instrument layouts, first-lap state, and wheel LCD.
Saved notification captures:

- `/home/radhey/.cache/f1-drive-fixes/notification-wide.png`
- `/home/radhey/.cache/f1-drive-fixes/notification-narrow.png`

## Checks and limits

- `cargo test --workspace --lib --bins`: passed.
- `F1_TEST_ASSETS=/home/radhey/code/f1-the-game/assets cargo test -p racecraft-sim --test race_starts -- --nocapture`: passed all twelve starts.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `rendered_timing_and_pass_states --ignored --nocapture`: passed at both sizes.

Human-driven interactions, subjective manual gearbox feel, full 20-car starts, and a
complete rendered race were not tested. The AI probe exercises real generated
track collision geometry and simulation, rather than a visual replay.
