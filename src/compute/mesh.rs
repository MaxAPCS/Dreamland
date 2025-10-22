use std::{iter::StepBy, ops::Range};

use bevy::{
    asset::RenderAssetUsages,
    math::vec2,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};

pub struct Chunk {
    pub x: i32,
    pub z: i32,
    pub w: usize,
    pub h: usize,
    pub lod: usize,
}

impl Chunk {
    fn range_x(&self) -> StepBy<Range<i32>> {
        (self.x..(self.x + self.w as i32)).step_by(self.lod)
    }
    fn range_z(&self) -> StepBy<Range<i32>> {
        (self.z..(self.z + self.h as i32)).step_by(self.lod)
    }
    fn samples(&self) -> usize {
        (self.w * self.h) / (self.lod * self.lod)
    }
    fn samples_z(&self) -> usize {
        self.h / self.lod
    }
}

pub fn mesh(chunk: Chunk, heightmap: &dyn Fn(Vec2) -> f32) -> Mesh {
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    vertices.resize(chunk.samples(), [0.0f32, 0.0f32, 0.0f32]);
    normals.resize(chunk.samples(), [0.0f32, 0.0f32, 0.0f32]);
    let uvs = vec![[0.0, 0.0]; vertices.len()];

    let width = chunk.samples_z();

    let mut vertex_index = 0;
    for cz in chunk.range_z() {
        for cx in chunk.range_x() {
            vertices[vertex_index] = [cx as f32, heightmap(vec2(cx as f32, cz as f32)), cz as f32];

            if cx != chunk.x && cz != chunk.z {
                // Triangle 1
                indices.push(vertex_index as u32); // bottom right
                indices.push((vertex_index - width) as u32); // top right
                indices.push((vertex_index - 1) as u32); // bottom left

                // Triangle 2
                indices.push((vertex_index - width) as u32); // top right
                indices.push((vertex_index - width - 1) as u32); // top left
                indices.push((vertex_index - 1) as u32); // bottom left
            }
            vertex_index += 1;
        }
    }
    
    // Compute normals
    for tri in indices.chunks(3) {
        let a = tri[0] as usize;
        let b = tri[1] as usize;
        let c = tri[2] as usize;
        let v0 = vertices[a];
        let v1 = vertices[b];
        let v2 = vertices[c];
        let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
        let normal = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];
        normals[a][0] += normal[0];
        normals[a][1] += normal[1];
        normals[a][2] += normal[2];
        normals[b][0] += normal[0];
        normals[b][1] += normal[1];
        normals[b][2] += normal[2];
        normals[c][0] += normal[0];
        normals[c][1] += normal[1];
        normals[c][2] += normal[2];
    }
    
    // Normalize normals
    for normal in &mut normals {
        let length = (normal[0].powi(2) + normal[1].powi(2) + normal[2].powi(2)).sqrt();
        if length > 0.0 {
            normal[0] /= length;
            normal[1] /= length;
            normal[2] /= length;
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(vertices),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(normals),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(uvs))
    .with_inserted_indices(Indices::U32(indices))
}
