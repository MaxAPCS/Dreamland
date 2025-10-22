use std::f32::consts::FRAC_PI_4;
use std::f32::consts::PI;

use avian3d::prelude::{RigidBody, Collider};
use bevy::prelude::*;

mod compute;
use compute::mesh;
use compute::simplexcpu;
mod player;
use player::PlayerPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        window_theme: Some(bevy::window::WindowTheme::Dark),
                        ..default()
                    }),
                    ..default()
                }),
            avian3d::PhysicsPlugins::default(),
        ))
        .add_plugins(PlayerPlugin)
        .add_systems(Startup, setup_world)
        .add_systems(FixedUpdate, animate_light_direction)
        .run();
}


fn setup_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(DirectionalLight {
        shadows_enabled: true,
        ..default()
    });

    let terrain = mesh::mesh(
        mesh::Chunk {
            x: -50,
            z: -50,
            w: 100,
            h: 100,
            lod: 1,
        },
        &|pos| simplexcpu::fbm_simplex_2d_seeded(pos, 4, 2., 0.5, 0.) * 4.,
    );
    commands.spawn((
        RigidBody::Static,
        Collider::trimesh_from_mesh(&terrain).unwrap(),
        Mesh3d(asset_server.add(terrain)),
        MeshMaterial3d(asset_server.add(StandardMaterial {
            base_color: Color::linear_rgb(0., 1., 0.),
            ..Default::default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}


fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.elapsed_secs() * PI / 5.0,
            -FRAC_PI_4,
        );
    }
}
