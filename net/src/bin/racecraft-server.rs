use racecraft_net::{RaceSettings, Server, Stage};
use std::{
    net::SocketAddr,
    path::PathBuf,
    time::{Duration, Instant},
};
fn main() -> Result<(), String> {
    let mut root = PathBuf::from("assets");
    let mut address: SocketAddr = "0.0.0.0:7777".parse().unwrap();
    let mut settings = RaceSettings::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--assets" => root = args.next().ok_or("--assets needs a directory")?.into(),
            "--bind" => {
                address = args
                    .next()
                    .ok_or("--bind needs IP:port")?
                    .parse()
                    .map_err(|_| "invalid bind address")?
            }
            "--settings" => {
                settings = serde_json::from_slice(
                    &std::fs::read(args.next().ok_or("--settings needs a JSON file")?)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?
            }
            "--help" | "-h" => {
                println!("racecraft-server --assets DIR --bind 0.0.0.0:7777 [--settings rules.json]\nUse --bind [::]:7777 to accept IPv6 and IPv4 where the OS allows it.\nThe first admitted player owns the lobby; ownership passes on when they leave. Allow UDP on this port.");
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    // A dedicated server hands a room to the next driver when its owner leaves
    // and resets when it empties. A room that closes (host overload, a load
    // timeout) is replaced by a fresh one on the same port.
    loop {
        let mut server = Server::bind(&root, address, settings.clone(), None)?;
        println!(
            "Race server listening on {}. First player owns the room.",
            server.address()
        );
        let mut previous = Instant::now();
        let mut report = previous;
        while server.lobby.stage != Stage::Closed {
            let now = Instant::now();
            server.update(now.duration_since(previous))?;
            previous = now;
            if now.duration_since(report) > Duration::from_secs(10) {
                println!(
                    "drivers={} phase={:?} tick={} snapshot={} bytes (full {}) overloads={}",
                    server.lobby.players.len(),
                    server.lobby.stage,
                    server.race.as_ref().map_or(0, |r| r.tick),
                    server.snapshot_bytes,
                    server.full_snapshot_bytes,
                    server.overloads
                );
                report = now;
            }
            std::thread::sleep(server.idle_time());
        }
        println!("Room closed; opening a new one.");
        // Let the close message reach clients before the socket is replaced.
        for _ in 0..20 {
            let _ = server.update(Duration::from_millis(5));
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
