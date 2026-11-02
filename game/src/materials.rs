//! Bring imported car surfaces into the same daylight response as the track.

use bevy::{gltf::GltfMaterialName, mesh::UvChannel, prelude::*};
use std::collections::{HashMap, HashSet};

#[derive(Component)]
pub(crate) struct CarSurface {
    pub lod: Option<&'static str>,
    pub compound: racecraft_sim::Compound,
}

#[derive(Component)]
pub(crate) struct SurfaceReady;

type PendingSurfaces<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GltfMaterialName,
        &'static CarSurface,
        &'static MeshMaterial3d<StandardMaterial>,
    ),
    (With<CarSurface>, Without<SurfaceReady>),
>;

#[allow(clippy::too_many_arguments)]
pub fn prepare(
    mut commands: Commands,
    surfaces: PendingSurfaces,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut prepared: Local<HashSet<AssetId<StandardMaterial>>>,
    mut compounds: Local<CompoundMaterials>,
    server: Res<AssetServer>,
    options: Res<crate::options::Options>,
) {
    for (entity, name, surface, handle) in &surfaces {
        {
            let Some(mut material) = materials.get_mut(&handle.0) else {
                continue;
            };
            if prepared.insert(handle.0.id()) {
                calibrate(&name.0, &mut material);
            }
        }
        if options.car == "gen22" && name.0 == "tyre" {
            let compound = surface.compound.label().to_ascii_lowercase();
            let path = tyre_texture(surface.lod, surface.compound);
            let instance =
                compounds.instance(&compound, &handle.0, server.load(path), &mut materials);
            commands.entity(entity).insert(MeshMaterial3d(instance));
        }
        commands.entity(entity).insert(SurfaceReady);
    }
}

fn tyre_texture(lod: Option<&str>, compound: racecraft_sim::Compound) -> String {
    let compound = compound.label().to_ascii_lowercase();
    let directory = lod.map_or_else(String::new, |lod| format!("{lod}/"));
    format!("cars/gen22/tyres/{directory}{compound}.png")
}

#[derive(Default)]
pub(crate) struct CompoundMaterials(
    HashMap<(String, AssetId<StandardMaterial>), Handle<StandardMaterial>>,
);

impl CompoundMaterials {
    fn instance(
        &mut self,
        compound: &str,
        source: &Handle<StandardMaterial>,
        texture: Handle<Image>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        self.0
            .entry((compound.to_owned(), source.id()))
            .or_insert_with(|| {
                let mut material = materials.get(source).unwrap().clone();
                material.base_color_texture = Some(texture);
                material.base_color_channel = UvChannel::Uv0;
                materials.add(material)
            })
            .clone()
    }
}

