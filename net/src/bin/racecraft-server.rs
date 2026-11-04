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
    let mut websocket = false;
    let mut explicit_bind = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--assets" => root = args.next().ok_or("--assets needs a directory")?.into(),
            "--websocket" => websocket = true,
            "--bind" => {
                explicit_bind = true;
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
                println!("racecraft-server --assets DIR [--websocket] [--bind IP:PORT] [--settings rules.json]\nUse --bind [::]:7777 to accept IPv6 and IPv4 where the OS allows it.\n--websocket serves /ws and /health over HTTP; defaults to 0.0.0.0:$PORT (10000 without PORT). Use a TLS proxy for public WSS.\nThe first admitted player owns the lobby; ownership passes on when they leave. Allow UDP on this port.");
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    if websocket && !explicit_bind {
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "10000".into())
            .parse::<u16>()
            .map_err(|_| "PORT must be an integer from 1 to 65535")?;
        if port == 0 {
            return Err("PORT must be nonzero".into());
        }
        address = SocketAddr::from(([0, 0, 0, 0], port));
    }
    // A dedicated server hands a room to the next driver when its owner leaves
    // and resets when it empties. A room that closes (host overload, a load
    // timeout) is replaced by a fresh one on the same port.
    loop {
        let mut server = if websocket {
            Server::bind_websocket(&root, address, settings.clone())?
        } else {
            Server::bind(&root, address, settings.clone(), None)?
        };
        let mode = if websocket {
            "WebSocket /ws, HTTP /health"
        } else {
            "UDP"
        };
        println!(
            "Race server listening on {} ({mode}). First player owns the room.",
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
