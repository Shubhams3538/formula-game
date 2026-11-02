# Frame-time optimization, 2026-10-30

## Scope and constraints

Code changes are limited to game/src/graphics.rs, game/src/wheel_dash.rs and the
capture-run Winit settings in game/src/main.rs. No commit, staging,
reset, stash, or changes to the reserved car pipeline/material/loading files.
The measured workload is Spa, RB6, chase camera, 1920x1080, two moving AI cars,
seed 24301, 20 seconds, muted, uncapped Vulkan through PRIME/XWayland.

## Measurement controls

The initial Weston runs were capped at 60 FPS and are excluded. Early desktop
builds were affected by concurrent material edits and are exploratory only.
The primary desktop A/B measurements use one snapshotted binary with temporary experiment
switches. Thus graphics changes are isolated from concurrent car-source edits.
Those switches were removed from the shipped source. A second same-binary
comparison on isolated Weston includes the capture-throttling fix on both sides. Asset fingerprints and all
six exact screenshot/sample state hashes must match for each final pair.
The final runner waits for an idle GPU/compiler and rejects a run if another GPU
process or compiler starts. Telemetry records clocks, utilization, power,
temperature and VRAM. CPU-only background desktop/asset work remains present.
All reported comparison runs enable F1_PROFILE on both sides.

## Results

Primary desktop XWayland comparison, identical binary, assets, drive and camera.
Values are before -> after. These are 20-second samples, not confidence intervals.

| Preset | Average FPS | Mean ms | Median ms | p95 ms | 1% low FPS |
|---|---:|---:|---:|---:|---:|
| Low | 233.4 -> 238.1 | 4.284 -> 4.201 | 4.232 -> 4.037 | 5.237 -> 5.732 | 150.1 -> 104.8 |
| Medium | 190.2 -> 189.0 | 5.256 -> 5.290 | 5.016 -> 4.780 | 7.345 -> 8.287 | 95.4 -> 48.5 |
| High | 141.0 -> 156.1 | 7.094 -> 6.407 | 7.027 -> 6.226 | 8.658 -> 8.136 | 87.0 -> 62.1 |
| Ultra | 103.2 -> 109.5 | 9.693 -> 9.135 | 9.531 -> 8.978 | 12.142 -> 12.305 | 44.5 -> 44.9 |

High improves 10.7%, saving 0.688 ms/frame. Ultra improves 6.1%, saving
0.558 ms/frame. Low improves 2.0%; Medium average is flat (-0.6%). Median frame
time improves on all four presets. The 1% lows do not consistently improve.
Background CPU work and memory pressure remain uncontrolled; no claim is made
that stutters or Low/Medium performance have been fixed.

## Isolated display cross-check

Both sides use continuous Winit updates on headless Weston/XWayland. The suite
completed successfully and verifies matching workload and asset hashes.

| Preset | Average FPS before -> after | Mean ms before -> after |
|---|---:|---:|
| Low | 156.7 -> 248.4 | 6.380 -> 4.026 |
| Medium | 186.9 -> 177.3 | 5.349 -> 5.640 |
| High | 132.9 -> 149.9 | 7.526 -> 6.669 |
| Ultra | 99.8 -> 104.9 | 10.020 -> 9.536 |

High/Ultra throughput gains repeat, at 12.8% and 5.1%. The large isolated Low
jump is not a credible estimate of the change alone: its reference has much
larger CPU-side frame times despite similar GPU-pass timings. Medium remains
inconclusive. Both comparison sets are shown instead of selecting the fastest
runs or treating system noise as a code improvement.

Reducing the reference High resolution to 960x540 raises throughput from
132.9 to 221.4 FPS and lowers mean frame time from 7.526 to 4.517 ms. The same
simulation drive and same two AI cars support a GPU bottleneck at 1080p.
No evidence here identifies sim stepping as the limiting cost. CPU-bound HUD
and full-field simulation need a dedicated trace once the asset work is idle.
The optimized cockpit workload completes at 138.7 FPS; it retains live LCD
rendering, so its rate is not directly comparable with chase view.

## Retained changes

- Capture and benchmark windows update continuously, even without focus. This
  prevents the inherited 60 Hz throttle and keeps isolated runs uncapped.
- Smaller atmosphere lookup textures. Integration sample counts and the aerial
  perspective volume are unchanged. High sky-view table is 200x100; Ultra
  300x150; transmittance 128x64 and multiscattering 16x16 on every preset.
- Short contact-shadow rays use 8 steps on High, 12 on Ultra instead of 16.
  Ray length, thickness, shadow maps, cascade distances, SSAO and TAA stay intact.
- Chase view skips the hidden cockpit LCD's egui drawing, offscreen camera and
  nine mip downsampling passes. Cockpit and TV views resume all of them.

## Individual High-preset impact

Same desktop binary, cumulative switches, identical scenario:

| Enabled changes | Average FPS | Mean ms | Incremental mean saving |
|---|---:|---:|---:|
| Reference | 141.0 | 7.094 | - |
| Atmosphere tables | 144.2 | 6.934 | 0.160 ms |
| Plus contact-shadow samples | 153.3 | 6.525 | 0.409 ms |
| Plus hidden LCD skipped | 156.1 | 6.407 | 0.118 ms |

