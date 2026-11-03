//! Timing tower additions use the incumbent instrument palette and fonts.

use super::telemetry::Split;
use super::*;

/// Digital readouts and analog chronographs share the main cluster's materials.
fn panel(hud: &Hud, x: f32, y: f32, w: f32, h: f32) {
    let fill = match hud.finish {
        Finish::Digital => Color::from_rgb(6, 10, 11),
        Finish::Analog => Color::from_black_alpha(210),
    };
    hud.fill_rect(x, y, w, h, fill);
}

/// Broadcast timing rows share the standings tower's flat, tight row style.
/// Returns the strip height so following panels can stack beneath it.
pub(super) fn strip(
    hud: &Hud,
    style: &HudStyle,
    session: &Session,
    x: f32,
    y: f32,
    w: f32,
    s: f32,
) -> f32 {
    let colors = hud.finish.colors();
    let race = &session.race;
    let timing = &race.entrants[session.player].timing;
    let running = matches!(race.phase, Phase::Racing | Phase::Finishing)
        && timing.finish_time.is_none()
        && timing.distance >= 0.;
    let fill = match hud.finish {
        Finish::Digital => Color::from_rgb(6, 10, 11),
        Finish::Analog => Color::from_black_alpha(185),
    };
    let left = x + 12. * s;
    let right = x + w - 12. * s;

    // Lap clock on the left; the live sector counter sits mid-right and the
    // sector it just finished slides out to the right edge.
    let clock_h = 38. * s;
    hud.fill_rect(x, y, w, clock_h - s, fill);
    let mid = y + (clock_h - s) * 0.5;
    readout(
        hud,
        style,
        24. * s,
        (left, mid),
        Align::Left,
        colors.ink,
        &lap_clock(race, session.player),
    );
    let counter_x = right - 86. * s;
    let since = (race.time - timing.sector_start).max(0.);
    if running {
        readout(
            hud,
            style,
            18. * s,
            (counter_x, mid),
            Align::Right,
            BLUE,
            &format!("{since:.1}"),
        );
    }
    if let Some((split, color)) = last_split(session) {
        let slide = if running {
            ease_out((since / SLIDE_SECONDS).clamp(0., 1.) as f32)
        } else {
            1.
        };
        readout(
            hud,
            style,
            16. * s,
            (counter_x + (right - counter_x) * slide, mid),
            Align::Right,
            color,
            &format!("{:.3}", split.time),
        );
    }

    // One flat line of minisectors across all three sectors.
    let bar_y = y + clock_h;
    let bar_h = 16. * s;
    hud.fill_rect(x, bar_y, w, bar_h - s, fill);
    sector_bar(
        hud,
        (left, bar_y + 4. * s),
        (right - left, bar_h - 9. * s),
        session,
        running,
        s,
    );
    let mut height = clock_h + bar_h;

    if let Some(best) = timing.best_lap {
        let row_h = 28. * s;
        let row_y = y + height;
        hud.fill_rect(x, row_y, w, row_h - s, fill);
        let mid = row_y + (row_h - s) * 0.5;
        hud.text_mid(
            &style.bold,
            (13. * s).max(11.),
            (left, mid),
            Align::Left,
            colors.muted,
            "BEST",
        );
        readout(
            hud,
            style,
            16. * s,
            (left + 48. * s, mid),
            Align::Left,
            PURPLE,
            &format_time(best),
        );
        if let Some(delta) = session.delta.current.filter(|_| running) {
            readout(
                hud,
                style,
                16. * s,
                (right, mid),
                Align::Right,
                if delta <= 0. { GREEN } else { ACCENT },
                &format!("{delta:+.2}"),
            );
        }
        height += row_h;
    }
    height
}

const SLIDE_SECONDS: f64 = 0.35;

fn ease_out(t: f32) -> f32 {
    1. - (1. - t).powi(3)
}

