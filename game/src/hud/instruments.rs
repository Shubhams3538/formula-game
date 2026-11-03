use super::*;
use racecraft_sim::car::powertrain::ShiftRefusal;

pub(super) fn draw(hud: &Hud, style: &HudStyle, session: &Session, layout: Layout, scale: f32) {
    match layout {
        Layout::Segments => segments(hud, style, session, scale),
        Layout::Gauges => gauges(hud, style, session, scale),
    }
}

/// What the DRS badge shows for the player's car.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum DrsLamp {
    /// The car has DRS but may not open it here.
    Off,
    /// In a zone with DRS earned: press to open.
    Ready,
    Open,
}

impl DrsLamp {
    pub(super) fn of(session: &Session) -> Option<Self> {
        let car = &session.race.entrants[session.player].car;
        if !car.spec.aero.has_drs()
            || (session.race.drs_zones.is_empty() && !car.assists.drs_anywhere)
        {
            return None;
        }
        Some(if car.drs.flap > 0.0 {
            Self::Open
        } else if car.drs.available {
            Self::Ready
        } else {
            Self::Off
        })
    }
}

/// DRS tell-tale shared by every cluster: unlit when the car may not open
/// it, outlined green when it can, solid green while the flap is open.
fn drs_lamp(hud: &Hud, style: &HudStyle, rect: egui::Rect, text: f32, lamp: Option<DrsLamp>) {
    let radius = rect.height() * 0.22;
    let unlit = Color::from_rgb(20, 22, 23);
    let (fill, ink) = match lamp {
        None => (unlit, Color::from_gray(55)),
        Some(DrsLamp::Off) => (unlit, Color::from_gray(85)),
        Some(DrsLamp::Ready) => (unlit, GREEN),
        Some(DrsLamp::Open) => (GREEN, Color::from_rgb(8, 9, 10)),
    };
    let (x, y, w, h) = (rect.left(), rect.top(), rect.width(), rect.height());
    hud.fill_round(x, y, w, h, radius, fill);
    if lamp == Some(DrsLamp::Ready) {
        hud.stroke_round(x, y, w, h, radius, (h * 0.08).max(1.5), GREEN);
    }
    hud.text_mid(
        &style.bold,
        text,
        (rect.center().x, rect.center().y),
        Align::Center,
        ink,
        "DRS",
    );
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
}

