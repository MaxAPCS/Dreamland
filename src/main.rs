use std::f32::consts::{FRAC_PI_4, FRAC_PI_8};

use bevy::{
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    pbr::{Atmosphere, AtmosphereSettings},
    post_process::bloom::Bloom,
    prelude::*,
};

mod compute;
mod cpu;
mod player;
mod quadtree;
mod world;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                recognize_pinch_gesture: false,
                recognize_rotation_gesture: false,
                recognize_doubletap_gesture: false,
                ..default()
            }),
            ..default()
        }))
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
            falloff: FogFalloff::from_visibility(
                1024.0, // distance in world units up to which objects retain visibility (>= 5% contrast)
            ),
            ..default()
        },
        Atmosphere { ..default() },
        AtmosphereSettings { ..default() },
        Exposure::SUNLIGHT,
        Bloom::NATURAL,
        Tonemapping::Reinhard,
    ));
}

fn setup_world(mut commands: Commands) {
    commands.spawn(DirectionalLight {
        illuminance: light_consts::lux::DIRECT_SUNLIGHT,
        shadows_enabled: true,
        ..default()
    });
    commands.spawn((
        Transform::from_xyz(0.0, 8.0, 0.0),
        avian3d::prelude::Collider::cylinder(4.0, 1.0),
        avian3d::prelude::RigidBody::Static,
    ));
}

fn animate_light_direction(
    time: Res<Time>,
    mut light: Single<&mut Transform, With<DirectionalLight>>,
) {
    light.rotation = Quat::from_euler(
        EulerRot::ZYX,
        FRAC_PI_4,
        0.0,
        time.elapsed_secs() * FRAC_PI_8 * 0.25,
    );
}
