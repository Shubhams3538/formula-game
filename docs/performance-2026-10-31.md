# GEN22 GPU performance and paint, 2026-10-31

## Hardware and workload

Ryzen 5 5600H, RTX 3050 Laptop 4 GiB, NVIDIA 615.78.08, Vulkan through
XWayland. Release builds, Spa, GEN22, McLaren, TV camera, player slot 6,
seven opponents, uncapped, muted. `WINIT_X11_SCALE_FACTOR=1.5` makes the
1280x720 logical window render at 1920x1080 physical pixels. Every accepted
report records that physical resolution.

The previous benchmark replaced the requested field with two cars. This change
preserves the selected field and player camera, checks all pairs for contact,
and fingerprints the selected team's per-part textures. Its moving workload
uses 80 m initial spacing so the complete 20 seconds remain contact-free.
The player's slot is automated, giving eight AI drivers during measurement
but the full player mesh and seven opponent meshes. This is distinct from a
packed race start. `--benchmark-grid` separately holds the actual default grid
stationary for 20 seconds, with the player camera and all eight cars.

Use three runs and the median of each reported metric. GPU memory in the final preset matrix is total
adapter memory sampled every 50 ms, including loading and capture, rather
than only steady race residency. It includes unrelated desktop residency.
`contention.log` records compute processes every two seconds. Early first
stage runs used 500 ms telemetry and have no contention log; their FPS is
included in the three-run stage medians, but their peaks do not establish
loading headroom. The OOM trace and final preset matrix use 50 ms telemetry. Loading, warm-up
and screenshot readback are excluded from frame-time measurements. The 1% low
is the reciprocal of the mean frame time of the slowest 1% of frames.

Evidence, source snapshots, executable snapshots, per-run JSON/CSV/PNGs,
telemetry and scripts are under `/home/radhey/.cache/f1-perf-20261031`.
`/tmp/f1-perf-20261031` is a symlink there. The reference snapshot preserves
all pre-existing work; no Git reset, stash, stage or commit was used.

## Results

Three-run medians. VRAM is the median peak across each complete run, in MiB.
Full-texture Ultra fails before its timing window on the reference, so it has
no valid reference FPS or 1% low. Its three observed peaks were 3673, 3806
and 3802 MiB. The user's roughly 45 FPS observation is not substituted for a
matched benchmark result.

| Preset | Average FPS before | After | 1% low before | After | Peak VRAM before | After |
|---|---:|---:|---:|---:|---:|---:|
| Low | 108.81 | 161.69 | 79.83 | 108.74 | 1109 | 663 |
| Medium | 84.66 | 130.74 | 51.19 | 88.18 | 2134 | 1051 |
| High | 67.33 | 100.68 | 47.19 | 65.54 | 2017 | 1134 |
| Ultra | OOM, 3/3 | 77.26 | unavailable | 52.91 | 3802 | 2143 |
| Ultra, packed grid | OOM | 70.68 | unavailable | 47.71 | unavailable | 2017 |

All three packed-grid results exceed 70 FPS: 72.14, 70.64 and 70.68.
The moving Ultra results are 78.96, 77.01 and 77.26 FPS. The highest preset-matrix
Ultra loading peak is 2145 MiB, leaving about 1.9 GiB of the physical 4 GiB.
Steady measured Ultra residency is about 1897 MiB, packed-grid 1899 MiB.
Lower-preset averages and 1% lows improve in this matrix. This is a 20-second
workload comparison, not a claim that every lap or every frame exceeds 70 FPS.

The final measured executable is `cache-game`; accepted reports are
`accepted-*-r*`. Earlier `final-*-r*` names refer to the player-shadow-only
intermediate. The late source changes update log wording and runtime-test
assertions; rendering is unchanged from the measured candidate.

For context, keeping the reference at Ultra but using Half textures completes
at a median **52.31 FPS / 36.86 FPS 1% low**, with 2273 MiB peak residency.
This is a different texture setting, not the Full-texture baseline.

### Recheck after the concurrent artwork update

