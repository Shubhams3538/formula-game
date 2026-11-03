//! Race setup and pause menus. Racing keeps controls on keyboard/gamepad.
//! Setup stays editable; only Start replaces a race.

mod benchmark;
pub(crate) mod car_preview;
#[cfg(test)]
mod proof;

use crate::{
    assets,
    camera::{ChaseCamera, Mode},
    controls::Controls,
    hud,
    options::Options,
    Session,
};
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Color32, RichText},
    EguiContexts,
};
use racecraft_sim::{Assists, Compound};
use serde::Deserialize;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Main,
    Loading,
    Racing,
    Paused,
    CameraEditor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Tab {
    Track,
    Car,
    Driver,
    Camera,
    Controls,
    Aids,
    Graphics,
    Effects,
    Audio,
    Hud,
    Benchmark,
}

impl Tab {
    const ALL: [Self; 11] = [
        Self::Track,
        Self::Car,
        Self::Driver,
        Self::Camera,
        Self::Controls,
        Self::Aids,
        Self::Graphics,
        Self::Effects,
        Self::Hud,
        Self::Audio,
        Self::Benchmark,
    ];

    fn group(self) -> &'static str {
        match self {
            Self::Track | Self::Car | Self::Driver => "GAMEPLAY",
            Self::Camera | Self::Controls | Self::Aids => "DRIVING",
            Self::Graphics | Self::Effects | Self::Hud => "VISUALS",
            Self::Audio => "AUDIO",
            Self::Benchmark => "TOOLS",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Track => "Track",
            Self::Car => "Car",
            Self::Driver => "Driver",
            Self::Camera => "Camera",
            Self::Controls => "Controls",
            Self::Aids => "Driving assistance",
            Self::Graphics => "Graphics",
            Self::Effects => "Effects",
            Self::Audio => "Sound",
            Self::Hud => "Interface",
            Self::Benchmark => "Benchmark",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Track => "Choose a circuit, race length and grid.",
            Self::Car => "Choose your car, tyres and wing levels.",
            Self::Driver => "Choose your team and driver.",
            Self::Camera => "Choose your driving view and adjust its framing.",
            Self::Controls => "See the keyboard and gamepad bindings.",
            Self::Aids => "Choose an assist preset or tune braking, steering and car control.",
            Self::Graphics => "Set image quality, detail and lighting.",
            Self::Effects => "Tune exposure, color, lens and motion effects.",
            Self::Audio => "Choose an engine sound and balance the race mix.",
            Self::Hud => "Customize dashboard and track map readouts.",
            Self::Benchmark => "Measure performance with your current setup.",
        }
    }
}

pub struct Circuit {
    pub id: String,
    pub name: String,
    pub outline: Vec<[f64; 3]>,
    pub unavailable: Option<String>,
}

pub struct CarChoice {
    pub id: String,
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Default)]
pub struct Catalog {
    pub circuits: Vec<Circuit>,
    pub cars: Vec<CarChoice>,
}

#[derive(Deserialize)]
struct TrackSummary {
    name: String,
    centerline: Vec<[f64; 3]>,
}

impl Catalog {
    pub fn discover(root: &Path) -> Self {
        let mut catalog = Self::default();
        for (directory, deferred) in [("tracks", false), ("deferred", true)] {
            if deferred && !cfg!(feature = "exp") {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(root.join(directory)) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let id = entry.file_name().to_string_lossy().into_owned();
                if !Options::track_enabled(&id) {
                    continue;
                }
                let summary = std::fs::read(path.join("track.json"))
                    .ok()
                    .and_then(|data| serde_json::from_slice::<TrackSummary>(&data).ok());
                let Some(summary) = summary else { continue };
                let unavailable = if deferred {
                    Some("Conversion deferred: the drivable surface still needs repair.".into())
                } else if !path.join("visual.glb").is_file()
                    || !path.join("collision.bin").is_file()
                    || summary.centerline.len() < 8
                {
                    Some("Missing converted scene, collision geometry or centerline.".into())
                } else {
                    None
                };
                catalog.circuits.push(Circuit {
                    id,
                    name: summary.name,
                    outline: summary.centerline,
                    unavailable,
                });
            }
        }
        catalog.circuits.sort_by(|a, b| {
            a.unavailable
                .is_some()
                .cmp(&b.unavailable.is_some())
                .then(a.name.cmp(&b.name))
        });
        for (id, name, description) in [
            ("rb6", "Red Bull RB6", "2010 / V8 / 7-speed / 18,000 RPM"),
            (
                "gen22",
                "2022 ground effect",
                "V6 hybrid / 8-speed / shared team model",
            ),
        ] {
            let car = root.join("cars").join(id);
            if car.join("car.glb").is_file()
                && car.join("car.json").is_file()
                && (id != "gen22" || crate::teams::Teams::load(root).is_ok())
            {
                catalog.cars.push(CarChoice {
                    id: id.into(),
                    name,
                    description,
                });
            }
        }
        catalog
    }

    fn car_available(&self, options: &Options) -> bool {
        self.cars.iter().any(|car| car.id == options.car)
            && (options.car != "gen22"
                || options.opponents == 0
                || options.assets.join("cars/gen22/car_lod1.glb").is_file())
    }
}

#[derive(Resource)]
pub struct Frontend {
    pub catalog: Catalog,
    pub teams: crate::teams::Teams,
    pub draft: Options,
    pub aids: Assists,
    pub corner_braking: f64,
    pub show_racing_line: bool,
    pub graphics: crate::graphics::Quality,
    pub video: crate::graphics::Video,
    pub vsync: bool,
    pub display: hud::Settings,
    pub audio: crate::audio::Settings,
    /// Session-only; M toggles it in a race.
    pub muted: bool,
    pub views: std::collections::BTreeMap<String, crate::camera::Tuning>,
    benchmark: benchmark::Benchmark,
    store: crate::preferences::Store,
    pub pending: bool,
    pub error: Option<String>,
    pub race_started: bool,
    pub(crate) tab: Tab,
    #[cfg(test)]
    pub widgets: std::collections::HashMap<&'static str, egui::Rect>,
}

impl Frontend {
    pub fn new(options: &Options) -> Self {
        let (store, prefs) = crate::preferences::Store::load(options);
        let mut draft = options.clone();
        draft.camera = prefs.camera;
        draft.weather = prefs.weather;
        let teams = crate::teams::Teams::load(&options.assets).unwrap_or_default();
        if !options.quick_race && options.frames.is_none() {
            if options.team.is_empty() {
                draft.team = prefs.team.clone();
            }
            if options.driver.is_empty() && (options.team.is_empty() || options.team == prefs.team)
            {
                draft.driver = prefs.driver.clone();
            }
        }
        if let Ok((team, driver)) = teams
            .selection(&draft.team, &draft.driver)
            .or_else(|_| teams.selection(&draft.team, ""))
            .or_else(|_| teams.selection("", ""))
        {
            draft.team = team.id.clone();
            draft.driver = driver.id.clone();
        }
        Self {
            catalog: Catalog::discover(&options.assets),
            teams,
            draft,
            aids: prefs.aids,
            corner_braking: prefs.corner_braking,
            show_racing_line: prefs.racing_line,
            graphics: prefs.graphics,
            video: video_override().unwrap_or_else(|| prefs.video()),
            vsync: prefs.vsync,
            display: prefs.display,
            audio: prefs.audio,
            muted: options.mute,
            views: prefs.views,
            store,
            benchmark: default(),
            pending: false,
            error: None,
            race_started: options.quick_race || options.frames.is_some(),
            tab: Tab::Track,
            #[cfg(test)]
            widgets: default(),
        }
    }
}

/// `F1_VIDEO` (a JSON `graphics::Video`) overrides the video settings,
/// including for capture and benchmark runs.
fn video_override() -> Option<crate::graphics::Video> {
    let json = std::env::var("F1_VIDEO").ok()?;
    match serde_json::from_str::<crate::graphics::Video>(&json) {
        Ok(mut video) => {
            video.sanitize();
            Some(video)
        }
        Err(error) => {
            warn!("ignoring F1_VIDEO: {error}");
            None
        }
    }
}

pub fn resume(session: &mut Session) {
    session.screen = Screen::Racing;
    session.paused = false;
    session.controls = Controls::default();
}

