//! Isolated studio view of the selected car, rendered into the setup menu.
use super::{Frontend, Screen, Tab};
use crate::Session;
use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    camera::{visibility::RenderLayers, Exposure, RenderTarget, ScalingMode},
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::render_resource::TextureFormat,
    world_serialization::{WorldAsset, WorldInstanceReady},
};
use bevy_egui::{egui, EguiTextureHandle, EguiUserTextures};
use racecraft_sim::Compound;

const LAYER: usize = 2;
const SIZE: (u32, u32) = (960, 400);

#[derive(Component)]
pub(crate) struct PreviewCamera;

#[derive(Resource)]
pub(crate) struct CarPreview {
    texture: egui::TextureId,
    selected: Option<(String, Compound)>,
    root: Option<Entity>,
    asset: Option<Handle<WorldAsset>>,
    pub(super) ready: bool,
    failed: bool,
}

impl CarPreview {
    #[cfg(test)]
    pub(super) fn selected_car(&self) -> Option<&str> {
        self.selected.as_ref().map(|(car, _)| car.as_str())
    }

    pub(super) fn show(&self, ui: &mut egui::Ui) {
        let size = egui::vec2(
            ui.available_width(),
            ui.available_width() * SIZE.1 as f32 / SIZE.0 as f32,
        );
        let response = ui.add(egui::Image::new((self.texture, size)).corner_radius(6));
        if !self.ready {
            ui.painter().text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                if self.failed {
                    "Car preview unavailable. Check the car assets."
                } else {
                    "Loading car…"
                },
                egui::FontId::proportional(17.),
                super::DIM,
            );
        }
    }
}

pub(crate) fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut textures: ResMut<EguiUserTextures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let image = images.add(Image::new_target_texture(
        SIZE.0,
        SIZE.1,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    let texture = textures.add_image(EguiTextureHandle::Strong(image.clone()));
    commands.insert_resource(CarPreview {
        texture,
        selected: None,
        root: None,
        asset: None,
        ready: false,
        failed: false,
    });
    commands.spawn((
        PreviewCamera,
        Camera3d::default(),
        Camera {
            order: -2,
            is_active: false,
            clear_color: Color::srgb_u8(9, 10, 12).into(),
            ..default()
        },
        RenderTarget::Image(image.into()),
        RenderLayers::layer(LAYER),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 3.5,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(8., 4.8, 9.).looking_at(Vec3::new(0., 0.5, 0.35), Vec3::Y),
        Exposure { ev100: 10.5 },
        Tonemapping::AcesFitted,
        Msaa::Sample4,
    ));
    for (position, intensity) in [
        (Vec3::new(4., 7., 6.), 4_000_000.),
        (Vec3::new(-5., 3., -4.), 2_000_000.),
    ] {
        commands.spawn((
            PointLight {
                intensity,
                range: 30.,
                radius: 1.5,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_translation(position),
            RenderLayers::layer(LAYER),
        ));
    }
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(200., 200.))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb_u8(22, 23, 27),
            perceptual_roughness: 1.,
            ..default()
        })),
        Transform::from_xyz(0., -0.025, 0.),
        RenderLayers::layer(LAYER),
    ));
}

pub(crate) fn update(
    mut commands: Commands,
    frontend: Res<Frontend>,
    session: Res<Session>,
    server: Res<AssetServer>,
    mut preview: ResMut<CarPreview>,
    mut cameras: Query<&mut Camera, With<PreviewCamera>>,
) {
    let visible = matches!(session.screen, Screen::Main | Screen::Paused)
        && matches!(frontend.tab, Tab::Track | Tab::Team);
    for mut camera in &mut cameras {
        camera.is_active = visible;
    }
    if !visible {
        return;
    }
    let selected = (&frontend.draft.car, frontend.draft.setup.compound);
    if preview
        .selected
        .as_ref()
        .is_none_or(|(car, compound)| (car, *compound) != selected)
    {
        if let Some(root) = preview.root.take() {
            commands.entity(root).despawn();
        }
        let scene = server.load(
            GltfAssetLabel::Scene(0).from_asset(format!("cars/{}/car.glb", frontend.draft.car)),
        );
        preview.root = Some(
            commands
                .spawn((WorldAssetRoot(scene.clone()), Transform::default()))
                .observe(ready)
                .id(),
        );
        preview.asset = Some(scene);
        preview.selected = Some((selected.0.clone(), selected.1));
        preview.ready = false;
        preview.failed = false;
    }
    preview.failed = preview.asset.as_ref().is_some_and(|asset| {
        matches!(
            server.get_load_state(asset.id()),
            Some(LoadState::Failed(_))
        ) || matches!(
            server.get_recursive_dependency_load_state(asset.id()),
            Some(RecursiveDependencyLoadState::Failed(_))
        )
    });
}

fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    surfaces: Query<(
        &bevy::gltf::GltfMaterialName,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
    mut preview: ResMut<CarPreview>,
) {
    if preview.root != Some(event.entity) {
        return;
    }
    for entity in children.iter_descendants(event.entity) {
        commands.entity(entity).insert(RenderLayers::layer(LAYER));
        if let Ok((name, handle)) = surfaces.get(entity) {
            if let Some(source) = materials.get(&handle.0) {
                let mut material = source.clone();
                crate::materials::calibrate(&name.0, &mut material);
                if let Some((_, compound)) =
                    preview.selected.as_ref().filter(|(car, _)| car == "gen22")
                {
                    if name.0 == "tyre" {
                        material.base_color_texture =
                            Some(server.load(crate::materials::tyre_texture(None, *compound)));
                    }
                }
                commands
                    .entity(entity)
                    .insert(MeshMaterial3d(materials.add(material)));
            }
        }
    }
    preview.ready = true;
}
