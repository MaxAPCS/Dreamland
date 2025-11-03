use std::sync::{Arc, Mutex, atomic::AtomicBool};
use tokio::sync::mpsc;

use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup,
        render_resource::{Buffer, CachedComputePipelineId},
    },
};
mod generation;
mod invocation;

pub(crate) struct ComputePlugin;
impl Plugin for ComputePlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<invocation::GlobalComputeQueue>() // FIXME i should be in the global app not render app
            .add_systems(RenderStartup, invocation::create_pipeline.in_set(ComputeInit))
            .add_systems(
                Render,
                (invocation::invocation_worker, generation::generation_worker).chain(),
            );
    }
}


#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ComputeInit;

pub(self) const CHUNK: [u32; 2] = [128, 128]; // must be multiple of workgroup
pub(self) const COMPUTEBUFFERSIZE: usize = 16;

#[derive(Resource)]
pub(self) struct GlobalComputeStore {
    pipeline: CachedComputePipelineId,
    buffers: Arc<[ComputeInvocation; COMPUTEBUFFERSIZE]>,
    sender: mpsc::Sender<usize>,
    receiver: Mutex<mpsc::Receiver<usize>>,
}

struct ComputeInvocation {
    chunk: Mutex<Option<IRect>>,
    active: AtomicBool,
    output: Buffer,
    staging: Buffer,
}

pub use invocation::GlobalComputeQueue;