/// Commit the pending selection transactionally: CPU-load failures leave the old race intact.
#[allow(clippy::too_many_arguments)]
pub fn apply(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut frontend: ResMut<Frontend>,
    mut options: ResMut<Options>,
    mut session: ResMut<Session>,
    roots: Query<Entity, With<assets::RaceVisualRoot>>,
    mut lights: Query<&mut Transform, With<DirectionalLight>>,
    mut windows: Query<&mut Window>,
    cameras: Query<
        (Entity, Option<&bevy::camera::visibility::RenderLayers>),
        With<bevy_egui::PrimaryEguiContext>,
    >,
) {
    // The benchmark window owns the GPU workload; keep the menu available
    // without also rendering a second race behind it.
    let layers = if frontend.benchmark.child.is_some() {
        bevy::camera::visibility::RenderLayers::none()
    } else {
        bevy::camera::visibility::RenderLayers::default()
    };
    for (camera, current) in &cameras {
        if current != Some(&layers) {
            commands.entity(camera).insert(layers.clone());
        }
    }
    // Persist the actual active camera, not the next race's draft selection.
    if frontend.race_started {
        frontend
            .views
            .insert(options.car.clone(), session.camera.tuning.clone());
        if matches!(session.screen, Screen::Racing | Screen::CameraEditor) {
            frontend.draft.camera = session.camera.mode;
        }
    }
    let prefs = crate::preferences::Preferences {
        team: frontend.draft.team.clone(),
        weather: frontend.draft.weather,
        driver: frontend.draft.driver.clone(),
        aids: frontend.aids,
        corner_braking: frontend.corner_braking,
        racing_line: frontend.show_racing_line,
        camera: frontend.draft.camera,
        views: frontend.views.clone(),
        graphics: frontend.graphics,
        video: (frontend.video != frontend.graphics.video()).then_some(frontend.video),
        vsync: frontend.vsync,
        display: frontend.display,
        audio: frontend.audio,
    };
    if let Err(error) = frontend.store.save(prefs) {
        frontend.error = Some(error);
    }
    if !frontend.pending {
        return;
    }
    frontend.pending = false;
    let available = frontend
        .catalog
        .circuits
        .iter()
        .any(|track| track.id == frontend.draft.track && track.unavailable.is_none());
    if !available {
        frontend.error = Some("This circuit is not ready. Select an available circuit.".into());
        return;
    }
    if !frontend.catalog.car_available(&frontend.draft) {
        frontend.error = Some("This car is not available in the asset directory.".into());
        return;
    }
    let mut selected = frontend.draft.clone();
    selected.driver_aids = Some(frontend.aids);
    selected.automatic_gears = frontend.aids.automatic_gears;
    selected.corner_braking = frontend.corner_braking;
    selected.grid_slot = selected.grid_slot.clamp(1, selected.opponents + 1);
    let race = match assets::load_race(&selected) {
        Ok(race) => race,
        Err(error) => {
            frontend.error = Some(error);
            return;
        }
    };
    let player = selected.grid_slot - 1;
    let rig = match crate::teams::Rig::load(&selected) {
        Ok(rig) => rig,
        Err(error) => {
            frontend.error = Some(error);
            return;
        }
    };
    let identities = match frontend.teams.field(&selected) {
        Ok(field) => field,
        Err(error) => {
            frontend.error = Some(error);
            return;
        }
    };
    let mut camera = ChaseCamera::new(&race, player);
    camera.set_cockpit_eye(rig.cockpit_eye);
    camera.select(selected.camera);
    camera.configure(
        frontend
            .views
            .get(&selected.car)
            .cloned()
            .unwrap_or_else(|| crate::camera::Tuning::for_car(&selected.car)),
    );
    for root in &roots {
        commands.entity(root).despawn();
    }
    let sun = race.track.sun_direction;
    for mut light in &mut lights {
        *light = Transform::IDENTITY
            .looking_to(Vec3::new(sun.x as f32, sun.y as f32, sun.z as f32), Vec3::Y);
    }
    for mut window in &mut windows {
        window.title = format!("F1 Racecraft - {}", race.track.name);
    }
    *session = Session {
        identities,
        rig,
        guidance: crate::driving::Guidance::new(&race, player),
        corner_braking: frontend.corner_braking,
        show_racing_line: frontend.show_racing_line,
        show_performance: session.show_performance,
        minimap: hud::Minimap::new(&race.track),
        poses: crate::pose::Poses::new(&race),
        race,
        player,
        camera,
        controls: default(),
        paused: true,
        messages: default(),
        delta: default(),
        telemetry: default(),
        ready: false,
        screen: Screen::Loading,
    };
    *options = selected;
    frontend.error = None;
    frontend.race_started = false;
    assets::spawn_visuals(&mut commands, &server, &options, &session);
}

const RED: Color32 = Color32::from_rgb(230, 35, 48);
const DIM: Color32 = Color32::from_rgb(160, 166, 178);
const LABEL: Color32 = Color32::from_rgb(120, 125, 136);
const BACKDROP: Color32 = Color32::from_rgb(20, 21, 24);
const HOUSING: Color32 = Color32::from_rgb(37, 38, 42);
const HOUSING_EDGE: Color32 = Color32::from_rgb(74, 76, 83);
const WELL: Color32 = Color32::from_rgb(9, 10, 12);
const WELL_RIM: Color32 = Color32::from_rgb(56, 58, 64);

/// Raised charcoal panel matching the race HUD's instrument housings.
fn housing() -> egui::Frame {
    egui::Frame::new()
        .fill(HOUSING)
        .stroke(egui::Stroke::new(1.5, HOUSING_EDGE))
        .corner_radius(12)
        .inner_margin(18)
}

/// Spaced uppercase section label, printed like a dash caption.
fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(15.)
            .strong()
            .color(LABEL),
    );
}

/// Form rows keep labels, dropdowns, sliders, and value boxes on shared axes.
fn setting_row(ui: &mut egui::Ui, label: &str, add_control: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    let label_width = (width * 0.38).clamp(96., 190.);
    let row_height = if width < 440. { 40. } else { 32. };
    let response = ui.horizontal(|row| {
        row.spacing_mut().item_spacing.x = 8.;
        row.add_sized(
            egui::vec2(label_width, row_height),
            egui::Label::new(RichText::new(label).color(DIM)).wrap(),
        );
        // egui's slider_width sizes only the rail, excluding the editable value.
        let font = row.style().drag_value_text_style.resolve(row.style());
        let value_width = row.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap("-300.00 EV".into(), font, Color32::WHITE)
                .size()
                .x
        }) + 2. * row.spacing().button_padding.x;
        // Font metrics are empty on the first pass, before the atlas is ready.
        row.spacing_mut().slider_width =
            (row.available_width() - value_width.max(96.) - row.spacing().item_spacing.x).max(0.);
        add_control(row);
    });
    #[cfg(test)]
    assert!(
        response.response.rect.width() <= width + 0.5,
        "{label} row exceeds its {width}-point column: {:?}",
        response.response.rect
    );
    #[cfg(not(test))]
    let _ = response;
    ui.add_space(3.);
}