/// The most recently completed sector, coloured by how it compared to best.
fn last_split(session: &Session) -> Option<(Split, Color)> {
    let timing = &session.race.entrants[session.player].timing;
    let sector = (timing.sector + 2) % 3;
    let lap = if timing.sector == 0 {
        timing.laps_completed.checked_sub(1)?
    } else {
        timing.laps_completed
    };
    let split = session.telemetry.splits[sector].filter(|split| split.lap == lap)?;
    Some((split, split_color(Some(split), false, true)))
}

fn split_color(split: Option<Split>, active: bool, fresh: bool) -> Color {
    if active {
        BLUE
    } else if let Some(split) = split.filter(|_| fresh) {
        split
            .delta
            .map_or(GREEN, |delta| if delta <= 0. { PURPLE } else { AMBER })
    } else if split.is_some() {
        DIM
    } else {
        Color::from_gray(65)
    }
}

/// Nine flat cells: three minisectors per sector, with wider breaks between
/// sectors. Finished sectors take their timing colour; the live one fills blue.
fn sector_bar(
    hud: &Hud,
    (x, y): (f32, f32),
    (w, h): (f32, f32),
    session: &Session,
    running: bool,
    s: f32,
) {
    let race = &session.race;
    let timing = &race.entrants[session.player].timing;
    let mini_gap = 2. * s;
    let sector_gap = 6. * s;
    let sector_w = (w - sector_gap * 2.) / 3.;
    let cell_w = (sector_w - mini_gap * 2.) / 3.;
    let sector_length = race.track.length / 3.;
    for sector in 0..3 {
        let active = running && timing.sector == sector;
        let split = session.telemetry.splits[sector];
        let fresh = split.is_some_and(|split| split.lap == timing.laps_completed);
        let progress = if active && sector_length > 0. {
            ((timing.distance.rem_euclid(race.track.length) - sector as f64 * sector_length)
                / sector_length)
                .clamp(0., 1.)
        } else {
            0.
        };
        let filled = (progress * 3.).floor() as usize;
        let done = split_color(split, false, fresh);
        for mini in 0..3 {
            let color = if active {
                if mini < filled {
                    BLUE
                } else if mini == filled && progress > 0. {
                    BLUE.gamma_multiply(0.45)
                } else {
                    Color::from_gray(65)
                }
            } else if fresh {
                done
            } else {
                Color::from_gray(65)
            };
            hud.fill_rect(
                x + sector as f32 * (sector_w + sector_gap) + mini as f32 * (cell_w + mini_gap),
                y,
                cell_w,
                h,
                color,
            );
        }
    }
}

pub(super) fn pass_notice(
    hud: &Hud,
    style: &HudStyle,
    session: &Session,
    x: f32,
    y: f32,
    w: f32,
    s: f32,
) {
    let Some(pass) = session
        .telemetry
        .pass
        .as_ref()
        .filter(|pass| pass.until > session.race.tick && session.race.phase != Phase::Finished)
    else {
        return;
    };
    let color = if pass.gained { GREEN } else { AMBER };
    let colors = hud.finish.colors();
    panel(hud, x, y, w, 66. * s);
    hud.text_mid(
        &style.bold,
        (14. * s).max(11.),
        (x + 12. * s, y + 18. * s),
        Align::Left,
        color,
        if pass.gained {
            "POSITION GAINED"
        } else {
            "POSITION LOST"
        },
    );
    let name = session
        .identities
        .get(pass.rival)
        .and_then(Option::as_ref)
        .map_or_else(
            || session.race.entrants[pass.rival].name.to_uppercase(),
            |id| id.abbr.clone(),
        );
    let max_width = w - 100. * s;
    let size = (17. * s).max(12.);
    let size = size * (max_width / hud.text_width(&style.regular, size, &name).max(1.)).min(1.);
    hud.text_mid(
        &style.regular,
        size,
        (x + 12. * s, y + 44. * s),
        Align::Left,
        colors.ink,
        &name,
    );
    hud.text_mid(
        &style.bold,
        30. * s,
        (x + w - 12. * s, y + 39. * s),
        Align::Right,
        color,
        &format!("P{}", pass.position),
    );
    let remaining = ((pass.until - session.race.tick) as f32 / 900.).min(1.);
    hud.fill_rect(x, y + 64. * s, w * remaining, 2. * s, color);
}
