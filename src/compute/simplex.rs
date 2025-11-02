use std::borrow::Cow;
use std::iter;

use bevy::{
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
    },
};

pub(crate) struct SimplexPlugin;

impl Plugin for SimplexPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .add_systems(RenderStartup, create_pipeline)
            .add_systems(bevy::render::Render, begin);
    }
}

pub fn begin(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    pipeline_id: Res<SimplexPipeline>
) {
    let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id.0) else {
        return;
    };

    once!(invoke_pipeline(render_device, render_queue, pipeline, &IRect::from_center_size(IVec2::ZERO, IVec2::splat(128))));
}

const WORKGROUP: [u32; 3] = [8, 1, 8];
const SIDE: u32 = 128;

#[repr(C, packed)]
#[derive(Clone, Copy, bytemuck::NoUninit)]
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

#[repr(C, packed)]
#[derive(Clone, Copy, bytemuck::NoUninit)]
struct FractalNoiseBounds {
    x: f32,
    z: f32,
    xstep: f32,
    zstep: f32,
}

#[derive(Resource)]
pub(crate) struct SimplexPipeline(pub CachedComputePipelineId);

fn create_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
) {
    let bind_group_layout = render_device.create_bind_group_layout(
        "Simplex Bind Group Layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        std::mem::size_of::<FractalNoiseConfig>() as _
                    ),
                },
            },
            BindGroupLayoutEntry {
                binding: 1,
                count: None,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        std::mem::size_of::<FractalNoiseBounds>() as _
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
                        std::mem::size_of::<[f32; (WORKGROUP[0] * WORKGROUP[1] * WORKGROUP[2]) as usize]>() as _
                    ),
                },
            },
        ],
    );

    let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some(Cow::Borrowed("Simplex Pipeline")),
        layout: vec![bind_group_layout],
        shader_defs: vec![],
        shader: asset_server.load("shaders/simplex.wgsl"),
        entry_point: Some(Cow::Borrowed("main")),
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: false,
    });

    commands.insert_resource(SimplexPipeline(pipeline));
}

pub fn invoke_pipeline(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline: &ComputePipeline,
    chunk: &IRect,
) {
    // Create Data Buffers
    let cfg_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Fractal Noise Config Buffer"),
        usage: BufferUsages::UNIFORM,
        contents: bytemuck::bytes_of(&CONFIG),
    });
    let bnd_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Fractal Noise Bounds Buffer"),
        usage: BufferUsages::UNIFORM,
        contents: bytemuck::bytes_of(&FractalNoiseBounds {
            x: chunk.min.x as f32,
            z: chunk.min.y as f32,
            xstep: chunk.width() as f32 / (SIDE - 1) as f32,
            zstep: chunk.height() as f32 / (SIDE - 1) as f32,
        }),
    });
    let out_buf = render_device.create_buffer(&BufferDescriptor {
        label: Some("Fractal Noise Output Buffer"),
        usage: BufferUsages::COPY_SRC | BufferUsages::STORAGE,
        size: std::mem::size_of::<[f32; (SIDE * SIDE) as usize]>() as _,
        mapped_at_creation: false,
    });
    let staging_buf = render_device.create_buffer(&BufferDescriptor {
        label: Some("Fractal Noise Staging Buffer"),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        size: out_buf.size(),
        mapped_at_creation: false,
    });

    // Create Bind Group
    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = render_device.create_bind_group(
        None,
        &bind_group_layout.into(), // convert into wgpu version
        &BindGroupEntries::sequential((
            cfg_buf.as_entire_binding(),
            bnd_buf.as_entire_binding(),
            out_buf.as_entire_binding(),
        )),
    );

    // Create Compute Pass
    let mut encoder =
        render_device.create_command_encoder(&CommandEncoderDescriptor { label: None });
    {
        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups(SIDE / WORKGROUP[0], 1, SIDE / WORKGROUP[2]);
    }
    encoder.copy_buffer_to_buffer(&out_buf, 0, &staging_buf, 0, out_buf.size());

    // Execute and Return
    render_queue.submit(iter::once(encoder.finish()));

    let capturable = staging_buf.clone();
    staging_buf.map_async(MapMode::Read, .., move |err| {
        err.unwrap();
        let view = capturable.get_mapped_range(..);
        let floats: &[f32] = bytemuck::cast_slice(&view);

        bevy::log::info!("{:?}", floats);
    });
}