#[allow(clippy::too_many_arguments)]
pub fn ui(
    mut contexts: EguiContexts,
    mut frontend: ResMut<Frontend>,
    mut session: ResMut<Session>,
    mut options: ResMut<Options>,
    mut styled: Local<bool>,
    mut warmed: Local<u8>,
    car_preview: Res<car_preview::CarPreview>,
    mut exit: MessageWriter<AppExit>,
) -> Result {
    frontend.benchmark.poll();
    let context = contexts.ctx_mut()?;
    if !*styled {
        let mut style = (*context.style_of(egui::Theme::Dark)).clone();
        style.visuals = egui::Visuals::dark();
        let visuals = &mut style.visuals;
        visuals.panel_fill = BACKDROP;
        visuals.window_fill = HOUSING;
        visuals.window_stroke = egui::Stroke::new(1.5, HOUSING_EDGE);
        visuals.extreme_bg_color = WELL;
        visuals.faint_bg_color = HOUSING;
        visuals.selection.bg_fill = RED;
        visuals.selection.stroke = egui::Stroke::new(1., Color32::WHITE);
        visuals.slider_trailing_fill = true;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1., WELL_RIM);
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1., DIM);
        // Controls read as black inset windows; hover lifts the rim and
        // pressed or selected states light up in the race red.
        for (widget, rim) in [
            (&mut visuals.widgets.inactive, WELL_RIM),
            (
                &mut visuals.widgets.hovered,
                Color32::from_rgb(120, 124, 134),
            ),
            (&mut visuals.widgets.active, RED),
            (&mut visuals.widgets.open, HOUSING_EDGE),
        ] {
            widget.bg_fill = WELL;
            widget.weak_bg_fill = WELL;
            widget.bg_stroke = egui::Stroke::new(1., rim);
            widget.corner_radius = egui::CornerRadius::same(6);
            widget.expansion = 0.;
        }
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1., Color32::from_gray(225));
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(22, 23, 27);
        visuals.widgets.active.bg_fill = RED;
        visuals.widgets.active.weak_bg_fill = RED;
        style.spacing.item_spacing = egui::vec2(14., 8.);
        style.spacing.button_padding = egui::vec2(14., 6.);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(17.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(17.));
        context.set_theme(egui::Theme::Dark);
        context.set_style_of(egui::Theme::Dark, style);
        *styled = true;
    }
    // Warm both font sets: the HUD installs its definitions on the first pass.
    // Keeping the atlas stable avoids missing glyphs on a panel's first frame.
    if *warmed == 1 {
        warm_fonts(context);
    }
    if *warmed < 2 {
        *warmed += 1;
    }
    let mut viewport = egui::Ui::new(
        context.clone(),
        "menu-viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(context.viewport_rect()),
    );
    if session.screen == Screen::Racing {
        frontend.draft.camera = session.camera.mode;
        return Ok(());
    }
    if session.screen == Screen::CameraEditor {
        camera_editor(&viewport, &mut session, &options.car);
        return Ok(());
    }
    if session.screen == Screen::Loading {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BACKDROP.gamma_multiply(0.9)))
            .show(&mut viewport, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.38);
                    housing().inner_margin(28).show(ui, |ui| {
                        ui.spinner();
                        ui.heading("Loading circuit and cars");
                        ui.label(
                            RichText::new(
                                "Preparing the scene. Your race starts when every car is ready.",
                            )
                            .color(DIM),
                        );
                    });
                });
            });
        return Ok(());
    }
    let size = context.content_rect().size();
    let margin = (size.x * 0.018).clamp(10., 24.);
    let content_max_width = (size.x * 0.94).clamp(720., 1600.);
    let compact_footer = size.x < 720. || size.y < 640.;
    let footer_height = if size.y < 520. {
        56.
    } else if compact_footer {
        66.
    } else {
        82.
    };
    let available = frontend
        .catalog
        .circuits
        .iter()
        .any(|track| track.id == frontend.draft.track && track.unavailable.is_none())
        && frontend.catalog.car_available(&frontend.draft);

    // Every footer action shares one size.
    let action = egui::vec2(
        ((content_max_width.min(size.x) - margin * 2. - 24.) / 3.).clamp(120., 190.),
        44.,
    );
    egui::Panel::bottom("settings-actions")
        .exact_size(footer_height)
        .show_separator_line(false)
        .frame(
            egui::Frame::new()
                .fill(BACKDROP)
                .inner_margin(egui::Margin::symmetric(margin as i8, 6)),
        )
        .show(&mut viewport, |ui| {
            let width = ui.available_width().min(content_max_width);
            ui.vertical_centered(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 12.;
                            if frontend.race_started && session.ready {
                                let resume_button = ui.add_enabled(
                                    frontend.benchmark.child.is_none(),
                                    egui::Button::new(RichText::new("Resume race").strong())
                                        .fill(RED)
                                        .min_size(action),
                                );
                                if resume_button.clicked() {
                                    resume(&mut session);
                                }
                            }
                            let start = ui.add_enabled(
                                available
                                    && !frontend.pending
                                    && frontend.benchmark.child.is_none(),
                                egui::Button::new(
                                    RichText::new(if frontend.race_started {
                                        "Start new race"
                                    } else {
                                        "Start race"
                                    })
                                    .strong(),
                                )
                                .fill(if frontend.race_started { HOUSING } else { RED })
                                .min_size(action),
                            );
                            #[cfg(test)]
                            frontend.widgets.insert("start", start.rect);
                            if start.clicked() {
                                frontend.pending = true;
                            }
                            let quit = ui.add(
                                egui::Button::new(RichText::new("Quit game").strong())
                                    .fill(HOUSING)
                                    .min_size(action),
                            );
                            if quit.clicked() {
                                if let Some(child) = frontend.benchmark.child.as_mut() {
                                    let _ = child.kill();
                                    let _ = child.wait();
                                }
                                exit.write(AppExit::Success);
                            }
                        });
                    },
                );
            });
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(BACKDROP).inner_margin(margin as i8))
        .show(&mut viewport, |ui| {
            let width = ui.available_width().min(content_max_width);
            ui.vertical_centered(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.label(
                            RichText::new(if session.screen == Screen::Paused {
                                "RACE PAUSED"
                            } else {
                                "F1 RACECRAFT"
                            })
                            .size(if size.x < 700. { 28. } else { 36. })
                            .strong(),
                        );
                        if let Some(error) = &frontend.error {
                            ui.colored_label(Color32::from_rgb(255, 150, 150), error);
                        }
                        ui.add_space(8.);
                        let pane_height = (ui.available_height() - 22.).max(100.);
                        let body = housing().inner_margin(10).show(ui, |ui| {
                            let compact_navigation =
                                ui.available_width() < 980. || pane_height < 560.;
                            if compact_navigation {
                                #[cfg(test)]
                                {
                                    frontend.widgets.remove("settings-navigation-popup");
                                    frontend.widgets.remove("Game play");
                                }
                                #[cfg(test)]
                                for tab in Tab::ALL {
                                    frontend.widgets.remove(tab.label());
                                }
                                let navigation = egui::ComboBox::from_id_salt("settings-page")
                                    .width(ui.available_width())
                                    .height(400.)
                                    .selected_text(frontend.tab.label())
                                    .show_ui(ui, |ui| {
                                        #[cfg(test)]
                                        frontend
                                            .widgets
                                            .insert("settings-navigation-popup", ui.clip_rect());
                                        for tab in Tab::ALL {
                                            let selected = tab == frontend.tab;
                                            let response =
                                                ui.selectable_label(selected, tab.label());
                                            if response.clicked() && !selected {
                                                frontend.tab = tab;
                                            }
                                            #[cfg(test)]
                                            frontend.widgets.insert(tab.label(), response.rect);
                                            #[cfg(not(test))]
                                            let _ = response;
                                        }
                                    });
                                #[cfg(test)]
                                frontend
                                    .widgets
                                    .insert("settings-navigation", navigation.response.rect);
                                #[cfg(not(test))]
                                let _ = navigation;
                                ui.separator();
                                if ui.available_height() >= 500. {
                                    ui.label(RichText::new(frontend.tab.description()).color(DIM));
                                    ui.add_space(4.);
                                }
                                let page_height = (ui.available_height() - 4.).max(20.);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), page_height),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        render_settings_page(
                                            ui,
                                            &mut frontend,
                                            &mut session,
                                            size,
                                            context.pixels_per_point(),
                                            &car_preview,
                                        );
                                    },
                                );
                            } else {
                                let nav_width = (ui.available_width() * 0.15).clamp(176., 206.);
                                let nav_button_height =
                                    ((pane_height - 180.) / Tab::ALL.len() as f32).clamp(24., 38.);
                                ui.horizontal_top(|ui| {
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(nav_width, pane_height),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.spacing_mut().item_spacing.y = 3.;
                                            let mut group = "";
                                            for tab in Tab::ALL {
                                                if tab.group() != group {
                                                    if !group.is_empty() {
                                                        ui.add_space(8.);
                                                    }
                                                    group = tab.group();
                                                    section(ui, group);
                                                    ui.add_space(3.);
                                                }
                                                let selected = tab == frontend.tab;
                                                let label = tab.label();
                                                let response = ui.add_sized(
                                                    egui::vec2(nav_width, nav_button_height),
                                                    egui::Button::new(
                                                        RichText::new(label).strong().color(
                                                            if selected {
                                                                Color32::WHITE
                                                            } else {
                                                                DIM
                                                            },
                                                        ),
                                                    )
                                                    .fill(if selected {
                                                        RED
                                                    } else {
                                                        Color32::TRANSPARENT
                                                    }),
                                                );
                                                if response.clicked() && !selected {
                                                    frontend.tab = tab;
                                                }
                                                #[cfg(test)]
                                                frontend.widgets.insert(label, response.rect);
                                            }
                                        },
                                    );
                                    ui.separator();
                                    ui.add_space(6.);
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 4.;
                                        {
                                            ui.heading(
                                                RichText::new(frontend.tab.label())
                                                    .size(if size.x < 700. { 22. } else { 28. })
                                                    .strong(),
                                            );
                                            ui.label(
                                                RichText::new(frontend.tab.description())
                                                    .color(DIM),
                                            );
                                            ui.add_space(5.);
                                            ui.separator();
                                        }
                                        let page_height = (ui.available_height() - 4.).max(20.);
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(ui.available_width(), page_height),
                                            egui::Layout::top_down(egui::Align::Min),
                                            |ui| {
                                                render_settings_page(
                                                    ui,
                                                    &mut frontend,
                                                    &mut session,
                                                    size,
                                                    context.pixels_per_point(),
                                                    &car_preview,
                                                );
                                            },
                                        );
                                    });
                                });
                            }
                        });
                        #[cfg(test)]
                        frontend.widgets.insert("settings-body", body.response.rect);
                        #[cfg(not(test))]
                        let _ = body;
                    },
                );
            });
        });
    if frontend.race_started {
        let player = session.player;
        session.race.entrants[player].car.assists = frontend.aids;
        options.driver_aids = Some(frontend.aids);
        options.automatic_gears = frontend.aids.automatic_gears;
        options.corner_braking = frontend.corner_braking;
        session.corner_braking = frontend.corner_braking;
        session.show_racing_line = frontend.show_racing_line;
    }
    Ok(())
}

