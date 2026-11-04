# HUD style and tyre information refinement

The screen tyre instruments now show each temperature once. Digital cockpit keeps its large per-axle temperature and moves wear beside the FRONT/REAR header, removing the separate thermal-state text row. Analog telemetry keeps its two temperature needles and replaces repeated temperature numbers below the sweep with colour-keyed FRONT/REAR labels. Wear and per-wheel lock/slide warnings remain visible.

The timing strip and pass notice use the selected instrument finish. Digital cockpit uses chamfered charcoal housings, recessed readouts and segment numbers. Analog telemetry uses a narrow metal bezel, matte face and warm-white typographic times. The rectangular timing layout retains the running lap, last/best, delta and all sector states.

| Surface | Applicability and evidence |
|---|---|
| Digital screen tyre module | Changed; no separate COLD/OPTIMAL row, wear beside axle heading |
| Analog screen tyre gauge | Changed; one shared temperature scale and two keyed needles, no repeated temperature numbers |
| Lap timing and pass notice | Both instrument finishes changed; values and timing semantics preserved |
| Standings | Existing layout-specific numerals and palette retained |
| Cockpit wheel LCD | No change requested; retains its compact independent layout |
| Physics, lap/sector timing, telemetry averaging | No behavior change |
| Network and archived engine | Not applicable to this local Bevy drawing change |

Focused HUD tests passed: 11 passed, with the native renderer fixture ignored by the normal test command. The thermal-window color boundary tests remain; the removed text-label assertions were deleted with the unused label method.

The existing native `hud::proof::rendered_timing_and_pass_states` fixture passed at 1920×1080 on the NVIDIA RTX 3050 Vulkan renderer. It switched between Digital cockpit and Analog telemetry and completed all four captures: digital position gained, analog position lost, first-lap empty state and the actual wheel LCD. The final run used an isolated null ALSA output because the host audio backend had reported errors in an unrelated capture; this is visual verification, not an audio test.

The first reduced contact-sheet inspection found the analog FRONT key too close to the 40°C graduation. Moving the axle keys down resolved that overlap in the final confirmation capture. The final digital and analog contact sheet is `/tmp/f1-hud-style-final/hud-style-contact.jpg`; individual captures and the run log are `/tmp/f1-hud-style-final/` and `/tmp/f1-hud-style-final.log`.

The fixture uses synthetic timing states and freezes physics. It proves presentation and renderer integration, not racing events. Human judgement during a complete race, extreme tyre wear and narrow-window layout are not covered by this refinement's native capture.