fn segments(hud: &Hud, style: &HudStyle, session: &Session, scale: f32) {
    let ink = hud.finish.colors().ink;
    let width = hud.width();
    let height = hud.height();
    let race = &session.race;
    let player = &race.entrants[session.player];
    let car = &player.car;
    let lap = (player.timing.laps_completed + 1).min(race.laps);
    // ---- Dash (bottom right): RPM LEDs, lap / gear / speed, lap time and
    // position, then pedal traces.
    let pad = 14.0 * scale;
    let gap = 10.0 * scale;
    let dash_w = 508.0 * scale;
    let led_h = 34.0 * scale;
    let main_h = 78.0 * scale;
    let lower_h = 46.0 * scale;
    let dash_h = pad + led_h + gap + main_h + gap + lower_h + 48.0 * scale;
    let px = width - dash_w - 24.0 * scale;
    let py = height - dash_h - 24.0 * scale;
    housing(hud, px, py, dash_w, dash_h, scale);
    for (x, y) in [
        (px + 7. * scale, py + 7. * scale),
        (px + dash_w - 7. * scale, py + 7. * scale),
        (px + 7. * scale, py + dash_h - 7. * scale),
        (px + dash_w - 7. * scale, py + dash_h - 7. * scale),
    ] {
        hud.disc(
            (x, y),
            3.5 * scale,
            Color::from_gray(97),
            Color::from_gray(25),
        );
        hud.line(
            (x - 1.5 * scale, y),
            (x + 1.5 * scale, y),
            scale,
            Color::from_gray(12),
        );
    }
    let inner_x = px + pad;
    let inner_w = dash_w - pad * 2.0;

    let engine = &car.spec.engine;
    let rpm_fraction = ((car.powertrain.rpm - engine.idle_rpm)
        / (engine.rev_limit - engine.idle_rpm))
        .clamp(0.0, 1.0) as f32;
    // Shift lights: green, red, then blue at the optimum change point.
    let led_y = py + pad;
    hud.fill_round(inner_x, led_y, inner_w, led_h, led_h * 0.5, WELL);
    hud.stroke_round(
        inner_x,
        led_y,
        inner_w,
        led_h,
        led_h * 0.5,
        1.0 * scale,
        WELL_RIM,
    );
    let lights = 15;
    let led_pitch = (inner_w - led_h) / (lights - 1) as f32;
    let flash = car.powertrain.limiter && (race.time * 12.0).sin() > 0.0;
    for light in 0..lights {
        let threshold = 0.55 + 0.45 * light as f32 / lights as f32;
        let lit = rpm_fraction >= threshold;
        let color = if light < 5 {
            GREEN
        } else if light < 10 {
            ACCENT
        } else {
            BLUE
        };
        let color = if flash {
            BLUE
        } else if lit {
            color
        } else {
            Color::from_rgb(80, 83, 69)
        };
        let centre = (
            inner_x + led_h * 0.5 + light as f32 * led_pitch,
            led_y + led_h * 0.5,
        );
        hud.circle(centre, 12.0 * scale, Color::BLACK);
        hud.disc(centre, 10.5 * scale, color, color.gamma_multiply(0.35));
        // Lens highlight.
        hud.circle(
            (centre.0 - 3.0 * scale, centre.1 - 3.5 * scale),
            3.0 * scale,
            rgba(1.0, 1.0, 1.0, if flash || lit { 0.45 } else { 0.08 }),
        );
    }

    let main_y = led_y + led_h + gap;
    let gear_w = 92.0 * scale;
    let side_w = (inner_w - gear_w - gap * 2.0) * 0.5;
    let gear_x = inner_x + side_w + gap;
    let speed_x = gear_x + gear_w + gap;
    // Lap / laps.
    well(hud, inner_x, main_y, side_w, main_h, scale);
    caption(
        hud,
        style,
        14.0 * scale,
        (inner_x + 10.0 * scale, main_y + 5.0 * scale),
        Align::Left,
        "LAP",
    );
    let value_y = main_y + 27.0 * scale;
    let value_size = 42.0 * scale;
    hud.segments(
        value_size,
        (inner_x + side_w - 12.0 * scale, value_y),
        Align::Right,
        ink,
        &format!("{lap}/{}", race.laps),
    );
    // Gear sits in the same recessed aperture as the other LCD readouts.
    let refusal = car.powertrain.last_refusal.map(|refusal| match refusal {
        ShiftRefusal::OverRev => "OVER-REV",
        ShiftRefusal::TooFast => "TOO FAST",
    });
    let bezel = if refusal.is_some() { AMBER } else { WELL_RIM };
    hud.fill_round(
        gear_x,
        main_y - 9. * scale,
        gear_w,
        main_h + 18. * scale,
        5.0 * scale,
        WELL,
    );
    hud.stroke_round(
        gear_x,
        main_y - 9. * scale,
        gear_w,
        main_h + 18. * scale,
        5.0 * scale,
        1.0 * scale,
        bezel,
    );
    let gear = match car.powertrain.gear {
        -1 => "R".to_string(),
        0 => "N".to_string(),
        gear => gear.to_string(),
    };
    let gear_size = 62.0 * scale;
    let gear_y = main_y + 8.0 * scale;
    hud.segments(
        gear_size,
        (gear_x + gear_w * 0.5, gear_y),
        Align::Center,
        ink,
        &gear,
    );
    // Speed, with engine speed as its caption.
    well(hud, speed_x, main_y, side_w, main_h, scale);
    caption(
        hud,
        style,
        14.0 * scale,
        (speed_x + 10.0 * scale, main_y + 5.0 * scale),
        Align::Left,
        "KM/H",
    );
    caption(
        hud,
        style,
        14.0 * scale,
        (speed_x + side_w - 10.0 * scale, main_y + 5.0 * scale),
        Align::Right,
        "RPM",
    );
    hud.segments(
        11. * scale,
        (speed_x + side_w - 42. * scale, main_y + 6. * scale),
        Align::Right,
        DIM,
        &format!(
            "{:.0}",
            (session.telemetry.rpm.value(car.powertrain.rpm) / 50.).round() * 50.
        ),
    );
    let kmh = (session.telemetry.speed.value(car.speed() * 3.6)).round() as i32;
    hud.segments(
        value_size,
        (speed_x + side_w - 12.0 * scale, value_y),
        Align::Right,
        ink,
        &kmh.to_string(),
    );

    // Lower row: lap time on the left and the live sector clock on the right.
    let lower_y = main_y + main_h + 14. * scale;
    let time_w = speed_x - gap - inner_x;
    well(hud, inner_x, lower_y, time_w, lower_h, scale);
    caption(
        hud,
        style,
        14.0 * scale,
        (inner_x + 10.0 * scale, lower_y + 14.0 * scale),
        Align::Left,
        "LAP TIME",
    );
    let lower_size = 25.0 * scale;
    let lower_value_y = lower_y + 11.0 * scale;
    hud.segments(
        lower_size,
        (inner_x + time_w - 12.0 * scale, lower_value_y),
        Align::Right,
        ink,
        &lap_clock(race, session.player),
    );
    // Sector timing lives in the timing strip; this cell is the DRS lamp.
    well(hud, speed_x, lower_y, side_w, lower_h, scale);
    drs_lamp(
        hud,
        style,
        rect(
            speed_x + 6. * scale,
            lower_y + 6. * scale,
            side_w - 12. * scale,
            lower_h - 12. * scale,
        ),
        22. * scale,
        DrsLamp::of(session),
    );
    if let Some(text) = refusal {
        let tag_w = hud.text_width(&style.bold, 20.0 * scale, text) + 24.0 * scale;
        let tag_x = gear_x + gear_w * 0.5 - tag_w * 0.5;
        hud.fill_round(
            tag_x,
            py - 38.0 * scale,
            tag_w,
            30.0 * scale,
            6.0 * scale,
            WELL,
        );
        hud.stroke_round(
            tag_x,
            py - 38.0 * scale,
            tag_w,
            30.0 * scale,
            6.0 * scale,
            2.0 * scale,
            AMBER,
        );
        hud.text(
            &style.bold,
            20.0 * scale,
            (gear_x + gear_w * 0.5, py - 36.0 * scale),
            Align::Center,
            AMBER,
            text,
        );
    }

    let pedal_y = lower_y + lower_h + 8. * scale;
    let pedal_w = (inner_w - 18. * scale) * 0.5;
    pedal(
        hud,
        style,
        egui::Rect::from_min_size(
            egui::pos2(inner_x, pedal_y),
            egui::vec2(pedal_w, 28. * scale),
        ),
        "BRAKE",
        session
            .telemetry
            .brake
            .value(session.guidance.activity.input.brake) as f32,
        ACCENT,
    );
    pedal(
        hud,
        style,
        egui::Rect::from_min_size(
            egui::pos2(inner_x + pedal_w + 18. * scale, pedal_y),
            egui::vec2(pedal_w, 28. * scale),
        ),
        "THROTTLE",
        session
            .telemetry
            .throttle
            .value(session.guidance.activity.input.throttle) as f32,
        GREEN,
    );
}