fn render_settings_page(
    ui: &mut egui::Ui,
    frontend: &mut Frontend,
    session: &mut Session,
    size: egui::Vec2,
    pixels_per_point: f32,
    car_preview: &car_preview::CarPreview,
) {
    let content = egui::ScrollArea::vertical()
        .id_salt(("settings-content", frontend.tab))
        .auto_shrink([false, false])
        .show(ui, |ui| match frontend.tab {
            Tab::Track => page(ui, |ui, part| match part {
                Part::Settings => selection(ui, &frontend.catalog, &mut frontend.draft),
                Part::Preview => track_preview(ui, &frontend.catalog, &frontend.draft),
            }),
            Tab::Car => page(ui, |ui, part| match part {
                Part::Settings => car_choice(ui, frontend),
                Part::Preview => car_preview.show(ui),
            }),
            Tab::Driver => page(ui, |ui, part| match part {
                Part::Settings => driver_choice(ui, frontend),
                Part::Preview => driver_card(ui, frontend),
            }),
            Tab::Aids => {
                let presets = aids_panel(ui, frontend);
                #[cfg(test)]
                for (name, rect) in ["beginner", "balanced", "unassisted"]
                    .into_iter()
                    .zip(presets)
                {
                    frontend.widgets.insert(name, rect);
                }
                #[cfg(not(test))]
                let _ = presets;
            }
            Tab::Camera => camera_panel(ui, &mut frontend.draft.camera, session),
            Tab::Controls => controls_panel(ui),
            Tab::Graphics => graphics_panel(ui, frontend, session),
            Tab::Effects => effects_panel(ui, frontend),
            Tab::Audio => audio_panel(ui, frontend),
            Tab::Hud => display_panel(ui, frontend),
            Tab::Benchmark => benchmark::panel(ui, frontend, size * pixels_per_point),
        });
    #[cfg(test)]
    frontend
        .widgets
        .insert("settings-content", content.inner_rect);
    #[cfg(not(test))]
    let _ = content;
}

/// Which half of a page is being drawn.
#[derive(Clone, Copy)]
enum Part {
    Settings,
    Preview,
}

/// Settings on the left and the page's own preview on the right, stacked when
/// the window is narrow.
fn page(ui: &mut egui::Ui, mut draw: impl FnMut(&mut egui::Ui, Part)) {
    if ui.available_width() > 980. {
        ui.columns(2, |columns| {
            draw(&mut columns[0], Part::Settings);
            draw(&mut columns[1], Part::Preview);
        });
    } else {
        draw(ui, Part::Settings);
        ui.add_space(12.);
        draw(ui, Part::Preview);
    }
}

fn selection(ui: &mut egui::Ui, catalog: &Catalog, draft: &mut Options) {
    section(ui, "Circuit");
    let selected = catalog
        .circuits
        .iter()
        .find(|track| track.id == draft.track)
        .map_or("Select a circuit", |track| track.name.as_str());
    egui::ComboBox::from_id_salt("circuit")
        .width(ui.available_width())
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for track in &catalog.circuits {
                ui.add_enabled_ui(track.unavailable.is_none(), |ui| {
                    ui.selectable_value(&mut draft.track, track.id.clone(), &track.name);
                });
                if let Some(reason) = &track.unavailable {
                    ui.small(reason);
                }
            }
        });
    ui.separator();
    section(ui, "Weather");
    ui.horizontal(|ui| {
        for weather in crate::sky::Weather::ALL {
            ui.selectable_value(&mut draft.weather, weather, weather.label());
        }
    });
    ui.separator();
    section(ui, "Race distance & grid");
    if ui.available_width() >= 720. {
        ui.columns(2, |columns| {
            setting_row(&mut columns[0], "Laps", |ui| {
                ui.add_sized(
                    egui::vec2(ui.available_width(), 28.),
                    egui::Slider::new(&mut draft.laps, 1..=30),
                );
            });
            setting_row(&mut columns[1], "AI opponents", |ui| {
                ui.add_sized(
                    egui::vec2(ui.available_width(), 28.),
                    egui::Slider::new(&mut draft.opponents, 0..=crate::options::MAX_OPPONENTS),
                );
            });
        });
    } else {
        setting_row(ui, "Laps", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut draft.laps, 1..=30),
            );
        });
        setting_row(ui, "AI opponents", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut draft.opponents, 0..=crate::options::MAX_OPPONENTS),
            );
        });
    }
    draft.grid_slot = draft.grid_slot.clamp(1, draft.opponents + 1);
    setting_row(ui, "Grid position", |ui| {
        ui.add_sized(
            egui::vec2(ui.available_width(), 28.),
            egui::Slider::new(&mut draft.grid_slot, 1..=draft.opponents + 1),
        );
    });
}

/// Car model, tyres and wings.
fn car_choice(ui: &mut egui::Ui, frontend: &mut Frontend) {
    let draft = &mut frontend.draft;
    let catalog = &frontend.catalog;
    section(ui, "Car");
    ui.horizontal_wrapped(|ui| {
        for car in &catalog.cars {
            let response = ui
                .radio_value(
                    &mut draft.car,
                    car.id.clone(),
                    if car.id == "gen22" {
                        "2022 car"
                    } else {
                        car.name
                    },
                )
                .on_hover_text(car.description);
            #[cfg(test)]
            match car.id.as_str() {
                "gen22" => {
                    frontend.widgets.insert("car-gen22", response.rect);
                }
                "rb6" => {
                    frontend.widgets.insert("car-rb6", response.rect);
                }
                _ => {}
            }
            #[cfg(not(test))]
            let _ = response;
        }
    });
    ui.add_space(12.);
    section(ui, "Tyres & aero");
    setting_row(ui, "Tyre compound", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (compound, label) in [
                (Compound::Soft, "Soft"),
                (Compound::Medium, "Medium"),
                (Compound::Hard, "Hard"),
            ] {
                ui.selectable_value(&mut draft.setup.compound, compound, label);
            }
        });
    });
    if ui.available_width() >= 720. {
        ui.columns(2, |columns| {
            setting_row(&mut columns[0], "Front wing", |ui| {
                ui.add_sized(
                    egui::vec2(ui.available_width(), 28.),
                    egui::Slider::new(&mut draft.setup.front_wing, 0.0..=10.0).step_by(1.),
                );
            });
            setting_row(&mut columns[1], "Rear wing", |ui| {
                ui.add_sized(
                    egui::vec2(ui.available_width(), 28.),
                    egui::Slider::new(&mut draft.setup.rear_wing, 0.0..=10.0).step_by(1.),
                );
            });
        });
    } else {
        setting_row(ui, "Front wing", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut draft.setup.front_wing, 0.0..=10.0).step_by(1.),
            );
        });
        setting_row(ui, "Rear wing", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut draft.setup.rear_wing, 0.0..=10.0).step_by(1.),
            );
        });
    }
}

/// Team and driver choices use the roster names, without color swatches.
fn driver_choice(ui: &mut egui::Ui, frontend: &mut Frontend) {
    let draft = &mut frontend.draft;
    let teams = &frontend.teams;
    if draft.car != "gen22" {
        ui.heading("Red Bull Racing");
        ui.label(
            "The 2010 RB6 races in its own livery. Pick the 2022 car under Car to choose a team.",
        );
        return;
    }
    section(ui, "Team");
    ui.columns(2, |columns| {
        for (index, team) in teams.teams.iter().enumerate() {
            let column = &mut columns[index % 2];
            let response = column.add_sized(
                egui::vec2(column.available_width(), 36.),
                egui::Button::new(&team.name)
                    .wrap()
                    .selected(draft.team == team.id),
            );
            #[cfg(test)]
            if team.id == "ferrari" {
                frontend.widgets.insert("team-ferrari", response.rect);
            }
            if response.clicked() {
                select_team(draft, team);
            }
        }
    });
    ui.add_space(16.);
    section(ui, "Driver");
    if let Some(team) = teams.teams.iter().find(|team| team.id == draft.team) {
        ui.horizontal_wrapped(|ui| {
            for driver in &team.drivers {
                ui.selectable_value(
                    &mut draft.driver,
                    driver.id.clone(),
                    format!("#{}  {}", driver.number, driver.name),
                );
            }
        });
    } else {
        ui.label("No team available. Check the car assets.");
    }
}