The other agent replaced McLaren main-body PNGs at full/LOD1/LOD2 after the
matched matrix finished. No geometry, dimensions or material settings changed.
The two executables' High captures were both recaptured with that same new
artwork, so the displayed paint difference does not mix artwork revisions.
Six fresh benchmarks against the final release executable verify the current
assets. Fingerprints match across all six new reports and only those three
PNG fingerprints differ from the original matrix.

| Current artwork workload | Avg FPS | 1% low | Median peak MiB | Highest peak MiB |
|---|---:|---:|---:|---:|
| Ultra moving | 78.45 | 54.99 | 2017 | 2137 |
| Ultra packed grid | 71.81 | 53.88 | 2273 | 2529 |

All three current grid runs also exceed 70 FPS: 72.02, 71.58 and 71.81.
The maximum observed final-build residency including these repeats is
2529 MiB, leaving 1567 MiB of the physical 4 GiB. The loading peak varies
with upload overlap; this is why the table records both the median and maximum.
These reports are `current-*-r*`, with audit summary `current-audit.json`.
The original lower-preset A/B matrix remains internally matched; it was not
repeated after this colour-artwork-only update.

### Retained changes and their gains

| Cumulative moving stage | Avg FPS | 1% low | Shadow GPU ms |
|---|---:|---:|---:|
| BC7, original full-player shadows | 52.51 | 42.85 | 7.099 |
| Enclosed controls excluded from shadows | 62.18 | 45.90 | 4.345 |
| Player LOD1 shadow proxy | 70.27 | 51.19 | 2.599 |
| Final, including opponent proxies and index ordering | 77.26 | 52.91 | 2.264 |

BC7 converts an OOM into a complete full-texture run. No isolated throughput
gain is assigned to compression because the reference cannot finish at Full.
Removing enclosed control shadow work saves about 2.75 ms and adds 9.67 FPS.
The complete player proxy supersedes that intermediate, saving another
1.75 ms of shadows and 8.09 FPS. Combined, player shadow work saves about
4.50 ms and raises the moving average by 33.8% over the BC7-only build.
Normal-strength correction accompanies the proxy stage, so the wall-time
increment is cumulative; the shadow-pass reduction isolates the main cause.

| Cumulative packed-grid stage | Avg FPS | 1% low | Shadow GPU ms |
|---|---:|---:|---:|
| Player LOD1 proxy only | 59.42 | 36.92 | 4.274 |
| Plus opponent LOD2 shadow proxies | 64.22 | 30.42 | 3.022 |
| Plus index-cache ordering | 70.68 | 47.71 | 2.763 |
| Occlusion culling instead of index ordering, rejected | 62.70 | 47.29 | 2.975 |

Opponent shadow proxies add 4.80 FPS, saving about 1.25 ms of shadow work.
Index ordering adds another 6.46 FPS, saving about 1.43 ms of total frame time,
including roughly 0.67 ms prepass, 0.62 ms opaque and 0.26 ms shadow work.
These separately sampled GPU-pass medians are approximate and need not sum
exactly to the difference between total-frame medians. The opponent-proxy
intermediate has a poor 1% low in two runs; this does not carry through to the
final accepted matrix. Occlusion culling's average falls by 1.52 FPS versus
opponent proxies alone, so it is removed.

In the final
packed-grid profile, index ordering reduces the depth prepass to about
1.74 ms, main opaque to 4.36 ms, and shadows to 2.76 ms. High SSAO stays
about 1.67 ms. The one-time light-probe diagnostics retain their last value
after generation stops, so those stale log values are not counted as recurring
passes.


## The failed allocation

The reference Ultra capture fails during loading. A temporary Vulkan loader
shim records `vkCreateImage`, memory requirements and `vkAllocateMemory`.
The fatal resource is a **4096x4096 RGBA8 sRGB texture with 13 mip levels**.
Its memory requirement is **89,480,192 bytes**, aligned to 1024 bytes. The
allocator requests a **268,435,456-byte block** in memory type 1 and receives
`VK_ERROR_OUT_OF_DEVICE_MEMORY`, result -2. Before the failure that type
already held **2,818,572,288 bytes**; device-local, host-visible upload memory
in type 5 held another **1 GiB**. Earlier failed 128 MiB staging requests could
fall back to host memory; the image-pool failure cannot.

