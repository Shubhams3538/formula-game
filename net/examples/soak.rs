//! Real converted-track, 16-scripted-client UDP soak. No renderer or human driving claim.
use racecraft_net::*;
use racecraft_sim::{AiDriver, Assists, CarInput, DriverProfile, PHYSICS_RATE};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
fn main() -> Result<(), String> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "assets".into()));
    let seconds = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "15".into())
        .parse::<u64>()
        .map_err(|e| e.to_string())?;
    let server = Server::bind(
        &root,
        "127.0.0.1:0".parse().unwrap(),
        RaceSettings::default(),
        None,
    )?;
    let address = server.address();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let worker = std::thread::spawn(move || -> Result<(), String> {
        let mut server = server;
        let mut previous = Instant::now();
        let mut worst = Duration::ZERO;
        let mut racing_at = None;
        while !flag.load(Ordering::Relaxed) && server.lobby.stage != Stage::Closed {
            let now = Instant::now();
            let racing = matches!(server.lobby.stage, Stage::Grid | Stage::Racing);
            server.update(now.duration_since(previous))?;
            previous = now;
            if racing {
                worst = worst.max(now.elapsed());
            }
            if matches!(server.lobby.stage, Stage::Grid | Stage::Racing) {
                racing_at.get_or_insert(now);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        println!("SERVER: tick={} connected={} snapshot={} bytes max_race_update={:?} overloads={} race_wall={:?}",server.race.as_ref().map_or(0,|r|r.tick),server.lobby.players.len(),server.snapshot_bytes,worst,server.overloads,racing_at.map(|t|t.elapsed()));
        if server.overloads > 0 {
            return Err("server overload".into());
        }
        Ok(())
    });
    let result = (|| {
        let mut clients: Vec<_> = (0..16)
            .map(|i| {
                Client::connect(
                    &root,
                    address,
                    format!("Soak {i:02}"),
                    Assists::default(),
                    None,
                )
            })
            .collect::<Result<_, _>>()?;
        let mut races: Vec<Option<racecraft_sim::Race>> = (0..16).map(|_| None).collect();
        let mut drivers: Vec<Option<AiDriver>> = (0..16).map(|_| None).collect();
        let mut ready = [false; 16];
        let mut started = false;
        let mut racing_at = None;
        let mut previous = Instant::now();
        let mut debt = 0.;
        let mut snapshots = [0u64; 16];
        let deadline = Instant::now() + Duration::from_secs(seconds + 60);
        while Instant::now() < deadline {
            let now = Instant::now();
            let dt = now.duration_since(previous);
            previous = now;
            debt = (debt + dt.as_secs_f64()).min(0.1);
            for (i, client) in clients.iter_mut().enumerate() {
                client.poll(dt)?;
                if client.closed {
                    return Err(format!("client {i} closed: {:?}", client.error));
                }
                if !ready[i] && client.lobby.as_ref().is_some_and(|l| l.players.len() == 16) {
                    client.send(ClientMessage::Ready(true))?;
                    ready[i] = true;
                }
                if let Some(lobby) = client.load.take() {
                    let race = load_race(&root, &lobby.settings, &lobby.players)?;
                    let slot = client.slot().unwrap();
                    drivers[i] = Some(AiDriver::new(
                        DriverProfile {
                            pace: 0.93,
                            aggression: 0.4,
                            consistency: 1.,
                        },
                        &race.entrants[slot].car,
                        &race.track,
                        i as u64 + 1,
                    ));
                    races[i] = Some(race);
                    client.loaded()?;
                }
                if let Some(race) = &mut races[i] {
                    if client.reconcile(race)? {
                        snapshots[i] += 1;
                    }
                }
            }
            if !started
                && clients.iter().any(|c| {
                    c.owner()
                        && c.lobby.as_ref().is_some_and(|l| {
                            l.players.len() == 16 && l.players.iter().all(|p| p.ready)
                        })
                })
            {
                clients
                    .iter_mut()
                    .find(|c| c.owner())
                    .unwrap()
                    .send(ClientMessage::Start)?;
                started = true;
            }
            while debt >= 1. / PHYSICS_RATE {
                debt -= 1. / PHYSICS_RATE;
                for (i, client) in clients.iter_mut().enumerate() {
                    if let Some(race) = &mut races[i] {
                        let slot = client.slot().unwrap();
                        let input = drivers[i].as_mut().map_or(CarInput::default(), |d| {
                            d.drive(
                                &race.entrants[slot].car,
                                &race.track,
                                &[],
                                1. / PHYSICS_RATE,
                            )
                        });
                        client.predict(race, input)?;
                    }
                }
            }
            for client in &mut clients {
                client.flush()?;
            }
            if clients.iter().all(Client::active) {
                racing_at.get_or_insert(Instant::now());
            }
            if racing_at.is_some_and(|t| t.elapsed() >= Duration::from_secs(seconds)) {
                println!("CLIENTS: circuit=spa car=rb6 scripted=16 received_snapshots={snapshots:?} authoritative_ticks={:?}",clients.iter().map(|c|c.authoritative_tick).collect::<Vec<_>>());
                if snapshots.iter().any(|n| *n < seconds * 10) {
                    return Err("snapshot delivery below 10 Hz on a client".into());
                }
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Err("soak startup timed out".into())
    })();
    stop.store(true, Ordering::Relaxed);
    worker.join().map_err(|_| "server thread panicked")??;
    result
}