/// The selected driver as a team-coloured card: number, code, name and team.
fn driver_card(ui: &mut egui::Ui, frontend: &Frontend) {
    let width = ui.available_width();
    let height = (width * 0.5).clamp(170., 300.);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter();
    let selected = (frontend.draft.car == "gen22")
        .then(|| {
            frontend
                .teams
                .selection(&frontend.draft.team, &frontend.draft.driver)
                .ok()
        })
        .flatten();
    let Some((team, driver)) = selected else {
        painter.rect_filled(rect, 10., WELL);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Red Bull RB6",
            egui::FontId::proportional(26.),
            DIM,
        );
        return;
    };
    let colour =
        |hex: &str| crate::teams::rgb(hex).map_or(HOUSING, |[r, g, b]| Color32::from_rgb(r, g, b));
    let (primary, secondary, accent) = (
        colour(&team.colors.primary),
        colour(&team.colors.secondary),
        colour(&team.colors.accent),
    );
    painter.rect_filled(rect, 10., WELL);
    // Team colour panel with a diagonal stripe in the secondary and accent.
    let panel = rect.shrink(1.);
    painter.rect_filled(panel, 10., primary.gamma_multiply(0.9));
    let stripe = |offset: f32, thickness: f32, colour: Color32| {
        let x = panel.right() - panel.width() * offset;
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(x, panel.top()),
                egui::pos2(x + thickness, panel.top()),
                egui::pos2(x + thickness - panel.height() * 0.45, panel.bottom()),
                egui::pos2(x - panel.height() * 0.45, panel.bottom()),
            ],
            colour,
            egui::Stroke::NONE,
        ));
    };
    stripe(0.34, panel.width() * 0.07, secondary);
    stripe(0.24, panel.width() * 0.025, accent);
    // Legibility scrim behind the text, only where the team colour is light.
    let luminance = 0.2126 * f32::from(primary.r())
        + 0.7152 * f32::from(primary.g())
        + 0.0722 * f32::from(primary.b());
    if luminance > 140. {
        painter.rect_filled(
            egui::Rect::from_min_max(
                panel.left_top(),
                egui::pos2(panel.center().x + 40., panel.bottom()),
            ),
            10.,
            Color32::from_black_alpha(130),
        );
    }
    let pad = 22.;
    painter.text(
        panel.left_top() + egui::vec2(pad, pad - 6.),
        egui::Align2::LEFT_TOP,
        driver.number.to_string(),
        egui::FontId::proportional(height * 0.36),
        Color32::WHITE,
    );
    painter.text(
        panel.left_bottom() + egui::vec2(pad, -pad - 52.),
        egui::Align2::LEFT_BOTTOM,
        &driver.abbr,
        egui::FontId::proportional(20.),
        accent,
    );
    painter.text(
        panel.left_bottom() + egui::vec2(pad, -pad - 22.),
        egui::Align2::LEFT_BOTTOM,
        &driver.name,
        egui::FontId::proportional(26.),
        Color32::WHITE,
    );
    painter.text(
        panel.left_bottom() + egui::vec2(pad, -pad),
        egui::Align2::LEFT_BOTTOM,
        team.name.to_uppercase(),
        egui::FontId::proportional(14.),
        Color32::from_gray(215),
    );
}

fn select_team(draft: &mut Options, team: &crate::teams::Team) {
    if draft.team != team.id {
        draft.team.clone_from(&team.id);
        draft.driver.clone_from(&team.drivers[0].id);
    }
}

fn track_preview(ui: &mut egui::Ui, catalog: &Catalog, draft: &Options) {
    let Some(track) = catalog
        .circuits
        .iter()
        .find(|track| track.id == draft.track)
    else {
        ui.label("No converted circuits found in the asset directory.");
        return;
    };
    let width = ui.available_width();
    let height = (width * 0.62).clamp(160., 420.);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    ui.painter().rect_filled(rect, 6., WELL);
    ui.painter().rect_stroke(
        rect,
        6.,
        egui::Stroke::new(1., WELL_RIM),
        egui::StrokeKind::Inside,
    );
    if !track.outline.is_empty() {
        let (mut min, mut max) = (
            egui::pos2(f32::MAX, f32::MAX),
            egui::pos2(f32::MIN, f32::MIN),
        );
        for point in &track.outline {
            min.x = min.x.min(point[0] as f32);
            min.y = min.y.min(point[2] as f32);
            max.x = max.x.max(point[0] as f32);
            max.y = max.y.max(point[2] as f32);
        }
        let bounds = max - min;
        let scale =
            ((rect.width() - 40.) / bounds.x.max(1.)).min((rect.height() - 40.) / bounds.y.max(1.));
        let points: Vec<_> = track
            .outline
            .iter()
            .step_by(3)
            .map(|p| {
                rect.center()
                    + egui::vec2(
                        (p[0] as f32 - (min.x + max.x) * 0.5) * scale,
                        (p[2] as f32 - (min.y + max.y) * 0.5) * scale,
                    )
            })
            .collect();
        if let Some(start) = points.first() {
            ui.painter().circle_filled(*start, 5., RED);
        }
        ui.painter().add(egui::Shape::closed_line(
            points,
            egui::Stroke::new(3., Color32::from_rgb(217, 225, 238)),
        ));
    }
}

fn aids_panel(ui: &mut egui::Ui, frontend: &mut Frontend) -> [egui::Rect; 3] {
    section(ui, "Assists");
    ui.label("Presets are a starting point. Fine-tune each assist below.");
    let aids = &mut frontend.aids;
    let presets = ui
        .horizontal_wrapped(|ui| {
            let beginner = ui.button("Beginner");
            if beginner.clicked() {
                frontend.corner_braking = 1.;
                frontend.show_racing_line = true;
                *aids = Assists {
                    steering: 1.,
                    braking: 1.,
                    traction: 1.,
                    stability: 1.,
                    automatic_gears: true,
                    // A mode, not an aid: presets leave it alone.
                    drs_anywhere: aids.drs_anywhere,
                };
            }
            let balanced = ui.button("Balanced");
            if balanced.clicked() {
                frontend.corner_braking = 0.6;
                frontend.show_racing_line = true;
                *aids = Assists {
                    steering: 0.6,
                    braking: 0.6,
                    traction: 0.6,
                    stability: 0.5,
                    automatic_gears: true,
                    // A mode, not an aid: presets leave it alone.
                    drs_anywhere: aids.drs_anywhere,
                };
            }
            let unassisted = ui.button("Unassisted");
            if unassisted.clicked() {
                frontend.corner_braking = 0.;
                frontend.show_racing_line = false;
                *aids = Assists {
                    steering: 0.,
                    braking: 0.,
                    traction: 0.,
                    stability: 0.,
                    automatic_gears: false,
                    // A mode, not an aid: presets leave it alone.
                    drs_anywhere: aids.drs_anywhere,
                };
            }
            [beginner.rect, balanced.rect, unassisted.rect]
        })
        .inner;
    ui.separator();
    section(ui, "Driving options");
    setting_row(ui, "Automatic gears", |ui| {
        ui.checkbox(&mut aids.automatic_gears, "Enabled")
            .on_hover_text("Manual shifts: E / Shift up, Q / Ctrl down.");
    });
    setting_row(ui, "Racing line", |ui| {
        ui.checkbox(&mut frontend.show_racing_line, "Enabled");
    });
    setting_row(ui, "DRS anywhere", |ui| {
        ui.checkbox(&mut aids.drs_anywhere, "Enabled")
            .on_hover_text(
                "Open DRS with F (gamepad B) anywhere on track. Braking still closes it. \
             AI cars keep to the race rules.",
            );
    });
    ui.add_space(6.);
    if ui.available_width() >= 760. {
        ui.columns(2, |columns| {
            section(&mut columns[0], "Driving guidance");
            corner_braking_slider(&mut columns[0], &mut frontend.corner_braking);
            car_control_sliders(&mut columns[1], aids);
        });
    } else {
        section(ui, "Driving guidance");
        corner_braking_slider(ui, &mut frontend.corner_braking);
        ui.add_space(8.);
        car_control_sliders(ui, aids);
    }
    presets
}

