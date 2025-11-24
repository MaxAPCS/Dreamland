use bevy::{
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    pbr::{Atmosphere, AtmosphereSettings},
    post_process::bloom::Bloom,
    prelude::*,
    render::experimental::occlusion_culling::OcclusionCulling,
};

mod compute;
mod ischromium;
mod player;
mod quadtree;
mod world;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        recognize_pinch_gesture: false,
                        recognize_rotation_gesture: false,
                        recognize_doubletap_gesture: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    filter: "warn,bevy_pbr=error".to_string(), // portfolio=debug,
                    ..default()
                }),
        )
        .insert_resource(Time::<Fixed>::from_hz(24.))
        .add_plugins(avian3d::PhysicsPlugins::default())
        .add_plugins(player::PlayerPlugin)
        .add_plugins(compute::ComputePlugin)
        .add_plugins(world::WorldPlugin)
        .add_systems(Startup, (setup_world, setup_camera))
        .add_systems(FixedUpdate, animate_light_direction)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        DistanceFog {
            falloff: FogFalloff::from_visibility_squared(1024.0),
            ..default()
        },
        Atmosphere { ..default() },
        AtmosphereSettings { ..default() },
        Exposure::SUNLIGHT,
        Bloom::NATURAL,
        Tonemapping::Reinhard,
        OcclusionCulling,
    ));
}

fn setup_world(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::DIRECT_SUNLIGHT,
            shadows_enabled: true,
            shadow_depth_bias: 0.6,
            ..default()
        },
        CascadeShadowConfigBuilder {
            maximum_distance: world::LEVELS[2] as f32,
            first_cascade_far_bound: world::LEVELS[0] as f32,
            ..default()
        }
        .build(),
    ));
    commands.insert_resource(DirectionalLightShadowMap { size: 1024 });
    commands.spawn((
        Transform::from_xyz(0.0, 16.0, 0.0),
        avian3d::prelude::Collider::cylinder(4.0, 1.0),
        avian3d::prelude::RigidBody::Static,
    ));
}

static MUL: std::sync::LazyLock<f32> = std::sync::LazyLock::new(|| 3f32.powf(1.5) / 4.);
fn animate_light_direction(
    time: Res<Time>,
    mut light: Single<&mut Transform, With<DirectionalLight>>,
) {
    let x = time.elapsed_secs() / 4.;
    light.rotation = Quat::from_axis_angle(
        Vec3::new(0., 0.2, 1.).normalize(),
        x.cos().powi(4) - 0.5 + x * *MUL,
    );
}
