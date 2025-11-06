#[cfg(test)]
mod test {
    use bevy::prelude::*;
    use bevy::time::Stopwatch;

    use crate::compute;
    use crate::cpu;

    #[derive(Resource, Default)]
    pub struct Profiler(Stopwatch);

    #[test]
    fn gpu_vs_cpu() {
        let sample = IRect::from_center_size(IVec2::ZERO, IVec2::splat(128));

        let cpu;
        let cputime;
        {
            let now = std::time::Instant::now();
            cpu = cpu::invocation::invoke_function(&sample);
            cputime = now.elapsed();
        }

        App::new()
            .init_resource::<Profiler>()
            .add_plugins(MinimalPlugins)
            .add_plugins(compute::ComputePlugin)
            .add_plugins(bevy::log::LogPlugin {
                level: bevy::log::Level::TRACE,
                ..Default::default()
            })
            .add_systems(
                Startup,
                move |mut inv: ResMut<compute::InvocationQueue>, mut timer: ResMut<Profiler>| {
                    inv.add(sample);
                    timer.0.reset();
                    timer.0.unpause();
                },
            )
            .add_systems(
                FixedPreUpdate,
                move |global_compute_store: Res<compute::GlobalComputeStoreRef>,
                      mut gen_dequeue: ResMut<compute::GenerationDequeue>,
                      timer: Res<Profiler>| {
                    let Ok((index, chu)) = gen_dequeue.0.try_recv() else {
                        return;
                    };

                    assert_eq!(chu, sample);

                    let gpubuf = &global_compute_store.0[index].staging;
                    let view = gpubuf.get_mapped_range(..);
                    let gpu: &[f32] = bytemuck::cast_slice(&view);
                    let gputime = timer.0.elapsed();

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
}