fn corner_braking_slider(ui: &mut egui::Ui, value: &mut f64) {
    setting_row(ui, "Corner braking", |ui| {
        ui.add_sized(
            egui::vec2(ui.available_width(), 28.),
            egui::Slider::new(value, 0.0..=1.0)
                .custom_formatter(|value, _| format!("{:.0}%", value * 100.)),
        )
        .on_hover_text("Brakes for bends and reduces a held accelerator.");
    });
}

fn car_control_sliders(ui: &mut egui::Ui, aids: &mut Assists) {
    section(ui, "Car control");
    for (value, label, help) in [
        (
            &mut aids.steering,
            "Steering assistance",
            "Guides towards the racing line. Strong manual steering takes priority.",
        ),
        (
            &mut aids.braking,
            "Anti-lock braking",
            "Helps prevent wheel lock-ups while braking.",
        ),
        (
            &mut aids.traction,
            "Traction control",
            "Reduces wheelspin when applying power.",
        ),
        (
            &mut aids.stability,
            "Stability assistance",
            "Helps recover a developing slide.",
        ),
    ] {
        setting_row(ui, label, |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(value, 0.0..=1.0)
                    .step_by(0.05)
                    .custom_formatter(|value, _| format!("{:.0}%", value * 100.)),
            )
            .on_hover_text(help);
        });
    }
}

fn camera_editor(viewport: &egui::Ui, session: &mut Session, car: &str) {
    egui::Area::new("camera-editor".into())
        .anchor(egui::Align2::RIGHT_CENTER, egui::vec2(-18., 0.))
        .show(viewport, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("camera-editor-scroll")
                .max_height((viewport.max_rect().height() - 48.).max(80.))
                .show(ui, |ui| {
                    housing().fill(HOUSING.gamma_multiply(0.96)).show(ui, |ui| {
                        ui.set_width((viewport.max_rect().width() - 76.).clamp(180., 330.));
                        section(ui, "Camera position");
                        ui.label(RichText::new("Live preview / saved for this car").color(DIM));
                        ui.horizontal(|ui| {
                            for mode in Mode::ALL {
                                if ui
                                    .selectable_label(session.camera.mode == mode, mode.label())
                                    .clicked()
                                {
                                    session.camera.select(mode);
                                }
                            }
                        });
                        let mut tuning = session.camera.tuning.clone();
                        let view = tuning.view_mut(session.camera.mode);
                        setting_row(ui, "Height", |ui| {
                            ui.add_sized(
                                egui::vec2(ui.available_width(), 28.),
                                egui::Slider::new(&mut view.height, 0.65..=4.),
                            );
                        });
                        let range = if session.camera.mode == Mode::Chase {
                            -10.0..=-3.5
                        } else {
                            -1.5..=0.7
                        };
                        setting_row(ui, "Forward / back", |ui| {
                            ui.add_sized(
                                egui::vec2(ui.available_width(), 28.),
                                egui::Slider::new(&mut view.fore_aft, range),
                            );
                        });
                        setting_row(ui, "Look down", |ui| {
                            ui.add_sized(
                                egui::vec2(ui.available_width(), 28.),
                                egui::Slider::new(&mut view.pitch, -10.0..=20.),
                            );
                        });
                        setting_row(ui, "Field of view", |ui| {
                            ui.add_sized(
                                egui::vec2(ui.available_width(), 28.),
                                egui::Slider::new(&mut view.fov, 40.0..=95.),
                            );
                        });
                        ui.separator();
                        setting_row(ui, "Vibration", |ui| {
                            ui.add_sized(
                                egui::vec2(ui.available_width(), 28.),
                                egui::Slider::new(&mut tuning.shake, 0.0..=1.),
                            );
                        });
                        setting_row(ui, "Stable horizon", |ui| {
                            ui.add_sized(
                                egui::vec2(ui.available_width(), 28.),
                                egui::Slider::new(&mut tuning.horizon, 0.0..=1.),
                            );
                        });
                        if ui.button("Reset this view").clicked() {
                            *tuning.view_mut(session.camera.mode) =
                                crate::camera::Tuning::for_car(car).view(session.camera.mode);
                        }
                        session.camera.configure(tuning);
                        ui.separator();
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Done / resume  F4 or Esc").strong(),
                                )
                                .fill(RED)
                                .min_size(egui::vec2(ui.available_width(), 36.)),
                            )
                            .clicked()
                        {
                            resume(session);
                        }
                    });
                });
        });
}

fn warm_fonts(context: &egui::Context) {
    let glyphs: String = (32u8..=126).map(char::from).chain("°·—ü".chars()).collect();
    let mut fonts: Vec<_> = context
        .style_of(egui::Theme::Dark)
        .text_styles
        .values()
        .cloned()
        .collect();
    fonts.extend([15., 36.].map(egui::FontId::proportional));
    let scale = hud::scale(context.viewport_rect().size());
    for family in [
        egui::FontFamily::Name("regular".into()),
        egui::FontFamily::Name("bold".into()),
    ] {
        for size in [
            10., 11., 12., 13., 14., 15., 16., 18., 20., 22., 24., 26., 28., 30., 32., 36., 40.,
            43., 44.,
        ] {
            fonts.push(egui::FontId::new(size * scale, family.clone()));
        }
    }
    context.fonts_mut(|atlas| {
        for font in fonts {
            atlas.layout_no_wrap(glyphs.clone(), font, Color32::WHITE);
        }
    });
}

fn video_choice<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    value: &mut T,
    all: &[T],
    name: fn(T) -> &'static str,
) {
    setting_row(ui, label, |ui| {
        egui::ComboBox::from_id_salt(id)
            .width(ui.available_width().min(240.))
            .selected_text(name(*value))
            .show_ui(ui, |ui| {
                for &option in all {
                    ui.selectable_value(value, option, name(option));
                }
            });
    });
}

fn graphics_panel(ui: &mut egui::Ui, frontend: &mut Frontend, session: &mut Session) {
    use crate::graphics::{AntiAlias, Level, ShadowFilter, TextureDetail};
    section(ui, "Quality preset");
    ui.horizontal_wrapped(|ui| {
        for quality in crate::graphics::Quality::ALL {
            let active = frontend.video.preset() == Some(quality);
            if ui
                .selectable_label(active, quality.label())
                .on_hover_text(quality.description())
                .clicked()
            {
                frontend.graphics = quality;
                frontend.video = quality.video();
            }
        }
        let custom = frontend.video.preset().is_none();
        ui.add_enabled(false, egui::Button::new("Custom").selected(custom));
    });
    setting_row(ui, "Vertical sync", |ui| {
        ui.checkbox(&mut frontend.vsync, "Enabled")
            .on_hover_text("Off renders as fast as the GPU allows, with possible tearing.");
    });
    setting_row(ui, "Frame rate counter", |ui| {
        ui.checkbox(&mut session.show_performance, "Show  ·  F3");
    });
    ui.add_space(6.);

    let video = &mut frontend.video;
    let column_count = if ui.available_width() >= 640. { 2 } else { 1 };
    ui.columns(column_count, |columns| {
        let quality = &mut columns[0];
        section(quality, "Image quality");
        video_choice(
            quality,
            "aa",
            "Anti-aliasing",
            &mut video.anti_alias,
            &AntiAlias::ALL,
            AntiAlias::label,
        );
        if video.anti_alias == AntiAlias::Msaa4 {
            quality.label(
                RichText::new("MSAA disables ambient occlusion.")
                    .size(15.)
                    .color(DIM),
            );
        }
        setting_row(quality, "Sharpening", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.sharpening, 0.0..=1.0),
            );
        });
        video_choice(
            quality,
            "textures",
            "Texture detail",
            &mut video.texture,
            &TextureDetail::ALL,
            TextureDetail::label,
        );
        video_choice(
            quality,
            "ai_detail",
            "AI detail",
            &mut video.ai_detail,
            &Level::ALL,
            Level::label,
        );

        let lighting = &mut columns[column_count - 1];
        if column_count == 1 {
            lighting.add_space(8.);
        }
        section(lighting, "Shadows & lighting");
        video_choice(
            lighting,
            "shadow_res",
            "Shadow resolution",
            &mut video.shadow_resolution,
            &[1024, 2048, 4096, 8192],
            |size| match size {
                1024 => "1024",
                2048 => "2048",
                4096 => "4096",
                _ => "8192",
            },
        );
        setting_row(lighting, "Shadow cascades", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.shadow_cascades, 1..=4),
            );
        });
        setting_row(lighting, "Shadow distance", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.shadow_distance, 30.0..=300.0).suffix(" m"),
            );
        });
        video_choice(
            lighting,
            "shadow_filter",
            "Shadow filtering",
            &mut video.shadow_filter,
            &ShadowFilter::ALL,
            ShadowFilter::label,
        );
        video_choice(
            lighting,
            "ao",
            "Ambient occlusion",
            &mut video.ambient_occlusion,
            &Level::ALL,
            Level::label,
        );
        setting_row(lighting, "Contact shadows", |ui| {
            ui.add_enabled(
                video.ambient_occlusion != Level::Off,
                egui::Checkbox::new(&mut video.contact_shadows, "Enabled"),
            );
        });
        video_choice(
            lighting,
            "sky",
            "Sky lighting",
            &mut video.sky,
            &Level::ALL[1..],
            Level::label,
        );
        setting_row(lighting, "Haze", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.haze, 0.0..=1.0),
            );
        });
    });
    if let Some(preset) = frontend.video.preset() {
        frontend.graphics = preset;
    }
}

