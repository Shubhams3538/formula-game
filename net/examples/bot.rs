//! Join an existing graphical/dedicated room as a scripted driver for diagnostics.
use racecraft_net::*;
use racecraft_sim::{AiDriver, Assists, CarInput, DriverProfile, PHYSICS_RATE};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let address = args
        .next()
        .unwrap_or_else(|| "127.0.0.1:7777".into())
        .parse()
        .map_err(|_| "expected IP:port or ws(s)://server")?;
    let root = PathBuf::from(args.next().unwrap_or_else(|| "assets".into()));
    let name = args.next().unwrap_or_else(|| "Scripted driver".into());
    let mut client = Client::connect(&root, address, name, Assists::default(), None)?;
    let mut race = None;
    let mut driver = None;
    let mut readied = false;
    let mut previous = Instant::now();
    let mut debt = 0.;
    let mut until = None;
    let deadline = Instant::now() + Duration::from_secs(40);
    while Instant::now() < deadline {
        let now = Instant::now();
        let dt = now.duration_since(previous);
        previous = now;
        debt = (debt + dt.as_secs_f64()).min(0.1);
        client.poll(dt)?;
        if !readied && client.slot().is_some() {
            println!(
                "Admitted to room: {} drivers, settings {:?}",
                client.lobby.as_ref().unwrap().players.len(),
                client.lobby.as_ref().unwrap().settings
            );
            client.send(ClientMessage::Ready(true))?;
            readied = true;
            until = Some(Instant::now() + Duration::from_secs(8));
        }
        if let Some(lobby) = client.load.take() {
            let loaded = load_race(&root, &lobby.settings, &lobby.players)?;
            let slot = client.slot().unwrap();
            driver = Some(AiDriver::new(
                DriverProfile {
                    pace: 0.92,
                    aggression: 0.5,
                    consistency: 1.,
                },
                &loaded.entrants[slot].car,
                &loaded.track,
                123,
            ));
            race = Some(loaded);
            client.loaded()?;
            until = Some(Instant::now() + Duration::from_secs(15));
        }
        if let Some(race) = &mut race {
            client.reconcile(race)?;
            while debt >= 1. / PHYSICS_RATE {
                debt -= 1. / PHYSICS_RATE;
                let slot = client.slot().unwrap();
                let input = driver.as_mut().map_or(CarInput::default(), |d| {
                    d.drive(
                        &race.entrants[slot].car,
                        &race.track,
                        &[],
                        1. / PHYSICS_RATE,
                    )
                });
                client.predict(race, input)?;
            }
        } else {
            debt = 0.;
        }
        client.flush()?;
        if until.is_some_and(|t| Instant::now() > t) {
            println!(
                "Finished probe, authoritative_tick={:?}",
                client.authoritative_tick
            );
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err("probe timed out before joining".into())
}