fn polar(center: (f32, f32), radius: f32, degrees: f32) -> (f32, f32) {
    let angle = degrees.to_radians();
    (
        center.0 + radius * angle.cos(),
        center.1 + radius * angle.sin(),
    )
}

fn arc(
    hud: &Hud,
    center: (f32, f32),
    radius: f32,
    angles: (f32, f32),
    thickness: f32,
    color: Color,
) {
    let steps = ((angles.1 - angles.0).abs() / 4.0).ceil().max(1.0) as usize;
    let points = (0..=steps)
        .map(|i| {
            let p = polar(
                center,
                radius,
                angles.0 + (angles.1 - angles.0) * i as f32 / steps as f32,
            );
            egui::pos2(p.0, p.1)
        })
        .collect();
    hud.painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(thickness, color),
    ));
}

/// One input bar, shared by both layouts so pedals never appear twice. The
/// bar is the reading; a percentage beside it would only repeat it.
fn pedal(hud: &Hud, style: &HudStyle, rect: egui::Rect, label: &str, value: f32, color: Color) {
    let scale = rect.height() / 30.;
    hud.text_mid(
        &style.bold,
        12. * scale,
        (rect.left(), rect.top() + 8. * scale),
        Align::Left,
        hud.finish.colors().muted,
        label,
    );
    hud.fill_round(
        rect.left(),
        rect.bottom() - 6. * scale,
        rect.width(),
        5. * scale,
        scale,
        Color::from_rgb(3, 9, 11),
    );
    hud.fill_round(
        rect.left(),
        rect.bottom() - 6. * scale,
        rect.width() * value,
        5. * scale,
        scale,
        color,
    );
}

