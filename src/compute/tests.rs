use std::time::Instant;

use bevy::log::Level;
use bevy::log::LogPlugin;
use bevy::prelude::*;

use crate::compute;
use crate::cpu;

#[test]
fn gpu_vs_cpu() {
    let sample = IRect::from_center_size(IVec2::ZERO, IVec2::splat(128));

    let cpu;
    let cputime;
    {
        let now = Instant::now();
        cpu = cpu::invocation::invoke_function(&sample);
        cputime = now.elapsed();
    }

    let mut now = None;
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(compute::ComputePlugin)
        .add_plugins(LogPlugin {
            level: Level::TRACE,
            ..Default::default()
        })
        .add_systems(Startup, move |mut inv: ResMut<compute::InvocationQueue>| {
            inv.add(sample);
            now = Some(Instant::now());
        })
        .add_systems(
            FixedPreUpdate,
            move |global_compute_store: Res<compute::GlobalComputeStoreRef>,
                  mut gen_dequeue: ResMut<compute::GenerationDequeue>| {
                let Ok((index, chu)) = gen_dequeue.0.try_recv() else {
                    return;
                };

                assert_eq!(chu, sample);

                let gpubuf = &global_compute_store.0[index].staging;
                let view = gpubuf.get_mapped_range(..);
                let gpu: &[f32] = bytemuck::cast_slice(&view);
                let gputime = (&now.unwrap()).elapsed();

                assert_eq!(cpu, *gpu);

                bevy::log::info!(
                    "CPU Runtime: {}ms \nGPU Runtine: {}ms ",
                    cputime.as_millis(),
                    gputime.as_millis()
                );
            },
        )
        .run();
}