The 50 ms sample catches **3802 MiB** total adapter residency. This explains
why a normal-play observation near 1.3 GB does not exclude a loading spike.
The Vulkan limits report `maxMemoryAllocationSize=0xffffffffffffffff` and
`maxBufferSize=0x10000000000`. The failure is not the player's large mesh
exceeding a single-buffer or single-allocation limit. The original images
have empty Vulkan debug labels, so the trace identifies the exact GPU
resource and allocator request, but not a unique source PNG among the
many same-sized imported images.

See `vulkan-trace/baseline-allocations.log`,
`vulkan-trace/baseline-gpu-50ms.csv`, `oom-allocation/game.log`, and
`vulkaninfo.txt`. The first shim's printed memory-property flags were zero
because of a lookup bug; memory-type properties come from `vulkaninfo.txt`.

The final release Ultra capture exits 0 with **zero failed Vulkan memory
allocations**, 325 BC7 image creations, and a **2154 MiB** peak at 50 ms.
See `vulkan-trace/final-allocations.log` and
`verified-ultra-capture/{game.log,gpu.csv,capture.png}`. The fixed shim prints
the correct heap/type properties. Large full-resolution maps are BC7 images
with the same mip count rather than failed RGBA allocations.

Imported RGBA images now keep their complete mip chain and resolution in
BC7 on adapters supporting BC texture compression. Colour images remain
sRGB, while normal, roughness and AO data remain linear. Preparing them after
asset events and before render extraction avoids first uploading the raw
RGBA image. Immutable imported image pixels are released from the main
world after extraction. Content-addressed blocks are cached in
`$XDG_CACHE_HOME/f1-racecraft/bc7-v1`, falling back to
`~/.cache/f1-racecraft/bc7-v1`. The cache key includes dimensions, mip count,
format and pixels. Unsupported adapters retain uncompressed textures.

The first cold Ultra encoding took about 70 seconds in an exploratory run.
Warm-cache runs reuse the blocks. This fixes GPU loading pressure but does
not solve cold-start latency; shipping precompressed assets remains useful
future work. No livery artwork or generated car asset was changed here.

## GPU throughput

The dominant measured cost was rendering the 2.24M-triangle player through
four shadow cascades. The main camera still draws that full model. The sun
instead renders the existing roughly 140k-triangle LOD1 scene on a dedicated
render layer excluded from the main camera. It follows the same car,
steering, uprights and DRS transforms. The full player receives shadows.
Opponents use the existing approximately 45k-triangle LOD2 as a sun-only
silhouette, while their visible near and far LODs keep the selected settings.
These proxies avoid new geometry assets or texture loads when both LODs are
already used by the field. Proxy drivers do not join player morph/control hooks.
Packages without LOD2 retain the opponents' visible-mesh shadow casters.

The exported triangle order also had poor post-transform vertex-cache reuse.
On loading each GEN22 primitive, meshoptimizer now reorders only its indices.
Vertex buffers, every triangle and its winding, UV seams, materials, skinning
and morph targets remain unchanged. The packed-grid main pass dropped from
about 5.85 million to 3.60 million vertex shader invocations. This work happens
once before the initial GPU extraction, with no per-frame mesh processing.
The current generated GEN22 materials are all opaque.

Ultra retains full texture resolution, 4096 shadow maps, four cascades,
High SSAO, TAA and the existing post-processing settings. No preset setting
was reduced for this result. Shadow silhouettes are approximations; the
external review and cockpit captures are the evidence for its visual cost.

## Paint response

The team livery hook replaces only base-colour textures and shares material
instances per team and source material. Imported glTF colour and PNG livery
maps already use sRGB. Normal/AO maps are linear. Global exposure,
tonemapping, colour grading and environment intensity were not changed.

