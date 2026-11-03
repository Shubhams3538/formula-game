---
name: F1 Racecraft HUD
description: Native charcoal instruments for the Bevy racing display.
colors:
  digital-top: "#222524"
  digital-bottom: "#0a0d0d"
  digital-edge: "#54574f"
  digital-ink: "#eaf0d5"
  digital-muted: "#a3ac99"
  digital-accent: "#f63d31"
  analog-top: "#222324"
  analog-bottom: "#0c0e10"
  analog-edge: "#555859"
  analog-ink: "#ebe9db"
  analog-muted: "#aeb1ab"
  well: "#090a0c"
  well-top: "#030708"
  well-bottom: "#0a1113"
  segment-off: "#1c1d1c"
  red: "#ed212b"
  green: "#40e673"
  amber: "#ffb826"
  blue: "#4d99ff"
  purple: "#b85cff"
  tyre-yellow: "#ffdc00"
typography:
  label:
    fontFamily: "Titillium Web"
    fontSize: "13px"
    fontWeight: 700
  body:
    fontFamily: "Titillium Web"
    fontSize: "20px"
    fontWeight: 600
  wheel-label:
    fontFamily: "Titillium Web"
    fontSize: "22px"
    fontWeight: 700
spacing:
  readout-inset: "12px"
  instrument-pad: "14px"
  instrument-gap: "10px"
  corner-margin: "24px"
components:
  timing-strip:
    backgroundColor: "{colors.well}"
    width: "278px"
    height: "248px"
  pass-gained:
    backgroundColor: "{colors.well}"
    textColor: "{colors.green}"
    width: "278px"
    height: "66px"
  pass-lost:
    backgroundColor: "{colors.well}"
    textColor: "{colors.amber}"
    width: "278px"
    height: "66px"
  readout-well:
    textColor: "{colors.digital-ink}"
  drs-open:
    backgroundColor: "{colors.green}"
    rounded: "6px"
---
# Design System: F1 Racecraft HUD

## Overview

**Creative North Star: "Charcoal cockpit instruments"**

A shared instrument set for the screen HUD and the cockpit wheel LCD. Digital mode uses illuminated segments, olive-white ink and chamfered charcoal housings. Analog mode uses warm-white Titillium Web, metal-rimmed dials and a red needle. Both keep the road and physical wheel visible.

This record covers the racing HUD, timing and wheel displays in the current checkout. It does not define menus or a new product identity. PRODUCT.md records the confirmed readability problem, scope and constraints; README.md, the native painters and docs/verification/hud-stability.md supply the implementation evidence.

**Key Characteristics:**

- Two material finishes with shared semantic colors.
- Fixed numeric columns and persistent sector rows.
- Corner instruments around an open driving view.

## Colors

Charcoal material layers carry pale instrument ink; saturated colors report racing state. Frontmatter colors transcribe the Rust RGB constants. They are a documentation mapping, not a new runtime theme file.

### Primary

- Red marks braking, contact, cooked tyres, positive lap delta and the analog tachometer needle. Digital housing accent is a separate warmer red used for the player stripe.

### Secondary

- Green marks throttle, optimal tyre temperature, nonpositive live lap delta, position gained and open DRS.
- Blue marks cold tyres, live sectors and the final shift-light stage.
- Amber marks position lost, slower completed sector comparisons and shift refusal warnings. Tyre-yellow specifically means lift and coast.
- Purple marks personal bests and nonpositive completed sector comparisons.

### Neutral

- The digital family is slightly olive; the analog family is warm gray. Use each finish's top, bottom, edge, ink and muted colors together.
- Wells are near black. Digital unlit segments remain visible against dark readouts; the digital timing tower uses opaque dark rows. Analog tower rows may be translucent, and the player row inverts to pale gray.
- Team identity colors come from race identities, independently of this semantic palette.

**The Shared Meaning Rule.** Keep semantic colors consistent across both finishes and both egui contexts. Material accent colors do not replace telemetry state colors.

