use std::sync::atomic::Ordering;

use crate::compute::{
    CHUNK, ComputeInvocation, GlobalComputeStore, invocation::FractalNoiseBounds,
};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};

pub(super) fn generation_worker(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    global_compute_store: Res<GlobalComputeStore>,
) {
    let mut receiver = global_compute_store.receiver.lock().unwrap();

    bevy::log::info!("generation worker: {}", receiver.len());

    while let Ok(index) = receiver.try_recv() {
        // Load Reusable Storage Buffers
        let ComputeInvocation {
            chunk,
            active,
            output: _,
            staging,
        } = &global_compute_store.buffers[index];

        debug_assert_eq!(active.load(Ordering::Acquire), true);

        {
            // Load Data from Buffer
            let view = staging.get_mapped_range(..);
            let heightmap = bytemuck::cast_slice(&view);
            let chunk = (*chunk.lock().unwrap()).unwrap();

            // Generate Chunk
            let mesh = generate_mesh(&FractalNoiseBounds::from(&chunk), heightmap);

            // Spawn Chunk
            commands.spawn(( // FIXME SPAWN ME IN THE OTHER WORLD
                Mesh3d(asset_server.add(mesh)),
                MeshMaterial3d(asset_server.add(StandardMaterial {
                    base_color: Color::linear_rgb(0., 1., 0.),
                    ..Default::default()
                })),
                Transform::from_translation(Vec3 {
                    x: chunk.min.x as f32,
                    y: 0.,
                    z: chunk.min.y as f32,
                }),
            ));
        }

        // Clean Up Buffers
        staging.unmap();
        active.store(false, Ordering::Release);
    }
}

pub fn generate_mesh(chunk: &FractalNoiseBounds, heightmap: &[f32]) -> Mesh {
    let mut vertice = [[0f32; 3]; (CHUNK[0] * CHUNK[1]) as usize];
    let mut normals = [[0f32; 3]; (CHUNK[0] * CHUNK[1]) as usize];
    let mut indices = Vec::<u32>::with_capacity((2 * CHUNK[0] * (CHUNK[1] - 1)) as usize);

    let mut vertex_index = 0;
    for cz in 0..CHUNK[1] {
        let z = chunk.zstep * cz as f32;
        for cx in 0..CHUNK[0] {
            let x = chunk.xstep * cx as f32;

            // Sample Vertex
            vertice[vertex_index] = [x, heightmap[vertex_index], z];

            // Generate Indices for Quad
            if cz != CHUNK[1] - 1 {
                if cz % 2 == 0 {
                    indices.push((vertex_index) as u32);
                    //( cz    * SIDE + cx) as u32); // top left
                    indices.push(vertex_index as u32 + CHUNK[0]);
                    //((cz+1) * SIDE + cx) as u32); // bottom left
                } else {
                    let temp = vertex_index as u32 + CHUNK[0] - 2 * cx - 1;
                    indices.push(temp + CHUNK[0]);
                    //((cz+1) * SIDE + (SIDE - (cx+1))) as u32); // opposite side - bottom right
                    indices.push(temp);
                    //( cz    * SIDE + (SIDE - (cx+1))) as u32); // opposite side - top right
                }
            }

            vertex_index += 1;
        }
    }

    // for cz in 0..CHUNK[1] {
    //     for cx in 0..CHUNK[0] {
    //         let i = (cz * CHUNK[0] + cx) as usize;

    //         let me = Vec3::from_array(vertice[i]);
    //         let sample = me.xz() + Vec2::new(chunk.x, chunk.z);

    //         let left = if cx >= 1 {
    //             vertice[i - 1][1]
    //         } else {
    //             heightmap(sample - samplesize.with_y(0.))
    //         };
    //         let right = if cx < CHUNK[0] - 1 {
    //             vertice[i + 1][1]
    //         } else {
    //             heightmap(sample + samplesize.with_y(0.))
    //         };
    //         let top = if cz >= 1 {
    //             vertice[i - CHUNK[0]][1]
    //         } else {
    //             heightmap(sample + samplesize.with_x(0.))
    //         };
    //         let bottom = if cz < CHUNK[1] - 1 {
    //             vertice[i + CHUNK[0]][1]
    //         } else {
    //             heightmap(sample - samplesize.with_x(0.))
    //         };

    //         normals[i] = Vec3 {
    //             x: left - right,
    //             y: 1.0,
    //             z: top - bottom,
    //         }
    //         .normalize_or_zero()
    //         .to_array();
    //     }
    // }

    debug_assert_eq!(indices.capacity(), (2 * CHUNK[0] * (CHUNK[1] - 1)) as usize);

    // Create Mesh
    Mesh::new(
        PrimitiveTopology::TriangleStrip,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(Vec::from(vertice)),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(Vec::from(normals)),
    )
    .with_inserted_indices(Indices::U32(indices))
}