/// Offset shadow, turned metal rim, gasket, then a slightly dished face.
fn bezel(hud: &Hud, c: (f32, f32), r: f32, scale: f32) {
    hud.circle(
        (c.0 + 2. * scale, c.1 + 5. * scale),
        r + 2. * scale,
        Color::from_black_alpha(90),
    );
    hud.disc(c, r, Color::from_gray(134), Color::from_gray(28));
    hud.disc(
        c,
        r - 2. * scale,
        Color::from_gray(42),
        Color::from_gray(104),
    );
    hud.circle(c, r - 4. * scale, Color::from_rgb(5, 6, 7));
    hud.disc(
        c,
        r - 7. * scale,
        Color::from_rgb(31, 33, 34),
        Color::from_rgb(13, 15, 16),
    );
}

/// Machined bezel and matte face; every graduation maps to a simulation value.
struct Dial {
    center: (f32, f32),
    radius: f32,
    minimum: f32,
    maximum: f32,
    divisions: usize,
    major_every: usize,
    redline: Option<f32>,
}

impl Dial {
    fn point(&self, fraction: f32, radius: f32) -> (f32, f32) {
        polar(self.center, self.radius * radius, 135. + fraction * 270.)
    }

    fn fraction(&self, value: f32) -> f32 {
        ((value - self.minimum) / (self.maximum - self.minimum)).clamp(0., 1.)
    }

    /// Coloured scale band just inside the graduations.
    fn band(&self, hud: &Hud, from: f32, to: f32, color: Color) {
        let (from, to) = (self.fraction(from), self.fraction(to));
        if to > from {
            arc(
                hud,
                self.center,
                self.radius * 0.86,
                (135. + from * 270., 135. + to * 270.),
                self.radius * 0.06,
                color,
            );
        }
    }

    fn face(&self, hud: &Hud, style: &HudStyle, scale: f32) {
        let c = self.center;
        let r = self.radius;
        bezel(hud, c, r, scale);
        let ink = hud.finish.colors().ink;
        if let Some(redline) = self.redline {
            let span = self.maximum - self.minimum;
            self.band(hud, self.minimum + redline * span, self.maximum, ACCENT);
        }
        // Small meters keep only their graduations; numerals there would be
        // too small to read at a glance.
        let numeral = r * 0.14;
        let numbered = numeral >= 9. * scale;
        for i in 0..=self.divisions {
            let fraction = i as f32 / self.divisions as f32;
            let major = i % self.major_every == 0;
            let color = if self.redline.is_some_and(|redline| fraction >= redline) {
                ACCENT
            } else {
                ink
            };
            hud.line(
                self.point(fraction, if major { 0.75 } else { 0.82 }),
                self.point(fraction, 0.89),
                if major { 1.8 } else { 0.8 } * scale,
                color,
            );
            if major && numbered {
                hud.text_mid(
                    &style.regular,
                    numeral,
                    self.point(fraction, 0.62),
                    Align::Center,
                    color,
                    &format!(
                        "{:.0}",
                        self.minimum + fraction * (self.maximum - self.minimum)
                    ),
                );
            }
        }
    }

    fn needle(&self, hud: &Hud, value: f32, color: Color, scale: f32) {
        self.pointer(hud, value, 0.8, color, scale);
    }

    fn pointer(&self, hud: &Hud, value: f32, length: f32, color: Color, scale: f32) {
        let theta = 135. + self.fraction(value) * 270.;
        let points = [
            polar(self.center, self.radius * 0.17, theta + 180.),
            polar(self.center, 2. * scale, theta + 90.),
            polar(self.center, self.radius * length, theta),
            polar(self.center, 2. * scale, theta - 90.),
        ];
        hud.painter.add(egui::Shape::convex_polygon(
            points.into_iter().map(|p| egui::pos2(p.0, p.1)).collect(),
            color,
            egui::Stroke::NONE,
        ));
        hud.disc(
            self.center,
            6. * scale,
            Color::from_gray(111),
            Color::from_gray(21),
        );
        hud.circle(self.center, 2. * scale, Color::from_gray(12));
    }
}

