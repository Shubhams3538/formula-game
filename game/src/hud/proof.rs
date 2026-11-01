//! Native presentation fixtures complement driving flows and tick-based tests.

use super::*;
use crate::graphics::TextureDetail;
use crate::options::Options;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Resource, Default)]
struct Proof {
    frame: usize,
    completed: Arc<AtomicUsize>,
}

#[test]
#[ignore = "requires Vulkan display and F1_TEST_ASSETS; synthetic timing states"]
fn rendered_timing_and_pass_states() {
    let root = std::env::var("F1_TEST_ASSETS").expect("set F1_TEST_ASSETS");
    let size = std::env::var("F1_TEST_SIZE").unwrap_or_else(|_| "1920x1080".into());
    let options = Options::parse(
        [
            "--assets",
            &root,
            "--size",
            &size,
            "--quick-race",
            "--mute",
            "--camera",
            "cockpit",
            "--frames",
            "10000",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();
    let mut app = crate::build_app(options).unwrap();
    // Keep the full field on the 4 GB GPU, using the supported texture setting.
    app.world_mut()
        .resource_mut::<crate::menu::Frontend>()
        .video
        .texture = TextureDetail::Half;
    let proof = Proof::default();
    let completed = proof.completed.clone();
    app.insert_resource(proof)
        .add_systems(Update, fixture.after(crate::camera::update));
    assert!(app.run().is_success());
    assert_eq!(completed.load(Ordering::SeqCst), 4);
}

fn fixture(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut proof: ResMut<Proof>,
    mut frontend: ResMut<crate::menu::Frontend>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) {
    if !session.ready {
        return;
    }
    session.paused = true;
    proof.frame += 1;
    let player = session.player;
    let rival = (player + 1) % session.race.entrants.len();
    if proof.frame == 1 {
        session.race.phase = Phase::Racing;
        session.race.time = 73.4;
        let entrant = &mut session.race.entrants[player];
        entrant.timing.laps_completed = 1;
        entrant.timing.distance = 4000.;
        entrant.timing.lap_start = 0.;
        entrant.timing.sector = 2;
        entrant.timing.sector_start = 65.1;
        entrant.timing.last_lap = Some(108.432);
        entrant.timing.best_lap = Some(108.432);
        entrant.car.velocity = glam::DVec2::Y * 61.1;
        entrant.car.powertrain.rpm = 10820.;
        entrant.car.powertrain.gear = 5;
        session.telemetry.splits = [
            Some(telemetry::Split {
                time: 32.123,
                delta: Some(-0.24),
                lap: 1,
            }),
            Some(telemetry::Split {
                time: 32.977,
                delta: Some(0.18),
                lap: 1,
            }),
            Some(telemetry::Split {
                time: 43.332,
                delta: None,
                lap: 0,
            }),
        ];
        session.delta.current = Some(-0.17);
        session.telemetry.pass = Some(telemetry::PassNotice {
            gained: true,
            rival,
            position: 5,
            until: session.race.tick + 900,
        });
    }
    if proof.frame == 60 {
        frontend.display.layout = Layout::Gauges;
        session.telemetry.pass.as_mut().unwrap().gained = false;
        session.telemetry.pass.as_mut().unwrap().position = 6;
    }
    if proof.frame == 90 {
        frontend.display.layout = Layout::Segments;
        session.telemetry.pass = None;
        session.delta.current = None;
        session.telemetry.splits = [None; 3];
        let now = session.race.time;
        let timing = &mut session.race.entrants[player].timing;
        timing.laps_completed = 0;
        timing.last_lap = None;
        timing.best_lap = None;
        timing.sector = 0;
        timing.sector_start = now;
        session.race.phase = Phase::Countdown {
            lights: 4,
            timer: 0.,
            hold: 1.,
        };
    }
    let name = match proof.frame {
        30 => "timing-gained-digital",
        75 => "timing-lost-analog",
        105 => "timing-first-lap",
        120..=180 if proof.completed.load(Ordering::SeqCst) == 4 => {
            exit.write(bevy::app::AppExit::Success);
            return;
        }
        181 => panic!("timing captures did not complete"),
        _ => return,
    };
    capture(
        &mut commands,
        Screenshot::primary_window(),
        name,
        proof.completed.clone(),
    );
    if proof.frame == 30 {
        // Capture the real offscreen LCD as well as the perspective cockpit.
        // Its render target uses float pixels, so use Bevy's image conversion.
        commands.queue(|world: &mut World| {
            let target = world
                .resource::<crate::wheel_dash::WheelDash>()
                .capture_target();
            let completed = world.resource::<Proof>().completed.clone();
            capture(
                &mut world.commands(),
                Screenshot::image(target),
                "timing-wheel-lcd",
                completed,
            );
        });
    }
}

fn capture(commands: &mut Commands, target: Screenshot, name: &str, completed: Arc<AtomicUsize>) {
    let directory = std::env::var_os("F1_TEST_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("f1-hud-timing-proof"));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{name}.png"));
    commands
        .spawn(target)
        .observe(move |event: On<ScreenshotCaptured>| {
            let mut image = event.image.clone();
            if image.texture_descriptor.format
                == bevy::render::render_resource::TextureFormat::Rgba16Float
            {
                let mut pixels = Vec::with_capacity((image.width() * image.height() * 4) as usize);
                for y in 0..image.height() {
                    for x in 0..image.width() {
                        pixels.extend_from_slice(
                            &image.get_color_at(x, y).unwrap().to_srgba().to_u8_array(),
                        );
                    }
                }
                image.texture_descriptor.format =
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb;
                image.data = Some(pixels);
            }
            image
                .try_into_dynamic()
                .unwrap()
                .to_rgb8()
                .save(&path)
                .unwrap();
            completed.fetch_add(1, Ordering::SeqCst);
            info!("Timing fixture screenshot: {}", path.display());
        });
}