## Typography

Bundled Titillium Web SemiBold supplies the regular face at weight 600; Bold supplies labels at weight 700. Both egui proportional and monospace family names resolve to the bundled regular face. There is no separate monospace typeface or system display face.

The frontmatter records recurring text roles, not a modular type scale. On-screen labels commonly use 13 or 14 logical units; tower identities use 20. Numeric readout heights vary by task. At scale 1, timing uses 28 for the running clock, 16 for last/best, 20 for delta and 17 for sector values. Digital mode draws these with seven-segment geometry when every character is supported. Analog mode uses SemiBold at 1.3 times the requested readout height. Unsupported digital strings also use that text path.

The wheel texture has its own denser hierarchy: labels at 22, side values at 62 and gear at 250. These are texture coordinates, not a screen-wide display ramp. Text and segment numbers align by capital midline through `text_mid` and `readout`; no CSS line-height or tracking rule exists.

**The Fixed Column Rule.** Right-align values in reserved columns. Fit long identity text to its allotted width rather than letting it move the numeric columns.

**The Timing Legibility Rule.** Timing strip labels, sector labels/status and pass headings stop shrinking at 11 logical units. Sector readout heights stop at 12. Pass rival text starts at a minimum of 12 before long-name fitting, which may reduce it further. These exceptions preserve the panel position, width and fixed columns.

## Layout

All frontmatter dimensions written as `px` represent native logical drawing units at scale 1. They do not prescribe CSS pixels or physical display pixels. HUD geometry scales by `min(height / 1080, width / 1500)`. There are no breakpoint swaps, browser media queries or minimum geometry scale clamp. Timing typography has the minimum sizes recorded above; the panel positions, widths and row spacing continue to use the shared scale.

The timing tower anchors at the upper left with a 24-unit margin and 278-unit width. Its entrant rows advance by 28; timing follows after an 8-unit gap. The 248-unit timing strip has 12-unit side insets. Its three sector rows advance by 28 and reserve separate label, time and status columns. A 66-unit pass notice sits 12 units below the timing strip.

The unboxed minimap anchors upper right, sized at 240 for Full circuit or 330 for Nearby. Instruments anchor at the bottom corners. The digital main housing is 508 wide; its common pad is 14 with a 10-unit gap. Analog mode uses a 165-unit-radius tachometer at bottom right. Tyres and aero occupy the lower left. Tall or narrow windows shrink these anchored groups by width, keeping the wheel in view; they do not reflow into a vertical dashboard.

The full wheel LCD is a separate 768 by 448 render texture. Its pad is 14, gaps are 10, and three 124-unit cells flank the central gear. Wheels without an LCD use a 156-unit-high compact pod for shift lights, speed, gear and delta. Projection onto the wheel and texture mipmaps affect its final apparent size.

## Elevation & Depth

Native gradients describe the charcoal housings, recessed readout glass, lamp lenses and turned metal dial rims. Plates have a fine edge stroke. Analog dials also cast a soft black offset shadow at 2 units right and 5 down, with alpha 90/255. This is physical instrument shading, not a general card-shadow system. Timing and pass notices follow the selected instrument finish. Digital mode uses chamfered charcoal housings and recessed readouts. Analog mode adds a narrow turned-metal bezel around its matte chronograph face, with warm-white typographic times.

**The Instrument Depth Rule.** Use the selected finish consistently for instruments, timing and position notices. Keep numeric columns fixed and the timing face rectangular so lap and sector values remain easy to scan.

## Shapes

Housings use eight-sided chamfered plates, cut by 6 units in digital mode and 12 in analog mode. Readout wells use a 3-unit chamfer. These are clipped corners, not rounded-corner tokens. Dials and lamp lenses are circular; pedal rails and DRS badges use local rounded rectangles. The on-screen DRS badge uses a 6-unit radius. There is no universal card radius.

## Components

### Timing strip and sector rows

