//! Graphical lobby and the bridge from authoritative networking into the existing Session.
use crate::{
    menu::{Frontend, Screen},
    options::Options,
    pose::Poses,
    Session,
};
use bevy::prelude::*;
use bevy_egui::egui;
use racecraft_net::{
    Client, ClientMessage, Endpoint, Lobby, RaceSettings, Server, ServerHandle, Stage, MAX_PLAYERS,
};
use std::{net::SocketAddr, time::Duration};

#[derive(Resource)]
pub struct Online {
    pub client: Option<Client>,
    host: Option<ServerHandle>,
    open: bool,
    identity: Option<racecraft_net::DriverIdentity>,
    name: String,
    address: String,
    port: u16,
    settings: RaceSettings,
    draft_owner: Option<u64>,
    pub error: Option<String>,
    install: Option<Lobby>,
    loading: Option<u64>,
    last_event: u64,
    request: Option<bool>,
    pub pilot: Option<racecraft_sim::AiDriver>,
    test_started: bool,
}
impl Online {
    pub fn new(options: &Options) -> Self {
        Self {
            pilot: None,
            test_started: false,
            client: None,
            host: None,
            open: options.host.is_some() || options.join.is_some(),
            identity: if !options.team.is_empty() && !options.driver.is_empty() {
                Some(racecraft_net::DriverIdentity {
                    team: options.team.clone(),
                    driver: options.driver.clone(),
                })
            } else {
                None
            },
            name: options.name.clone(),
            address: options
                .join
                .clone()
                .unwrap_or_else(|| "127.0.0.1:7777".into()),
            port: options.host.unwrap_or(7777),
            draft_owner: None,
            settings: RaceSettings {
                track: options.track.clone(),
                car: options.car.clone(),
                laps: options.laps,
                ai_cars: options.ai_cars,
                setup: options.setup,
                assists: options.player_assists(),
                corner_braking: options.corner_braking,
                ..RaceSettings::default()
            },
            error: None,
            install: None,
            loading: None,
            last_event: 0,
            request: if options.host.is_some() {
                Some(true)
            } else if options.join.is_some() {
                Some(false)
            } else {
                None
            },
        }
    }
    fn begin(
        &mut self,
        host: bool,
        options: &Options,
        aids: racecraft_sim::Assists,
    ) -> Result<(), String> {
        if self.name.trim().is_empty() || self.name.len() > 32 {
            return Err("Choose a driver name of 1–32 bytes".into());
        }
        let id = racecraft_net::random_id();
        let address = if host {
            // Dual-stack where available, so IPv6 friends can join too; plain
            // IPv4 otherwise (no IPv6 on this machine, or the OS refuses).
            let bind = |address: SocketAddr| {
                Server::bind(&options.assets, address, self.settings.clone(), Some(id))
            };
            let server = bind(SocketAddr::from((
                std::net::Ipv6Addr::UNSPECIFIED,
                self.port,
            )))
            .or_else(|_| bind(SocketAddr::from(([0, 0, 0, 0], self.port))))?;
            self.host = Some(ServerHandle::spawn(server));
            Endpoint::Udp(SocketAddr::from(([127, 0, 0, 1], self.port)))
        } else {
            self.address.parse::<Endpoint>()?
        };
        match Client::connect(
            &options.assets,
            address,
            self.name.trim().to_owned(),
            aids,
            Some(id),
        ) {
            Ok(mut client) => {
                client.identity = self.identity.clone();
                client.observe = options.test_driver;
                self.client = Some(client);
            }
            Err(error) => {
                self.host = None;
                return Err(error);
            }
        }
        self.error = None;
        self.last_event = 0;
        Ok(())
    }
    fn leave(&mut self, session: &mut Session) {
        self.pilot = None;
        self.test_started = false;
        self.client = None;
        self.host = None;
        self.draft_owner = None;
        self.install = None;
        self.loading = None;
        session.paused = true;
        session.screen = Screen::Main;
        session.controls = default();
    }
    /// Returns true while this panel owns race/session controls.
    pub fn ui(
        &mut self,
        context: &egui::Context,
        frontend: &mut Frontend,
        session: &mut Session,
        options: &Options,
    ) -> bool {
        if self.client.is_none() && !self.open {
            egui::Area::new("multiplayer-open".into())
                .anchor(egui::Align2::RIGHT_TOP, [-16., 16.])
                .show(context, |ui| {
                    if ui.button("Multiplayer").clicked() {
                        self.open = true;
                    }
                });
            return false;
        }
        if self
            .client
            .as_ref()
            .is_some_and(|c| c.active() && c.stage() != Stage::Results)
            && session.screen == Screen::Racing
        {
            egui::Area::new("online-status".into())
                .anchor(egui::Align2::CENTER_TOP, [0., 8.])
                .show(context, |ui| {
                    let client = self.client.as_ref().unwrap();
                    ui.label(format!("ONLINE · {:.0} ms · Esc for room", client.rtt_ms()));
                });
            return true;
        }
        let mut leave = false;
        let mut start = None;
        let mut action = None;
        egui::Window::new("Multiplayer · 16 drivers")
            .default_width(430.)
            .collapsible(false)
            .vscroll(true)
            .max_height((context.viewport_rect().height() - 60.).max(200.))
            .show(context, |ui| {
                if let Some(error) = &self.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                if let Some(client) = &mut self.client {
                    if let Some(error) = client.error.take() {
                        self.error = Some(error);
                    }
                    if let Some(lobby) = client.lobby.clone() {
                        if self.draft_owner != Some(lobby.owner) {
                            self.settings = lobby.settings.clone();
                            self.draft_owner = Some(lobby.owner);
                        }
                        ui.label(format!(
                            "{} humans + {} AI / {} cars · {:?} · {:.0} ms",
                            lobby.players.len(),
                            lobby.settings.ai_cars,
                            MAX_PLAYERS,
                            lobby.stage,
                            client.rtt_ms()
                        ));
                        ui.label(format!(
                            "{} · {} · {} laps",
                            lobby.settings.track, lobby.settings.car, lobby.settings.laps
                        ));
                        if self.host.is_some() {
                            ui.label(format!(
                                "Hosting on UDP port {}. Share your LAN/VPN IP.",
                                self.port
                            ));
                        }
                        for (slot, player) in lobby.players.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "{}  {}{}  {}",
                                    slot + 1,
                                    player.name,
                                    if player.id == lobby.owner {
                                        " (owner)"
                                    } else {
                                        ""
                                    },
                                    if !player.connected {
                                        "AI takeover"
                                    } else if player.ready {
                                        "Ready"
                                    } else {
                                        "Waiting"
                                    }
                                ));
                                if lobby.stage == Stage::Lobby && client.owner() {
                                    let mut ids: Vec<_> =
                                        lobby.players.iter().map(|p| p.id).collect();
                                    if ui.add_enabled(slot > 0, egui::Button::new("↑")).clicked()
                                    {
                                        ids.swap(slot, slot - 1);
                                        action = Some(ClientMessage::Grid(ids.clone()));
                                    }
                                    if ui
                                        .add_enabled(slot + 1 < ids.len(), egui::Button::new("↓"))
                                        .clicked()
                                    {
                                        ids.swap(slot, slot + 1);
                                        action = Some(ClientMessage::Grid(ids));
                                    }
                                }
                            });
                        }
                        if lobby.stage == Stage::Lobby {
                            let ready = lobby
                                .players
                                .iter()
                                .find(|p| p.id == client.id)
                                .is_some_and(|p| p.ready);
                            if ui
                                .button(if ready { "Not ready" } else { "Ready" })
                                .clicked()
                            {
                                action = Some(ClientMessage::Ready(!ready));
                            }
                            if client.owner() {
                                let mut settings = self.settings.clone();
                                let before = lobby.settings.clone();
                                ui.add(egui::Slider::new(&mut settings.laps, 1..=100).text("Laps"));
                                let max_ai = MAX_PLAYERS - lobby.players.len();
                                ui.add(
                                    egui::Slider::new(&mut settings.ai_cars, 0..=max_ai)
                                        .text("AI cars"),
                                );
                                ui.checkbox(
                                    &mut settings.lock_aids,
                                    "Everyone uses the owner's aids",
                                );
                                aids_panel(ui, &mut settings);
                                if ui.button("Reset shared rules to defaults").clicked() {
                                    settings = RaceSettings {
                                        car: lobby.settings.car.clone(),
                                        track: lobby.settings.track.clone(),
                                        ..RaceSettings::default()
                                    };
                                    action = Some(ClientMessage::Settings(settings.clone()));
                                }
                                // The controls above edit a draft; the room changes only on Apply.
                                // Starting with unapplied edits would race with rules the
                                // controls no longer show, so require Apply or Discard first.
                                let unapplied = settings != before && action.is_none();
                                if unapplied {
                                    ui.colored_label(
                                        egui::Color32::YELLOW,
                                        "Rule changes are not applied to the room yet.",
                                    );
                                    ui.horizontal(|ui| {
                                        if ui
                                            .button("Apply race settings (clears readiness)")
                                            .clicked()
                                        {
                                            action =
                                                Some(ClientMessage::Settings(settings.clone()));
                                        }
                                        if ui.button("Discard changes").clicked() {
                                            settings = before.clone();
                                        }
                                    });
                                }
                                // Keep the draft across frames without mutating server settings on a slider drag.
                                self.settings = settings;
                                if ui
                                    .add_enabled(
                                        can_start(&lobby, unapplied),
                                        egui::Button::new("Start race"),
                                    )
                                    .clicked()
                                {
                                    action = Some(ClientMessage::Start);
                                }
                            }
                        } else if lobby.stage == Stage::Loading {
                            ui.label("Loading track and cars. Waiting for every driver…");
                        } else if lobby.stage == Stage::Results {
                            for &slot in session.race.order() {
                                ui.label(format!(
                                    "P{}  {}",
                                    session.race.position_of(slot),
                                    session.race.entrants[slot].name
                                ));
                            }
                            if client.owner() && ui.button("Return everyone to lobby").clicked() {
                                action = Some(ClientMessage::Return);
                            }
                        } else {
                            if session.screen != Screen::Racing
                                && ui.button("Resume driving").clicked()
                            {
                                crate::menu::resume(session);
                            }
                            ui.label("Opening this menu does not pause the shared race.");
                            ui.add(
                                egui::Slider::new(&mut frontend.audio.master, 0. ..=1.)
                                    .text("Local audio"),
                            );
                            ui.checkbox(&mut frontend.muted, "Mute");
                        }
                    } else {
                        ui.label("Connecting and checking matching assets…");
                    }
                    if ui.button("Leave room").clicked() {
                        leave = true;
                    }
                } else {
                    ui.label("Driver name");
                    ui.text_edit_singleline(&mut self.name);
                    if !frontend.teams.teams.is_empty() {
                        egui::ComboBox::from_label("GEN22 driver identity")
                            .selected_text(self.identity.as_ref().map_or_else(
                                || "Assign a free driver".to_owned(),
                                |i| format!("{} / {}", i.team, i.driver),
                            ))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.identity,
                                    None,
                                    "Assign a free driver",
                                );
                                for team in &frontend.teams.teams {
                                    for driver in &team.drivers {
                                        ui.selectable_value(
                                            &mut self.identity,
                                            Some(racecraft_net::DriverIdentity {
                                                team: team.id.clone(),
                                                driver: driver.id.clone(),
                                            }),
                                            format!("{} · {}", team.name, driver.name),
                                        );
                                    }
                                }
                            });
                    }
                    ui.label("Join address (LAN IP:port or wss://server.onrender.com)");
                    ui.text_edit_singleline(&mut self.address);
                    if ui.button("Join room").clicked() {
                        start = Some(false);
                    }
                    ui.separator();
                    ui.label("Create a room");
                    ui.add(
                        egui::DragValue::new(&mut self.port)
                            .range(1024..=65535)
                            .prefix("UDP port "),
                    );
                    egui::ComboBox::from_label("Car")
                        .selected_text(&self.settings.car)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.settings.car, "rb6".into(), "RB6");
                            ui.selectable_value(&mut self.settings.car, "gen22".into(), "GEN22");
                        });
                    ui.label(format!(
                        "Circuit: {} (select another circuit in Race setup)",
                        frontend.draft.track
                    ));
                    self.settings.track = frontend.draft.track.clone();
                    ui.add(egui::Slider::new(&mut self.settings.laps, 1..=100).text("Laps"));
                    ui.add(
                        egui::Slider::new(&mut self.settings.ai_cars, 0..=MAX_PLAYERS - 1)
                            .text("AI cars"),
                    );
                    ui.checkbox(&mut self.settings.lock_aids, "Lock aids for everyone");
                    aids_panel(ui, &mut self.settings);
                    if ui.button("Host room").clicked() {
                        start = Some(true);
                    }
                    if ui.button("Back to offline").clicked() {
                        self.open = false;
                    }
                }
            });
        if let Some(host) = start {
            if let Err(error) = self.begin(host, options, frontend.aids) {
                self.error = Some(error);
            }
        }
        if let Some(action) = action {
            if let Some(client) = &mut self.client {
                if let Err(error) = client.send(action) {
                    self.error = Some(error);
                }
            }
        }
        if leave {
            self.leave(session);
        }
        self.client.is_some()
    }
}
/// The owner may start once every driver is ready and the rules shown are the room's rules.
fn can_start(lobby: &Lobby, unapplied: bool) -> bool {
    !unapplied && lobby.players.iter().all(|p| p.ready)
}
fn aids_panel(ui: &mut egui::Ui, settings: &mut RaceSettings) {
    ui.collapsing("Custom shared rules", |ui| {
        let mut level = settings.assists.steering;
        if ui
            .add(egui::Slider::new(&mut level, 0. ..=1.).text("Driving aids"))
            .changed()
        {
            settings.assists.steering = level;
            settings.assists.braking = level;
            settings.assists.traction = level;
            settings.assists.stability = level;
            settings.corner_braking = level;
        }
        ui.checkbox(&mut settings.assists.automatic_gears, "Automatic gears");
        ui.checkbox(&mut settings.assists.drs_anywhere, "DRS anywhere");
        ui.add(egui::Slider::new(&mut settings.setup.front_wing, 0. ..=10.).text("Front wing"));
        ui.add(egui::Slider::new(&mut settings.setup.rear_wing, 0. ..=10.).text("Rear wing"));
        egui::ComboBox::from_label("Tyres")
            .selected_text(format!("{:?}", settings.setup.compound))
            .show_ui(ui, |ui| {
                for compound in [
                    racecraft_sim::Compound::Soft,
                    racecraft_sim::Compound::Medium,
                    racecraft_sim::Compound::Hard,
                ] {
                    ui.selectable_value(
                        &mut settings.setup.compound,
                        compound,
                        format!("{compound:?}"),
                    );
                }
            });
    });
}
/// Poll on real display time; loading and opening menus never stops the network.
pub fn poll(
    mut online: ResMut<Online>,
    time: Res<Time<Real>>,
    mut frontend: ResMut<Frontend>,
    mut session: ResMut<Session>,
    options: Res<Options>,
    mut log_stage: Local<Option<(Stage, usize)>>,
) {
    if let Some(host) = online.request.take() {
        if let Err(error) = online.begin(host, &options, frontend.aids) {
            online.error = Some(error);
        }
    }
    let mut test_started = online.test_started;
    let Some(client) = &mut online.client else {
        return;
    };
    if let Err(error) = client.poll(Duration::from_secs_f64(time.delta_secs_f64())) {
        client.error = Some(error);
        client.closed = true;
    }
    if let Some(lobby) = &client.lobby {
        if *log_stage != Some((lobby.stage, lobby.players.len())) {
            info!(
                "Online phase {:?}, {} drivers admitted",
                lobby.stage,
                lobby.players.len()
            );
            *log_stage = Some((lobby.stage, lobby.players.len()));
        }
    }
    if let Some(expected) = options.test_lobby {
        if let Some(lobby) = client.lobby.clone() {
            if lobby.stage == Stage::Lobby && lobby.players.len() >= expected {
                if lobby
                    .players
                    .iter()
                    .find(|p| p.id == client.id)
                    .is_some_and(|p| !p.ready)
                {
                    let _ = client.send(ClientMessage::Ready(true));
                }
                if client.owner() && !test_started && lobby.players.iter().all(|p| p.ready) {
                    let _ = client.send(ClientMessage::Start);
                    test_started = true;
                }
            }
        }
    }
    if client.closed {
        let error = client
            .error
            .take()
            .unwrap_or_else(|| "The room closed.".into());
        online.leave(&mut session);
        online.error = Some(error);
        return;
    }
    if let Some(lobby) = client.load.take() {
        let Some(slot) = lobby.players.iter().position(|p| p.id == client.id) else {
            return;
        };
        let settings = &lobby.settings;
        frontend.draft.track = settings.track.clone();
        frontend.draft.car = settings.car.clone();
        frontend.draft.laps = settings.laps;
        frontend.draft.setup = settings.setup;
        frontend.draft.opponents = settings.ai_cars + lobby.players.len() - 1;
        frontend.draft.grid_slot = slot + 1;
        if let Some(identity) = &lobby.players[slot].identity {
            frontend.draft.team = identity.team.clone();
            frontend.draft.driver = identity.driver.clone();
        }
        frontend.aids = if settings.lock_aids {
            settings.assists
        } else {
            lobby.players[slot].assists
        };
        // DRS is always a room rule, even when ordinary aids are personal.
        frontend.aids.drs_anywhere = settings.assists.drs_anywhere;
        frontend.corner_braking = if settings.lock_aids {
            settings.corner_braking
        } else {
            frontend.corner_braking
        };
        frontend.pending = true;
        online.last_event = 0;
        online.loading = Some(lobby.epoch);
        online.install = Some(lobby);
    }
    online.test_started = test_started;
}
pub fn install(
    mut online: ResMut<Online>,
    mut session: ResMut<Session>,
    mut frontend: ResMut<Frontend>,
    options: Res<Options>,
    time: Res<Time<Real>>,
) {
    if options.test_driver {
        frontend.vsync = true;
    }
    session.poses.advance_correction(time.delta_secs_f64());
    if let Some(lobby) = online.install.take() {
        if let Some(error) = frontend.error.clone() {
            online.error = Some(error);
            online.leave(&mut session);
            return;
        }
        match racecraft_net::load_race(&options.assets, &lobby.settings, &lobby.players) {
            Ok(race) => {
                session.guidance = crate::driving::Guidance::new(&race, session.player);
                session.poses = Poses::new(&race);
                session.race = race;
                if options.test_driver {
                    online.pilot = Some(racecraft_sim::AiDriver::new(
                        racecraft_sim::DriverProfile {
                            pace: 0.93,
                            aggression: 0.5,
                            consistency: 1.,
                        },
                        &session.race.entrants[session.player].car,
                        &session.race.track,
                        session.player as u64 + 100,
                    ));
                }
                if options.car == "gen22" {
                    let mut field = Vec::new();
                    for player in &lobby.players {
                        if let Some(identity) = &player.identity {
                            match frontend.teams.selection(&identity.team, &identity.driver) {
                                Ok((team, driver)) => {
                                    field.push(Some(crate::teams::Teams::identity(team, driver)))
                                }
                                Err(error) => {
                                    online.error = Some(error);
                                    online.leave(&mut session);
                                    return;
                                }
                            }
                        }
                    }
                    let remaining: Vec<_> = frontend
                        .teams
                        .teams
                        .iter()
                        .flat_map(|team| {
                            team.drivers
                                .iter()
                                .map(move |driver| crate::teams::Teams::identity(team, driver))
                        })
                        .filter(|identity| {
                            !field.iter().flatten().any(|i| i.driver == identity.driver)
                        })
                        .collect();
                    field.extend(
                        remaining
                            .into_iter()
                            .map(Some)
                            .take(session.race.entrants.len().saturating_sub(field.len())),
                    );
                    session.identities = field;
                }
            }
            Err(error) => {
                online.error = Some(error);
                online.leave(&mut session);
                return;
            }
        }
    }
    let ready = session.ready;
    if ready && online.loading.is_some() {
        online.loading = None;
        if let Some(client) = &mut online.client {
            if let Err(error) = client.loaded() {
                online.error = Some(error);
            }
        }
    }
    let mut last_event = online.last_event;
    if let Some(client) = &mut online.client {
        if client.active() && ready {
            let before: Vec<_> = (0..session.race.entrants.len())
                .map(|i| session.poses.at(&session.race, i, 1.))
                .collect();
            match client.reconcile(&mut session.race) {
                Ok(true) => {
                    if let Some(observation) = client.observation.take() {
                        if let Ok(json) = serde_json::to_string(&observation) {
                            info!("ONLINE_TEST {json}");
                        }
                    }
                    let Session { poses, race, .. } = &mut *session;
                    poses.correct(&before, race);
                }
                Err(error) => {
                    client.error = Some(error);
                    client.closed = true;
                }
                _ => {}
            }
            // Predicted events never reach the HUD; authoritative reliable events do.
            for event in client.events.drain(..) {
                if event.id > last_event {
                    last_event = event.id;
                    session.race.events.push(event.event);
                }
            }
            if session.race.events.len() > 1024 {
                session.race.events.clear();
                session.messages.seen_events = 0;
            }
            if let Some(lobby) = &client.lobby {
                let slot = session.player;
                let mut aids = if lobby.settings.lock_aids {
                    lobby.settings.assists
                } else {
                    lobby.players[slot].assists
                };
                aids.drs_anywhere = lobby.settings.assists.drs_anywhere;
                session.race.entrants[slot].car.assists = aids;
                frontend.aids = aids;
                session.corner_braking = if lobby.settings.lock_aids {
                    lobby.settings.corner_braking
                } else {
                    frontend.corner_braking
                };
            }
        }
    }
    online.last_event = last_event;
}