These incremental wall-time estimates include run-to-run noise. The direct GPU
medians give stronger attribution: atmosphere LUTs 0.685 -> 0.351 ms, opaque
shading 2.151 -> 1.856 ms. SSAO, TAA and shadow-map timings stay roughly unchanged.
The hidden LCD saves drawing/tessellation, its offscreen view and mip work.

The unfocused benchmark fix removes an artificial 60 FPS cap; it is not an
interactive rendering speedup. Both sides of the isolated comparison use it.

## Bottlenecks and rejected changes

Bevy 0.20.1 already defaults to HighPerformance, and Vulkan logs confirm the
NVIDIA RTX 3050 Laptop GPU, driver 615.78.08, rather than the AMD desktop adapter.
The final executable also selects that GPU without PRIME environment overrides;
a 12-frame capture exits successfully. See `default-adapter.log`.
MSAA is already off; temporal AA is enabled above Low. Static sky environment
refiltering is already frozen. Its unchanged diagnostic values are historical,
not current per-frame timings. Atmosphere LUTs still regenerate every frame.
The main opaque pass, SSAO, atmosphere LUTs and shadows dominate measured GPU time.
Render-pass CPU encoding takes hundredths of a millisecond per pass.
GPU occlusion culling was slower on High in an exploratory measurement and was
removed. No release-profile or sim-tick changes were made without evidence of
an affected CPU bottleneck. The track already has 512 m material tiles, about
132k triangles and 620 primitives; blindly merging it would weaken culling.

## Coverage ledger

- Applies: CLI preset selection and menu preset changes use graphics::apply.
- Applies: all four presets; High and Ultra deterministic screenshot comparisons.
- Applies: chase, TV and cockpit camera transitions, plus camera editor and pause.
- Applies: LCD render extraction, offscreen camera and egui drawing share active state.
- Applies: replacing a race reuses the dash camera; active state follows current session.
- Applies: RB6 pod and GEN22 LCD use the same wheel dash renderer.
- Does not apply: new asset conversion, livery contracts, multiplayer, remote services.
- Untested: Windows/macOS, AMD runtime performance, the upcoming 1–1.5M-triangle
  player asset with 20 opponents, and new-circuit visual coverage.

## Evidence and reproduction

Raw logs, per-frame CSVs, six PNGs per run, reports, telemetry and runner scripts
are under `/tmp/f1-perf-20261030`. Primary accepted reports are
`final-reference-{low,medium,high}-1`, `final-reference-ultra-2`,
`final-optimized-{low,medium,high}-1` and `final-optimized-ultra-2`.
The `isolated-*` reports cross-check the result away from desktop focus changes.
Do not compare the initial `before-*`, `desktop-before-*`, `luts-*`, `occlusion-*`
or interrupted output directories as a controlled final result.

The primary A/B binary SHA256 is
`a1df32721724b689cce753054fd9125381e11807264604348c690447de785ecd`.
Final source release executable SHA256 at verification:
`e463ef400e8165282ec81b166db6ab69b6d1f916ee94fcfa02e56c67b9dfbe59`.

Temporary switches in the A/B binary select reference, LUT-only, LUT+contact or
all changes. They are absent from final source and are not a public CLI.

A normal new benchmark can be reproduced with:

```sh
prime-run cargo run --release -p f1-the-game -- --assets ./assets \
  --track spa --car rb6 --camera chase --size 1920x1080 --graphics high \
  --mute --benchmark /tmp/f1-new-benchmark --benchmark-seed 24301
```

Set `F1_PROFILE=1` to collect matching per-pass diagnostics. A fresh output folder
is required. Run without Blender or compilers for meaningful timing comparisons.

## Validation

- cargo test --workspace --lib --bins: 98 passed, 13 ignored, zero failed.
- cargo clippy --workspace --all-targets -- -D warnings: passed.
- cargo fmt --all -- --check: passed.
- Actual Vulkan rendered_game_flows: passed. Injected camera cycling,
  pause/resume, camera editing, recovery and HUD/map variations; inspected
  moving cockpit capture showing the live wheel display.
- New regression test: chase -> cockpit -> TV -> chase resumes/stops only
  the LCD camera while preserving the scene camera.

The High six-sample image comparison has mean absolute RGB difference
0.412/255; at most 0.042% of pixels differ by more than 8/255. Ultra is
0.203/255 and 0.026%, respectively. Visual inspection preserves road detail,
car grounding/contact shadows, scenery and the sky. All asset fingerprints,
final simulation hashes and six sample hashes match within each pair.

## Next work

Cache the camera-independent transmittance/multiscattering LUTs in Bevy itself
rather than forking its private render systems here. Profile the finished GEN22
and full LOD field once its asset rebuild is stable, including texture/VRAM and
shadow-vertex cost. Longer runs are needed for reliable frame-tail comparisons
on this shared development desktop.