Two material issues were found. Bevy 0.20.1 ignores glTF
`normalTexture.scale`, treating authored paint strength 0.16 and LOD strength
0.35 as 1.0. A loader extension now records the authored strengths and applies
them to tangent-space slopes before filtering/compression. Separately,
`source_paint_*` materials mix painted artwork and exposed dark composite
under a uniform clearcoat of 0.16. That neutral reflection lobe brightened
black areas and desaturated the AI cars. Paint materials now use no uniform
clearcoat and reflectance 0.4 instead of the default 0.5. Base maps, roughness,
AO and non-paint cockpit/control materials retain their authored values.
A sky patch changes by about 0.005/255 per channel, while a road patch changes
by about 0.4–0.5/255 in the final matched High captures. The paint correction
leaves the scene's global colour response intact.
The normal-strength correction alone had little effect on the washout;
removing the uniform reflection lobe produced the visible correction.

Matched deterministic 1920x1080 captures at High, recaptured with the same
new artwork on both executables after the other agent's livery update:

- Before: `verified-before-high-capture/capture.png`.
- After: `verified-high-capture/capture.png`.
- External and cockpit review: `verified-rendered_car_review/*.png`.

These paths are relative to the evidence directory above. Existing opponent
fallbacks still display the supplied Audi artwork where installed per-part
team artwork is absent. That is separate from material washout and the
reserved livery work was not modified.

## Rejected experiments and measurement exclusions

- Moving the mip preparation before extraction alone still OOMed.
- A 64 MiB/frame upload budget still OOMed without compression and later
  caused blank LCD/fender textures. It is removed.
- Suppressing duplicate image-modification events alone did not fix OOM.
  Untracked preparation is retained to avoid redundant uploads after the
  initial Added event has already scheduled extraction.
- Paint reflectance 0.25 with no coat was too dark; 0.4 is the retained value.
- GPU occlusion culling added overhead without reducing the dominant visible
  geometry enough. It is excluded from the final build.
- The first index-cache grid run had twenty frames above 40 ms and concurrent
  CPU-heavy livery mapping. Its 66.8 FPS / 8.4 FPS low is retained as exploratory
  evidence, and fresh repeats determine the final median. No frames are
  removed from any accepted run.
- Excluding enclosed cockpit controls from shadows was an intermediate
  improvement, superseded by the LOD1 sun-only proxy.
- `before-*-r*` runs followed entrant 0 instead of the player after field
  expansion. They are discarded. Reference runs are `ref-*-r*`.
- Early 25 m moving-field spacing failed the full-drive clearance preflight.
  It was not made valid by weakening the contact check.

## Verification and limits

Passed against the final source:

- `cargo test --workspace --lib --bins`: 68 game and 39 sim tests pass;
  13 asset/display-dependent tests remain ignored by that command.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo fmt --all -- --check` and `git diff --check`.
- `cargo build --release` and the release renderer-test build.
- `rendered_car_review`: player and opponent shadow-proxy triangle budgets,
  caster flags, near/far visibility and player controls/morph counts.
- `rendered_game_flows` with GEN22 and RB6: real gameplay, all three cameras,
  pause/resume, recovery, camera editor, both HUDs and both map modes.
- `rendered_wheel_drs_controls`: real input, steering, DRS controls, GLB morphs
  and LCD presentation.
- `all_converted_tracks_have_clean_repeatable_moving_workloads`: Spa RB6 and
  GEN22 full-field preflights, the tracks enabled in this normal build.
- 42 completed reports pass the dimensions, eight-car count, matching assets
  and moving-state/sample-hash audit. No Blender process appears in the
  available contention logs. The two extra player-proxy grid comparisons
  also complete at matching resolution and field size.

Renderer screenshots and logs are `verified-rendered_*` and
`verified-rb6-flows*` under the evidence directory. The ignored full-tree
menu-selection and CLI-exit renderer suites were not run in this task.
Live graphics-menu switching was not separately automated; the existing
unit coverage for settings and benchmark menu arguments passes.
Linux Vulkan is the runtime boundary tested.
Other GPUs, unsupported-BC adapters, Windows and macOS are not tested.
No networking or historical-engine changes apply to this task.