#[cfg(test)]
mod tests {
    use super::*;
    use racecraft_net::Player;
    #[test]
    fn online_pause_keeps_prediction_alive_but_offline_pause_stops_the_race() {
        let options = Options::parse(std::iter::empty()).unwrap();
        let mut session = crate::tests::session();
        session.ready = true;
        session.paused = true;
        session.screen = Screen::Paused;
        let mut client = Client::connect(
            std::path::Path::new("."),
            "127.0.0.1:9".parse().unwrap(),
            "Test".into(),
            racecraft_sim::Assists::default(),
            Some(1),
        )
        .unwrap();
        client.lobby = Some(Lobby {
            settings: RaceSettings::default(),
            players: vec![Player {
                id: 1,
                name: "Test".into(),
                ready: true,
                connected: true,
                identity: None,
                assists: racecraft_sim::Assists::default(),
            }],
            owner: 1,
            epoch: 0,
            stage: Stage::Racing,
        });
        client.authoritative_tick = Some(session.race.tick);
        let mut online = Online::new(&options);
        online.client = Some(client);
        let mut app = App::new();
        app.insert_resource(session)
            .insert_resource(online)
            .add_systems(Update, crate::step_race);
        app.update();
        assert_eq!(app.world().resource::<Session>().race.tick, 1);
        app.world_mut().resource_mut::<Online>().client = None;
        app.update();
        assert_eq!(app.world().resource::<Session>().race.tick, 1);
    }
    #[test]
    fn start_waits_for_unapplied_rule_changes() {
        let lobby = Lobby {
            settings: RaceSettings::default(),
            players: vec![Player {
                id: 1,
                name: "Owner".into(),
                ready: true,
                connected: true,
                identity: None,
                assists: racecraft_sim::Assists::default(),
            }],
            owner: 1,
            epoch: 0,
            stage: Stage::Lobby,
        };
        assert!(can_start(&lobby, false));
        // e.g. AI cars moved to 0 on the slider but not applied yet.
        assert!(!can_start(&lobby, true));
    }

    #[test]
    fn host_join_panel_renders_with_existing_frontend() {
        let options = Options::parse(std::iter::empty()).unwrap();
        let mut online = Online::new(&options);
        online.open = true;
        let mut frontend = Frontend::new(&options);
        let mut session = crate::tests::session();
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            online.ui(ui.ctx(), &mut frontend, &mut session, &options);
        });
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
    }
}
