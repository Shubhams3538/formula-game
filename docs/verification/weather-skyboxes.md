# Dry weather skyboxes

Two visual weather modes: Clear sky and Storm. Storm has cloud cover, reduced
scene illumination, spaced single lightning flashes and delayed thunder.
Both keep the existing dry surface and tyre physics.

## Applicable paths

- Track page, before the race and while paused: selected weather is
  applied through `Frontend::draft.weather`, including the reverse transition.
- Saved preferences: typed Weather, defaults to Clear for older files; explicit
  `--weather clear|storm` overrides saved state.
- Quick race, frame capture and benchmark: CLI Weather and selected weather
  forwarded into the benchmark child process.
- Rendering: only the primary race camera gets the skybox, matching generated
  environment maps and weather ambient light. The car preview retains its
  studio lighting. Sun direction follows the active track.
- Graphics presets: Low/Off use 128-pixel cube faces, Medium 256, High/Ultra 512.
  A source change resets environment-map settling before filtering is frozen.
- Audio: delayed thunder is synthesized in the existing effects/master bus,
  follows mute and session pause; changing back to Clear stops its envelope.
- Packaging: PNG cube faces are embedded in the executable, so existing Linux
  and Windows asset bundles need no additional sky downloads.
- Physics, network protocols and rain/wet tyre rules do not apply. Weather
  changes neither simulation state nor surface grip.

## Evidence

- Workspace tests: 82 game and 44 sim passed after weather implementation.
  New checks cover all embedded cubemap dimensions/view types, strict CLI,
  old preference defaults, delayed thunder/pause clock, effects/master mute,
  and stopping thunder when returning to Clear.
- Weather implementation passed Clippy with warnings denied and formatting.
  A later concurrent menu change introduced a `too_many_arguments` warning;
  parent owns final integrated checks. Latest map-settle counter correction
  requires the parent's final integrated Cargo check.
- Release built and native Vulkan captures on RTX 3050 were inspected:
  `/tmp/f1-weather-clear.png` and `/tmp/f1-weather-storm-final.png`.
  Clear has blue sky and sunlight. Storm has dark cloud layers with readable
  dry road/car lighting. No GPU/skybox validation errors were observed.
- Timed storm capture `/tmp/f1-weather-lightning.png` missed the brief flash;
  it does not prove native flash appearance. Timer behavior has unit coverage.
- Native captures were muted. Thunder listening quality, interactive weather
  reversal/persistence, and Windows runtime were not manually verified.
- One capture had host ALSA backend POLLERR messages even while muted. The
  process saved the screenshot and exited successfully; audio output remains
  an explicit runtime limit.

CC0 sources, credit, checksums and rebuild command are in `game/sky/CREDITS.md`
and `tools/sky/build_skyboxes.py`.