fn effects_panel(ui: &mut egui::Ui, frontend: &mut Frontend) {
    use crate::graphics::ToneMap;
    let reset = ui
        .horizontal_wrapped(|ui| {
            ui.label("Changes apply immediately.");
            ui.button("Reset image effects").clicked()
        })
        .inner;
    let video = &mut frontend.video;
    if reset {
        reset_image_effects(video);
    }
    let column_count = if ui.available_width() >= 640. { 2 } else { 1 };
    ui.columns(column_count, |columns| {
        let color = &mut columns[0];
        section(color, "Color & tone");
        video_choice(
            color,
            "tonemap",
            "Tonemapping",
            &mut video.tonemapping,
            &ToneMap::ALL,
            ToneMap::label,
        );
        setting_row(color, "Exposure", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.exposure, -3.0..=3.0).suffix(" EV"),
            );
        });
        setting_row(color, "Saturation", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.saturation, 0.0..=2.0),
            );
        });
        setting_row(color, "Contrast", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.contrast, 0.5..=1.5),
            );
        });
        setting_row(color, "Temperature", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.temperature, -1.0..=1.0),
            );
        });

        let lens = &mut columns[column_count - 1];
        if column_count == 1 {
            lens.add_space(8.);
        }
        section(lens, "Lens & motion");
        setting_row(lens, "Bloom", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.bloom, 0.0..=1.0),
            );
        });
        setting_row(lens, "Vignette", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.vignette, 0.0..=1.0),
            );
        });
        setting_row(lens, "Chromatic aberration", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.chromatic_aberration, 0.0..=1.0),
            );
        });
        setting_row(lens, "Motion blur", |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 28.),
                egui::Slider::new(&mut video.motion_blur, 0.0..=1.0),
            );
        });
        setting_row(lens, "Depth of field", |ui| {
            ui.checkbox(&mut video.depth_of_field, "Enabled");
        });
        setting_row(lens, "Bokeh", |ui| {
            ui.add_enabled_ui(video.depth_of_field, |ui| {
                ui.checkbox(&mut video.bokeh, "Enabled  ·  slower");
            });
        });
        setting_row(lens, "Focus distance", |ui| {
            ui.add_enabled_ui(video.depth_of_field, |ui| {
                ui.add_sized(
                    egui::vec2(ui.available_width(), 28.),
                    egui::Slider::new(&mut video.focus_distance, 1.0..=200.0)
                        .logarithmic(true)
                        .suffix(" m"),
                );
            });
        });
    });
    if let Some(preset) = frontend.video.preset() {
        frontend.graphics = preset;
    }
}

fn reset_image_effects(video: &mut crate::graphics::Video) {
    let base = crate::graphics::Video::default();
    video.tonemapping = base.tonemapping;
    video.exposure = base.exposure;
    video.saturation = base.saturation;
    video.contrast = base.contrast;
    video.temperature = base.temperature;
    video.bloom = base.bloom;
    video.vignette = base.vignette;
    video.chromatic_aberration = base.chromatic_aberration;
    video.motion_blur = base.motion_blur;
    video.depth_of_field = base.depth_of_field;
    video.bokeh = base.bokeh;
    video.focus_distance = base.focus_distance;
    video.haze = base.haze;
}
fn audio_panel(ui: &mut egui::Ui, frontend: &mut Frontend) {
    ui.label("Changes apply immediately and are saved for your next race.");
    ui.add_space(12.);
    let column_count = if ui.available_width() >= 560. { 2 } else { 1 };
    ui.columns(column_count, |columns| {
        let ui = &mut columns[0];
        section(ui, "Engine sound");
        ui.add_space(6.);
        for choice in crate::audio::EngineChoice::ALL {
            let _response = ui.radio_value(&mut frontend.audio.engine, choice, choice.label());
            #[cfg(test)]
            frontend.widgets.insert(choice.label(), _response.rect);
            _response.on_hover_text(choice.description());
        }
        if let Some(selected) = crate::audio::EngineChoice::ALL
            .into_iter()
            .find(|choice| *choice == frontend.audio.engine)
        {
            ui.label(RichText::new(selected.description()).size(15.).color(DIM));
        }
        let ui = &mut columns[column_count - 1];
        if column_count == 1 {
            ui.add_space(16.);
        }
        section(ui, "Mix");
        ui.add_space(6.);
        let audio = &mut frontend.audio;
        for (value, label) in [
            (&mut audio.master, "Master"),
            (&mut audio.engine_volume, "Your engine"),
            (&mut audio.opponents, "Other cars"),
            (&mut audio.effects, "Tyres, wind & kerbs"),
        ] {
            setting_row(ui, label, |ui| {
                ui.add_sized(
                    egui::vec2(ui.available_width(), 28.),
                    egui::Slider::new(value, 0.0..=1.0)
                        .custom_formatter(|value, _| format!("{:.0}%", value * 100.)),
                );
            });
        }
        setting_row(ui, "Audio", |ui| {
            ui.checkbox(&mut frontend.muted, "Mute  ·  M");
        });
    });
}

fn display_panel(ui: &mut egui::Ui, frontend: &mut Frontend) {
    let column_count = if ui.available_width() >= 620. { 2 } else { 1 };
    ui.columns(column_count, |columns| {
        let instruments = &mut columns[0];
        section(instruments, "In-race instruments");
        instruments.add_space(4.);
        for (layout, label, description, _key) in [
            (
                hud::Layout::Segments,
                "Digital cockpit",
                "Segment readouts with tyre and aero instruments.",
                "hud-segments",
            ),
            (
                hud::Layout::Gauges,
                "Analog telemetry",
                "Needle gauges for revs, speed, pedals and tyres.",
                "hud-gauges",
            ),
        ] {
            let response = option_card(
                instruments,
                frontend.display.layout == layout,
                label,
                description,
            );
            if response.clicked() {
                frontend.display.layout = layout;
            }
            #[cfg(test)]
            frontend.widgets.insert(_key, response.rect);
            instruments.add_space(8.);
        }

        let minimap = &mut columns[column_count - 1];
        if column_count == 1 {
            minimap.add_space(10.);
        }
        section(minimap, "Minimap");
        minimap.add_space(4.);
        for (mode, label, description, _key) in [
            (
                hud::MapMode::FullCircuit,
                "Full circuit",
                "The whole circuit and every car.",
                "map-full",
            ),
            (
                hud::MapMode::Nearby,
                "Nearby track",
                "Zoomed in around you, your car pointing up.",
                "map-nearby",
            ),
        ] {
            let response = option_card(minimap, frontend.display.map == mode, label, description);
            if response.clicked() {
                frontend.display.map = mode;
            }
            #[cfg(test)]
            frontend.widgets.insert(_key, response.rect);
            minimap.add_space(8.);
        }
    });
}