/// One rev counter carries the whole drive: speed and gear on its face,
/// pedal travel in the arc gap below the sweep, shift light in the gear bezel.
fn gauges(hud: &Hud, style: &HudStyle, session: &Session, scale: f32) {
    let race = &session.race;
    let car = &race.entrants[session.player].car;
    let colors = hud.finish.colors();
    let r = 165. * scale;
    let c = (
        hud.width() - 28. * scale - r,
        hud.height() - 24. * scale - r,
    );
    let at = |x: f32, y: f32| (c.0 + x * r, c.1 + y * r);
    let engine = &car.spec.engine;
    let maximum = (engine.rev_limit / 1000.).ceil() as f32;
    let tach = Dial {
        center: c,
        radius: r,
        minimum: 0.,
        maximum,
        divisions: (maximum * 2.) as usize,
        major_every: 4,
        redline: Some(0.92),
    };
    tach.face(hud, style, scale);
    hud.text_mid(
        &style.bold,
        11. * scale,
        at(0., -0.44),
        Align::Center,
        colors.muted,
        "RPM × 1000",
    );
    hud.text_mid(
        &style.bold,
        0.2 * r,
        at(0., -0.25),
        Align::Center,
        colors.ink,
        &format!("{:.0}", session.telemetry.speed.value(car.speed() * 3.6)),
    );
    hud.text_mid(
        &style.bold,
        10. * scale,
        at(0., -0.11),
        Align::Center,
        colors.muted,
        "KM/H",
    );

    // Pedal travel fills inward from both ends of the gap under the sweep.
    for (start, direction, value, color, label) in [
        (
            130.,
            -1.,
            session
                .telemetry
                .brake
                .value(session.guidance.activity.input.brake),
            ACCENT,
            "BRK",
        ),
        (
            50.,
            1.,
            session
                .telemetry
                .throttle
                .value(session.guidance.activity.input.throttle),
            GREEN,
            "THR",
        ),
    ] {
        let span = 34.;
        arc(
            hud,
            c,
            r * 0.86,
            (start, start + direction * span),
            r * 0.05,
            Color::from_rgb(8, 9, 10),
        );
        if value > 0.005 {
            arc(
                hud,
                c,
                r * 0.86,
                (start, start + direction * span * value as f32),
                r * 0.05,
                color,
            );
        }
        hud.text_mid(
            &style.bold,
            9. * scale,
            polar(c, r * 0.74, start + direction * span * 0.5),
            Align::Center,
            colors.muted,
            label,
        );
    }

    // Gear window doubles as the shift light.
    let rpm_fraction = ((car.powertrain.rpm - engine.idle_rpm)
        / (engine.rev_limit - engine.idle_rpm))
        .clamp(0.0, 1.0) as f32;
    let flash = car.powertrain.limiter && (race.time * 12.0).sin() > 0.0;
    let shift = rpm_fraction >= 0.92 && !car.powertrain.limiter || flash;
    let (gw, gh) = (0.3 * r, 0.3 * r);
    let (gx, gy) = (c.0 - gw * 0.5, c.1 + 0.24 * r);
    if shift {
        hud.fill_round(
            gx - 4. * scale,
            gy - 4. * scale,
            gw + 8. * scale,
            gh + 8. * scale,
            7. * scale,
            BLUE.gamma_multiply(0.35),
        );
    }
    well(hud, gx, gy, gw, gh, scale);
    hud.stroke_round(
        gx,
        gy,
        gw,
        gh,
        3. * scale,
        if shift { 2. } else { 1. } * scale,
        if shift { BLUE } else { WELL_RIM },
    );
    let gear = match car.powertrain.gear {
        -1 => "R".to_string(),
        0 => "N".to_string(),
        n => n.to_string(),
    };
    hud.text_mid(
        &style.bold,
        0.26 * r,
        (c.0, gy + gh * 0.5),
        Align::Center,
        colors.ink,
        &gear,
    );
    // DRS sits in the free arc gap beneath the gear window.
    let (lw, lh) = (0.3 * r, 0.11 * r);
    drs_lamp(
        hud,
        style,
        rect(c.0 - lw * 0.5, c.1 + 0.59 * r, lw, lh),
        0.075 * r,
        DrsLamp::of(session),
    );
    tach.needle(
        hud,
        session.telemetry.rpm.value(car.powertrain.rpm) as f32 / 1000.,
        ACCENT,
        scale,
    );

    if let Some(refusal) = car.powertrain.last_refusal {
        let text = match refusal {
            ShiftRefusal::OverRev => "OVER-REV",
            ShiftRefusal::TooFast => "TOO FAST",
        };
        hud.text_mid(
            &style.bold,
            16. * scale,
            (c.0, c.1 - r - 16. * scale),
            Align::Center,
            AMBER,
            text,
        );
    }
}