fn calibrate(name: &str, material: &mut StandardMaterial) {
    if name.starts_with("source_paint_") {
        // Neutral stock paint also handles older assets with embedded artwork.
        material.base_color_texture = None;
        material.base_color = Color::linear_rgb(0.32, 0.34, 0.36);
        material.reflectance = 0.4;
        material.clearcoat = 0.;
        return;
    }
    // Other Gen22 PBR values and texture maps are authored by the exporter.
    if name.starts_with("source_") || name.starts_with("wheel_") {
        return;
    }
    let name = name.to_ascii_lowercase();
    // Imported exporter specular colors can exceed 1.0 (up to 12 on the RB6 visor).
    material.specular_tint = Color::WHITE;
    if name.contains("mirror") {
        material.metallic = 1.;
        material.perceptual_roughness = 0.12;
        material.reflectance = 0.5;
    } else if ["tyre", "tread", "sidewall"]
        .iter()
        .any(|part| name.contains(part))
    {
        material.metallic = 0.;
        material.perceptual_roughness = 0.7;
        material.reflectance = 0.5;
        material.clearcoat = 0.;
    } else if matches!(name.as_str(), "suit" | "rubber" | "brake") {
        material.metallic = 0.;
        material.perceptual_roughness = 0.8;
        material.reflectance = 0.25;
    } else if matches!(name.as_str(), "rim" | "metal") {
        material.metallic = 0.8;
        material.perceptual_roughness = 0.35;
    } else if name.contains("carbon") {
        material.metallic = 0.;
        material.perceptual_roughness = if name == "carbon_gloss" { 0.32 } else { 0.65 };
        material.reflectance = 0.18;
    } else if ["paint", "decals", "number", "redbull_color"]
        .iter()
        .any(|part| name.contains(part))
        || name == "body"
        || name == "body_slice"
    {
        // Painted composite is dielectric, not polished metal. Keep the livery map.
        material.metallic = 0.;
        material.perceptual_roughness = 0.48;
        material.reflectance = 0.25;
        // Legacy body artwork mixes painted and exposed composite regions.
        material.clearcoat = 0.;
    } else if name.contains("glass") {
        material.metallic = 0.;
        material.perceptual_roughness = 0.18;
        material.reflectance = 0.5;
    } else {
        material.perceptual_roughness = material.perceptual_roughness.max(0.4);
        material.reflectance = material.reflectance.clamp(0.2, 0.5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inspect real compound instances, UV artwork and the BC7 path in all LODs.
    #[test]
    #[ignore = "requires a Vulkan display and F1_TEST_ASSETS"]
    fn rendered_tyre_compounds() {
        let root = std::env::var("F1_TEST_ASSETS").expect("set F1_TEST_ASSETS");
        let options = crate::options::Options::parse(
            [
                "--assets",
                &root,
                "--car",
                "gen22",
                "--team",
                "mclaren",
                "--graphics",
                "ultra",
                "--size",
                "1280x720",
                "--mute",
                "--frames",
                "10000",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();
        let mut app = crate::build_app(options).unwrap();
        {
            let mut session = app.world_mut().resource_mut::<crate::Session>();
            for (index, entrant) in session.race.entrants.iter_mut().enumerate() {
                entrant.car.setup.compound = racecraft_sim::Compound::ALL[index % 3];
            }
        }
        app.add_systems(Update, tyre_review.after(crate::camera::update));
        assert!(app.run().is_success());
    }

    #[allow(clippy::too_many_arguments)]
    fn tyre_review(
        mut commands: Commands,
        mut session: ResMut<crate::Session>,
        mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
        surfaces: Query<
            (
                &CarSurface,
                &GltfMaterialName,
                &MeshMaterial3d<StandardMaterial>,
            ),
            With<SurfaceReady>,
        >,
        materials: Res<Assets<StandardMaterial>>,
        images: Res<Assets<Image>>,
        device: Res<bevy::render::renderer::RenderDevice>,
        server: Res<AssetServer>,
        mut frame: Local<u32>,
        mut captured: Local<usize>,
        mut exit: MessageWriter<AppExit>,
    ) {
        if !session.ready {
            return;
        }
        session.paused = true;
        *frame += 1;
        if *frame == 40 {
            let mut checked = std::collections::BTreeSet::new();
            for (surface, name, handle) in &surfaces {
                if name.0 != "tyre" {
                    continue;
                }
                let material = materials.get(&handle.0).unwrap();
                assert_eq!(material.perceptual_roughness, 0.7);
                assert_eq!(material.reflectance, 0.5);
                assert_eq!(material.metallic, 0.);
                assert_eq!(material.base_color, Color::WHITE);
                let texture = material.base_color_texture.as_ref().unwrap();
                let image = images.get(texture).unwrap();
                assert!(image.texture_descriptor.mip_level_count > 1);
                if device
                    .features()
                    .contains(bevy::render::render_resource::WgpuFeatures::TEXTURE_COMPRESSION_BC)
                {
                    assert_eq!(
                        image.texture_descriptor.format,
                        bevy::render::render_resource::TextureFormat::Bc7RgbaUnormSrgb,
                    );
                }
                assert_eq!(
                    server.get_path(texture.id()).unwrap().path(),
                    std::path::Path::new(&tyre_texture(surface.lod, surface.compound))
                );
                checked.insert((surface.lod, surface.compound.label()));
            }
            assert_eq!(
                checked.len(),
                7,
                "player plus three compounds in both opponent LODs"
            );
            info!("Tyre compound instances verified: {checked:?}");
        }
        let stage = (*frame / 60).min(5);
        let entrant = [session.player, 0, 1, 2, 0, 2][stage as usize];
        let pose = crate::pose::Pose::of(&session.race.entrants[entrant].car);
        let origin = pose.origin.as_vec3();
        let rotation = Quat::from_rotation_y(pose.yaw as f32);
        let offset = if stage < 4 {
            Vec3::new(3., 1., 1.4)
        } else {
            Vec3::new(-15., 40., 28.)
        };
        for (mut camera, mut projection) in &mut cameras {
            if let Projection::Perspective(lens) = &mut *projection {
                lens.fov = if stage < 4 { 60_f32 } else { 7_f32 }.to_radians();
            }
            *camera = Transform::from_translation(origin + rotation * offset)
                .looking_at(origin + rotation * Vec3::new(0., 0.4, 1.3), Vec3::Y);
        }
        if *frame % 60 == 50 && *frame < 360 {
            use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
            let directory = std::path::PathBuf::from(
                std::env::var("F1_TEST_OUTPUT").expect("set F1_TEST_OUTPUT"),
            );
            std::fs::create_dir_all(&directory).unwrap();
            let path = directory.join(format!("tyre-review-{stage}.png"));
            commands.spawn(Screenshot::primary_window()).observe(
                move |event: On<ScreenshotCaptured>| {
                    event
                        .image
                        .clone()
                        .try_into_dynamic()
                        .unwrap()
                        .to_rgb8()
                        .save(&path)
                        .unwrap();
                },
            );
            *captured += 1;
        }
        if *frame == 360 {
            assert_eq!(*captured, 6);
            exit.write(AppExit::Success);
        }
    }

    #[test]
    fn compound_textures_follow_each_entrant_at_every_lod() {
        for compound in racecraft_sim::Compound::ALL {
            let name = compound.label().to_ascii_lowercase();
            for lod in [None, Some("lod1"), Some("lod2")] {
                let path = tyre_texture(lod, compound);
                assert!(path.ends_with(&format!("/{name}.png")));
                assert_eq!(path.contains("lod1/"), lod == Some("lod1"));
                assert_eq!(path.contains("lod2/"), lod == Some("lod2"));
            }
        }
    }

    #[test]
    fn source_pbr_survives_runtime_preparation() {
        for name in [
            "source_BMD_Brass",
            "source_Translucent Plastic_red",
            "wheel_legends",
        ] {
            let mut material = StandardMaterial {
                base_color: Color::srgb(0.1, 0.02, 0.01),
                metallic: 1.,
                perceptual_roughness: 0.18,
                clearcoat: 0.25,
                emissive: LinearRgba::rgb(0.12, 0.01, 0.01),
                ..default()
            };
            let expected = material.clone();
            calibrate(name, &mut material);
            assert_eq!(material.base_color, expected.base_color);
            assert_eq!(material.metallic, expected.metallic);
            assert_eq!(material.perceptual_roughness, expected.perceptual_roughness);
            assert_eq!(material.clearcoat, expected.clearcoat);
            assert_eq!(material.emissive, expected.emissive);
        }
    }

    #[test]
    fn stock_gen22_paint_has_no_artwork_and_preserves_surface_detail() {
        let mut paint = StandardMaterial {
            base_color: Color::srgb(0.02, 0.01, 0.005),
            clearcoat: 0.16,
            reflectance: 0.5,
            perceptual_roughness: 0.3,
            ..default()
        };
        calibrate("source_paint_main_body_stickers_lod.001", &mut paint);
        assert_eq!(paint.base_color, Color::linear_rgb(0.32, 0.34, 0.36));
        assert!(paint.base_color_texture.is_none());
        assert_eq!(paint.perceptual_roughness, 0.3);
        assert_eq!(paint.clearcoat, 0.);
        assert_eq!(paint.reflectance, 0.4);
    }

    #[test]
    fn paint_is_dielectric_and_rubber_has_a_dielectric_sheen() {
        let mut paint = StandardMaterial {
            metallic: 0.8,
            perceptual_roughness: 0.,
            base_color: Color::srgb(0.05, 0.08, 0.2),
            ..default()
        };
        let color = paint.base_color;
        calibrate("body", &mut paint);
        assert_eq!(paint.base_color, color);
        assert_eq!(paint.metallic, 0.);
        assert!(paint.perceptual_roughness >= 0.3);
        for name in ["tyre", "tread", "sidewall.007"] {
            let mut tyre = StandardMaterial {
                base_color: Color::linear_rgb(0.04, 0.04, 0.04),
                ..default()
            };
            let rubber = tyre.base_color;
            calibrate(name, &mut tyre);
            assert_eq!(tyre.base_color, rubber);
            assert_eq!(tyre.perceptual_roughness, 0.7);
            assert_eq!(tyre.reflectance, 0.5);
            assert_eq!(tyre.metallic, 0.);
        }
    }
}
