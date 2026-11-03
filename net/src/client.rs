use crate::{codec::Encoded, *};
use racecraft_sim::{Assists, CarInput, Race, PHYSICS_RATE};
use renet::RenetClient;
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};
use std::{
    collections::VecDeque,
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    time::Duration,
};

/// Optional instrumentation for rendered multiplayer test sessions.
#[derive(Clone, Debug, serde::Serialize)]
pub struct AuthorityObservation {
    pub tick: u64,
    pub slot: usize,
    pub phase: String,
    pub hash: String,
    pub positions: Vec<[f64; 3]>,
    pub speeds: Vec<f64>,
    pub distances: Vec<f64>,
}
pub struct Client {
    net: RenetClient,
    transport: NetcodeClientTransport,
    root: PathBuf,
    name: String,
    assists: Assists,
    pub id: u64,
    pub identity: Option<DriverIdentity>,
    pub lobby: Option<Lobby>,
    pub error: Option<String>,
    pub closed: bool,
    pub load: Option<Lobby>,
    pub events: VecDeque<Event>,
    hello: bool,
    elapsed: Duration,
    sent: Duration,
    pending: VecDeque<InputSample>,
    commands: VecDeque<(u64, u64, Command)>,
    sequence: u64,
    /// Remote controls from the newest authoritative state, used for prediction.
    inputs: [PackedInput; MAX_PLAYERS],
    latest: Option<Box<State>>,
    /// Decoded snapshots kept as delta baselines, oldest first.
    received: VecDeque<(u64, Encoded)>,
    /// Newest snapshot tick held; reported to the server as the delta baseline.
    ack: u64,
    /// Encoded predicted race state per tick, to detect an exact prediction.
    history: VecDeque<(u64, Encoded)>,
    /// Ticks the prediction is ahead of where it needs to be; bled off gradually.
    excess: u64,
    steps: u64,
    /// Snapshots that needed a rollback and replay, and those that matched the prediction.
    pub resims: u64,
    pub resims_skipped: u64,
    pub authoritative_tick: Option<u64>,
    epoch: u64,
    pub last_correction: f64,
    pub observe: bool,
    pub observation: Option<AuthorityObservation>,
}
impl Client {
    pub fn connect(
        root: &Path,
        address: SocketAddr,
        name: String,
        assists: Assists,
        id: Option<u64>,
    ) -> Result<Self, String> {
        let id = id.unwrap_or_else(random_id);
        let bind = if address.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };
        let socket = UdpSocket::bind(bind).map_err(|e| e.to_string())?;
        let transport = NetcodeClientTransport::new(
            now(),
            ClientAuthentication::Unsecure {
                server_addr: address,
                client_id: id,
                user_data: None,
                protocol_id: PROTOCOL_ID,
            },
            socket,
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            net: RenetClient::new(connection_config()),
            transport,
            root: root.to_path_buf(),
            name,
            assists,
            id,
            identity: None,
            lobby: None,
            error: None,
            closed: false,
            load: None,
            events: VecDeque::new(),
            hello: false,
            elapsed: Duration::ZERO,
            sent: Duration::ZERO,
            pending: VecDeque::new(),
            commands: VecDeque::new(),
            sequence: 0,
            inputs: [PackedInput::default(); MAX_PLAYERS],
            latest: None,
            received: VecDeque::new(),
            ack: 0,
            history: VecDeque::new(),
            excess: 0,
            steps: 0,
            resims: 0,
            resims_skipped: 0,
            authoritative_tick: None,
            epoch: 0,
            last_correction: 0.,
            observe: false,
            observation: None,
        })
    }
    pub fn send(&mut self, message: ClientMessage) -> Result<(), String> {
        let channel = if matches!(message, ClientMessage::Inputs { .. }) {
            STATE
        } else {
            CONTROL
        };
        let bytes = encode(&message)?;
        if !self.net.can_send_message(channel, bytes.len()) {
            return Err("Network send queue is full".into());
        }
        self.net.send_message(channel, bytes);
        Ok(())
    }
    pub fn slot(&self) -> Option<usize> {
        self.lobby
            .as_ref()?
            .players
            .iter()
            .position(|p| p.id == self.id)
    }
    pub fn stage(&self) -> Stage {
        self.lobby.as_ref().map_or(Stage::Lobby, |l| l.stage)
    }
    pub fn owner(&self) -> bool {
        self.lobby.as_ref().is_some_and(|l| l.owner == self.id)
    }
    pub fn active(&self) -> bool {
        !self.closed && matches!(self.stage(), Stage::Grid | Stage::Racing | Stage::Results)
    }
    pub fn loaded(&mut self) -> Result<(), String> {
        self.send(ClientMessage::Loaded(self.epoch))
    }
    pub fn poll(&mut self, dt: Duration) -> Result<(), String> {
        if self.closed {
            return Ok(());
        }
        self.elapsed += dt;
        self.net.update(dt);
        if let Err(e) = self.transport.update(dt, &mut self.net) {
            self.closed = true;
            return Err(format!("Connection ended: {e}"));
        }
        if self.net.is_connected() && !self.hello {
            self.send(ClientMessage::Hello {
                name: self.name.clone(),
            })?;
            self.hello = true;
        }
        for channel in [CONTROL, STATE] {
            for _ in 0..256 {
                let Some(bytes) = self.net.receive_message(channel) else {
                    break;
                };
                match decode::<ServerMessage>(&bytes)? {
                    ServerMessage::Offer { settings, content } => {
                        if fingerprint(&self.root, &settings)? != content {
                            self.closed = true;
                            self.transport.disconnect();
                            return Err("Game version or converted track/car assets differ from the host. Install the same build and assets.".into());
                        }
                        self.send(ClientMessage::Accept {
                            content,
                            assists: self.assists,
                            identity: self.identity.clone(),
                        })?;
                    }
                    ServerMessage::Lobby(lobby) => {
                        if lobby.players.len() > MAX_PLAYERS {
                            return Err("Invalid lobby size".into());
                        }
                        self.lobby = Some(lobby);
                    }
                    ServerMessage::Load(lobby) => {
                        if lobby.epoch <= self.epoch {
                            continue;
                        }
                        self.epoch = lobby.epoch;
                        self.authoritative_tick = None;
                        self.latest = None;
                        self.pending.clear();
                        self.commands.clear();
                        self.events.clear();
                        self.sequence = 0;
                        self.inputs = [PackedInput::default(); MAX_PLAYERS];
                        self.received.clear();
                        self.history.clear();
                        self.ack = 0;
                        self.excess = 0;
                        self.load = Some(lobby.clone());
                        self.lobby = Some(lobby);
                    }
                    ServerMessage::State(frame) => {
                        // Newer snapshots supersede older ones; a delta whose
                        // baseline we no longer hold is dropped, and our older
                        // ack makes the server fall back to a full snapshot.
                        if frame.epoch != self.epoch
                            || self.received.back().is_some_and(|(t, _)| frame.tick <= *t)
                        {
                            continue;
                        }
                        let data = match frame.base {
                            None => frame.data,
                            Some(base) => match self.received.iter().find(|(t, _)| *t == base) {
                                Some((_, old)) => frame.data.xor(old),
                                None => continue,
                            },
                        };
                        let state: State = codec::decode(&data)?;
                        if state.epoch != frame.epoch || state.race.tick != frame.tick {
                            return Err("Inconsistent snapshot from host".into());
                        }
                        if self.received.len() >= 64 {
                            self.received.pop_front();
                        }
                        self.received.push_back((frame.tick, data));
                        self.ack = frame.tick;
                        self.latest = Some(Box::new(state));
                    }
                    ServerMessage::Events { epoch, events } if epoch == self.epoch => {
                        for event in events {
                            if self.events.len() >= 1024 {
                                self.events.pop_front();
                            }
                            self.events.push_back(event);
                        }
                    }
                    ServerMessage::Error(error) => self.error = Some(error),
                    ServerMessage::Closed(reason) => {
                        self.error = Some(reason);
                        self.closed = true;
                    }
                    _ => {}
                }
            }
        }
        if self.transport.time_since_last_received_packet() > TIMEOUT
            || self.lobby.is_none() && self.elapsed > TIMEOUT
        {
            self.closed = true;
            return Err("Host is not responding. The race has ended.".into());
        }
        if matches!(self.stage(), Stage::Grid | Stage::Racing)
            && self.elapsed - self.sent >= Duration::from_secs_f64(1. / INPUT_RATE as f64)
        {
            self.sent = self.elapsed;
            let newest = self.pending.back().map_or(0, |s| s.tick);
            let samples = self
                .pending
                .iter()
                .rev()
                .take(MAX_INPUT_SAMPLES)
                .filter_map(|s| Some((u16::try_from(newest - s.tick).ok()?, s.input)))
                .collect();
            self.send(ClientMessage::Inputs {
                epoch: self.epoch,
                ack: self.ack,
                newest,
                samples,
            })?;
        }
        self.transport
            .send_packets(&mut self.net)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    /// Apply the newest authoritative state.
    ///
    /// When the prediction already reached exactly that state with the same
    /// remote controls, nothing changes and this returns `Ok(false)`. Otherwise
    /// the race rolls back to the snapshot, replays pending inputs up to the
    /// prediction lead and returns `Ok(true)` so the renderer can blend.
    pub fn reconcile(&mut self, race: &mut Race) -> Result<bool, String> {
        let Some(state) = self.latest.take() else {
            return Ok(false);
        };
        let slot = self.slot().ok_or("Host did not assign a driver slot")?;
        let tick = state.race.tick;
        let natural = tick + prediction_lead(self.net.rtt());
        self.authoritative_tick = Some(tick);
        self.pending.retain(|s| s.tick > tick);
        self.commands
            .retain(|(seq, _, _)| *seq > state.commands[slot]);
        let authoritative = codec::encode(&state.race)?;
        // Exact when the remote cars were predicted with these controls, every
        // own tick since was a real sample, and the stored state matches.
        let remote_unchanged =
            (0..MAX_PLAYERS).all(|i| i == slot || self.inputs[i] == state.inputs[i]);
        let own_covered = race.tick >= tick
            && self.pending.iter().filter(|s| s.tick <= race.tick).count() as u64
                == race.tick - tick;
        let predicted = self
            .history
            .iter()
            .find(|(t, _)| *t == tick)
            .is_some_and(|(_, predicted)| *predicted == authoritative);
        self.inputs = state.inputs;
        if !self.observe && remote_unchanged && own_covered && predicted {
            self.history.retain(|(t, _)| *t >= tick);
            self.excess = race.tick.saturating_sub(natural + LEAD_SLACK);
            self.last_correction = 0.;
            self.resims_skipped += 1;
            return Ok(false);
        }
        // Never step the local timeline backwards; a surplus lead is bled off
        // in `predict` instead, so it cannot ratchet up after a latency spike.
        let target = race.tick.max(natural).min(tick + MAX_LEAD);
        let previous = race.entrants[slot].car.position;
        race.restore(&state.race)?;
        if self.observe && tick.is_multiple_of(120) {
            use sha2::{Digest, Sha256};
            let hash = Sha256::digest(encode(&race.snapshot())?)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            self.observation = Some(AuthorityObservation {
                tick,
                slot,
                phase: format!("{:?}", race.phase),
                hash,
                positions: race
                    .entrants
                    .iter()
                    .map(|e| e.car.position.to_array())
                    .collect(),
                speeds: race.entrants.iter().map(|e| e.car.speed()).collect(),
                distances: race.entrants.iter().map(|e| e.timing.distance).collect(),
            });
        }
        self.history.clear();
        self.history.push_back((tick, authoritative));
        let remote = state.inputs.map(|p| p.unpack());
        let events = std::mem::take(&mut race.events);
        while race.tick < target && self.stage() != Stage::Results {
            let next = race.tick + 1;
            let mut inputs = remote;
            if let Ok(i) = self.pending.binary_search_by_key(&next, |s| s.tick) {
                inputs[slot] = self.pending[i].input.unpack();
            }
            for (_, t, cmd) in &self.commands {
                if *t == next {
                    apply_command(race, slot, &mut inputs[slot], *cmd);
                }
            }
            race.step(&inputs);
            race.events.clear();
            self.record(race)?;
        }
        race.events = events;
        self.excess = race.tick.saturating_sub(natural + LEAD_SLACK);
        self.last_correction = previous.distance(race.entrants[slot].car.position);
        self.resims += 1;
        Ok(true)
    }
    fn record(&mut self, race: &Race) -> Result<(), String> {
        if self.history.len() >= HISTORY {
            self.history.pop_front();
        }
        self.history
            .push_back((race.tick, codec::encode(&race.snapshot())?));
        Ok(())
    }
    /// Physics input targets one explicit server tick. Commands travel reliably; held controls are redundant UDP.
    pub fn predict(&mut self, race: &mut Race, mut input: CarInput) -> Result<(), String> {
        let Some(authoritative) = self.authoritative_tick else {
            return Ok(());
        };
        if !self.active() || self.stage() == Stage::Results || race.tick >= authoritative + MAX_LEAD
        {
            return Ok(());
        }
        let slot = self.slot().ok_or("Missing player slot")?;
        let tick = race.tick + 1;
        if input.shift_up {
            self.command_at(tick, Command::ShiftUp)?;
        }
        if input.shift_down {
            self.command_at(tick, Command::ShiftDown)?;
        }
        // Hold one tick now and then (about 6%) while ahead of the needed lead.
        // Any command just queued for this tick is applied on the next call.
        if self.excess > 0 {
            self.steps += 1;
            if self.steps.is_multiple_of(THROTTLE_EVERY) {
                self.excess -= 1;
                return Ok(());
            }
        }
        // Predict with exactly the resolution the server will simulate.
        let packed = PackedInput::pack(&input);
        input = packed.unpack();
        let mut inputs = self.inputs.map(|p| p.unpack());
        inputs[slot] = input;
        for (_, t, cmd) in &self.commands {
            if *t == tick {
                apply_command(race, slot, &mut inputs[slot], *cmd);
            }
        }
        self.pending.push_back(InputSample {
            tick,
            input: packed,
        });
        while self.pending.len() > MAX_LEAD as usize {
            self.pending.pop_front();
        }
        let events = std::mem::take(&mut race.events);
        race.step(&inputs);
        race.events = events;
        self.record(race)
    }
    fn command_at(&mut self, tick: u64, kind: Command) -> Result<(), String> {
        if self.commands.len() >= 64 {
            return Err("Too many pending commands".into());
        }
        self.sequence += 1;
        self.commands.push_back((self.sequence, tick, kind));
        self.send(ClientMessage::Command {
            epoch: self.epoch,
            sequence: self.sequence,
            tick,
            kind,
        })
    }
    pub fn recover(&mut self, tick: u64) -> Result<(), String> {
        self.command_at(tick, Command::Recover)
    }
    pub fn flush(&mut self) -> Result<(), String> {
        self.transport
            .send_packets(&mut self.net)
            .map_err(|e| e.to_string())
    }
    pub fn rtt_ms(&self) -> f64 {
        self.net.rtt() * 1000.
    }
}
/// Predicted states kept for exact-match checks; covers the maximum lead.
const HISTORY: usize = 2 * MAX_LEAD as usize;
/// Lead above the latency-based target tolerated before bleeding it off,
/// so ordinary round-trip jitter does not cause skipped ticks.
const LEAD_SLACK: u64 = 2;
/// While ahead, skip one predicted tick in this many (about 22 ticks per second).
const THROTTLE_EVERY: u64 = 16;
/// Keep enough predicted time for one-way transport delay and the next 60 Hz input send.
/// The fixed 33 ms baseline remains useful on a local connection.
fn prediction_lead(rtt_seconds: f64) -> u64 {
    let network_ticks = (rtt_seconds.max(0.) * PHYSICS_RATE / 2.).ceil() as u64;
    (INPUT_LEAD + network_ticks).min(MAX_LEAD)
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.send(ClientMessage::Leave);
        let _ = self.flush();
        self.transport.disconnect();
    }
}

fn apply_command(race: &mut Race, slot: usize, input: &mut CarInput, command: Command) {
    match command {
        Command::Recover => race.recover(slot),
        Command::ShiftUp => input.shift_up = true,
        Command::ShiftDown => input.shift_down = true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prediction_lead_grows_with_latency_but_stays_bounded() {
        assert_eq!(prediction_lead(0.), INPUT_LEAD);
        assert!(prediction_lead(0.100) > INPUT_LEAD);
        assert_eq!(prediction_lead(10.), MAX_LEAD);
    }
}
