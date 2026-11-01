//! Preserve the full GEN22 geometry while reusing transformed vertices between triangles.

use bevy::{mesh::Indices, prelude::*};

pub fn prepare(
    mut events: MessageReader<AssetEvent<Mesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    server: Res<AssetServer>,
) {
    for event in events.read() {
        let AssetEvent::Added { id } = event else {
            continue;
        };
        if !server.get_path(*id).is_some_and(|path| {
            path.path().starts_with("cars/gen22")
                && path
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "glb")
        }) {
            continue;
        }
        // Added already schedules extraction; do not trigger a second GPU upload.
        if let Some(mesh) = meshes.get_mut_untracked(*id) {
            optimize(mesh);
        }
    }
}

fn optimize(mesh: &mut Mesh) {
    if mesh.primitive_topology() != bevy::mesh::PrimitiveTopology::TriangleList {
        return;
    }
    let vertices = mesh.count_vertices();
    match mesh.indices_mut() {
        Some(Indices::U32(indices)) => meshopt::optimize_vertex_cache_in_place(indices, vertices),
        Some(Indices::U16(indices)) => {
            let original: Vec<_> = indices.iter().map(|&index| u32::from(index)).collect();
            *indices = meshopt::optimize_vertex_cache(&original, vertices)
                .into_iter()
                .map(|index| index as u16)
                .collect();
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;

    #[test]
    fn reordering_preserves_every_triangle_winding_and_vertex_attribute() {
        for narrow in [false, true] {
            let positions: Vec<_> = (0..40).map(|i| [i as f32, (i % 2) as f32, 0.]).collect();
            let indices: Vec<u32> = (0..36).flat_map(|i| [i, i + 1, i + 2]).collect();
            let mut mesh = Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone())
            .with_inserted_indices(if narrow {
                Indices::U16(indices.iter().map(|&index| index as u16).collect())
            } else {
                Indices::U32(indices.clone())
            });
            optimize(&mut mesh);
            let result: Vec<_> = mesh.indices().unwrap().iter().map(|i| i as u32).collect();
            let triangles = |values: &[u32]| {
                let mut triangles: Vec<_> =
                    values.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect();
                triangles.sort_unstable();
                triangles
            };
            assert_eq!(triangles(&indices), triangles(&result));
            assert_eq!(mesh.count_vertices(), positions.len());
            assert_eq!(
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                    .unwrap()
                    .as_float3()
                    .unwrap(),
                positions
            );
        }
    }
}
