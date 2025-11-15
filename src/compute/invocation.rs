use std::{
    iter,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
    },
};

use crate::compute::{
    ComputeInvocation, GenerationQueue, GlobalComputeStore, InvocationDequeue, SAMPLES,
};

const WORKGROUP: [u32; 3] = [8, 1, 8];

// WGSL Objects
#[repr(C, packed)]
#[derive(Clone, Copy, bytemuck::NoUninit)]
struct FractalNoiseConfig {
    seed: f32,
    octaves: u32,
    lacunarity: f32,
    gain: f32,
    scale: f32,
    erosion: f32,
}

const CONFIG: FractalNoiseConfig = FractalNoiseConfig {
    seed: 0.0,
    octaves: 5,
    lacunarity: 2.,
    gain: 0.5,
    scale: 24.,
    erosion: 0.2,
};

#[repr(C, packed)]
#[derive(Clone, Copy, bytemuck::NoUninit)]
pub(super) struct FractalNoiseBounds {
    pub(super) x: f32,
    pub(super) z: f32,
    pub(super) xstep: f32,
    pub(super) zstep: f32,
}

impl From<&IRect> for FractalNoiseBounds {
    fn from(chunk: &IRect) -> Self {
        Self {
            x: chunk.min.x as f32,
            z: chunk.min.y as f32,
            xstep: chunk.width() as f32 / (SAMPLES[0] - 1) as f32,
            zstep: chunk.height() as f32 / (SAMPLES[2] - 1) as f32,
        }
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub(super) struct PointAndNormal {
    pub(super) point: f32,
    pub(super) normal: [f32; 3],
}

// Bevy Systems
impl FromWorld for GlobalComputeStore {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let asset_server = world.resource::<AssetServer>();

        // Create Pipeline Bind Group Layout
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
                            std::mem::size_of::<FractalNoiseConfig>() as _,
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
                            std::mem::size_of::<FractalNoiseBounds>() as _,
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
                        min_binding_size: BufferSize::new(std::mem::size_of::<
                            [PointAndNormal; (WORKGROUP[0] * WORKGROUP[1] * WORKGROUP[2]) as usize],
                        >() as _),
                    },
                },
            ],
        );

        // Create Pipeline
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            layout: vec![bind_group_layout],
            shader: asset_server.load("shaders/simplex.wgsl"),
            ..default()
        });

        // Create Reusable Storage Buffers
        let bufsize = std::mem::size_of::<
            [PointAndNormal; (SAMPLES[0] * SAMPLES[1] * SAMPLES[2]) as usize],
        >() as _;
        Self {
            pipeline: pipeline,
            buffers: Arc::new(std::array::from_fn(|i| ComputeInvocation {
                active: AtomicBool::new(false),
                output: render_device.create_buffer(&BufferDescriptor {
                    label: Some(format!("Fractal Noise Output Buffer {}", i).as_str()),
                    usage: BufferUsages::COPY_SRC | BufferUsages::STORAGE,
                    size: bufsize,
                    mapped_at_creation: false,
                }),
                staging: render_device.create_buffer(&BufferDescriptor {
                    label: Some(format!("Fractal Noise Staging Buffer {}", i).as_str()),
                    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                    size: bufsize,
                    mapped_at_creation: false,
                }),
            })),
            config: render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Fractal Noise Config Buffer"),
                usage: BufferUsages::UNIFORM,
                contents: bytemuck::bytes_of(&CONFIG),
            }),
        }
    }
}

pub(super) fn invocation_worker(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    global_compute_store: Res<GlobalComputeStore>,
    mut inv_dequeue: ResMut<InvocationDequeue>,
    gen_queue: Res<GenerationQueue>,
) {
    let Some(pipeline) = pipeline_cache.get_compute_pipeline(global_compute_store.pipeline) else {
        return;
    };

    while let Ok(chunk) = inv_dequeue.1.try_recv() {
        if let Err(_) = invoke_pipeline(
            &render_device,
            &render_queue,
            &global_compute_store,
            &pipeline,
            &chunk,
            &gen_queue,
        ) {
            inv_dequeue.0.send(chunk).unwrap();
            break;
        }
    }
}

fn invoke_pipeline(
    render_device: &Res<RenderDevice>,
    render_queue: &Res<RenderQueue>,
    global_compute_store: &Res<GlobalComputeStore>,
    pipeline: &ComputePipeline,
    chunk: &IRect,
    gen_queue: &Res<GenerationQueue>,
) -> Result<(), &'static str> {
    // Fetch Reusable Storage Buffers
    let mut found_inv = None;
    for (i, buffers) in global_compute_store.buffers.iter().enumerate() {
        if !buffers.active.load(Ordering::Acquire) {
            buffers.active.store(true, Ordering::Release);
            found_inv = Some((i, buffers));
            break;
        }
    }
    let Some((
        index,
        ComputeInvocation {
            active: _,
            output: out_buf,
            staging: staging_buf,
        },
    )) = found_inv
    else {
        return Err("No Available Storage Buffers");
    };

    {
        // Create Temp. Uniform Buffer
        let bnd_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Fractal Noise Bounds Buffer"),
            usage: BufferUsages::UNIFORM,
            contents: bytemuck::bytes_of(&FractalNoiseBounds::from(chunk)),
        });

        // Create Bind Group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = render_device.create_bind_group(
            None,
            &bind_group_layout.into(), // convert into wgpu version
            &BindGroupEntries::sequential((
                global_compute_store.config.as_entire_binding(),
                bnd_buf.as_entire_binding(),
                out_buf.as_entire_binding(),
            )),
        );

        // Check Chunk-Workgroup Divisibility
        debug_assert_eq!(SAMPLES[0] % WORKGROUP[0], 0);
        debug_assert_eq!(SAMPLES[1] % WORKGROUP[1], 0);
        debug_assert_eq!(SAMPLES[2] % WORKGROUP[2], 0);

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
            cpass.dispatch_workgroups(
                SAMPLES[0] / WORKGROUP[0],
                SAMPLES[1] / WORKGROUP[1],
                SAMPLES[2] / WORKGROUP[2],
            );
        }
        encoder.copy_buffer_to_buffer(out_buf, 0, staging_buf, 0, out_buf.size());

        // Execute
        render_queue.submit(iter::once(encoder.finish()));
    }

    let sender = gen_queue.0.clone();
    let chunk = chunk.clone();
    staging_buf.map_async(MapMode::Read, .., move |err| {
        err.unwrap();
        sender.try_send((index, chunk)).unwrap();
    });

    Ok(())
}