pub(super) fn tyres(hud: &Hud, style: &HudStyle, session: &Session, layout: Layout, scale: f32) {
    match layout {
        Layout::Segments => tyre_display(hud, style, session, scale),
        Layout::Gauges => tyre_dials(hud, style, session, scale),
    }
}

/// LCD tyre module matching the segmented dash: per-axle temperature and wear
/// windows, then tow and wake as LED bars.
fn tyre_display(hud: &Hud, style: &HudStyle, session: &Session, scale: f32) {
    let car = &session.race.entrants[session.player].car;
    let compound = car.setup.compound;
    let pad = 14. * scale;
    let gap = 10. * scale;
    let tyre_h = 72. * scale;
    let aero_h = 38. * scale;
    let header_h = 22. * scale;
    let panel_w = 330. * scale;
    let panel_h = pad + header_h + tyre_h + gap + aero_h + pad;
    let px = 24. * scale;
    let py = hud.height() - panel_h - 24. * scale;
    housing(hud, px, py, panel_w, panel_h, scale);
    caption(
        hud,
        style,
        13. * scale,
        (px + pad, py + pad - 2. * scale),
        Align::Left,
        &format!("{} TYRES", compound.label()),
    );
    caption(
        hud,
        style,
        13. * scale,
        (px + panel_w - pad, py + pad - 2. * scale),
        Align::Right,
        "°C / WEAR",
    );
    let inner_w = panel_w - pad * 2.;
    let cell_w = (inner_w - gap) * 0.5;
    let tyre_y = py + pad + header_h;
    for (axle, label) in [(0, "FRONT"), (1, "REAR")] {
        let tyre = car.tyres[axle];
        let band = TyreBand::of(tyre.temperature, compound);
        let color = band.color();
        let x = px + pad + axle as f32 * (cell_w + gap);
        well(hud, x, tyre_y, cell_w, tyre_h, scale);
        caption(
            hud,
            style,
            13. * scale,
            (x + 10. * scale, tyre_y + 5. * scale),
            Align::Left,
            label,
        );
        // Two tyre footprints retain the per-wheel lock and slide indication.
        for side in 0..2 {
            let wheel = &car.wheels[axle * 2 + side];
            let fx = x + cell_w - (32. - side as f32 * 13.) * scale;
            hud.fill_round(
                fx,
                tyre_y + 7. * scale,
                9. * scale,
                15. * scale,
                2. * scale,
                color,
            );
            if wheel.locked || wheel.sliding > 0.3 {
                hud.stroke_round(
                    fx,
                    tyre_y + 7. * scale,
                    9. * scale,
                    15. * scale,
                    2. * scale,
                    2. * scale,
                    WHITE,
                );
            }
        }
        hud.segments(
            34. * scale,
            (x + cell_w - 12. * scale, tyre_y + 28. * scale),
            Align::Right,
            color,
            &format!(
                "{:.0}°C",
                session.telemetry.temperatures[axle].value(tyre.temperature)
            ),
        );
        readout(
            hud,
            style,
            11. * scale,
            (x + cell_w - 42. * scale, tyre_y + 14. * scale),
            Align::Right,
            hud.finish.colors().ink,
            &format!(
                "{:.0}%",
                session.telemetry.wear[axle].value(tyre.wear) * 100.
            ),
        );
    }
    let aero_y = tyre_y + tyre_h + gap;
    for (slot, label, value, color) in [
        (
            0,
            "TOW",
            session.telemetry.tow.value(car.aero.slipstream()),
            BLUE,
        ),
        (
            1,
            "WAKE",
            session.telemetry.wake.value(car.aero.dirty_air()),
            AMBER,
        ),
    ] {
        let x = px + pad + slot as f32 * (cell_w + gap);
        well(hud, x, aero_y, cell_w, aero_h, scale);
        caption(
            hud,
            style,
            13. * scale,
            (x + 10. * scale, aero_y + 10. * scale),
            Align::Left,
            label,
        );
        let bars = 10;
        let bar_x = x + 56. * scale;
        let bar_w = x + cell_w - 10. * scale - bar_x;
        let pitch = bar_w / bars as f32;
        for bar in 0..bars {
            let lit = value as f32 >= (bar as f32 + 0.5) / bars as f32;
            hud.fill_rect(
                bar_x + bar as f32 * pitch,
                aero_y + 11. * scale,
                pitch - 2. * scale,
                16. * scale,
                if lit {
                    color
                } else {
                    Color::from_rgb(28, 29, 28)
                },
            );
        }
    }
}