/// A selectable card: title, one-line description and a check mark when on.
fn option_card(
    ui: &mut egui::Ui,
    selected: bool,
    title: &str,
    description: &str,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 74.), egui::Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        10.,
        if selected {
            Color32::from_rgb(46, 24, 28)
        } else if hovered {
            Color32::from_rgb(30, 31, 35)
        } else {
            WELL
        },
    );
    painter.rect_stroke(
        rect,
        10.,
        egui::Stroke::new(
            if selected { 2. } else { 1. },
            if selected { RED } else { WELL_RIM },
        ),
        egui::StrokeKind::Inside,
    );
    let mark = egui::pos2(rect.right() - 28., rect.center().y);
    painter.circle_stroke(
        mark,
        9.,
        egui::Stroke::new(2., if selected { RED } else { WELL_RIM }),
    );
    if selected {
        painter.circle_filled(mark, 5., RED);
    }
    painter.text(
        rect.left_top() + egui::vec2(16., 14.),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(18.),
        Color32::WHITE,
    );
    painter.text(
        rect.left_top() + egui::vec2(16., 42.),
        egui::Align2::LEFT_TOP,
        description,
        egui::FontId::proportional(14.),
        DIM,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn camera_panel(ui: &mut egui::Ui, selected: &mut Mode, session: &mut Session) {
    ui.label(
        "Pick the view you want to drive from. You can fine-tune its position in the live scene.",
    );
    ui.add_space(14.);
    let descriptions = [
        "Wheel and instruments view.",
        "Broadcast camera above the track.",
        "Wide view behind the car.",
    ];
    let columns = if ui.available_width() >= 520. { 3 } else { 1 };
    ui.columns(columns, |columns_ui| {
        for (index, mode) in Mode::ALL.into_iter().enumerate() {
            let column_index = if columns == 1 { 0 } else { index };
            let column = &mut columns_ui[column_index];
            let active = *selected == mode;
            let response = column.add_sized(
                egui::vec2(column.available_width(), 48.),
                egui::Button::new(
                    RichText::new(mode.label())
                        .size(17.)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(if active { RED } else { WELL }),
            );
            if response.clicked() && *selected != mode {
                *selected = mode;
                session.camera.select(mode);
            }
            column.add_space(8.);
            column.label(RichText::new(descriptions[index]).color(DIM));
        }
    });
    ui.add_space(20.);
    section(ui, "Live adjustment");
    ui.add_space(6.);
    ui.label(RichText::new("Tune framing and camera movement in the live scene.").color(DIM));
    ui.add_space(8.);
    if ui
        .add_enabled(
            session.ready,
            egui::Button::new(if session.ready {
                "Adjust camera in scene  ·  F4"
            } else {
                "Available after starting a race"
            }),
        )
        .clicked()
    {
        session.screen = Screen::CameraEditor;
        session.paused = true;
    }
}

fn controls_panel(ui: &mut egui::Ui) {
    ui.label("Keyboard and gamepad shortcuts available while driving.");
    ui.add_space(4.);
    let bindings = [
        ["Action", "Keyboard", "Gamepad"],
        ["Throttle / brake", "W / S or arrows", "RT / LT"],
        ["Steer", "A / D or arrows", "Left stick"],
        ["Shift up / down", "E / Q", "RB / LB"],
        ["Cycle camera", "C (or 1 / 2 / 3)", "Y"],
        ["Adjust camera", "F4", "-"],
        ["Pause / resume", "Esc or P", "Start"],
        ["Recover car", "R", "Back / Select"],
        ["Mute / unmute", "M", "-"],
    ];
    if ui.available_width() < 720. {
        egui::Grid::new("controls-grid-compact")
            .num_columns(2)
            .spacing([12., 6.])
            .max_col_width((ui.available_width() * 0.48).max(90.))
            .show(ui, |ui| {
                ui.label(RichText::new("Action").strong().color(LABEL));
                ui.label(RichText::new("Bindings").strong().color(LABEL));
                ui.end_row();
                for row in bindings.into_iter().skip(1) {
                    ui.label(row[0]);
                    ui.vertical(|ui| {
                        ui.label(format!("Keyboard  {}", row[1]));
                        ui.label(format!("Gamepad  {}", row[2]));
                    });
                    ui.end_row();
                }
            });
    } else {
        let cell_width = ((ui.available_width() - 32.) / 3.).max(50.);
        egui::Grid::new("controls-grid")
            .num_columns(3)
            .spacing([12., 6.])
            .max_col_width(cell_width)
            .show(ui, |ui| {
                for (index, row) in bindings.into_iter().enumerate() {
                    for cell in row {
                        if index == 0 {
                            section(ui, cell);
                        } else {
                            ui.label(cell);
                        }
                    }
                    ui.end_row();
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_teams_selects_a_valid_driver_and_reselecting_keeps_the_driver() {
        let team: crate::teams::Team = serde_json::from_value(serde_json::json!({
            "id": "ferrari", "name": "Scuderia Ferrari", "short": "Ferrari",
            "colors": {"primary": "#ff0000", "secondary": "#000000", "accent": "#ffffff", "carbon": "#111111"},
            "drivers": [
                {"id": "leclerc", "name": "Charles Leclerc", "abbr": "LEC", "number": 16},
                {"id": "sainz", "name": "Carlos Sainz", "abbr": "SAI", "number": 55}
            ]
        })).unwrap();
        let mut draft = Options::parse(std::iter::empty()).unwrap();
        draft.team = "redbull".into();
        draft.driver = "verstappen".into();
        select_team(&mut draft, &team);
        assert_eq!((&*draft.team, &*draft.driver), ("ferrari", "leclerc"));
        draft.driver = "sainz".into();
        select_team(&mut draft, &team);
        assert_eq!(draft.driver, "sainz");
    }

    #[test]
    fn settings_sliders_fit_their_columns() {
        let context = egui::Context::default();
        context.set_fonts(hud::fonts());
        let mut style = (*context.style_of(egui::Theme::Dark)).clone();
        style.spacing.button_padding = egui::vec2(14., 6.);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(17.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(17.));
        context.set_style_of(egui::Theme::Dark, style);
        for width in [260., 320., 440., 657., 900.] {
            context.begin_pass(egui::RawInput::default());
            let mut ui = egui::Ui::new(
                context.clone(),
                "slider-layout".into(),
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 400.),
                )),
            );
            let mut value = 300.;
            setting_row(&mut ui, "Shadow distance", |ui| {
                let bounds = ui.available_rect_before_wrap();
                let response = ui.add_sized(
                    egui::vec2(ui.available_width(), 28.),
                    egui::Slider::new(&mut value, 30.0..=300.0).suffix(" m"),
                );
                assert!(
                    response.rect.left() >= bounds.left()
                        && response.rect.right() <= bounds.right(),
                    "slider overflows {width}-point column: {bounds:?}, {:?}",
                    response.rect
                );
            });
            assert!(ui.min_rect().right() <= width + 0.5);
            context.end_pass().textures_delta.clear();
        }
    }

    #[test]
    fn gen22_requires_roster_and_opponent_lod_for_the_selected_field() {
        let root = std::env::temp_dir().join(format!("f1-car-catalog-{}", std::process::id()));
        let car = root.join("cars/gen22");
        std::fs::create_dir_all(&car).unwrap();
        for name in ["car.glb", "car.json"] {
            std::fs::write(car.join(name), []).unwrap();
        }
        let mut options = Options::parse(std::iter::empty()).unwrap();
        options.assets = root.clone();
        options.opponents = 0;
        assert!(!Catalog::discover(&root).car_available(&options));
        std::fs::write(car.join("teams.json"), []).unwrap();
        assert!(!Catalog::discover(&root).car_available(&options));
        std::fs::write(car.join("teams.json"), br#"{"teams":[]}"#).unwrap();
        assert!(!Catalog::discover(&root).car_available(&options));
        std::fs::write(
            car.join("teams.json"),
            include_bytes!("../../tools/car/teams.json"),
        )
        .unwrap();
        let catalog = Catalog::discover(&root);
        assert!(catalog.car_available(&options));
        options.opponents = 1;
        assert!(!catalog.car_available(&options));
        std::fs::write(car.join("car_lod1.glb"), []).unwrap();
        assert!(catalog.car_available(&options));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn discovery_distinguishes_complete_incomplete_and_deferred_tracks() {
        let root = std::env::temp_dir().join(format!(
            "f1-catalog-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for (directory, complete) in [
            ("tracks/spa", true),
            ("tracks/silverstone", true),
            ("tracks/missing", false),
            ("deferred/needs-repair", true),
        ] {
            let path = root.join(directory);
            std::fs::create_dir_all(&path).unwrap();
            let track = serde_json::json!({"name":directory, "centerline":vec![[0.,0.,0.];8]});
            std::fs::write(path.join("track.json"), track.to_string()).unwrap();
            if complete {
                std::fs::write(path.join("visual.glb"), []).unwrap();
                std::fs::write(path.join("collision.bin"), []).unwrap();
            }
        }
        let catalog = Catalog::discover(&root);
        assert_eq!(
            catalog.circuits.len(),
            if cfg!(feature = "exp") { 4 } else { 1 }
        );
        assert_eq!(
            catalog
                .circuits
                .iter()
                .filter(|track| track.unavailable.is_none())
                .count(),
            if cfg!(feature = "exp") { 2 } else { 1 }
        );
        assert!(catalog.circuits.iter().any(|track| track.id == "spa"));
        assert_eq!(
            catalog
                .circuits
                .iter()
                .any(|track| track.id == "silverstone"),
            cfg!(feature = "exp")
        );
        if cfg!(feature = "exp") {
            assert!(catalog
                .circuits
                .iter()
                .find(|track| track.id == "needs-repair")
                .unwrap()
                .unavailable
                .as_ref()
                .unwrap()
                .contains("deferred"));
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
