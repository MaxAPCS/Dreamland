use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
};

const WORKGROUP: usize = 64;

struct FractalNoiseConfig {
    seed: f32,
    octaves: u32,
    lacunarity: f32,
    gain: f32,
}

const CONFIG: FractalNoiseConfig = FractalNoiseConfig {
    seed: 0.0,
    octaves: 4,
    lacunarity: 2.0,
    gain: 0.5,
};

#[derive(Resource)]
pub struct SimplexPipeline {
    pub pipeline: CachedComputePipelineId,
}

pub fn create(mut commands: Commands, asset_server: Res<AssetServer>, pipeline_cache: Res<PipelineCache>, render_device: Res<RenderDevice>) {
    let texture_bind_group_layout = render_device.create_bind_group_layout(
        "Simplex Layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        (2 * std::mem::size_of::<FractalNoiseConfig>()) as _,
                    ),
                },
            },
            BindGroupLayoutEntry {
                binding: 1,
                count: None,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        (2 * WORKGROUP * 2 * std::mem::size_of::<f32>()) as _,
                    ),
                },
            },
            BindGroupLayoutEntry {
                binding: 2,
                count: None,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        (2 * WORKGROUP * std::mem::size_of::<f32>()) as _,
                    ),
                },
            },
        ],
    );

    let pipeline =
       pipeline_cache
            .queue_compute_pipeline(ComputePipelineDescriptor {
                layout: vec![texture_bind_group_layout],
                shader_defs: vec![],
                shader: asset_server.load("shaders/simplex.wgsl"),
                entry_point: None,
                push_constant_ranges: Vec::new(),
                zero_initialize_workgroup_memory: true,
                label: Some(std::borrow::Cow::Borrowed("Simplex Pipeline")),
            });

    commands.insert_resource(
        SimplexPipeline {
            pipeline
        }
    );
}
