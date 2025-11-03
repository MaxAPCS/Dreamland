use std::{
    collections::VecDeque,
    iter,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::mpsc;

use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
    },
};

use crate::compute::{CHUNK, COMPUTEBUFFERSIZE, ComputeInvocation, GlobalComputeStore};

const WORKGROUP: [u32; 3] = [8, 1, 8];

// Public Interface
#[derive(Resource)]
pub struct GlobalComputeQueue(VecDeque<IRect>);
impl Default for GlobalComputeQueue {
    fn default() -> Self {
        Self(VecDeque::new())
    }
}
impl GlobalComputeQueue {
    pub fn add(&mut self, chunk: IRect) {
        self.0.push_back(chunk);
    }
}

// WGSL Objects
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
            xstep: chunk.width() as f32 / (CHUNK[0] - 1) as f32,
            zstep: chunk.height() as f32 / (CHUNK[1] - 1) as f32,
        }
    }
}

// Bevy Systems
pub(super) fn create_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
) {
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
                    min_binding_size: BufferSize::new(std::mem::size_of::<
                        [f32; (WORKGROUP[0] * WORKGROUP[1] * WORKGROUP[2]) as usize],
                    >() as _),
                },
            },
        ],
    );

    // Create Pipeline
    let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: None,
        layout: vec![bind_group_layout],
        shader_defs: vec![],
        shader: asset_server.load("shaders/simplex.wgsl"),
        entry_point: None,
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: false,
    });

    // Create Sender & Receiver
    let (tx, rx) = mpsc::channel(COMPUTEBUFFERSIZE);

    // Create Reusable Storage Buffers
    let bufsize = std::mem::size_of::<[f32; (CHUNK[0] * CHUNK[1]) as usize]>() as _;
    commands.insert_resource(GlobalComputeStore {
        pipeline: pipeline,
        buffers: Arc::new(std::array::from_fn(|i| ComputeInvocation {
            chunk: Mutex::new(None),
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
        sender: tx,
        receiver: Mutex::new(rx),
    });
}

pub(super) fn invocation_worker(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    global_compute_store: Res<GlobalComputeStore>,
    mut global_compute_queue: ResMut<GlobalComputeQueue>,
) {
    bevy::log::info!("invocation worker: {}", global_compute_queue.0.len());
    
    let Some(pipeline) = pipeline_cache.get_compute_pipeline(global_compute_store.pipeline) else {
        return;
    };

    while let Some(chunk) = global_compute_queue.0.front() {
        if let Err(_) = invoke_pipeline(
            &render_device,
            &render_queue,
            &global_compute_store,
            &pipeline,
            &chunk,
        ) {
            break;
        }
        global_compute_queue.0.pop_front();
    }
}

fn invoke_pipeline(
    render_device: &Res<RenderDevice>,
    render_queue: &Res<RenderQueue>,
    global_compute_store: &Res<GlobalComputeStore>,
    pipeline: &ComputePipeline,
    bounds: &IRect,
) -> Result<(), &'static str> {
    // Fetch Reusable Storage Buffers
    let mut found_buffers = None;
    for (i, buffers) in global_compute_store.buffers.iter().enumerate() {
        if !buffers.active.load(Ordering::Acquire) {
            buffers.active.store(true, Ordering::Release);
            *buffers.chunk.lock().unwrap() = Some(bounds.to_owned());
            found_buffers = Some((i, &buffers.output, &buffers.staging));
            break;
        }
    }
    let Some((index, out_buf, staging_buf)) = found_buffers else {
        return Err("No Available Storage Buffers");
    };

    // Create Sender
    let sender = global_compute_store.sender.clone();

    // Create Temp. Uniform Buffers
    let cfg_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Fractal Noise Config Buffer"),
        usage: BufferUsages::UNIFORM,
        contents: bytemuck::bytes_of(&CONFIG),
    });
    let bnd_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Fractal Noise Bounds Buffer"),
        usage: BufferUsages::UNIFORM,
        contents: bytemuck::bytes_of(&FractalNoiseBounds::from(bounds)),
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

    // Check Chunk-Workgroup Divisibility
    debug_assert_eq!(CHUNK[0] % WORKGROUP[0], 0);
    debug_assert_eq!(WORKGROUP[1], 1);
    debug_assert_eq!(CHUNK[1] % WORKGROUP[2], 0);

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
        cpass.dispatch_workgroups(CHUNK[0] / WORKGROUP[0], 1, CHUNK[1] / WORKGROUP[2]);
    }
    encoder.copy_buffer_to_buffer(out_buf, 0, staging_buf, 0, out_buf.size());

    // Execute and Return
    render_queue.submit(iter::once(encoder.finish()));
    staging_buf.map_async(MapMode::Read, .., move |err| {
        err.unwrap();
        sender.try_send(index).unwrap();
    });

    Ok(())
}
