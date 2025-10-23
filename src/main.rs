use std::f32::consts::FRAC_PI_4;
use std::f32::consts::PI;

use bevy::prelude::*;

mod compute;
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
        .add_plugins(world::WorldPlugin)
        .add_systems(Startup, (setup_world, setup_camera))
        .add_systems(FixedUpdate, animate_light_direction)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        DistanceFog {
            color: Color::srgba(0.35, 0.48, 0.66, 1.0),
            directional_light_color: Color::srgba(1.0, 0.95, 0.85, 0.5),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::from_visibility_colors(
                1024.0, // distance in world units up to which objects retain visibility (>= 5% contrast)
                Color::srgb(0.35, 0.5, 0.66), // atmospheric extinction color (after light is lost due to absorption by atmospheric particles)
                Color::srgb(0.8, 0.844, 1.0), // atmospheric inscattering color (light gained due to scattering from the sun)
            ),
        },
    ));
}

fn setup_world(mut commands: Commands) {
    commands.spawn(DirectionalLight {
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
        0.0,
        time.elapsed_secs() * PI / 5.0,
        -FRAC_PI_4,
    );
}

pub fn contains(big: &IRect, small: &IRect) -> bool {
    // Check if small is completely inside big
    big.min.x <= small.min.x
        && big.max.x >= small.max.x
        && big.min.y <= small.min.y
        && big.max.y >= small.max.y
}
