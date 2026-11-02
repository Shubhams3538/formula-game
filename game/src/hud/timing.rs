//! Timing tower additions use the incumbent instrument palette and fonts.

use super::*;

/// Fixed columns keep timing readable as digits and sector states change.
pub(super) fn strip(
    hud: &Hud,
    style: &HudStyle,
    session: &Session,
    x: f32,
    y: f32,
    w: f32,
    s: f32,
) {
    let colors = hud.finish.colors();
    let label_size = (13. * s).max(11.);
    let race = &session.race;
    let timing = &race.entrants[session.player].timing;
    let running =
        matches!(race.phase, Phase::Racing | Phase::Finishing) && timing.finish_time.is_none();
    hud.fill_rect(x, y, w, 248. * s, WELL);
    let label_x = x + 12. * s;
    let value_x = x + w - 12. * s;
    hud.text_mid(
        &style.bold,
        label_size,
        (label_x, y + 15. * s),
        Align::Left,
        colors.muted,
        "LAP TIME",
    );
    let clock = lap_clock(race, session.player);
    readout(
        hud,
        style,
        28. * s,
        (value_x, y + 43. * s),
        Align::Right,
        colors.ink,
        &clock,
    );
    for (row, label, value, color) in [
        (0, "LAST", timing.last_lap, colors.ink),
        (1, "BEST", timing.best_lap, PURPLE),
    ] {
        let mid = y + (76. + row as f32 * 24.) * s;
        hud.text_mid(
            &style.bold,
            label_size,
            (label_x, mid),
            Align::Left,
            colors.muted,
            label,
        );
        readout(
            hud,
            style,
            16. * s,
            (value_x, mid),
            Align::Right,
            color,
            &value.map(format_time).unwrap_or_else(|| "-:--.---".into()),
        );
    }
    hud.line(
        (label_x, y + 118. * s),
        (value_x, y + 118. * s),
        s,
        colors.edge,
    );
    hud.text_mid(
        &style.bold,
        label_size,
        (label_x, y + 138. * s),
        Align::Left,
        colors.muted,
        "VS BEST",
    );
    let (delta, color) = session.delta.current.map_or_else(
        || ("SET A LAP".into(), colors.muted),
        |delta| {
            (
                format!("{delta:+.2}"),
                if delta <= 0. { GREEN } else { ACCENT },
            )
        },
    );
    readout(
        hud,
        style,
        20. * s,
        (value_x, y + 138. * s),
        Align::Right,
        color,
        &delta,
    );
    for sector in 0..3 {
        let top = y + (160. + sector as f32 * 28.) * s;
        let active = running && timing.distance >= 0. && timing.sector == sector;
        let split = session.telemetry.splits[sector];
        let fresh = split.is_some_and(|split| split.lap == timing.laps_completed);
        let color = if active {
            BLUE
        } else if fresh {
            colors.ink
        } else {
            colors.muted
        };
        if active {
            hud.fill_rect(
                x + 5. * s,
                top,
                w - 10. * s,
                26. * s,
                Color::from_rgb(16, 30, 45),
            );
        }
        let mid = top + 13. * s;
        hud.text_mid(
            &style.bold,
            (14. * s).max(11.),
            (label_x, mid),
            Align::Left,
            color,
            &format!("S{}", sector + 1),
        );
        let value = if active {
            format!("{:.1}", (race.time - timing.sector_start).max(0.))
        } else {
            split.map_or_else(|| "--.---".into(), |split| format!("{:.3}", split.time))
        };
        readout(
            hud,
            style,
            (17. * s).max(12.),
            (x + 156. * s, mid),
            Align::Right,
            color,
            &value,
        );
        let (status, color) = if active {
            ("LIVE".into(), BLUE)
        } else if let Some(split) = split {
            if !fresh && timing.finish_time.is_none() {
                ("LAST".into(), colors.muted)
            } else {
                split.delta.map_or_else(
                    || ("FIRST".into(), colors.muted),
                    |delta| {
                        (
                            format!("{delta:+.2}"),
                            if delta <= 0. { PURPLE } else { AMBER },
                        )
                    },
                )
            }
        } else {
            ("--".into(), colors.muted)
        };
        hud.text_mid(
            &style.regular,
            (14. * s).max(11.),
            (value_x, mid),
            Align::Right,
            color,
            &status,
        );
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
    hud.fill_rect(x, y, w, 66. * s, WELL);
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
