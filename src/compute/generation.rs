use std::sync::atomic::Ordering;

use crate::compute::{
    ComputeInvocation, GenerationDequeue, GlobalComputeStoreRef, SAMPLES,
    invocation::{FractalNoiseBounds, PointAndNormal},
};
use avian3d::prelude::{Collider, RigidBody};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};

#[derive(Message)]
pub struct GenerationMessage {
    pub bounds: IRect,
    pub entity: Entity,
}

pub(super) fn generation_worker(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    global_compute_store: Res<GlobalComputeStoreRef>,
    mut gen_dequeue: ResMut<GenerationDequeue>,
    mut messager: MessageWriter<GenerationMessage>,
) {
    while let Ok((index, chunk)) = gen_dequeue.0.try_recv() {
        // Load Reusable Storage Buffers
        let ComputeInvocation {
            active,
            output: _,
            staging,
        } = &global_compute_store.0[index];

        debug_assert_eq!(active.load(Ordering::Acquire), true);

        let id = {
            // Load Data from Buffer
            let view = staging.get_mapped_range(..);
            let heightmap: &[PointAndNormal] =
                bytemuck::try_cast_slice(&view).expect("Buffer Interpretation Failed");

            if chunk.width() == SAMPLES[0] as i32 && chunk.height() == SAMPLES[2] as i32 {
                // Highest Detail - Generate Physics
                let mesh = generate_mesh_trilist(&FractalNoiseBounds::from(&chunk), heightmap);

                commands
                    .spawn((
                        RigidBody::Static,
                        Collider::trimesh_from_mesh(&mesh).unwrap(),
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
                    ))
                    .id()
            } else {
                // Lower Detail - No Physics
                let mesh = generate_mesh(&FractalNoiseBounds::from(&chunk), heightmap);

                commands
                    .spawn((
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
                    ))
                    .id()
            }
        };
        messager.write(GenerationMessage {
            bounds: chunk,
            entity: id,
        });

        // Clean Up Buffers
        staging.unmap();
        active.store(false, Ordering::Release);
    }
}

fn generate_mesh(chunk: &FractalNoiseBounds, heightmap: &[PointAndNormal]) -> Mesh {
    let mut vertice = [[0f32; 3]; (SAMPLES[0] * SAMPLES[1] * SAMPLES[2]) as usize];
    let mut normals = [[0f32; 3]; (SAMPLES[0] * SAMPLES[1] * SAMPLES[2]) as usize];
    let mut indices = Vec::<u32>::with_capacity((2 * SAMPLES[0] * (SAMPLES[2] - 1)) as usize);

    debug_assert_eq!(heightmap.len() as u32, SAMPLES[0] * SAMPLES[1] * SAMPLES[2]);

    let mut i = 0;
    for cz in 0..SAMPLES[2] {
        let z = chunk.zstep * cz as f32;
        for cx in 0..SAMPLES[0] {
            let x = chunk.xstep * cx as f32;

            debug_assert_eq!(i, (cx + cz * SAMPLES[0]) as usize);

            // Sample Vertex
            vertice[i] = [x, heightmap[i].point, z];
            normals[i] = heightmap[i].normal;

            // Generate Indices
            if cz != SAMPLES[2] - 1 {
                if cz % 2 == 0 {
                    indices.push(i as u32);
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

fn generate_mesh_trilist(chunk: &FractalNoiseBounds, heightmap: &[PointAndNormal]) -> Mesh {
    let mut vertice = [[0f32; 3]; (SAMPLES[0] * SAMPLES[1] * SAMPLES[2]) as usize];
    let mut normals = [[0f32; 3]; (SAMPLES[0] * SAMPLES[1] * SAMPLES[2]) as usize];
    let mut indices = Vec::<u32>::with_capacity((6 * (SAMPLES[0] - 1) * (SAMPLES[2] - 1)) as usize);

    debug_assert_eq!(heightmap.len() as u32, SAMPLES[0] * SAMPLES[1] * SAMPLES[2]);

    let mut i = 0;
    for cz in 0..SAMPLES[2] {
        let z = chunk.zstep * cz as f32;
        for cx in 0..SAMPLES[0] {
            let x = chunk.xstep * cx as f32;

            debug_assert_eq!(i, (cx + cz * SAMPLES[0]) as usize);

            // Sample Vertex
            vertice[i] = [x, heightmap[i].point, z];
            normals[i] = heightmap[i].normal;

            // Generate Indices
            if cx != SAMPLES[0] - 1 && cz != SAMPLES[2] - 1 {
                debug_assert!(i as u32 + SAMPLES[0] + 1 < vertice.len() as u32);

                // Triangle 1
                indices.push(i as u32);
                indices.push(i as u32 + SAMPLES[0]);
                indices.push(i as u32 + 1);

                // Triangle 2
                indices.push(i as u32 + SAMPLES[0]);
                indices.push(i as u32 + SAMPLES[0] + 1);
                indices.push(i as u32 + 1);
            }

            i += 1;
        }
    }

    debug_assert_eq!(i, heightmap.len());

    debug_assert_eq!(
        indices.capacity(),
        (6 * (SAMPLES[0] - 1) * (SAMPLES[2] - 1)) as usize
    );

    // Create Mesh
    Mesh::new(
        PrimitiveTopology::TriangleList,
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
