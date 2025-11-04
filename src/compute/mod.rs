use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::mpsc;

use bevy::{
    prelude::*,
    render::{
        Render, RenderApp,
        render_resource::{Buffer, CachedComputePipelineId},
    },
};
mod generation;
mod invocation;

pub(crate) struct ComputePlugin;
impl Plugin for ComputePlugin {
    fn build(&self, app: &mut App) {
        let (itx, irx) = mpsc::unbounded_channel();
        let (gtx, grx) = mpsc::channel(COMPUTEBUFFERSIZE);

        app.sub_app_mut(RenderApp)
            .insert_resource(InvocationDequeue(itx.clone(), irx))
            .insert_resource(GenerationQueue(gtx))
            .add_systems(Render, invocation::invocation_worker);
        app.insert_resource(InvocationQueue(itx))
            .insert_resource(GenerationDequeue(grx))
            .add_systems(FixedPreUpdate, generation::generation_worker);
    }

    fn finish(&self, app: &mut App) {
        let gcsref = app
            .sub_app_mut(RenderApp)
            .init_resource::<GlobalComputeStore>()
            .world()
            .resource::<GlobalComputeStore>()
            .buffers
            .clone();

        // Insert the cloned Arc into the main app
        app.insert_resource(GlobalComputeStoreRef(gcsref));
    }
}

pub(self) const SAMPLES: [u32; 3] = [128, 1, 128]; // must be multiple of workgroup
const COMPUTEBUFFERSIZE: usize = 16;

// Public Interface
#[derive(Resource)]
pub(crate) struct InvocationQueue(mpsc::UnboundedSender<IRect>);
impl InvocationQueue {
    pub fn add(&mut self, chunk: IRect) {
        self.0.send(chunk).unwrap();
    }
}

#[derive(Resource)]
pub(self) struct InvocationDequeue(mpsc::UnboundedSender<IRect>, mpsc::UnboundedReceiver<IRect>);

#[derive(Resource)]
pub(self) struct GenerationQueue(mpsc::Sender<(usize, IRect)>);

#[derive(Resource)]
pub(self) struct GenerationDequeue(mpsc::Receiver<(usize, IRect)>);

#[derive(Resource)]
pub(self) struct GlobalComputeStore {
    pipeline: CachedComputePipelineId,
    buffers: Arc<[ComputeInvocation; COMPUTEBUFFERSIZE]>,
    config: Buffer,
}

struct ComputeInvocation {
    active: AtomicBool,
    output: Buffer,
    staging: Buffer,
}

#[derive(Resource)]
pub(self) struct GlobalComputeStoreRef(Arc<[ComputeInvocation; COMPUTEBUFFERSIZE]>);
