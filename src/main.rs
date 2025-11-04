use std::f32::consts::{FRAC_PI_3, FRAC_PI_8};

use bevy::{
    camera::Exposure,
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
                window_theme: Some(bevy::window::WindowTheme::Dark),
                ..default()
            }),
            ..default()
        }))
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
    ));
}

fn setup_world(mut commands: Commands) {
    commands.spawn(DirectionalLight {
        illuminance: light_consts::lux::DIRECT_SUNLIGHT,
        shadows_enabled: true,
        ..default()
    });
}

fn animate_light_direction(
    time: Res<Time>,
    mut light: Single<&mut Transform, With<DirectionalLight>>,
) {
    light.rotation = Quat::from_euler(
        EulerRot::ZYX,
        FRAC_PI_3,
        0.0,
        time.elapsed_secs() * FRAC_PI_8 * 0.25,
    );
}