A fixed readout under the tower carries running lap time, LAST, BEST, VS BEST and S1 through S3. Its housing follows the selected HUD style: segmented charcoal readouts in Digital cockpit, and a metal bezel with warm-white typographic times in Analog telemetry. Running lap and active sector clocks show tenths; completed laps and sectors retain milliseconds. Delta displays hundredths against a recorded best lap. Until a reference exists it says SET A LAP. The active sector has a blue row and LIVE label. Stored sectors remain visible and say LAST when from the previous lap; FIRST means no previous best-sector comparison, while unavailable values use dashes. Completed sector comparisons use purple for nonpositive and amber for positive values. The build uses a 200 ms delta average.

### Position notice

One compact instrument panel under the timing strip shows POSITION GAINED in green or POSITION LOST in amber, rival identity and new position. The heading uses `max(14 × scale, 11)` and rival text uses `max(17 × scale, 12)` before fitting to its reserved column. A thin remaining-duration bar runs along the bottom. The notice requires 108 consecutive simulation ticks, or 300 ms, with at least one metre clearance; it lasts 900 ticks, or 2.5 seconds, and clears at finish. These are presentation rules, not changes to race order.

### Instrument readouts and wheel LCD

Speed and numeric RPM average 100 ms; tyre temperature, wear, aero effects and gaps average 250 ms. Samples come from 360 Hz physics ticks, and averaged numbers publish every 18 ticks, or 20 Hz. Screen and wheel consume the same stored values. Pausing adds no samples. Gear, pedal inputs, limiter, shift lights, tyre lock/slide outlines and DRS states use immediate data. The digital gear aperture stays neutral except for the existing shift/refusal indications.

### Tyres, pedals and DRS

Tyre status compares the compound-specific temperature window: cold blue, optimal green, lift and coast yellow, cooked red. Digital tyre modules show one large temperature per axle, with wear beside the FRONT or REAR header. There is no separate thermal-state text row. Analog mode uses two keyed needles on one temperature scale; FRONT and REAR labels match their needles, with axle wear beneath the sweep. It does not repeat temperatures as numeric values. Wheel footprints gain white outlines for locking or sliding. Brake is red and throttle green. DRS is absent on cars/tracks without the feature; otherwise it is dim when unavailable, outlined green when ready and filled green when open. These racing readouts are not interactive buttons and have no hover or focus behavior.

## Do's and Don'ts

### Do:

- Do use the bundled Titillium Web faces for words and the native segment painter for digital numbers.
- Do share averaged telemetry between the HUD and wheel LCD while keeping gears, inputs and warnings immediate.
- Do retain all three sector rows and distinguish LIVE, LAST, FIRST and unavailable values with text.
- Do reserve the center driving view and keep position notices beside the timing tower.

### Don't:

- Don't substitute a system display font or a monospace font for the native digit geometry.
- Don't smooth completed lap or sector times, or treat live gap estimates as official results.
- Don't infer browser breakpoints, hover controls or CSS layout rules from these native instruments.
- Don't treat the smallest incidental caption sizes or unlimited downscaling as a legibility target for new HUD work.

Evidence: `game/src/hud.rs`, `game/src/hud/timing.rs`, `game/src/hud/telemetry.rs`, `game/src/hud/instruments.rs`, `game/src/hud/wheel.rs`, `game/src/canvas.rs`, `game/src/wheel_dash.rs`, `PRODUCT.md`, `README.md` and `docs/verification/hud-stability.md`. Native review used `/tmp/f1-hud-stability/review-contact.jpg` and `/tmp/f1-hud-stability/timing-contact.jpg`. Those contact sheets cover wide/narrow digital and analog timing plus the full wheel LCD; they do not establish readability at every window size or verify live temporal behavior.

Not canonized: incidental tiny captions elsewhere and unrestricted geometry shrinkage are not minimum legibility standards. No new eyebrow, glyph-icon or system-display-font convention is introduced.
