---
name: F1 Racecraft native race setup
description: Scoped Track and Team extension of the charcoal instrument settings menu.
colors:
  race-red: "#e62330"
  backdrop: "#141518"
  housing: "#25262a"
  housing-edge: "#4a4c53"
  well: "#090a0c"
  well-rim: "#383a40"
  dim-text: "#a0a6b2"
typography:
  body:
    fontFamily: "Titillium Web"
    fontSize: "17px"
    fontWeight: 600
  button:
    fontFamily: "Titillium Web"
    fontSize: "17px"
    fontWeight: 600
rounded:
  control: "6px"
  housing: "12px"
spacing:
  horizontal: "14px"
  vertical: "8px"
components:
  choice:
    backgroundColor: "{colors.well}"
    rounded: "{rounded.control}"
    typography: "{typography.button}"
    padding: "6px 14px"
  choice-selected:
    backgroundColor: "{colors.race-red}"
    textColor: "#ffffff"
    rounded: "{rounded.control}"
    typography: "{typography.button}"
    padding: "6px 14px"
---

# Design System: Native race setup

## Overview

**Creative North Star: "Charcoal racing instruments"**

This component record covers the shipped Game play settings in `game/src/menu.rs` and `game/src/menu/car_preview.rs`. It retains the existing Bevy/egui instrument world. Root `DESIGN.md`, `PRODUCT.md` and `.impeccable/design.json` describe the separate racing HUD and are outside this record.

**Key Characteristics:**

- One Game play entry before a race and while paused, with Car setup, Driving assistance and Track subtabs.
- Explicit team names with drivers beneath, beside a circuit outline and rendered selected car.
- Charcoal housings, black inset controls and red selected states.

## Colors

### Primary

Race red marks selected navigation, team and driver choices, slider progress and the circuit start point.

### Neutral

Backdrop surrounds the housing. Housing edge separates the panel from the backdrop; well and well rim make controls and circuit previews read as inset windows. Dim text carries descriptions and form labels. Team selection uses the shared palette rather than team-colored swatches.

## Typography

Titillium Web SemiBold is the installed proportional face for body text and controls. The shared body/button size is the frontmatter value; native egui logical points are represented as CSS pixels for token portability. Page titles use the existing egui heading style. No new display family is introduced.

## Layout

Track holds circuit, race distance, grid, tyres and aero. Car setup holds car selection, then the 2022 team grid in two columns and the selected team's drivers beneath. Red Bull RB6 instead shows its fixed Red Bull Racing identity.

Above 980 available content points, choices and previews occupy equal columns. At or below that width, choices precede previews in one vertically scrollable page. Game play has one sidebar entry; its three subtabs form the content header, without a repeated Game play title or description. Navigation uses a page selector when its available width is below 980 points or pane height is below 560 points; otherwise it uses the left rail. This navigation selector is separate from team selection. Race actions remain in the bottom panel outside the page scroll.

**The Preview Order Rule.** Show the circuit outline first and the actual selected car beneath it. Do not repeat the circuit name, distance, lap count or car summary in this preview column.

## Elevation & Depth

Panel and control depth comes from charcoal fills and thin rims. The car image adds real lighting and ground shadows from an isolated Bevy studio scene; these shadows are part of the rendered asset view, not a UI shadow token.

## Shapes

Controls and both preview frames share gently rounded corners. The outer housing uses the larger radius. The circuit is a fitted pale closed line with a red start marker, centered inside its black well.

## Components

Team choices are full-width named buttons with wrapping labels, arranged in two columns. Drivers are wrapping selectable names with race numbers below the grid. Changing teams selects that team's first driver; reselecting the same team preserves its selected driver. Team choice has no dropdown or color swatches.

Controls rest in black wells. Hover brightens the rim and fill; pressed or selected states use race red. Car choices use radio controls. The circuit retains its existing dropdown.

The car preview displays the selected asset at a 12:5 aspect ratio from a 960 × 400 render target. Car or tyre compound changes replace the preview scene; team and driver changes preserve it. The camera runs only on Track and Car setup before a race or while paused. It never replaces or advances the race. Loading and failed-asset states show readable messages within the image.

**The Asset Truth Rule.** The 2022 car uses this branch's neutral stock paint. Team selection changes race identity and does not synthesize a team livery. The RB6 preview uses its own selected asset.

## Do's and Don'ts

- **Do** keep team names visible and drivers immediately beneath their team choices.
- **Do** preserve the circuit-outline/car order in wide and stacked layouts.
- **Don't** imply that selecting Ferrari or another team repaints the neutral 2022 asset.
- **Don't** replace the actual car render with an illustration or duplicate metadata.

Not canonized: the build's muted uppercase section captions are incumbent eyebrow-like labels, not a pattern for future screens to inherit under the craft floor.

Evidence: final native wide and portrait captures under `/tmp/f1-track-team-proof-20261103-final/`, including Track, Team, RB6 and portrait scroll-end views. Independent finish review returned ship with no material fixes in wide or portrait. The compact technical check remains pending; native Windows rendering is untested. This documentation change adds no runtime verification.
