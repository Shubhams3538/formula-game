//! First-corner traffic probes against generated track geometry.
use racecraft_sim::{
    Assists, CarGeometry, CarSpec, DriverProfile, EntrantConfig, Race, RaceEvent, Setup, Track,
    PHYSICS_RATE,
};
use std::{path::PathBuf, sync::Arc};

#[test]
fn field_clears_opening_corners() {
    let explicit = std::env::var_os("F1_TEST_ASSETS");
    let root = explicit
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets"));
    if !root.join("cars/gen22/car.json").is_file() {
        assert!(explicit.is_none(), "F1_TEST_ASSETS missing Gen22 geometry");
        eprintln!("skipping opening-corner probe: generated Gen22 geometry absent");
        return;
    }
    let spec = Arc::new(CarSpec::gen22_with(
        &CarGeometry::load(root.join("cars/gen22/car.json")).unwrap(),
    ));
    for name in ["spa", "bahrain", "silverstone", "cota"] {
        if !root.join(format!("tracks/{name}/track.json")).is_file() {
            assert!(explicit.is_none(), "F1_TEST_ASSETS missing {name} track");
            eprintln!("skipping {name}: generated track absent");
            continue;
        }
        let track = Arc::new(Track::load(root.join(format!("tracks/{name}"))).unwrap());
        for seed in [5, 17, 42] {
            let entrants = (0..8)
                .map(|index| EntrantConfig {
                    name: format!("AI {index}"),
                    spec: spec.clone(),
                    setup: Setup::default(),
                    assists: Assists::default(),
                    ai: Some(DriverProfile {
                        pace: 0.95 + index as f64 * 0.004,
                        aggression: 0.3 + index as f64 * 0.08,
                        consistency: 0.9,
                    }),
                    human_slot: 0,
                    livery: [1.; 3],
                })
                .collect();
            let mut race = Race::new(track.clone(), entrants, 1, seed);
            for _ in 0..(45. * PHYSICS_RATE) as usize {
                race.step(&[]);
            }
            let impulses: Vec<_> = race
                .events
                .iter()
                .filter_map(|event| match event {
                    RaceEvent::Contact {
                        b: Some(_),
                        impulse,
                        ..
                    } => Some(*impulse),
                    _ => None,
                })
                .collect();
            let severe = impulses.iter().filter(|&&impulse| impulse > 6000.).count();
            let worst = impulses.iter().copied().fold(0., f64::max);
            let progress = race
                .entrants
                .iter()
                .map(|e| e.timing.distance)
                .fold(f64::INFINITY, f64::min);
            println!("{name} seed={seed}: contacts={}, severe={severe}, worst={worst:.0}, minimum progress={progress:.0}", impulses.len());
            // Light racing contact is possible; repeated hard hits and a field
            // stranded at the opening corners are not an acceptable start.
            assert!(
                impulses.len() <= 5,
                "{name}/{seed}: repeated contact: {impulses:?}"
            );
            assert!(
                severe <= 1 && worst < 9000.,
                "{name}/{seed}: opening pileup: {impulses:?}"
            );
            assert!(
                progress > 1200.,
                "{name}/{seed}: field stranded at {progress:.0}m"
            );
        }
    }
}