/// Tyre gauge: the temperature scale sweeps the top of the face, with the
/// front pointer outside its band and the rear pointer inside. Beneath it a
/// top-down tyre map colours each tyre by its thermal band and carries each
/// axle's temperature and wear between its pair; tow and wake sit below.
fn tyre_dials(hud: &Hud, style: &HudStyle, session: &Session, scale: f32) {
    const MIN: f32 = 40.;
    const MAX: f32 = 160.;
    // Screen angles (y down): 200° is left of centre, 340° right, over the top.
    const FROM: f32 = 200.;
    const SWEEP: f32 = 140.;
    let car = &session.race.entrants[session.player].car;
    let colors = hud.finish.colors();
    let compound = car.setup.compound;
    let props = compound.properties();
    let (optimal, window) = (props.optimal_temperature as f32, props.window as f32);
    let r = 104. * scale;
    let c = (24. * scale + r, hud.height() - 24. * scale - r);
    let at = |x: f32, y: f32| (c.0 + x * r, c.1 + y * r);
    let angle = |value: f32| FROM + ((value - MIN) / (MAX - MIN)).clamp(0., 1.) * SWEEP;
    let ink = colors.ink;
    bezel(hud, c, r, scale);

    for (low, high, color) in [
        (MIN, optimal - window, BLUE),
        (optimal - window, optimal + window, GREEN),
        (optimal + window, optimal + 2. * window, TYRE_YELLOW),
        (optimal + 2. * window, MAX, ACCENT),
    ] {
        let (low, high) = (low.max(MIN), high.min(MAX));
        if high > low {
            arc(
                hud,
                c,
                r * 0.8,
                (angle(low), angle(high)),
                r * 0.06,
                color.gamma_multiply(0.85),
            );
        }
    }
    for step in 0..=12 {
        let value = MIN + step as f32 * 10.;
        let major = step % 3 == 0;
        hud.line(
            polar(c, r * if major { 0.64 } else { 0.68 }, angle(value)),
            polar(c, r * 0.73, angle(value)),
            if major { 1.6 } else { 0.8 } * scale,
            Color::from_gray(150),
        );
    }
    for (value, x) in [(MIN, -0.72), (MAX, 0.72)] {
        hud.text_mid(
            &style.regular,
            0.09 * r,
            at(x, -0.12),
            Align::Center,
            colors.muted,
            &format!("{value:.0}"),
        );
    }
    hud.text_mid(
        &style.bold,
        0.09 * r,
        at(0., -0.5),
        Align::Center,
        colors.muted,
        &format!("{} °C", compound.label()),
    );

    // Tyre map: fronts narrower than rears, each pair around its readout.
    let tyre_w = [0.16 * r, 0.18 * r];
    let tyre_h = 0.3 * r;
    let axle_y = [-0.08, 0.36];
    let half_track = [0.35, 0.36];
    for axle in 0..2 {
        let tyre = car.tyres[axle];
        let temperature = session.telemetry.temperatures[axle].value(tyre.temperature);
        let band = TyreBand::of(temperature, compound).color();
        for side in 0..2 {
            let wheel = &car.wheels[axle * 2 + side];
            let sign = if side == 0 { -1. } else { 1. };
            let (x, y) = at(sign * half_track[axle], axle_y[axle]);
            let (left, top) = (x - tyre_w[axle] * 0.5, y - tyre_h * 0.5);
            hud.fill_round(left, top, tyre_w[axle], tyre_h, 3. * scale, band);
            if wheel.locked || wheel.sliding > 0.3 {
                hud.stroke_round(
                    left - 1.5 * scale,
                    top - 1.5 * scale,
                    tyre_w[axle] + 3. * scale,
                    tyre_h + 3. * scale,
                    4. * scale,
                    2. * scale,
                    WHITE,
                );
            }
        }
        // Pointer colour keys the axle: white front, amber rear.
        let key = if axle == 0 { ink } else { AMBER };
        hud.fill_rect(
            c.0 - 0.14 * r,
            c.1 + (axle_y[axle] - 0.16) * r,
            0.28 * r,
            1.5 * scale,
            key,
        );
        hud.text_mid(
            &style.bold,
            0.2 * r,
            at(0., axle_y[axle] - 0.02),
            Align::Center,
            band,
            &format!("{temperature:.0}"),
        );
        hud.text_mid(
            &style.regular,
            0.09 * r,
            at(0., axle_y[axle] + 0.12),
            Align::Center,
            colors.muted,
            &format!(
                "{:.0}% WEAR",
                session.telemetry.wear[axle].value(tyre.wear) * 100.
            ),
        );
    }

    let (w, h) = (0.3 * r, 0.12 * r);
    for (x, label, value, color) in [
        (
            -0.17,
            "TOW",
            session.telemetry.tow.value(car.aero.slipstream()),
            BLUE,
        ),
        (
            0.17,
            "WAKE",
            session.telemetry.wake.value(car.aero.dirty_air()),
            AMBER,
        ),
    ] {
        let value = value.clamp(0., 1.) as f32;
        let (x, y) = at(x, 0.74);
        let lit = value > 0.05;
        let (left, top) = (x - w * 0.5, y - h * 0.5);
        if lit {
            hud.fill_round(
                left,
                top,
                w,
                h,
                h * 0.25,
                color.gamma_multiply(0.3 + 0.7 * value),
            );
        } else {
            hud.stroke_round(left, top, w, h, h * 0.25, scale, Color::from_gray(60));
        }
        hud.text_mid(
            &style.bold,
            0.085 * r,
            (x, y),
            Align::Center,
            if lit {
                Color::from_rgb(8, 9, 10)
            } else {
                Color::from_gray(115)
            },
            label,
        );
    }

    // Front pointer outside the band pointing in, rear inside pointing out;
    // rear first so the front reads on top.
    for axle in [1, 0] {
        let value = session.telemetry.temperatures[axle].value(car.tyres[axle].temperature) as f32;
        let theta = angle(value);
        let (base, tip) = if axle == 0 {
            (0.93, 0.83)
        } else {
            (0.66, 0.77)
        };
        let points = [
            polar(c, r * base, theta - 5.),
            polar(c, r * tip, theta),
            polar(c, r * base, theta + 5.),
        ];
        hud.painter.add(egui::Shape::convex_polygon(
            points.into_iter().map(|p| egui::pos2(p.0, p.1)).collect(),
            if axle == 0 { ink } else { AMBER },
            egui::Stroke::new(scale, Color::from_rgb(8, 9, 10)),
        ));
    }
}

#[cfg(test)]
mod drs_tests {
    use super::*;
    use racecraft_sim::drs::Zone;

    #[test]
    fn drs_lamp_tracks_the_players_flap() {
        let mut session = crate::tests::session();
        let player = session.player;
        // A circuit without zones shows no badge at all.
        assert_eq!(DrsLamp::of(&session), None);
        session.race.drs_zones = vec![Zone {
            detection: 0.,
            start: 10.,
            end: 100.,
        }];
        assert_eq!(DrsLamp::of(&session), Some(DrsLamp::Off));
        session.race.entrants[player].car.drs.available = true;
        assert_eq!(DrsLamp::of(&session), Some(DrsLamp::Ready));
        session.race.entrants[player].car.drs.flap = 0.4;
        assert_eq!(DrsLamp::of(&session), Some(DrsLamp::Open));
    }
}
