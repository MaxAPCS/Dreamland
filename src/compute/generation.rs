use std::sync::atomic::Ordering;

use crate::compute::{
    ComputeInvocation, GenerationDequeue, GlobalComputeStoreRef, SAMPLES,
    invocation::FractalNoiseBounds,
};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};

pub(super) fn generation_worker(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    global_compute_store: Res<GlobalComputeStoreRef>,
    mut gen_dequeue: ResMut<GenerationDequeue>,
) {
    while let Ok((index, chunk)) = gen_dequeue.0.try_recv() {
        // Load Reusable Storage Buffers
        let ComputeInvocation {
            active,
            output: _,
            staging,
        } = &global_compute_store.0[index];

        debug_assert_eq!(active.load(Ordering::Acquire), true);

        {
            // Load Data from Buffer
            let view = staging.get_mapped_range(..);
            let heightmap = bytemuck::cast_slice(&view);

            // Generate Chunk
            let mesh = generate_mesh(&FractalNoiseBounds::from(&chunk), heightmap);

            // Spawn Chunk
            commands.spawn((
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
    let mut vertice = [[0f32; 3]; (SAMPLES[0] * SAMPLES[1] * SAMPLES[2]) as usize];
    let mut normals = [[0f32; 3]; (SAMPLES[0] * SAMPLES[1] * SAMPLES[2]) as usize];
    let mut indices = Vec::<u32>::with_capacity((2 * SAMPLES[0] * (SAMPLES[2] - 1)) as usize);

    let mut i = 0;
    for cz in 0..SAMPLES[2] {
        let z = chunk.zstep * cz as f32;
        for cx in 0..SAMPLES[0] {
            let x = chunk.xstep * cx as f32;

            debug_assert_eq!(i, (cx + cz * SAMPLES[0]) as usize);

            // Sample Vertex
            vertice[i] = [x, heightmap[i], z];

            // Generate Indices for Quad
            if cz != SAMPLES[2] - 1 {
                if cz % 2 == 0 {
                    indices.push((i) as u32);
                    //( cz    * SIDE + cx) as u32); // top left
                    indices.push(i as u32 + SAMPLES[0]);
                    //((cz+1) * SIDE + cx) as u32); // bottom left
                } else {
                    let temp = i as u32 + SAMPLES[0] - 2 * cx - 1;
                    indices.push(temp + SAMPLES[0]);
                    //((cz+1) * SIDE + (SIDE - (cx+1))) as u32); // opposite side - bottom right
                    indices.push(temp);
                    //( cz    * SIDE + (SIDE - (cx+1))) as u32); // opposite side - top right
                }
            }

            i += 1;
        }
    }

    debug_assert_eq!(i, heightmap.len());

    i = 0;
    for cz in 0..SAMPLES[2] {
        for cx in 0..SAMPLES[0] {
            debug_assert_eq!(i, (cx + cz * SAMPLES[0]) as usize);

            let width = SAMPLES[0] as usize;
            let my_y = vertice[i][1];

            let xdiff = if cx == 0 {
                (my_y - vertice[i + 1][1]) * 2.
            } else if cx == SAMPLES[0] - 1 {
                (vertice[i - 1][1] - my_y) * 2.
            } else {
                vertice[i - 1][1] - vertice[i + 1][1]
            };

            let ydiff = if cz == 0 {
                (my_y - vertice[i + width][1]) * 2.
            } else if cz == SAMPLES[2] - 1 {
                (vertice[i - width][1] - my_y) * 2.
            } else {
                vertice[i - width][1] - vertice[i + width][1]
            };

            normals[i] = Vec3 {
                x: xdiff,
                y: 1.0,
                z: ydiff,
            }
            .normalize_or_zero()
            .to_array();

            i += 1;
        }
    }

    debug_assert_eq!(
        indices.capacity(),
        (2 * SAMPLES[0] * (SAMPLES[2] - 1)) as usize
    );

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
