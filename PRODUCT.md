# F1 Racecraft racing displays

F1 Racecraft is a Bevy racing game with a deterministic 360 Hz simulation. During a race, drivers read the on-screen HUD and the cockpit wheel dashboard while keeping their attention on the road.

This component change addresses the user's report that pass/overtake messages, sector timings and dashboard values jump too much. It keeps the established digital and analog instrument styles and makes live values readable through short tick-based averages.

- Show one compact, confirmed position-change notice beside the timing tower.
- Keep all three sector rows visible, distinguish the active sector and retain completed splits across lap boundaries.
- Share averaged telemetry between the HUD and wheel displays. Preserve immediate gear changes, pedals and warning lights.
- Keep completed lap and sector times exact. Do not change physics, official race order or race rules.
- Keep the central driving view clear and timing labels readable in narrow windows.

The verification record is `docs/verification/hud-stability.md`; `DESIGN.md` describes the shipped instruments. Automated and native renderer checks cover the implementation. Human judgement of the smoothing during a complete race remains untested.
