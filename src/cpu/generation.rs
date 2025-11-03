use avian3d::prelude::Collider;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};

const SIDE: usize = 128;

pub fn generate_mesh_physics(chunk: &IRect, heightmap: &dyn Fn(Vec2) -> f32) -> (Mesh, Collider) {
    let mut vertice = [[0f32; 3]; SIDE * SIDE];
    let mut normals = [[0f32; 3]; SIDE * SIDE];
    let mut indices = Vec::<u32>::with_capacity(2 * SIDE * (SIDE - 1));

    let mut physicsindices = Vec::<u32>::with_capacity(6 * (SIDE - 1) * (SIDE - 1));

    let samplesize = Vec2 {
        x: chunk.width() as f32 / (SIDE - 1) as f32,
        y: chunk.height() as f32 / (SIDE - 1) as f32,
    };

    {
        let mut vertex_index = 0;
        for cz in 0..SIDE {
            let z = samplesize.y * cz as f32;
            for cx in 0..SIDE {
                let x = samplesize.x * cx as f32;

                // Sample Vertex
                vertice[vertex_index] = [
                    x,
                    heightmap(Vec2 {
                        x: chunk.min.x as f32 + x,
                        y: chunk.min.y as f32 + z,
                    }),
                    z,
                ];

                // Generate Indices for Quad
                if cz != SIDE - 1 {
                    if cz % 2 == 0 {
                        indices.push((vertex_index) as u32);
                        //( cz    * SIDE + cx) as u32); // top left
                        indices.push((vertex_index + SIDE) as u32);
                        //((cz+1) * SIDE + cx) as u32); // bottom left
                    } else {
                        let temp = (vertex_index + SIDE - 2 * cx - 1) as u32;
                        indices.push(temp + SIDE as u32);
                        //((cz+1) * SIDE + (SIDE - (cx+1))) as u32); // opposite side - bottom right
                        indices.push(temp);
                        //( cz    * SIDE + (SIDE - (cx+1))) as u32); // opposite side - top right
                    }
                }

                if cx != SIDE - 1 && cz != SIDE - 1 {
                    // Triangle 1
                    physicsindices.push((vertex_index) as u32); // top left
                    physicsindices.push((vertex_index + SIDE) as u32); // bottom left
                    physicsindices.push((vertex_index + 1) as u32); // top right

                    // Triangle 2
                    physicsindices.push((vertex_index + SIDE) as u32); // bottom left
                    physicsindices.push((vertex_index + SIDE + 1) as u32); // bottom right
                    physicsindices.push((vertex_index + 1) as u32); // top right
                }

                vertex_index += 1;
            }
        }
    }

    for cz in 0..SIDE {
        for cx in 0..SIDE {
            let i = cz * SIDE + cx;

            let me = Vec3::from_array(vertice[i]);
            let sample = me.xz() + chunk.min.as_vec2();

            let left = if cx >= 1 {
                vertice[i - 1][1]
            } else {
                heightmap(sample - samplesize.with_y(0.))
            };
            let right = if cx < SIDE - 1 {
                vertice[i + 1][1]
            } else {
                heightmap(sample + samplesize.with_y(0.))
            };
            let top = if cz >= 1 {
                vertice[i - SIDE][1]
            } else {
                heightmap(sample + samplesize.with_x(0.))
            };
            let bottom = if cz < SIDE - 1 {
                vertice[i + SIDE][1]
            } else {
                heightmap(sample - samplesize.with_x(0.))
            };

            normals[i] = Vec3 {
                x: left - right,
                y: 1.0,
                z: top - bottom,
            }
            .normalize_or_zero()
            .to_array();
        }
    }

    debug_assert_eq!(indices.capacity(), 2 * SIDE * (SIDE - 1));

    // Create Mesh
    let verticevec = Vec::from(vertice);
    let collidervec = Vec::from_iter(verticevec.iter().map(|a| Vec3::from_array(*a)));
    (
        Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(verticevec),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            VertexAttributeValues::Float32x3(Vec::from(normals)),
        )
        .with_inserted_indices(Indices::U32(indices)),
        Collider::trimesh(
            collidervec,
            Vec::from_iter(physicsindices.chunks_exact(3).map(|c| [c[0], c[1], c[2]])),
        ),
    )
}
